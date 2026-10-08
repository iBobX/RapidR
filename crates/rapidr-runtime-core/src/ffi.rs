//! `DECLARE … LIB`: calling a DLL's function (docs/windows-dll-calls.md §1).
//!
//! Both compilers lower a call to a DECLAREd LIB routine to
//! `__dll_call(lib, alias, spec, args…)`; native builds reach [`rp_dll_call`],
//! the interpreter's native host [`dll_call`]. The spec
//! (`rapidr_value::dll::Spec`) carries the DECLARE's types: what RapidQ
//! passes by value (numbers, unless BYREF) and by address (strings, TYPEs,
//! BYREF numbers).
//!
//! Pointers are the program's own memory: before the call every live block
//! of `rapidr_value::memory` is materialised at its address (real pages on
//! Windows), an argument that is an address inside one becomes that
//! pointer, and afterwards what the DLL wrote is read back.
//!
//! The call itself goes through a table of exact `extern "system"`
//! signatures (the one convention 64-bit Windows has; "C" elsewhere): up to
//! 16 integer arguments, and DOUBLE arguments in calls of up to 4 — no C
//! library, no assembler.

use std::cell::RefCell;
use std::collections::HashMap;

use libloading::Library;

use crate::value::dll::{self, Param, Spec};
use crate::value::memory;
use crate::value::objects::codec::string_to_bytes;
use crate::value::{v_dbl, v_int, v_null, v_str, Value};

thread_local! {
    /// Loaded libraries by their lowercase name.
    static LOADED: RefCell<HashMap<String, Library>> = RefCell::new(HashMap::new());
}

/// The function being called, for the crash filter's message.
static CALLING: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
/// The last function called, for a crash after it returned.
static LAST_CALL: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

fn library_error(lib: &str, e: &libloading::Error) -> String {
    let text = e.to_string();
    #[cfg(windows)]
    {
        if text.contains("os error 193") {
            return format!("'{lib}' is a 32-bit DLL; RapidR programs are 64-bit, so Windows can't load it (a 64-bit build of that DLL is needed)");
        }
        if text.contains("os error 126") {
            return format!("can't find the DLL '{lib}' (LIB \"{lib}\"): it isn't one of Windows' and isn't beside the program");
        }
    }
    format!("can't load the library '{lib}': {text}")
}

/// The library `lib` names, loaded once (Windows' own search: `"user32"`,
/// `"user32.dll"`, a path).
fn with_library<T>(lib: &str, f: impl FnOnce(&Library) -> Result<T, String>) -> Result<T, String> {
    let key = lib.trim_matches('"').to_ascii_lowercase();
    LOADED.with(|l| {
        let mut l = l.borrow_mut();
        if !l.contains_key(&key) {
            // (Windows resolves a bare name to name.dll; elsewhere only a
            // `.dylib` / `.so` named in full gets here: `dll_call`)
            let name = lib.trim_matches('"');
            // SAFETY: loading a library runs its initialisers — what the
            // program asked for with DECLARE … LIB (the author's power, as
            // in RapidQ; the web has no ffi at all).
            let library = unsafe { Library::new(name) }.map_err(|e| library_error(name, &e))?;
            l.insert(key.clone(), library);
        }
        f(l.get(&key).expect("loaded"))
    })
}

/// A marshalled argument: an integer slot or a float slot.
enum Slot {
    Int(i64),
    Float(f64),
}

/// The bytes of a BYREF number the program passed as a value (not an
/// address): the DLL gets a buffer holding it.
fn temp_number(v: &Value, type_name: &str) -> Vec<u8> {
    let mut b = Vec::new();
    memory::encode(v, &memory::Kind::of(type_name), &mut b);
    b.resize(dll::numeric_size(type_name).max(b.len()).max(8), 0);
    b.resize(b.len() + TEMP_SLACK, 0);
    b
}

/// Zeros after each buffer of a call's own (a string given by value, a
/// BYREF number given as a value): an API told a bigger size than the text
/// it was given (`GetWindowsDirectory("", 260)`) writes into them, not past
/// the buffer. The program's own variables go through their blocks, whose
/// pages have room of their own (`memory::reserve_for`).
const TEMP_SLACK: usize = 4096;

/// The longest C string read from an address a DLL returned (`AS STRING`
/// results): a pointer to text with no NUL stops there.
const MAX_RESULT_STRING: usize = 1 << 20;

/// Calls `name` (its exported name) of `lib` with `args` as `spec` says.
pub fn dll_call(lib: &str, name: &str, spec: &str, args: &[Value]) -> Result<Value, String> {
    // Windows' DLLs (and any other DLL a RapidQ program names) on another
    // system: the error that names the function. Only a library of the
    // system's own format, named in full, loads there (RapidR's addition).
    if !cfg!(windows) && !dll::is_unix_library(lib) {
        return Err(dll::needs_windows_error(lib, name, false));
    }
    let spec = Spec::parse(spec);
    // (every parameter gets its argument: a missing one would leave the
    // callee reading a register or stack slot nobody set)
    if args.len() != spec.params.len() {
        let n = spec.params.len();
        return Err(format!("'{name}' takes {n} argument{}, {} given (its DECLARE)", if n == 1 { "" } else { "s" }, args.len()));
    }
    if args.len() > 16 {
        return Err(format!("'{name}': RapidR calls DLL functions of up to 16 arguments; this one has {}", args.len()));
    }
    // (a SUB or FUNCTION handed over as a callback: refused before the DLL
    // could jump to it)
    for a in args {
        if let Value::String(s) = a {
            if let Some(routine) = s.strip_prefix(dll::CALLBACK_MARKER) {
                return Err(dll::callback_error(name, routine));
            }
        }
    }
    for (p, a) in spec.params.iter().zip(args) {
        if p.is_float() && p.type_name != "DOUBLE" {
            return Err(format!("'{name}': a {} passed by value isn't supported; declare the parameter AS DOUBLE", p.type_name));
        }
        if p.is_float() && args.len() > 4 {
            return Err(format!("'{name}': DOUBLE arguments are supported in calls of up to 4 arguments"));
        }
        if matches!(a, Value::String(_)) && p.is_float() {
            return Err(format!("'{name}': a string was given for the DOUBLE parameter"));
        }
    }

    // Addresses of the TYPEs and arrays passed (their blocks exist before
    // the blocks are materialised).
    let mut addresses: Vec<Option<i64>> = Vec::with_capacity(args.len());
    for a in args {
        addresses.push(match a {
            Value::Object(_) | Value::Array(_) => Some(memory::address_of(a)?),
            _ => None,
        });
    }
    let blocks = memory::materialize();
    // Buffers of this call's own: C strings, BYREF numbers given as values.
    let mut temps: Vec<Vec<u8>> = Vec::new();
    let mut slots: Vec<Slot> = Vec::with_capacity(args.len());
    for (i, a) in args.iter().enumerate() {
        let p = &spec.params[i];
        let slot = match a {
            Value::String(s) => {
                let mut b = string_to_bytes(s);
                b.push(0);
                b.resize(b.len() + TEMP_SLACK, 0);
                temps.push(b);
                Slot::Int(temps.last_mut().unwrap().as_mut_ptr() as i64)
            }
            Value::Object(_) | Value::Array(_) => {
                let addr = addresses[i].unwrap_or(0);
                Slot::Int(memory::real_pointer(addr, &blocks).ok_or_else(|| format!("'{name}': the TYPE passed for '{}' has no memory", p.type_name))? as i64)
            }
            _ if p.is_float() => Slot::Float(a.to_f64()),
            _ => {
                let v = a.to_i64();
                if let Some(ptr) = memory::real_pointer(v, &blocks) {
                    Slot::Int(ptr as i64)
                } else if p.by_ref || (p.is_string() && v != 0) {
                    // (a BYREF number given as a value: a buffer with it;
                    // a STRING parameter given a number that isn't an
                    // address: the same, so the DLL can't read address 64)
                    temps.push(temp_number(a, &p.type_name));
                    Slot::Int(temps.last_mut().unwrap().as_mut_ptr() as i64)
                } else if let Some(real) = pointer_of_handle(v) {
                    // (a 64-bit pointer a DLL returned earlier, which the
                    // program holds as its 32-bit stand-in)
                    Slot::Int(real)
                } else {
                    Slot::Int(narrow(v, p))
                }
            }
        };
        slots.push(slot);
    }

    let raw = with_library(lib, |library| {
        // SAFETY: looking a symbol up only reads the library's export
        // table; the address is used as a function by `call_table` below.
        let sym: libloading::Symbol<*const ()> = unsafe { library.get(name.as_bytes()) }.map_err(|_| {
            format!("{} has no function '{name}' (the DECLARE's ALIAS must be the exported name; many Windows functions end in A or W)", dll::library_base(lib) + ".dll")
        })?;
        let ptr = *sym;
        let what = format!("{name} in {}", dll::library_base(lib));
        *LAST_CALL.lock().unwrap_or_else(|e| e.into_inner()) = what.clone();
        *CALLING.lock().unwrap_or_else(|e| e.into_inner()) = what;
        install_crash_filter();
        // SAFETY: a foreign function called with exactly the arguments its
        // DECLARE lists (the count checked above), each in the declared
        // type's slot: an integer register / stack slot, or a float one for
        // a DOUBLE. Whether the DECLARE matches the real function is the
        // program's responsibility, as in RapidQ (a wrong one is the
        // author's bug). Pointers in the slots are the program's own memory:
        // materialised blocks (kept for the process's life) or this call's
        // `temps` (alive until after the call). A crash inside is reported
        // by the crash filter, naming the call.
        let r = unsafe { call_table(ptr, &slots, spec.returns_float()) };
        #[cfg(windows)]
        let r = r.map(|raw| resource_dll_fallback(lib, name, args, raw));
        // (an AS STRING result is read while the call is still named, so an
        // address that isn't readable is reported as this call's crash)
        let r = r.map(|raw| match raw {
            Raw::Int(n) if spec.return_type == "STRING" && n != 0 => Raw::Text(read_c_string(n as usize)),
            other => other,
        });
        CALLING.lock().unwrap_or_else(|e| e.into_inner()).clear();
        r
    })?;
    // What the DLL wrote into the program's memory.
    memory::read_back(blocks);
    // (BYREF numbers given as values: nothing to write back to)
    drop(temps);

    Ok(match (raw, spec.return_type.as_str()) {
        (Raw::Float(d), _) => v_dbl(d),
        (Raw::Text(t), _) => v_str(&t),
        (Raw::Int(_), "") => v_null(),
        (Raw::Int(_), "STRING") => v_str(""),
        (Raw::Int(n), "INT64") => v_int(n),
        (Raw::Int(n), "BYTE") => v_int((n & 0xFF) as i64),
        (Raw::Int(n), "WORD") => v_int((n & 0xFFFF) as i64),
        (Raw::Int(n), "SHORT") => v_int(n as i16 as i64),
        // (a 32-bit result; a 64-bit pointer declared AS LONG — LoadLibrary,
        // GlobalAlloc, GetProcAddress — gets a 32-bit stand-in)
        (Raw::Int(n), _) => v_int(long_result(n)),
    })
}

/// 64-bit pointers DLLs returned for 32-bit results, by stand-in.
static POINTERS: std::sync::Mutex<Vec<i64>> = std::sync::Mutex::new(Vec::new());

/// The first stand-in (-1073741824): negative LONGs no API hands out as a
/// count or a flag, outside the program's own addresses (`memory`: 1 MB to
/// 2 GB) and below Windows' 32-bit handles.
const HANDLE_BASE: i64 = 0xC000_0000u32 as i32 as i64;
const MAX_HANDLES: usize = 1 << 24;

/// A DLL's result as RapidQ's 32-bit LONG: the value, when it is one (a
/// number, a handle — Windows keeps USER / GDI / kernel handles within 32
/// bits); a 64-bit pointer becomes a stand-in that turns back into the
/// pointer when the program hands it to a DLL again.
fn long_result(n: i64) -> i64 {
    let low = n as i32 as i64;
    if low == n || (n >> 32) == 0 {
        return low;
    }
    let mut p = POINTERS.lock().unwrap_or_else(|e| e.into_inner());
    let i = match p.iter().position(|&q| q == n) {
        Some(i) => i,
        None if p.len() < MAX_HANDLES => {
            p.push(n);
            p.len() - 1
        }
        None => return low,
    };
    HANDLE_BASE + i as i64
}

/// The pointer a stand-in from [`long_result`] stands for.
fn pointer_of_handle(v: i64) -> Option<i64> {
    let i = usize::try_from(v - HANDLE_BASE).ok()?;
    POINTERS.lock().unwrap_or_else(|e| e.into_inner()).get(i).copied()
}

/// `LoadLibrary` of a 32-bit DLL (one shipped with an old program) fails in
/// a 64-bit program. When it only holds resources — cursors, icons,
/// bitmaps, as RapidQ's cursor examples' CURSORS.DLL — it opens for them
/// (`LoadLibraryEx` as an image resource), so `LoadCursor(hInst, …)` works;
/// its code can't run (GetProcAddress finds nothing).
#[cfg(windows)]
fn resource_dll_fallback(lib: &str, name: &str, args: &[Value], raw: Raw) -> Raw {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_BAD_EXE_FORMAT};
    use windows_sys::Win32::System::LibraryLoader::{LoadLibraryExW, LOAD_LIBRARY_AS_DATAFILE, LOAD_LIBRARY_AS_IMAGE_RESOURCE};
    if !matches!(raw, Raw::Int(0)) || dll::library_base(lib) != "kernel32" || !matches!(name.to_ascii_lowercase().as_str(), "loadlibrarya" | "loadlibraryw") {
        return raw;
    }
    // SAFETY: reads the calling thread's last-error value.
    if unsafe { GetLastError() } != ERROR_BAD_EXE_FORMAT {
        return raw;
    }
    let Some(path) = args.first().map(Value::to_string_val) else { return raw };
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: a NUL-terminated UTF-16 path that lives across the call; the
    // flags map the file's resources only, running none of its code.
    let h = unsafe { LoadLibraryExW(wide.as_ptr(), 0, LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE) };
    Raw::Int(h as i64)
}

/// A by-value number as its declared size (the callee reads that many
/// bits; a LONG's sign extends so -1 is `HWND_TOPMOST` and `INFINITE`).
fn narrow(v: i64, p: &Param) -> i64 {
    match p.type_name.as_str() {
        "BYTE" => v & 0xFF,
        "WORD" => v & 0xFFFF,
        "SHORT" => v as i16 as i64,
        _ => v,
    }
}

enum Raw {
    Int(i64),
    Float(f64),
    /// An `AS STRING` result's text.
    Text(String),
}

/// The text at `addr` (a DLL's `AS STRING` result) up to its NUL, at most
/// [`MAX_RESULT_STRING`] bytes.
fn read_c_string(addr: usize) -> String {
    let mut bytes = Vec::new();
    for i in 0..MAX_RESULT_STRING {
        // SAFETY: the DLL returned this as a C string's address (the
        // DECLARE says AS STRING); it is read a byte at a time up to its
        // NUL and never past MAX_RESULT_STRING bytes. An address that isn't
        // readable faults, and the crash filter names the call.
        let b = unsafe { std::ptr::read_volatile((addr + i) as *const u8) };
        if b == 0 {
            break;
        }
        bytes.push(b);
    }
    crate::value::objects::codec::bytes_to_string(&bytes)
}

macro_rules! slot_type {
    (I) => { i64 };
    (F) => { f64 };
}

macro_rules! slot_value {
    ($slots:expr, I, $i:expr) => {
        match $slots[$i] { Slot::Int(n) => n, Slot::Float(f) => f.to_bits() as i64 }
    };
    ($slots:expr, F, $i:expr) => {
        match $slots[$i] { Slot::Float(f) => f, Slot::Int(n) => n as f64 }
    };
}

/// Calls `ptr` as `fn(types…) -> ret` with the slots named.
macro_rules! call_as {
    ($ptr:expr, $slots:expr, $ret:ty; $($k:ident $i:expr),*) => {{
        // SAFETY: (`call_table`'s contract) `$ptr` is an exported function's
        // address and this is the signature the slots describe; a function
        // pointer and a data pointer have the same size on every target.
        let f: unsafe extern "system" fn($(slot_type!($k)),*) -> $ret = unsafe { std::mem::transmute($ptr) };
        // SAFETY: as above — the call the DECLARE describes.
        unsafe { f($(slot_value!($slots, $k, $i)),*) }
    }};
}

/// The all-integer signatures (0 to 16 arguments) for one result type.
macro_rules! int_table {
    ($ptr:expr, $s:expr, $ret:ty) => {
        match $s.len() {
            0 => call_as!($ptr, $s, $ret;),
            1 => call_as!($ptr, $s, $ret; I 0),
            2 => call_as!($ptr, $s, $ret; I 0, I 1),
            3 => call_as!($ptr, $s, $ret; I 0, I 1, I 2),
            4 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3),
            5 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4),
            6 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5),
            7 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6),
            8 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6, I 7),
            9 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6, I 7, I 8),
            10 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6, I 7, I 8, I 9),
            11 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10),
            12 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11),
            13 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11, I 12),
            14 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11, I 12, I 13),
            15 => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11, I 12, I 13, I 14),
            _ => call_as!($ptr, $s, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11, I 12, I 13, I 14, I 15),
        }
    };
}

/// The signatures with DOUBLE arguments (up to 4 arguments; each slot an
/// integer or a float register) for one result type.
macro_rules! mixed_table {
    ($ptr:expr, $s:expr, $ret:ty) => {{
        let f = |i: usize| matches!($s.get(i), Some(Slot::Float(_)));
        match ($s.len(), f(0), f(1), f(2), f(3)) {
            (1, true, _, _, _) => call_as!($ptr, $s, $ret; F 0),
            (2, false, true, _, _) => call_as!($ptr, $s, $ret; I 0, F 1),
            (2, true, false, _, _) => call_as!($ptr, $s, $ret; F 0, I 1),
            (2, true, true, _, _) => call_as!($ptr, $s, $ret; F 0, F 1),
            (3, false, false, true, _) => call_as!($ptr, $s, $ret; I 0, I 1, F 2),
            (3, false, true, false, _) => call_as!($ptr, $s, $ret; I 0, F 1, I 2),
            (3, false, true, true, _) => call_as!($ptr, $s, $ret; I 0, F 1, F 2),
            (3, true, false, false, _) => call_as!($ptr, $s, $ret; F 0, I 1, I 2),
            (3, true, false, true, _) => call_as!($ptr, $s, $ret; F 0, I 1, F 2),
            (3, true, true, false, _) => call_as!($ptr, $s, $ret; F 0, F 1, I 2),
            (3, true, true, true, _) => call_as!($ptr, $s, $ret; F 0, F 1, F 2),
            (4, false, false, false, true) => call_as!($ptr, $s, $ret; I 0, I 1, I 2, F 3),
            (4, false, false, true, false) => call_as!($ptr, $s, $ret; I 0, I 1, F 2, I 3),
            (4, false, false, true, true) => call_as!($ptr, $s, $ret; I 0, I 1, F 2, F 3),
            (4, false, true, false, false) => call_as!($ptr, $s, $ret; I 0, F 1, I 2, I 3),
            (4, false, true, false, true) => call_as!($ptr, $s, $ret; I 0, F 1, I 2, F 3),
            (4, false, true, true, false) => call_as!($ptr, $s, $ret; I 0, F 1, F 2, I 3),
            (4, false, true, true, true) => call_as!($ptr, $s, $ret; I 0, F 1, F 2, F 3),
            (4, true, false, false, false) => call_as!($ptr, $s, $ret; F 0, I 1, I 2, I 3),
            (4, true, false, false, true) => call_as!($ptr, $s, $ret; F 0, I 1, I 2, F 3),
            (4, true, false, true, false) => call_as!($ptr, $s, $ret; F 0, I 1, F 2, I 3),
            (4, true, false, true, true) => call_as!($ptr, $s, $ret; F 0, I 1, F 2, F 3),
            (4, true, true, false, false) => call_as!($ptr, $s, $ret; F 0, F 1, I 2, I 3),
            (4, true, true, false, true) => call_as!($ptr, $s, $ret; F 0, F 1, I 2, F 3),
            (4, true, true, true, false) => call_as!($ptr, $s, $ret; F 0, F 1, F 2, I 3),
            (4, true, true, true, true) => call_as!($ptr, $s, $ret; F 0, F 1, F 2, F 3),
            _ => int_table!($ptr, $s, $ret),
        }
    }};
}

/// Calls `ptr` with `slots` and reads the result from the integer or the
/// float register.
///
/// # Safety
/// `ptr` must be a function of the convention and arity `slots` describes.
unsafe fn call_table(ptr: *const (), slots: &[Slot], float_result: bool) -> Result<Raw, String> {
    let mixed = slots.iter().any(|s| matches!(s, Slot::Float(_)));
    Ok(match (mixed, float_result) {
        (false, false) => Raw::Int(int_table!(ptr, slots, i64)),
        (false, true) => Raw::Float(int_table!(ptr, slots, f64)),
        (true, false) => Raw::Int(mixed_table!(ptr, slots, i64)),
        (true, true) => Raw::Float(mixed_table!(ptr, slots, f64)),
    })
}

/// A crash inside a DLL (a pointer that wasn't one) ends the program with
/// a message naming the call, instead of vanishing.
#[cfg(windows)]
fn install_crash_filter() {
    use std::sync::Once;
    use windows_sys::Win32::System::Diagnostics::Debug::{SetUnhandledExceptionFilter, EXCEPTION_POINTERS};
    static ONCE: Once = Once::new();
    unsafe extern "system" fn filter(info: *const EXCEPTION_POINTERS) -> i32 {
        let what = CALLING.lock().map(|c| c.clone()).unwrap_or_default();
        let last = LAST_CALL.lock().map(|c| c.clone()).unwrap_or_default();
        if what.is_empty() && last.is_empty() {
            // (no DLL called yet: Windows' own handling)
            return 0;
        }
        // SAFETY: Windows hands the filter a valid EXCEPTION_POINTERS (or
        // null, which `as_ref` turns into None) for the exception's time.
        let detail = unsafe { info.as_ref().and_then(|i| i.ExceptionRecord.as_ref()) }.map(|r| {
            let code = r.ExceptionCode as u32;
            let name = match code {
                0xC000_0005 => "access violation".to_string(),
                0xC000_001D => "illegal instruction".to_string(),
                0xC000_008C | 0xC000_008E | 0xC000_0094 => "arithmetic fault".to_string(),
                0xC000_00FD => "stack overflow".to_string(),
                c => format!("exception {c:#010X}"),
            };
            if code == 0xC000_0005 && r.NumberParameters >= 2 {
                let rw = if r.ExceptionInformation[0] == 0 { "reading" } else { "writing" };
                format!("{name} {rw} address {:#x}", r.ExceptionInformation[1])
            } else {
                name
            }
        });
        let detail = detail.unwrap_or_default();
        if what.is_empty() {
            // (after a call: a DLL kept a pointer or a callback and used it
            // later, e.g. a window procedure set with SetWindowLong)
            eprintln!("run-time error: the program crashed ({detail}) after calling DLL functions (the last: {last}); a DLL kept a pointer or a callback that wasn't valid");
        } else {
            eprintln!("run-time error: the call to {what} crashed ({detail}); an argument wasn't the pointer or handle the function expects");
        }
        1 // EXCEPTION_EXECUTE_HANDLER: the process ends
    }
    ONCE.call_once(|| {
        // SAFETY: installing a process-wide filter; it runs only when an
        // exception reaches the top.
        unsafe { SetUnhandledExceptionFilter(Some(filter)) };
    });
}

#[cfg(not(windows))]
fn install_crash_filter() {}

/// `__dll_call(lib, alias, spec, args…)` in a native build.
pub fn rp_dll_call(args: &[Value]) -> Value {
    let s = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    match dll_call(&s(0), &s(1), &s(2), args.get(3..).unwrap_or(&[])) {
        Ok(v) => v,
        Err(e) => crate::value::runtime_error(&e),
    }
}

/// Unload a library (`UNLOADLIBRARY`).
pub fn ffi_unload(lib: &str) {
    LOADED.with(|l| {
        l.borrow_mut().remove(&lib.trim_matches('"').to_ascii_lowercase());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_libraries_need_windows_elsewhere() {
        let r = dll_call("user32", "GetDC", "LONG|LONG:v", &[v_int(0)]);
        if cfg!(windows) {
            assert!(r.is_ok());
        } else {
            let e = r.unwrap_err();
            assert!(e.starts_with("'GetDC' is a Windows function (user32)"), "{e}");
        }
        let e = dll_call("nosuchlib", "F", "|", &[]).unwrap_err();
        assert!(e.contains("nosuchlib"), "{e}");
        if !cfg!(windows) {
            // Any DLL a RapidQ program names, not only Windows' own.
            assert_eq!(e, "'F' is a function of nosuchlib.dll, a Windows DLL: this program calls a DLL, so it runs on Windows only");
        }
    }

    #[test]
    fn narrowing_and_temps() {
        assert_eq!(narrow(-1, &Param { type_name: "BYTE".into(), by_ref: false }), 255);
        assert_eq!(narrow(-1, &Param { type_name: "WORD".into(), by_ref: false }), 65535);
        assert_eq!(narrow(70000, &Param { type_name: "SHORT".into(), by_ref: false }), 4464);
        assert_eq!(narrow(-1, &Param { type_name: "LONG".into(), by_ref: false }), -1);
        assert_eq!(&temp_number(&v_int(64), "LONG")[..4], &64i32.to_le_bytes());
        // 32-bit results stay themselves; a 64-bit pointer gets a stand-in
        // that turns back into it.
        assert_eq!((long_result(7), long_result(-1), long_result(0xFFFF_FFFF), long_result(0x8000_0001)), (7, -1, -1, -2147483647));
        let p = 0x7FF8_1234_0000_i64;
        let h = long_result(p);
        assert!(h < 0 && h >= HANDLE_BASE, "{h}");
        assert_eq!((long_result(p), pointer_of_handle(h)), (h, Some(p)));
        assert_eq!(pointer_of_handle(-1), None);
        assert_eq!(pointer_of_handle(12345), None);
    }

    /// The system's C library (every system has one): the table calls
    /// integer, pointer and double functions correctly.
    #[test]
    fn calls_the_c_library() {
        let lib = if cfg!(windows) { "msvcrt" } else if cfg!(target_os = "macos") { "libSystem.B.dylib" } else { "libc.so.6" };
        assert_eq!(dll_call(lib, "abs", "LONG|LONG:v", &[v_int(-7)]).unwrap(), v_int(7));
        assert_eq!(dll_call(lib, "floor", "DOUBLE|DOUBLE:v", &[v_dbl(2.7)]).unwrap(), v_dbl(2.0));
        assert_eq!(dll_call(lib, "strlen", "LONG|STRING:v", &[v_str("hello")]).unwrap(), v_int(5));
        // A program's own buffer filled by the library (strcpy into a
        // VARPTR'd string), read back into the variable.
        let dest = memory::varptr_var("main:s", &v_str("          "), "STRING").unwrap();
        dll_call(lib, "strcpy", "LONG|LONG:v,STRING:v", &[dest.clone(), v_str("abc")]).unwrap();
        assert_eq!(memory::sync("main:s", &v_str("          "), "STRING").to_string_val(), "abc\0      ");
        // A function the library doesn't have.
        let e = dll_call(lib, "no_such_function_xyz", "LONG|", &[]).unwrap_err();
        assert!(e.contains("no function 'no_such_function_xyz'"), "{e}");
        // An AS STRING result: the text at the address returned (here
        // inside this call's own copy of the argument).
        assert_eq!(dll_call(lib, "strstr", "STRING|STRING:v,STRING:v", &[v_str("hello"), v_str("ll")]).unwrap(), v_str("llo"));
        assert_eq!(dll_call(lib, "strstr", "STRING|STRING:v,STRING:v", &[v_str("hello"), v_str("xy")]).unwrap(), v_str(""));
        // Each parameter gets its argument, no more and no fewer.
        let e = dll_call(lib, "abs", "LONG|LONG:v", &[]).unwrap_err();
        assert_eq!(e, "'abs' takes 1 argument, 0 given (its DECLARE)");
        assert!(dll_call(lib, "abs", "LONG|LONG:v", &[v_int(1), v_int(2)]).is_err());
        // A SUB handed over as a callback is refused before the call.
        let e = dll_call(lib, "abs", "LONG|LONG:v", &[v_str(&format!("{}WndProc", dll::CALLBACK_MARKER))]).unwrap_err();
        assert!(e.starts_with("'abs' is given CODEPTR(WndProc), a callback"), "{e}");
    }
}
