//! RapidQ's memory functions — `VARPTR`, `MEMCPY`, `MEMSET`, `MEMCMP`,
//! `VARPTR$`, `SIZEOF`, `QMemoryStream.Pointer`, `ReadUDT` / `WriteUDT` —
//! without raw memory: one model for native builds, the interpreter and the
//! browser, memory-safe.
//!
//! An address is an ordinary number (it fits a LONG, and `ptr + 4` works)
//! inside a *block*. A block is a live view of something the program owns:
//!
//! * an array (`VARPTR(a(0))`): its elements, laid out one after another;
//! * a TYPE instance (a UDT passed to MEMCPY, `UDTPTR`): its fields;
//! * a stream's buffer (`Mem.Pointer`);
//! * a *mirror* of a plain variable (`VARPTR(i)`, `VARPTR(s$)`): its bytes,
//!   which the compilers copy back into the variable after a statement that
//!   writes memory (`rapidr_ast::memory`).
//!
//! Reading or writing an address encodes the value into bytes the way
//! RapidQ stores it (BYTE 1, WORD / SHORT 2, INTEGER / LONG / DWORD /
//! SINGLE 4, DOUBLE 8, `STRING * n` n bytes, a variable-length STRING as the
//! address of its characters, TYPE fields packed in order) and decodes the
//! bytes back. An address outside every block, or past a block's end, or of
//! something that no longer exists, is a run-time error — never a crash.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};

use crate::objects::codec::{bytes_to_string, string_to_bytes};
use crate::{BasicArray, Instance, Value};

// ---------------------------------------------------------------------------
// Types and layout
// ---------------------------------------------------------------------------

/// How a value is stored in memory.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Byte,
    Word,
    Short,
    Long,
    Dword,
    Single,
    Double,
    /// A variable-length STRING: stored as the address of its characters.
    Str,
    /// `STRING * n`: n bytes.
    Fixed(usize),
    /// A TYPE (its fields, packed).
    Udt(String),
    /// Untyped: by what it holds (a number 4 or 8 bytes, a string its address).
    Variant,
}

impl Kind {
    /// The kind for a declared type: `INTEGER`, `STRING*8`, `STRING * 8`,
    /// a TYPE name; anything unknown is a VARIANT.
    pub fn of(type_name: &str) -> Kind {
        let t: String = type_name.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase();
        if let Some(n) = t.strip_prefix("STRING*") {
            return n.parse().map_or(Kind::Str, Kind::Fixed);
        }
        match t.as_str() {
            "BYTE" => Kind::Byte,
            "WORD" => Kind::Word,
            "SHORT" => Kind::Short,
            "INTEGER" | "LONG" => Kind::Long,
            "DWORD" => Kind::Dword,
            "SINGLE" => Kind::Single,
            "DOUBLE" => Kind::Double,
            "STRING" => Kind::Str,
            "" | "VARIANT" => Kind::Variant,
            _ if has_layout(&t) => Kind::Udt(t),
            _ => Kind::Variant,
        }
    }

    /// The size of one value of this kind, when fixed (not for VARIANT).
    fn fixed_size(&self) -> Option<usize> {
        Some(match self {
            Kind::Byte => 1,
            Kind::Word | Kind::Short => 2,
            Kind::Long | Kind::Dword | Kind::Single | Kind::Str => 4,
            Kind::Double => 8,
            Kind::Fixed(n) => *n,
            Kind::Udt(_) | Kind::Variant => return None,
        })
    }
}

thread_local! {
    /// The field kinds of each TYPE, in slot order (from `__newobject`).
    static LAYOUTS: RefCell<HashMap<String, Rc<[Kind]>>> = RefCell::new(HashMap::new());
}

fn has_layout(type_upper: &str) -> bool {
    LAYOUTS.with(|l| l.borrow().contains_key(type_upper))
}

/// Records the field kinds of `type_name` from the compilers' field list
/// (`Name:STRING*5,Age:INTEGER`); fields without a type are VARIANTs.
pub fn register_layout(type_name: &str, fields: &str) {
    let key = type_name.to_ascii_uppercase();
    if has_layout(&key) {
        return;
    }
    // Registered before the kinds are worked out, so a field of this very
    // TYPE (or a later one) can name it.
    LAYOUTS.with(|l| l.borrow_mut().insert(key.clone(), Rc::from(Vec::new())));
    let kinds: Vec<Kind> = fields
        .split(',')
        .filter(|f| !f.is_empty())
        .map(|f| f.split_once(':').map_or(Kind::Variant, |(_, t)| Kind::of(t)))
        .collect();
    LAYOUTS.with(|l| l.borrow_mut().insert(key, kinds.into()));
}

fn layout(type_name: &str) -> Rc<[Kind]> {
    LAYOUTS.with(|l| l.borrow().get(&type_name.to_ascii_uppercase()).cloned()).unwrap_or_else(|| Rc::from(Vec::new()))
}

/// `SIZEOF(v)` for a value of kind `kind` (an array: all its elements).
pub fn size_of(v: &Value, kind: &Kind) -> usize {
    let mut out = Vec::new();
    encode(v, kind, &mut out);
    out.len()
}

/// `SIZEOF(INTEGER)`, `SIZEOF(TMyType)`: the size of a type, if known.
pub fn size_of_type(type_name: &str) -> Option<usize> {
    match Kind::of(type_name) {
        Kind::Udt(t) => {
            let fields = layout(&t);
            Some(fields.iter().map(|k| k.fixed_size().unwrap_or(4)).sum())
        }
        Kind::Variant if !type_name.trim().eq_ignore_ascii_case("VARIANT") => None,
        k => k.fixed_size(),
    }
}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// The kind a VARIANT's current value is stored as.
fn variant_kind(v: &Value) -> Kind {
    match v {
        Value::Double(_) => Kind::Double,
        Value::String(_) => Kind::Str,
        Value::Object(o) => Kind::Udt(o.type_name.to_ascii_uppercase()),
        _ => Kind::Long,
    }
}

/// Appends `v`'s bytes, stored as `kind`.
pub fn encode(v: &Value, kind: &Kind, out: &mut Vec<u8>) {
    if let Value::Array(a) = v {
        let a = a.borrow();
        for e in &a.data {
            encode(e, kind, out);
        }
        return;
    }
    match kind {
        Kind::Byte => out.push(v.to_i64() as u8),
        Kind::Word | Kind::Short => out.extend_from_slice(&(v.to_i64() as u16).to_le_bytes()),
        Kind::Long | Kind::Dword => out.extend_from_slice(&(v.to_i64() as u32).to_le_bytes()),
        Kind::Single => out.extend_from_slice(&(v.to_f64() as f32).to_le_bytes()),
        Kind::Double => out.extend_from_slice(&v.to_f64().to_le_bytes()),
        Kind::Fixed(n) => {
            let mut b = string_to_bytes(&v.to_string_val());
            b.resize(*n, 0);
            out.extend_from_slice(&b);
        }
        Kind::Str => {
            let addr = string_address(&v.to_string_val());
            out.extend_from_slice(&(addr as u32).to_le_bytes());
        }
        Kind::Udt(t) => match v {
            Value::Object(o) => encode_fields(o, out),
            // An uninitialised field of a TYPE: zeros of its size.
            _ => out.resize(out.len() + size_of_type(t).unwrap_or(0), 0),
        },
        Kind::Variant => encode(v, &variant_kind(v), out),
    }
}

fn encode_fields(o: &Instance, out: &mut Vec<u8>) {
    let kinds = layout(&o.type_name);
    let fields = o.fields.borrow();
    for (i, f) in fields.iter().enumerate() {
        encode(f, kinds.get(i).unwrap_or(&Kind::Variant), out);
    }
}

/// Reads a value stored as `kind` from the front of `bytes`; `old` is the
/// value being replaced (a VARIANT keeps its kind; an array its shape).
/// Returns it and the number of bytes used.
pub fn decode(bytes: &[u8], kind: &Kind, old: &Value) -> (Value, usize) {
    if let Value::Array(a) = old {
        let mut used = 0;
        let n = a.borrow().data.len();
        for i in 0..n {
            let e = a.borrow().data[i].clone();
            let (v, u) = decode(&bytes[used.min(bytes.len())..], kind, &e);
            a.borrow_mut().data[i] = v;
            used += u;
        }
        return (old.clone(), used);
    }
    let take = |n: usize| -> [u8; 8] {
        let mut b = [0u8; 8];
        let n = n.min(bytes.len());
        b[..n].copy_from_slice(&bytes[..n]);
        b
    };
    match kind {
        Kind::Byte => (Value::Integer(take(1)[0] as i64), 1),
        Kind::Word => (Value::Integer(u16::from_le_bytes(take(2)[..2].try_into().unwrap()) as i64), 2),
        Kind::Short => (Value::Integer(i16::from_le_bytes(take(2)[..2].try_into().unwrap()) as i64), 2),
        Kind::Long => (Value::Integer(i32::from_le_bytes(take(4)[..4].try_into().unwrap()) as i64), 4),
        Kind::Dword => (Value::Integer(u32::from_le_bytes(take(4)[..4].try_into().unwrap()) as i64), 4),
        Kind::Single => (Value::Double(f32::from_le_bytes(take(4)[..4].try_into().unwrap()) as f64), 4),
        Kind::Double => (Value::Double(f64::from_le_bytes(take(8))), 8),
        Kind::Fixed(n) => {
            let b = &bytes[..(*n).min(bytes.len())];
            let text = b.split(|&c| c == 0).next().unwrap_or_default();
            (Value::String(bytes_to_string(text)), *n)
        }
        Kind::Str => {
            let addr = u32::from_le_bytes(take(4)[..4].try_into().unwrap()) as i64;
            (Value::String(if addr == 0 { String::new() } else { c_string(addr).unwrap_or_default() }), 4)
        }
        Kind::Udt(_) => match old {
            Value::Object(o) => {
                let used = decode_fields(bytes, o);
                (old.clone(), used)
            }
            _ => (old.clone(), 0),
        },
        Kind::Variant => decode(bytes, &variant_kind(old), old),
    }
}

/// Decodes `bytes` into the fields of `o` (in place); returns the bytes used.
fn decode_fields(bytes: &[u8], o: &Instance) -> usize {
    let kinds = layout(&o.type_name);
    let n = o.fields.borrow().len();
    let mut used = 0;
    for i in 0..n {
        let old = o.get(i);
        let (v, u) = decode(&bytes[used.min(bytes.len())..], kinds.get(i).unwrap_or(&Kind::Variant), &old);
        o.set(i, v);
        used += u;
    }
    used
}

// ---------------------------------------------------------------------------
// The address space
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Target {
    Array { array: Weak<RefCell<BasicArray>>, kind: Kind },
    Object(Weak<Instance>),
    Mirror(Rc<RefCell<Mirror>>),
    Stream(String),
}

/// The bytes of a plain variable (or a string's characters), copied back
/// into it after a statement that wrote them (`sync`).
#[derive(Default)]
struct Mirror {
    bytes: Vec<u8>,
    dirty: bool,
}

struct Block {
    reserved: usize,
    target: Target,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Key {
    Array(usize),
    Object(usize),
    Mirror(String),
    Stream(String),
}

#[derive(Default)]
struct Space {
    blocks: BTreeMap<i64, Block>,
    by_key: HashMap<Key, i64>,
    next: i64,
    /// String addresses handed out while encoding (a STRING field, VARPTR
    /// of a string value): their text by address, reused for equal text.
    strings: HashMap<String, i64>,
    /// Every address handed to the program (VARPTR, Pointer, a UDT's or an
    /// element's address): what a native DLL call marshals (`is_issued`).
    issued: std::collections::HashSet<i64>,
}

fn issue(addr: i64) -> i64 {
    SPACE.with(|s| s.borrow_mut().issued.insert(addr));
    addr
}

/// Whether `addr` is an address this program was given (a DLL argument
/// that is one gets a real buffer: `rapidr_runtime_core::ffi`).
pub fn is_issued(addr: i64) -> bool {
    SPACE.with(|s| s.borrow().issued.contains(&addr))
}

/// The bytes from `addr` to the end of its block (what a DLL may use).
pub fn bytes_from(addr: i64) -> Result<Vec<u8>, String> {
    let (base, t) = find(addr)?;
    let bytes = contents(&t);
    let off = ((addr - base) as usize).min(bytes.len());
    Ok(bytes[off..].to_vec())
}

/// Addresses start above the first MB (so 0 and small numbers are never
/// valid) and stay below 2^31 (they fit a LONG).
const FIRST: i64 = 0x0010_0000;
const LIMIT: i64 = 0x7FFF_0000;

thread_local! {
    static SPACE: RefCell<Space> = RefCell::new(Space { next: FIRST, ..Default::default() });
}

fn reserve_for(len: usize) -> usize {
    // Room to grow (a stream written to, an array REDIMmed) and a gap so a
    // run past the end never lands in the next block.
    (len.saturating_mul(2).max(256) + 4095) & !4095
}

/// The base address of the block for `key`, making it (with `target`) when
/// there's none or it's too small for `len` bytes now.
fn block_for(key: Key, len: usize, target: impl FnOnce() -> Target) -> Result<i64, String> {
    SPACE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(&base) = s.by_key.get(&key) {
            if s.blocks.get(&base).is_some_and(|b| b.reserved >= len && alive(&b.target)) {
                return Ok(base);
            }
        }
        let reserved = reserve_for(len);
        let base = s.next;
        if base + reserved as i64 >= LIMIT {
            return Err("out of memory for addresses (VARPTR)".into());
        }
        s.next = base + reserved as i64 + 4096;
        s.blocks.insert(base, Block { reserved, target: target() });
        s.by_key.insert(key, base);
        Ok(base)
    })
}

fn alive(t: &Target) -> bool {
    match t {
        Target::Array { array, .. } => array.strong_count() > 0,
        Target::Object(o) => o.strong_count() > 0,
        _ => true,
    }
}

/// The block holding `addr`: (its base, its target).
fn find(addr: i64) -> Result<(i64, Target), String> {
    SPACE.with(|s| {
        let s = s.borrow();
        match s.blocks.range(..=addr).next_back() {
            Some((&base, b)) if addr < base + b.reserved as i64 => {
                if !alive(&b.target) {
                    return Err(format!("address {addr} belongs to an array or TYPE that no longer exists"));
                }
                Ok((base, b.target.clone()))
            }
            _ => Err(format!("address {addr} isn't memory of this program (use VARPTR of a variable, an array element, a TYPE, or a stream's Pointer)")),
        }
    })
}

/// The whole contents of a block now.
fn contents(t: &Target) -> Vec<u8> {
    let mut out = Vec::new();
    match t {
        Target::Array { array, kind } => {
            if let Some(a) = array.upgrade() {
                encode(&Value::Array(a), kind, &mut out);
            }
        }
        Target::Object(o) => {
            if let Some(o) = o.upgrade() {
                encode_fields(&o, &mut out);
            }
        }
        Target::Mirror(m) => out = m.borrow().bytes.clone(),
        Target::Stream(name) => out = crate::objects::with_stream(name, |s| s.data.clone()).unwrap_or_default(),
    }
    out
}

/// Stores `bytes` as the block's whole contents.
fn set_contents(t: &Target, bytes: &[u8]) {
    match t {
        Target::Array { array, kind } => {
            if let Some(a) = array.upgrade() {
                decode(bytes, kind, &Value::Array(a));
            }
        }
        Target::Object(o) => {
            if let Some(o) = o.upgrade() {
                decode_fields(bytes, &o);
            }
        }
        Target::Mirror(m) => {
            let mut m = m.borrow_mut();
            m.bytes = bytes.to_vec();
            m.dirty = true;
        }
        Target::Stream(name) => {
            crate::objects::with_stream_mut(name, |s| s.data = bytes.to_vec());
        }
    }
}

/// The fixed element size of an array block (for reads and writes of a
/// few elements of a big array without encoding all of it).
fn element_size(t: &Target) -> Option<(Rc<RefCell<BasicArray>>, Kind, usize)> {
    if let Target::Array { array, kind } = t {
        let size = kind.fixed_size().filter(|_| *kind != Kind::Str)?;
        return Some((array.upgrade()?, kind.clone(), size));
    }
    None
}

/// `n` bytes at `addr`.
pub fn read(addr: i64, n: usize) -> Result<Vec<u8>, String> {
    let (base, t) = find(addr)?;
    let off = (addr - base) as usize;
    if let Some((array, kind, size)) = element_size(&t) {
        let a = array.borrow();
        let total = a.data.len() * size;
        check_end(addr, off, n, total)?;
        let (first, last) = (off / size, (off + n).div_ceil(size));
        let mut out = Vec::with_capacity((last - first) * size);
        for e in &a.data[first..last] {
            encode(e, &kind, &mut out);
        }
        let start = off - first * size;
        return Ok(out[start..start + n].to_vec());
    }
    let bytes = contents(&t);
    check_end(addr, off, n, bytes.len())?;
    Ok(bytes[off..off + n].to_vec())
}

fn check_end(addr: i64, off: usize, n: usize, len: usize) -> Result<(), String> {
    if off + n > len {
        return Err(format!("{n} bytes at address {addr} run past the end of its memory ({} bytes left)", len.saturating_sub(off)));
    }
    Ok(())
}

/// Writes `bytes` at `addr`.
pub fn write(addr: i64, bytes: &[u8]) -> Result<(), String> {
    let (base, t) = find(addr)?;
    let off = (addr - base) as usize;
    if let Some((array, kind, size)) = element_size(&t) {
        let len = array.borrow().data.len();
        check_end(addr, off, bytes.len(), len * size)?;
        let (first, last) = (off / size, (off + bytes.len()).div_ceil(size));
        let mut buf = Vec::with_capacity((last - first) * size);
        for e in &array.borrow().data[first..last] {
            encode(e, &kind, &mut buf);
        }
        let start = off - first * size;
        buf[start..start + bytes.len()].copy_from_slice(bytes);
        for (i, chunk) in buf.chunks(size).enumerate() {
            let old = array.borrow().data[first + i].clone();
            let (v, _) = decode(chunk, &kind, &old);
            array.borrow_mut().data[first + i] = v;
        }
        return Ok(());
    }
    let mut all = contents(&t);
    // A stream grows when written past its end (within its block).
    if matches!(t, Target::Stream(_)) && off + bytes.len() > all.len() {
        all.resize(off + bytes.len(), 0);
    }
    check_end(addr, off, bytes.len(), all.len())?;
    all[off..off + bytes.len()].copy_from_slice(bytes);
    set_contents(&t, &all);
    Ok(())
}

/// The characters at `addr` up to a NUL (`VARPTR$`), within the block.
pub fn c_string(addr: i64) -> Result<String, String> {
    let (base, t) = find(addr)?;
    let bytes = contents(&t);
    let off = ((addr - base) as usize).min(bytes.len());
    let text = bytes[off..].split(|&c| c == 0).next().unwrap_or_default();
    Ok(bytes_to_string(text))
}

/// An address holding `text` and a NUL (a STRING stored in memory).
fn string_address(text: &str) -> i64 {
    if let Some(a) = SPACE.with(|s| s.borrow().strings.get(text).copied()) {
        return a;
    }
    let mut bytes = string_to_bytes(text);
    bytes.push(0);
    let mirror = Rc::new(RefCell::new(Mirror { bytes, dirty: false }));
    let len = mirror.borrow().bytes.len();
    let key = Key::Mirror(format!("\u{0}str:{}", SPACE.with(|s| s.borrow().strings.len())));
    let addr = block_for(key, len, || Target::Mirror(mirror)).map_or(0, issue);
    SPACE.with(|s| s.borrow_mut().strings.insert(text.to_string(), addr));
    addr
}

// ---------------------------------------------------------------------------
// What the compilers call
// ---------------------------------------------------------------------------

/// The address of a value passed where memory is expected: a number is an
/// address; a TYPE instance (RapidQ passes UDTs by address) or an array
/// (its first element) gets its block.
pub fn address_of(v: &Value) -> Result<i64, String> {
    match v {
        Value::Integer(n) => Ok(*n),
        Value::Double(d) => Ok(*d as i64),
        Value::Boolean(b) => Ok(if *b { -1 } else { 0 }),
        Value::Object(o) => object_address(o),
        Value::Array(a) => array_address(a, &Kind::Variant, 0),
        Value::String(_) => Err("a STRING isn't an address: use VARPTR(s$)".into()),
        Value::Null => Ok(0),
    }
}

fn object_address(o: &Rc<Instance>) -> Result<i64, String> {
    let len = { let mut b = Vec::new(); encode_fields(o, &mut b); b.len() };
    block_for(Key::Object(Rc::as_ptr(o) as usize), len, || Target::Object(Rc::downgrade(o))).map(issue)
}

/// The address of element `flat` (in memory order) of an array of `kind`.
fn array_address(a: &Rc<RefCell<BasicArray>>, kind: &Kind, flat: usize) -> Result<i64, String> {
    let kind = if *kind == Kind::Variant { a.borrow().data.first().map_or(Kind::Long, variant_kind) } else { kind.clone() };
    let len = size_of(&Value::Array(a.clone()), &kind);
    let base = block_for(Key::Array(Rc::as_ptr(a) as usize), len, || Target::Array { array: Rc::downgrade(a), kind: kind.clone() })?;
    // The offset of the element: the size of those before it.
    let before: usize = a.borrow().data[..flat.min(a.borrow().data.len())].iter().map(|e| size_of(e, &kind)).sum();
    Ok(issue(base + before as i64))
}

/// `VARPTR(a(i, j))`: the address of an element of an array declared `type`.
pub fn varptr_element(array: &Value, type_name: &str, indices: &[i64]) -> Result<Value, String> {
    let Value::Array(a) = array else { return Err("VARPTR of an element of something that isn't an array".into()) };
    let flat = if indices.is_empty() { 0 } else { a.borrow().offset(indices)? };
    // An element that's a TYPE instance has its own block.
    if let Some(Value::Object(o)) = a.borrow().data.get(flat) {
        return object_address(o).map(Value::Integer);
    }
    array_address(a, &Kind::of(type_name), flat).map(Value::Integer)
}

/// `VARPTR(x)` of a plain variable: its mirror (made or refreshed with the
/// value it holds now), keyed by the variable (`key`).
pub fn varptr_var(key: &str, v: &Value, type_name: &str) -> Result<Value, String> {
    match v {
        Value::Object(o) => return object_address(o).map(Value::Integer),
        Value::Array(a) => return array_address(a, &Kind::of(type_name), 0).map(Value::Integer),
        _ => {}
    }
    let bytes = mirror_bytes(v, &Kind::of(type_name));
    let len = bytes.len();
    let mirror = Rc::new(RefCell::new(Mirror { bytes: bytes.clone(), dirty: false }));
    let base = block_for(Key::Mirror(key.to_ascii_lowercase()), len, || Target::Mirror(mirror))?;
    // An existing mirror: refreshed.
    if let Ok((_, Target::Mirror(m))) = find(base) {
        let mut m = m.borrow_mut();
        m.bytes = bytes;
        m.dirty = false;
    }
    Ok(Value::Integer(issue(base)))
}

/// A variable's bytes in its mirror: a STRING's characters and a NUL (as
/// RapidQ's VARPTR of a string points at its characters), anything else as
/// stored.
fn mirror_bytes(v: &Value, kind: &Kind) -> Vec<u8> {
    match (kind, v) {
        (Kind::Str, _) | (Kind::Variant, Value::String(_)) => {
            let mut b = string_to_bytes(&v.to_string_val());
            b.push(0);
            b
        }
        _ => {
            let mut b = Vec::new();
            encode(v, kind, &mut b);
            b
        }
    }
}

/// Before a statement that uses memory: the mirror of `key` (if any) holds
/// the variable's value now.
pub fn refresh(key: &str, v: &Value, type_name: &str) {
    let base = SPACE.with(|s| s.borrow().by_key.get(&Key::Mirror(key.to_ascii_lowercase())).copied());
    let Some(base) = base else { return };
    if let Ok((_, Target::Mirror(m))) = find(base) {
        let bytes = mirror_bytes(v, &Kind::of(type_name));
        let mut m = m.borrow_mut();
        // (a longer value than the block holds keeps the old length: VARPTR
        // again gives a bigger block)
        if bytes.len() <= m.bytes.len() || m.bytes.is_empty() {
            m.bytes = bytes;
        } else {
            let n = m.bytes.len();
            m.bytes.copy_from_slice(&bytes[..n]);
        }
        m.dirty = false;
    }
}

/// After a statement that wrote memory: the variable's value, taken back
/// from its mirror if the statement wrote there.
pub fn sync(key: &str, v: &Value, type_name: &str) -> Value {
    let base = SPACE.with(|s| s.borrow().by_key.get(&Key::Mirror(key.to_ascii_lowercase())).copied());
    let Some(base) = base else { return v.clone() };
    let Ok((_, Target::Mirror(m))) = find(base) else { return v.clone() };
    let mut m = m.borrow_mut();
    if !m.dirty {
        return v.clone();
    }
    m.dirty = false;
    let kind = Kind::of(type_name);
    match (&kind, v) {
        // A string keeps its length (its buffer was written in place).
        (Kind::Str, _) | (Kind::Variant, Value::String(_)) => {
            let n = m.bytes.len().saturating_sub(1);
            Value::String(bytes_to_string(&m.bytes[..n]))
        }
        _ => decode(&m.bytes, &kind, v).0,
    }
}

/// `QMemoryStream.Pointer`: the address of the stream's first byte.
pub fn stream_pointer(name: &str) -> i64 {
    let len = crate::objects::with_stream(name, |s| s.data.len()).unwrap_or(0);
    block_for(Key::Stream(name.to_ascii_lowercase()), len, || Target::Stream(name.to_string())).map_or(0, issue)
}

/// `MEMCPY(dest, src, n)`.
pub fn memcpy(dest: &Value, src: &Value, n: &Value) -> Result<(), String> {
    let n = count(n)?;
    if n == 0 {
        return Ok(());
    }
    let bytes = read(address_of(src)?, n)?;
    write(address_of(dest)?, &bytes)
}

/// `MEMSET(ptr, byte, n)`.
pub fn memset(dest: &Value, byte: &Value, n: &Value) -> Result<(), String> {
    let n = count(n)?;
    if n == 0 {
        return Ok(());
    }
    write(address_of(dest)?, &vec![byte.to_i64() as u8; n])
}

/// `MEMCMP(a, b, n)`: non-zero (-1) when the bytes are the same, 0 if not.
pub fn memcmp(a: &Value, b: &Value, n: &Value) -> Result<Value, String> {
    let n = count(n)?;
    let same = read(address_of(a)?, n)? == read(address_of(b)?, n)?;
    Ok(Value::Integer(if same { -1 } else { 0 }))
}

fn count(n: &Value) -> Result<usize, String> {
    let n = n.to_i64();
    if n < 0 {
        return Err(format!("a negative byte count ({n})"));
    }
    Ok(n as usize)
}

/// `Stream.WriteUDT(x)`: the TYPE's bytes at the stream's position.
pub fn udt_bytes(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode(v, &Kind::Variant, &mut out);
    out
}

/// `Stream.ReadUDT(x)`: fills the TYPE from `bytes`; returns the bytes used.
pub fn udt_from_bytes(v: &Value, bytes: &[u8]) -> usize {
    match v {
        Value::Object(o) => decode_fields(bytes, o),
        _ => 0,
    }
}


// ---------------------------------------------------------------------------
// Entry points: the VM hosts (`shared`) and native builds (`rp_*`)
// ---------------------------------------------------------------------------

/// `SIZEOF(x)` at run time: a STRING's length, an array's or a TYPE's bytes,
/// a number's size as `type_name`.
pub fn sizeof_value(v: &Value, type_name: &str) -> i64 {
    let kind = Kind::of(type_name);
    match (v, &kind) {
        (Value::String(s), Kind::Str | Kind::Variant) => string_to_bytes(s).len() as i64,
        _ => size_of(v, &kind) as i64,
    }
}

/// The memory builtins for the VM hosts (`rapidr_value::shared_builtin`).
pub fn shared(key: &str, args: &[Value]) -> Option<Result<Value, String>> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    let s = |i: usize| arg(i).to_string_val();
    Some(match key {
        "__varptr_var" => varptr_var(&s(0), &arg(1), &s(2)),
        "__varptr_elem" => {
            let idx: Vec<i64> = args.get(2..).unwrap_or(&[]).iter().map(Value::to_i64).collect();
            varptr_element(&arg(0), &s(1), &idx)
        }
        "__mem_refresh" => {
            refresh(&s(0), &arg(1), &s(2));
            Ok(Value::Null)
        }
        "__mem_sync" => Ok(sync(&s(0), &arg(1), &s(2))),
        "__sizeof" => Ok(Value::Integer(sizeof_value(&arg(0), &s(1)))),
        "__sizeof_type" => Ok(Value::Integer(size_of_type(&s(0)).unwrap_or(0) as i64)),
        "__cstring" => address_of(&arg(0)).and_then(c_string).map(Value::String),
        "memcpy" => memcpy(&arg(0), &arg(1), &arg(2)).map(|_| Value::Null),
        "memset" => memset(&arg(0), &arg(1), &arg(2)).map(|_| Value::Null),
        "memcmp" => memcmp(&arg(0), &arg(1), &arg(2)),
        _ => return None,
    })
}

/// The builtins [`shared`] handles.
pub const BUILTINS: &[&str] = &["__varptr_var", "__varptr_elem", "__mem_refresh", "__mem_sync", "__sizeof", "__sizeof_type", "__cstring", "memcpy", "memset", "memcmp"];

fn or_fail<T>(r: Result<T, String>) -> T {
    r.unwrap_or_else(|e| crate::runtime_error(&e))
}

pub fn rp_varptr_var(key: &Value, v: &Value, type_name: &Value) -> Value {
    or_fail(varptr_var(&key.to_string_val(), v, &type_name.to_string_val()))
}

pub fn rp_varptr_elem(array: &Value, type_name: &Value, indices: &[i64]) -> Value {
    or_fail(varptr_element(array, &type_name.to_string_val(), indices))
}

pub fn rp_mem_refresh(key: &Value, v: &Value, type_name: &Value) -> Value {
    refresh(&key.to_string_val(), v, &type_name.to_string_val());
    Value::Null
}

pub fn rp_mem_sync(key: &Value, v: &Value, type_name: &Value) -> Value {
    sync(&key.to_string_val(), v, &type_name.to_string_val())
}

pub fn rp_sizeof(v: &Value, type_name: &Value) -> Value {
    Value::Integer(sizeof_value(v, &type_name.to_string_val()))
}

pub fn rp_sizeof_type(type_name: &Value) -> Value {
    Value::Integer(size_of_type(&type_name.to_string_val()).unwrap_or(0) as i64)
}

pub fn rp_cstring(addr: &Value) -> Value {
    Value::String(or_fail(address_of(addr).and_then(c_string)))
}

pub fn rp_memcpy(dest: &Value, src: &Value, n: &Value) -> Value {
    or_fail(memcpy(dest, src, n));
    Value::Null
}

pub fn rp_memset(dest: &Value, byte: &Value, n: &Value) -> Value {
    or_fail(memset(dest, byte, n));
    Value::Null
}

pub fn rp_memcmp(a: &Value, b: &Value, n: &Value) -> Value {
    or_fail(memcmp(a, b, n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{rp_new_array, v_dbl, v_int, v_str};

    fn udt() -> Value {
        register_layout("TREC", "S:STRING*8,N:INTEGER,D:DOUBLE,B:BYTE");
        let o = Instance::new("r", "TRec", "S:STRING*8,N:INTEGER,D:DOUBLE,B:BYTE");
        o.set(0, v_str("abc"));
        o.set(1, v_int(-2));
        o.set(2, v_dbl(1.5));
        o.set(3, v_int(7));
        Value::Object(o)
    }

    #[test]
    fn udt_layout_is_packed_like_rapidq() {
        let r = udt();
        assert_eq!(size_of_type("TRec"), Some(8 + 4 + 8 + 1));
        let b = udt_bytes(&r);
        assert_eq!(b.len(), 21);
        assert_eq!(&b[..4], b"abc\0");
        assert_eq!(&b[8..12], &(-2i32).to_le_bytes());
        let copy = Value::Object(Instance::new("c", "TRec", "S:STRING*8,N:INTEGER,D:DOUBLE,B:BYTE"));
        udt_from_bytes(&copy, &b);
        let Value::Object(c) = &copy else { unreachable!() };
        assert_eq!(c.get(0).to_string_val(), "abc");
        assert_eq!(c.get(1).to_i64(), -2);
        assert_eq!(c.get(2).to_f64(), 1.5);
        assert_eq!(c.get(3).to_i64(), 7);
    }

    #[test]
    fn memcpy_between_variables_arrays_and_udts() {
        // MEMCPY(VARPTR(i), VARPTR(j), 4)
        let pi = varptr_var("main:i", &v_int(90), "INTEGER").unwrap();
        let pj = varptr_var("main:j", &v_int(10), "INTEGER").unwrap();
        memcpy(&pi, &pj, &v_int(4)).unwrap();
        assert_eq!(sync("main:i", &v_int(90), "INTEGER").to_i64(), 10);
        assert_eq!(sync("main:i", &v_int(10), "INTEGER").to_i64(), 10, "synced once");
        // Array elements: MEMSET(VARPTR(a(1)), 0, 2 * SIZEOF(INTEGER))
        let a = rp_new_array(&[(0, 3)], v_int(5));
        let p1 = varptr_element(&a, "INTEGER", &[1]).unwrap();
        memset(&p1, &v_int(0), &v_int(8)).unwrap();
        assert_eq!((a.rp_get(&[0]).to_i64(), a.rp_get(&[1]).to_i64(), a.rp_get(&[2]).to_i64(), a.rp_get(&[3]).to_i64()), (5, 0, 0, 5));
        // Byte-level: a SHORT array written through bytes.
        let s = rp_new_array(&[(0, 1)], v_int(0));
        let ps = varptr_element(&s, "SHORT", &[0]).unwrap();
        write(ps.to_i64() + 1, &[0xFF]).unwrap();
        assert_eq!(s.rp_get(&[0]).to_i64(), -256);
        // A UDT copied to another (passed by address).
        let r = udt();
        let copy = Value::Object(Instance::new("c", "TRec", "S:STRING*8,N:INTEGER,D:DOUBLE,B:BYTE"));
        memcpy(&copy, &r, &v_int(21)).unwrap();
        let Value::Object(c) = &copy else { unreachable!() };
        assert_eq!((c.get(0).to_string_val(), c.get(3).to_i64()), ("abc".to_string(), 7));
        assert_eq!(memcmp(&copy, &r, &v_int(21)).unwrap().to_i64(), -1);
    }

    #[test]
    fn strings_and_bad_addresses() {
        let p = varptr_var("main:s", &v_str("This is a test"), "STRING").unwrap();
        assert_eq!(c_string(p.to_i64()).unwrap(), "This is a test");
        assert_eq!(c_string(p.to_i64() + 5).unwrap(), "is a test");
        write(p.to_i64(), b"THIS").unwrap();
        assert_eq!(sync("main:s", &v_str("This is a test"), "STRING").to_string_val(), "THIS is a test");
        assert!(read(0, 4).is_err());
        assert!(read(12345, 4).is_err());
        assert!(read(p.to_i64(), 100).unwrap_err().contains("past the end"));
        // A TYPE that's gone: its address is refused.
        let gone = {
            let r = udt();
            address_of(&r).unwrap()
        };
        assert!(read(gone, 4).unwrap_err().contains("no longer exists"));
    }
}
