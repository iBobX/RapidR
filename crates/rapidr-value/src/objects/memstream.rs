//! QMEMORYSTREAM and QFILESTREAM (manual, Appendix A): a growable byte
//! buffer with a position that reads and writes start from. A file stream is
//! the same buffer holding the file's bytes, with every change written
//! through to the file (`FileSink`), so both kinds share every method.
//!
//! Out of the data, as RapidQ's (RC.EXE, tests/conformance/cases/
//! memstream_out_of_range.bas): Position may be set past the end (and, on a
//! memory stream, before the start); reads there get no bytes and leave it;
//! `ReadStr(n)` is always n characters, spaces where there were no bytes; a
//! write past the end fills the gap with zeros, one before the start is
//! dropped; a Size that leaves Position past the new end moves it to the
//! old end. Where the two kinds differ (RC.EXE, filestream_reads.bas): a
//! QFILESTREAM's ReadStr(n) and Read(S$) give one more character, a space.

use super::codec::{bytes_to_string, decode_text, fit_characters, string_to_bytes};
use crate::{v_dbl, v_int, v_str, Value};

#[derive(Debug, Default)]
pub struct MemStream {
    pub data: Vec<u8>,
    /// RapidQ's Position: anywhere, even outside the data.
    pub pos: i64,
    /// Set for an open QFILESTREAM: where changes are written.
    pub file: Option<FileSink>,
}

/// The file behind a QFILESTREAM.
#[derive(Debug)]
pub struct FileSink {
    pub path: String,
    pub writable: bool,
    /// The open file (desktop); `None` when files go through the runtime's
    /// file hooks (the web), which then get the whole buffer at a flush.
    #[cfg(not(target_arch = "wasm32"))]
    pub handle: Option<std::fs::File>,
    /// What changed since the file was last written: the bytes
    /// `data[from..to]`, one run (writes one after another, as a program
    /// writes a file, make it longer; a write elsewhere writes it first).
    dirty: Option<(usize, usize)>,
    /// The stream's Size changed (Size = n): the file's length follows at
    /// the flush.
    resized: bool,
}

/// How many changed bytes a file stream keeps before writing them: what a
/// program that stops without a word (a crash, the power) can lose at most.
/// Everything else writes them at once — Close, the program's end (END, a
/// run-time error, the last window closed, its main code done), and any
/// other use of a file by its name (`OPEN`, another stream's `Open`,
/// `FILELEN`, `SHELL` … — `flush_all`).
const FLUSH_AT: usize = 64 * 1024;

thread_local! {
    /// Whether some file stream has changes not written yet (`flush_all`
    /// has nothing to do otherwise: one look, no search).
    static PENDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether some file stream has changes it hasn't written yet.
pub fn pending() -> bool {
    PENDING.with(std::cell::Cell::get)
}

/// (every file stream written: nothing pending)
pub(super) fn none_pending() {
    PENDING.with(|p| p.set(false));
}

impl FileSink {
    pub fn new(path: &str, writable: bool, #[cfg(not(target_arch = "wasm32"))] handle: Option<std::fs::File>) -> FileSink {
        FileSink {
            path: path.to_string(),
            writable,
            #[cfg(not(target_arch = "wasm32"))]
            handle,
            dirty: None,
            resized: false,
        }
    }

    /// `data[from..to]` changed (written there): kept to be written with the
    /// run before it, or that run written first when this one isn't next to
    /// it; written now once the run is [`FLUSH_AT`] long.
    fn changed(&mut self, data: &[u8], from: usize, to: usize) -> Result<(), String> {
        match self.dirty {
            Some((a, b)) if from <= b && to >= a => self.dirty = Some((a.min(from), b.max(to))),
            Some(_) => {
                self.flush(data)?;
                self.dirty = Some((from, to));
            }
            None => self.dirty = Some((from, to)),
        }
        PENDING.with(|p| p.set(true));
        match self.dirty {
            Some((a, b)) if b - a >= FLUSH_AT => self.flush(data),
            _ => Ok(()),
        }
    }

    /// Whether it has changes the file doesn't have yet.
    pub fn is_dirty(&self) -> bool {
        self.dirty.is_some() || self.resized
    }

    /// Writes the changes to the file: afterwards the file holds `data`,
    /// as it did when every write went straight through.
    pub fn flush(&mut self, data: &[u8]) -> Result<(), String> {
        if !self.is_dirty() {
            return Ok(());
        }
        let dirty = self.dirty.take();
        let resized = std::mem::take(&mut self.resized);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(f) = self.handle.as_mut() {
            use std::io::{Seek, SeekFrom, Write};
            let fail = |e: std::io::Error| format!("can't write {}: {e}", self.path);
            if let Some((a, b)) = dirty {
                let b = b.min(data.len());
                if a < b {
                    f.seek(SeekFrom::Start(a as u64)).map_err(fail)?;
                    f.write_all(&data[a..b]).map_err(fail)?;
                }
            }
            if resized {
                f.set_len(data.len() as u64).map_err(fail)?;
            }
            return Ok(());
        }
        let _ = (dirty, resized);
        super::write_file(&self.path, data)
    }
}

/// Methods that change a stream (refused on a file opened for reading).
pub const WRITE_METHODS: &[&str] =
    &["writestr", "writebinstr", "writetext", "writeline", "writenum", "write", "writebyte", "copyfrom", "extractres", "memcopyfrom", "saveudtarray", "writeudt"];

/// Largest stream allowed, so `Mem.Size = 1E12` fails cleanly.
const MAX_SIZE: usize = 1 << 31;

impl Drop for MemStream {
    /// A file stream that goes (freed, replaced by another Open) writes what
    /// it still has.
    fn drop(&mut self) {
        self.flush();
    }
}

impl MemStream {
    /// Writes at Position (a gap past the end filled with zeros; nothing
    /// before the start), Position after the bytes.
    pub fn write(&mut self, bytes: &[u8]) {
        let Ok(from) = usize::try_from(self.pos) else { return };
        let Some(end) = from.checked_add(bytes.len()).filter(|&e| e <= MAX_SIZE) else { return };
        let old_len = self.data.len();
        if end > old_len {
            self.data.resize(end, 0);
        }
        self.data[from..end].copy_from_slice(bytes);
        self.pos = end as i64;
        if let Some(sink) = self.file.as_mut() {
            // (kept, and written with the next ones: `FileSink::changed`)
            if let Err(e) = sink.changed(&self.data, from.min(old_len), end) {
                eprintln!("[rapidr] {e}");
            }
        }
    }

    /// Writes what the program changed in a file stream to its file (see
    /// [`FileSink`]); nothing for a memory stream.
    pub fn flush(&mut self) {
        if let Some(sink) = self.file.as_mut() {
            if let Err(e) = sink.flush(&self.data) {
                eprintln!("[rapidr] {e}");
            }
        }
    }

    /// Content as a string, one character per byte.
    pub fn text(&self) -> String {
        bytes_to_string(&self.data)
    }

    /// Where Position is in the data (None outside it: before the start or
    /// at / past the end).
    fn index(&self) -> Option<usize> {
        usize::try_from(self.pos).ok().filter(|&i| i < self.data.len())
    }

    /// Up to `n` bytes from Position, Position after them: none (and
    /// Position left) outside the data.
    pub fn read(&mut self, n: usize) -> Vec<u8> {
        let Some(from) = self.index() else { return Vec::new() };
        let end = from.saturating_add(n).min(self.data.len());
        self.pos = end as i64;
        self.data[from..end].to_vec()
    }

    /// `ReadBinStr(n)`: always `n` characters (RapidQ's), spaces where the
    /// stream has no more bytes; none for `n` ≤ 0.
    pub fn read_bin_str(&mut self, n: i64) -> String {
        let n = usize::try_from(n).unwrap_or(0).min(MAX_SIZE);
        let mut bytes = self.read(n);
        bytes.resize(n, b' ');
        bytes_to_string(&bytes)
    }

    /// `ReadStr(n)` and `Read(S$)`: a memory stream's is [`Self::read_bin_str`];
    /// a QFILESTREAM's has one more character, a space, after them —
    /// `ReadStr(0)` too, not a negative count's (RC.EXE: every count, at the
    /// start, the end and past it, after ReadLine; ReadBinStr doesn't).
    pub fn read_str(&mut self, n: i64) -> String {
        let mut s = self.read_bin_str(n);
        if self.file.is_some() && n >= 0 && (n as usize) < MAX_SIZE {
            s.push(' ');
        }
        s
    }

    /// `ReadLine` (RC.EXE, both kinds of stream): the bytes up to the next
    /// LF, less one CR right before it (other CRs stay, and a last line's
    /// without an LF); Position after the LF. A NUL before the LF ends the
    /// text there and Position goes to the end of the stream. Nothing (and
    /// Position left) outside the data.
    pub fn read_line(&mut self) -> String {
        let Some(from) = self.index() else { return String::new() };
        let rest = &self.data[from..];
        let lf = rest.iter().position(|&b| b == b'\n');
        let mut line = rest[..lf.unwrap_or(rest.len())].to_vec();
        if let Some(nul) = line.iter().position(|&b| b == 0) {
            line.truncate(nul);
            self.pos = self.data.len() as i64;
            return bytes_to_string(&line);
        }
        self.pos = (from + lf.map_or(rest.len(), |i| i + 1)) as i64;
        if lf.is_some() && line.last() == Some(&b'\r') {
            line.pop();
        }
        bytes_to_string(&line)
    }

    pub fn set_size(&mut self, size: i64) {
        let old_len = self.data.len() as i64;
        self.data.resize((size.max(0) as usize).min(MAX_SIZE), 0);
        let len = self.data.len();
        // (RapidQ's: a Position past the new end goes to the old one)
        if self.pos > len as i64 {
            self.pos = old_len;
        }
        if let Some(sink) = self.file.as_mut() {
            // (a new length: written at once, with what came before)
            sink.resized = true;
            if let Err(e) = sink.flush(&self.data) {
                eprintln!("[rapidr] {e}");
            }
        }
    }

    /// Position anywhere (RapidQ's): past the end, and before the start on
    /// a memory stream (a file's stays where it was).
    pub fn set_position(&mut self, pos: i64) {
        if pos >= 0 || self.file.is_none() {
            self.pos = pos;
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.data.len() as i64
    }

    /// Lines counted as RapidQ does (RC.EXE): the LFs in the stream (so
    /// CRLF is one; a last line without one isn't counted, nor a lone CR;
    /// LFs after a NUL are).
    pub fn line_count(&self) -> i64 {
        self.data.iter().filter(|&&b| b == b'\n').count() as i64
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "position" => v_int(self.pos),
            "size" => v_int(self.data.len() as i64),
            "eof" => v_int(if self.at_end() { -1 } else { 0 }),
            "filename" => v_str(self.file.as_ref().map_or("", |f| f.path.as_str())),
            // RapidR extension: the whole content as a string.
            "text" => Value::from(self.text()),
            "linecount" => v_int(self.line_count()),
            // (RapidQ's SetSize is a property only written: RC.EXE reads
            // it as nothing)
            "setsize" => v_str(""),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "position" => self.set_position(val.to_i64()),
            // (QMEMORYSTREAM's `SetSize = n`, RC.EXE: Size's other name)
            "size" | "setsize" => self.set_size(val.to_i64()),
            _ => return false,
        }
        true
    }

    /// Methods that only touch this stream. `None` = not one of them.
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        Some(match method {
            "close" => {
                // (what was written goes to the file first)
                self.flush();
                *self = Self::default();
                Value::Null
            }
            // QMEMORYSTREAM's Clear (RC.EXE: Size 0, Position 0).
            "clear" if self.file.is_none() => {
                self.data.clear();
                self.pos = 0;
                Value::Null
            }
            // RapidR extensions: the rest of the stream (nothing when
            // Position is outside it); `Read(n)` bytes.
            "readall" => Value::from(bytes_to_string(&self.read(usize::MAX))),
            "read" if !args.is_empty() => Value::from(bytes_to_string(&self.read(arg(0).to_i64().max(0) as usize))),
            // `Stream.Read(var)`, compiled as `var = Stream.__read(var)`: as
            // many bytes as the variable's type takes (a string: its length).
            "__read" => match arg(0) {
                Value::String(s) => Value::from(self.read_str(s.chars().count() as i64)),
                Value::Double(_) => read_number(&self.read(8), 8),
                _ => read_number(&self.read(4), 4),
            },
            // (the count is of characters: see `codec::fit_characters` —
            // `WriteStr(S, LEN(S))` must write every character of S)
            "writestr" | "writebinstr" => {
                let text = arg(0).to_string_val();
                let bytes = if args.len() > 1 {
                    string_to_bytes(&fit_characters(&text, arg(1).to_i64().max(0) as usize))
                } else {
                    string_to_bytes(&text)
                };
                self.write(&bytes);
                Value::Null
            }
            // RapidR extensions, for text on purpose: `WriteText(S)` writes
            // S as UTF-8 whatever it holds (WriteStr writes a string of
            // characters up to U+00FF as bytes, as RapidQ does);
            // `ReadAllText` reads the rest of the stream as text — UTF-8 (a
            // BOM dropped), else Windows-1252.
            "writetext" => {
                self.write(arg(0).to_string_val().as_bytes());
                Value::Null
            }
            "readalltext" => v_str(&decode_text(&self.read(usize::MAX))),
            "writeline" | "writeln" => {
                let mut bytes = string_to_bytes(&arg(0).to_string_val());
                bytes.extend_from_slice(b"\r\n");
                self.write(&bytes);
                Value::Null
            }
            "writenum" => {
                let kind = if args.len() > 1 { arg(1).to_i64() } else { 4 };
                self.write(&number_bytes(&arg(0), kind));
                Value::Null
            }
            // Generic write: the storage an INTEGER, DOUBLE or STRING takes.
            "write" => {
                let bytes = match arg(0) {
                    Value::Double(_) => number_bytes(&arg(0), 8),
                    Value::String(s) => string_to_bytes(&s),
                    v => number_bytes(&v, 4),
                };
                self.write(&bytes);
                Value::Null
            }
            "readstr" => Value::from(self.read_str(arg(0).to_i64())),
            "readbinstr" => Value::from(self.read_bin_str(arg(0).to_i64())),
            "readline" | "readln" => Value::from(self.read_line()),
            "readnum" => {
                let kind = if args.is_empty() { 4 } else { arg(0).to_i64() };
                read_number(&self.read(number_size(kind)), kind)
            }
            "seek" => {
                let offset = arg(0).to_i64();
                let base = match arg(1).to_i64() {
                    1 => self.pos,
                    2 => self.data.len() as i64,
                    _ => 0,
                };
                self.set_position(base.saturating_add(offset));
                v_int(self.pos)
            }
            "eof" => v_int(if self.at_end() { -1 } else { 0 }),
            // QFILESTREAM's ReadByte / WriteByte (RC.EXE): one byte, its
            // low 8 bits written; past the end ReadByte gives 26 (^Z, DOS's
            // end of file) and leaves Position.
            "writebyte" => {
                self.write(&[arg(0).to_i64() as u8]);
                Value::Null
            }
            "readbyte" => match self.index() {
                Some(i) => {
                    self.pos += 1;
                    v_int(self.data[i] as i64)
                }
                None => v_int(26),
            },
            // `Mem.ExtractRes(Resource(0))`: the resource's bytes, written
            // at the position (rapidr_value::resources).
            "extractres" => {
                match crate::resources::bytes(arg(0).to_i64()) {
                    Some(b) => self.write(&b),
                    None => eprintln!("[rapidr] ExtractRes: no resource {}", arg(0).to_i64()),
                }
                Value::Null
            }
            _ => return None,
        })
    }
}

/// Bytes each `Num_*` type takes (RAPIDQ.INC: Num_BYTE = 1, Num_SHORT = 2,
/// Num_WORD = 3, Num_LONG = 4, Num_DWORD = 5, Num_SINGLE = 6, Num_DOUBLE = 8).
pub(super) fn number_size(kind: i64) -> usize {
    match kind {
        1 => 1,
        2 | 3 => 2,
        8 => 8,
        _ => 4,
    }
}

pub(super) fn number_bytes(v: &Value, kind: i64) -> Vec<u8> {
    match kind {
        1 => vec![v.to_i64() as u8],
        2 | 3 => (v.to_i64() as u16).to_le_bytes().to_vec(),
        6 => (v.to_f64() as f32).to_le_bytes().to_vec(),
        8 => v.to_f64().to_le_bytes().to_vec(),
        _ => (v.to_i64() as u32).to_le_bytes().to_vec(),
    }
}

pub(super) fn read_number(b: &[u8], kind: i64) -> Value {
    let mut buf = [0u8; 8];
    buf[..b.len()].copy_from_slice(b);
    match kind {
        1 => v_int(buf[0] as i64),
        2 => v_int(i16::from_le_bytes([buf[0], buf[1]]) as i64),
        3 => v_int(u16::from_le_bytes([buf[0], buf[1]]) as i64),
        5 => v_int(u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as i64),
        6 => v_dbl(f32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as f64),
        8 => v_dbl(f64::from_le_bytes(buf)),
        _ => v_int(i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(m: &mut MemStream, method: &str, args: &[Value]) -> Value {
        m.call(method, args).unwrap()
    }

    /// Robert's Solar System 3D (2.121.3): Studio's Apply wrote a new
    /// translation file with `f.WriteStr(text, LEN(text))`. A text with
    /// Chinese came out cut short (the count was characters, applied to the
    /// UTF-8 bytes) and an `é` or `·` beside it as one Latin-1 byte, so the
    /// file wasn't UTF-8 and the editor later opened it as Latin-1.
    #[test]
    fn writestr_of_len_writes_all_of_a_unicode_text_as_utf8() {
        let text = "msgstr \"太阳系 3D · Español café\"\nmsgstr \"语言\"\n";
        let mut m = MemStream::default();
        call(&mut m, "writestr", &[v_str(text), v_int(text.chars().count() as i64)]);
        assert_eq!(m.data, text.as_bytes(), "every character, as UTF-8, none lost or cut");
        assert!(std::str::from_utf8(&m.data).is_ok());
        // (with no count, and WriteLine: the same text, the same bytes)
        let mut n = MemStream::default();
        call(&mut n, "writestr", &[v_str(text)]);
        assert_eq!(n.data, text.as_bytes());
        let mut l = MemStream::default();
        call(&mut l, "writeline", &[v_str("中文 é")]);
        assert_eq!(l.data, "中文 é\r\n".as_bytes());
        let mut w = MemStream::default();
        call(&mut w, "write", &[v_str("中文 é")]);
        assert_eq!(w.data, "中文 é".as_bytes());
    }

    /// RapidQ's strings are bytes: characters up to 255 are written as the
    /// bytes they are (CHR$ and ReadStr's), so binary data built or copied
    /// with strings is exact. WriteStr's count pads with NULs and cuts.
    #[test]
    fn writestr_of_byte_strings_stays_bytes() {
        let mut m = MemStream::default();
        call(&mut m, "writestr", &[v_str("Espa\u{f1}ol \u{e9}\u{ff}\u{0}\u{80}")]);
        assert_eq!(m.data, [b'E', b's', b'p', b'a', 0xF1, b'o', b'l', b' ', 0xE9, 0xFF, 0, 0x80]);
        let mut p = MemStream::default();
        call(&mut p, "writestr", &[v_str("ab"), v_int(4)]);
        call(&mut p, "writestr", &[v_str("abcdef"), v_int(3)]);
        assert_eq!(p.data, b"ab\0\0abc");
        // (read back as RapidQ reads: a character a byte)
        p.set("position", &v_int(0));
        assert_eq!(call(&mut p, "readstr", &[v_int(2)]).to_string_val(), "ab");
        // (the count is of characters in a Unicode text too)
        let mut u = MemStream::default();
        call(&mut u, "writestr", &[v_str("中文字"), v_int(2)]);
        assert_eq!(u.data, "中文".as_bytes());
        call(&mut u, "writestr", &[v_str("é中"), v_int(3)]);
        assert_eq!(&u.data[6..], "é中\0".as_bytes());
    }

    /// WriteText always writes UTF-8; ReadAllText reads UTF-8 (a BOM
    /// dropped), else Windows-1252 — what an older program's text file is.
    #[test]
    fn write_and_read_text_round_trip() {
        let text = "Español, 中文(简体), emoji 😀, € and ·";
        let mut m = MemStream::default();
        call(&mut m, "writetext", &[v_str(text)]);
        assert_eq!(m.data, text.as_bytes());
        m.set("position", &v_int(0));
        assert_eq!(call(&mut m, "readalltext", &[]).to_string_val(), text);
        // (an accent-only text too: UTF-8, where WriteStr would write bytes)
        let mut a = MemStream::default();
        call(&mut a, "writetext", &[v_str("café")]);
        assert_eq!(a.data, "café".as_bytes());
        // (a BOM, then an ANSI file)
        let mut b = MemStream::default();
        b.write(b"\xEF\xBB\xBFna\xC3\xAFve");
        b.set("position", &v_int(0));
        assert_eq!(call(&mut b, "readalltext", &[]).to_string_val(), "naïve");
        let mut c = MemStream::default();
        c.write(b"caf\xE9 \x80 \x96 \xB7");
        c.set("position", &v_int(0));
        assert_eq!(call(&mut c, "readalltext", &[]).to_string_val(), "café € – ·");
        // (ReadAll and ReadStr stay bytes, a character a byte)
        let mut d = MemStream::default();
        call(&mut d, "writetext", &[v_str("é")]);
        d.set("position", &v_int(0));
        assert_eq!(call(&mut d, "readall", &[]).to_string_val(), "\u{c3}\u{a9}");
    }

    #[test]
    fn manual_example() {
        let mut m = MemStream::default();
        call(&mut m, "writestr", &[v_str("Hello world!"), v_int(12)]);
        assert_eq!(m.get("size").unwrap().to_i64(), 12);
        m.set("position", &v_int(0));
        assert_eq!(call(&mut m, "readstr", &[v_int(12)]).to_string_val(), "Hello world!");
        call(&mut m, "close", &[]);
        assert_eq!(m.get("size").unwrap().to_i64(), 0);
    }

    #[test]
    fn numbers_lines_and_seek() {
        let mut m = MemStream::default();
        call(&mut m, "writenum", &[v_int(-2), v_int(2)]);
        call(&mut m, "writenum", &[v_dbl(1.5), v_int(8)]);
        call(&mut m, "write", &[v_int(70000)]);
        call(&mut m, "seek", &[v_int(0), v_int(0)]);
        assert_eq!(call(&mut m, "readnum", &[v_int(2)]).to_i64(), -2);
        assert_eq!(call(&mut m, "readnum", &[v_int(8)]).to_f64(), 1.5);
        assert_eq!(call(&mut m, "readnum", &[v_int(4)]).to_i64(), 70000);
        call(&mut m, "close", &[]);
        call(&mut m, "writeline", &[v_str("one")]);
        call(&mut m, "writestr", &[v_str("two")]);
        // (RC.EXE: the LFs; "two" has none)
        assert_eq!(m.line_count(), 1);
        m.set_position(0);
        assert_eq!(call(&mut m, "readline", &[]).to_string_val(), "one");
        assert_eq!(call(&mut m, "readline", &[]).to_string_val(), "two");
        assert_eq!(call(&mut m, "seek", &[v_int(-1), v_int(2)]).to_i64(), 7);
        // (RapidQ's: a Position past the new end goes to the old end)
        m.set_size(2);
        assert_eq!(m.pos, 8);
    }

    /// A file stream keeps its writes (one run) and writes them together:
    /// at a flush, a write elsewhere, every 64 KB, Close and when it goes —
    /// and the file then holds the stream's bytes, as when each write went
    /// straight through.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn file_writes_are_kept_then_written_together() {
        let dir = std::env::temp_dir().join(format!("rapidr-memstream-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("kept.bin");
        let open = || std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&path).unwrap();
        let on_disk = || std::fs::read(&path).unwrap();
        let mut m = MemStream { data: Vec::new(), pos: 0, file: Some(FileSink::new(path.to_str().unwrap(), true, Some(open()))) };
        for b in 0..100u8 {
            m.write(&[b]);
        }
        assert!(on_disk().is_empty(), "kept, not written byte by byte");
        assert!(pending());
        m.flush();
        assert_eq!(on_disk(), (0..100u8).collect::<Vec<_>>());
        // (back to the start, then the end: the first run written when the
        // second isn't next to it)
        m.pos = 0;
        m.write(b"AB");
        m.pos = 100;
        m.write(b"YZ");
        assert_eq!(&on_disk()[..2], b"AB");
        assert_eq!(on_disk().len(), 100);
        // (a gap past the end: zeros, written with the bytes after it)
        m.pos = 110;
        m.write(b"!");
        m.flush();
        assert_eq!(on_disk(), m.data);
        assert_eq!(&on_disk()[100..111], b"YZ\0\0\0\0\0\0\0\0!");
        // (64 KB kept: written without being asked)
        m.pos = 0;
        let big = vec![7u8; FLUSH_AT];
        m.write(&big);
        assert_eq!(&on_disk()[..FLUSH_AT], &big[..]);
        // (Size cut: the file follows at once)
        m.set_size(5);
        assert_eq!(on_disk(), vec![7u8; 5]);
        // (Close, and a stream that goes, write what they keep)
        m.pos = 5;
        m.write(b"xy");
        call(&mut m, "close", &[]);
        assert_eq!(on_disk(), b"\x07\x07\x07\x07\x07xy");
        let mut m = MemStream { data: on_disk(), pos: 7, file: Some(FileSink::new(path.to_str().unwrap(), true, Some(std::fs::OpenOptions::new().write(true).open(&path).unwrap()))) };
        m.write(b"end");
        drop(m);
        assert_eq!(on_disk(), b"\x07\x07\x07\x07\x07xyend");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_reads_and_lines_as_rapidq() {
        let file = || FileSink::new(
            "x",
            false,
            #[cfg(not(target_arch = "wasm32"))]
            None,
        );
        let mut f = MemStream { data: b"ab\0cd\r\nef".to_vec(), pos: 0, file: Some(file()) };
        // a file's ReadStr: one more character, a space; ReadBinStr: n
        assert_eq!(call(&mut f, "readstr", &[v_int(2)]).to_string_val(), "ab ");
        assert_eq!(f.pos, 2);
        f.set_position(0);
        assert_eq!(call(&mut f, "readstr", &[v_int(0)]).to_string_val(), " ");
        assert_eq!(call(&mut f, "readstr", &[v_int(-1)]).to_string_val(), "");
        assert_eq!(call(&mut f, "readbinstr", &[v_int(2)]).to_string_val(), "ab");
        // ReadLine: a NUL before the LF ends the text, Position to the end
        f.set_position(0);
        assert_eq!((call(&mut f, "readline", &[]).to_string_val(), f.pos), ("ab".to_string(), 9));
        f.set_position(3);
        assert_eq!((call(&mut f, "readline", &[]).to_string_val(), f.pos), ("cd".to_string(), 7));
        // one CR before the LF goes; others, and a last line's, stay
        let mut m = MemStream { data: b"x\r\r\ny\r".to_vec(), pos: 0, file: None };
        assert_eq!(call(&mut m, "readline", &[]).to_string_val(), "x\r");
        assert_eq!(call(&mut m, "readline", &[]).to_string_val(), "y\r");
        assert_eq!(m.line_count(), 1);
        assert_eq!(call(&mut m, "readstr", &[v_int(0)]).to_string_val(), "");
    }

    #[test]
    fn out_of_the_data_as_rapidq() {
        let mut m = MemStream::default();
        call(&mut m, "writestr", &[v_str("ABCDEFGHIJ"), v_int(10)]);
        // ReadAll past the start (it once overflowed: pos + usize::MAX)
        m.set_position(3);
        assert_eq!(call(&mut m, "readall", &[]).to_string_val(), "DEFGHIJ");
        assert_eq!(m.pos, 10);
        assert_eq!(call(&mut m, "readall", &[]).to_string_val(), "");
        // ReadStr: always n characters, spaces where there are no bytes
        m.set_position(8);
        assert_eq!(call(&mut m, "readstr", &[v_int(4)]).to_string_val(), "IJ  ");
        assert_eq!(m.pos, 10);
        assert_eq!(call(&mut m, "readstr", &[v_int(-2)]).to_string_val(), "");
        // Position anywhere; no bytes outside the data, Position left
        m.set_position(20);
        assert_eq!(call(&mut m, "readstr", &[v_int(2)]).to_string_val(), "  ");
        assert_eq!((m.pos, call(&mut m, "readall", &[]).to_string_val()), (20, String::new()));
        m.set_position(-3);
        assert_eq!(call(&mut m, "readline", &[]).to_string_val(), "");
        call(&mut m, "writestr", &[v_str("XY"), v_int(2)]);
        assert_eq!((m.pos, m.data.len()), (-3, 10));
        // a write past the end: zeros in the gap
        m.set_position(12);
        call(&mut m, "writestr", &[v_str("Z"), v_int(1)]);
        assert_eq!(&m.data[9..], b"J\0\0Z");
        // every count, however large
        m.set_position(5);
        assert_eq!(m.read(usize::MAX).len(), 8);
        m.set_position(i64::MAX);
        assert!(m.read(usize::MAX).is_empty());
        assert_eq!(call(&mut m, "seek", &[v_int(i64::MAX), v_int(1)]).to_i64(), i64::MAX);
    }
}
