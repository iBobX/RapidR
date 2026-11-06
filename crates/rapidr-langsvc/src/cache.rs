//! One parse per program version, whoever asks (docs/speed-audit-2026-11.md,
//! "Studio: the edit pause"): RapidR Studio's code editor and its Outline
//! and Problems panes each have an [`Analysis`](crate::Analysis) of the
//! same texts, and ask for the same program after a pause in typing. The
//! snapshot made for one is kept here (a few, the latest first), and
//! another analysis with the very same input takes it — the parse, the
//! names and, made once on it, the compiler's checks
//! ([`Snapshot::compiled`](crate::Snapshot::compiled)).
//!
//! The input is everything the parse reads: the main file's path and text,
//! the editor's text of every open file, the preprocessor's options and the
//! user's include folders — compared whole, never by a hash alone — and the
//! files it read from the disk (or the web page's store), read again and
//! compared before a kept snapshot is taken: an `$INCLUDE` changed outside
//! Studio is parsed again. A program whose preprocessing failed (an
//! `$INCLUDE` not found, which may be there at the next try) is never kept.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rapidr_preprocessor::PreprocessOptions;

use crate::Snapshot;

/// How many programs are kept (the editor's and the panes' of the active
/// one, a document or two switched between: a big program's parse is tens
/// of megabytes).
const KEEP: usize = 3;

struct Kept {
    root: PathBuf,
    text: String,
    options: String,
    open: Vec<(PathBuf, String)>,
    /// The files read from the disk (or the page's store) and their text.
    read: Vec<(PathBuf, String)>,
    snapshot: Arc<Snapshot>,
}

static KEPT: Mutex<Vec<Kept>> = Mutex::new(Vec::new());

/// Whether parses are shared (the tests compare with every analysis made
/// afresh: [`crate::background::share_parses`]).
static SHARED: AtomicBool = AtomicBool::new(true);

pub(crate) fn set_shared(on: bool) {
    SHARED.store(on, Ordering::SeqCst);
    if !on {
        if let Ok(mut kept) = KEPT.lock() {
            kept.clear();
        }
    }
}

/// The options' part of the input, as text (the include folders the user
/// configured too: the parse adds them).
fn options_key(options: &PreprocessOptions) -> String {
    format!("{options:?}|{:?}", rapidr_preprocessor::configured_include_dirs())
}

/// The snapshot of `root` (its text `text`, the open files `open` in the
/// paths' order) with `options`: one kept for the same input, else `make`'s.
pub(crate) fn snapshot(
    root: &Path,
    text: String,
    options: &PreprocessOptions,
    open: Vec<(PathBuf, String)>,
    make: impl FnOnce(&Path, &str, &PreprocessOptions, &[(PathBuf, String)]) -> Snapshot,
) -> Arc<Snapshot> {
    if !SHARED.load(Ordering::SeqCst) {
        return Arc::new(make(root, &text, options, &open));
    }
    let key = options_key(options);
    if let Some(s) = find(root, &text, &key, &open) {
        return s;
    }
    let s = Arc::new(make(root, &text, options, &open));
    let tools = &s.parsed.tools;
    if tools.preprocessor_diagnostics == 0 {
        let read = tools
            .preprocessed
            .origins
            .files
            .iter()
            .filter_map(|f| {
                let p = f.path.as_ref()?;
                let from_editor = p.as_path() == root || open.iter().any(|(o, _)| o == p);
                (!from_editor && !p.starts_with("<RapidR>")).then(|| (p.clone(), f.text.clone()))
            })
            .collect();
        if let Ok(mut kept) = KEPT.lock() {
            kept.retain(|k| !(k.root == root && k.text == text && k.options == key && k.open == open));
            kept.insert(0, Kept { root: root.to_path_buf(), text, options: key, open, read, snapshot: s.clone() });
            kept.truncate(KEEP);
        }
    }
    s
}

fn find(root: &Path, text: &str, key: &str, open: &[(PathBuf, String)]) -> Option<Arc<Snapshot>> {
    let mut kept = KEPT.lock().ok()?;
    let at = kept.iter().position(|k| k.root == root && k.text == text && k.options == key && k.open == open)?;
    // (the files it read, as they are now: one changed — or gone — and the
    // program is parsed again)
    let same = kept[at].read.iter().all(|(p, t)| rapidr_preprocessor::read_source(p).is_ok_and(|now| now == *t));
    if !same {
        kept.remove(at);
        return None;
    }
    let k = kept.remove(at);
    let s = k.snapshot.clone();
    kept.insert(0, k);
    Some(s)
}

