//! A program's file names on the system it runs on.
//!
//! RapidQ's programs were written on Windows, where `\` separates folders
//! and a name finds its file in any case: `OPEN "data\Notes.txt"`,
//! `Bitmap.BMP = "OPEN.BMP"` beside an `open.bmp`, `Application.Path +
//! "\save.dat"`. On macOS and Linux a `\` is an ordinary character and Linux
//! (and a case-sensitive macOS volume) tells `OPEN.BMP` from `open.bmp`, so
//! such a program found none of its files there. [`host`] finds them as
//! Windows does; every file and folder a program names goes through it
//! (`OPEN`, QFILESTREAM, LoadFromFile, pictures, FILEEXISTS, KILL, MKDIR …),
//! on the interpreter and native builds alike. The web runtime keys its
//! files the same way (`file_key` in rapidr-runtime-web).
//!
//! A name that exists as it's written is always taken as written, so a
//! macOS or Linux path is never changed; a drive letter (`C:\…`) is left
//! alone (no such folder: the program's own error, as on a Windows without
//! that folder).

use std::path::PathBuf;

/// The path the system knows for `path`, a file or folder name as a
/// RapidQ program writes it: `\` read as `/`, then each part found in any
/// case (the first entry, in the folder's order, whose name matches). Parts
/// past the first that doesn't exist yet (a file to create) stay as
/// written. On Windows (and in the browser) the name is taken as it is.
pub fn host(path: &str) -> PathBuf {
    // (a file stream's writes it still keeps go to their file first: what
    // uses the file by its name sees them, as when every write went
    // straight through — crate::objects::flush_streams)
    crate::objects::flush_streams();
    if cfg!(any(windows, target_arch = "wasm32")) || path.is_empty() {
        return PathBuf::from(path);
    }
    let given = PathBuf::from(path);
    if given.exists() {
        return given;
    }
    let unified = path.replace('\\', "/");
    let mut parts = unified.split('/').peekable();
    let mut current = PathBuf::new();
    if unified.starts_with('/') {
        current.push("/");
    }
    let mut found_all = true;
    while let Some(part) = parts.next() {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || !found_all {
            current.push(part);
            continue;
        }
        let exact = current.join(part);
        if exact.symlink_metadata().is_ok() {
            current = exact;
            continue;
        }
        let folder = if current.as_os_str().is_empty() { PathBuf::from(".") } else { current.clone() };
        let wanted = part.to_lowercase();
        let other = std::fs::read_dir(&folder).ok().and_then(|entries| {
            entries.flatten().map(|e| e.file_name()).find(|name| name.to_str().is_some_and(|n| n.to_lowercase() == wanted))
        });
        match other {
            Some(name) => current.push(name),
            None => {
                found_all = false;
                current.push(part);
            }
        }
    }
    // (`dir\` with nothing after it: the folder)
    if current.as_os_str().is_empty() {
        return PathBuf::from(if unified.starts_with('/') { "/" } else { "." });
    }
    current
}

/// [`host`] as text, for what takes a string.
pub fn host_string(path: &str) -> String {
    host(path).to_string_lossy().into_owned()
}

#[cfg(all(test, not(windows), not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rapidr-paths-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("Data/Sub")).unwrap();
        std::fs::write(d.join("Data/Notes.TXT"), "x").unwrap();
        std::fs::write(d.join("Data/Sub/open.bmp"), "x").unwrap();
        d
    }

    /// The same file or folder (a case-insensitive macOS volume keeps the
    /// name as written: compared by what it is), or for one that doesn't
    /// exist, the same name in the same folder.
    fn same(got: PathBuf, want: PathBuf) {
        match (std::fs::canonicalize(&got), std::fs::canonicalize(&want)) {
            (Ok(a), Ok(b)) => assert_eq!(a, b, "{got:?} vs {want:?}"),
            _ => {
                assert_eq!(got.file_name(), want.file_name(), "{got:?} vs {want:?}");
                same(got.parent().unwrap().to_path_buf(), want.parent().unwrap().to_path_buf());
            }
        }
    }

    #[test]
    fn backslashes_and_any_case() {
        let d = folder("case");
        let base = d.to_string_lossy();
        same(host(&format!("{base}\\data\\notes.txt")), d.join("Data/Notes.TXT"));
        same(host(&format!("{base}/DATA/SUB/OPEN.BMP")), d.join("Data/Sub/open.bmp"));
        // (a file to create: its folder found, its name as written)
        same(host(&format!("{base}\\data\\New.txt")), d.join("Data/New.txt"));
        assert!(host(&format!("{base}\\nothing\\x\\y.txt")).ends_with("nothing/x/y.txt"));
        // (`\\` doubled, `.` parts)
        same(host(&format!("{base}\\\\Data\\.\\notes.txt")), d.join("Data/Notes.TXT"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn relative_names_from_the_current_folder() {
        // (relative to the current folder: the test's own crate folder)
        same(host("SRC\\PATHS.RS"), PathBuf::from("src/paths.rs"));
        same(host("src\\"), PathBuf::from("src"));
        same(host("..\\rapidr-value\\Cargo.toml"), PathBuf::from("../rapidr-value/Cargo.toml"));
    }

    #[test]
    fn names_that_exist_stay_as_written() {
        let d = folder("exact");
        // (a file whose name has a `\` in it: taken as it is)
        let odd = d.join("a\\b.txt");
        std::fs::write(&odd, "x").unwrap();
        assert_eq!(host(&odd.to_string_lossy()), odd);
        assert_eq!(host(""), PathBuf::from(""));
        // (a drive letter is no folder here: left as written)
        assert_eq!(host("C:\\RapidQ\\Exit.bmp"), PathBuf::from("C:/RapidQ/Exit.bmp"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
