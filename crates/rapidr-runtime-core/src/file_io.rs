//! BASIC File I/O — handle-based file operations.
//!
//! BASIC file semantics: `OPEN "file.txt" FOR INPUT AS #1`
//! Files are tracked by integer handle in a thread-local table.

use crate::value::{v_int, v_str, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

enum FileHandle {
    Reader(BufReader<File>),
    Writer(BufWriter<File>),
}

thread_local! {
    static FILE_HANDLES: RefCell<HashMap<i64, FileHandle>> = RefCell::new(HashMap::new());
    static DIR_ITER: RefCell<Option<std::vec::IntoIter<String>>> = RefCell::new(None);
}

// ---------------------------------------------------------------------------
// FREEFILE — return next available file number
// ---------------------------------------------------------------------------

pub fn rp_freefile() -> Value {
    FILE_HANDLES.with(|fh| {
        let map = fh.borrow();
        for i in 1..=255 {
            if !map.contains_key(&i) {
                return v_int(i);
            }
        }
        v_int(0)
    })
}

// ---------------------------------------------------------------------------
// OPEN "filename" FOR mode AS #n
// ---------------------------------------------------------------------------

pub fn rp_open(filename: &Value, mode: &Value, file_num: &Value) {
    let path = filename.to_string_val();
    let mode_str = mode.to_string_val().to_uppercase();
    let num = file_num.to_i64();

    let handle = match mode_str.as_str() {
        "INPUT" => {
            File::open(&path).ok().map(|f| FileHandle::Reader(BufReader::new(f)))
        }
        "OUTPUT" => {
            File::create(&path).ok().map(|f| FileHandle::Writer(BufWriter::new(f)))
        }
        "APPEND" => {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .ok()
                .map(|f| FileHandle::Writer(BufWriter::new(f)))
        }
        "BINARY" | "RANDOM" => {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)
                .ok()
                .map(|f| FileHandle::Writer(BufWriter::new(f)))
        }
        _ => {
            // Default to input
            File::open(&path).ok().map(|f| FileHandle::Reader(BufReader::new(f)))
        }
    };

    if let Some(h) = handle {
        FILE_HANDLES.with(|fh| {
            fh.borrow_mut().insert(num, h);
        });
    }
}

// ---------------------------------------------------------------------------
// CLOSE #n
// ---------------------------------------------------------------------------

pub fn rp_close(file_num: &Value) {
    let num = file_num.to_i64();
    FILE_HANDLES.with(|fh| {
        if let Some(handle) = fh.borrow_mut().remove(&num) {
            // Flush writer before dropping
            if let FileHandle::Writer(mut w) = handle {
                let _ = w.flush();
            }
        }
    });
}

// ---------------------------------------------------------------------------
// LINE INPUT #n — read a line from file
// ---------------------------------------------------------------------------

pub fn rp_line_input(file_num: &Value) -> Value {
    let num = file_num.to_i64();
    FILE_HANDLES.with(|fh| {
        let mut map = fh.borrow_mut();
        if let Some(FileHandle::Reader(reader)) = map.get_mut(&num) {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => v_str(""),
                Ok(_) => {
                    // Strip trailing newline
                    if line.ends_with('\n') {
                        line.pop();
                    }
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    Value::String(line)
                }
                Err(_) => v_str(""),
            }
        } else {
            v_str("")
        }
    })
}

// ---------------------------------------------------------------------------
// PRINT #n — write to file
// ---------------------------------------------------------------------------

pub fn rp_print_hash(file_num: &Value, items: &[Value]) {
    let num = file_num.to_i64();
    FILE_HANDLES.with(|fh| {
        let mut map = fh.borrow_mut();
        if let Some(FileHandle::Writer(writer)) = map.get_mut(&num) {
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    let _ = write!(writer, " ");
                }
                let _ = write!(writer, "{}", item.to_string_val());
            }
            let _ = writeln!(writer);
        }
    });
}

// ---------------------------------------------------------------------------
// WRITE #n — write comma-separated quoted values
// ---------------------------------------------------------------------------

pub fn rp_write_hash(file_num: &Value, items: &[Value]) {
    let num = file_num.to_i64();
    FILE_HANDLES.with(|fh| {
        let mut map = fh.borrow_mut();
        if let Some(FileHandle::Writer(writer)) = map.get_mut(&num) {
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    let _ = write!(writer, ",");
                }
                match item {
                    Value::String(s) => {
                        let _ = write!(writer, "\"{}\"", s);
                    }
                    _ => {
                        let _ = write!(writer, "{}", item.to_string_val());
                    }
                }
            }
            let _ = writeln!(writer);
        }
    });
}

// ---------------------------------------------------------------------------
// EOF(#n) — check if at end of file
// ---------------------------------------------------------------------------

pub fn rp_eof(file_num: &Value) -> Value {
    let num = file_num.to_i64();
    FILE_HANDLES.with(|fh| {
        let mut map = fh.borrow_mut();
        if let Some(FileHandle::Reader(reader)) = map.get_mut(&num) {
            // Check if next read would return 0 bytes
            match reader.fill_buf() {
                Ok(buf) => v_int(if buf.is_empty() { -1 } else { 0 }),
                Err(_) => v_int(-1),
            }
        } else {
            v_int(-1)
        }
    })
}

// ---------------------------------------------------------------------------
// LOF(#n) — length of open file
// ---------------------------------------------------------------------------

pub fn rp_lof(file_num: &Value) -> Value {
    let num = file_num.to_i64();
    FILE_HANDLES.with(|fh| {
        let mut map = fh.borrow_mut();
        match map.get_mut(&num) {
            Some(FileHandle::Reader(reader)) => {
                let inner = reader.get_mut();
                let pos = inner.stream_position().unwrap_or(0);
                let end = inner.seek(SeekFrom::End(0)).unwrap_or(0);
                let _ = inner.seek(SeekFrom::Start(pos));
                v_int(end as i64)
            }
            Some(FileHandle::Writer(writer)) => {
                let _ = writer.flush();
                let inner = writer.get_mut();
                let pos = inner.stream_position().unwrap_or(0);
                let end = inner.seek(SeekFrom::End(0)).unwrap_or(0);
                let _ = inner.seek(SeekFrom::Start(pos));
                v_int(end as i64)
            }
            None => v_int(0),
        }
    })
}

// ---------------------------------------------------------------------------
// SEEK #n, pos — set file position (1-based)
// ---------------------------------------------------------------------------

pub fn rp_seek(file_num: &Value, position: &Value) {
    let num = file_num.to_i64();
    let pos = (position.to_i64() - 1).max(0) as u64;
    FILE_HANDLES.with(|fh| {
        let mut map = fh.borrow_mut();
        match map.get_mut(&num) {
            Some(FileHandle::Reader(reader)) => {
                let _ = reader.seek(SeekFrom::Start(pos));
            }
            Some(FileHandle::Writer(writer)) => {
                let _ = writer.flush();
                let _ = writer.seek(SeekFrom::Start(pos));
            }
            None => {}
        }
    });
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
