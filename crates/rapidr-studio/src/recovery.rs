//! Studio keeps your work (RPROJECT's Recovery… methods): a copy of every
//! document with unsaved changes, kept in Studio's own folder as you work
//! (and the moment something goes wrong in Studio), so that even when
//! Studio is ended — a crash, a kill, the computer switched off — the next
//! start offers "Restore unsaved work?". A clean exit removes the copies.
//!
//! * **Where**: Studio's folder ([`studio_folder`]), never the project's:
//!   `RAPIDR_STUDIO_DATA` when set (the tests'); else beside the settings
//!   store (`RAPIDR_REGISTRY`'s folder, `studio/`); else RapidR's folder of
//!   the user's settings (`~/Library/Application Support/RapidR` on macOS,
//!   `%APPDATA%\RapidR` on Windows, `~/.config/rapidr` on Linux). On the
//!   web, the page's store (`/rapidr-studio-data`), which the page keeps
//!   in the browser's private file system.
//! * **What**: `recovery/<run>/index.json` (each document's key, its path
//!   — "" for one never saved —, its caption) and one text file each. A
//!   run is this Studio's (its process id on the desktop; the page's on the
//!   web); another Studio running at the same time keeps its own, and its
//!   copies are never offered while it runs.
//! * **The log**: `studio-errors.log` beside it — what went wrong in
//!   Studio's own code, with the file and line, a line each (kept short:
//!   the last 256 KB).

use serde_json::{json, Value as Json};

use crate::{join, read_text, slashes, write_text};

/// The log's size it is kept under.
const LOG_MAX: usize = 256 * 1024;

thread_local! {
    /// Whether the page's store has a file (the web: its files listed by
    /// the host — a file read that isn't there asks the site for it, and
    /// the console says 404).
    static STORE_HAS: std::cell::RefCell<Option<Box<dyn Fn(&str) -> bool>>> = const { std::cell::RefCell::new(None) };
}

/// The host's way to tell whether a file is there without reading it
/// (RPROJECT gives its host's, `Host::list_files`).
pub fn set_store_has(f: Box<dyn Fn(&str) -> bool>) {
    STORE_HAS.with(|s| *s.borrow_mut() = Some(f));
}

/// Whether the host's way is known already.
pub fn store_known() -> bool {
    STORE_HAS.with(|s| s.borrow().is_some())
}

/// A file's text when it is there (on the web: asked of the store first,
/// never of the site).
pub(crate) fn read_if_there(path: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    if STORE_HAS.with(|s| s.borrow().as_ref().is_some_and(|has| !has(path))) {
        return None;
    }
    read_text(path).ok()
}

/// A file's bytes when it is there (as [`read_if_there`]).
pub(crate) fn read_bytes_if_there(path: &str) -> Option<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    if STORE_HAS.with(|s| s.borrow().as_ref().is_some_and(|has| !has(path))) {
        return None;
    }
    rapidr_value::objects::read_file(path).ok()
}

/// Studio's own folder (`/` separators, no trailing `/`).
pub fn studio_folder() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        "/rapidr-studio-data".to_string()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::path::PathBuf;
        let dir = if let Some(p) = std::env::var_os("RAPIDR_STUDIO_DATA").filter(|p| !p.is_empty()) {
            PathBuf::from(p)
        } else if let Some(r) = std::env::var_os("RAPIDR_REGISTRY").filter(|p| !p.is_empty()) {
            let r = PathBuf::from(r);
            r.parent().map(|d| d.join("studio")).unwrap_or_else(|| PathBuf::from("studio"))
        } else {
            let base = if cfg!(windows) {
                std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("RapidR"))
            } else if cfg!(target_os = "macos") {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library").join("Application Support").join("RapidR"))
            } else {
                let home = std::env::var_os("HOME").map(PathBuf::from);
                std::env::var_os("XDG_CONFIG_HOME").filter(|p| !p.is_empty()).map(PathBuf::from).or_else(|| home.map(|h| h.join(".config"))).map(|c| c.join("rapidr"))
            };
            base.unwrap_or_else(|| std::env::temp_dir().join("rapidr-studio"))
        };
        slashes(&dir.to_string_lossy()).trim_end_matches('/').to_string()
    }
}

/// This Studio's run: its copies are kept under `recovery/<run>`.
fn this_run() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        "page".to_string()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::process::id().to_string()
    }
}

fn recovery_root() -> String {
    join(&studio_folder(), "recovery")
}

fn run_folder(run: &str) -> String {
    join(&recovery_root(), run)
}

/// A run's documents: (key, path, caption), as its index says.
fn index_of(run: &str) -> Vec<(String, String, String)> {
    let Some(text) = read_if_there(&join(&run_folder(run), "index.json")) else { return Vec::new() };
    let Ok(Json::Array(items)) = serde_json::from_str::<Json>(&text) else { return Vec::new() };
    items
        .iter()
        .filter_map(|i| Some((i.get("key")?.as_str()?.to_string(), i.get("path").and_then(Json::as_str).unwrap_or("").to_string(), i.get("caption").and_then(Json::as_str).unwrap_or("").to_string())))
        .collect()
}

fn write_index(run: &str, docs: &[(String, String, String)]) -> Result<(), String> {
    let items: Vec<Json> = docs.iter().map(|(k, p, c)| json!({"key": k, "path": p, "caption": c})).collect();
    write_text(&join(&run_folder(run), "index.json"), &Json::Array(items).to_string())
}

/// The file a document's copy is in (its key made a file name).
fn copy_file(run: &str, key: &str) -> String {
    let safe: String = key.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).take(80).collect();
    // (two keys that make the same name stay apart: a short digest)
    let digest = key.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
    join(&run_folder(run), &format!("{safe}-{:08x}.txt", digest as u32))
}

pub(crate) fn make_folder(path: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::fs::create_dir_all(path);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
    }
}

/// Keeps a copy of document `key` (`path`: its file, "" for a new one never
/// saved; `caption`: as its tab says it) with `text`, the unsaved text.
pub fn keep(key: &str, path: &str, caption: &str, text: &str) -> Result<(), String> {
    let run = this_run();
    make_folder(&run_folder(&run));
    write_text(&copy_file(&run, key), text)?;
    let mut docs = index_of(&run);
    docs.retain(|(k, _, _)| k != key);
    docs.push((key.to_string(), slashes(path), caption.to_string()));
    write_index(&run, &docs)
}

/// Forgets document `key`'s copy (it was saved, or closed and its changes
/// let go).
pub fn forget(key: &str) {
    let run = this_run();
    let mut docs = index_of(&run);
    let before = docs.len();
    docs.retain(|(k, _, _)| k != key);
    if docs.len() != before {
        let _ = write_index(&run, &docs);
        remove(&copy_file(&run, key));
    }
}

/// Studio closed as it should (every change saved or let go): this run's
/// copies go.
pub fn end() {
    let run = this_run();
    let _ = write_index(&run, &[]);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::fs::remove_dir_all(run_folder(&run));
    }
}

fn remove(path: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::fs::remove_file(path);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
    }
}

/// Whether run `run` (another Studio's) is still going: a process id
/// that's alive (Unix); elsewhere, its index changed in the last two
/// minutes (Windows). The page's runs on the web are over (a new page).
fn running(run: &str) -> bool {
    if run == this_run() {
        return true;
    }
    #[cfg(target_arch = "wasm32")]
    {
        false
    }
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    {
        match run.parse::<i32>() {
            // SAFETY: kill with signal 0 only asks whether the process exists.
            Ok(pid) if pid > 0 => unsafe { libc::kill(pid, 0) == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) },
            _ => false,
        }
    }
    #[cfg(all(not(unix), not(target_arch = "wasm32")))]
    {
        std::fs::metadata(join(&run_folder(run), "index.json")).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age.as_secs() < 120)
    }
}

/// The runs under `recovery/` (their folders' names).
fn runs() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    {
        vec![this_run()]
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Ok(rd) = std::fs::read_dir(recovery_root()) else { return Vec::new() };
        let mut out: Vec<String> = rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        out.sort();
        out
    }
}

/// What can be restored: each copy of a Studio that ended without
/// closing (not this one, not one still running), a line each:
/// `run <TAB> key <TAB> path <TAB> caption`. On the web the page's last
/// visit's are this page's run until it has looked ([`found`] is asked
/// once, at the start).
pub fn found() -> String {
    let mut out = String::new();
    for run in runs() {
        #[cfg(not(target_arch = "wasm32"))]
        if running(&run) {
            continue;
        }
        #[cfg(target_arch = "wasm32")]
        let _ = running(&run);
        for (key, path, caption) in index_of(&run) {
            out.push_str(&format!("{run}\t{key}\t{path}\t{caption}\n"));
        }
    }
    out
}

/// A copy's text ("" when there is none).
pub fn text(run: &str, key: &str) -> String {
    read_if_there(&copy_file(run, key)).unwrap_or_default()
}

/// Lets the copies of `run` go ("": every run that can be restored).
pub fn discard(run: &str) {
    let all: Vec<String> = if run.is_empty() { runs().into_iter().filter(|r| r == &this_run() || !running(r)).collect() } else { vec![run.to_string()] };
    for r in all {
        // (this page's run on the web: the last visit's copies, taken)
        let _ = write_index(&r, &[]);
        #[cfg(not(target_arch = "wasm32"))]
        if r != this_run() {
            let _ = std::fs::remove_dir_all(run_folder(&r));
        }
    }
}

/// The log of what went wrong in Studio's own code.
pub fn log_file() -> String {
    join(&studio_folder(), "studio-errors.log")
}

/// Adds `line` to the log (the oldest lines go past [`LOG_MAX`]).
pub fn log(line: &str) -> Result<(), String> {
    make_folder(&studio_folder());
    let path = log_file();
    let mut text = read_if_there(&path).unwrap_or_default();
    text.push_str(line.trim_end());
    text.push('\n');
    if text.len() > LOG_MAX {
        let cut = text.len() - LOG_MAX;
        let at = text[cut..].find('\n').map_or(cut, |i| cut + i + 1);
        text = text[at..].to_string();
    }
    write_text(&path, &text)
}

// ---- an Apply of RAI's changes: all or nothing ----------------------------------------
//
// Studio checks every new file an Apply would write before it writes any
// ([`can_write_new`]), then writes them ([`write_new`]), each file and
// folder it makes noted; when one write fails after all, what this Apply
// made is taken away again ([`apply_undo`]) and the project is as it was.

thread_local! {
    /// What the Apply under way made: (path, a folder?), in order.
    static MADE: std::cell::RefCell<Vec<(String, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// An Apply starts: nothing made yet.
pub fn apply_begin() {
    MADE.with(|m| m.borrow_mut().clear());
}

/// Whether a new file can be written at `path`: it isn't there yet, and
/// its folder is one (or can be made: the nearest folder above that is
/// there is a folder, not a file, and can be written).
pub fn can_write_new(path: &str) -> Result<(), String> {
    let path = slashes(path);
    if path.trim().is_empty() {
        return Err("no file named".into());
    }
    #[cfg(target_arch = "wasm32")]
    {
        if read_if_there(&path).is_some() {
            return Err(format!("{path} is there already"));
        }
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::path::Path;
        let p = Path::new(&path);
        if p.exists() {
            return Err(format!("{path} is there already"));
        }
        let mut up = p.parent();
        while let Some(d) = up {
            if d.as_os_str().is_empty() {
                break;
            }
            if d.exists() {
                if !d.is_dir() {
                    return Err(format!("its folder can't be made: {} is a file", slashes(&d.to_string_lossy())));
                }
                // (a folder that can't be written: found now, not half-way)
                let probe = d.join(format!(".rapidr-write-probe-{}", std::process::id()));
                return match std::fs::write(&probe, b"") {
                    Ok(()) => {
                        let _ = std::fs::remove_file(&probe);
                        Ok(())
                    }
                    Err(e) => Err(format!("{} can't be written: {e}", slashes(&d.to_string_lossy()))),
                };
            }
            up = d.parent();
        }
        Ok(())
    }
}

/// Writes a new file (its folders made first), noting what it made for
/// [`apply_undo`]. Never over a file that is there.
pub fn write_new(path: &str, text: &str) -> Result<(), String> {
    can_write_new(path)?;
    let path = slashes(path);
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::path::Path;
        // (the folders that aren't there, outermost first)
        let mut missing = Vec::new();
        let mut up = Path::new(&path).parent();
        while let Some(d) = up.filter(|d| !d.as_os_str().is_empty() && !d.exists()) {
            missing.push(d.to_path_buf());
            up = d.parent();
        }
        for d in missing.into_iter().rev() {
            std::fs::create_dir(&d).map_err(|e| format!("{}: {e}", slashes(&d.to_string_lossy())))?;
            MADE.with(|m| m.borrow_mut().push((slashes(&d.to_string_lossy()), true)));
        }
        // (whole or not at all: written beside it, then put in its place —
        // the text's own characters, UTF-8, as Studio saves a document;
        // RFileStream.WriteStr wrote a byte a character: 太阳系 became ???)
        let tmp = format!("{path}.rapidr-new-{}", std::process::id());
        if let Err(e) = std::fs::write(&tmp, text.as_bytes()).and_then(|()| std::fs::rename(&tmp, &path)) {
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("{path}: {e}"));
        }
    }
    #[cfg(target_arch = "wasm32")]
    write_text(&path, text)?;
    MADE.with(|m| m.borrow_mut().push((path, false)));
    Ok(())
}

/// Takes away what the Apply under way made (its files, then its folders,
/// the newest first): the project as it was. The paths taken away, a line
/// each.
pub fn apply_undo() -> String {
    let made = MADE.with(|m| std::mem::take(&mut *m.borrow_mut()));
    let mut out = String::new();
    for (path, folder) in made.into_iter().rev() {
        #[cfg(not(target_arch = "wasm32"))]
        let gone = if folder { std::fs::remove_dir(&path).is_ok() } else { std::fs::remove_file(&path).is_ok() };
        // (the page's store can't take a file away here: it is left empty)
        #[cfg(target_arch = "wasm32")]
        let gone = !folder && write_text(&path, "").is_ok();
        if gone {
            out.push_str(&path);
            out.push('\n');
        }
    }
    out
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// A run's copies kept, found by another Studio once the run is over,
    /// restored and let go; this run's are never offered to itself.
    #[test]
    fn copies_are_kept_found_and_let_go() {
        let dir = std::env::temp_dir().join(format!("rapidr-recovery-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("RAPIDR_STUDIO_DATA", &dir);
        keep("/p/main.rr", "/p/main.rr", "main.rr", "PRINT 1\n").unwrap();
        keep("untitled-1", "", "Untitled 1", "PRINT 2\n").unwrap();
        keep("/p/main.rr", "/p/main.rr", "main.rr", "PRINT 3\n").unwrap();
        assert_eq!(index_of(&this_run()).len(), 2);
        assert_eq!(text(&this_run(), "/p/main.rr"), "PRINT 3\n");
        // (this Studio's own copies are never offered to it)
        assert_eq!(found(), "");
        // (a Studio that ended without closing: process id 0 is never alive)
        let dead = "0";
        std::fs::create_dir_all(run_folder(dead)).unwrap();
        for e in std::fs::read_dir(run_folder(&this_run())).unwrap().flatten() {
            std::fs::copy(e.path(), run_folder(dead) + "/" + &e.file_name().to_string_lossy()).unwrap();
        }
        let f = found();
        assert!(f.contains("0\t/p/main.rr\t/p/main.rr\tmain.rr\n") && f.contains("0\tuntitled-1\t\tUntitled 1\n"), "{f}");
        assert_eq!(text(dead, "untitled-1"), "PRINT 2\n");
        forget("/p/main.rr");
        assert_eq!(index_of(&this_run()).len(), 1);
        discard("");
        assert_eq!(found(), "");
        end();
        assert!(!std::path::Path::new(&run_folder(&this_run())).exists());
        log("2026-10-29 10:00 one").unwrap();
        log("2026-10-29 10:01 two").unwrap();
        assert_eq!(read_text(&log_file()).unwrap(), "2026-10-29 10:00 one\n2026-10-29 10:01 two\n");
        std::env::remove_var("RAPIDR_STUDIO_DATA");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An Apply's new files: checked before any is written (a folder that
    /// is a file, a file there already), written with their folders, and
    /// taken away again — files and the folders made for them — when the
    /// Apply can't finish.
    #[test]
    fn an_apply_writes_all_or_takes_it_back() {
        let dir = std::env::temp_dir().join(format!("rapidr-apply-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = |rel: &str| slashes(&dir.join(rel).to_string_lossy());
        std::fs::write(dir.join("main.rr"), "PRINT 1\n").unwrap();
        assert!(can_write_new(&p("icons/a/folder.svg")).is_ok());
        assert!(can_write_new(&p("main.rr")).unwrap_err().contains("is there already"));
        assert!(can_write_new(&p("main.rr/blocked.svg")).unwrap_err().contains("is a file"));
        apply_begin();
        write_new(&p("icons/a/folder.svg"), "<svg/>").unwrap();
        // (the text's own characters on disk, UTF-8: not a byte a character)
        let zh = "msgid \"Solar System 3D\"\nmsgstr \"太阳系 3D\"\n";
        write_new(&p("lang/zh-Hans.po"), zh).unwrap();
        assert!(dir.join("icons/a/folder.svg").is_file());
        assert_eq!(std::fs::read(dir.join("lang/zh-Hans.po")).unwrap(), zh.as_bytes());
        assert!(std::fs::read_dir(dir.join("lang")).unwrap().count() == 1, "no temporary file left");
        // (the third can't be written: what the Apply made goes)
        assert!(write_new(&p("main.rr/blocked.svg"), "x").is_err());
        let gone = apply_undo();
        assert_eq!(gone.lines().count(), 5, "{gone}");
        assert!(!dir.join("icons").exists() && !dir.join("lang").exists());
        assert_eq!(std::fs::read_to_string(dir.join("main.rr")).unwrap(), "PRINT 1\n");
        // (a folder there before isn't taken away)
        std::fs::create_dir_all(dir.join("keep")).unwrap();
        apply_begin();
        write_new(&p("keep/x.txt"), "x").unwrap();
        apply_undo();
        assert!(dir.join("keep").is_dir() && !dir.join("keep/x.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
