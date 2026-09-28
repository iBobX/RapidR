//! BASIC file I/O by file number — `OPEN name FOR INPUT | OUTPUT | APPEND |
//! BINARY AS #n`, `PRINT #n`, `WRITE #n`, `INPUT #n, a, b`, `LINE INPUT #n, s`,
//! `EOF(n)`, `LOF(n)`, `SEEK`, `CLOSE #n`, `FREEFILE` — the same on the desktop,
//! in native builds and on the web.
//!
//! A file is read whole when it's opened and kept in memory; what's written
//! goes back through the runtime's file hooks (`objects::set_file_io`: the
//! disk on the desktop, the page's own files in the browser) when the file is
//! closed, its length is asked for, or the program ends ([`close_all`]).
//! A file that can't be opened (missing, for INPUT) simply isn't open: reads
//! from it give "" and `EOF` is true, as RapidQ programs check.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::objects::{read_file, write_file};
use crate::{v_int, v_str, Value};

/// Largest file kept open (bytes) and most files open at once.
const MAX_FILE: usize = 256 << 20;
const MAX_HANDLES: usize = 255;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Input,
    Output,
    Append,
    Binary,
}

struct Handle {
    path: String,
    mode: Mode,
    data: Vec<u8>,
    pos: usize,
    dirty: bool,
}

impl Handle {
    fn flush(&mut self) {
        if self.dirty && self.mode != Mode::Input {
            let _ = write_file(&self.path, &self.data);
            self.dirty = false;
        }
    }

    fn write(&mut self, bytes: &[u8]) {
        if self.mode == Mode::Input || self.data.len() + bytes.len() > MAX_FILE {
            return;
        }
        if self.mode == Mode::Binary {
            let end = self.pos + bytes.len();
            if self.data.len() < end {
                self.data.resize(end, 0);
            }
            self.data[self.pos..end].copy_from_slice(bytes);
            self.pos = end;
        } else {
            self.data.extend_from_slice(bytes);
            self.pos = self.data.len();
        }
        self.dirty = true;
    }
}

thread_local! {
    static HANDLES: RefCell<BTreeMap<i64, Handle>> = const { RefCell::new(BTreeMap::new()) };
}

fn with<R>(num: i64, f: impl FnOnce(&mut Handle) -> R) -> Option<R> {
    HANDLES.with(|h| h.borrow_mut().get_mut(&num).map(f))
}

/// `FREEFILE`: the lowest file number not in use (0 if all are).
pub fn freefile() -> Value {
    HANDLES.with(|h| {
        let h = h.borrow();
        v_int((1..=MAX_HANDLES as i64).find(|i| !h.contains_key(i)).unwrap_or(0))
    })
}

/// `OPEN filename FOR mode AS #n`.
pub fn open(filename: &Value, mode: &Value, file_num: &Value) {
    let path = filename.to_string_val();
    let num = file_num.to_i64();
    let mode = match mode.to_string_val().to_ascii_uppercase().as_str() {
        "OUTPUT" => Mode::Output,
        "APPEND" => Mode::Append,
        "BINARY" | "RANDOM" => Mode::Binary,
        _ => Mode::Input,
    };
    if num <= 0 || HANDLES.with(|h| h.borrow().len() >= MAX_HANDLES && !h.borrow().contains_key(&num)) {
        return;
    }
    // A number that's open already is closed first.
    close(file_num);
    // (OUTPUT starts empty: what's there isn't read.)
    let existing = if mode == Mode::Output { None } else { read_file(&path).ok().filter(|d| d.len() <= MAX_FILE) };
    let (data, pos, dirty) = match (mode, existing) {
        (Mode::Input, None) => return,
        (Mode::Input, Some(d)) => (d, 0, false),
        // OUTPUT creates or empties the file at once.
        (Mode::Output, _) => {
            if write_file(&path, &[]).is_err() {
                return;
            }
            (Vec::new(), 0, false)
        }
        (Mode::Append, d) => {
            let d = d.unwrap_or_default();
            let n = d.len();
            (d, n, false)
        }
        (Mode::Binary, d) => (d.unwrap_or_default(), 0, false),
    };
    HANDLES.with(|h| h.borrow_mut().insert(num, Handle { path, mode, data, pos, dirty }));
}

/// `CLOSE #n`.
pub fn close(file_num: &Value) {
    close_num(file_num.to_i64());
}

fn close_num(num: i64) {
    if let Some(mut h) = HANDLES.with(|h| h.borrow_mut().remove(&num)) {
        h.flush();
    }
}

/// Closes every open file (the program ends): what was written is kept.
pub fn close_all() {
    let all: Vec<i64> = HANDLES.with(|h| h.borrow().keys().copied().collect());
    for n in all {
        close_num(n);
    }
}

/// `LINE INPUT #n, s`: the rest of the line, without its line break.
pub fn line_input(file_num: &Value) -> Value {
    with(file_num.to_i64(), |h| {
        if h.pos >= h.data.len() {
            return v_str("");
        }
        let rest = &h.data[h.pos..];
        let end = rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
        let mut line = &rest[..end];
        h.pos += (end + 1).min(rest.len());
        if line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }
        Value::String(String::from_utf8_lossy(line).into_owned())
    })
    .unwrap_or_else(|| v_str(""))
}

/// `INPUT #n, a, b`: the next field — a quoted string, or text up to the
/// next comma or line break, with the spaces around it left out.
pub fn input_field(file_num: &Value) -> Value {
    with(file_num.to_i64(), |h| {
        let d = &h.data;
        let mut i = h.pos;
        while i < d.len() && matches!(d[i], b' ' | b'\t' | b'\r' | b'\n') {
            i += 1;
        }
        let mut out: Vec<u8> = Vec::new();
        if i < d.len() && d[i] == b'"' {
            i += 1;
            while i < d.len() && d[i] != b'"' {
                out.push(d[i]);
                i += 1;
            }
            i = (i + 1).min(d.len());
            while i < d.len() && matches!(d[i], b' ' | b'\t') {
                i += 1;
            }
        } else {
            while i < d.len() && !matches!(d[i], b',' | b'\n' | b'\r') {
                out.push(d[i]);
                i += 1;
            }
            while out.last() == Some(&b' ') || out.last() == Some(&b'\t') {
                out.pop();
            }
        }
        // The separator after the field.
        if i < d.len() && d[i] == b',' {
            i += 1;
        } else if i < d.len() && d[i] == b'\r' {
            i += 1;
            if i < d.len() && d[i] == b'\n' {
                i += 1;
            }
        } else if i < d.len() && d[i] == b'\n' {
            i += 1;
        }
        h.pos = i;
        Value::String(String::from_utf8_lossy(&out).into_owned())
    })
    .unwrap_or_else(|| v_str(""))
}

/// `PRINT #n, a, b`: the items separated by a space, then a line break.
pub fn print_hash(file_num: &Value, items: &[Value]) {
    let text: Vec<String> = items.iter().map(Value::to_string_val).collect();
    with(file_num.to_i64(), |h| h.write(format!("{}\n", text.join(" ")).as_bytes()));
}

/// `WRITE #n, a, b`: strings quoted, everything separated by commas.
pub fn write_hash(file_num: &Value, items: &[Value]) {
    let text: Vec<String> = items
        .iter()
        .map(|v| match v {
            Value::String(s) => format!("\"{s}\""),
            other => other.to_string_val(),
        })
        .collect();
    with(file_num.to_i64(), |h| h.write(format!("{}\n", text.join(",")).as_bytes()));
}

/// `EOF(n)`: -1 when nothing is left to read (or the file isn't open).
pub fn eof(file_num: &Value) -> Value {
    v_int(with(file_num.to_i64(), |h| if h.mode == Mode::Input || h.mode == Mode::Binary { h.pos >= h.data.len() } else { true }).map_or(-1, |e| if e { -1 } else { 0 }))
}

/// `LOF(n)`: the file's length in bytes.
pub fn lof(file_num: &Value) -> Value {
    v_int(with(file_num.to_i64(), |h| {
        h.flush();
        h.data.len() as i64
    })
    .unwrap_or(0))
}

/// `SEEK #n, pos`: the next byte read or written (1-based).
pub fn seek(file_num: &Value, position: &Value) {
    let pos = (position.to_i64() - 1).max(0) as usize;
    with(file_num.to_i64(), |h| h.pos = pos.min(h.data.len()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(test: &str) -> String {
        let d = std::env::temp_dir().join(format!("rapidr_basic_files_{}_{test}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        d.to_string_lossy().into_owned()
    }

    #[test]
    fn write_then_read_back() {
        let path = format!("{}/a.txt", dir("rw"));
        let (f, n) = (v_str(&path), v_int(1));
        open(&f, &v_str("OUTPUT"), &n);
        print_hash(&n, &[v_str("Hello"), v_int(5)]);
        write_hash(&n, &[v_str("a b"), v_int(7), v_str("z")]);
        close(&n);
        open(&f, &v_str("INPUT"), &n);
        assert_eq!(eof(&n), v_int(0));
        assert_eq!(line_input(&n).to_string_val(), "Hello 5");
        assert_eq!(input_field(&n).to_string_val(), "a b");
        assert_eq!(input_field(&n).to_string_val(), "7");
        assert_eq!(input_field(&n).to_string_val(), "z");
        assert_eq!(eof(&n), v_int(-1));
        assert_eq!(lof(&n), v_int(20));
        close(&n);
        // APPEND keeps what's there.
        open(&f, &v_str("APPEND"), &n);
        print_hash(&n, &[v_str("more")]);
        close_all();
        open(&f, &v_str("INPUT"), &n);
        line_input(&n);
        line_input(&n);
        assert_eq!(line_input(&n).to_string_val(), "more");
        close(&n);
        // A missing file isn't open: EOF, and "" to read.
        open(&v_str(&format!("{}/nope.txt", dir("rw"))), &v_str("INPUT"), &v_int(2));
        assert_eq!((eof(&v_int(2)), line_input(&v_int(2)).to_string_val()), (v_int(-1), String::new()));
        assert_eq!(freefile(), v_int(1));
        let _ = std::fs::remove_dir_all(dir("rw"));
    }

    #[test]
    fn binary_seek_overwrites() {
        let path = format!("{}/b.bin", dir("seek"));
        let (f, n) = (v_str(&path), v_int(3));
        open(&f, &v_str("OUTPUT"), &n);
        print_hash(&n, &[v_str("abcdef")]);
        close(&n);
        open(&f, &v_str("BINARY"), &n);
        seek(&n, &v_int(3));
        with(3, |h| h.write(b"XY"));
        close(&n);
        assert_eq!(std::fs::read(&path).unwrap(), b"abXYef\n");
        let _ = std::fs::remove_dir_all(dir("seek"));
    }
}
