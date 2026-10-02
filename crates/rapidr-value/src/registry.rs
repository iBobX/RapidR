//! QREGISTRY (RapidQ manual, Appendix A): keys holding sub-keys and named
//! values, under a root (HKEY_CURRENT_USER by default). RapidR keeps them in
//! a per-user store, the same on every platform — a text file in the
//! Windows `.reg` format (Regedit's), in the user's settings folder (on the
//! web, the page's local storage) — so a program's settings persist between
//! runs the way RapidQ's did in the registry; nothing outside the store is
//! read or changed.
//!
//! The API is Delphi's TRegistry, which QREGISTRY's names follow: OpenKey /
//! KeyExists / CreateKey / DeleteKey take a path relative to the open key
//! (`\` first: from the root); names are case-insensitive and keep their
//! case; GetDataType is TRegDataType's (0 unknown, 1 string, 2 expandable
//! string, 3 integer, 4 binary); GetDataSize is -1 for no such value; a
//! float is stored as its 8 bytes. The functions answer 1 or 0 (the
//! manual's KeyExists: "Returns 0 or 1"). ReadBinary(Name, Index) keeps
//! RapidQ's bug — the first byte is Index -1 — which its programs count on.

use std::cell::{Cell, RefCell};

use crate::{v_int, v_str, Value};

pub const HKEY_CLASSES_ROOT: u32 = 0x8000_0000;
pub const HKEY_CURRENT_USER: u32 = 0x8000_0001;
pub const HKEY_LOCAL_MACHINE: u32 = 0x8000_0002;
pub const HKEY_USERS: u32 = 0x8000_0003;
pub const HKEY_PERFORMANCE_DATA: u32 = 0x8000_0004;
pub const HKEY_CURRENT_CONFIG: u32 = 0x8000_0005;
pub const HKEY_DYN_DATA: u32 = 0x8000_0006;

const ROOTS: [(u32, &str); 7] = [
    (HKEY_CLASSES_ROOT, "HKEY_CLASSES_ROOT"),
    (HKEY_CURRENT_USER, "HKEY_CURRENT_USER"),
    (HKEY_LOCAL_MACHINE, "HKEY_LOCAL_MACHINE"),
    (HKEY_USERS, "HKEY_USERS"),
    (HKEY_PERFORMANCE_DATA, "HKEY_PERFORMANCE_DATA"),
    (HKEY_CURRENT_CONFIG, "HKEY_CURRENT_CONFIG"),
    (HKEY_DYN_DATA, "HKEY_DYN_DATA"),
];

/// Largest value kept (the registry's own limit is about this), and the
/// longest key or value name.
const MAX_DATA: usize = 1 << 20;
const MAX_NAME: usize = 16_383;
/// Most keys under one key and values in one key (a runaway loop can't
/// fill the disk).
const MAX_ITEMS: usize = 100_000;

/// A value's data.
#[derive(Clone, Debug, PartialEq)]
pub enum Data {
    Str(String),
    /// REG_EXPAND_SZ (read as a string, as TRegistry does).
    Expand(String),
    Int(i32),
    Bin(Vec<u8>),
}

impl Data {
    /// TRegDataType.
    fn kind(&self) -> i64 {
        match self {
            Data::Str(_) => 1,
            Data::Expand(_) => 2,
            Data::Int(_) => 3,
            Data::Bin(_) => 4,
        }
    }

    /// Its bytes (a string's as an ANSI program reads them: with its NUL).
    fn bytes(&self) -> Vec<u8> {
        match self {
            Data::Str(s) | Data::Expand(s) => {
                let mut b = s.as_bytes().to_vec();
                b.push(0);
                b
            }
            Data::Int(n) => n.to_le_bytes().to_vec(),
            Data::Bin(b) => b.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Key {
    keys: Vec<(String, Key)>,
    values: Vec<(String, Data)>,
}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b) || a.to_lowercase() == b.to_lowercase()
}

fn name_of(s: &str) -> String {
    s.chars().take(MAX_NAME).collect()
}

impl Key {
    fn child(&self, name: &str) -> Option<&Key> {
        self.keys.iter().find(|(n, _)| same(n, name)).map(|(_, k)| k)
    }

    fn child_mut(&mut self, name: &str) -> Option<&mut Key> {
        self.keys.iter_mut().find(|(n, _)| same(n, name)).map(|(_, k)| k)
    }

    fn at(&self, path: &[String]) -> Option<&Key> {
        path.iter().try_fold(self, |k, p| k.child(p))
    }

    fn at_mut(&mut self, path: &[String]) -> Option<&mut Key> {
        let mut k = self;
        for p in path {
            k = k.child_mut(p)?;
        }
        Some(k)
    }

    /// The key at `path`, made (with the keys on the way) if it isn't there.
    fn create(&mut self, path: &[String]) -> Option<&mut Key> {
        let mut k = self;
        for p in path {
            if k.child(p).is_none() {
                if k.keys.len() >= MAX_ITEMS {
                    return None;
                }
                k.keys.push((name_of(p), Key::default()));
            }
            k = k.child_mut(p)?;
        }
        Some(k)
    }

    fn value(&self, name: &str) -> Option<&Data> {
        self.values.iter().find(|(n, _)| same(n, name)).map(|(_, d)| d)
    }

    fn set_value(&mut self, name: &str, data: Data) -> bool {
        if let Some(slot) = self.values.iter_mut().find(|(n, _)| same(n, name)) {
            slot.1 = data;
            return true;
        }
        if self.values.len() >= MAX_ITEMS {
            return false;
        }
        self.values.push((name_of(name), data));
        true
    }

    /// Sub-key names in the order Windows lists them (alphabetical,
    /// ignoring case).
    fn key_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.keys.iter().map(|(n, _)| n.clone()).collect();
        names.sort_by_key(|n| n.to_lowercase());
        names
    }

    /// Another key's sub-keys and values copied in (MoveKey).
    fn merge(&mut self, other: &Key) {
        for (n, d) in &other.values {
            self.set_value(n, d.clone());
        }
        for (n, k) in &other.keys {
            if let Some(dst) = self.create(std::slice::from_ref(n)) {
                dst.merge(k);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The store and its file
// ---------------------------------------------------------------------------

/// Reads the store's text (`None`: none yet) and writes it back: the web
/// runtime installs its own (local storage).
pub type Load = fn() -> Option<String>;
pub type Save = fn(&str);

#[derive(Default)]
struct Store {
    roots: Vec<(u32, Key)>,
    loaded: bool,
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::default());
    static IO: Cell<Option<(Load, Save)>> = const { Cell::new(None) };
    /// The numbers CurrentKey shows for open keys.
    static NEXT_HANDLE: Cell<i64> = const { Cell::new(0x100) };
}

/// Where the store is kept (instead of the user's settings folder): the web
/// runtime's local storage; a test's own file.
pub fn set_io(load: Load, save: Save) {
    // (installed again with the same functions: the store read stays)
    let same = IO.with(Cell::get).is_some_and(|(l, s)| l as usize == load as usize && s as usize == save as usize);
    IO.with(|io| io.set(Some((load, save))));
    if !same {
        STORE.with(|s| *s.borrow_mut() = Store::default());
    }
}

/// The store's file: `RAPIDR_REGISTRY`, else `registry.reg` in RapidR's
/// folder of the user's settings.
#[cfg(not(target_arch = "wasm32"))]
pub fn store_path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Some(p) = std::env::var_os("RAPIDR_REGISTRY").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let dir = if cfg!(target_os = "macos") {
        home?.join("Library").join("Application Support").join("RapidR")
    } else if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?).join("RapidR")
    } else {
        std::env::var_os("XDG_CONFIG_HOME").filter(|p| !p.is_empty()).map(PathBuf::from).or_else(|| home.map(|h| h.join(".config")))?.join("rapidr")
    };
    Some(dir.join("registry.reg"))
}

fn load_text() -> Option<String> {
    if let Some((load, _)) = IO.with(Cell::get) {
        return load();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(store_path()?).ok()
    }
    #[cfg(target_arch = "wasm32")]
    None
}

fn save_text(text: &str) {
    if let Some((_, save)) = IO.with(Cell::get) {
        save(text);
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = store_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // (written whole, then put in place: never half a file)
        let tmp = path.with_extension("reg.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

fn with_store<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        if !s.loaded {
            s.loaded = true;
            if let Some(text) = load_text() {
                s.roots = parse(&text);
            }
        }
        f(&mut s)
    })
}

/// A change to the store: made, then saved.
fn change<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    let (r, text) = with_store(|s| {
        let r = f(s);
        (r, write(&s.roots))
    });
    save_text(&text);
    r
}

impl Store {
    fn root(&self, root: u32) -> Option<&Key> {
        self.roots.iter().find(|(r, _)| *r == root).map(|(_, k)| k)
    }

    fn root_mut(&mut self, root: u32) -> &mut Key {
        if let Some(i) = self.roots.iter().position(|(r, _)| *r == root) {
            return &mut self.roots[i].1;
        }
        self.roots.push((root, Key::default()));
        &mut self.roots.last_mut().unwrap().1
    }
}

// ---------------------------------------------------------------------------
// The `.reg` text
// ---------------------------------------------------------------------------

fn root_name(root: u32) -> &'static str {
    ROOTS.iter().find(|(r, _)| *r == root).map_or("HKEY_CURRENT_USER", |(_, n)| n)
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(",")
}

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().chain(std::iter::once(0)).flat_map(u16::to_le_bytes).collect()
}

fn from_utf16(b: &[u8]) -> String {
    let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

fn write_key(out: &mut String, path: &str, key: &Key) {
    out.push_str(&format!("\n[{path}]\n"));
    for (n, d) in &key.values {
        let name = if n.is_empty() { "@".to_string() } else { quote(n) };
        let data = match d {
            // (a string with a line break: as Regedit writes it, hex(1))
            Data::Str(s) if s.contains(['\r', '\n']) => format!("hex(1):{}", hex(&utf16(s))),
            Data::Str(s) => quote(s),
            Data::Expand(s) => format!("hex(2):{}", hex(&utf16(s))),
            Data::Int(i) => format!("dword:{:08x}", *i as u32),
            Data::Bin(b) => format!("hex:{}", hex(b)),
        };
        out.push_str(&format!("{name}={data}\n"));
    }
    for (n, k) in &key.keys {
        write_key(out, &format!("{path}\\{n}"), k);
    }
}

fn write(roots: &[(u32, Key)]) -> String {
    let mut out = String::from("Windows Registry Editor Version 5.00\n");
    for (r, k) in roots {
        write_key(&mut out, root_name(*r), k);
    }
    out
}

/// A quoted string's text, and what follows it.
fn unquote(s: &str) -> Option<(String, &str)> {
    let mut chars = s.strip_prefix('"')?.char_indices();
    let mut out = String::new();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?.1),
            '"' => return Some((out, &s[i + 2..])),
            c => out.push(c),
        }
    }
    None
}

fn parse_hex(s: &str) -> Vec<u8> {
    s.split(',').filter_map(|h| u8::from_str_radix(h.trim(), 16).ok()).collect()
}

fn parse(text: &str) -> Vec<(u32, Key)> {
    let mut store = Store::default();
    let mut current: Option<(u32, Vec<String>)> = None;
    // (lines ending in `\` go on: Regedit wraps long hex)
    let mut joined: Vec<String> = Vec::new();
    let mut pending = String::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(head) = line.strip_suffix('\\').filter(|_| !line.trim_start().starts_with('[')) {
            pending.push_str(head.trim());
            continue;
        }
        pending.push_str(line.trim());
        joined.push(std::mem::take(&mut pending));
    }
    for line in joined {
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let mut parts = section.split('\\').filter(|p| !p.is_empty());
            let root = parts.next().and_then(|r| ROOTS.iter().find(|(_, n)| n.eq_ignore_ascii_case(r)).map(|(c, _)| *c));
            current = root.map(|r| (r, parts.map(String::from).collect()));
            if let Some((r, p)) = &current {
                store.root_mut(*r).create(p);
            }
            continue;
        }
        let Some((root, path)) = &current else { continue };
        let (name, rest) = if let Some(rest) = line.strip_prefix('@') {
            (String::new(), rest)
        } else {
            match unquote(&line) {
                Some(v) => v,
                None => continue,
            }
        };
        let Some(data) = rest.trim_start().strip_prefix('=') else { continue };
        let data = data.trim();
        let value = if data.starts_with('"') {
            unquote(data).map(|(s, _)| Data::Str(s))
        } else if let Some(h) = data.strip_prefix("dword:") {
            u32::from_str_radix(h.trim(), 16).ok().map(|n| Data::Int(n as i32))
        } else if let Some(h) = data.strip_prefix("hex(1):") {
            Some(Data::Str(from_utf16(&parse_hex(h))))
        } else if let Some(h) = data.strip_prefix("hex(2):") {
            Some(Data::Expand(from_utf16(&parse_hex(h))))
        } else if let Some(h) = data.strip_prefix("hex:") {
            Some(Data::Bin(parse_hex(h)))
        } else {
            // (other kinds: their bytes)
            data.split_once("):").map(|(_, h)| Data::Bin(parse_hex(h)))
        };
        if let (Some(v), Some(k)) = (value, store.root_mut(*root).create(path)) {
            k.set_value(&name, v);
        }
    }
    store.roots
}

// ---------------------------------------------------------------------------
// A QREGISTRY object
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Registry {
    pub root: u32,
    /// The open key's path ("" parts never), empty with no key open.
    path: Vec<String>,
    /// CurrentKey: 0 with no key open.
    handle: i64,
}

impl Default for Registry {
    fn default() -> Self {
        Registry { root: HKEY_CURRENT_USER, path: Vec::new(), handle: 0 }
    }
}

fn flag(b: bool) -> Value {
    v_int(i64::from(b))
}

impl Registry {
    /// A key path from the open key (`\` first: from the root); empty
    /// parts (`"\\Control Panel\\Colors"` without escapes) don't count.
    fn resolve(&self, key: &str) -> Vec<String> {
        let mut path = if key.starts_with('\\') { Vec::new() } else { self.path.clone() };
        path.extend(key.split('\\').filter(|p| !p.is_empty()).map(String::from));
        path
    }

    /// Reads the open key (the root with none open).
    fn read<R>(&self, f: impl FnOnce(&Key) -> R) -> Option<R> {
        with_store(|s| s.root(self.root).and_then(|r| r.at(&self.path)).map(f))
    }

    /// Changes the open key (made if it was deleted meanwhile).
    fn write<R>(&self, f: impl FnOnce(&mut Key) -> R) -> Option<R> {
        let root = self.root;
        let path = self.path.clone();
        change(|s| s.root_mut(root).create(&path).map(f))
    }

    fn value(&self, name: &str) -> Option<Data> {
        self.read(|k| k.value(name).cloned()).flatten()
    }

    fn close(&mut self) {
        self.path.clear();
        self.handle = 0;
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "rootkey" => v_int(i64::from(self.root)),
            "currentkey" => v_int(self.handle),
            "currentpath" => v_str(&self.path.join("\\")),
            "hassubkeys" => flag(self.read(|k| !k.keys.is_empty()).unwrap_or(false)),
            "keyitemcount" => v_int(self.read(|k| k.keys.len() as i64).unwrap_or(0)),
            "valueitemcount" => v_int(self.read(|k| k.values.len() as i64).unwrap_or(0)),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            // (a new root closes the open key, as TRegistry's)
            "rootkey" => {
                self.root = (val.to_i64() & 0xFFFF_FFFF) as u32;
                self.close();
                true
            }
            _ => false,
        }
    }

    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let s = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
        let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
        let root = self.root;
        Some(match method {
            "openkey" => {
                let path = self.resolve(&s(0));
                let ok = if n(1) != 0 {
                    change(|st| st.root_mut(root).create(&path).is_some())
                } else {
                    with_store(|st| st.root(root).and_then(|r| r.at(&path)).is_some() || path.is_empty())
                };
                if ok {
                    self.path = path;
                    self.handle = NEXT_HANDLE.with(|h| {
                        h.set(h.get() + 4);
                        h.get()
                    });
                }
                flag(ok)
            }
            "closekey" => {
                self.close();
                Value::Null
            }
            "createkey" => {
                let path = self.resolve(&s(0));
                flag(change(|st| st.root_mut(root).create(&path).is_some()))
            }
            "keyexists" => {
                let path = self.resolve(&s(0));
                flag(with_store(|st| st.root(root).and_then(|r| r.at(&path)).is_some()))
            }
            "deletekey" => {
                let path = self.resolve(&s(0));
                let Some((last, parent)) = path.split_last() else { return Some(flag(false)) };
                let ok = change(|st| {
                    let Some(p) = st.root_mut(root).at_mut(parent) else { return false };
                    let before = p.keys.len();
                    p.keys.retain(|(k, _)| !same(k, last));
                    p.keys.len() != before
                });
                // (the open key went with it)
                if ok && self.path.len() >= path.len() && self.path.iter().zip(&path).all(|(a, b)| same(a, b)) {
                    self.close();
                }
                flag(ok)
            }
            "valueexists" => flag(self.value(&s(0)).is_some()),
            "deletevalue" => {
                let name = s(0);
                flag(self.write(|k| {
                    let before = k.values.len();
                    k.values.retain(|(v, _)| !same(v, &name));
                    k.values.len() != before
                }) == Some(true))
            }
            "renamevalue" => {
                let (old, new) = (s(0), s(1));
                self.write(|k| {
                    if k.value(&new).is_none() || same(&old, &new) {
                        if let Some(slot) = k.values.iter_mut().find(|(v, _)| same(v, &old)) {
                            slot.0 = name_of(&new);
                        }
                    }
                });
                Value::Null
            }
            "keyitem" => v_str(&self.read(|k| k.key_names().get(n(0).max(0) as usize).cloned()).flatten().unwrap_or_default()),
            "valueitem" => v_str(&self.read(|k| k.values.get(n(0).max(0) as usize).map(|(v, _)| v.clone())).flatten().unwrap_or_default()),
            "movekey" => {
                let (from, to) = (self.resolve(&s(0)), self.resolve(&s(1)));
                let delete = n(2) != 0;
                let ok = change(|st| {
                    let r = st.root_mut(root);
                    let Some(src) = r.at(&from).cloned() else { return false };
                    if from.is_empty() || to.starts_with(&from) {
                        return false;
                    }
                    let Some(dst) = r.create(&to) else { return false };
                    dst.merge(&src);
                    if delete {
                        if let Some((last, parent)) = from.split_last() {
                            if let Some(p) = r.at_mut(parent) {
                                p.keys.retain(|(k, _)| !same(k, last));
                            }
                        }
                    }
                    true
                });
                flag(ok)
            }
            "getdatasize" => v_int(self.value(&s(0)).map_or(-1, |d| d.bytes().len() as i64)),
            "getdatatype" => v_int(self.value(&s(0)).map_or(0, |d| d.kind())),
            "readstring" => v_str(&match self.value(&s(0)) {
                Some(Data::Str(t) | Data::Expand(t)) => t,
                _ => String::new(),
            }),
            "readinteger" => v_int(match self.value(&s(0)) {
                Some(Data::Int(i)) => i64::from(i),
                _ => 0,
            }),
            "readfloat" => Value::Double(match self.value(&s(0)) {
                Some(Data::Bin(b)) if b.len() == 8 => f64::from_le_bytes(b[..8].try_into().unwrap_or_default()),
                _ => 0.0,
            }),
            // (RapidQ's bug: Index -1 is the first byte)
            "readbinary" => v_int(self.value(&s(0)).and_then(|d| usize::try_from(n(1) + 1).ok().and_then(|i| d.bytes().get(i).copied())).map_or(0, i64::from)),
            "writestring" => {
                let (name, text): (String, String) = (s(0), s(1).chars().take(MAX_DATA).collect());
                self.write(|k| k.set_value(&name, Data::Str(text)));
                Value::Null
            }
            "writeinteger" => {
                let (name, v) = (s(0), n(1) as i32);
                self.write(|k| k.set_value(&name, Data::Int(v)));
                Value::Null
            }
            "writefloat" => {
                let (name, v) = (s(0), args.get(1).map_or(0.0, Value::to_f64));
                self.write(|k| k.set_value(&name, Data::Bin(v.to_le_bytes().to_vec())));
                Value::Null
            }
            "writebinary" => {
                let name = s(0);
                let size = usize::try_from(n(2)).unwrap_or(0).min(MAX_DATA);
                let bytes: Vec<u8> = match args.get(1) {
                    Some(Value::Array(a)) => a.borrow().data.iter().take(size).map(|v| v.to_i64() as u8).collect(),
                    Some(v) => v.to_string_val().bytes().take(size).collect(),
                    None => Vec::new(),
                };
                self.write(|k| k.set_value(&name, Data::Bin(bytes)));
                Value::Null
            }
            // (another computer's registry: not reachable)
            "registryconnect" => flag(false),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        static TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
    }
    fn load() -> Option<String> {
        TEXT.with(|t| t.borrow().clone())
    }
    fn save(s: &str) {
        TEXT.with(|t| *t.borrow_mut() = Some(s.to_string()));
    }

    #[test]
    fn rapidq_manual_example() {
        set_io(load, save);
        let mut r = Registry::default();
        assert_eq!(r.call("openkey", &[v_str("TestToDelete"), v_int(1)]), Some(v_int(1)));
        r.call("writeinteger", &[v_str("MyInteger"), v_int(1234567890)]);
        r.call("writestring", &[v_str("Mystring"), v_str("1234567890")]);
        let arr = crate::BasicArray { bounds: vec![(0, 10)], data: (1..=11).map(|i| v_int(i % 10)).collect() };
        r.call("writebinary", &[v_str("MyBinary"), Value::Array(std::rc::Rc::new(RefCell::new(arr))), v_int(10)]);
        r.call("closekey", &[]);
        assert_eq!(r.get("currentkey"), Some(v_int(0)));
        r.call("openkey", &[v_str("TestToDelete"), v_int(1)]);
        assert_eq!(r.call("readinteger", &[v_str("MyInteger")]), Some(v_int(1234567890)));
        assert_eq!(r.call("readstring", &[v_str("mystring")]), Some(v_str("1234567890")), "names ignore case");
        // (the bug: index -1 is the first byte)
        let bytes: Vec<i64> = (-1..=8).map(|i| r.call("readbinary", &[v_str("MyBinary"), v_int(i)]).unwrap().to_i64()).collect();
        assert_eq!(bytes, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
        assert_eq!(r.call("getdatatype", &[v_str("MyBinary")]), Some(v_int(4)));
        assert_eq!(r.call("getdatasize", &[v_str("Mystring")]), Some(v_int(11)));
        assert_eq!(r.call("getdatasize", &[v_str("Nothing")]), Some(v_int(-1)));
        assert_eq!(r.get("currentpath"), Some(v_str("TestToDelete")));
        assert_eq!(r.get("valueitemcount"), Some(v_int(3)));
        r.call("closekey", &[]);
        // it persisted: a fresh store reads the text back
        let text = load().unwrap();
        assert!(text.contains("[HKEY_CURRENT_USER\\TestToDelete]") && text.contains("\"MyInteger\"=dword:499602d2"));
        set_io(load, save);
        let mut r2 = Registry::default();
        assert_eq!(r2.call("keyexists", &[v_str("\\TestToDelete")]), Some(v_int(1)));
        r2.call("openkey", &[v_str("TestToDelete"), v_int(0)]);
        assert_eq!(r2.call("readinteger", &[v_str("MyInteger")]), Some(v_int(1234567890)));
        r2.call("closekey", &[]);
        assert_eq!(r2.call("deletekey", &[v_str("TestToDelete")]), Some(v_int(1)));
        assert_eq!(r2.call("keyexists", &[v_str("TestToDelete")]), Some(v_int(0)));
    }

    #[test]
    fn paths_roots_and_moves() {
        set_io(load, save);
        let mut r = Registry::default();
        r.set("rootkey", &v_int(-2147483648)); // &H80000000 as a LONG
        assert_eq!(r.root, HKEY_CLASSES_ROOT);
        r.call("openkey", &[v_str("\\.bc"), v_int(1)]);
        r.call("writestring", &[v_str(""), v_str("BCFile")]);
        r.call("openkey", &[v_str("\\BCFile"), v_int(1)]);
        r.call("openkey", &[v_str("shell\\open\\command"), v_int(1)]);
        assert_eq!(r.get("currentpath"), Some(v_str("BCFile\\shell\\open\\command")));
        r.call("writestring", &[v_str(""), v_str("c:\\x \"%1\"")]);
        // doubled backslashes (no escapes in RapidQ strings) are one
        assert_eq!(r.call("openkey", &[v_str("\\\\BCFile\\\\shell"), v_int(0)]), Some(v_int(1)));
        assert_eq!(r.get("keyitemcount"), Some(v_int(1)));
        assert_eq!(r.call("keyitem", &[v_int(0)]), Some(v_str("open")));
        assert_eq!(r.call("movekey", &[v_str("\\BCFile"), v_str("\\Moved"), v_int(1)]), Some(v_int(1)));
        assert_eq!(r.call("keyexists", &[v_str("\\Moved\\shell\\open\\command")]), Some(v_int(1)));
        assert_eq!(r.call("keyexists", &[v_str("\\BCFile")]), Some(v_int(0)));
        // the text round-trips (quotes, backslashes, default values)
        let text = load().unwrap();
        assert_eq!(write(&parse(&text)), text);
        assert!(text.contains("@=\"c:\\\\x \\\"%1\\\"\""));
    }
}
