//! RapidR bytecode format.
//!
//! A `.rrbc` file is the linear binary serialization of a [`Module`] —
//! a constant pool, a function table, and per-function instruction streams.
//!
//! ## File layout
//!
//! ```text
//! magic       : 4 bytes  "RRBC"
//! version     : u16 LE    the format (current = 3; 2 is still read)
//! flags       : u16 LE    bit 0: a source map follows, bit 1: resources
//! -- format 3 on: the header, which every later format keeps as it is --
//! header_len  : u16 LE    bytes of header after this field (now 7)
//! min_runtime : 3 × u16 LE the oldest RapidR Runtime that runs it
//! app_type    : u8        [`AppType`] ($APPTYPE)
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
    /// The source line of the instruction at `ip`: the last `line_info`
    /// entry at or before it (entries are in code order).
    pub fn get_line_for_ip(&self, ip: usize) -> Option<u32> {
        self.line_at(ip).map(|(line, _)| line)
    }

    /// [`Self::get_line_for_ip`], and whether a statement starts at `ip`
    /// (a breakpoint stops there, not where a call returns mid-line).
    pub fn line_at(&self, ip: usize) -> Option<(u32, bool)> {
        let after = self.line_info.partition_point(|&(off, _)| off as usize <= ip);
        after.checked_sub(1).map(|i| (self.line_info[i].1, self.line_info[i].0 as usize == ip))
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
    /// The program's `$RESOURCE`s in order: name and bytes
    /// (`rapidr_value::resources`, registered by the host at startup).
    pub resources: Vec<(String, Vec<u8>)>,
    /// Console or windowed ($APPTYPE): which launcher runs it.
    pub app_type: AppType,
}

/// What kind of program a module is: `$APPTYPE CONSOLE | GUI | CGI` (and
/// RapidR's WEB), or, without one, GUI when it creates components, else
/// CONSOLE — RapidQ "detects what kind of application your program is just
/// by looking at the source code". The runtime's launchers read it: a
/// console program opened from the desktop gets a console.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppType {
    /// A format-2 file (it doesn't say).
    #[default]
    Unknown = 0,
    Console = 1,
    Gui = 2,
    Cgi = 3,
    Web = 4,
}

impl AppType {
    /// `$APPTYPE <word>`'s word.
    pub fn from_directive(word: &str) -> Option<AppType> {
        match word.trim().to_ascii_uppercase().as_str() {
            "CONSOLE" => Some(AppType::Console),
            "GUI" => Some(AppType::Gui),
            "CGI" => Some(AppType::Cgi),
            "WEB" => Some(AppType::Web),
            _ => None,
        }
    }

    pub fn from_u8(b: u8) -> AppType {
        match b {
            1 => AppType::Console,
            2 => AppType::Gui,
            3 => AppType::Cgi,
            4 => AppType::Web,
            _ => AppType::Unknown,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AppType::Unknown => "unknown",
            AppType::Console => "console",
            AppType::Gui => "gui",
            AppType::Cgi => "cgi",
            AppType::Web => "web",
        }
    }

    /// It reads and writes a console (CONSOLE, CGI).
    pub fn wants_console(self) -> bool {
        matches!(self, AppType::Console | AppType::Cgi)
    }
}

/// The start of a `.rrbc`, read without the rest ([`Header::read`]): enough
/// for a launcher to pick a console, and for an older runtime to say which
/// version a newer program needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub format: u16,
    pub min_runtime: [u16; 3],
    pub app_type: AppType,
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

    /// The index of `file` in [`Self::files`]: its name (any case, a path
    /// is reduced to its name, as the map keeps only names).
    pub fn file_index(&self, file: &str) -> Option<u32> {
        let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
        self.files.iter().position(|f| f.eq_ignore_ascii_case(name)).map(|i| i as u32)
    }

    /// The compiled lines that came from line `line` of `file` (a file
    /// included twice has two), in order — what a breakpoint in that file
    /// stops at.
    pub fn compiled_lines(&self, file: &str, line: u32) -> Vec<u32> {
        let Some(idx) = self.file_index(file) else { return Vec::new() };
        self.runs
            .iter()
            .filter(|r| r.1 == idx && (r.2..r.2.saturating_add(r.3)).contains(&line))
            .map(|r| r.0 + (line - r.2))
            .collect()
    }
}

impl Module {
    pub fn new() -> Self {
        Self::default()
    }

    /// The program's `$APPTYPE`, when it has one, over what the compiler
    /// made of it.
    pub fn apply_app_type_directive(&mut self, directive: Option<&str>) {
        if let Some(t) = directive.and_then(AppType::from_directive) {
            self.app_type = t;
        }
    }

    /// Every compiled line where a statement starts — the lines the
    /// debugger can stop at — sorted, once each.
    pub fn code_lines(&self) -> Vec<u32> {
        let mut lines: Vec<u32> = self.functions.iter().flat_map(|f| f.line_info.iter().map(|&(_, l)| l)).collect();
        lines.sort_unstable();
        lines.dedup();
        lines
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
/// The bytecode format written. A runtime reads every format from
/// [`OLDEST_VERSION`] to its own; a newer file is refused with the runtime
/// version it needs (its header's layout never changes from format 3 on).
pub const VERSION: u16 = 3;
/// Format 2: no header (no minimum runtime, no app type).
pub const OLDEST_VERSION: u16 = 2;
/// This runtime's version: RapidR's.
pub const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The oldest RapidR Runtime that runs what this compiler writes, recorded
/// in every module. Raise it to the release's version whenever the compiler
/// starts writing something older runtimes don't know (an opcode, a
/// builtin, a header field); format 3 came with 2.117.
pub const MIN_RUNTIME: [u16; 3] = [2, 117, 0];
/// Where a newer runtime is.
pub const RELEASES_URL: &str = "https://github.com/iBobX/RapidR/releases";

/// `major.minor.patch` → numbers (missing parts are 0).
pub fn parse_version(v: &str) -> [u16; 3] {
    let mut out = [0u16; 3];
    for (slot, part) in out.iter_mut().zip(v.split(['.', '-', '+'])) {
        *slot = part.parse().unwrap_or(0);
    }
    out
}
