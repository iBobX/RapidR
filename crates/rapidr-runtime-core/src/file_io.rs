//! BASIC File I/O — handle-based file operations.
//!
//! BASIC file semantics: `OPEN "file.txt" FOR INPUT AS #1`
//! Files are tracked by integer handle in a thread-local table.

use crate::value::{v_int, v_str, Value};
/// `RESOURCE(n)`, `RESOURCECOUNT`, `EXTRACTRESOURCE` (shared, rapidr_value::resources).
pub use crate::value::resources::{rp_extractresource, rp_resource, rp_resourcecount};
use std::cell::RefCell;

use std::fs;
use std::path::Path;

thread_local! {
    static DIR_ITER: RefCell<Option<(std::path::PathBuf, std::vec::IntoIter<String>)>> = RefCell::new(None);
}

// BASIC file I/O by file number is shared with the web runtime and the
// interpreter (rapidr_value::basic_files).
pub use crate::value::basic_files::{close_all as rp_close_all, input_field as rp_input_field};
pub fn rp_freefile() -> Value {
    crate::value::basic_files::freefile()
}
pub fn rp_open(filename: &Value, mode: &Value, file_num: &Value) {
    crate::value::basic_files::open(filename, mode, file_num)
}
pub fn rp_close(file_num: &Value) {
    crate::value::basic_files::close(file_num)
}
pub fn rp_line_input(file_num: &Value) -> Value {
    crate::value::basic_files::line_input(file_num)
}
pub fn rp_print_hash(file_num: &Value, items: &[Value]) {
    crate::value::basic_files::print_hash(file_num, items)
}
pub fn rp_write_hash(file_num: &Value, items: &[Value]) {
    crate::value::basic_files::write_hash(file_num, items)
}
pub fn rp_eof(file_num: &Value) -> Value {
    crate::value::basic_files::eof(file_num)
}
pub fn rp_lof(file_num: &Value) -> Value {
    crate::value::basic_files::lof(file_num)
}
pub fn rp_seek(file_num: &Value, position: &Value) {
    crate::value::basic_files::seek(file_num, position)
}

// ---------------------------------------------------------------------------
// FILELEN(filename) — return file size in bytes
// ---------------------------------------------------------------------------

pub fn rp_filelen(filename: &Value) -> Value {
    let path = filename.to_string_val();
    match fs::metadata(&path) {
        Ok(m) => v_int(m.len() as i64),
        Err(_) => v_int(0),
    }
}

// ---------------------------------------------------------------------------
// DIR$() — stateful directory iteration using glob patterns
// ---------------------------------------------------------------------------

pub fn rp_dir(pattern: &Value, attr: &Value) -> Value {
    let pat = pattern.to_string_val();
    if !pat.is_empty() {
        let attr = if matches!(attr, Value::Null) { 0 } else { attr.to_i64() };
        let (folder, entries) = dir_entries(&pat, attr);
        DIR_ITER.with(|di| *di.borrow_mut() = Some((folder, entries.into_iter())));
    }
    DIR_ITER.with(|di| {
        let mut di = di.borrow_mut();
        let Some((folder, iter)) = di.as_mut() else { return v_str("") };
        match iter.next() {
            Some(name) => {
                set_file_rec(&folder.join(&name), &name);
                Value::String(name)
            }
            None => v_str(""),
        }
    })
}

/// DIR$'s matches, as RapidQ (Windows' FindFirstFile) lists them: names
/// matching the pattern's last part (`*`, `?`, any case), in name order
/// (any case); with faDirectory (&H10) its folders too, `.` and `..`
/// first; without it files only. Hidden files (a dot first, faHidden &H2)
/// only when asked. `\` separates folders as `/` does.
fn dir_entries(pattern: &str, attr: i64) -> (std::path::PathBuf, Vec<String>) {
    let pat = if cfg!(windows) { pattern.to_string() } else { pattern.replace('\\', "/") };
    let path = Path::new(&pat);
    let folder = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => std::path::PathBuf::from("."),
    };
    let glob = path.file_name().and_then(|s| s.to_str()).unwrap_or("*").to_string();
    let dirs = attr & 0x10 != 0;
    let hidden = attr & 0x02 != 0;
    let mut names = Vec::new();
    if let Ok(dir) = fs::read_dir(&folder) {
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if (is_dir && !dirs) || (name.starts_with('.') && !hidden) || !matches_glob(&name, &glob) {
                continue;
            }
            names.push(name);
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    if dirs && matches_glob(".", &glob) {
        names.splice(0..0, [".".to_string(), "..".to_string()]);
    }
    (folder, names)
}

/// FileRec for the file DIR$ just found (crate::value::globals::FileRec).
fn set_file_rec(path: &Path, name: &str) {
    use chrono::{DateTime, Datelike, Local, Timelike};
    let meta = fs::metadata(path).ok();
    let size = meta.as_ref().filter(|m| m.is_file()).map_or(0, |m| m.len() as i64);
    let modified: Option<DateTime<Local>> = meta.and_then(|m| m.modified().ok()).map(DateTime::from);
    let (date, time, file_time) = match modified {
        Some(t) => (
            format!("{}-{}-{}", t.month(), t.day(), t.year()),
            format!("{}:{:02}", t.hour(), t.minute()),
            // (a DOS date and time, as Delphi's FileAge: newer is greater)
            (((t.year() as i64 - 1980) << 25) | ((t.month() as i64) << 21) | ((t.day() as i64) << 16) | ((t.hour() as i64) << 11) | ((t.minute() as i64) << 5) | (t.second() as i64 / 2)),
        ),
        None => (String::new(), String::new(), 0),
    };
    crate::value::globals::set_file_rec(crate::value::globals::FileRec { file_name: name.to_string(), short_name: String::new(), date, time, size, file_time });
}

/// Windows' wildcards: `*` any run, `?` any one character, any case; `*.*`
/// everything.
fn matches_glob(name: &str, pattern: &str) -> bool {
    if pattern == "*" || pattern == "*.*" {
        return true;
    }
    let n: Vec<char> = name.to_lowercase().chars().collect();
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    fn m(n: &[char], p: &[char]) -> bool {
        match p.first() {
            None => n.is_empty(),
            Some('*') => (0..=n.len()).any(|i| m(&n[i..], &p[1..])),
            Some('?') => !n.is_empty() && m(&n[1..], &p[1..]),
            Some(c) => n.first() == Some(c) && m(&n[1..], &p[1..]),
        }
    }
    m(&n, &p)
}

// ---------------------------------------------------------------------------
// MKDIR, RMDIR, KILL, RENAME
// ---------------------------------------------------------------------------

pub fn rp_mkdir(path: &Value) {
    let _ = fs::create_dir_all(path.to_string_val());
}

pub fn rp_rmdir(path: &Value) {
    let _ = fs::remove_dir(path.to_string_val());
}

pub fn rp_kill(filename: &Value) {
    let _ = fs::remove_file(filename.to_string_val());
}

pub fn rp_rename(old_name: &Value, new_name: &Value) {
    let _ = fs::rename(old_name.to_string_val(), new_name.to_string_val());
}

// ---------------------------------------------------------------------------
// CURDIR$ — current working directory
// ---------------------------------------------------------------------------

pub fn rp_curdir() -> Value {
    match std::env::current_dir() {
        Ok(p) => Value::String(p.to_string_lossy().into_owned()),
        Err(_) => v_str(""),
    }
}

// ---------------------------------------------------------------------------
// CHDIR — change directory
// ---------------------------------------------------------------------------

pub fn rp_chdir(path: &Value) {
    let _ = std::env::set_current_dir(path.to_string_val());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{v_int, v_str};
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_freefile() {
        let n = rp_freefile();
        assert_eq!(n, v_int(1));
    }

    #[test]
    fn test_open_write_close_read() {
        let tmp = std::env::temp_dir().join("rapidr_test_file_io.txt");
        let path = Value::String(tmp.to_string_lossy().into_owned());

        // Write
        rp_open(&path, &v_str("OUTPUT"), &v_int(1));
        rp_print_hash(&v_int(1), &[v_str("Hello"), v_str("World")]);
        rp_close(&v_int(1));

        // Read back
        rp_open(&path, &v_str("INPUT"), &v_int(1));
        let line = rp_line_input(&v_int(1));
        assert_eq!(line.to_string_val(), "Hello World");

        let eof = rp_eof(&v_int(1));
        assert_eq!(eof, v_int(-1)); // at EOF after reading only line

        rp_close(&v_int(1));

        // Cleanup
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_filelen() {
        let tmp = std::env::temp_dir().join("rapidr_test_filelen.txt");
        {
            let mut f = File::create(&tmp).unwrap();
            f.write_all(b"12345").unwrap();
        }
        let len = rp_filelen(&Value::String(tmp.to_string_lossy().into_owned()));
        assert_eq!(len, v_int(5));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_dir_glob() {
        // DIR$("*.rs") in the crate src directory should return something
        let result = rp_dir(&v_str("src/*.rs"), &v_int(0));
        // Should find at least lib.rs
        assert!(!result.to_string_val().is_empty());
    }

    #[test]
    fn test_mkdir_rmdir() {
        let tmp = std::env::temp_dir().join("rapidr_test_mkdir");
        let path = Value::String(tmp.to_string_lossy().into_owned());
        rp_mkdir(&path);
        assert!(tmp.is_dir());
        rp_rmdir(&path);
        assert!(!tmp.exists());
    }
}
