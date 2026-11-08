//! `$INCLUDE "RWin32.inc"`: RapidR's one standard set of Windows API
//! declarations (`libraries/RWin32.inc`, written by `tools/gen-win32` from
//! Microsoft's Win32 metadata; docs/legal/win32-declarations-provenance.md).
//!
//! The file is big (tens of thousands of constants, thousands of TYPEs and
//! functions), so a program gets only what it uses:
//!
//! - **Built in**: the name `RWin32.inc` in any case and with any path is
//!   this file, never one on disk; desktop builds carry it compressed and
//!   unpack it the first time a program includes it; the web fetches it from
//!   beside the runtime then ([`set_reader`]), once per visit.
//! - **Only what is used**: the preprocessor reads the program once with the
//!   include empty, collects the words it uses ([`select`]), then reads it
//!   again with the include holding those entries and the TYPEs they need, on
//!   their own lines of `<RapidR>/RWin32.inc` (every other line blank), so a
//!   message points at the file's real line.
//! - **A name of the program's own always wins** ([`own_names`]): its CONST,
//!   DECLARE, TYPE, SUB, FUNCTION, DIM, CREATE, `$DEFINE`, a variable it
//!   stores into, and what RAPIDQ.INC gives it are never declared again.
//! - **RAPIDQ.INC's Windows numbers** (MB_OK, VK_F1, the charsets, True and
//!   False …) aren't in the file: RapidR's compatibility layer has them. A
//!   classic program that uses RWin32.inc without RAPIDQ.INC gets those it
//!   uses, with RAPIDQ.INC's numbers, as the classic WIN32API.INC gave them; a
//!   RapidR program (`.rr`) has them built in already.
//! - **RapidR's built-ins**: a DECLARE marked "a RapidR built-in too"
//!   (`Sleep`, `Beep`, `MessageBox` …) is given to a classic program, as its
//!   old include did; a RapidR program keeps RapidR's own.

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};

/// The include's file name.
pub const FILE_NAME: &str = "RWin32.inc";

/// The mark the generator puts after a DECLARE of a name RapidR has a
/// built-in of.
const BUILTIN_MARK: &str = "' (a RapidR built-in too)";

/// Whether `include_file` (as an `$INCLUDE` writes it) is RWin32.inc.
pub fn is_rwin32(include_file: &str) -> bool {
    let name = include_file.trim().rsplit(['/', '\\']).next().unwrap_or("");
    name.eq_ignore_ascii_case(FILE_NAME)
}

/// The compressed file desktop builds carry (gzip; `tools/gen-win32`
/// writes it beside the text, and a test checks it is the same text).
#[cfg(not(target_arch = "wasm32"))]
static COMPRESSED: &[u8] = include_bytes!("libraries/RWin32.inc.gz");

/// Where the compressed file comes from instead: the web runtime's fetch of
/// `RWin32.inc.gz` beside it (a synchronous request of the page's own
/// origin, made the first time a program includes the file).
pub type Reader = fn() -> Option<Vec<u8>>;

static READER: RwLock<Option<Reader>> = RwLock::new(None);

/// Reads RWin32.inc's compressed bytes through `reader` (the web).
pub fn set_reader(reader: Option<Reader>) {
    if let Ok(mut r) = READER.write() {
        *r = reader;
    }
}

fn compressed() -> Option<Vec<u8>> {
    if let Some(reader) = READER.read().ok().and_then(|r| *r) {
        return reader();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Some(COMPRESSED.to_vec())
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

fn unpack(bytes: &[u8]) -> Option<String> {
    use std::io::Read;
    let mut text = String::new();
    flate2::read::GzDecoder::new(bytes).read_to_string(&mut text).ok()?;
    Some(text)
}

/// The file's text and its index, read once (`None`: it can't be had —
/// the web page's fetch failed).
fn index() -> Option<&'static Index> {
    static INDEX: OnceLock<Option<Index>> = OnceLock::new();
    INDEX.get_or_init(|| compressed().and_then(|b| unpack(&b)).map(Index::new)).as_ref()
}

/// The full text of RWin32.inc (`None`: not available here).
pub fn text() -> Option<&'static str> {
    index().map(|i| i.text.as_str())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Const,
    Declare,
    Type,
}

#[derive(Debug)]
struct Entry {
    kind: Kind,
    /// the lines it is on (from 0), with its `$IFDEF WIN64` lines
    ranges: Vec<(usize, usize)>,
    /// the TYPEs it names (upper case)
    deps: Vec<String>,
    /// a DECLARE of a name RapidR has a built-in of
    builtin: bool,
}

struct Index {
    text: String,
    /// where each line starts in `text`, and one past the end
    starts: Vec<usize>,
    /// upper-case name → entry
    entries: HashMap<String, Entry>,
    /// RAPIDQ.INC's names that are Windows' too (upper case → as written)
    rapidq: HashMap<String, String>,
}

/// A name's words after ` AS ` (a field's or parameter's type).
fn as_types(line: &str) -> impl Iterator<Item = String> + '_ {
    line.match_indices(" AS ").filter_map(move |(at, _)| {
        let rest = &line[at + 4..];
        let word: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        (!word.is_empty()).then(|| word.to_ascii_uppercase())
    })
}

impl Index {
    fn new(text: String) -> Index {
        let mut starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        if *starts.last().unwrap_or(&0) != text.len() {
            starts.push(text.len());
        }
        let mut entries: HashMap<String, Entry> = HashMap::new();
        let mut rapidq = HashMap::new();
        // an open `$IFDEF WIN64` group: (its first line, the names in it)
        let mut group: Option<(usize, Vec<String>)> = None;
        // an open TYPE: (its name, its first line, its field types)
        let mut open_type: Option<(String, usize, Vec<String>)> = None;
        let lines: Vec<&str> = text.split('\n').collect();
        fn add(entries: &mut HashMap<String, Entry>, name: String, kind: Kind, range: (usize, usize), deps: Vec<String>, builtin: bool) {
            let e = entries.entry(name).or_insert(Entry { kind, ranges: Vec::new(), deps: Vec::new(), builtin });
            e.ranges.push(range);
            for d in deps {
                if !e.deps.contains(&d) {
                    e.deps.push(d);
                }
            }
        }
        for (i, line) in lines.iter().enumerate() {
            let line = line.trim_end_matches('\r');
            if let Some((name, first, deps)) = open_type.as_mut() {
                if line == "END TYPE" {
                    let (name, first, deps) = (name.clone(), *first, std::mem::take(deps));
                    open_type = None;
                    if let Some((_, names)) = group.as_mut() {
                        names.push(name.clone());
                    }
                    add(&mut entries, name, Kind::Type, (first, i), deps, false);
                } else {
                    deps.extend(as_types(line));
                }
                continue;
            }
            if line == "$IFDEF WIN64" || line == "$IFNDEF WIN64" {
                group = Some((i, Vec::new()));
            } else if line == "$ENDIF" {
                if let Some((first, names)) = group.take() {
                    // (the whole group is the entry's: both layouts and the
                    // directives that pick one)
                    for n in names {
                        if let Some(e) = entries.get_mut(&n) {
                            e.ranges.retain(|&(a, _)| a < first);
                            e.ranges.push((first, i));
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("CONST ") {
                let name = rest.split(" = ").next().unwrap_or("").trim().to_ascii_uppercase();
                add(&mut entries, name, Kind::Const, (i, i), Vec::new(), false);
            } else if let Some(rest) = line.strip_prefix("DECLARE ") {
                let rest = rest.strip_prefix("FUNCTION ").or_else(|| rest.strip_prefix("SUB ")).unwrap_or(rest);
                let name = rest.split(' ').next().unwrap_or("").to_ascii_uppercase();
                if let Some((_, names)) = group.as_mut() {
                    names.push(name.clone());
                }
                add(&mut entries, name, Kind::Declare, (i, i), as_types(line).collect(), line.ends_with(BUILTIN_MARK));
            } else if let Some(name) = line.strip_prefix("TYPE ") {
                open_type = Some((name.trim().to_ascii_uppercase(), i, Vec::new()));
            } else if let Some(names) = line.strip_prefix("' RAPIDQ.INC: ") {
                for n in names.split_whitespace() {
                    rapidq.insert(n.to_ascii_uppercase(), n.to_string());
                }
            }
        }
        // (a dependency that is no TYPE of the file — LONG, STRING — goes)
        let types: HashSet<String> = entries.iter().filter(|(_, e)| e.kind == Kind::Type).map(|(n, _)| n.clone()).collect();
        for e in entries.values_mut() {
            e.deps.retain(|d| types.contains(d));
        }
        Index { text, starts, entries, rapidq }
    }

    fn line(&self, i: usize) -> &str {
        let end = self.starts.get(i + 1).map_or(self.text.len(), |&e| e.saturating_sub(1));
        self.text.get(self.starts[i]..end).unwrap_or("")
    }
}

/// RWin32.inc's declaration of `name` (any case): its CONST or DECLARE line
/// (the 64-bit one of two), or `TYPE NAME` (`None`: the file has no such
/// name, or isn't available here). For tools: the importer's comparison
/// with a program's old include file.
pub fn declaration(name: &str) -> Option<&'static str> {
    let index = index()?;
    let e = index.entries.get(&name.to_ascii_uppercase())?;
    let &(a, b) = e.ranges.first()?;
    (a..=b).map(|i| index.line(i)).find(|l| !l.starts_with('$'))
}

/// Whether RAPIDQ.INC (RapidR's compatibility layer) has `name`, one of
/// the Windows numbers RWin32.inc leaves to it.
pub fn left_to_rapidq_inc(name: &str) -> bool {
    index().is_some_and(|i| i.rapidq.contains_key(&name.to_ascii_uppercase()))
}

/// The words a program's text uses (upper case): its identifiers outside
/// strings and comments.
pub fn used_words(text: &str) -> HashSet<String> {
    let mut words = HashSet::new();
    for line in text.lines() {
        let mut word = String::new();
        let mut in_string = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if in_string {
                if c == '"' {
                    in_string = false;
                }
                continue;
            }
            if c.is_ascii_alphanumeric() || c == '_' {
                word.push(c);
                continue;
            }
            if !word.is_empty() {
                let w = std::mem::take(&mut word).to_ascii_uppercase();
                if w == "REM" {
                    break;
                }
                words.insert(w);
            }
            match c {
                '"' => in_string = true,
                '\'' => break,
                _ => {}
            }
        }
        if !word.is_empty() {
            let w = word.to_ascii_uppercase();
            if w != "REM" {
                words.insert(w);
            }
        }
    }
    words.retain(|w| !w.chars().next().is_some_and(|c| c.is_ascii_digit()));
    words
}

/// A statement's text split at `:` outside strings, its comment cut off.
fn statements(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_string = false;
    for c in line.chars() {
        match c {
            '"' => {
                in_string = !in_string;
                cur.push(c);
            }
            '\'' if !in_string => break,
            ':' if !in_string => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// An identifier at the start of `s` (upper case), a type suffix dropped,
/// and the text after it.
fn ident(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    let end = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(s.len());
    if end == 0 || s.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let rest = s[end..].trim_start_matches(['$', '%', '&', '!', '#', '?']);
    Some((s[..end].to_ascii_uppercase(), rest))
}

/// The names a list of declarators declares (`a AS LONG, b(3), c = 5`):
/// each at the start or after a comma outside brackets.
fn declarators(s: &str, into: &mut HashSet<String>) {
    let mut depth = 0i32;
    let mut start = true;
    let mut i = 0;
    let b = s.as_bytes();
    while i < b.len() {
        let c = b[i] as char;
        if start && depth == 0 && (c.is_ascii_alphabetic() || c == '_') {
            if let Some((name, _)) = ident(&s[i..]) {
                if name != "SHARED" {
                    into.insert(name.clone());
                    start = false;
                }
                i += name.len();
                continue;
            }
        }
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '"' => {
                // (a string: to its end)
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += 1;
                }
            }
            ',' if depth == 0 => start = true,
            _ => {}
        }
        i += 1;
    }
}

/// The names a program has of its own (upper case), as two kinds: its
/// TYPEs, and the rest (a variable and a TYPE may share a name: `DIM msg AS
/// MSG`).
#[derive(Clone, Debug, Default)]
pub struct Own {
    pub types: HashSet<String>,
    pub values: HashSet<String>,
}

impl Own {
    /// A name of both kinds (a `$DEFINE`: its text stands everywhere).
    pub fn insert_everywhere(&mut self, name: String) {
        self.types.insert(name.clone());
        self.values.insert(name);
    }
}

/// The names a program makes its own: what it declares — CONST, DECLARE,
/// SUB, FUNCTION, TYPE, CREATE, a DIM in its main program — and the
/// variables it stores into (`x = …`, FOR, INPUT, READ, SWAP, INC).
pub fn own_names(text: &str) -> Own {
    let mut types = HashSet::new();
    let mut own = HashSet::new();
    let mut in_routine = false;
    let mut in_type = false;
    for line in text.lines() {
        for st in statements(line) {
            let Some((first, rest)) = ident(&st) else { continue };
            let (first, rest) = match first.as_str() {
                "PUBLIC" | "PRIVATE" | "GLOBAL" => match ident(rest) {
                    Some((f, r)) if matches!(f.as_str(), "CONST" | "DECLARE" | "SUB" | "FUNCTION" | "TYPE" | "DIM") => (f, r),
                    _ => {
                        declarators(rest, &mut own);
                        continue;
                    }
                },
                _ => (first, rest),
            };
            if in_type {
                if first == "END" && ident(rest).is_some_and(|(w, _)| w == "TYPE" || w == "STRUCT") {
                    in_type = false;
                }
                continue;
            }
            match first.as_str() {
                "CONST" => {
                    // every `name =` of the statement
                    let mut seg = rest;
                    loop {
                        if let Some((n, after)) = ident(seg) {
                            if after.trim_start().starts_with('=') {
                                own.insert(n);
                            }
                        }
                        match seg.find(',') {
                            Some(c) => seg = &seg[c + 1..],
                            None => break,
                        }
                    }
                }
                "DECLARE" => {
                    if let Some((kind, r)) = ident(rest) {
                        let r = if matches!(kind.as_str(), "SUB" | "FUNCTION" | "SUBI" | "FUNCTIONI") { r } else { rest };
                        if let Some((n, _)) = ident(r) {
                            own.insert(n);
                        }
                    }
                }
                "SUB" | "FUNCTION" | "SUBI" | "FUNCTIONI" => {
                    if let Some((n, _)) = ident(rest) {
                        own.insert(n);
                    }
                    in_routine = true;
                }
                "END" => {
                    if ident(rest).is_some_and(|(w, _)| matches!(w.as_str(), "SUB" | "FUNCTION" | "SUBI" | "FUNCTIONI")) {
                        in_routine = false;
                    }
                }
                "TYPE" | "STRUCT" => {
                    if let Some((n, _)) = ident(rest) {
                        types.insert(n);
                    }
                    in_type = true;
                }
                "CREATE" => {
                    if let Some((n, _)) = ident(rest) {
                        own.insert(n);
                    }
                }
                "DIM" | "REDIM" | "STATIC" | "DEFINT" | "DEFLNG" | "DEFSTR" | "DEFDBL" | "DEFSNG" | "DEFBYTE" | "DEFWORD" | "DEFSHORT" | "DEFDWORD" | "DEFINT64" => {
                    if !in_routine {
                        declarators(rest, &mut own);
                    }
                }
                "FOR" | "LET" | "INC" | "DEC" => {
                    if let Some((n, _)) = ident(rest) {
                        own.insert(n);
                    }
                }
                "INPUT" | "READ" | "SWAP" => {
                    // (INPUT "prompt"; a, b: the prompt isn't a name)
                    let r = rest.trim_start();
                    let r = if r.starts_with('"') { r.find([';', ',']).map_or("", |p| &r[p + 1..]) } else { r };
                    declarators(r, &mut own);
                }
                "LINE" => {
                    if let Some(("INPUT", r)) = ident(rest).as_ref().map(|(w, r)| (w.as_str(), *r)) {
                        let r = r.trim_start();
                        let r = if r.starts_with('"') { r.find([';', ',']).map_or("", |p| &r[p + 1..]) } else { r };
                        declarators(r, &mut own);
                    }
                }
                _ => {
                    // `name = …` (not `name.member = …`, not `name(i) = …`)
                    if rest.trim_start().starts_with('=') && !rest.trim_start().starts_with("==") {
                        own.insert(first);
                    }
                }
            }
        }
    }
    Own { types, values: own }
}

/// What a program gets of RWin32.inc.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// The file's text with every line the program doesn't need blank (the
    /// same number of lines).
    pub text: String,
    /// RAPIDQ.INC's Windows numbers it uses without RAPIDQ.INC (a classic
    /// program): (the name as RAPIDQ.INC writes it, its value).
    pub rapidq_constants: Vec<(String, i64)>,
    /// How many entries were given (for tests and `rapidr check`).
    pub entries: usize,
}

/// What the program whose text is `program` (read with the include empty)
/// gets: the entries it uses, minus the names it has (`own`), with the
/// TYPEs they need. `rapidr_program`: a `.rr` file (RAPIDQ.INC's numbers
/// built in, RapidR's built-ins kept). `rapidq_inc`: RAPIDQ.INC's constants.
pub fn select(program: &str, own: &Own, rapidr_program: bool, rapidq_inc: &[(&str, i64)]) -> Option<Selection> {
    let index = index()?;
    let used = used_words(program);
    let mut chosen: Vec<&str> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    let mut queue: Vec<&str> = Vec::new();
    for w in &used {
        if let Some((name, e)) = index.entries.get_key_value(w) {
            let mine = if e.kind == Kind::Type { own.types.contains(w) } else { own.values.contains(w) };
            if mine || rapidr_program && e.builtin {
                continue;
            }
            queue.push(name.as_str());
        }
    }
    while let Some(n) = queue.pop() {
        if !seen.insert(n) {
            continue;
        }
        chosen.push(n);
        for d in &index.entries[n].deps {
            if !own.types.contains(d) {
                if let Some((dn, _)) = index.entries.get_key_value(d) {
                    queue.push(dn.as_str());
                }
            }
        }
    }
    // the text: the chosen entries' lines, every other one blank
    let lines = index.starts.len().saturating_sub(1);
    let mut keep = vec![false; lines.max(1)];
    for n in &chosen {
        for &(a, b) in &index.entries[*n].ranges {
            for k in keep.iter_mut().take(b + 1).skip(a) {
                *k = true;
            }
        }
    }
    let mut text = String::with_capacity(lines + 1024);
    for (i, k) in keep.iter().enumerate() {
        if i > 0 {
            text.push('\n');
        }
        if *k {
            text.push_str(index.line(i));
        }
    }
    // RAPIDQ.INC's Windows numbers, for a classic program that uses them
    // and has them from nowhere else
    let mut rapidq_constants = Vec::new();
    if !rapidr_program {
        let mut names: Vec<&String> = used.iter().filter(|w| index.rapidq.contains_key(*w) && !own.values.contains(*w)).collect();
        names.sort();
        for w in names {
            if let Some((n, v)) = rapidq_inc.iter().find(|(n, _)| n.eq_ignore_ascii_case(w)) {
                rapidq_constants.push((n.to_string(), *v));
            }
        }
    }
    Some(Selection { text, rapidq_constants, entries: chosen.len() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_there_and_indexed() {
        let i = index().expect("RWin32.inc unpacks");
        assert!(i.entries.len() > 60_000, "{}", i.entries.len());
        let mb = &i.entries["MESSAGEBOX"];
        assert_eq!(mb.kind, Kind::Declare);
        assert!(mb.builtin, "MessageBox is a RapidR built-in too");
        assert!(i.entries["GETWINDOWRECT"].deps.contains(&"RECT".to_string()));
        assert!(i.rapidq.contains_key("MB_OK") && i.rapidq.contains_key("TRUE"));
        // a TYPE with two layouts: one entry, its $IFDEF group whole
        let msg = &i.entries["MSG"];
        assert_eq!(msg.ranges.len(), 1);
        assert_eq!(i.line(msg.ranges[0].0), "$IFDEF WIN64");
        assert_eq!(i.line(msg.ranges[0].1), "$ENDIF");
        assert!(msg.deps.contains(&"POINT".to_string()));
    }

    #[test]
    fn names() {
        assert!(is_rwin32("RWin32.inc") && is_rwin32("c:\\x\\rwin32.INC") && is_rwin32("inc/RWIN32.inc"));
        assert!(!is_rwin32("WIN32API.INC") && !is_rwin32("RWin32.inc.bak"));
        let own = own_names("CONST WM_MOUSEWHEEL = &H20A, MY_X = 2\nDECLARE FUNCTION GetTickCount LIB \"kernel32\" () AS LONG\nTYPE POINTAPI\n x AS LONG\nEND TYPE\nDIM hWnd AS LONG, rc AS RECT\nCREATE Form AS QFORM\nEND CREATE\nINFINITE = 5 : FOR i = 1 TO 2 : NEXT\nSUB Go(cx AS LONG)\n DIM local AS LONG\nEND SUB\nINPUT \"x\"; answer\nr.Left = 4");
        for n in ["WM_MOUSEWHEEL", "MY_X", "GETTICKCOUNT", "HWND", "RC", "FORM", "INFINITE", "I", "GO", "ANSWER"] {
            assert!(own.values.contains(n), "{n}");
        }
        assert!(own.types.contains("POINTAPI") && !own.values.contains("POINTAPI"));
        for n in ["X", "LOCAL", "CX", "RECT", "R", "LEFT"] {
            assert!(!own.values.contains(n) && !own.types.contains(n), "{n}");
        }
        let used = used_words("PRINT GetTickCount ' SendMessage\nMessageBox 0, \"FindWindow\", \"\", MB_OK : REM PostMessage");
        assert!(used.contains("GETTICKCOUNT") && used.contains("MESSAGEBOX") && used.contains("MB_OK"));
        assert!(!used.contains("SENDMESSAGE") && !used.contains("FINDWINDOW") && !used.contains("POSTMESSAGE"));
    }

    #[test]
    fn selection_takes_what_is_used_and_its_types() {
        let program = "DIM r AS RECT\nPRINT GetWindowRect(0, r), GetTickCount, MB_ICONERROR, WM_CLOSE\nCONST SW_SHOW = 99\n";
        let own = own_names(program);
        let s = select(program, &own, false, &[("MB_ICONERROR", 16), ("MB_OK", 0)]).unwrap();
        assert!(s.text.lines().any(|l| l.starts_with("DECLARE FUNCTION GetWindowRect ")));
        assert!(s.text.lines().any(|l| l == "TYPE RECT"));
        assert!(s.text.lines().any(|l| l.starts_with("CONST WM_CLOSE = ")));
        assert!(!s.text.lines().any(|l| l.starts_with("CONST SW_SHOW ")), "the program's own SW_SHOW wins");
        assert!(!s.text.lines().any(|l| l.starts_with("DECLARE FUNCTION SendMessage")));
        assert_eq!(s.rapidq_constants, vec![("MB_ICONERROR".to_string(), 16)]);
        assert_eq!(s.text.split('\n').count(), index().unwrap().starts.len() - 1, "every line of the file keeps its number");
        // a RapidR program: RapidR's own Sleep, RAPIDQ.INC's numbers built in
        let rr = select("SLEEP 1\nPRINT MB_ICONERROR\n", &Own::default(), true, &[("MB_ICONERROR", 16)]).unwrap();
        assert!(!rr.text.lines().any(|l| l.starts_with("DECLARE SUB Sleep ")) && rr.rapidq_constants.is_empty());
        let bas = select("Sleep 1\n", &Own::default(), false, &[]).unwrap();
        assert!(bas.text.lines().any(|l| l.starts_with("DECLARE SUB Sleep ")));
        // a variable named as a TYPE: both are the program's
        let both = "DIM msg AS MSG\nmsg = msg\n";
        let s = select(both, &own_names(both), false, &[]).unwrap();
        assert!(s.text.lines().any(|l| l == "TYPE MSG"));
    }
}
