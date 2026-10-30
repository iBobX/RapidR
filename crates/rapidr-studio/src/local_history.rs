//! **Local history** (Versions G3, docs/versions-plan.md §1 "nothing is ever
//! lost"): every save keeps a copy of the file, before the project has
//! versions and after, on the desktop and on the web — RREPOSITORY's
//! `Snapshot`, `LocalHistory` and `SnapshotText`.
//!
//! * **Where**: the project's own folder, `.rapidr/history/` (Studio's
//!   folder in a project: never part of a version — rapidr-vcs leaves
//!   `.rapidr/` out of every save, safety point and checkpoint, and
//!   "Start versioning" puts it in the project's `.gitignore`), read and
//!   written through the runtime's file hooks: the disk on the desktop,
//!   the page's store on the web.
//! * **What**: `index.jsonl`, one line per snapshot (its number, the
//!   file's path relative to the project — `/` separated —, when, why, its
//!   size and its content's SHA-256), and `blobs/<sha-256>`, each content
//!   once however many snapshots have it. A snapshot equal to the file's
//!   newest one isn't taken again. A blob is checked against its digest
//!   when it is read back: a damaged copy is said, never shown as the file.
//! * **How much**: the newest [`KEEP_PER_FILE`] snapshots of each file; a
//!   file over [`MAX_BYTES`] isn't copied (said). Older snapshots go, and
//!   their blobs with them when no other snapshot has them (the desktop;
//!   the web's store keeps them until the page's data is cleared).
//! * **For other parts of Studio**: [`snapshot`] (the bytes given) and
//!   [`snapshot_file`] (what the disk has now) are the API the
//!   external-changes work calls before it reloads a file changed outside
//!   Studio (`why` = [`WHY_BEFORE_RELOAD`]), so the text that was there
//!   can always be found again.

use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};

use crate::{join, slashes};

/// The newest snapshots kept of each file.
pub const KEEP_PER_FILE: usize = 100;
/// A file larger than this isn't copied (a big data file: its versions
/// are the place for it).
pub const MAX_BYTES: usize = 8 << 20;

/// Why a snapshot was taken (the `why` it is kept with; the UI says it in
/// words).
pub const WHY_SAVED: &str = "saved";
/// The file as the disk had it before Studio first wrote it (or after
/// another program changed it).
pub const WHY_ON_DISK: &str = "on-disk";
/// Before reloading a file another program changed (the external-changes
/// work).
pub const WHY_BEFORE_RELOAD: &str = "before-reload";
/// Before a snapshot was put back in the editor.
pub const WHY_BEFORE_RESTORE: &str = "before-restore";

type Clock = fn() -> i64;

fn system_now() -> i64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
}

thread_local! {
    static CLOCK: std::cell::Cell<Clock> = const { std::cell::Cell::new(system_now) };
}

/// The runtime's clock, in seconds since 1970 (the web's: the browser's
/// Date, installed by the web runtime; the desktop's own by default).
pub fn set_clock(now: Clock) {
    CLOCK.with(|c| c.set(now));
}

/// Now, in seconds since 1970 (the runtime's clock).
pub fn now() -> i64 {
    CLOCK.with(|c| (c.get())())
}

/// One snapshot of one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// Its number in the project's history (from 1): its id.
    pub n: u64,
    /// The file, relative to the project's folder (`/` separated; an
    /// absolute path for a file outside it).
    pub file: String,
    pub seconds: i64,
    /// [`WHY_SAVED`], [`WHY_ON_DISK`], [`WHY_BEFORE_RELOAD`] …
    pub why: String,
    pub size: usize,
    /// The content's SHA-256, hex.
    pub digest: String,
}

impl Snapshot {
    fn to_json(&self) -> String {
        json!({"n": self.n, "file": self.file, "seconds": self.seconds, "why": self.why, "size": self.size, "sha256": self.digest}).to_string()
    }

    fn from_json(line: &str) -> Option<Snapshot> {
        let v: Json = serde_json::from_str(line).ok()?;
        Some(Snapshot {
            n: v.get("n")?.as_u64()?,
            file: v.get("file")?.as_str()?.to_string(),
            seconds: v.get("seconds")?.as_i64()?,
            why: v.get("why").and_then(Json::as_str).unwrap_or(WHY_SAVED).to_string(),
            size: v.get("size").and_then(Json::as_u64).unwrap_or(0) as usize,
            digest: v.get("sha256")?.as_str()?.to_string(),
        })
    }
}

/// The history's folder in project `folder`.
pub fn history_folder(folder: &str) -> String {
    join(&join(folder, ".rapidr"), "history")
}

fn index_path(folder: &str) -> String {
    join(&history_folder(folder), "index.jsonl")
}

fn blob_path(folder: &str, digest: &str) -> String {
    join(&join(&history_folder(folder), "blobs"), digest)
}

fn digest_of(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// `path` as the history keys it: relative to `folder` when it is inside
/// it, `/` separated.
pub fn key_of(folder: &str, path: &str) -> String {
    let p = slashes(path);
    let f = slashes(folder);
    let f = f.trim_end_matches('/');
    if !f.is_empty() {
        if let Some(rest) = p.strip_prefix(f).and_then(|r| r.strip_prefix('/')) {
            return rest.to_string();
        }
    }
    p
}

fn read_index(folder: &str) -> Vec<Snapshot> {
    let Some(text) = crate::recovery::read_if_there(&index_path(folder)) else { return Vec::new() };
    // (a line cut short by a crash is skipped, not fatal)
    text.lines().filter_map(Snapshot::from_json).collect()
}

fn write_index(folder: &str, all: &[Snapshot]) -> Result<(), String> {
    let mut text = String::new();
    for s in all {
        text.push_str(&s.to_json());
        text.push('\n');
    }
    crate::write_text(&index_path(folder), &text)
}

fn make_folders(folder: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::fs::create_dir_all(join(&history_folder(folder), "blobs"));
        // (what the folder is, for whoever finds it)
        let readme = join(&history_folder(folder), "README.txt");
        if !std::path::Path::new(&readme).exists() {
            let _ = std::fs::write(
                &readme,
                "RapidR Studio's local history of this project: a copy of each file every time it was saved,\n\
                 so nothing is ever lost (Versions > Local History in RapidR Studio).\n\
                 It is never part of a version. You can delete this folder: only the copies go.\n",
            );
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = folder;
    }
}

fn remove_blob(folder: &str, digest: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::fs::remove_file(blob_path(folder, digest));
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (folder, digest);
    }
}

/// Keeps `bytes` as file `path`'s newest snapshot in project `folder`'s
/// history, `why` it was taken. `Ok(None)`: not taken — it equals the
/// file's newest one. An error says why in words (a file too big, a disk
/// that refused).
pub fn snapshot(folder: &str, path: &str, bytes: &[u8], why: &str) -> Result<Option<Snapshot>, String> {
    if folder.trim().is_empty() {
        return Err("there is no project folder to keep the history in".into());
    }
    if bytes.len() > MAX_BYTES {
        return Err(format!("{} is bigger than {} MB: its local history isn't kept (save versions of it instead)", key_of(folder, path), MAX_BYTES >> 20));
    }
    let file = key_of(folder, path);
    let digest = digest_of(bytes);
    let mut all = read_index(folder);
    if all.iter().rev().find(|s| s.file == file).is_some_and(|s| s.digest == digest) {
        return Ok(None);
    }
    make_folders(folder);
    if !all.iter().any(|s| s.digest == digest) {
        rapidr_value::objects::write_file(&blob_path(folder, &digest), bytes)?;
    }
    let n = all.iter().map(|s| s.n).max().unwrap_or(0) + 1;
    let snap = Snapshot { n, file: file.clone(), seconds: now(), why: why.to_string(), size: bytes.len(), digest };
    all.push(snap.clone());
    // (the newest KEEP_PER_FILE of this file; the blobs no snapshot has any more go)
    let mine: Vec<u64> = all.iter().filter(|s| s.file == file).map(|s| s.n).collect();
    if mine.len() > KEEP_PER_FILE {
        let drop: Vec<u64> = mine[..mine.len() - KEEP_PER_FILE].to_vec();
        let gone: Vec<String> = all.iter().filter(|s| drop.contains(&s.n)).map(|s| s.digest.clone()).collect();
        all.retain(|s| !drop.contains(&s.n));
        for d in gone {
            if !all.iter().any(|s| s.digest == d) {
                remove_blob(folder, &d);
            }
        }
    }
    write_index(folder, &all)?;
    Ok(Some(snap))
}

/// Keeps what the disk (the web's store) has now at `path` (see
/// [`snapshot`]): what to call before something replaces the file — a
/// reload, a restore. `Ok(None)` when the file isn't there or equals its
/// newest snapshot.
pub fn snapshot_file(folder: &str, path: &str, why: &str) -> Result<Option<Snapshot>, String> {
    let Some(bytes) = crate::recovery::read_bytes_if_there(path) else { return Ok(None) };
    snapshot(folder, path, &bytes, why)
}

/// File `path`'s snapshots in project `folder`, the newest first.
pub fn list(folder: &str, path: &str) -> Vec<Snapshot> {
    let file = key_of(folder, path);
    let mut out: Vec<Snapshot> = read_index(folder).into_iter().filter(|s| s.file == file).collect();
    out.reverse();
    out
}

/// Every file with snapshots in project `folder` (as [`key_of`] names
/// them), in the order they were first kept.
pub fn files(folder: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in read_index(folder) {
        if !out.contains(&s.file) {
            out.push(s.file);
        }
    }
    out
}

/// Snapshot `n`'s bytes (checked against its digest).
pub fn read(folder: &str, n: u64) -> Result<Vec<u8>, String> {
    let Some(s) = read_index(folder).into_iter().find(|s| s.n == n) else {
        return Err(format!("there is no snapshot {n} in the local history"));
    };
    let bytes = rapidr_value::objects::read_file(&blob_path(folder, &s.digest)).map_err(|_| format!("the copy of {} kept at snapshot {n} is missing", s.file))?;
    if digest_of(&bytes) != s.digest {
        return Err(format!("the copy of {} kept at snapshot {n} is damaged, so it isn't shown", s.file));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder() -> (tempfile::TempDir, String) {
        let t = tempfile::tempdir().unwrap();
        let f = slashes(&t.path().to_string_lossy());
        (t, f)
    }

    #[test]
    fn saves_are_kept_once_each_newest_first() {
        let (_t, f) = folder();
        let main = format!("{f}/main.rr");
        assert_eq!(snapshot(&f, &main, b"PRINT 1\n", WHY_ON_DISK).unwrap().unwrap().n, 1);
        assert!(snapshot(&f, &main, b"PRINT 1\n", WHY_SAVED).unwrap().is_none(), "the same again: not taken");
        snapshot(&f, &main, b"PRINT 2\n", WHY_SAVED).unwrap().unwrap();
        snapshot(&f, "other.rr", b"PRINT 1\n", WHY_SAVED).unwrap().unwrap();
        snapshot(&f, &main, b"PRINT 3\n", WHY_SAVED).unwrap().unwrap();
        let l = list(&f, "main.rr");
        assert_eq!(l.iter().map(|s| s.n).collect::<Vec<_>>(), [4, 2, 1]);
        assert_eq!(l[2].why, WHY_ON_DISK);
        assert_eq!(read(&f, 2).unwrap(), b"PRINT 2\n");
        assert_eq!(files(&f), ["main.rr", "other.rr"]);
        // (the same content once: other.rr's snapshot shares main.rr's first blob)
        let blobs = std::fs::read_dir(format!("{f}/.rapidr/history/blobs")).unwrap().count();
        assert_eq!(blobs, 3);
        // a damaged copy is said, never shown
        std::fs::write(blob_path(&f, &l[0].digest), b"PRINT 666\n").unwrap();
        assert!(read(&f, 4).unwrap_err().contains("damaged"));
        assert!(read(&f, 99).is_err());
        assert!(std::path::Path::new(&format!("{f}/.rapidr/history/README.txt")).exists());
    }

    #[test]
    fn only_the_newest_are_kept_and_big_files_are_said() {
        let (_t, f) = folder();
        for i in 0..KEEP_PER_FILE + 5 {
            snapshot(&f, "a.rr", format!("{i}\n").as_bytes(), WHY_SAVED).unwrap().unwrap();
        }
        let l = list(&f, "a.rr");
        assert_eq!(l.len(), KEEP_PER_FILE);
        assert_eq!(read(&f, l[0].n).unwrap(), format!("{}\n", KEEP_PER_FILE + 4).as_bytes());
        assert_eq!(std::fs::read_dir(format!("{f}/.rapidr/history/blobs")).unwrap().count(), KEEP_PER_FILE, "the old copies went");
        let big = vec![b'x'; MAX_BYTES + 1];
        assert!(snapshot(&f, "big.csv", &big, WHY_SAVED).unwrap_err().contains("bigger than"));
        // what the disk has now
        std::fs::write(format!("{f}/disk.rr"), "on disk\n").unwrap();
        assert_eq!(snapshot_file(&f, &format!("{f}/disk.rr"), WHY_BEFORE_RELOAD).unwrap().unwrap().why, WHY_BEFORE_RELOAD);
        assert!(snapshot_file(&f, &format!("{f}/none.rr"), WHY_BEFORE_RELOAD).unwrap().is_none());
        assert_eq!(key_of("/p/x", "/p/x/sub/a.rr"), "sub/a.rr");
        assert_eq!(key_of("/p/x", "/elsewhere/a.rr"), "/elsewhere/a.rr");
    }
}
