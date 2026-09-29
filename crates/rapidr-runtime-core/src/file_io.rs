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
    static DIR_ITER: RefCell<Option<std::vec::IntoIter<String>>> = RefCell::new(None);
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

pub fn rp_dir(pattern: &Value, _attr: &Value) -> Value {
    let pat = pattern.to_string_val();

    if !pat.is_empty() {
        // Initial call — build the list
        let parent = Path::new(&pat)
            .parent()
            .unwrap_or(Path::new("."));
        let filename_pattern = Path::new(&pat)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("*");

        let mut entries = Vec::new();
        if let Ok(dir) = fs::read_dir(parent) {
            for entry in dir.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if matches_glob(&name, filename_pattern) {
                    entries.push(name.into_owned());
                }
            }
        }
        entries.sort();

        let first = entries.first().cloned().unwrap_or_default();

        // Store remaining entries for subsequent calls
        let mut iter = entries.into_iter();
        iter.next(); // consume the first one we're returning
        DIR_ITER.with(|di| {
            *di.borrow_mut() = Some(iter);
        });

        if first.is_empty() {
            v_str("")
        } else {
            Value::String(first)
        }
    } else {
        // Continuation call
        DIR_ITER.with(|di| {
            let mut iter = di.borrow_mut();
            match iter.as_mut().and_then(|i| i.next()) {
                Some(name) => Value::String(name),
                None => v_str(""),
            }
        })
    }
}

/// Minimal glob matching supporting `*` wildcards and `*.ext` patterns.
fn matches_glob(name: &str, pattern: &str) -> bool {
    if pattern == "*" || pattern == "*.*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return name.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return name.starts_with(prefix);
    }
    name == pattern
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
