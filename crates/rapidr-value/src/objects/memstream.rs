//! QMEMORYSTREAM (manual, Appendix A): a growable byte buffer with a
//! position that reads and writes start from.

use super::codec::{bytes_to_string, string_to_bytes};
use crate::{v_dbl, v_int, v_str, Value};

#[derive(Debug, Clone, Default)]
pub struct MemStream {
    pub data: Vec<u8>,
    pub pos: usize,
}

/// Largest stream allowed, so `Mem.Size = 1E12` fails cleanly.
const MAX_SIZE: usize = 1 << 31;

impl MemStream {
    pub fn write(&mut self, bytes: &[u8]) {
        let end = self.pos + bytes.len();
        if end > MAX_SIZE {
            return;
        }
        if end > self.data.len() {
            self.data.resize(end, 0);
        }
        self.data[self.pos..end].copy_from_slice(bytes);
        self.pos = end;
    }

    pub fn read(&mut self, n: usize) -> Vec<u8> {
        let end = (self.pos + n).min(self.data.len());
        let out = self.data[self.pos.min(end)..end].to_vec();
        self.pos = end;
        out
    }

    pub fn set_size(&mut self, size: i64) {
        self.data.resize((size.max(0) as usize).min(MAX_SIZE), 0);
        self.pos = self.pos.min(self.data.len());
    }

    pub fn set_position(&mut self, pos: i64) {
        self.pos = (pos.max(0) as usize).min(self.data.len());
    }

    /// Lines counted as RapidQ does: by LF (so CRLF is one line), plus an
    /// unterminated last line.
    pub fn line_count(&self) -> i64 {
        let lf = self.data.iter().filter(|&&b| b == b'\n').count() as i64;
        lf + i64::from(self.data.last().is_some_and(|&b| b != b'\n'))
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "position" => v_int(self.pos as i64),
            "size" => v_int(self.data.len() as i64),
            "linecount" => v_int(self.line_count()),
            // There are no raw memory addresses in RapidR.
            "pointer" => v_int(0),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "position" => self.set_position(val.to_i64()),
            "size" => self.set_size(val.to_i64()),
            _ => return false,
        }
        true
    }

    /// Methods that only touch this stream. `None` = not one of them.
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        Some(match method {
            "close" => {
                *self = Self::default();
                Value::Null
            }
            "writestr" | "writebinstr" => {
                let mut bytes = string_to_bytes(&arg(0).to_string_val());
                if args.len() > 1 {
                    bytes.resize(arg(1).to_i64().max(0) as usize, 0);
                }
                self.write(&bytes);
                Value::Null
            }
            "writeline" => {
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
            "readstr" | "readbinstr" => v_str(&bytes_to_string(&self.read(arg(0).to_i64().max(0) as usize))),
            "readline" => {
                let rest = &self.data[self.pos..];
                let len = rest.iter().position(|&b| b == b'\n').map_or(rest.len(), |i| i + 1);
                let mut line = self.read(len);
                while matches!(line.last(), Some(b'\n' | b'\r')) {
                    line.pop();
                }
                v_str(&bytes_to_string(&line))
            }
            "readnum" => {
                let kind = if args.is_empty() { 4 } else { arg(0).to_i64() };
                read_number(&self.read(number_size(kind)), kind)
            }
            "seek" => {
                let offset = arg(0).to_i64();
                let base = match arg(1).to_i64() {
                    1 => self.pos as i64,
                    2 => self.data.len() as i64,
                    _ => 0,
                };
                self.set_position(base + offset);
                v_int(self.pos as i64)
            }
            "eof" => v_int(if self.pos >= self.data.len() { -1 } else { 0 }),
            _ => return None,
        })
    }
}

/// Bytes each `Num_*` type takes (RAPIDQ.INC: Num_BYTE = 1, Num_SHORT = 2,
/// Num_WORD = 3, Num_LONG = 4, Num_DWORD = 5, Num_SINGLE = 6, Num_DOUBLE = 8).
fn number_size(kind: i64) -> usize {
    match kind {
        1 => 1,
        2 | 3 => 2,
        8 => 8,
        _ => 4,
    }
}

fn number_bytes(v: &Value, kind: i64) -> Vec<u8> {
    match kind {
        1 => vec![v.to_i64() as u8],
        2 | 3 => (v.to_i64() as u16).to_le_bytes().to_vec(),
        6 => (v.to_f64() as f32).to_le_bytes().to_vec(),
        8 => v.to_f64().to_le_bytes().to_vec(),
        _ => (v.to_i64() as u32).to_le_bytes().to_vec(),
    }
}

fn read_number(b: &[u8], kind: i64) -> Value {
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
        assert_eq!(m.line_count(), 2);
        m.set_position(0);
        assert_eq!(call(&mut m, "readline", &[]).to_string_val(), "one");
        assert_eq!(call(&mut m, "readline", &[]).to_string_val(), "two");
        assert_eq!(call(&mut m, "seek", &[v_int(-1), v_int(2)]).to_i64(), 7);
        m.set_size(2);
        assert_eq!(m.pos, 2);
    }
}
