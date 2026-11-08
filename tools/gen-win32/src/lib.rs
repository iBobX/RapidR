//! RapidR's generator of `RWin32.inc`: one standard set of Windows API
//! declarations (`DECLARE … LIB … ALIAS`, `CONST`, `TYPE`) for RapidR
//! programs.
//!
//! **Source, and only this source** (docs/legal/win32-declarations-review.md):
//! the Win32 API metadata Microsoft publishes in its windows-rs project, as
//! the `windows-sys` crate carries it — version [`WINDOWS_SYS_VERSION`],
//! built from windows-rs commit [`WINDOWS_SYS_COMMIT`], licensed MIT OR
//! Apache-2.0 and used here under MIT. The crate is the one RapidR already
//! depends on (Cargo.lock), read from cargo's own cache: no network. Nothing
//! else is read — no SDK header, no other include file. The few names the
//! metadata has no entry for (Visual Basic's `POINTAPI` …) are a table of
//! RapidR's own here ([`CLASSIC_TYPE_ALIASES`]), checked against Microsoft's
//! documentation.
//!
//! **What it writes**, in RapidR's syntax and RapidR's own layout:
//!
//! - every integer and text constant of the API areas RapidR covers
//!   ([`DLLS`]: the DLLs' namespaces), `CONST NAME = value`, the value as the
//!   declared type's bits (`&H80000000` for an unsigned 32-bit flag, as
//!   RapidR's 32-bit LONG holds it);
//! - every structure those areas use, `TYPE … END TYPE`, laid out as
//!   Windows' compiler lays it out — RapidR's TYPEs are packed, so the gaps
//!   are fields of their own (`rrPad1`), pointers are 8 bytes on 64-bit
//!   Windows and 4 on 32-bit (`$IFDEF WIN64 … $ELSE … $ENDIF` when the two
//!   differ), an anonymous union becomes its largest member;
//! - every function of [`DLLS`], `DECLARE FUNCTION Name LIB "dll" ALIAS
//!   "Name" (…) AS …`, with the modern A and W names;
//! - the classic names an older program uses: the unsuffixed name of every
//!   A/W pair (`MessageBox` for `MessageBoxA`, the ANSI one, and `LOGFONT`
//!   for `LOGFONTA`), as the SDK's own ANSI build names them.
//!
//! The same input gives the same bytes ([`generate`] is a pure function of
//! the input files; a test checks it).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The `windows-sys` version read (the one in RapidR's Cargo.lock).
pub const WINDOWS_SYS_VERSION: &str = "0.61.2";
/// The windows-rs commit that version was published from (the crate's
/// `.cargo_vcs_info.json`).
pub const WINDOWS_SYS_COMMIT: &str = "32c3144490c016fe496a0aed769bce60987a2e9d";
/// crates.io's SHA-256 of `windows-sys-0.61.2.crate` (Cargo.lock's checksum).
pub const WINDOWS_SYS_CRATE_SHA256: &str = "ae137229bcbd6cdf0f7b80a31df61766145077ddf49416a728b02cb3921ff3fc";

/// The DLLs whose functions RWin32.inc declares — the ones RapidQ-era
/// programs call (kernel32 … netapi32: the classic set) and the ones
/// RapidR's DLL corpus and today's programs reach for — and the short name
/// a `LIB` says. Their namespaces' constants and structures come with them.
pub const DLLS: &[(&str, &str)] = &[
    ("advapi32.dll", "advapi32"),
    ("avicap32.dll", "avicap32"),
    ("avifil32.dll", "avifil32"),
    ("comctl32.dll", "comctl32"),
    ("comdlg32.dll", "comdlg32"),
    ("dwmapi.dll", "dwmapi"),
    ("gdi32.dll", "gdi32"),
    ("glu32.dll", "glu32"),
    ("imm32.dll", "imm32"),
    ("kernel32.dll", "kernel32"),
    ("lz32.dll", "lz32"),
    ("mpr.dll", "mpr"),
    ("msacm32.dll", "msacm32"),
    ("msimg32.dll", "msimg32"),
    ("msvfw32.dll", "msvfw32"),
    ("netapi32.dll", "netapi32"),
    ("ole32.dll", "ole32"),
    ("oleaut32.dll", "oleaut32"),
    ("opengl32.dll", "opengl32"),
    ("psapi.dll", "psapi"),
    ("shell32.dll", "shell32"),
    ("shlwapi.dll", "shlwapi"),
    ("urlmon.dll", "urlmon"),
    ("user32.dll", "user32"),
    ("uxtheme.dll", "uxtheme"),
    ("version.dll", "version"),
    ("wininet.dll", "wininet"),
    ("winmm.dll", "winmm"),
    ("winspool.drv", "winspool.drv"),
    ("ws2_32.dll", "ws2_32"),
    ("wsock32.dll", "wsock32"),
];

/// Namespaces that hold constants only (no functions of their own) and
/// which the covered DLLs' calls take: their constants come too.
pub const CONSTANT_NAMESPACES: &[&str] = &["Foundation", "System/SystemServices"];

/// Namespaces a covered DLL has a few functions in, whose many constants
/// belong to other APIs (Windows Search, cryptography, Active Directory,
/// HID usages, UI Automation …): their constants and structures stay out,
/// but for the structures a declared function takes.
pub const NAMESPACES_LEFT_OUT: &[&str] = &[
    "Devices/HumanInterfaceDevice",
    "NetworkManagement/NetManagement",
    "Networking/ActiveDirectory",
    "Security/AppLocker",
    "Security/Authentication/Identity",
    "Storage/DistributedFileSystem",
    "Storage/Packaging/Appx",
    "System/ApplicationInstallationAndServicing",
    "System/ComponentServices",
    "System/Diagnostics/Etw",
    "UI/Accessibility",
];

/// Classic structure names the metadata has no entry of its own for:
/// (the classic name, the metadata structure it is the same as, why).
/// Checked against Microsoft's documentation of each structure.
pub const CLASSIC_TYPE_ALIASES: &[(&str, &str, &str)] = &[
    // Visual Basic called POINT `POINTAPI` (Point was a word of its own there)
    ("POINTAPI", "POINT", "Visual Basic's name for POINT"),
    // SIZEL is the SDK's second name for SIZE (wingdi.h: typedef SIZE SIZEL)
    ("SIZEL", "SIZE", "the SDK's other name for SIZE"),
];

/// Structures whose name is a statement of RapidR's (`INPUT`): the TYPE is
/// the C tag name Microsoft's documentation gives (`typedef struct tagINPUT
/// { … } INPUT`).
pub const TYPE_RENAMES: &[(&str, &str)] = &[("INPUT", "tagINPUT")];

/// Statement words a TYPE can't be named (the metadata's names that are
/// such words: [`TYPE_RENAMES`], else `NAME_T`).
const STATEMENT_WORDS: &[&str] = &["input", "print", "line", "open", "close", "data", "read", "name", "key", "get", "put", "field", "seek", "lock", "color", "screen", "width", "date", "time", "timer", "beep", "sleep", "kill", "files", "error", "on", "swap", "erase", "restore", "stop", "chain", "shell", "run", "locate", "cls"];

/// The mark after a DECLARE of a name RapidR has a built-in of
/// (`Sleep`, `Beep`): a classic program gets the DLL's, as its old include
/// gave it; a RapidR program (`.rr`) keeps RapidR's own.
pub const BUILTIN_MARK: &str = " ' (a RapidR built-in too)";

/// Whether RapidR has a built-in of this name (a function or statement the
/// compilers know, or a name RC.EXE keeps for itself): a CONST or a TYPE
/// can't take it.
pub fn is_rapidr_builtin(n: &str) -> bool {
    rapidr_bytecode::builtins::is_builtin(n) || rapidr_ast::RAPIDQ_RESERVED_NAMES.iter().any(|r| r.eq_ignore_ascii_case(n))
}

/// The TYPE name RapidR writes for structure `n`.
pub fn rr_type_name(n: &str) -> String {
    if let Some((_, r)) = TYPE_RENAMES.iter().find(|(m, _)| m.eq_ignore_ascii_case(n)) {
        return r.to_string();
    }
    if STATEMENT_WORDS.iter().any(|w| w.eq_ignore_ascii_case(n)) || is_reserved(n) {
        return format!("{n}_T");
    }
    n.to_string()
}

// ───────────────────────────── the input ─────────────────────────────

/// The input: the crate's sources, read in a fixed order.
pub struct Source {
    pub root: PathBuf,
    /// (path inside the crate with `/`, its text), sorted by path.
    pub files: Vec<(String, String)>,
    /// SHA-256 of every file read (its path, a NUL, its length as 8 bytes
    /// little-endian, its bytes, in path order).
    pub sha256: String,
}

/// Where cargo keeps `windows-sys-0.61.2`'s sources on this machine (its
/// registry cache), if it does.
pub fn default_source_dir() -> Option<PathBuf> {
    let cargo_home = std::env::var_os("CARGO_HOME").map(PathBuf::from).or_else(|| {
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(|h| PathBuf::from(h).join(".cargo"))
    })?;
    let mut found: Vec<PathBuf> = std::fs::read_dir(cargo_home.join("registry").join("src"))
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path().join(format!("windows-sys-{WINDOWS_SYS_VERSION}")))
        .filter(|p| p.is_dir())
        .collect();
    found.sort();
    found.into_iter().next()
}

fn collect(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, root, out)?;
        } else if p.extension().is_some_and(|e| e == "rs") {
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            out.push((rel, text));
        }
    }
    Ok(())
}

/// Reads the crate at `root`, checking it is the pinned version, from the
/// pinned commit, under the MIT licence.
pub fn read_source(root: &Path) -> Result<Source, String> {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).map_err(|e| format!("{}: {e}", root.display()))?;
    if !manifest.lines().any(|l| l.trim() == format!("version = \"{WINDOWS_SYS_VERSION}\"")) {
        return Err(format!("{} is not windows-sys {WINDOWS_SYS_VERSION}", root.display()));
    }
    if !manifest.lines().any(|l| l.trim() == "license = \"MIT OR Apache-2.0\"") {
        return Err(format!("{}: the licence is no longer MIT OR Apache-2.0: stop, the lawyer checks it again", root.display()));
    }
    let vcs = std::fs::read_to_string(root.join(".cargo_vcs_info.json")).unwrap_or_default();
    if !vcs.contains(WINDOWS_SYS_COMMIT) {
        return Err(format!("{}: not built from windows-rs commit {WINDOWS_SYS_COMMIT}", root.display()));
    }
    let mut files = Vec::new();
    collect(&root.join("src"), root, &mut files)?;
    let licence = std::fs::read_to_string(root.join("license-mit")).map_err(|e| format!("license-mit: {e}"))?;
    if !licence.contains("Copyright (c) Microsoft Corporation.") || !licence.contains("Permission is hereby granted, free of charge") {
        return Err("license-mit is not Microsoft's MIT licence".into());
    }
    files.push(("license-mit".into(), licence));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut h = Sha256::new();
    for (path, text) in &files {
        h.update(path.as_bytes());
        h.update([0u8]);
        h.update((text.len() as u64).to_le_bytes());
        h.update(text.as_bytes());
    }
    let sha256 = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok(Source { root: root.to_path_buf(), files, sha256 })
}

// ───────────────────────────── the model ─────────────────────────────

/// The architectures an item is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Arch {
    Any,
    /// 64-bit Windows (x64, ARM64)
    Bit64,
    /// 32-bit Windows (x86)
    Bit32,
    /// One 64-bit kind only (x64 or ARM64): left out.
    Other,
}

impl Arch {
    fn on(self, bit64: bool) -> bool {
        match self {
            Arch::Any => true,
            Arch::Bit64 => bit64,
            Arch::Bit32 => !bit64,
            Arch::Other => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prim {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    U128,
    I128,
    F32,
    F64,
    Usize,
    Isize,
    Bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Void,
    Prim(Prim),
    /// a pointer (`true`: `*const`)
    Ptr(bool, Box<Ty>),
    Array(Box<Ty>, usize),
    /// a function pointer
    Fn,
    /// a name: an alias, a structure (the path's last part)
    Named(String),
}

#[derive(Clone, Debug)]
pub struct Struct {
    pub name: String,
    pub ns: String,
    pub arch: Arch,
    pub union: bool,
    pub packed: Option<usize>,
    pub fields: Vec<(String, Ty)>,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: String,
    pub ns: String,
    pub arch: Arch,
    pub dll: String,
    pub abi: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Option<Ty>,
    pub varargs: bool,
}

#[derive(Clone, Debug)]
pub enum CValue {
    Int(i128),
    Str(String),
    Skip(&'static str),
}

#[derive(Clone, Debug)]
pub struct Const {
    pub name: String,
    pub ns: String,
    pub arch: Arch,
    pub ty: Ty,
    pub value: CValue,
}

/// Everything read from the metadata.
#[derive(Default)]
pub struct Metadata {
    pub structs: Vec<Struct>,
    pub funcs: Vec<Func>,
    pub consts: Vec<Const>,
    /// type aliases: name → (arch, type)
    pub aliases: HashMap<String, Vec<(Arch, Ty)>>,
    /// `pub use A as B` (32-bit Windows' GetWindowLongPtrA is GetWindowLongA): (B, A, namespace, arch)
    pub reexports: Vec<(String, String, String, Arch)>,
}

// ───────────────────────────── parsing ─────────────────────────────

fn arch_of(cfg: &str) -> Arch {
    let mut arches = BTreeSet::new();
    let mut rest = cfg;
    while let Some(i) = rest.find("target_arch = \"") {
        let after = &rest[i + 15..];
        let end = after.find('"').unwrap_or(after.len());
        arches.insert(after[..end].to_string());
        rest = &after[end..];
    }
    if cfg.contains("target_pointer_width = \"32\"") {
        return Arch::Bit32;
    }
    if cfg.contains("target_pointer_width = \"64\"") {
        return Arch::Bit64;
    }
    let a: Vec<&str> = arches.iter().map(String::as_str).collect();
    match a.as_slice() {
        [] => Arch::Any,
        ["aarch64", "arm64ec", "x86_64"] => Arch::Bit64,
        ["x86"] => Arch::Bit32,
        _ => Arch::Other,
    }
}

/// Splits `s` at top-level commas (not inside `<>`, `()`, `[]`).
fn split_top(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if !s[start..].trim().is_empty() {
        out.push(&s[start..]);
    }
    out
}

pub fn parse_ty(s: &str) -> Ty {
    let s = s.trim();
    if s.starts_with("* mut ") || s.starts_with("* const ") {
        return parse_ty(&s.replacen("* ", "*", 1));
    }
    if let Some(r) = s.strip_prefix("*mut ") {
        return Ty::Ptr(false, Box::new(parse_ty(r)));
    }
    if let Some(r) = s.strip_prefix("*const ") {
        return Ty::Ptr(true, Box::new(parse_ty(r)));
    }
    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len() - 1];
        if let Some(semi) = inner.rfind(';') {
            let n: usize = inner[semi + 1..].trim().replace('_', "").parse().unwrap_or(0);
            return Ty::Array(Box::new(parse_ty(&inner[..semi])), n);
        }
    }
    if s.starts_with("Option<") || s.starts_with("unsafe extern") || s.contains(" fn(") {
        return Ty::Fn;
    }
    let last = s.rsplit("::").next().unwrap_or(s).trim();
    match last {
        "u8" => Ty::Prim(Prim::U8),
        "i8" => Ty::Prim(Prim::I8),
        "u16" => Ty::Prim(Prim::U16),
        "i16" => Ty::Prim(Prim::I16),
        "u32" => Ty::Prim(Prim::U32),
        "i32" => Ty::Prim(Prim::I32),
        "u64" => Ty::Prim(Prim::U64),
        "i64" => Ty::Prim(Prim::I64),
        "u128" => Ty::Prim(Prim::U128),
        "i128" => Ty::Prim(Prim::I128),
        "f32" => Ty::Prim(Prim::F32),
        "f64" => Ty::Prim(Prim::F64),
        "usize" => Ty::Prim(Prim::Usize),
        "isize" => Ty::Prim(Prim::Isize),
        "bool" => Ty::Prim(Prim::Bool),
        "c_void" => Ty::Void,
        other => Ty::Named(other.to_string()),
    }
}

/// An integer literal of the metadata: `-1i32`, `0x8000_u32`, `260u32 as _`.
fn parse_int(v: &str) -> Option<i128> {
    let v = v.trim().trim_end_matches(" as _").trim();
    let (neg, v) = match v.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, v),
    };
    let (digits, radix) = match v.strip_prefix("0x") {
        Some(h) => (h, 16),
        None => (v, 10),
    };
    let suffixes = ["usize", "isize", "u128", "i128", "u64", "i64", "u32", "i32", "u16", "i16", "u8", "i8"];
    let mut d = digits;
    for s in suffixes {
        if let Some(x) = d.strip_suffix(s) {
            d = x;
            break;
        }
    }
    let d = d.replace('_', "");
    if d.is_empty() || !d.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let n = i128::from_str_radix(&d, radix).ok()?;
    Some(if neg { -n } else { n })
}

/// The text of `s!("…")` / `w!("…")`, unescaped (`None`: an escape RapidR
/// strings can't hold).
fn parse_str_lit(v: &str) -> Option<String> {
    let start = v.find("!(\"")? + 3;
    let end = v.rfind("\")")?;
    let raw = &v[start..end];
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                '\\' => out.push('\\'),
                _ => return None,
            }
        } else if c == '"' || (c as u32) < 0x20 || (c as u32) > 0x7e {
            return None;
        } else {
            out.push(c);
        }
    }
    Some(out)
}

fn parse_const(line: &str, ns: &str, arch: Arch) -> Option<Const> {
    let rest = line.strip_prefix("pub const ")?.strip_suffix(';')?;
    let colon = rest.find(':')?;
    let name = rest[..colon].trim().to_string();
    let after = &rest[colon + 1..];
    let eq = after.find(" = ")?;
    let ty = parse_ty(&after[..eq]);
    let v = after[eq + 3..].trim();
    let value = if v.contains("s!(\"") || v.contains("w!(\"") {
        match parse_str_lit(v) {
            Some(s) => CValue::Str(s),
            None => CValue::Skip("text RapidR strings can't hold"),
        }
    } else if v.contains("GUID::from_u128") {
        CValue::Skip("a GUID")
    } else if let Some(n) = parse_int(v) {
        CValue::Int(n)
    } else if v.contains('{') {
        CValue::Skip("a structure value")
    } else if v.contains('.') {
        CValue::Skip("a floating-point value")
    } else {
        CValue::Skip("another kind of value")
    };
    Some(Const { name, ns: ns.to_string(), arch, ty, value })
}

fn parse_link(line: &str, ns: &str, arch: Arch) -> Option<Func> {
    let rest = line.strip_prefix("windows_link::link!(\"")?;
    let q = rest.find('"')?;
    let dll = rest[..q].to_ascii_lowercase();
    let rest = rest[q + 1..].trim_start().strip_prefix('"')?;
    let q = rest.find('"')?;
    let abi = rest[..q].to_string();
    let rest = rest[q + 1..].trim_start().strip_prefix("fn ")?;
    let open = rest.find('(')?;
    let name = rest[..open].trim().to_string();
    // the parameters: up to the matching ')'
    let body = &rest[open + 1..];
    let mut depth = 1i32;
    let mut close = None;
    for (i, c) in body.char_indices() {
        match c {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let mut params = Vec::new();
    let mut varargs = false;
    for p in split_top(&body[..close]) {
        let p = p.trim();
        if p == "..." {
            varargs = true;
            continue;
        }
        let (n, t) = p.split_once(':')?;
        params.push((n.trim().trim_start_matches("r#").to_string(), parse_ty(t)));
    }
    let tail = body[close + 1..].trim();
    let ret = tail.strip_prefix("->").map(|r| parse_ty(r.trim().trim_end_matches(");").trim_end_matches(')')));
    Some(Func { name, ns: ns.to_string(), arch, dll, abi, params, ret, varargs })
}

/// Reads every namespace of the crate.
pub fn parse(src: &Source) -> Metadata {
    let mut md = Metadata::default();
    for (path, text) in &src.files {
        let ns = if path == "src/core/mod.rs" {
            "core".to_string()
        } else if let Some(r) = path.strip_prefix("src/Windows/Win32/") {
            r.trim_end_matches("mod.rs").trim_end_matches('/').to_string()
        } else {
            continue;
        };
        parse_file(&ns, text, &mut md);
    }
    md
}

fn parse_file(ns: &str, text: &str, md: &mut Metadata) {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    let mut arch = Arch::Any;
    let mut packed: Option<usize> = None;
    while i < lines.len() {
        let t = lines[i].trim_end();
        i += 1;
        if t.starts_with("#[cfg(") {
            // (an item may have two: the architecture's, then a feature's)
            let a = arch_of(t);
            if a != Arch::Any {
                arch = a;
            }
            continue;
        }
        if let Some(r) = t.strip_prefix("#[repr(") {
            packed = r.find("packed(").and_then(|p| r[p + 7..].split(')').next()).and_then(|n| n.parse().ok());
            continue;
        }
        if t.starts_with("#[") || t.is_empty() || t.starts_with("//") {
            continue;
        }
        if t.starts_with("impl ") {
            if !t.ends_with('}') {
                while i < lines.len() && lines[i] != "}" {
                    i += 1;
                }
                i += 1;
            }
        } else if t.starts_with("windows_link::link!(") {
            if let Some(f) = parse_link(t, ns, arch) {
                md.funcs.push(f);
            }
        } else if t.starts_with("pub const ") {
            let mut whole = t.to_string();
            while !whole.ends_with(';') && i < lines.len() {
                whole.push(' ');
                whole.push_str(lines[i].trim());
                i += 1;
            }
            if let Some(c) = parse_const(&whole, ns, arch) {
                md.consts.push(c);
            }
        } else if let Some(r) = t.strip_prefix("pub type ") {
            let mut whole = r.to_string();
            while !whole.ends_with(';') && i < lines.len() {
                whole.push(' ');
                whole.push_str(lines[i].trim());
                i += 1;
            }
            if let Some((n, ty)) = whole.trim_end_matches(';').split_once(" = ") {
                md.aliases.entry(n.trim().to_string()).or_default().push((arch, parse_ty(ty)));
            }
        } else if t.starts_with("pub struct ") || t.starts_with("pub union ") {
            let union = t.starts_with("pub union ");
            let head = t.trim_start_matches("pub struct ").trim_start_matches("pub union ");
            if let Some(name) = head.strip_suffix(" {") {
                let mut fields = Vec::new();
                while i < lines.len() && lines[i] != "}" {
                    let f = lines[i].trim().trim_end_matches(',');
                    if let Some(f) = f.strip_prefix("pub ") {
                        if let Some((n, ty)) = f.split_once(':') {
                            fields.push((n.trim().trim_start_matches("r#").to_string(), parse_ty(ty)));
                        }
                    }
                    i += 1;
                }
                i += 1;
                md.structs.push(Struct { name: name.trim().to_string(), ns: ns.to_string(), arch, union, packed, fields });
            } else if let Some(open) = head.find("(pub ") {
                // `pub struct HandleType(pub i32);`: the inner type
                let name = head[..open].trim().to_string();
                let inner = head[open + 5..].trim_end_matches(';').trim_end_matches(')');
                md.aliases.entry(name).or_default().push((arch, parse_ty(inner)));
            }
        } else if let Some(r) = t.strip_prefix("pub use ") {
            if let Some((a, b)) = r.trim_end_matches(';').split_once(" as ") {
                if !a.contains("::") {
                    md.reexports.push((b.trim().to_string(), a.trim().to_string(), ns.to_string(), arch));
                }
            }
        }
        arch = Arch::Any;
        packed = None;
    }
}

// ───────────────────────────── layout ─────────────────────────────

/// How RapidR passes and stores a value of a type: the type resolved
/// through the metadata's aliases.
struct World<'a> {
    md: &'a Metadata,
    /// structure name → its definitions (one per arch kind)
    structs: HashMap<&'a str, Vec<&'a Struct>>,
}

/// One field of a TYPE as RapidR declares it.
#[derive(Clone, Debug, PartialEq)]
struct Leaf {
    name: String,
    offset: usize,
    size: usize,
    /// RapidR's declaration after the name: ` AS LONG`, `(15) AS WORD`, ` AS STRING * 32`
    decl: String,
    /// a TYPE this field is (for the order the TYPEs are written in)
    dep: Option<String>,
}

impl<'a> World<'a> {
    fn new(md: &'a Metadata) -> Self {
        let mut structs: HashMap<&str, Vec<&Struct>> = HashMap::new();
        for s in &md.structs {
            structs.entry(s.name.as_str()).or_default().push(s);
        }
        World { md, structs }
    }

    fn struct_for(&self, name: &str, bit64: bool) -> Option<&'a Struct> {
        self.structs.get(name)?.iter().copied().find(|s| s.arch.on(bit64))
    }

    /// `ty` with its aliases resolved (a name that is a structure stays a name).
    fn resolve(&self, ty: &Ty, bit64: bool) -> Ty {
        let mut t = ty.clone();
        for _ in 0..16 {
            match &t {
                Ty::Named(n) if !self.structs.contains_key(n.as_str()) => match self.md.aliases.get(n).and_then(|v| v.iter().find(|(a, _)| a.on(bit64))) {
                    Some((_, inner)) => t = inner.clone(),
                    None => return t,
                },
                _ => return t,
            }
        }
        t
    }

    fn size_align(&self, ty: &Ty, bit64: bool, depth: usize) -> Option<(usize, usize)> {
        if depth > 32 {
            return None;
        }
        let p = if bit64 { 8 } else { 4 };
        match self.resolve(ty, bit64) {
            Ty::Void => None,
            Ty::Prim(pr) => Some(match pr {
                Prim::U8 | Prim::I8 | Prim::Bool => (1, 1),
                Prim::U16 | Prim::I16 => (2, 2),
                Prim::U32 | Prim::I32 | Prim::F32 => (4, 4),
                Prim::U64 | Prim::I64 | Prim::F64 => (8, 8),
                Prim::U128 | Prim::I128 => return None,
                Prim::Usize | Prim::Isize => (p, p),
            }),
            Ty::Ptr(..) | Ty::Fn => Some((p, p)),
            Ty::Array(t, n) => {
                let (s, a) = self.size_align(&t, bit64, depth + 1)?;
                if n == 0 {
                    return None;
                }
                Some((s * n, a))
            }
            Ty::Named(n) => {
                let s = self.struct_for(&n, bit64)?;
                let l = self.layout(s, bit64, depth + 1)?;
                Some((l.0, l.1))
            }
        }
    }

    /// (size, alignment, each field's offset) of a structure as Windows'
    /// compiler lays it out.
    fn layout(&self, s: &Struct, bit64: bool, depth: usize) -> Option<(usize, usize, Vec<usize>)> {
        let cap = s.packed.unwrap_or(usize::MAX);
        let mut off = 0usize;
        let mut size = 0usize;
        let mut align = 1usize;
        let mut offsets = Vec::new();
        for (_, ty) in &s.fields {
            let (fs, fa) = self.size_align(ty, bit64, depth + 1)?;
            let fa = fa.min(cap);
            align = align.max(fa);
            if s.union {
                offsets.push(0);
                size = size.max(fs);
            } else {
                off = off.div_ceil(fa) * fa;
                offsets.push(off);
                off += fs;
                size = off;
            }
        }
        if s.fields.is_empty() {
            return None;
        }
        Some((size.div_ceil(align) * align, align, offsets))
    }

    /// RapidR's type of a pointer-sized value.
    fn ptr_rr(bit64: bool) -> &'static str {
        if bit64 {
            "INT64"
        } else {
            "LONG"
        }
    }

    /// A field's leaves: one, or a nested anonymous structure's / union's
    /// fields inline (`None`: a type RapidR's TYPEs can't hold).
    fn leaves(&self, name: &str, ty: &Ty, base: usize, bit64: bool, parent: &str, out: &mut Vec<Leaf>, depth: usize) -> Option<()> {
        if depth > 16 {
            return None;
        }
        let raw = ty.clone();
        let t = self.resolve(ty, bit64);
        let (size, _) = self.size_align(&t, bit64, 0)?;
        let mut push = |decl: String, size: usize, dep: Option<String>| out.push(Leaf { name: name.to_string(), offset: base, size, decl, dep });
        match &t {
            Ty::Void => return None,
            Ty::Prim(p) => {
                let rr = match p {
                    Prim::U8 | Prim::I8 | Prim::Bool => "BYTE",
                    Prim::U16 => "WORD",
                    Prim::I16 => "SHORT",
                    Prim::U32 | Prim::I32 => "LONG",
                    Prim::U64 => "UINT64",
                    Prim::I64 => "INT64",
                    Prim::F32 => "SINGLE",
                    Prim::F64 => "DOUBLE",
                    Prim::Usize | Prim::Isize => Self::ptr_rr(bit64),
                    Prim::U128 | Prim::I128 => return None,
                };
                push(format!(" AS {rr}"), size, None);
            }
            Ty::Ptr(_, to) if is_ansi_text(&raw, to) => {
                // a text pointer: a STRING (its characters' address, 4 bytes
                // in RapidR's TYPEs), and on 64-bit Windows the pointer's
                // high half, always 0 for RapidR's own addresses
                push(" AS STRING".into(), 4, None);
                if bit64 {
                    out.push(Leaf { name: format!("{name}Hi"), offset: base + 4, size: 4, decl: " AS LONG".into(), dep: None });
                }
            }
            Ty::Ptr(..) | Ty::Fn => push(format!(" AS {}", Self::ptr_rr(bit64)), size, None),
            Ty::Array(elem, n) => {
                let mut e = self.resolve(elem, bit64);
                let mut count = *n;
                while let Ty::Array(inner, m) = e.clone() {
                    count *= m;
                    e = self.resolve(&inner, bit64);
                }
                match &e {
                    Ty::Prim(Prim::U8) | Ty::Prim(Prim::I8) => push(format!(" AS STRING * {count}"), size, None),
                    Ty::Prim(p) => {
                        let rr = match p {
                            Prim::Bool => "BYTE",
                            Prim::U16 => "WORD",
                            Prim::I16 => "SHORT",
                            Prim::U32 | Prim::I32 => "LONG",
                            Prim::U64 => "UINT64",
                            Prim::I64 => "INT64",
                            Prim::F32 => "SINGLE",
                            Prim::F64 => "DOUBLE",
                            Prim::Usize | Prim::Isize => Self::ptr_rr(bit64),
                            _ => return None,
                        };
                        push(format!("({}) AS {rr}", count - 1), size, None);
                    }
                    Ty::Ptr(..) | Ty::Fn => push(format!("({}) AS {}", count - 1, Self::ptr_rr(bit64)), size, None),
                    Ty::Named(sn) => {
                        let s = self.struct_for(sn, bit64)?;
                        if is_anonymous(sn, parent) || s.union {
                            // (an array of anonymous parts: the bytes)
                            push(format!("({}) AS BYTE", size - 1), size, None);
                        } else {
                            push(format!("({}) AS {}", count - 1, rr_type_name(sn)), size, Some(sn.clone()));
                        }
                    }
                    _ => return None,
                }
            }
            Ty::Named(sn) => {
                let s = self.struct_for(sn, bit64)?;
                if s.union || is_anonymous(sn, parent) {
                    self.inline(s, base, bit64, out, depth + 1)?;
                } else {
                    push(format!(" AS {}", rr_type_name(sn)), size, Some(sn.clone()));
                }
            }
        }
        Some(())
    }

    /// A nested part's fields at `base`: a structure's in order, a union's
    /// largest member (the first of the largest).
    fn inline(&self, s: &Struct, base: usize, bit64: bool, out: &mut Vec<Leaf>, depth: usize) -> Option<()> {
        let (_, _, offsets) = self.layout(s, bit64, 0)?;
        if s.union {
            let mut best: Option<(usize, usize)> = None;
            for (i, (_, ty)) in s.fields.iter().enumerate() {
                let (fs, _) = self.size_align(ty, bit64, 0)?;
                if best.is_none_or(|(_, b)| fs > b) {
                    best = Some((i, fs));
                }
            }
            let (i, _) = best?;
            let (n, ty) = &s.fields[i];
            self.leaves(&field_name(n), ty, base, bit64, &s.name, out, depth)
        } else {
            for ((n, ty), off) in s.fields.iter().zip(offsets) {
                self.leaves(&field_name(n), ty, base + off, bit64, &s.name, out, depth)?;
            }
            Some(())
        }
    }

    /// A structure's TYPE body for one architecture: its lines (each
    /// field, the gaps as `rrPadN`) and the TYPEs it uses.
    fn type_body(&self, s: &Struct, bit64: bool) -> Option<(Vec<String>, BTreeSet<String>, usize)> {
        let (size, _, _) = self.layout(s, bit64, 0)?;
        let mut leaves = Vec::new();
        if s.union {
            self.inline(s, 0, bit64, &mut leaves, 0)?;
        } else {
            let (_, _, offsets) = self.layout(s, bit64, 0)?;
            for ((n, ty), off) in s.fields.iter().zip(offsets) {
                self.leaves(&field_name(n), ty, off, bit64, &s.name, &mut leaves, 0)?;
            }
        }
        let mut lines = Vec::new();
        let mut deps = BTreeSet::new();
        let mut at = 0usize;
        let mut pads = 0;
        let mut names: HashSet<String> = HashSet::new();
        let mut pad = |gap: usize, lines: &mut Vec<String>| {
            pads += 1;
            lines.push(match gap {
                1 => format!("    rrPad{pads} AS BYTE"),
                2 => format!("    rrPad{pads} AS WORD"),
                4 => format!("    rrPad{pads} AS LONG"),
                n => format!("    rrPad{pads}({}) AS BYTE", n - 1),
            });
        };
        for l in &leaves {
            if l.offset < at {
                return None;
            }
            if l.offset > at {
                pad(l.offset - at, &mut lines);
            }
            let mut name = l.name.clone();
            let mut k = 2;
            while !names.insert(name.to_ascii_lowercase()) {
                name = format!("{}{k}", l.name);
                k += 1;
            }
            lines.push(format!("    {name}{}", l.decl));
            if let Some(d) = &l.dep {
                deps.insert(d.clone());
            }
            at = l.offset + l.size;
        }
        if at > size {
            return None;
        }
        if size > at {
            pad(size - at, &mut lines);
        }
        Some((lines, deps, size))
    }
}

/// Whether a pointer is ANSI text (PSTR, PCSTR, a byte pointer): a STRING.
fn is_ansi_text(raw: &Ty, to: &Ty) -> bool {
    matches!(raw, Ty::Named(n) if n == "PSTR" || n == "PCSTR") || matches!(to, Ty::Prim(Prim::U8) | Ty::Prim(Prim::I8))
}

/// Whether a structure is a nested anonymous part of `parent` (the
/// metadata names them `PARENT_0`, `PARENT_0_1` …).
fn is_anonymous(name: &str, parent: &str) -> bool {
    let rest = match name.strip_prefix(parent) {
        Some(r) => r,
        None => {
            // (a part of a part: NAME_0_1 inside NAME_0, or of the same root)
            let root = parent.split('_').next().unwrap_or(parent);
            match name.strip_prefix(root) {
                Some(r) if !r.is_empty() => return r.trim_start_matches('_').split('_').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())) && name.contains('_'),
                _ => return false,
            }
        }
    };
    let rest = rest.trim_start_matches('_');
    !rest.is_empty() && rest.split('_').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// A metadata field or parameter name as a RapidR name.
fn field_name(n: &str) -> String {
    let mut s: String = n.trim_start_matches('_').chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    if s.is_empty() {
        s = "bits".into();
    }
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        s.insert(0, 'f');
    }
    if is_reserved(&s) {
        s.push('_');
    }
    s
}

/// Words RapidR's grammar keeps for itself, which a parameter or a field
/// can't be named (the metadata's names that are such words get a `_`).
const RESERVED: &[&str] = &[
    "and", "as", "byref", "byval", "call", "case", "const", "create", "declare", "dim", "do", "else", "elseif", "end", "event", "exit", "extends", "for", "function",
    "gosub", "goto", "if", "is", "let", "lib", "loop", "mod", "next", "not", "or", "redim", "rem", "return", "select", "shared", "shl", "shr", "static", "step",
    "sub", "then", "to", "type", "until", "wend", "while", "with", "xor", "alias", "string", "integer", "long", "short", "byte", "word", "dword", "single", "double",
    "variant", "int64", "uint64", "uint32", "imp", "eqv", "subi", "functioni", "constructor", "struct", "true", "false",
];

fn is_reserved(s: &str) -> bool {
    RESERVED.iter().any(|r| r.eq_ignore_ascii_case(s))
}

// ───────────────────────────── selection and output ─────────────────────────────

/// What a generation gives: the file and its counts.
pub struct Output {
    pub text: String,
    pub counts: Counts,
}

#[derive(Default, Debug, Clone)]
pub struct Counts {
    pub consts: usize,
    pub string_consts: usize,
    pub types: usize,
    pub types_arch_specific: usize,
    pub functions: usize,
    pub classic_functions: usize,
    pub classic_types: usize,
    pub skipped_consts: BTreeMap<String, usize>,
    pub skipped_functions: BTreeMap<String, usize>,
    /// the functions left out, by name (for `--verbose`)
    pub skipped_function_names: Vec<String>,
    pub skipped_types: usize,
    pub left_to_rapidq_inc: usize,
    pub name_clashes: usize,
}

/// A constant's value written for RapidR: the declared type's bits.
fn const_text(w: &World, c: &Const) -> Option<String> {
    let CValue::Int(v) = c.value else { return None };
    let (bits, signed) = match w.resolve(&c.ty, true) {
        Ty::Prim(p) => match p {
            Prim::U8 | Prim::Bool => (8, false),
            Prim::I8 => (8, true),
            Prim::U16 => (16, false),
            Prim::I16 => (16, true),
            Prim::U32 => (32, false),
            Prim::I32 => (32, true),
            Prim::U64 | Prim::Usize => (64, false),
            Prim::I64 | Prim::Isize => (64, true),
            _ => return None,
        },
        // (a handle or a resource name: the bits as a signed pointer)
        Ty::Ptr(..) | Ty::Fn => (64, true),
        _ => return None,
    };
    let mask: i128 = (1i128 << bits) - 1;
    let pattern = v & mask;
    let value = if signed && pattern >> (bits - 1) == 1 { pattern - (1i128 << bits) } else { pattern };
    Some(if bits <= 32 {
        if value == -(1i128 << 31) {
            "&H80000000".to_string()
        } else if value < 0 || value < 1i128 << 31 {
            value.to_string()
        } else {
            format!("&H{pattern:X}")
        }
    } else if value > -(1i128 << 31) && value < (1i128 << 31) {
        value.to_string()
    } else if value == -(1i128 << 31) {
        "&H80000000".to_string()
    } else if value < 0 {
        value.to_string()
    } else {
        format!("&H{value:X}")
    })
}

/// A parameter as RapidR declares it (`None`: one RapidR can't pass).
fn param_text(w: &World, name: &str, ty: &Ty, emitted: &HashSet<String>) -> Option<String> {
    let n = field_name(name);
    let t = w.resolve(ty, true);
    let t32 = w.resolve(ty, false);
    Some(match (&t, ty) {
        (_, Ty::Named(a)) if a == "PSTR" || a == "PCSTR" => format!("BYVAL {n} AS STRING"),
        (_, Ty::Named(a)) if a == "PWSTR" || a == "PCWSTR" || a == "BSTR" => format!("BYVAL {n} AS LONG"),
        (Ty::Prim(p), _) => match p {
            Prim::U8 | Prim::I8 | Prim::Bool => format!("BYVAL {n} AS BYTE"),
            Prim::U16 => format!("BYVAL {n} AS WORD"),
            Prim::I16 => format!("BYVAL {n} AS SHORT"),
            Prim::U32 | Prim::I32 => format!("BYVAL {n} AS LONG"),
            Prim::U64 => format!("BYVAL {n} AS UINT64"),
            Prim::I64 => format!("BYVAL {n} AS INT64"),
            Prim::F32 => format!("BYVAL {n} AS SINGLE"),
            Prim::F64 => format!("BYVAL {n} AS DOUBLE"),
            Prim::Usize | Prim::Isize => format!("BYVAL {n} AS LONG"),
            _ => return None,
        },
        (Ty::Fn, _) => format!("BYVAL {n} AS LONG"),
        (Ty::Ptr(_, to), _) => {
            let to = w.resolve(to, true);
            match &to {
                Ty::Prim(Prim::U8) | Ty::Prim(Prim::I8) => format!("BYVAL {n} AS STRING"),
                Ty::Prim(Prim::U16) => format!("BYREF {n} AS WORD"),
                Ty::Prim(Prim::I16) => format!("BYREF {n} AS SHORT"),
                Ty::Prim(Prim::U32) | Ty::Prim(Prim::I32) => format!("BYREF {n} AS LONG"),
                Ty::Prim(Prim::U64) => format!("BYREF {n} AS UINT64"),
                Ty::Prim(Prim::I64) => format!("BYREF {n} AS INT64"),
                Ty::Prim(Prim::F32) => format!("BYREF {n} AS SINGLE"),
                Ty::Prim(Prim::F64) => format!("BYREF {n} AS DOUBLE"),
                Ty::Prim(Prim::Usize) | Ty::Prim(Prim::Isize) | Ty::Ptr(..) | Ty::Fn => format!("BYREF {n} AS LONG"),
                Ty::Named(sn) if emitted.contains(sn) => format!("{n} AS {}", rr_type_name(sn)),
                _ => format!("BYVAL {n} AS LONG"),
            }
        }
        (Ty::Named(sn), _) => match by_value_struct(w, sn, &t32)? {
            // a structure by value: its bytes as one number when they fit
            // one (8 bytes: two stack words on 32-bit Windows, one register
            // on 64-bit; a structure of one pointer: the pointer)
            "INT64" => format!("BYVAL {n} AS INT64"),
            _ => format!("BYVAL {n} AS LONG"),
        },
        _ => return None,
    })
}

/// How a structure passed or returned by value goes through RapidR's DLL
/// calls: as an INT64 (8 bytes on both kinds of Windows), a LONG (4 bytes,
/// or one pointer), or not at all.
fn by_value_struct(w: &World, sn: &str, t32: &Ty) -> Option<&'static str> {
    let (s64, _) = w.size_align(&Ty::Named(sn.to_string()), true, 0)?;
    let (s32, _) = w.size_align(t32, false, 0)?;
    let one_pointer = w.struct_for(sn, true).is_some_and(|s| s.fields.len() == 1 && matches!(w.resolve(&s.fields[0].1, true), Ty::Ptr(..) | Ty::Fn | Ty::Prim(Prim::Usize) | Ty::Prim(Prim::Isize)));
    match (s64, s32) {
        _ if one_pointer => Some("LONG"),
        (8, 8) => Some("INT64"),
        (4, 4) => Some("LONG"),
        _ => None,
    }
}

/// A result as RapidR declares it (`Some(None)`: a SUB).
fn ret_text(w: &World, ty: &Option<Ty>) -> Option<Option<&'static str>> {
    let Some(ty) = ty else { return Some(None) };
    if matches!(ty, Ty::Named(n) if n == "!") {
        return Some(None);
    }
    if let Ty::Named(a) = ty {
        if a == "PSTR" || a == "PCSTR" {
            return Some(Some("STRING"));
        }
    }
    Some(Some(match w.resolve(ty, true) {
        Ty::Prim(p) => match p {
            Prim::U8 | Prim::I8 | Prim::Bool => "BYTE",
            Prim::U16 => "WORD",
            Prim::I16 => "SHORT",
            Prim::U32 | Prim::I32 => "LONG",
            Prim::U64 => "UINT64",
            Prim::I64 => "INT64",
            Prim::F32 => "SINGLE",
            Prim::F64 => "DOUBLE",
            Prim::Usize | Prim::Isize => "LONG",
            _ => return None,
        },
        Ty::Ptr(..) | Ty::Fn => "LONG",
        Ty::Void => return Some(None),
        Ty::Named(sn) => return by_value_struct(w, &sn, &w.resolve(ty, false)).map(Some),
        _ => return None,
    }))
}

/// One TYPE, as written: its lines for each architecture.
struct TypeOut {
    name: String,
    ns: String,
    /// (64-bit body, 32-bit body)
    b64: Option<Vec<String>>,
    b32: Option<Vec<String>>,
    deps: BTreeSet<String>,
}

fn write_type(out: &mut String, name: &str, b64: &Option<Vec<String>>, b32: &Option<Vec<String>>) -> bool {
    let block = |out: &mut String, lines: &[String]| {
        let _ = writeln!(out, "TYPE {name}");
        for l in lines {
            out.push_str(l);
            out.push('\n');
        }
        out.push_str("END TYPE\n");
    };
    match (b64, b32) {
        (Some(a), Some(b)) if a == b => {
            block(out, a);
            false
        }
        (Some(a), Some(b)) => {
            out.push_str("$IFDEF WIN64\n");
            block(out, a);
            out.push_str("$ELSE\n");
            block(out, b);
            out.push_str("$ENDIF\n");
            true
        }
        (Some(a), None) => {
            out.push_str("$IFDEF WIN64\n");
            block(out, a);
            out.push_str("$ENDIF\n");
            true
        }
        (None, Some(b)) => {
            out.push_str("$IFNDEF WIN64\n");
            block(out, b);
            out.push_str("$ENDIF\n");
            true
        }
        (None, None) => false,
    }
}

/// The header every generated file starts with: the legal notice
/// (docs/legal/win32-declarations-review.md, "Exact notice text"), then
/// what the file is and how a program uses it.
fn header(src_sha: &str, counts_line: &str) -> String {
    let mut h = String::new();
    let lines = [
        "' RWin32.inc: one standard set of Windows API declarations for RapidR.",
        "'",
        "' Windows API declarations. Generated by RapidR's own tool (tools/gen-win32)",
        "' from the Win32 API metadata published by Microsoft in the windows-rs project,",
        &format!("' https://github.com/microsoft/windows-rs (commit {WINDOWS_SYS_COMMIT}), which is licensed"),
        "' MIT OR Apache-2.0; RapidR uses it under the MIT License.",
        "'",
        "' Copyright (c) Microsoft Corporation.",
        "'",
        "' Permission is hereby granted, free of charge, to any person obtaining a copy",
        "' of this software and associated documentation files (the \"Software\"), to deal",
        "' in the Software without restriction, including without limitation the rights",
        "' to use, copy, modify, merge, publish, distribute, sublicense, and/or sell",
        "' copies of the Software, and to permit persons to whom the Software is",
        "' furnished to do so, subject to the following conditions:",
        "'",
        "' The above copyright notice and this permission notice shall be included in all",
        "' copies or substantial portions of the Software.",
        "'",
        "' THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR",
        "' IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,",
        "' FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE",
        "' AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER",
        "' LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,",
        "' OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE",
        "' SOFTWARE.",
        "'",
        "' Microsoft and Windows are trademarks of the Microsoft group of companies.",
        "' Named here only to describe compatibility; RapidR is not affiliated with or",
        "' endorsed by Microsoft.",
        "'",
        &format!("' Input: the windows-sys crate {WINDOWS_SYS_VERSION} (crates.io SHA-256 {WINDOWS_SYS_CRATE_SHA256});"),
        &format!("' the sources read: SHA-256 {src_sha}."),
        &format!("' {counts_line}"),
        "'",
        "' Use: $INCLUDE \"RWin32.inc\" (no path: it is built into RapidR). A program gets the",
        "' declarations it uses and doesn't declare itself; a name of its own always wins.",
        "' The classic names (MessageBox, LOGFONT) are the ANSI ones, as older programs had them;",
        "' the A and W names are there too. DLL calls run on Windows; elsewhere the program",
        "' compiles and a call says it needs Windows. A DECLARE marked (a RapidR built-in too)",
        "' names something RapidR has (Sleep, Beep): a classic program gets the DLL's, as its",
        "' old include gave it; a RapidR program (.rr) keeps RapidR's. Do not edit: run tools/gen-win32.",
        "'",
    ];
    for l in lines {
        h.push_str(l);
        h.push('\n');
    }
    h
}

/// Generates RWin32.inc from the input. `rapidq_inc` is RapidR's built-in
/// RAPIDQ.INC (name, value): those names are left to it.
pub fn generate(src: &Source, rapidq_inc: &[(&str, i64)]) -> Output {
    let md = parse(src);
    let w = World::new(&md);
    let mut counts = Counts::default();
    let dll_short: HashMap<&str, &str> = DLLS.iter().copied().collect();
    let rapidq: HashSet<String> = rapidq_inc.iter().map(|(n, _)| n.to_ascii_uppercase()).collect();

    // the functions
    let funcs: Vec<&Func> = md.funcs.iter().filter(|f| dll_short.contains_key(f.dll.as_str())).collect();
    let mut namespaces: BTreeSet<String> = funcs.iter().map(|f| f.ns.clone()).collect();
    namespaces.extend(CONSTANT_NAMESPACES.iter().map(|s| s.to_string()));
    for n in NAMESPACES_LEFT_OUT {
        namespaces.remove(*n);
    }

    // every name, once (first come: constants, TYPEs, functions)
    let mut taken: HashSet<String> = HashSet::new();
    // (a function's name wins over a constant's or a TYPE's of the same
    // letters: RapidR's names don't tell case apart, and StretchBlt,
    // mouse_event, EndDoc are what programs call — the printer escapes
    // STRETCHBLT, ENDDOC and the console's MOUSE_EVENT give way, as in the
    // classic file; and the classic unsuffixed names of A/W pairs too)
    let mut function_names: HashSet<String> = HashSet::new();
    for f in &funcs {
        function_names.insert(f.name.to_ascii_uppercase());
        if let Some(base) = f.name.strip_suffix('A') {
            if funcs.iter().any(|g| g.name == format!("{base}W")) {
                function_names.insert(base.to_ascii_uppercase());
            }
        }
    }

    // ── the constants
    let mut const_out = String::new();
    let mut by_ns: BTreeMap<&str, Vec<&Const>> = BTreeMap::new();
    for c in &md.consts {
        if namespaces.contains(&c.ns) {
            by_ns.entry(c.ns.as_str()).or_default().push(c);
        }
    }
    for (ns, list) in &by_ns {
        let mut lines = Vec::new();
        for c in list {
            if c.arch != Arch::Any {
                *counts.skipped_consts.entry("one architecture only".into()).or_default() += 1;
                continue;
            }
            let upper = c.name.to_ascii_uppercase();
            if rapidq.contains(&upper) {
                counts.left_to_rapidq_inc += 1;
                continue;
            }
            if is_reserved(&c.name) || is_rapidr_builtin(&c.name) {
                *counts.skipped_consts.entry("the name of a RapidR word or built-in".into()).or_default() += 1;
                continue;
            }
            let text = match &c.value {
                CValue::Str(s) => {
                    counts.string_consts += 1;
                    format!("\"{s}\"")
                }
                CValue::Skip(why) => {
                    *counts.skipped_consts.entry(why.to_string()).or_default() += 1;
                    continue;
                }
                CValue::Int(_) => match const_text(&w, c) {
                    Some(t) => t,
                    None => {
                        *counts.skipped_consts.entry("a value of another type".into()).or_default() += 1;
                        continue;
                    }
                },
            };
            if function_names.contains(&upper) {
                *counts.skipped_consts.entry("the name of a function too".into()).or_default() += 1;
                continue;
            }
            if !taken.insert(upper) {
                counts.name_clashes += 1;
                continue;
            }
            lines.push(format!("CONST {} = {text}", c.name));
            counts.consts += 1;
        }
        if !lines.is_empty() {
            let _ = writeln!(const_out, "' --- {} ---", ns.replace('/', "."));
            for l in lines {
                const_out.push_str(&l);
                const_out.push('\n');
            }
        }
    }

    // ── the TYPEs: every structure of the namespaces and every one a
    // function or another TYPE uses, in an order where each comes after
    // the TYPEs its fields are
    let mut wanted: BTreeSet<String> = BTreeSet::new();
    for s in &md.structs {
        if namespaces.contains(&s.ns) && !looks_anonymous(&s.name, &w) {
            wanted.insert(s.name.clone());
        }
    }
    for f in &funcs {
        for (_, t) in &f.params {
            if let Ty::Ptr(_, to) = t {
                if let Ty::Named(n) = w.resolve(to, true) {
                    if w.structs.contains_key(n.as_str()) {
                        wanted.insert(n);
                    }
                }
            }
        }
    }
    let mut types: BTreeMap<String, TypeOut> = BTreeMap::new();
    let mut queue: Vec<String> = wanted.iter().cloned().collect();
    let mut seen: HashSet<String> = HashSet::new();
    while let Some(name) = queue.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let defs = match w.structs.get(name.as_str()) {
            Some(d) => d,
            None => continue,
        };
        let ns = defs[0].ns.clone();
        let b64 = w.struct_for(&name, true).and_then(|s| w.type_body(s, true));
        let b32 = w.struct_for(&name, false).and_then(|s| w.type_body(s, false));
        if b64.is_none() && b32.is_none() {
            counts.skipped_types += 1;
            continue;
        }
        let mut deps = BTreeSet::new();
        for (_, d, _) in b64.iter().chain(b32.iter()) {
            deps.extend(d.iter().cloned());
        }
        for d in &deps {
            queue.push(d.clone());
        }
        types.insert(name.clone(), TypeOut { name, ns, b64: b64.map(|b| b.0), b32: b32.map(|b| b.0), deps });
    }
    // (a TYPE whose field TYPE couldn't be written can't be either)
    loop {
        let missing: Vec<String> = types.values().filter(|t| t.deps.iter().any(|d| !types.contains_key(d))).map(|t| t.name.clone()).collect();
        if missing.is_empty() {
            break;
        }
        for m in missing {
            types.remove(&m);
            counts.skipped_types += 1;
        }
    }
    let emitted: HashSet<String> = types.keys().filter(|n| !rapidq.contains(&n.to_ascii_uppercase())).cloned().collect();
    // order: dependencies first, then by namespace and name
    let mut order: Vec<String> = Vec::new();
    let mut placed: HashSet<String> = HashSet::new();
    let mut sorted: Vec<&TypeOut> = types.values().collect();
    sorted.sort_by(|a, b| (a.ns.as_str(), a.name.as_str()).cmp(&(b.ns.as_str(), b.name.as_str())));
    fn place(n: &str, types: &BTreeMap<String, TypeOut>, placed: &mut HashSet<String>, order: &mut Vec<String>, depth: usize) {
        if depth > 64 || placed.contains(n) {
            return;
        }
        let Some(t) = types.get(n) else { return };
        placed.insert(n.to_string());
        for d in &t.deps {
            place(d, types, placed, order, depth + 1);
        }
        order.push(n.to_string());
    }
    for t in &sorted {
        place(&t.name, &types, &mut placed, &mut order, 0);
    }
    let mut type_out = String::new();
    for n in &order {
        let t = &types[n];
        if !emitted.contains(n) {
            continue;
        }
        if is_rapidr_builtin(&rr_type_name(n)) {
            counts.skipped_types += 1;
            continue;
        }
        if function_names.contains(&rr_type_name(n).to_ascii_uppercase()) || !taken.insert(rr_type_name(n).to_ascii_uppercase()) {
            counts.name_clashes += 1;
            continue;
        }
        if write_type(&mut type_out, &rr_type_name(n), &t.b64, &t.b32) {
            counts.types_arch_specific += 1;
        }
        counts.types += 1;
    }

    // ── the functions, by DLL
    let mut decl_out = String::new();
    let mut by_dll: BTreeMap<&str, Vec<&Func>> = BTreeMap::new();
    for f in &funcs {
        by_dll.entry(dll_short[f.dll.as_str()]).or_default().push(f);
    }
    // (32-bit Windows' names for the Ptr functions: GetWindowLongPtrA is GetWindowLongA there)
    let reexport32: HashMap<&str, &str> = md.reexports.iter().filter(|r| r.3 == Arch::Bit32).map(|r| (r.0.as_str(), r.1.as_str())).collect();
    let mut declared: BTreeMap<String, String> = BTreeMap::new(); // name → its DECLARE (for the classic names)
    for (lib, list) in &by_dll {
        let _ = writeln!(decl_out, "' --- {lib} ---");
        let mut names_here: BTreeSet<&str> = BTreeSet::new();
        for f in list {
            names_here.insert(&f.name);
        }
        for f in list {
            if f.varargs {
                *counts.skipped_functions.entry("takes any number of arguments".into()).or_default() += 1;
                continue;
            }
            if f.arch == Arch::Other || f.arch == Arch::Bit32 && list.iter().any(|g| g.name == f.name && g.arch == Arch::Bit64) {
                if f.arch == Arch::Other {
                    *counts.skipped_functions.entry("one 64-bit kind of Windows only".into()).or_default() += 1;
                }
                continue;
            }
            if is_reserved(&f.name) {
                *counts.skipped_functions.entry("a word of RapidR's".into()).or_default() += 1;
                continue;
            }
            let params: Option<Vec<String>> = f.params.iter().map(|(n, t)| param_text(&w, n, t, &emitted)).collect();
            let (Some(params), Some(ret)) = (params, ret_text(&w, &f.ret)) else {
                *counts.skipped_functions.entry("a structure passed or returned by value".into()).or_default() += 1;
                counts.skipped_function_names.push(f.name.clone());
                continue;
            };
            if !taken.insert(f.name.to_ascii_uppercase()) {
                counts.name_clashes += 1;
                continue;
            }
            let line = declare_line(&f.name, lib, &f.name, &params, ret);
            // 64-bit only (GetWindowLongPtrA, a "C" function): on 32-bit
            // Windows the same name is its 32-bit function, or absent
            let only64 = f.arch == Arch::Bit64 || f.abi != "system";
            if only64 {
                decl_out.push_str("$IFDEF WIN64\n");
                decl_out.push_str(&line);
                if let Some(target) = reexport32.get(f.name.as_str()) {
                    decl_out.push_str("$ELSE\n");
                    decl_out.push_str(&declare_line(&f.name, lib, target, &params, ret));
                }
                decl_out.push_str("$ENDIF\n");
            } else {
                decl_out.push_str(&line);
            }
            declared.insert(f.name.clone(), if only64 { String::new() } else { line });
            counts.functions += 1;
        }
    }

    // ── the classic names: the unsuffixed name of every A/W pair, the ANSI one
    let mut classic_out = String::new();
    classic_out.push_str("' --- the classic names: the unsuffixed name of each A/W pair is the A (ANSI) one ---\n");
    let struct_names: HashSet<&str> = types.keys().map(String::as_str).collect();
    let mut classic_types: BTreeMap<String, String> = BTreeMap::new(); // unsuffixed → the A struct
    for n in &order {
        if let Some(stem) = n.strip_suffix('A') {
            if stem.is_empty() || !struct_names.contains(format!("{stem}W").as_str()) || !emitted.contains(n) {
                continue;
            }
            // (TRUSTEE_A's generic name is TRUSTEE: a name never ends in _,
            // which RapidR reads as a line going on)
            let base = stem.trim_end_matches('_');
            if base.is_empty() {
                continue;
            }
            if taken.contains(&base.to_ascii_uppercase()) || is_reserved(base) || rapidq.contains(&base.to_ascii_uppercase()) {
                continue;
            }
            classic_types.insert(base.to_string(), n.clone());
        }
    }
    for (classic, metadata, why) in CLASSIC_TYPE_ALIASES {
        if emitted.contains(*metadata) && !taken.contains(&classic.to_ascii_uppercase()) {
            let _ = why;
            classic_types.insert(classic.to_string(), metadata.to_string());
        }
    }
    // the A TYPEs' bodies name other A TYPEs: the classic copy names the classic ones
    let classic_of: HashMap<String, String> = classic_types.iter().map(|(c, a)| (rr_type_name(a), c.clone())).collect();
    let rename = |line: &str| -> String {
        let mut l = line.to_string();
        if let Some(at) = l.rfind(" AS ") {
            let ty = l[at + 4..].to_string();
            if ty.ends_with('A') {
                if let Some(c) = classic_of.get(&ty) {
                    l = format!("{}{c}", &l[..at + 4]);
                }
            }
        }
        l
    };
    // (in the TYPEs' order: a classic TYPE after the ones its fields are)
    let mut classic_order: Vec<(&String, &String)> = classic_types.iter().collect();
    let pos: HashMap<&str, usize> = order.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    classic_order.sort_by_key(|(c, a)| (pos.get(a.as_str()).copied().unwrap_or(usize::MAX), (*c).clone()));
    for (classic, a) in classic_order {
        let t = &types[a];
        let b64 = t.b64.as_ref().map(|b| b.iter().map(|l| rename(l)).collect::<Vec<_>>());
        let b32 = t.b32.as_ref().map(|b| b.iter().map(|l| rename(l)).collect::<Vec<_>>());
        taken.insert(classic.to_ascii_uppercase());
        write_type(&mut classic_out, classic, &b64, &b32);
        counts.classic_types += 1;
    }
    let classic_param = |p: &str| -> String {
        if let Some(at) = p.rfind(" AS ") {
            let ty = &p[at + 4..];
            if let Some(c) = classic_of.get(ty) {
                return format!("{}{c}", &p[..at + 4]);
            }
        }
        p.to_string()
    };
    for (lib, list) in &by_dll {
        for f in list {
            // kernel32's _lopen, _lread …: `lopen`, as a name can't start with _ in older BASICs
            if let Some(stem) = f.name.strip_prefix('_') {
                let Some(line) = declared.get(&f.name) else { continue };
                if line.is_empty() || stem.is_empty() || stem.starts_with('_') || taken.contains(&stem.to_ascii_uppercase()) || is_reserved(stem) {
                    continue;
                }
                let params: Vec<String> = f.params.iter().map(|(n, t)| param_text(&w, n, t, &emitted).map(|p| classic_param(&p))).collect::<Option<_>>().unwrap_or_default();
                taken.insert(stem.to_ascii_uppercase());
                classic_out.push_str(&declare_line(stem, lib, &f.name, &params, ret_text(&w, &f.ret).flatten()));
                counts.classic_functions += 1;
                continue;
            }
            let Some(base) = f.name.strip_suffix('A') else { continue };
            if base.is_empty() || !list.iter().any(|g| g.name == format!("{base}W")) {
                continue;
            }
            let Some(line) = declared.get(&f.name) else { continue };
            if line.is_empty() || taken.contains(&base.to_ascii_uppercase()) || is_reserved(base) {
                continue;
            }
            let params: Vec<String> = f.params.iter().map(|(n, t)| param_text(&w, n, t, &emitted).map(|p| classic_param(&p))).collect::<Option<_>>().unwrap_or_default();
            let ret = ret_text(&w, &f.ret).flatten();
            taken.insert(base.to_ascii_uppercase());
            classic_out.push_str(&declare_line(base, lib, &f.name, &params, ret));
            counts.classic_functions += 1;
        }
    }

    let counts_line = format!(
        "{} constants, {} TYPEs ({} with a 64-bit and a 32-bit layout), {} functions; classic names: {} functions, {} TYPEs.",
        counts.consts, counts.types, counts.types_arch_specific, counts.functions, counts.classic_functions, counts.classic_types
    );
    let mut text = header(&src.sha256, &counts_line);
    text.push_str("\n' ===== constants =====\n");
    text.push_str(&const_out);
    text.push_str("\n' ===== TYPEs =====\n");
    text.push_str(&type_out);
    text.push_str("\n' ===== functions =====\n");
    text.push_str(&decl_out);
    text.push('\n');
    text.push_str(&classic_out);
    Output { text, counts }
}

/// The size Windows' compiler gives structure `name` (`None`: unknown or
/// not one RapidR can write): the generator's own layout, for the tests.
pub fn struct_size(md: &Metadata, name: &str, bit64: bool) -> Option<usize> {
    struct_sizes(md, bit64).remove(name)
}

/// [`struct_size`] of every structure at once.
pub fn struct_sizes(md: &Metadata, bit64: bool) -> HashMap<String, usize> {
    let w = World::new(md);
    md.structs.iter().filter_map(|s| Some((s.name.clone(), w.layout(w.struct_for(&s.name, bit64)?, bit64, 0)?.0))).collect()
}

/// The size RapidR gives each TYPE of a generated file, for one kind of
/// Windows (`bit64`), worked out from the file's own lines as RapidR lays a
/// TYPE out (packed: LONG 4, INT64 8, `STRING * n` n, a STRING 4, an array
/// field its elements, a TYPE field that TYPE's size).
pub fn rapidr_type_sizes(text: &str, bit64: bool) -> BTreeMap<String, usize> {
    let mut sizes: BTreeMap<String, usize> = BTreeMap::new();
    // which `$IFDEF WIN64` branch a line is in: (this branch taken)
    let mut stack: Vec<bool> = Vec::new();
    let mut current: Option<(String, usize)> = None;
    for line in text.lines() {
        let t = line.trim();
        if t == "$IFDEF WIN64" {
            stack.push(bit64);
            continue;
        }
        if t == "$IFNDEF WIN64" {
            stack.push(!bit64);
            continue;
        }
        if t == "$ELSE" {
            if let Some(top) = stack.last_mut() {
                *top = !*top;
            }
            continue;
        }
        if t == "$ENDIF" {
            stack.pop();
            continue;
        }
        if stack.iter().any(|on| !on) {
            continue;
        }
        if let Some(name) = t.strip_prefix("TYPE ") {
            current = Some((name.to_string(), 0));
        } else if t == "END TYPE" {
            if let Some((n, size)) = current.take() {
                sizes.insert(n.to_ascii_uppercase(), size);
            }
        } else if let Some((_, size)) = current.as_mut() {
            let Some(at) = t.find(" AS ") else { continue };
            let ty = &t[at + 4..];
            let count = t[..at].find('(').and_then(|o| t[..at][o + 1..].trim_end_matches(')').parse::<usize>().ok()).map_or(1, |k| k + 1);
            let one = if let Some(n) = ty.strip_prefix("STRING * ") {
                n.parse().unwrap_or(0)
            } else {
                match ty {
                    "BYTE" => 1,
                    "WORD" | "SHORT" => 2,
                    "LONG" | "SINGLE" | "STRING" => 4,
                    "INT64" | "UINT64" | "DOUBLE" => 8,
                    other => sizes.get(&other.to_ascii_uppercase()).copied().unwrap_or(usize::MAX / 4),
                }
            };
            *size += one * count;
        }
    }
    sizes
}

/// Whether a structure name is a nested anonymous part of another (`NAME_0`).
fn looks_anonymous(name: &str, w: &World) -> bool {
    let mut parts: Vec<&str> = name.split('_').collect();
    let mut tail = 0;
    while parts.len() > 1 && parts.last().is_some_and(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())) {
        parts.pop();
        tail += 1;
    }
    tail > 0 && w.structs.contains_key(parts.join("_").as_str())
}

fn declare_line(name: &str, lib: &str, alias: &str, params: &[String], ret: Option<&str>) -> String {
    let mark = if is_rapidr_builtin(name) { BUILTIN_MARK } else { "" };
    match ret {
        Some(r) => format!("DECLARE FUNCTION {name} LIB \"{lib}\" ALIAS \"{alias}\" ({}) AS {r}{mark}\n", params.join(", ")),
        None => format!("DECLARE SUB {name} LIB \"{lib}\" ALIAS \"{alias}\" ({}){mark}\n", params.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(text: &str) -> Metadata {
        let mut m = Metadata::default();
        parse_file("Test", text, &mut m);
        m
    }

    #[test]
    fn literals() {
        assert_eq!(parse_int("-1i32"), Some(-1));
        assert_eq!(parse_int("0x8000_u32 as _"), Some(0x8000));
        assert_eq!(parse_int("4294967295u32"), Some(4294967295));
        assert_eq!(parse_int("260u32"), Some(260));
        assert_eq!(parse_str_lit("windows_sys::core::s!(\"Button\")").as_deref(), Some("Button"));
        assert_eq!(parse_str_lit("windows_sys::core::s!(\"a\\\"b\")"), None);
    }

    #[test]
    fn layouts_follow_windows_compiler() {
        let m = md("#[repr(C)]\n#[derive(Clone, Copy)]\npub struct MSG {\n    pub hwnd: HWND,\n    pub message: u32,\n    pub wParam: WPARAM,\n    pub lParam: LPARAM,\n    pub time: u32,\n    pub pt: POINT,\n}\n#[repr(C)]\npub struct POINT {\n    pub x: i32,\n    pub y: i32,\n}\npub type HWND = *mut core::ffi::c_void;\npub type WPARAM = usize;\npub type LPARAM = isize;\n");
        let w = World::new(&m);
        let s = w.struct_for("MSG", true).unwrap();
        let (size, align, offs) = w.layout(s, true, 0).unwrap();
        assert_eq!((size, align, offs), (48, 8, vec![0, 8, 16, 24, 32, 36]));
        let (size32, _, offs32) = w.layout(s, false, 0).unwrap();
        assert_eq!((size32, offs32), (28, vec![0, 4, 8, 12, 16, 20]));
        let (body, deps, _) = w.type_body(s, true).unwrap();
        assert_eq!(body, vec!["    hwnd AS INT64", "    message AS LONG", "    rrPad1 AS LONG", "    wParam AS INT64", "    lParam AS INT64", "    time AS LONG", "    pt AS POINT", "    rrPad2 AS LONG"]);
        assert!(deps.contains("POINT"));
        let (body32, _, _) = w.type_body(s, false).unwrap();
        assert_eq!(body32, vec!["    hwnd AS LONG", "    message AS LONG", "    wParam AS LONG", "    lParam AS LONG", "    time AS LONG", "    pt AS POINT"]);
    }

    #[test]
    fn unions_and_text_pointers() {
        let m = md("pub type PSTR = *mut u8;\n#[repr(C)]\npub struct T {\n    pub cb: u32,\n    pub Anonymous: T_0,\n    pub psz: windows_sys::core::PSTR,\n    pub name: [u8; 8],\n}\n#[repr(C)]\npub union T_0 {\n    pub a: u16,\n    pub b: u64,\n}\n");
        let w = World::new(&m);
        let s = w.struct_for("T", true).unwrap();
        let (body, _, size) = w.type_body(s, true).unwrap();
        assert_eq!(size, 32);
        assert_eq!(body, vec!["    cb AS LONG", "    rrPad1 AS LONG", "    b AS UINT64", "    psz AS STRING", "    pszHi AS LONG", "    name AS STRING * 8"]);
        let (body32, _, size32) = w.type_body(s, false).unwrap();
        // (a u64 is aligned to 8 on 32-bit Windows too)
        assert_eq!(size32, 32);
        assert_eq!(body32, vec!["    cb AS LONG", "    rrPad1 AS LONG", "    b AS UINT64", "    psz AS STRING", "    name AS STRING * 8", "    rrPad2 AS LONG"]);
    }

    #[test]
    fn constants_keep_their_bits() {
        let m = md("pub const A: u32 = 4294967295u32;\npub const B: i32 = -1i32;\npub const C: HKEY = -2147483648i32 as _;\npub const D: u32 = 0x80000000u32;\npub const E: u64 = 4294967296u64;\npub type HKEY = *mut core::ffi::c_void;\n");
        let w = World::new(&m);
        let t: Vec<String> = m.consts.iter().map(|c| const_text(&w, c).unwrap()).collect();
        assert_eq!(t, vec!["&HFFFFFFFF", "-1", "&H80000000", "&H80000000", "&H100000000"]);
    }

    /// The input, when cargo has it on this machine (it is in Cargo.lock).
    fn real_source() -> Option<Source> {
        let root = default_source_dir()?;
        read_source(&root).ok()
    }

    /// Every TYPE of the real file is as big in RapidR (its fields packed,
    /// the gaps written out) as Windows' compiler makes the structure, on
    /// both kinds of Windows.
    #[test]
    fn every_type_has_windows_size() {
        let Some(src) = real_source() else {
            eprintln!("(windows-sys {WINDOWS_SYS_VERSION} isn't in cargo's cache here: skipped)");
            return;
        };
        let md = parse(&src);
        let out = generate(&src, rapidr_preprocessor::RAPIDQ_INC_CONSTANTS);
        for bit64 in [true, false] {
            let sizes = rapidr_type_sizes(&out.text, bit64);
            assert!(sizes.len() > 3000, "{}", sizes.len());
            let want_sizes = struct_sizes(&md, bit64);
            let mut checked = 0;
            // (RapidR's names don't tell case apart: COLOR and Color are one
            // name, written once; such names are left to the oracle test)
            let mut spellings: HashMap<String, HashSet<&str>> = HashMap::new();
            for s in &md.structs {
                spellings.entry(rr_type_name(&s.name).to_ascii_uppercase()).or_default().insert(&s.name);
            }
            for s in &md.structs {
                let rr = rr_type_name(&s.name).to_ascii_uppercase();
                if spellings[&rr].len() > 1 {
                    continue;
                }
                let (Some(&got), Some(&want)) = (sizes.get(&rr), want_sizes.get(&s.name)) else { continue };
                assert_eq!(got, want, "{} on {}-bit Windows", s.name, if bit64 { 64 } else { 32 });
                checked += 1;
            }
            assert!(checked > 3000, "{checked}");
        }
    }

    /// The layouts agree with Rust's own for the same metadata (windows-sys
    /// compiled for this 64-bit machine): the oracle for the generator's
    /// alignment rules.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn layouts_agree_with_windows_sys() {
        use std::mem::size_of;
        use windows_sys::Win32::{Foundation as F, Graphics::Gdi as G, Media::Audio as A, Media::Multimedia as M, Networking::WinSock as WS, Security as SEC, Storage::FileSystem as FS, System::SystemInformation as SI, System::Threading as T, UI::Controls as C, UI::Controls::Dialogs as D, UI::Input::KeyboardAndMouse as K, UI::Shell as SH, UI::WindowsAndMessaging as W};
        let Some(src) = real_source() else { return };
        let md = parse(&src);
        let cases: &[(&str, usize)] = &[
            ("RECT", size_of::<F::RECT>()),
            ("POINT", size_of::<F::POINT>()),
            ("SYSTEMTIME", size_of::<F::SYSTEMTIME>()),
            ("FILETIME", size_of::<F::FILETIME>()),
            ("MSG", size_of::<W::MSG>()),
            ("WNDCLASSEXA", size_of::<W::WNDCLASSEXA>()),
            ("MENUITEMINFOA", size_of::<W::MENUITEMINFOA>()),
            ("WINDOWPLACEMENT", size_of::<W::WINDOWPLACEMENT>()),
            ("NONCLIENTMETRICSA", size_of::<W::NONCLIENTMETRICSA>()),
            ("NOTIFYICONDATAA", size_of::<SH::NOTIFYICONDATAA>()),
            ("NOTIFYICONDATAW", size_of::<SH::NOTIFYICONDATAW>()),
            ("SHFILEOPSTRUCTA", size_of::<SH::SHFILEOPSTRUCTA>()),
            ("OPENFILENAMEA", size_of::<D::OPENFILENAMEA>()),
            ("CHOOSECOLORA", size_of::<D::CHOOSECOLORA>()),
            ("CHOOSEFONTA", size_of::<D::CHOOSEFONTA>()),
            ("LOGFONTA", size_of::<G::LOGFONTA>()),
            ("TEXTMETRICA", size_of::<G::TEXTMETRICA>()),
            ("BITMAPINFOHEADER", size_of::<G::BITMAPINFOHEADER>()),
            ("PAINTSTRUCT", size_of::<G::PAINTSTRUCT>()),
            ("DEVMODEA", size_of::<G::DEVMODEA>()),
            ("MONITORINFO", size_of::<G::MONITORINFO>()),
            ("WAVEFORMATEX", size_of::<A::WAVEFORMATEX>()),
            ("WAVEHDR", size_of::<A::WAVEHDR>()),
            ("MIDIHDR", size_of::<A::MIDIHDR>()),
            ("MIDIOUTCAPSA", size_of::<A::MIDIOUTCAPSA>()),
            ("MCI_OPEN_PARMSA", size_of::<M::MCI_OPEN_PARMSA>()),
            ("TCITEMA", size_of::<C::TCITEMA>()),
            ("LVITEMA", size_of::<C::LVITEMA>()),
            ("NMHDR", size_of::<C::NMHDR>()),
            ("INPUT", size_of::<K::INPUT>()),
            ("KEYBDINPUT", size_of::<K::KEYBDINPUT>()),
            ("SECURITY_ATTRIBUTES", size_of::<SEC::SECURITY_ATTRIBUTES>()),
            ("WIN32_FIND_DATAA", size_of::<FS::WIN32_FIND_DATAA>()),
            ("STARTUPINFOA", size_of::<T::STARTUPINFOA>()),
            ("PROCESS_INFORMATION", size_of::<T::PROCESS_INFORMATION>()),
            ("MEMORYSTATUSEX", size_of::<SI::MEMORYSTATUSEX>()),
            ("OSVERSIONINFOA", size_of::<SI::OSVERSIONINFOA>()),
            ("SYSTEM_INFO", size_of::<SI::SYSTEM_INFO>()),
            ("WSADATA", size_of::<WS::WSADATA>()),
            ("SOCKADDR_IN", size_of::<WS::SOCKADDR_IN>()),
        ];
        let sizes = struct_sizes(&md, true);
        for (name, want) in cases {
            assert_eq!(sizes.get(*name), Some(want), "{name}");
        }
    }

    /// The same input gives the same bytes, and the committed file is them.
    #[test]
    fn generation_is_reproducible() {
        let Some(src) = real_source() else { return };
        let a = generate(&src, rapidr_preprocessor::RAPIDQ_INC_CONSTANTS);
        let b = generate(&read_source(&src.root).unwrap(), rapidr_preprocessor::RAPIDQ_INC_CONSTANTS);
        assert!(a.text == b.text, "two generations differ");
        let committed = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../crates/rapidr-preprocessor/src/libraries/RWin32.inc")).unwrap();
        assert!(committed == a.text, "RWin32.inc is not what the input gives: run `cargo run -p rapidr-gen-win32 --release`");
    }

    #[test]
    fn arch_specific_functions() {
        let m = md("#[cfg(any(target_arch = \"aarch64\", target_arch = \"arm64ec\", target_arch = \"x86_64\"))]\nwindows_link::link!(\"user32.dll\" \"system\" fn GetWindowLongPtrA(hwnd : HWND, nindex : i32) -> isize);\n#[cfg(target_pointer_width = \"32\")]\npub use GetWindowLongA as GetWindowLongPtrA;\n");
        assert_eq!(m.funcs[0].arch, Arch::Bit64);
        assert_eq!(m.funcs[0].params.len(), 2);
        assert_eq!(m.reexports, vec![("GetWindowLongPtrA".to_string(), "GetWindowLongA".to_string(), "Test".to_string(), Arch::Bit32)]);
    }
}
