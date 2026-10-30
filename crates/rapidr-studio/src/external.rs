//! RapidR Studio notices files changed outside it (lane EXTERNAL-CHANGES,
//! Preview 5's "Never lose anything"): another AI agent, an editor, `git
//! checkout` rewriting a file Studio has open. RPROJECT's `Watch`,
//! `Unwatch`, `ExternalChanges` and `ChangedOnDisk` are this module.
//!
//! Per watched file it remembers two stamps of the disk — the file's
//! modification time (to the nanosecond the system keeps), its size and a
//! cheap hash of its bytes:
//!
//! - **base**: the disk as Studio last read or wrote it (`watch`, after a
//!   load, a save, a reload). [`changed_since_base`] compares the disk with
//!   it: Save asks before it would overwrite newer content.
//! - **seen**: the disk as Studio last looked at it. [`check`] reports each
//!   file whose disk differs from it, once, and remembers what it saw — so
//!   an agent's 20 rewrites come back from one check together (Studio
//!   batches them into one note), and a change the person chose to keep
//!   theirs over is never reported again until the disk changes again.
//!
//! Comparing: a different size is a change; a different time with the same
//! size is checked by the hash (a `touch`, or `git checkout` writing the
//! same bytes back, is no change); the same time and size is no change —
//! unless the stamp was taken while the file's time was less than two
//! seconds old ("racy", as git calls it: a file system that keeps times to
//! the second or two could take a second write in the same tick), when the
//! hash decides, until the stamp is old enough.
//!
//! Where there is no disk (Studio in a browser: files live in the page's
//! store, and change only through Studio) nothing is watched: the checks
//! answer nothing.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// A file's time is "racy" for this long after it: a stamp taken that
/// soon is checked by its hash too.
const RACY_NS: i128 = 2_000_000_000;

/// Files bigger than this aren't hashed (a time change is then a change).
const HASH_LIMIT: u64 = 64 << 20;

/// What the disk had.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disk {
    /// No file there.
    Missing,
    File(Stamp),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    /// Modified, nanoseconds since 1970.
    pub modified: i128,
    pub size: u64,
    /// FNV-1a of the bytes (0 when not hashed: too big, unreadable).
    pub hash: u64,
    /// When the stamp was taken (nanoseconds since 1970).
    pub taken: i128,
}

impl Stamp {
    fn racy(&self) -> bool {
        self.taken - self.modified < RACY_NS
    }
}

/// What [`check`] found a file had become.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Its text changed.
    Changed,
    /// It isn't there any more.
    Deleted,
    /// It is there again (or there for the first time) after being missing.
    Back,
}

impl Change {
    pub fn word(self) -> &'static str {
        match self {
            Change::Changed => "changed",
            Change::Deleted => "deleted",
            Change::Back => "back",
        }
    }
}

struct Entry {
    /// The path as Studio gave it.
    path: String,
    base: Disk,
    seen: Disk,
}

/// The watched files.
#[derive(Default)]
pub struct Watcher {
    files: BTreeMap<String, Entry>,
}

/// The key a path is kept under: `/` separators; case folded where the
/// system's file names are (macOS, Windows).
fn key(path: &str) -> String {
    let p = path.replace('\\', "/");
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        p.to_lowercase()
    } else {
        p
    }
}

fn now_ns() -> i128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as i128)
}

/// FNV-1a, 64 bits: cheap, and only ever compared within one Studio.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The disk's time and size of `path` (`Some(None)`: no file there;
/// `None`: there is no disk to ask — the web).
fn stat(path: &str) -> Option<Option<(i128, u64)>> {
    if cfg!(target_arch = "wasm32") || path.is_empty() {
        return None;
    }
    Some(std::fs::metadata(path).ok().filter(std::fs::Metadata::is_file).map(|m| {
        let modified = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos() as i128);
        (modified, m.len())
    }))
}

fn hash_of(path: &str, size: u64) -> u64 {
    if size > HASH_LIMIT {
        return 0;
    }
    std::fs::read(path).map_or(0, |b| fnv1a(&b))
}

/// The disk now (`None`: no disk).
fn disk_now(path: &str) -> Option<Disk> {
    Some(match stat(path)? {
        None => Disk::Missing,
        Some((modified, size)) => Disk::File(Stamp { modified, size, hash: hash_of(path, size), taken: now_ns() }),
    })
}

/// The disk compared with `old`: what it has now, and whether that differs
/// (`None`: no disk). Hashes only when the time moved, or the old stamp
/// is racy.
fn probe(old: &Disk, path: &str) -> Option<(Disk, bool)> {
    let now = stat(path)?;
    Some(match (old, now) {
        (Disk::Missing, None) => (Disk::Missing, false),
        (Disk::File(_), None) => (Disk::Missing, true),
        (Disk::Missing, Some((modified, size))) => (Disk::File(Stamp { modified, size, hash: hash_of(path, size), taken: now_ns() }), true),
        (Disk::File(o), Some((modified, size))) => {
            if o.size != size {
                (Disk::File(Stamp { modified, size, hash: hash_of(path, size), taken: now_ns() }), true)
            } else if o.modified != modified || o.racy() {
                let hash = hash_of(path, size);
                let same = hash == o.hash && hash != 0;
                (Disk::File(Stamp { modified, size, hash, taken: now_ns() }), !same && (hash != 0 || o.modified != modified))
            } else {
                (*old, false)
            }
        }
    })
}

impl Watcher {
    /// Remembers the disk's `path` as Studio's own now (it was just read or
    /// written): its base and what was seen. False where there's no disk.
    pub fn watch(&mut self, path: &str) -> bool {
        let Some(d) = disk_now(path) else { return false };
        self.files.insert(key(path), Entry { path: path.to_string(), base: d, seen: d });
        true
    }

    /// Forgets `path` (its document closed).
    pub fn unwatch(&mut self, path: &str) {
        self.files.remove(&key(path));
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }

    pub fn watching(&self, path: &str) -> bool {
        self.files.contains_key(&key(path))
    }

    /// Every watched file whose disk changed since it was last seen, each
    /// reported once — all of one check together, in path order.
    pub fn check(&mut self) -> Vec<(Change, String)> {
        let mut out = Vec::new();
        for e in self.files.values_mut() {
            let Some((now, differs)) = probe(&e.seen, &e.path) else { continue };
            let before = e.seen;
            // (a racy stamp confirmed is kept as now: no longer racy once old enough)
            e.seen = now;
            if differs {
                let change = match (before, now) {
                    (_, Disk::Missing) => Change::Deleted,
                    (Disk::Missing, _) => Change::Back,
                    _ => Change::Changed,
                };
                out.push((change, e.path.clone()));
            }
        }
        out
    }

    /// Whether the disk's `path` differs from what Studio last read or
    /// wrote (a file never watched: false).
    pub fn changed_since_base(&self, path: &str) -> bool {
        let Some(e) = self.files.get(&key(path)) else { return false };
        probe(&e.base, &e.path).is_some_and(|(_, differs)| differs)
    }

    /// Whether the disk had no `path` when last seen (deleted outside).
    pub fn missing(&self, path: &str) -> bool {
        self.files.get(&key(path)).is_some_and(|e| e.seen == Disk::Missing)
    }
}

thread_local! {
    static WATCHER: std::cell::RefCell<Watcher> = std::cell::RefCell::new(Watcher::default());
}

/// Studio's watcher (one per Studio: RPROJECT's methods use it).
pub fn with<R>(f: impl FnOnce(&mut Watcher) -> R) -> R {
    WATCHER.with(|w| f(&mut w.borrow_mut()))
}

/// [`Watcher::check`] as RPROJECT.ExternalChanges says it: a line each,
/// what happened and the path, separated by a tab.
pub fn changes_text(changes: &[(Change, String)]) -> String {
    changes.iter().map(|(c, p)| format!("{}\t{p}\n", c.word())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::time::Duration;

    /// A file written with its time set to `secs` after 2020 (old enough
    /// never to be racy).
    fn write_at(path: &str, text: &str, secs: u64) {
        fs::write(path, text).unwrap();
        let t = UNIX_EPOCH + Duration::from_secs(1_577_836_800 + secs);
        File::options().write(true).open(path).unwrap().set_modified(t).unwrap();
    }

    fn dir() -> (tempfile::TempDir, String) {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().to_string_lossy().replace('\\', "/");
        (d, p)
    }

    #[test]
    fn a_change_is_reported_once() {
        let (_d, f) = dir();
        let main = format!("{f}/main.rr");
        write_at(&main, "PRINT 1\n", 10);
        let mut w = Watcher::default();
        assert!(w.watch(&main));
        assert!(w.check().is_empty(), "nothing changed");
        write_at(&main, "PRINT 22\n", 20);
        assert_eq!(w.check(), vec![(Change::Changed, main.clone())]);
        assert!(w.check().is_empty(), "reported once");
        assert!(w.changed_since_base(&main), "Save still asks: the disk is newer than what Studio read");
        // (Studio reloads it, or saves it: its own again)
        assert!(w.watch(&main));
        assert!(!w.changed_since_base(&main));
        assert_eq!(changes_text(&[(Change::Changed, main.clone())]), format!("changed\t{main}\n"));
    }

    #[test]
    fn same_size_new_time_is_hashed() {
        let (_d, f) = dir();
        let main = format!("{f}/main.rr");
        write_at(&main, "PRINT 1\n", 10);
        let mut w = Watcher::default();
        w.watch(&main);
        // (touched: the same bytes, a new time — git checkout writing them back)
        write_at(&main, "PRINT 1\n", 30);
        assert!(w.check().is_empty(), "a touch is no change");
        assert!(!w.changed_since_base(&main));
        // (the same size, other bytes, a new time)
        write_at(&main, "PRINT 2\n", 40);
        assert_eq!(w.check(), vec![(Change::Changed, main.clone())]);
    }

    #[test]
    fn same_time_and_size_trusted_unless_racy() {
        let (_d, f) = dir();
        let main = format!("{f}/main.rr");
        write_at(&main, "PRINT 1\n", 10);
        let mut w = Watcher::default();
        w.watch(&main);
        // (other bytes under the very same time and size: a stamp old
        // enough trusts the time — no file read every two seconds)
        write_at(&main, "PRINT 2\n", 10);
        assert!(w.check().is_empty(), "an old stamp: the time and size decide");
        // (a stamp taken while the time was fresh — a file system keeping
        // seconds could take a second write in the same tick: the hash decides)
        fs::write(&main, "PRINT 3\n").unwrap();
        let mut w = Watcher::default();
        w.watch(&main);
        let modified = fs::metadata(&main).unwrap().modified().unwrap();
        fs::write(&main, "PRINT 4\n").unwrap();
        File::options().write(true).open(&main).unwrap().set_modified(modified).unwrap();
        assert_eq!(w.check(), vec![(Change::Changed, main.clone())], "racy: the hash saw it");
    }

    #[test]
    fn a_bigger_file_under_the_same_time_is_a_change() {
        let (_d, f) = dir();
        let main = format!("{f}/main.rr");
        write_at(&main, "PRINT 1\n", 10);
        let mut w = Watcher::default();
        w.watch(&main);
        write_at(&main, "PRINT 1\nPRINT 2\n", 10);
        assert_eq!(w.check(), vec![(Change::Changed, main)]);
    }

    #[test]
    fn deleted_then_back() {
        let (_d, f) = dir();
        let greet = format!("{f}/greet.rr");
        write_at(&greet, "SUB Greet\nEND SUB\n", 10);
        let mut w = Watcher::default();
        w.watch(&greet);
        fs::remove_file(&greet).unwrap();
        assert_eq!(w.check(), vec![(Change::Deleted, greet.clone())]);
        assert!(w.missing(&greet));
        assert!(w.check().is_empty());
        assert!(w.changed_since_base(&greet), "Save asks before it makes it again");
        write_at(&greet, "SUB Greet\nEND SUB\n", 50);
        assert_eq!(w.check(), vec![(Change::Back, greet.clone())]);
        assert!(!w.missing(&greet));
        assert!(!w.changed_since_base(&greet), "back as Studio had it");
    }

    #[test]
    fn studios_own_writes_say_nothing() {
        let (_d, f) = dir();
        let main = format!("{f}/main.rr");
        write_at(&main, "PRINT 1\n", 10);
        let mut w = Watcher::default();
        w.watch(&main);
        // (Save, RAI's Apply then Save, an AI app's change saved: Studio
        // writes, then watches what it wrote — RPROJECT.Watch after the write)
        fs::write(&main, "PRINT 1\nPRINT \"saved by Studio\"\n").unwrap();
        w.watch(&main);
        assert!(w.check().is_empty(), "Studio's own write is no outside change");
        assert!(!w.changed_since_base(&main));
        // (a file never watched — a build's output — is never reported)
        write_at(&format!("{f}/out.txt"), "x", 10);
        assert!(w.check().is_empty());
        assert!(!w.changed_since_base(&format!("{f}/out.txt")));
        // (closed: forgotten)
        w.unwatch(&main);
        write_at(&main, "PRINT 9\n", 99);
        assert!(w.check().is_empty());
        assert!(!w.watching(&main));
    }

    #[test]
    fn many_changes_come_back_from_one_check() {
        let (_d, f) = dir();
        let mut w = Watcher::default();
        let files: Vec<String> = (0..12).map(|i| format!("{f}/file{i:02}.rr")).collect();
        for p in &files {
            write_at(p, "PRINT 1\n", 10);
            w.watch(p);
        }
        // (an agent rewrites twelve files, git checkout another branch)
        for p in &files {
            write_at(p, "PRINT \"rewritten\"\n", 20);
        }
        let found = w.check();
        assert_eq!(found.len(), 12, "one check: all twelve, for one note");
        assert!(found.iter().all(|(c, _)| *c == Change::Changed));
        assert_eq!(found.iter().map(|(_, p)| p.clone()).collect::<Vec<_>>(), files, "in path order");
        assert!(w.check().is_empty());
    }

    #[test]
    fn no_disk_no_watch() {
        // (no path, as the web's store has no disk to ask: nothing watched,
        // nothing said)
        let mut w = Watcher::default();
        assert!(!w.watch(""));
        assert!(w.check().is_empty());
        assert!(!w.changed_since_base(""));
    }

    #[test]
    fn fnv_is_stable() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_ne!(fnv1a(b"PRINT 1\n"), fnv1a(b"PRINT 2\n"));
    }
}
