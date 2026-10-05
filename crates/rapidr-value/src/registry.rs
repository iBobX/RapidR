//! QREGISTRY (RapidQ manual, Appendix A): keys holding sub-keys and named
//! values, under a root (HKEY_CURRENT_USER by default).
//!
//! On Windows they are Windows' own registry, as RapidQ's were — in native
//! and interpreted builds alike (both run on this file). `RAPIDR_REGISTRY`
//! naming a file puts them in that file instead, on Windows too, so test
//! runs never touch the machine's registry. Elsewhere RapidR keeps them in a
//! per-user store — a text file in the Windows `.reg` format (Regedit's), in
//! the user's settings folder (on the web, the page's local storage) — so a
//! program's settings persist between runs the way RapidQ's did in the
//! registry; nothing outside the store is read or changed.
//!
//! The API is Delphi's TRegistry, which QREGISTRY's names follow: OpenKey /
//! KeyExists / CreateKey / DeleteKey take a path relative to the open key
//! (`\` first: from the root); names are case-insensitive and keep their
//! case; GetDataType is TRegDataType's (0 unknown, 1 string, 2 expandable
//! string, 3 integer, 4 binary); GetDataSize is -1 for no such value; a
//! float is stored as its 8 bytes. The functions answer 1 or 0 (the
//! manual's KeyExists: "Returns 0 or 1"). ReadBinary(Name, Index) keeps
//! RapidQ's bug — the first byte is Index -1 — which its programs count on.
//!
//! What a call answers is worked out once, here, from a few operations on
//! keys ([`Keys`]) that the store and Windows' registry each do, so both
//! answer alike. A refusal (no such key, access denied — HKEY_LOCAL_MACHINE
//! without elevation) is TRegistry's 0 / nothing, never an error. The store
//! keeps limits of its own (`MAX_*`); Windows' registry keeps its own.

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

/// The store's limits: the largest value kept (the registry's own limit is
/// about this), and the longest key or value name.
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
    /// Another kind (REG_MULTI_SZ, REG_QWORD, REG_NONE, …): TRegistry's
    /// rdUnknown; its bytes as they are.
    Other(u32, Vec<u8>),
}

impl Data {
    /// TRegDataType.
    fn kind(&self) -> i64 {
        match self {
            Data::Str(_) => 1,
            Data::Expand(_) => 2,
            Data::Int(_) => 3,
            Data::Bin(_) => 4,
            Data::Other(..) => 0,
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
            Data::Bin(b) | Data::Other(_, b) => b.clone(),
        }
    }

    /// A value as the registry keeps it: its kind (REG_SZ = 1, …) and bytes
    /// (strings in UTF-16).
    fn from_raw(kind: u32, bytes: Vec<u8>) -> Data {
        match kind {
            1 => Data::Str(from_utf16(&bytes)),
            2 => Data::Expand(from_utf16(&bytes)),
            3 => Data::Bin(bytes),
            4 if bytes.len() == 4 => Data::Int(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
            _ => Data::Other(kind, bytes),
        }
    }

    /// [`Data::from_raw`]'s other way.
    #[cfg_attr(not(windows), allow(dead_code))]
    fn to_raw(&self) -> (u32, Vec<u8>) {
        match self {
            Data::Str(s) => (1, utf16(s)),
            Data::Expand(s) => (2, utf16(s)),
            Data::Bin(b) => (3, b.clone()),
            Data::Int(i) => (4, i.to_le_bytes().to_vec()),
            Data::Other(k, b) => (*k, b.clone()),
        }
    }

    /// Within the store's limit.
    fn limited(self) -> Data {
        let cut = |s: String| if s.len() > MAX_DATA { s.chars().take(MAX_DATA).collect() } else { s };
        match self {
            Data::Str(s) => Data::Str(cut(s)),
            Data::Expand(s) => Data::Expand(cut(s)),
            Data::Bin(mut b) => {
                b.truncate(MAX_DATA);
                Data::Bin(b)
            }
            Data::Other(k, mut b) => {
                b.truncate(MAX_DATA);
                Data::Other(k, b)
            }
            d @ Data::Int(_) => d,
        }
    }
}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b) || a.to_lowercase() == b.to_lowercase()
}

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().chain(std::iter::once(0)).flat_map(u16::to_le_bytes).collect()
}

fn from_utf16(b: &[u8]) -> String {
    let units: Vec<u16> = b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
    let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

// ---------------------------------------------------------------------------
// Where the keys are
// ---------------------------------------------------------------------------

/// The keys under a root, by path (its parts, none empty; none: the root
/// itself) — the per-user store, or Windows' registry. Names ignore case.
/// A refusal (no such key, access denied) is `false` / nothing.
trait Keys {
    /// The key is there (a root always is).
    fn exists(&self, root: u32, path: &[String]) -> bool;
    /// The key, made (with the keys on the way) if it isn't there.
    fn create(&self, root: u32, path: &[String]) -> bool;
    /// The key and everything under it (never a root).
    fn delete(&self, root: u32, path: &[String]) -> bool;
    /// Its sub-keys' names, in the order Windows lists them (alphabetical,
    /// ignoring case).
    fn key_names(&self, root: u32, path: &[String]) -> Vec<String>;
    /// Its values, in the order they're listed.
    fn values(&self, root: u32, path: &[String]) -> Vec<(String, Data)>;
    fn key_count(&self, root: u32, path: &[String]) -> usize;
    fn key_item(&self, root: u32, path: &[String], index: usize) -> Option<String>;
    fn value_count(&self, root: u32, path: &[String]) -> usize;
    fn value_item(&self, root: u32, path: &[String], index: usize) -> Option<String>;
    fn value(&self, root: u32, path: &[String], name: &str) -> Option<Data>;
    /// Sets a value, the key made if it isn't there (deleted meanwhile).
    fn set_value(&self, root: u32, path: &[String], name: &str, data: Data) -> bool;
    fn delete_value(&self, root: u32, path: &[String], name: &str) -> bool;
    /// A call's changes kept (the store's text written once, at its end).
    fn flush(&self) {}
}

/// Windows' registry on Windows — unless `RAPIDR_REGISTRY` names a file, or
/// a store was installed ([`set_io`]: a test's) — the per-user store
/// elsewhere.
fn keys() -> &'static dyn Keys {
    #[cfg(windows)]
    if IO.with(Cell::get).is_none() && std::env::var_os("RAPIDR_REGISTRY").is_none_or(|p| p.is_empty()) {
        return &win::Registry;
    }
    &UserStore
}

/// A key and everything under it, read (what MoveKey copies).
fn tree(keys: &dyn Keys, root: u32, path: &[String]) -> Option<Key> {
    if !keys.exists(root, path) {
        return None;
    }
    let mut key = Key { keys: Vec::new(), values: keys.values(root, path) };
    for name in keys.key_names(root, path) {
        let sub = [path, std::slice::from_ref(&name)].concat();
        if let Some(k) = tree(keys, root, &sub) {
            key.keys.push((name, k));
        }
    }
    Some(key)
}

/// A key's sub-keys and values put under `path` (MoveKey: added to what's
/// there).
fn copy(keys: &dyn Keys, root: u32, path: &[String], key: &Key) {
    for (name, data) in &key.values {
        keys.set_value(root, path, name, data.clone());
    }
    for (name, k) in &key.keys {
        let sub = [path, std::slice::from_ref(name)].concat();
        if keys.create(root, &sub) {
            copy(keys, root, &sub, k);
        }
    }
}

// ---------------------------------------------------------------------------
// The per-user store
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq)]
struct Key {
    keys: Vec<(String, Key)>,
    values: Vec<(String, Data)>,
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
        let data = data.limited();
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
}

/// Reads the store's text (`None`: none yet) and writes it back: the web
/// runtime installs its own (local storage).
pub type Load = fn() -> Option<String>;
pub type Save = fn(&str);

#[derive(Default)]
struct Store {
    roots: Vec<(u32, Key)>,
    loaded: bool,
    /// Changed since its text was last written.
    changed: bool,
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::default());
    static IO: Cell<Option<(Load, Save)>> = const { Cell::new(None) };
    /// The numbers CurrentKey shows for open keys.
    static NEXT_HANDLE: Cell<i64> = const { Cell::new(0x100) };
}

/// Where the store is kept (instead of the user's settings folder): the web
/// runtime's local storage; a test's own (on Windows too: then it, not the
/// registry).
pub fn set_io(load: Load, save: Save) {
    // (installed again with the same functions: the store read stays)
    let same = IO.with(Cell::get).is_some_and(|(l, s)| l as usize == load as usize && s as usize == save as usize);
    IO.with(|io| io.set(Some((load, save))));
    if !same {
        STORE.with(|s| *s.borrow_mut() = Store::default());
    }
}

/// The store's file: `RAPIDR_REGISTRY`, else `registry.reg` in RapidR's
/// folder of the user's settings (none on Windows: the registry itself).
#[cfg(not(target_arch = "wasm32"))]
pub fn store_path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Some(p) = std::env::var_os("RAPIDR_REGISTRY").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    if cfg!(windows) {
        return None;
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let dir = if cfg!(target_os = "macos") {
        home?.join("Library").join("Application Support").join("RapidR")
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

/// A change to the store (its text written when the call is done).
fn change<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    with_store(|s| {
        s.changed = true;
        f(s)
    })
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

/// The per-user store as [`Keys`].
struct UserStore;

impl UserStore {
    fn read<R>(&self, root: u32, path: &[String], f: impl FnOnce(&Key) -> R) -> Option<R> {
        with_store(|s| s.root(root).and_then(|r| r.at(path)).map(f))
    }
}

impl Keys for UserStore {
    fn exists(&self, root: u32, path: &[String]) -> bool {
        path.is_empty() || self.read(root, path, |_| ()).is_some()
    }

    fn create(&self, root: u32, path: &[String]) -> bool {
        change(|s| s.root_mut(root).create(path).is_some())
    }

    fn delete(&self, root: u32, path: &[String]) -> bool {
        let Some((last, parent)) = path.split_last() else { return false };
        change(|s| {
            let Some(p) = s.root_mut(root).at_mut(parent) else { return false };
            let before = p.keys.len();
            p.keys.retain(|(k, _)| !same(k, last));
            p.keys.len() != before
        })
    }

    fn key_names(&self, root: u32, path: &[String]) -> Vec<String> {
        self.read(root, path, Key::key_names).unwrap_or_default()
    }

    fn values(&self, root: u32, path: &[String]) -> Vec<(String, Data)> {
        self.read(root, path, |k| k.values.clone()).unwrap_or_default()
    }

    fn key_count(&self, root: u32, path: &[String]) -> usize {
        self.read(root, path, |k| k.keys.len()).unwrap_or(0)
    }

    fn key_item(&self, root: u32, path: &[String], index: usize) -> Option<String> {
        self.read(root, path, |k| k.key_names().get(index).cloned()).flatten()
    }

    fn value_count(&self, root: u32, path: &[String]) -> usize {
        self.read(root, path, |k| k.values.len()).unwrap_or(0)
    }

    fn value_item(&self, root: u32, path: &[String], index: usize) -> Option<String> {
        self.read(root, path, |k| k.values.get(index).map(|(n, _)| n.clone())).flatten()
    }

    fn value(&self, root: u32, path: &[String], name: &str) -> Option<Data> {
        self.read(root, path, |k| k.value(name).cloned()).flatten()
    }

    fn set_value(&self, root: u32, path: &[String], name: &str, data: Data) -> bool {
        change(|s| s.root_mut(root).create(path).is_some_and(|k| k.set_value(name, data)))
    }

    fn delete_value(&self, root: u32, path: &[String], name: &str) -> bool {
        change(|s| {
            s.root_mut(root).at_mut(path).is_some_and(|k| {
                let before = k.values.len();
                k.values.retain(|(v, _)| !same(v, name));
                k.values.len() != before
            })
        })
    }

    fn flush(&self) {
        let text = STORE.with(|s| {
            let mut s = s.borrow_mut();
            std::mem::take(&mut s.changed).then(|| write(&s.roots))
        });
        if let Some(text) = text {
            save_text(&text);
        }
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
            Data::Other(k, b) => format!("hex({k:x}):{}", hex(b)),
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
        } else if let Some(h) = data.strip_prefix("hex:") {
            Some(Data::Bin(parse_hex(h)))
        } else {
            // (hex(kind): the other kinds, REG_EXPAND_SZ = hex(2) among them)
            data.strip_prefix("hex(").and_then(|d| d.split_once("):")).and_then(|(k, h)| Some(Data::from_raw(u32::from_str_radix(k.trim(), 16).ok()?, parse_hex(h))))
        };
        if let (Some(v), Some(k)) = (value, store.root_mut(*root).create(path)) {
            k.set_value(&name, v);
        }
    }
    store.roots
}

// ---------------------------------------------------------------------------
// Windows' registry
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod win {
    //! The keys in Windows' registry (where RapidQ's QREGISTRY kept them).
    //! Each call opens the key it needs and closes it again: an open key is
    //! its path, as in the store. Keys open for reading unless the call
    //! changes something, so OpenKey / KeyExists / the Read… calls work on
    //! keys a program may only read (HKEY_LOCAL_MACHINE without elevation),
    //! where changes are refused.

    use super::{Data, Keys, HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS};
    use std::cell::RefCell;
    use windows_registry::{Key, Type, Value};

    pub(super) struct Registry;

    /// A root's key — Windows' five (HKEY_PERFORMANCE_DATA isn't keys,
    /// HKEY_DYN_DATA was Windows 9x's); no other number is one, never a
    /// handle the program has open.
    fn root_key(root: u32) -> Option<&'static Key> {
        Some(match root {
            HKEY_CLASSES_ROOT => windows_registry::CLASSES_ROOT,
            HKEY_CURRENT_USER => windows_registry::CURRENT_USER,
            HKEY_LOCAL_MACHINE => windows_registry::LOCAL_MACHINE,
            HKEY_USERS => windows_registry::USERS,
            HKEY_CURRENT_CONFIG => windows_registry::CURRENT_CONFIG,
            _ => return None,
        })
    }

    /// The key, open for reading.
    fn open(root: u32, path: &[String]) -> Option<Key> {
        root_key(root)?.open(path.join("\\")).ok()
    }

    fn data(v: &Value) -> Data {
        Data::from_raw(v.ty().into(), v.to_vec())
    }

    struct Listed {
        root: u32,
        path: Vec<String>,
        values: bool,
        names: Vec<String>,
    }

    thread_local! {
        /// The names a key listed last. KeyItem / ValueItem go through a
        /// key's list by index; Windows lists from the start each time, so
        /// a walk through thousands of keys (HKEY_CLASSES_ROOT) reads the
        /// list once: item 0 (a walk's start), the counts and any change
        /// through QREGISTRY list it afresh.
        static LISTED: RefCell<Option<Listed>> = const { RefCell::new(None) };
    }

    fn changed() {
        LISTED.with(|l| *l.borrow_mut() = None);
    }

    fn names(root: u32, path: &[String], values: bool) -> Vec<String> {
        let names: Vec<String> = match open(root, path) {
            Some(key) if values => key.values().map(|it| it.map(|(n, _)| n).collect()).unwrap_or_default(),
            Some(key) => key.keys().map(|it| it.collect()).unwrap_or_default(),
            None => Vec::new(),
        };
        LISTED.with(|l| *l.borrow_mut() = Some(Listed { root, path: path.to_vec(), values, names: names.clone() }));
        names
    }

    fn item(root: u32, path: &[String], values: bool, index: usize) -> Option<String> {
        if index > 0 {
            let listed = LISTED.with(|l| l.borrow().as_ref().filter(|l| l.root == root && l.values == values && l.path == path).map(|l| l.names.get(index).cloned()));
            if let Some(name) = listed {
                return name;
            }
        }
        names(root, path, values).into_iter().nth(index)
    }

    impl Keys for Registry {
        fn exists(&self, root: u32, path: &[String]) -> bool {
            open(root, path).is_some()
        }

        fn create(&self, root: u32, path: &[String]) -> bool {
            changed();
            // (one already there opens for reading: OpenKey(…, 1) works on
            // keys the program can't change)
            root_key(root).is_some_and(|r| r.options().read().create().open(path.join("\\")).is_ok())
        }

        fn delete(&self, root: u32, path: &[String]) -> bool {
            changed();
            // (never a root: RegDeleteTree would empty it)
            !path.is_empty() && root_key(root).is_some_and(|r| r.remove_tree(path.join("\\")).is_ok())
        }

        fn key_names(&self, root: u32, path: &[String]) -> Vec<String> {
            names(root, path, false)
        }

        fn values(&self, root: u32, path: &[String]) -> Vec<(String, Data)> {
            open(root, path).and_then(|k| Some(k.values().ok()?.map(|(n, v)| (n, data(&v))).collect())).unwrap_or_default()
        }

        fn key_count(&self, root: u32, path: &[String]) -> usize {
            names(root, path, false).len()
        }

        fn key_item(&self, root: u32, path: &[String], index: usize) -> Option<String> {
            item(root, path, false, index)
        }

        fn value_count(&self, root: u32, path: &[String]) -> usize {
            names(root, path, true).len()
        }

        fn value_item(&self, root: u32, path: &[String], index: usize) -> Option<String> {
            item(root, path, true, index)
        }

        fn value(&self, root: u32, path: &[String], name: &str) -> Option<Data> {
            open(root, path)?.get_value(name).ok().map(|v| data(&v))
        }

        fn set_value(&self, root: u32, path: &[String], name: &str, data: Data) -> bool {
            changed();
            let (kind, bytes) = data.to_raw();
            let Some(key) = root_key(root).and_then(|r| r.options().write().create().open(path.join("\\")).ok()) else { return false };
            // (the kind as its number: the bytes go as they are — a value
            // MoveKey copies keeps its kind even if it isn't well formed)
            key.set_bytes(name, Type::Other(kind), &bytes).is_ok()
        }

        fn delete_value(&self, root: u32, path: &[String], name: &str) -> bool {
            changed();
            root_key(root).and_then(|r| r.options().write().open(path.join("\\")).ok()).is_some_and(|k| k.remove_value(name).is_ok())
        }
    }
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

    fn close(&mut self) {
        self.path.clear();
        self.handle = 0;
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        let keys = keys();
        Some(match prop {
            "rootkey" => v_int(i64::from(self.root)),
            "currentkey" => v_int(self.handle),
            "currentpath" => v_str(&self.path.join("\\")),
            "hassubkeys" => flag(keys.key_count(self.root, &self.path) > 0),
            "keyitemcount" => v_int(keys.key_count(self.root, &self.path) as i64),
            "valueitemcount" => v_int(keys.value_count(self.root, &self.path) as i64),
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
        let keys = keys();
        let answer = self.answer(keys, method, args);
        keys.flush();
        answer
    }

    fn answer(&mut self, keys: &dyn Keys, method: &str, args: &[Value]) -> Option<Value> {
        let s = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
        let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
        let (root, open) = (self.root, self.path.clone());
        let value = |name: &str| keys.value(root, &open, name);
        let write = |name: &str, data: Data| {
            keys.set_value(root, &open, name, data);
            Value::Null
        };
        Some(match method {
            "openkey" => {
                let path = self.resolve(&s(0));
                let ok = if n(1) != 0 { keys.create(root, &path) } else { keys.exists(root, &path) };
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
            "createkey" => flag(keys.create(root, &self.resolve(&s(0)))),
            "keyexists" => flag(keys.exists(root, &self.resolve(&s(0)))),
            "deletekey" => {
                let path = self.resolve(&s(0));
                // (never a root)
                let ok = !path.is_empty() && keys.delete(root, &path);
                // (the open key went with it)
                if ok && self.path.len() >= path.len() && self.path.iter().zip(&path).all(|(a, b)| same(a, b)) {
                    self.close();
                }
                flag(ok)
            }
            "valueexists" => flag(value(&s(0)).is_some()),
            "deletevalue" => flag(keys.delete_value(root, &open, &s(0))),
            // (TRegistry's: the data under the new name, unless that's
            // another value's — then nothing)
            "renamevalue" => {
                let (old, new) = (s(0), s(1));
                if same(&old, &new) || value(&new).is_none() {
                    if let Some(data) = value(&old) {
                        keys.delete_value(root, &open, &old);
                        keys.set_value(root, &open, &new, data);
                    }
                }
                Value::Null
            }
            "keyitem" => v_str(&keys.key_item(root, &open, n(0).max(0) as usize).unwrap_or_default()),
            "valueitem" => v_str(&keys.value_item(root, &open, n(0).max(0) as usize).unwrap_or_default()),
            "movekey" => {
                let (from, to) = (self.resolve(&s(0)), self.resolve(&s(1)));
                // (not a root, nor into itself)
                let into_itself = to.len() >= from.len() && from.iter().zip(&to).all(|(a, b)| same(a, b));
                let ok = !from.is_empty()
                    && !into_itself
                    && tree(keys, root, &from).is_some_and(|src| {
                        keys.create(root, &to) && {
                            copy(keys, root, &to, &src);
                            if n(2) != 0 {
                                keys.delete(root, &from);
                            }
                            true
                        }
                    });
                flag(ok)
            }
            "getdatasize" => v_int(value(&s(0)).map_or(-1, |d| d.bytes().len() as i64)),
            "getdatatype" => v_int(value(&s(0)).map_or(0, |d| d.kind())),
            "readstring" => v_str(&match value(&s(0)) {
                Some(Data::Str(t) | Data::Expand(t)) => t,
                _ => String::new(),
            }),
            "readinteger" => v_int(match value(&s(0)) {
                Some(Data::Int(i)) => i64::from(i),
                _ => 0,
            }),
            "readfloat" => Value::Double(match value(&s(0)) {
                Some(Data::Bin(b)) if b.len() == 8 => f64::from_le_bytes(b[..8].try_into().unwrap_or_default()),
                _ => 0.0,
            }),
            // (RapidQ's bug: Index -1 is the first byte)
            "readbinary" => v_int(value(&s(0)).and_then(|d| usize::try_from(n(1) + 1).ok().and_then(|i| d.bytes().get(i).copied())).map_or(0, i64::from)),
            "writestring" => write(&s(0), Data::Str(s(1))),
            "writeinteger" => write(&s(0), Data::Int(n(1) as i32)),
            "writefloat" => write(&s(0), Data::Bin(args.get(1).map_or(0.0, Value::to_f64).to_le_bytes().to_vec())),
            "writebinary" => {
                let size = usize::try_from(n(2)).unwrap_or(0);
                let bytes: Vec<u8> = match args.get(1) {
                    Some(Value::Array(a)) => a.borrow().data.iter().take(size).map(|v| v.to_i64() as u8).collect(),
                    Some(v) => v.to_string_val().bytes().take(size).collect(),
                    None => Vec::new(),
                };
                write(&s(0), Data::Bin(bytes))
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

    fn call(r: &mut Registry, method: &str, args: &[Value]) -> Value {
        r.call(method, args).unwrap_or_else(|| panic!("no {method}"))
    }
    fn int(r: &mut Registry, method: &str, args: &[Value]) -> i64 {
        call(r, method, args).to_i64()
    }
    fn text(r: &mut Registry, method: &str, args: &[Value]) -> String {
        call(r, method, args).to_string_val()
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

    /// What every place the keys can be answers alike, under `base` (a key
    /// path from HKEY_CURRENT_USER's root, made and deleted here).
    fn shared_semantics(base: &str) {
        let mut r = Registry::default();
        let at = |p: &str| v_str(&format!("{base}\\{p}"));
        assert_eq!(int(&mut r, "deletekey", &[v_str(base)]), 0, "{base} was left over");
        // a root is always there, and is never deleted
        assert_eq!(int(&mut r, "keyexists", &[v_str("")]), 1);
        assert_eq!(int(&mut r, "keyexists", &[v_str("\\")]), 1);
        assert_eq!(int(&mut r, "openkey", &[at("Nope"), v_int(0)]), 0);
        assert_eq!(r.get("currentkey"), Some(v_int(0)));

        // every kind of data
        assert_eq!(int(&mut r, "openkey", &[at("Kinds"), v_int(1)]), 1);
        assert_ne!(r.get("currentkey"), Some(v_int(0)));
        call(&mut r, "writestring", &[v_str("Str"), v_str("héllo, wörld")]);
        call(&mut r, "writestring", &[v_str(""), v_str("the default")]);
        call(&mut r, "writestring", &[v_str("Lines"), v_str("one\r\ntwo")]);
        call(&mut r, "writestring", &[v_str("Empty"), v_str("")]);
        call(&mut r, "writeinteger", &[v_str("Int"), v_int(-5)]);
        call(&mut r, "writefloat", &[v_str("Float"), Value::Double(-2.5)]);
        let arr = crate::BasicArray { bounds: vec![(0, 3)], data: [7, 0, 255, 9].map(v_int).to_vec() };
        call(&mut r, "writebinary", &[v_str("Bin"), Value::Array(std::rc::Rc::new(RefCell::new(arr))), v_int(3)]);
        call(&mut r, "writebinary", &[v_str("FromText"), v_str("abc"), v_int(99)]);
        let names: Vec<String> = (0..9).map(|i| text(&mut r, "valueitem", &[v_int(i)])).collect();
        assert_eq!(names, ["Str", "", "Lines", "Empty", "Int", "Float", "Bin", "FromText", ""], "in the order written");
        assert_eq!(r.get("valueitemcount"), Some(v_int(8)));
        let kinds: Vec<i64> = ["str", "", "lines", "empty", "int", "float", "bin", "fromtext", "nope"].iter().map(|n| int(&mut r, "getdatatype", &[v_str(n)])).collect();
        assert_eq!(kinds, [1, 1, 1, 1, 3, 4, 4, 4, 0]);
        let sizes: Vec<i64> = ["Str", "", "Lines", "Empty", "Int", "Float", "Bin", "FromText", "Nope"].iter().map(|n| int(&mut r, "getdatasize", &[v_str(n)])).collect();
        assert_eq!(sizes, [15, 12, 9, 1, 4, 8, 3, 3, -1]);
        assert_eq!(text(&mut r, "readstring", &[v_str("STR")]), "héllo, wörld");
        assert_eq!(text(&mut r, "readstring", &[v_str("")]), "the default");
        assert_eq!(text(&mut r, "readstring", &[v_str("Lines")]), "one\r\ntwo");
        assert_eq!(text(&mut r, "readstring", &[v_str("Int")]), "", "not a string");
        assert_eq!(int(&mut r, "readinteger", &[v_str("Int")]), -5);
        assert_eq!(int(&mut r, "readinteger", &[v_str("Str")]), 0);
        assert_eq!(call(&mut r, "readfloat", &[v_str("Float")]), Value::Double(-2.5));
        assert_eq!(call(&mut r, "readfloat", &[v_str("Bin")]), Value::Double(0.0));
        let bin: Vec<i64> = (-2..=3).map(|i| int(&mut r, "readbinary", &[v_str("Bin"), v_int(i)])).collect();
        assert_eq!(bin, [0, 7, 0, 255, 0, 0]);
        assert_eq!(int(&mut r, "readbinary", &[v_str("FromText"), v_int(-1)]), 97);
        assert_eq!(int(&mut r, "readbinary", &[v_str("Str"), v_int(-1)]), 104, "a string's bytes");
        assert_eq!(int(&mut r, "readbinary", &[v_str("Int"), v_int(-1)]), 251);
        // written again: the same place in the list
        call(&mut r, "writeinteger", &[v_str("STR"), v_int(1)]);
        assert_eq!(text(&mut r, "valueitem", &[v_int(0)]), "Str");
        assert_eq!(int(&mut r, "getdatatype", &[v_str("Str")]), 3);

        // RenameValue: to the end of the list; not over another value
        call(&mut r, "renamevalue", &[v_str("Str"), v_str("Renamed")]);
        assert_eq!((int(&mut r, "valueexists", &[v_str("Str")]), int(&mut r, "readinteger", &[v_str("renamed")])), (0, 1));
        assert_eq!(text(&mut r, "valueitem", &[v_int(7)]), "Renamed");
        call(&mut r, "renamevalue", &[v_str("Renamed"), v_str("Int")]);
        assert_eq!((int(&mut r, "readinteger", &[v_str("Renamed")]), int(&mut r, "readinteger", &[v_str("Int")])), (1, -5));
        call(&mut r, "renamevalue", &[v_str("Renamed"), v_str("RENAMED")]);
        assert_eq!(text(&mut r, "valueitem", &[v_int(7)]), "RENAMED", "a new case");
        assert_eq!((int(&mut r, "deletevalue", &[v_str("renamed")]), int(&mut r, "deletevalue", &[v_str("renamed")])), (1, 0));
        assert_eq!(r.get("valueitemcount"), Some(v_int(7)));

        // sub-keys: listed alphabetically, ignoring case
        for k in ["b", "A2", "c\\deep"] {
            assert_eq!(int(&mut r, "createkey", &[v_str(k)]), 1);
        }
        assert_eq!(r.get("hassubkeys"), Some(v_int(1)));
        let keys: Vec<String> = (0..4).map(|i| text(&mut r, "keyitem", &[v_int(i)])).collect();
        assert_eq!(keys, ["A2", "b", "c", ""]);
        assert_eq!(int(&mut r, "deletekey", &[v_str("B")]), 1);
        assert_eq!(text(&mut r, "keyitem", &[v_int(1)]), "c", "a walk after a change");
        assert_eq!(r.get("keyitemcount"), Some(v_int(2)));
        assert_eq!(int(&mut r, "keyexists", &[v_str("C\\Deep")]), 1);
        assert_eq!(int(&mut r, "keyexists", &[at("kinds\\c\\deep")]), 1);

        // MoveKey: everything under it, added to what's there
        call(&mut r, "closekey", &[]);
        call(&mut r, "openkey", &[at("Moved"), v_int(1)]);
        call(&mut r, "writestring", &[v_str("Kept"), v_str("yes")]);
        assert_eq!(int(&mut r, "movekey", &[at("Kinds"), at("kinds\\inside"), v_int(1)]), 0, "not into itself");
        assert_eq!(int(&mut r, "movekey", &[at("Nope"), at("Elsewhere"), v_int(1)]), 0);
        assert_eq!(int(&mut r, "movekey", &[at("Kinds"), at("Moved"), v_int(0)]), 1);
        assert_eq!(int(&mut r, "keyexists", &[at("Kinds")]), 1, "copied");
        assert_eq!(int(&mut r, "movekey", &[at("Kinds"), at("Moved"), v_int(1)]), 1);
        assert_eq!(int(&mut r, "keyexists", &[at("Kinds")]), 0, "moved");
        assert_eq!(text(&mut r, "readstring", &[v_str("Kept")]), "yes");
        assert_eq!(text(&mut r, "readstring", &[v_str("")]), "the default");
        assert_eq!(call(&mut r, "readfloat", &[v_str("Float")]), Value::Double(-2.5));
        assert_eq!(int(&mut r, "getdatatype", &[v_str("Bin")]), 4);
        assert_eq!(r.get("valueitemcount"), Some(v_int(8)));
        assert_eq!(int(&mut r, "keyexists", &[v_str("c\\deep")]), 1);

        // the open key deleted meanwhile: reads find nothing, a write makes it again
        let mut other = Registry::default();
        assert_eq!(int(&mut other, "deletekey", &[at("Moved")]), 1);
        assert_eq!(r.get("currentpath"), Some(v_str(&format!("{}\\Moved", base.trim_start_matches('\\')))));
        assert_eq!((r.get("valueitemcount"), int(&mut r, "getdatasize", &[v_str("Kept")])), (Some(v_int(0)), -1));
        call(&mut r, "writeinteger", &[v_str("Again"), v_int(3)]);
        assert_eq!(int(&mut other, "keyexists", &[at("Moved")]), 1);
        // deleting a key above the open one closes it
        assert_eq!(int(&mut r, "deletekey", &[v_str(base)]), 1);
        assert_eq!((r.get("currentkey"), r.get("currentpath")), (Some(v_int(0)), Some(v_str(""))));
        assert_eq!(int(&mut r, "keyexists", &[v_str(base)]), 0);
    }

    #[test]
    fn shared_semantics_in_the_store() {
        set_io(load, save);
        shared_semantics("\\Software\\RapidR-Test\\unit");
        // what was written is text that reads back the same
        let text = load().unwrap();
        assert_eq!(write(&parse(&text)), text);
    }

    #[test]
    fn roots_are_never_deleted() {
        set_io(load, save);
        let mut r = Registry::default();
        r.call("openkey", &[v_str("Kept"), v_int(1)]);
        r.call("closekey", &[]);
        assert_eq!(r.call("deletekey", &[v_str("")]), Some(v_int(0)));
        assert_eq!(r.call("deletekey", &[v_str("\\")]), Some(v_int(0)));
        assert_eq!(r.call("movekey", &[v_str("\\"), v_str("\\Elsewhere"), v_int(1)]), Some(v_int(0)));
        assert_eq!(r.call("keyexists", &[v_str("Kept")]), Some(v_int(1)));
    }

    #[test]
    fn other_kinds_of_value() {
        // (as Regedit exports them: REG_MULTI_SZ, REG_QWORD, REG_NONE, a
        // REG_EXPAND_SZ, a REG_DWORD in hex(4))
        let reg = "Windows Registry Editor Version 5.00\n\n[HKEY_CURRENT_USER\\Kinds]\n\"Multi\"=hex(7):61,00,00,00,62,00,00,00,00,00\n\"Qword\"=hex(b):01,00,00,00,00,00,00,00\n\"None\"=hex(0):\n\"Path\"=hex(2):25,00,54,00,4d,00,50,00,25,00,00,00\n\"Dword\"=hex(4):2a,00,00,00\n";
        TEXT.with(|t| *t.borrow_mut() = Some(reg.to_string()));
        set_io(load, save);
        let mut r = Registry::default();
        r.call("openkey", &[v_str("Kinds"), v_int(0)]);
        let kinds: Vec<i64> = ["Multi", "Qword", "None", "Path", "Dword"].iter().map(|n| int(&mut r, "getdatatype", &[v_str(n)])).collect();
        assert_eq!(kinds, [0, 0, 0, 2, 3]);
        let sizes: Vec<i64> = ["Multi", "Qword", "None", "Path"].iter().map(|n| int(&mut r, "getdatasize", &[v_str(n)])).collect();
        assert_eq!(sizes, [10, 8, 0, 6]);
        assert_eq!(text(&mut r, "readstring", &[v_str("Path")]), "%TMP%");
        assert_eq!(text(&mut r, "readstring", &[v_str("Multi")]), "");
        assert_eq!(int(&mut r, "readinteger", &[v_str("Dword")]), 42);
        assert_eq!(int(&mut r, "readbinary", &[v_str("Qword"), v_int(-1)]), 1);
        // they're kept as they were (MoveKey too)
        assert_eq!(int(&mut r, "movekey", &[v_str("\\Kinds"), v_str("\\Moved"), v_int(1)]), 1);
        let text = load().unwrap();
        assert!(text.contains("[HKEY_CURRENT_USER\\Moved]\n\"Multi\"=hex(7):61,00,00,00,62,00,00,00,00,00\n\"Qword\"=hex(b):01,"), "{text}");
        assert!(text.contains("\"None\"=hex(0):\n\"Path\"=hex(2):25,00,54,") && text.contains("\"Dword\"=dword:0000002a"), "{text}");
    }

    /// The same answers from Windows' registry, under
    /// `HKEY_CURRENT_USER\Software\RapidR-Test` (made and deleted). Not run
    /// by default — test runs leave the machine's registry alone:
    /// `cargo test -p rapidr-value registry -- --ignored` on Windows, without
    /// RAPIDR_REGISTRY.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn shared_semantics_in_windows_registry() {
        use windows_registry::CURRENT_USER;
        assert!(std::env::var_os("RAPIDR_REGISTRY").is_none(), "RAPIDR_REGISTRY is set: the store, not the registry");
        // (the test's key goes, whatever happens; a run stopped half-way
        // left it: gone first)
        struct Cleanup;
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = CURRENT_USER.remove_tree("Software\\RapidR-Test");
            }
        }
        drop(Cleanup);
        let _cleanup = Cleanup;
        let base = "\\Software\\RapidR-Test\\unit";
        shared_semantics(base);
        // other kinds of value (put there as other programs would), kept
        // by MoveKey as they were
        let key = CURRENT_USER.create("Software\\RapidR-Test\\unit\\Kinds").unwrap();
        key.set_multi_string("Multi", &["a", "b"]).unwrap();
        key.set_u64("Qword", 1).unwrap();
        key.set_expand_string("Path", "%TMP%").unwrap();
        let before: Vec<_> = ["Multi", "Qword", "Path"].iter().map(|n| key.get_value(n).unwrap()).collect();
        drop(key);
        let mut r = Registry::default();
        r.call("openkey", &[v_str(&format!("{base}\\Kinds")), v_int(0)]);
        let kinds: Vec<i64> = ["Multi", "Qword", "Path"].iter().map(|n| int(&mut r, "getdatatype", &[v_str(n)])).collect();
        assert_eq!(kinds, [0, 0, 2]);
        let sizes: Vec<i64> = ["Multi", "Qword", "Path"].iter().map(|n| int(&mut r, "getdatasize", &[v_str(n)])).collect();
        assert_eq!(sizes, [10, 8, 6]);
        assert_eq!(text(&mut r, "readstring", &[v_str("Path")]), "%TMP%");
        assert_eq!(int(&mut r, "movekey", &[v_str(&format!("{base}\\Kinds")), v_str(&format!("{base}\\Moved")), v_int(1)]), 1);
        let moved = CURRENT_USER.open("Software\\RapidR-Test\\unit\\Moved").unwrap();
        let after: Vec<_> = ["Multi", "Qword", "Path"].iter().map(|n| moved.get_value(n).unwrap()).collect();
        assert_eq!(after, before);
        drop(moved);
        // a key the program may only read: read, opened, never changed
        r.set("rootkey", &v_int(i64::from(HKEY_LOCAL_MACHINE)));
        assert_eq!(int(&mut r, "openkey", &[v_str("\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"), v_int(0)]), 1);
        assert!(!text(&mut r, "readstring", &[v_str("ProductName")]).is_empty());
        assert_eq!(int(&mut r, "openkey", &[v_str("\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"), v_int(1)]), 1, "there: opened");
        r.set("rootkey", &v_int(i64::from(HKEY_CURRENT_USER)));
        assert_eq!(int(&mut r, "deletekey", &[v_str("\\Software\\RapidR-Test")]), 1);
        assert!(CURRENT_USER.open("Software\\RapidR-Test").is_err());
    }
}
