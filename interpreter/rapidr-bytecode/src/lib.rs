//! RapidR bytecode format.
//!
//! A `.rrbc` file is the linear binary serialization of a [`Module`] —
//! a constant pool, a function table, and per-function instruction streams.
//!
//! ## File layout
//!
//! ```text
//! magic       : 4 bytes  "RRBC"
//! version     : u16 LE    (current = 1)
//! flags       : u16 LE    (reserved = 0)
//!
//! n_consts    : u32 LE
//! consts      : n_consts * Const
//!
//! n_strings   : u32 LE
//! strings     : n_strings * (u32 len + bytes)   ; identifier pool
//!
//! n_funcs     : u32 LE
//! funcs       : n_funcs * Function
//!
//! entry_fn    : u32 LE    ; index of the implicit __main
//! ```
//!
//! See [`Op`] for the opcode set and [`io`] for read/write helpers.

#![allow(clippy::needless_range_loop)]

pub mod builtins;
pub mod io;
pub mod op;

pub use op::Op;

use rapidr_value::Value;

/// A constant value encoded in the constant pool.
#[derive(Debug, Clone, PartialEq)]
pub enum Const {
    Null,
    Bool(bool),
    Int(i64),
    Double(f64),
    Str(String),
}

impl Const {
    pub fn to_value(&self) -> Value {
        match self {
            Const::Null => Value::Null,
            Const::Bool(b) => Value::Boolean(*b),
            Const::Int(n) => Value::Integer(*n),
            Const::Double(n) => Value::Double(*n),
            Const::Str(s) => Value::String(s.clone()),
        }
    }
}

/// A compiled function (a SUB, FUNCTION, or the implicit __main).
#[derive(Debug, Clone, Default)]
pub struct Function {
    /// Symbolic name (used by Host for diagnostics; resolution is by index).
    pub name: String,
    /// Number of parameters, BYREF flag per param.
    pub params: Vec<Param>,
    /// Number of local slots (including parameters).
    pub n_locals: u32,
    /// Bytecode instruction stream.
    pub code: Vec<u8>,
    /// Optional debug-info side table: instruction-offset → source-line.
    pub line_info: Vec<(u32, u32)>,
    /// Optional debug-info: names of local variable slots (1-to-1 mapping to slots)
    pub local_names: Vec<String>,
}

impl Function {
    pub fn get_line_for_ip(&self, ip: usize) -> Option<u32> {
        if self.line_info.is_empty() {
            return None;
        }
        let mut best_line = None;
        for &(off, line) in &self.line_info {
            if off as usize <= ip {
                best_line = Some(line);
            } else {
                break;
            }
        }
        best_line
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub by_ref: bool,
}

/// A complete bytecode module.
#[derive(Debug, Clone, Default)]
pub struct Module {
    pub consts: Vec<Const>,
    /// Identifier pool (component names, builtin names, etc.).
    pub strings: Vec<String>,
    pub functions: Vec<Function>,
    /// Index into `functions` for the entry point (__main).
    pub entry: u32,
    /// Where each line of the compiled (preprocessed) program came from, for
    /// run-time error messages; empty when unknown.
    pub source_map: SourceMap,
}

/// Lines of the compiled program → the file (name only, never its path) and
/// line they came from: runs of consecutive lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMap {
    pub files: Vec<String>,
    /// (first compiled line, file index, its line in that file, line count)
    pub runs: Vec<(u32, u32, u32, u32)>,
}

impl SourceMap {
    /// From one origin per compiled line (index = line − 1): a file name
    /// (None: the program itself, `main`) and its line there.
    pub fn from_origins<'a>(main: &str, origins: impl IntoIterator<Item = (Option<&'a str>, u32)>) -> SourceMap {
        let mut map = SourceMap::default();
        for (i, (file, line)) in origins.into_iter().enumerate() {
            let name = file.unwrap_or(main);
            let name = name.rsplit(['/', '\\']).next().unwrap_or(name).to_string();
            let idx = match map.files.iter().position(|f| *f == name) {
                Some(i) => i,
                None => {
                    map.files.push(name);
                    map.files.len() - 1
                }
            } as u32;
            let compiled = i as u32 + 1;
            match map.runs.last_mut() {
                Some(r) if r.1 == idx && r.0 + r.3 == compiled && r.2 + r.3 == line => r.3 += 1,
                _ => map.runs.push((compiled, idx, line, 1)),
            }
        }
        map
    }

    /// The file and line compiled line `line` came from.
    pub fn locate(&self, line: u32) -> Option<(&str, u32)> {
        let r = self.runs.iter().find(|r| (r.0..r.0.saturating_add(r.3)).contains(&line))?;
        Some((self.files.get(r.1 as usize)?.as_str(), r.2 + (line - r.0)))
    }
}

impl Module {
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a constant; returns its index.
    pub fn add_const(&mut self, c: Const) -> u32 {
        if let Some(i) = self.consts.iter().position(|x| x == &c) {
            return i as u32;
        }
        self.consts.push(c);
        (self.consts.len() - 1) as u32
    }

    /// Intern a string; returns its index in the string pool.
    pub fn add_string(&mut self, s: &str) -> u32 {
        if let Some(i) = self.strings.iter().position(|x| x == s) {
            return i as u32;
        }
        self.strings.push(s.to_string());
        (self.strings.len() - 1) as u32
    }

    /// Add a function; returns its index.
    pub fn add_function(&mut self, f: Function) -> u32 {
        self.functions.push(f);
        (self.functions.len() - 1) as u32
    }
}

pub const MAGIC: &[u8; 4] = b"RRBC";
pub const VERSION: u16 = 2;
