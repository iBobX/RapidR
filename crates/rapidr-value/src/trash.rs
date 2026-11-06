//! MOVETOTRASH (RapidR's): a file or folder moved to the system's Trash
//! (the Recycle Bin on Windows), where the person can put it back — never
//! deleted for good. One small implementation per system, no library:
//!
//! - **macOS:** Foundation's `NSFileManager trashItemAtURL:` (the Finder's
//!   own "Move to Trash"), called through the Objective-C runtime.
//! - **Windows:** the shell's `SHFileOperationW` with `FOF_ALLOWUNDO` (the
//!   Recycle Bin), no question shown.
//! - **Linux and the other Unix systems:** the freedesktop.org Trash
//!   specification (1.0), written here: the home Trash
//!   (`$XDG_DATA_HOME/Trash`, `~/.local/share/Trash`) for files on the
//!   home's disk, else the disk's own `$topdir/.Trash-$uid`; each item in
//!   `files/` with its `info/<name>.trashinfo` (where it was, when), so the
//!   desktop's Trash lists it and Restore puts it back.
//! - **The browser:** there is no Trash: [`move_to_trash`] says so.

use std::path::Path;

/// Moves `path` (a file or a folder) to the Trash. `Err` says why not, in
/// plain words (it doesn't exist, this system has no Trash for it …).
pub fn move_to_trash(path: &Path) -> Result<(), String> {
    if cfg!(target_arch = "wasm32") {
        return Err("the browser has no Trash".to_string());
    }
    if std::fs::symlink_metadata(path).is_err() {
        return Err(format!("{} doesn't exist", path.display()));
    }
    let full = std::path::absolute(path).map_err(|e| e.to_string())?;
    // (RapidR's own tests: a folder of theirs as the Trash — never the
    // person's — in test mode only)
    #[cfg(unix)]
    if let Some(t) = test_trash() {
        return freedesktop::trash_into(&full, &t, None);
    }
    system_trash(&full)
}

/// (tests) `RAPIDR_TEST_TRASH`: a folder used as the Trash (the
/// freedesktop.org layout) — honoured in RapidR's test mode only, so a
/// test run never fills the real one.
#[cfg(unix)]
fn test_trash() -> Option<std::path::PathBuf> {
    rapidr_testmode::var_os("RAPIDR_TEST_TRASH").map(std::path::PathBuf::from).filter(|p| p.is_absolute())
}

#[cfg(target_os = "macos")]
fn system_trash(path: &Path) -> Result<(), String> {
    mac::trash(path)
}

#[cfg(windows)]
fn system_trash(path: &Path) -> Result<(), String> {
    windows_trash(path)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn system_trash(path: &Path) -> Result<(), String> {
    freedesktop::trash(path)
}

#[cfg(not(any(unix, windows)))]
fn system_trash(_path: &Path) -> Result<(), String> {
    Err("this system has no Trash".to_string())
}

// ---- macOS -------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod mac {
    use std::ffi::{c_char, c_void, CStr, CString};
    use std::path::Path;

    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_msgSend();
        fn objc_autoreleasePoolPush() -> *mut c_void;
        fn objc_autoreleasePoolPop(pool: *mut c_void);
    }
    #[link(name = "Foundation", kind = "framework")]
    extern "C" {}

    fn class(name: &CStr) -> Id {
        unsafe { objc_getClass(name.as_ptr()) }
    }

    fn sel(name: &CStr) -> Sel {
        unsafe { sel_registerName(name.as_ptr()) }
    }

    /// `[[NSFileManager defaultManager] trashItemAtURL:[NSURL
    /// fileURLWithPath:path] resultingItemURL:nil error:&error]`.
    pub fn trash(path: &Path) -> Result<(), String> {
        let c_path = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| "the name has a NUL character".to_string())?;
        // (objc_msgSend called with each message's own prototype)
        let send_ptr: unsafe extern "C" fn(Id, Sel, *const c_char) -> Id = unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        let send_id: unsafe extern "C" fn(Id, Sel, Id) -> Id = unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        let send: unsafe extern "C" fn(Id, Sel) -> Id = unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        let send_trash: unsafe extern "C" fn(Id, Sel, Id, *mut Id, *mut Id) -> i8 = unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        let send_text: unsafe extern "C" fn(Id, Sel) -> *const c_char = unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
        unsafe {
            let pool = objc_autoreleasePoolPush();
            let out = (|| {
                let s = send_ptr(class(c"NSString"), sel(c"stringWithUTF8String:"), c_path.as_ptr());
                if s.is_null() {
                    return Err("the name couldn't be read".to_string());
                }
                let url = send_id(class(c"NSURL"), sel(c"fileURLWithPath:"), s);
                let fm = send(class(c"NSFileManager"), sel(c"defaultManager"));
                if url.is_null() || fm.is_null() {
                    return Err("the Trash couldn't be reached".to_string());
                }
                let mut error: Id = std::ptr::null_mut();
                if send_trash(fm, sel(c"trashItemAtURL:resultingItemURL:error:"), url, std::ptr::null_mut(), &mut error) != 0 {
                    return Ok(());
                }
                if error.is_null() {
                    return Err("the system didn't move it".to_string());
                }
                let words = send(error, sel(c"localizedDescription"));
                let text = if words.is_null() { std::ptr::null() } else { send_text(words, sel(c"UTF8String")) };
                Err(if text.is_null() { "the system didn't move it".to_string() } else { CStr::from_ptr(text).to_string_lossy().into_owned() })
            })();
            objc_autoreleasePoolPop(pool);
            out
        }
    }
}

// ---- Windows -----------------------------------------------------------------

#[cfg(windows)]
fn windows_trash(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::{SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW};
    // (pFrom: the full name, ended by two NULs)
    let mut from: Vec<u16> = path.as_os_str().encode_wide().collect();
    from.push(0);
    from.push(0);
    let mut op: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
    op.wFunc = FO_DELETE;
    op.pFrom = from.as_ptr();
    op.fFlags = (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT) as u16;
    let r = unsafe { SHFileOperationW(&mut op) };
    if r == 0 && op.fAnyOperationsAborted == 0 {
        Ok(())
    } else {
        Err(format!("the Recycle Bin didn't take it (error {r})"))
    }
}

// ---- freedesktop.org (Linux, the BSDs) -------------------------------------------

/// The Trash specification's parts, kept for every Unix so that macOS'
/// tests run them too (Linux uses them).
#[cfg(unix)]
pub(crate) mod freedesktop {
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
    use std::path::{Path, PathBuf};

    /// Moves `path` (absolute) to its disk's Trash.
    pub fn trash(path: &Path) -> Result<(), String> {
        let dev = fs::symlink_metadata(path).map_err(|e| e.to_string())?.dev();
        let home = home_trash();
        // (the home Trash, when the item is on the same disk: a rename there)
        if let Some(home) = &home {
            let _ = make_dir(home);
            if fs::metadata(home).is_ok_and(|m| m.dev() == dev) {
                return trash_into(path, home, None);
            }
        }
        // (else the disk's own Trash, at the top of its mount)
        let top = top_dir(path, dev).ok_or("its disk's top folder couldn't be found")?;
        let uid = unsafe { libc::getuid() };
        let shared = top.join(".Trash");
        let shared_ok = fs::symlink_metadata(&shared).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink() && m.permissions().mode() & 0o1000 != 0);
        let dir = if shared_ok { shared.join(uid.to_string()) } else { top.join(format!(".Trash-{uid}")) };
        trash_into(path, &dir, Some(&top))
    }

    /// `$XDG_DATA_HOME/Trash`, else `~/.local/share/Trash`.
    fn home_trash() -> Option<PathBuf> {
        if let Some(d) = std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty() && Path::new(d).is_absolute()) {
            return Some(PathBuf::from(d).join("Trash"));
        }
        std::env::var_os("HOME").filter(|h| !h.is_empty()).map(|h| PathBuf::from(h).join(".local/share/Trash"))
    }

    /// The top folder of the mount `path` is on: the last parent on the
    /// same device.
    fn top_dir(path: &Path, dev: u64) -> Option<PathBuf> {
        let mut top = path.parent()?.to_path_buf();
        while let Some(up) = top.parent() {
            if fs::metadata(up).map(|m| m.dev()).ok() != Some(dev) {
                break;
            }
            top = up.to_path_buf();
        }
        Some(top)
    }

    fn make_dir(d: &Path) -> std::io::Result<()> {
        fs::DirBuilder::new().recursive(true).mode(0o700).create(d)
    }

    /// Moves `path` into Trash folder `trash` (made if needed): its
    /// `info/<name>.trashinfo` first (made new: a name another item has is
    /// "name 2", "name 3" …), then the item renamed into `files/`. `top`:
    /// a disk's Trash, whose `Path=` is relative to that disk's top.
    pub fn trash_into(path: &Path, trash: &Path, top: Option<&Path>) -> Result<(), String> {
        let (files, info) = (trash.join("files"), trash.join("info"));
        make_dir(&files).and_then(|_| make_dir(&info)).map_err(|e| format!("the Trash folder couldn't be made: {e}"))?;
        let base = path.file_name().ok_or("it has no name")?.to_string_lossy().into_owned();
        let shown = match top.and_then(|t| path.strip_prefix(t).ok()) {
            Some(rel) => rel.to_path_buf(),
            None => path.to_path_buf(),
        };
        let body = format!("[Trash Info]\nPath={}\nDeletionDate={}\n", encode(&shown.to_string_lossy()), local_now());
        for n in 1..10_000u32 {
            let name = if n == 1 { base.clone() } else { stem_n(&base, n) };
            if fs::symlink_metadata(files.join(&name)).is_ok() {
                continue;
            }
            let info_file = info.join(format!("{name}.trashinfo"));
            let made = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&info_file);
            let Ok(mut f) = made else { continue };
            if f.write_all(body.as_bytes()).is_err() {
                let _ = fs::remove_file(&info_file);
                return Err("the Trash's note couldn't be written".to_string());
            }
            drop(f);
            return match fs::rename(path, files.join(&name)) {
                Ok(()) => Ok(()),
                Err(e) => {
                    let _ = fs::remove_file(&info_file);
                    Err(format!("it couldn't be moved to the Trash: {e}"))
                }
            };
        }
        Err("the Trash has too many items of that name".to_string())
    }

    /// "report.txt" → "report 2.txt" (a folder "Old" → "Old 2").
    fn stem_n(base: &str, n: u32) -> String {
        match base.rfind('.').filter(|&k| k > 0) {
            Some(k) => format!("{} {n}{}", &base[..k], &base[k..]),
            None => format!("{base} {n}"),
        }
    }

    /// A path as the specification writes it: percent-encoded, `/` kept.
    pub fn encode(path: &str) -> String {
        let mut out = String::new();
        for b in path.bytes() {
            if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
                out.push(b as char);
            } else {
                out.push_str(&format!("%{b:02X}"));
            }
        }
        out
    }

    /// Now, in local time, as `2026-11-06T14:30:05`.
    fn local_now() -> String {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as libc::time_t;
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
            return String::new();
        }
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::freedesktop::{encode, trash_into};

    /// The specification's layout: the item in `files/`, its note in
    /// `info/` saying where it was; a second item of the same name is
    /// "name 2".
    #[test]
    fn a_file_goes_to_a_freedesktop_trash_with_its_note() {
        let base = std::env::temp_dir().join(format!("rapidr_trash_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let trash = base.join("Trash");
        std::fs::create_dir_all(&base).unwrap();
        for _ in 0..2 {
            let f = base.join("my notes.txt");
            std::fs::write(&f, "hello").unwrap();
            trash_into(&f, &trash, None).unwrap();
            assert!(!f.exists());
        }
        assert_eq!(std::fs::read_to_string(trash.join("files/my notes.txt")).unwrap(), "hello");
        assert!(trash.join("files/my notes 2.txt").exists());
        let note = std::fs::read_to_string(trash.join("info/my notes 2.txt.trashinfo")).unwrap();
        assert!(note.starts_with("[Trash Info]\nPath="), "{note}");
        assert!(note.contains("my%20notes.txt\nDeletionDate=20"), "{note}");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn trash_paths_are_percent_encoded() {
        assert_eq!(encode("/home/ana/año 1/a&b.txt"), "/home/ana/a%C3%B1o%201/a%26b.txt");
    }

    /// Nothing there: said, nothing moved.
    #[test]
    fn a_missing_file_is_said() {
        let e = super::move_to_trash(std::path::Path::new("/no/such/rapidr-file.txt")).unwrap_err();
        assert!(e.contains("doesn't exist"), "{e}");
    }
}
