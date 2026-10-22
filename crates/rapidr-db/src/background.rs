//! Background queries (RapidR's; REQUEST-001 item 2): `DB.QueryAsync(sql
//! [, values …])` on RSQLITE and RMYSQL runs the query away from the
//! program, so its windows keep answering while a million rows come in.
//!
//! - **Where it runs.** On the desktop (native builds and the interpreter)
//!   on the connection's own worker thread; in a browser, where there are
//!   no threads, on the page's thread in slices of a few milliseconds,
//!   the page getting its turn between them (the runtime's executor: the
//!   same code, `pause` awaited between chunks).
//! - **The rows** come in chunks: the first few dozen at once (they show
//!   soonest), then bigger ones, a chunk at least every slice of time. Each
//!   chunk is a frame built where the rows are read; the program's thread
//!   appends it to the frame under the component's name ([`poll`]), which
//!   an RDBGrid bound to the component draws — only the rows on screen —
//!   and FetchRow / Row(i) walk. RowCount follows.
//! - **The events** (fired on the program's thread, each to completion —
//!   runtimes never call into the VM: [`poll`] fires them through the
//!   host, which queues them): `OnRows(Count)` as rows arrive (the rows so
//!   far), then `OnDone(Rows, Ms)` (the rows and the milliseconds the
//!   query took) or `OnError(Message)`. The columns' names and types —
//!   the result's own metadata, never guessed — are ColName(i) / ColType(i)
//!   ([`meta`]). `Cancel` stops it: OnError
//!   "cancelled" — SQLite's interrupt and a check between rows; MySQL's
//!   `KILL QUERY` from a second connection.
//! - **One at a time** per component: QueryAsync, Query and QueryScalar
//!   while one runs are refused with OnError ("busy: …"), never waited for
//!   (the program would freeze).

use std::cell::RefCell;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rapidr_value::datascience::frame::{self, Frame, FrameBuilder};
use rapidr_value::{v_int, v_str};
use web_time::Instant;

use crate::Host;

/// What a component's query while a background one runs gets (OnError).
pub const BUSY: &str = "busy: a background query is running (Cancel it, or wait for OnDone)";

/// What OnError gets after Cancel.
pub const CANCELLED: &str = "cancelled";

/// A result's columns: their names and declared types.
pub type Columns = (Vec<String>, Vec<String>);

/// How a background query ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum End {
    Done,
    Failed(String),
    Cancelled,
}

/// What the query's producer (a worker thread, or the page's task) and the
/// program's thread share.
#[derive(Default)]
pub struct Shared {
    state: Mutex<Progress>,
    cancel: AtomicBool,
}

#[derive(Default)]
struct Progress {
    /// The current statement's columns (names, declared types), new since
    /// the program's thread last looked (a statement with result columns
    /// starts the rows again).
    columns: Option<Columns>,
    /// Chunks of rows not yet taken.
    chunks: Vec<Frame>,
    /// How it ended, and the milliseconds it took.
    end: Option<(End, u64)>,
}

impl Shared {
    pub fn new() -> Arc<Shared> {
        Arc::new(Shared::default())
    }

    fn lock(&self) -> MutexGuard<'_, Progress> {
        // (a producer that panicked left its rows: they still count)
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Cancel was asked for.
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// A statement's rows start (its columns' names and declared types):
    /// rows of one before dropped.
    pub fn start_rows(&self, columns: Vec<String>, types: Vec<String>) {
        let mut p = self.lock();
        p.columns = Some((columns, types));
        p.chunks.clear();
    }

    pub fn rows(&self, chunk: Frame) {
        self.lock().chunks.push(chunk);
    }

    pub fn end(&self, end: End, ms: u64) {
        self.lock().end = Some((end, ms));
    }

    /// What came since the last look: new columns, chunks, the end.
    fn take(&self) -> (Option<Columns>, Vec<Frame>, Option<(End, u64)>) {
        let mut p = self.lock();
        (p.columns.take(), std::mem::take(&mut p.chunks), p.end.take())
    }
}

/// The first chunk's rows (shown soonest), and the most a chunk has.
const FIRST_CHUNK: usize = 64;
const MAX_CHUNK: usize = 65_536;

/// A statement's rows gathered into chunks for [`Shared::rows`]: a chunk
/// is handed over when it has its target of rows (64, then twice as many
/// each time, up to 65,536) or a slice of time has passed since the last.
pub struct Chunker<'a> {
    shared: &'a Shared,
    builder: FrameBuilder,
    target: usize,
    since: Instant,
    slice: Duration,
}

impl<'a> Chunker<'a> {
    /// The statement's columns, their names and declared types (the rows
    /// start again).
    pub fn new(shared: &'a Shared, columns: Vec<String>, types: Vec<String>, slice: Duration) -> Chunker<'a> {
        let builder = FrameBuilder::new(&columns, FIRST_CHUNK);
        shared.start_rows(columns, types);
        Chunker { shared, builder, target: FIRST_CHUNK, since: Instant::now(), slice }
    }

    pub fn width(&self) -> usize {
        self.builder.width()
    }

    /// Cell `col` of the row being read (None: NULL).
    #[inline]
    pub fn push(&mut self, col: usize, cell: Option<&str>) {
        self.builder.push(col, cell);
    }

    /// The row is in. True when a chunk was handed over (the producer may
    /// pause, and look whether it's cancelled).
    pub fn end_row(&mut self) -> bool {
        self.builder.end_row();
        let n = self.builder.rows();
        // (the clock read every 64 rows: cheap, and a slice is still kept)
        if n >= self.target || (n.is_multiple_of(64) && self.since.elapsed() >= self.slice) {
            self.flush();
            return true;
        }
        false
    }

    /// The rows gathered, handed over (none: nothing).
    pub fn flush(&mut self) {
        if self.builder.rows() > 0 {
            let next = (self.target * 2).min(MAX_CHUNK);
            self.shared.rows(self.builder.finish(next));
            self.target = next;
        }
        self.since = Instant::now();
    }
}

/// How long a slice of a background query is: on the desktop's worker a
/// chunk at least this often; in a browser the page's thread is given back
/// after it.
pub fn slice() -> Duration {
    if cfg!(all(target_family = "wasm", target_os = "unknown")) {
        Duration::from_millis(8)
    } else {
        Duration::from_millis(20)
    }
}

/// Milliseconds since `start`.
pub fn ms_since(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

/// The time now (std's on the desktop; `performance.now()` in a browser).
pub fn now() -> Instant {
    Instant::now()
}

/// Runs a producer's future to its end on this thread (the desktop's
/// worker: its pauses are ready at once, so this is one poll).
pub fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// A pause that is over at once (the desktop's worker, the tests).
pub fn no_pause() -> std::future::Ready<()> {
    std::future::ready(())
}

// ---------------------------------------------------------------------------
// The program's side: the components' running queries
// ---------------------------------------------------------------------------

/// A component's running background query.
struct Active {
    /// The component's name (as the program wrote it) and key (lowercase:
    /// its frame's name).
    name: String,
    key: String,
    shared: Arc<Shared>,
    /// The rows appended so far, and how many OnRows last told.
    rows: usize,
    told: usize,
    /// How to stop the server's work too (MySQL's KILL QUERY), besides
    /// the flag.
    kill: Option<Box<dyn Fn()>>,
}

thread_local! {
    static ACTIVE: RefCell<Vec<Active>> = const { RefCell::new(Vec::new()) };
    /// Each component's background result's columns, as the database named
    /// and typed them (the frame's names make a repeated one unique: `id_2`).
    static META: RefCell<std::collections::HashMap<String, Columns>> = RefCell::new(std::collections::HashMap::new());
}

/// Component `key`'s background result's columns: their names and
/// declared types ("" where the database has none).
pub fn meta(key: &str) -> Option<Columns> {
    META.with(|m| m.borrow().get(key).cloned())
}

/// Whether component `key` has a background query running.
pub fn busy(key: &str) -> bool {
    ACTIVE.with(|a| a.borrow().iter().any(|q| q.key == key))
}

/// Whether any component has one (the window loop then polls often).
pub fn running() -> bool {
    ACTIVE.with(|a| !a.borrow().is_empty())
}

/// Component `name` starts a background query: its rows (and an RDBGrid
/// bound to it) are emptied, Busy is 1.
pub fn begin(host: &dyn Host, name: &str, key: &str, shared: Arc<Shared>, kill: Option<Box<dyn Fn()>>) {
    frame::set_frame(key, Frame::default());
    META.with(|m| m.borrow_mut().remove(key));
    ACTIVE.with(|a| a.borrow_mut().push(Active { name: name.to_string(), key: key.to_string(), shared, rows: 0, told: 0, kill }));
    host.set(name, "busy", v_int(1));
    host.set(name, "rowcount", v_int(0));
    host.row_changed(name);
}

/// `Cancel`: component `key`'s background query stops (OnError
/// "cancelled" comes with the next [`poll`]). Whether one was running.
pub fn cancel(key: &str) -> bool {
    ACTIVE.with(|a| {
        let a = a.borrow();
        let Some(q) = a.iter().find(|q| q.key == key) else { return false };
        q.shared.cancel();
        if let Some(kill) = &q.kill {
            kill();
        }
        true
    })
}

/// What one look at a running query found, to tell the program.
enum Heard {
    Columns(String, usize),
    Rows(String, usize),
    End(String, End, usize, u64),
}

/// The background queries' news, on the program's thread: new rows
/// appended to each component's frame (RowCount, ColCount; OnRows once a
/// look with the rows so far), and the queries that ended (Busy 0; OnDone
/// or OnError). Whether any is still running. The events are fired once
/// nothing is borrowed: a handler may start another query, or poll.
pub fn poll(host: &dyn Host) -> bool {
    if !running() {
        return false;
    }
    let mut heard = Vec::new();
    ACTIVE.with(|a| {
        let mut a = a.borrow_mut();
        let mut i = 0;
        while i < a.len() {
            let q = &mut a[i];
            let (columns, chunks, end) = q.shared.take();
            if let Some((columns, types)) = columns {
                let width = columns.len();
                frame::set_frame(&q.key, Frame::with_columns(&columns));
                META.with(|m| m.borrow_mut().insert(q.key.clone(), (columns, types)));
                q.rows = 0;
                heard.push(Heard::Columns(q.name.clone(), width));
            }
            if !chunks.is_empty() {
                frame::with_frame_mut(&q.key, |f| {
                    for c in &chunks {
                        f.append(c);
                    }
                    q.rows = f.height();
                });
            }
            if q.rows != q.told || !chunks.is_empty() {
                q.told = q.rows;
                heard.push(Heard::Rows(q.name.clone(), q.rows));
            }
            match end {
                Some((end, ms)) => {
                    let q = a.remove(i);
                    heard.push(Heard::End(q.name, end, q.rows, ms));
                }
                None => i += 1,
            }
        }
    });
    for h in heard {
        match h {
            Heard::Columns(name, width) => {
                host.set(&name, "colcount", v_int(width as i64));
                host.set(&name, "fieldcount", v_int(width as i64));
            }
            Heard::Rows(name, rows) => {
                host.set(&name, "rowcount", v_int(rows as i64));
                host.row_changed(&name);
                host.fire(&name, "onrows", &[v_int(rows as i64)]);
            }
            Heard::End(name, end, rows, ms) => {
                host.set(&name, "busy", v_int(0));
                match end {
                    End::Done => host.fire(&name, "ondone", &[v_int(rows as i64), v_int(ms as i64)]),
                    // (asked for: nothing to report)
                    End::Cancelled => host.fire(&name, "onerror", &[v_str(CANCELLED)]),
                    End::Failed(e) => {
                        host.report(&format!("[DB] {name}.QueryAsync error: {e}"));
                        host.fire(&name, "onerror", &[v_str(&e)]);
                    }
                }
            }
        }
    }
    running()
}

/// The component closed (or connected again): its running query is
/// cancelled and forgotten (no events follow).
pub fn forget(key: &str) {
    let gone = ACTIVE.with(|a| {
        let mut a = a.borrow_mut();
        let i = a.iter().position(|q| q.key == key)?;
        Some(a.remove(i))
    });
    if let Some(q) = gone {
        q.shared.cancel();
        if let Some(kill) = &q.kill {
            kill();
        }
    }
}

/// The refusal for a query on component `key` while its background one
/// runs (None: it may run).
pub fn refused(key: &str) -> Option<String> {
    busy(key).then(|| BUSY.to_string())
}
