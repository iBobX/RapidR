//! Binary (de)serialization for [`Module`].

use crate::{Const, Function, Module, Param, MAGIC, VERSION};

#[derive(Debug)]
pub enum Error {
    BadMagic,
    BadVersion(u16),
    Truncated,
    InvalidUtf8,
    InvalidConstTag(u8),
    Io(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BadMagic => write!(f, "not a RRBC file (bad magic)"),
            Error::BadVersion(v) => write!(f, "unsupported RRBC version {v}"),
            Error::Truncated => write!(f, "truncated RRBC file"),
            Error::InvalidUtf8 => write!(f, "invalid UTF-8 in RRBC string"),
            Error::InvalidConstTag(t) => write!(f, "invalid const tag 0x{t:02X}"),
            Error::Io(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {}

impl Module {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        // flags: bit 0 = a source map follows the entry point.
        let has_map = !self.source_map.runs.is_empty();
        out.extend_from_slice(&(has_map as u16).to_le_bytes());

        // consts
        write_u32(&mut out, self.consts.len() as u32);
        for c in &self.consts {
            write_const(&mut out, c);
        }

        // strings
        write_u32(&mut out, self.strings.len() as u32);
        for s in &self.strings {
            write_str(&mut out, s);
        }

        // functions
        write_u32(&mut out, self.functions.len() as u32);
        for f in &self.functions {
            write_function(&mut out, f);
        }

        // entry
        write_u32(&mut out, self.entry);

        if has_map {
            write_u32(&mut out, self.source_map.files.len() as u32);
            for f in &self.source_map.files {
                write_str(&mut out, f);
            }
            write_u32(&mut out, self.source_map.runs.len() as u32);
            for &(a, b, c, d) in &self.source_map.runs {
                for v in [a, b, c, d] {
                    write_u32(&mut out, v);
                }
            }
        }
        out
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Module, Error> {
        let mut r = Reader { buf, pos: 0 };
        let magic = r.read_n(4)?;
        if magic != MAGIC {
            return Err(Error::BadMagic);
        }
        let version = r.read_u16()?;
        if version != VERSION {
            return Err(Error::BadVersion(version));
        }
        let flags = r.read_u16()?;

        let n_consts = r.read_count(1)?;
        let mut consts = Vec::with_capacity(n_consts);
        for _ in 0..n_consts {
            consts.push(read_const(&mut r)?);
        }

        let n_strings = r.read_count(4)?;
        let mut strings = Vec::with_capacity(n_strings);
        for _ in 0..n_strings {
            strings.push(read_str(&mut r)?);
        }

        let n_funcs = r.read_count(4)?;
        let mut functions = Vec::with_capacity(n_funcs);
        for _ in 0..n_funcs {
            functions.push(read_function(&mut r)?);
        }

        let entry = r.read_u32()?;
        let mut source_map = crate::SourceMap::default();
        if flags & 1 != 0 {
            let n_files = r.read_count(4)?;
            for _ in 0..n_files {
                source_map.files.push(read_str(&mut r)?);
            }
            let n_runs = r.read_count(16)?;
            for _ in 0..n_runs {
                source_map.runs.push((r.read_u32()?, r.read_u32()?, r.read_u32()?, r.read_u32()?));
            }
        }
        Ok(Module { consts, strings, functions, entry, source_map })
    }
}

// ---------------- writers ----------------

fn write_u16(out: &mut Vec<u8>, v: u16) { out.extend_from_slice(&v.to_le_bytes()); }
fn write_u32(out: &mut Vec<u8>, v: u32) { out.extend_from_slice(&v.to_le_bytes()); }
fn write_i32(out: &mut Vec<u8>, v: i32) { out.extend_from_slice(&v.to_le_bytes()); }
fn write_i64(out: &mut Vec<u8>, v: i64) { out.extend_from_slice(&v.to_le_bytes()); }
fn write_f64(out: &mut Vec<u8>, v: f64) { out.extend_from_slice(&v.to_le_bytes()); }

fn write_str(out: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    write_u32(out, b.len() as u32);
    out.extend_from_slice(b);
}

fn write_const(out: &mut Vec<u8>, c: &Const) {
    match c {
        Const::Null => out.push(0),
        Const::Bool(b) => { out.push(1); out.push(if *b { 1 } else { 0 }); }
        Const::Int(n)  => { out.push(2); write_i64(out, *n); }
        Const::Double(n) => { out.push(3); write_f64(out, *n); }
        Const::Str(s) => { out.push(4); write_str(out, s); }
    }
}

fn write_function(out: &mut Vec<u8>, f: &Function) {
    write_str(out, &f.name);
    write_u32(out, f.params.len() as u32);
    for p in &f.params {
        write_str(out, &p.name);
        out.push(if p.by_ref { 1 } else { 0 });
    }
    write_u32(out, f.n_locals);
    write_u32(out, f.code.len() as u32);
    out.extend_from_slice(&f.code);
    write_u32(out, f.line_info.len() as u32);
    for (off, line) in &f.line_info {
        write_u32(out, *off);
        write_u32(out, *line);
    }
    write_u32(out, f.local_names.len() as u32);
    for name in &f.local_names {
        write_str(out, name);
    }
    let _ = write_u16; let _ = write_i32; // suppress unused warnings; reserved
}

// ---------------- reader ----------------

struct Reader<'a> { buf: &'a [u8], pos: usize }

impl<'a> Reader<'a> {
    fn read_n(&mut self, n: usize) -> Result<&'a [u8], Error> {
        // Checked: `n` comes from the file (and usize is 32-bit on wasm).
        let end = self.pos.checked_add(n).filter(|&e| e <= self.buf.len()).ok_or(Error::Truncated)?;
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    /// A count of items at least `item_size` bytes each: never more than
    /// the rest of the file could hold (so a corrupt count can't make the
    /// reader allocate gigabytes).
    fn read_count(&mut self, item_size: usize) -> Result<usize, Error> {
        let n = self.read_u32()? as usize;
        if n.saturating_mul(item_size.max(1)) > self.buf.len() - self.pos {
            return Err(Error::Truncated);
        }
        Ok(n)
    }
    fn read_u8(&mut self) -> Result<u8, Error> { Ok(self.read_n(1)?[0]) }
    fn read_u16(&mut self) -> Result<u16, Error> {
        let b = self.read_n(2)?; Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn read_u32(&mut self) -> Result<u32, Error> {
        let b = self.read_n(4)?; Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn read_i64(&mut self) -> Result<i64, Error> {
        let b = self.read_n(8)?;
        Ok(i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }
    fn read_f64(&mut self) -> Result<f64, Error> {
        let b = self.read_n(8)?;
        Ok(f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }
}

fn read_str(r: &mut Reader) -> Result<String, Error> {
    let n = r.read_u32()? as usize;
    let bytes = r.read_n(n)?.to_vec();
    String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)
}

fn read_const(r: &mut Reader) -> Result<Const, Error> {
    let tag = r.read_u8()?;
    Ok(match tag {
        0 => Const::Null,
        1 => Const::Bool(r.read_u8()? != 0),
        2 => Const::Int(r.read_i64()?),
        3 => Const::Double(r.read_f64()?),
        4 => Const::Str(read_str(r)?),
        t => return Err(Error::InvalidConstTag(t)),
    })
}

fn read_function(r: &mut Reader) -> Result<Function, Error> {
    let name = read_str(r)?;
    let n_params = r.read_count(5)?;
    let mut params = Vec::with_capacity(n_params);
    for _ in 0..n_params {
        let pname = read_str(r)?;
        let by_ref = r.read_u8()? != 0;
        params.push(Param { name: pname, by_ref });
    }
    // Slots are addressed with 16 bits (LoadLocal); more is a corrupt file,
    // which would otherwise make every call allocate that many.
    let n_locals = r.read_u32()?;
    if n_locals > u16::MAX as u32 + 1 {
        return Err(Error::Truncated);
    }
    let code_len = r.read_u32()? as usize;
    let code = r.read_n(code_len)?.to_vec();
    let n_lines = r.read_count(8)?;
    let mut line_info = Vec::with_capacity(n_lines);
    for _ in 0..n_lines {
        let off = r.read_u32()?;
        let line = r.read_u32()?;
        line_info.push((off, line));
    }
    let n_local_names = r.read_count(4)?;
    let mut local_names = Vec::with_capacity(n_local_names);
    for _ in 0..n_local_names {
        local_names.push(read_str(r)?);
    }
    Ok(Function { name, params, n_locals, code, line_info, local_names })
}

#[cfg(test)]
mod tests {
    #[test]
    fn corrupt_counts_are_rejected_not_allocated() {
        // A header, then a const count of four billion.
        let mut bytes = crate::MAGIC.to_vec();
        bytes.extend_from_slice(&crate::VERSION.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(crate::Module::from_bytes(&bytes).is_err());
    }

    #[test]
    fn source_map_round_trip() {
        let mut m = crate::Module::new();
        m.source_map = crate::SourceMap::from_origins(
            "/home/me/prog.bas",
            [(Some("/inc/RAPIDQ.INC"), 1), (Some("/inc/RAPIDQ.INC"), 2), (None, 3), (None, 4), (Some("C:\\lib\\x.inc"), 9)],
        );
        assert_eq!(m.source_map.files, ["RAPIDQ.INC", "prog.bas", "x.inc"], "names only, never paths");
        let back = crate::Module::from_bytes(&m.to_bytes()).unwrap();
        assert_eq!(back.source_map, m.source_map);
        assert_eq!(back.source_map.locate(2), Some(("RAPIDQ.INC", 2)));
        assert_eq!(back.source_map.locate(4), Some(("prog.bas", 4)));
        assert_eq!(back.source_map.locate(5), Some(("x.inc", 9)));
        assert_eq!(back.source_map.locate(6), None);
    }

    use crate::*;

    #[test]
    fn round_trip_empty() {
        let m = Module::new();
        let bytes = m.to_bytes();
        let m2 = Module::from_bytes(&bytes).unwrap();
        assert_eq!(m2.consts.len(), 0);
        assert_eq!(m2.functions.len(), 0);
    }

    #[test]
    fn round_trip_full() {
        let mut m = Module::new();
        m.add_const(Const::Int(42));
        m.add_const(Const::Str("hello".into()));
        m.add_string("RFORM");
        let mut f = Function::default();
        f.name = "main".into();
        f.n_locals = 2;
        f.code = vec![Op::LoadConst as u8, 0, 0, 0, 0, Op::Halt as u8];
        f.line_info.push((0, 1));
        f.local_names.push("foo".to_string());
        m.add_function(f);
        let bytes = m.to_bytes();
        let m2 = Module::from_bytes(&bytes).unwrap();
        assert_eq!(m2.consts, m.consts);
        assert_eq!(m2.strings, m.strings);
        assert_eq!(m2.functions[0].code, m.functions[0].code);
        assert_eq!(m2.functions[0].line_info, m.functions[0].line_info);
        assert_eq!(m2.functions[0].local_names, m.functions[0].local_names);
    }
}
