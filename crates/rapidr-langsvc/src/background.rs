//! The analysis off the UI thread (docs/speed-audit-2026-11.md, "Studio:
//! the edit pause"). An [`Analysis`] set to work in the background
//! ([`Analysis::set_background`]: RapidR Studio's code editors and its
//! Outline and Problems panes) never parses, names or compiles a program on
//! the thread that asks when it's asked ahead with [`Analysis::request`]:
//!
//! 1. the UI sends a job: its version of the texts (the generation), every
//!    open file's text and the options, the file and what it wants of it
//!    ([`Want`]: the program's problems, the file's colours …);
//! 2. a worker — a thread of its own, one for the process — brings its own
//!    copy of the analysis (a mirror, one per asking analysis) to those
//!    texts and makes what was wanted with the very same code a synchronous
//!    call runs, so the answers are the same;
//! 3. the UI takes the work in ([`Analysis::poll`]) only when it is for the
//!    texts as they are now (its generation): older work is dropped, and
//!    newer work is then on its way — every change of the texts is a new
//!    generation, and whoever asks again asks for it. A newer job of the
//!    same analysis supersedes an older one: the worker leaves it between
//!    two of its steps.
//!
//! What crosses threads is plain data (texts, paths, the parse and its
//! diagnostics behind `Arc`s): never one of the runtime's values.
//!
//! Where there are no threads (a browser's wasm without them: RapidR
//! Studio on the web) the same jobs run on the UI thread in steps, one
//! [`Want`] a step, each when the page's loop next asks
//! ([`Analysis::poll`]): the work spread between frames, never in the
//! keystroke's.
//!
//! Asked for something exact meanwhile (a definition, the problems before
//! a run), the analysis waits for the work under way rather than make it
//! twice ([`Analysis::settle`]).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{Analysis, FileDiagnostic, OutlineItem, SemanticToken, Snapshot};

/// What a job makes of its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Want {
    /// The file's analysis (its program's parse and names).
    Snapshot,
    /// The problems of the program the file belongs to (its main file's:
    /// what a code editor shows).
    RootDiagnostics,
    /// [`Analysis::diagnostics`] of this file (RLanguageService's
    /// `Diagnostics`).
    Diagnostics,
    /// The file's semantic colours.
    Tokens,
    /// The file's helpers (docs/code-helpers-plan.md).
    Helpers,
    /// The file's outline.
    Outline,
    /// The project's problems ([`Analysis::project_diagnostics`] of this
    /// main file).
    Project(PathBuf),
}

/// One analysis's work, sent to the worker: everything it asked for the
/// texts of one generation (a later question of the same texts sends them
/// all again: what was made already is the mirror's at once).
struct Job {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    client: u64,
    generation: u64,
    options: crate::Options,
    docs: Vec<(PathBuf, String)>,
    wants: Vec<(PathBuf, Want)>,
}

/// What a job made: the mirror's caches for its texts.
#[derive(Default)]
pub(crate) struct Done {
    generation: u64,
    /// The work failed (the analysis panicked): asked again, it's made on
    /// the thread that asks, as it would be without a worker.
    failed: bool,
    snapshots: Vec<(PathBuf, Arc<Snapshot>)>,
    diagnostics: Vec<(PathBuf, Vec<FileDiagnostic>)>,
    project: Option<(PathBuf, Vec<FileDiagnostic>)>,
    tokens: Vec<(PathBuf, Vec<SemanticToken>)>,
    helpers: Vec<(PathBuf, Vec<rapidr_editor::service::HelperSite>)>,
    outlines: Vec<(PathBuf, Vec<OutlineItem>)>,
}

/// An analysis's link to its worker.
pub(crate) struct Client {
    id: u64,
    /// The job sent and not taken in yet: its generation and its wants.
    asked: Option<(u64, Vec<(PathBuf, Want)>)>,
    /// The generation whose job failed (the analysis panicked): not sent
    /// again — what it wanted is made by the thread that asks, as without a
    /// worker.
    failed: Option<u64>,
    #[cfg(not(target_arch = "wasm32"))]
    shared: Arc<threaded::Shared>,
    /// (no threads) The job under way, a step at a time.
    #[cfg(target_arch = "wasm32")]
    inline: Option<inline::Running>,
    #[cfg(target_arch = "wasm32")]
    mirror: Option<Box<Analysis>>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").field("id", &self.id).field("asked", &self.asked).finish()
    }
}

fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

impl Client {
    fn new() -> Client {
        Client {
            id: next_id(),
            asked: None,
            failed: None,
            #[cfg(not(target_arch = "wasm32"))]
            shared: Arc::default(),
            #[cfg(target_arch = "wasm32")]
            inline: None,
            #[cfg(target_arch = "wasm32")]
            mirror: None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Client {
    fn drop(&mut self) {
        threaded::forget(self.id);
    }
}

/// Brings `mirror` to the job's texts and options (only what differs).
fn sync_mirror(mirror: &mut Analysis, options: &crate::Options, docs: &[(PathBuf, String)]) {
    if mirror.options() != options {
        // (the case alone: nothing made depends on it — Analysis::set_case)
        let mut same = mirror.options().clone();
        same.case = options.case;
        if same == *options {
            mirror.set_case(options.case);
        } else {
            mirror.set_options(options.clone());
        }
    }
    let gone: Vec<PathBuf> = mirror.docs.keys().filter(|p| !docs.iter().any(|(d, _)| d == *p)).cloned().collect();
    for p in gone {
        mirror.close(&p);
    }
    for (p, t) in docs {
        if mirror.docs.get(p) != Some(t) {
            mirror.update(p.clone(), t.clone());
        }
    }
    // (a mirror never answers typing's quick questions: no older analysis
    // kept for them)
    mirror.stale.clear();
}

/// One of a job's wants, made by the mirror.
fn make(mirror: &mut Analysis, file: &Path, want: &Want) {
    match want {
        Want::Snapshot => {
            let _ = mirror.snapshot(file);
        }
        Want::RootDiagnostics => {
            let root = mirror.snapshot(file).map_or_else(|| file.to_path_buf(), |s| s.parsed.root.clone());
            let _ = mirror.diagnostics(&root);
        }
        Want::Diagnostics => {
            let _ = mirror.diagnostics(file);
        }
        Want::Tokens => {
            let _ = mirror.semantic_tokens(file);
        }
        Want::Helpers => {
            let _ = mirror.helpers(file);
        }
        Want::Outline => {
            let _ = mirror.outline(file);
        }
        Want::Project(main) => {
            let _ = mirror.project_diagnostics(main);
        }
    }
}

/// What the mirror made for generation `generation`.
fn harvest(mirror: &Analysis, generation: u64) -> Done {
    Done {
        generation,
        failed: false,
        snapshots: mirror.snapshots.iter().map(|(p, s)| (p.clone(), s.clone())).collect(),
        diagnostics: mirror.diagnostics.iter().map(|(p, d)| (p.clone(), d.clone())).collect(),
        project: mirror.project_diags.clone(),
        tokens: mirror.tokens.iter().map(|(p, t)| (p.clone(), t.clone())).collect(),
        helpers: mirror.helper_sites.iter().map(|(p, h)| (p.clone(), h.clone())).collect(),
        outlines: mirror.outlines.iter().map(|(p, o)| (p.clone(), o.clone())).collect(),
    }
}

/// The wants not answered yet by what `a` has for its texts now.
fn missing(a: &Analysis, file: &Path, wants: &[Want]) -> Vec<Want> {
    wants
        .iter()
        .filter(|w| match w {
            Want::Snapshot => !a.snapshots.contains_key(file),
            Want::RootDiagnostics => !a.snapshots.get(file).is_some_and(|s| a.diagnostics.contains_key(&s.parsed.root)),
            Want::Diagnostics => !a.diagnostics.contains_key(file),
            Want::Tokens => !a.tokens.contains_key(file),
            Want::Helpers => !a.helper_sites.contains_key(file),
            Want::Outline => !a.outlines.contains_key(file),
            Want::Project(main) => !a.project_diags.as_ref().is_some_and(|(m, _)| m == main),
        })
        .cloned()
        .collect()
}

impl Analysis {
    /// From now on (`on`), what [`Analysis::request`] asks for is made in
    /// the background, and the exact questions wait for that work rather
    /// than make it again; off: everything on the asking thread, as the
    /// language server and the tools have it.
    pub fn set_background(&mut self, on: bool) {
        if on != self.background.is_some() {
            self.background = on.then(Client::new);
        }
    }

    /// Whether the analysis works in the background.
    pub fn in_background(&self) -> bool {
        self.background.is_some()
    }

    /// Asks for `wants` of `file` for the texts as they are now, made in the
    /// background (synchronously, now, without [`Analysis::set_background`]);
    /// returns at once. Once [`Analysis::ready`] they answer without work.
    /// Asked again before they're in: nothing more is sent unless more is
    /// wanted, or the texts changed (then the newer job supersedes).
    pub fn request(&mut self, file: &Path, wants: &[Want]) {
        self.poll();
        let wanted = missing(self, file, wants);
        if wanted.is_empty() {
            return;
        }
        if self.background.is_none() {
            for w in &wanted {
                make(self, file, w);
            }
            return;
        }
        let generation = self.generation;
        let mut all: Vec<(PathBuf, Want)> = Vec::new();
        if let Some(client) = self.background.as_ref() {
            if client.failed == Some(generation) {
                return;
            }
            if let Some((g, asked)) = &client.asked {
                if *g == generation {
                    if wanted.iter().all(|w| asked.iter().any(|(f, a)| f == file && a == w)) {
                        return;
                    }
                    // (more wanted of the same texts: one job with both)
                    all = asked.clone();
                }
            }
        }
        // (the file's snapshot first: everything else is made from it)
        if !all.iter().any(|(f, w)| f == file && *w == Want::Snapshot) {
            all.push((file.to_path_buf(), Want::Snapshot));
        }
        for w in wanted {
            if !all.iter().any(|(f, a)| f == file && *a == w) {
                all.push((file.to_path_buf(), w));
            }
        }
        all.sort_by_key(|(_, w)| !matches!(w, Want::Snapshot));
        let mut docs: Vec<(PathBuf, String)> = self.docs.iter().map(|(p, t)| (p.clone(), t.clone())).collect();
        docs.sort();
        let options = self.options.clone();
        let Some(client) = self.background.as_mut() else { return };
        client.asked = Some((generation, all.clone()));
        let job = Job { client: client.id, generation, options, docs, wants: all };
        #[cfg(not(target_arch = "wasm32"))]
        threaded::send(job, &client.shared);
        #[cfg(target_arch = "wasm32")]
        {
            client.inline = Some(inline::Running { job, step: 0 });
        }
    }

    /// Whether `wants` of `file` answer now, from what was made for the
    /// texts as they are (the background's work taken in first).
    pub fn ready(&mut self, file: &Path, wants: &[Want]) -> bool {
        self.poll();
        missing(self, file, wants).is_empty()
    }

    /// Whether work asked for the texts as they are now isn't in yet.
    pub fn pending(&self) -> bool {
        self.background.as_ref().is_some_and(|c| c.asked.as_ref().is_some_and(|(g, _)| *g == self.generation))
    }

    /// Whether the background's work for the texts as they are now failed
    /// (what it wanted is then made by the thread that asks).
    pub fn failed(&self) -> bool {
        self.background.as_ref().is_some_and(|c| c.failed == Some(self.generation))
    }

    /// The background's finished work taken in when it is for the texts as
    /// they are now — older work dropped: the newer is on its way —; with
    /// no threads, the next step of the job under way first. Whether
    /// anything was taken in.
    pub fn poll(&mut self) -> bool {
        let Some(client) = self.background.as_mut() else { return false };
        #[cfg(target_arch = "wasm32")]
        let done = inline::step(client);
        #[cfg(not(target_arch = "wasm32"))]
        let done = client.shared.take();
        match done {
            Some(done) => self.take_in(done),
            None => false,
        }
    }

    /// The work under way for the texts as they are now, waited for (or,
    /// with no threads, done now) and taken in; whether there was some.
    /// What an exact question (a definition, a rename, the problems before
    /// a run) asks first, so the work isn't made twice.
    pub(crate) fn settle(&mut self) -> bool {
        if !self.pending() {
            return false;
        }
        let generation = self.generation;
        let Some(client) = self.background.as_mut() else { return false };
        #[cfg(not(target_arch = "wasm32"))]
        let done = client.shared.wait_for(generation);
        #[cfg(target_arch = "wasm32")]
        let done = inline::finish(client);
        let _ = generation;
        match done {
            Some(done) => self.take_in(done),
            None => false,
        }
    }

    fn take_in(&mut self, done: Done) -> bool {
        let current = done.generation == self.generation;
        if let Some(client) = self.background.as_mut() {
            if client.asked.as_ref().is_some_and(|(g, _)| *g <= done.generation) {
                client.asked = None;
            }
            if done.failed {
                client.failed = Some(done.generation);
            }
        }
        if !current || done.failed {
            return false;
        }
        for (p, s) in done.snapshots {
            if s.parsed.root == p {
                self.stale.remove(&p);
            }
            self.snapshots.entry(p).or_insert(s);
        }
        for (p, d) in done.diagnostics {
            self.diagnostics.entry(p).or_insert(d);
        }
        if self.project_diags.is_none() {
            self.project_diags = done.project;
        }
        for (p, t) in done.tokens {
            self.tokens.entry(p).or_insert(t);
        }
        for (p, h) in done.helpers {
            self.helper_sites.entry(p).or_insert(h);
        }
        for (p, o) in done.outlines {
            self.outlines.entry(p).or_insert(o);
        }
        true
    }
}

/// A job made by `mirror`, its wants in order; `superseded` is asked
/// between them (a newer job of the same analysis came: this one is left).
/// `None`: left. A panic is caught: the job failed.
#[cfg(not(target_arch = "wasm32"))]
fn run(mirror: &mut Analysis, job: &Job, superseded: impl Fn() -> bool) -> Option<Done> {
    let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        sync_mirror(mirror, &job.options, &job.docs);
        for (file, w) in &job.wants {
            if superseded() {
                return None;
            }
            make(mirror, file, w);
        }
        Some(harvest(mirror, job.generation))
    }));
    match made {
        Ok(done) => done,
        Err(_) => Some(Done { generation: job.generation, failed: true, ..Done::default() }),
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod threaded {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Arc, Condvar, Mutex, OnceLock};
    use std::time::Duration;

    use super::{run, Done, Job};
    use crate::Analysis;

    /// A client's side of the worker: the latest generation asked (older
    /// jobs are left), the work done, and its bell.
    #[derive(Default)]
    pub(crate) struct Shared {
        latest: AtomicU64,
        done: Mutex<Option<Done>>,
        bell: Condvar,
    }

    impl Shared {
        /// The finished work, if any (never waits).
        pub(crate) fn take(&self) -> Option<Done> {
            self.done.lock().ok()?.take()
        }

        /// The work for `generation`, waited for (a minute at most: a
        /// worker that never answers isn't waited for forever — the work is
        /// then made by the thread that asks).
        pub(crate) fn wait_for(&self, generation: u64) -> Option<Done> {
            let mut done = self.done.lock().ok()?;
            let until = std::time::Instant::now() + Duration::from_secs(60);
            loop {
                if done.as_ref().is_some_and(|d| d.generation >= generation) {
                    return done.take();
                }
                let left = until.saturating_duration_since(std::time::Instant::now());
                if left.is_zero() {
                    return done.take();
                }
                done = self.bell.wait_timeout(done, left).ok()?.0;
            }
        }
    }

    enum Msg {
        Job(Job, Arc<Shared>),
        Forget(u64),
    }

    fn worker() -> Option<Sender<Msg>> {
        static WORKER: OnceLock<Option<Mutex<Sender<Msg>>>> = OnceLock::new();
        let w = WORKER.get_or_init(|| {
            let (tx, rx) = mpsc::channel();
            // (the parser and the compiler recurse on deep programs: the
            // UI thread's room, not a spawned thread's 2 MB)
            std::thread::Builder::new().name("rapidr-analysis".into()).stack_size(64 << 20).spawn(move || serve(rx)).ok()?;
            Some(Mutex::new(tx))
        });
        w.as_ref()?.lock().ok().map(|tx| tx.clone())
    }

    pub(crate) fn send(job: Job, shared: &Arc<Shared>) {
        shared.latest.store(job.generation, Ordering::SeqCst);
        let generation = job.generation;
        let sent = worker().is_some_and(|tx| tx.send(Msg::Job(job, shared.clone())).is_ok());
        if !sent {
            // (no worker: the job "failed", made by the thread that asks)
            if let Ok(mut d) = shared.done.lock() {
                *d = Some(Done { generation, failed: true, ..Done::default() });
            }
            shared.bell.notify_all();
        }
    }

    pub(crate) fn forget(client: u64) {
        if let Some(tx) = worker() {
            let _ = tx.send(Msg::Forget(client));
        }
    }

    fn serve(rx: Receiver<Msg>) {
        let mut mirrors: HashMap<u64, Box<Analysis>> = HashMap::new();
        while let Ok(first) = rx.recv() {
            // (everything sent meanwhile: each client's latest job only)
            let mut jobs: Vec<(Job, Arc<Shared>)> = Vec::new();
            for msg in std::iter::once(first).chain(rx.try_iter()) {
                match msg {
                    Msg::Job(job, shared) => {
                        jobs.retain(|(j, _)| j.client != job.client);
                        jobs.push((job, shared));
                    }
                    Msg::Forget(client) => {
                        mirrors.remove(&client);
                        jobs.retain(|(j, _)| j.client != client);
                    }
                }
            }
            for (job, shared) in jobs {
                if job.generation < shared.latest.load(Ordering::SeqCst) {
                    continue;
                }
                let mirror = mirrors.entry(job.client).or_insert_with(|| Box::new(Analysis::new(job.options.clone())));
                let done = run(mirror, &job, || job.generation < shared.latest.load(Ordering::SeqCst));
                if let Some(done) = done {
                    if done.failed {
                        mirrors.remove(&job.client);
                    }
                    if let Ok(mut d) = shared.done.lock() {
                        *d = Some(done);
                    }
                    shared.bell.notify_all();
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod inline {
    use super::{harvest, make, sync_mirror, Client, Done, Job};
    use crate::Analysis;

    /// A job under way on the UI thread: its next want.
    pub(crate) struct Running {
        pub(crate) job: Job,
        pub(crate) step: usize,
    }

    /// The next step of the job under way (its mirror brought to its texts
    /// first, then one want); the work when that was the last.
    pub(crate) fn step(client: &mut Client) -> Option<Done> {
        let mut running = client.inline.take()?;
        let mirror = client.mirror.get_or_insert_with(|| Box::new(Analysis::new(running.job.options.clone())));
        let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if running.step == 0 {
                sync_mirror(mirror, &running.job.options, &running.job.docs);
            }
            if let Some((file, w)) = running.job.wants.get(running.step) {
                make(mirror, file, w);
            }
        }));
        if made.is_err() {
            client.mirror = None;
            return Some(Done { generation: running.job.generation, failed: true, ..Done::default() });
        }
        running.step += 1;
        if running.step >= running.job.wants.len() {
            return Some(harvest(mirror, running.job.generation));
        }
        client.inline = Some(running);
        None
    }

    /// The job under way, finished now.
    pub(crate) fn finish(client: &mut Client) -> Option<Done> {
        while client.inline.is_some() {
            if let Some(done) = step(client) {
                return Some(done);
            }
        }
        None
    }
}

/// (for the tests) Whether analyses of the same texts share one parse
/// (on, as RapidR Studio has it); off: every analysis parses afresh — what
/// the tests compare the shared one with.
#[doc(hidden)]
pub fn share_parses(on: bool) {
    crate::cache::set_shared(on);
}

/// (for the tests) Whether every kind of work crosses threads as plain
/// data.
#[allow(dead_code)]
fn plain_data() {
    fn crosses<T: Send + Sync>() {}
    crosses::<Snapshot>();
    crosses::<FileDiagnostic>();
    crosses::<crate::Options>();
    crosses::<OutlineItem>();
    crosses::<SemanticToken>();
    crosses::<rapidr_editor::service::HelperSite>();
}
