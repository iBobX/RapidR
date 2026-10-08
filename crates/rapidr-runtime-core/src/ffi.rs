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
use std::ffi::{c_char, CStr};

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
            let name = lib.trim_matches('"');
            // Windows resolves a bare name to name.dll; the other systems
            // need the extension spelled out.
            let path = if cfg!(windows) || name.contains('.') { name.to_string() } else { format!("{name}.{}", if cfg!(target_os = "macos") { "dylib" } else { "so" }) };
            // SAFETY: loading a library runs its initialisers — what the
            // program asked for with DECLARE … LIB.
            let library = unsafe { Library::new(&path) }.map_err(|e| library_error(name, &e))?;
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
    b
}

/// Calls `name` (its exported name) of `lib` with `args` as `spec` says.
pub fn dll_call(lib: &str, name: &str, spec: &str, args: &[Value]) -> Result<Value, String> {
    if !cfg!(windows) && dll::is_windows_system_library(lib) {
        return Err(dll::needs_windows_error(lib, name, false));
    }
    let spec = Spec::parse(spec);
    if args.len() > spec.params.len() || args.len() > 16 {
        return Err(format!("'{name}' takes {} arguments, {} given", spec.params.len(), args.len()));
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
                } else {
                    Slot::Int(narrow(v, p))
                }
            }
        };
        slots.push(slot);
    }

    let raw = with_library(lib, |library| {
        // SAFETY: the symbol is called through the signature the DECLARE
        // describes; the pointer type is fixed by `call_table`.
        let sym: libloading::Symbol<*const ()> = unsafe { library.get(name.as_bytes()) }.map_err(|_| {
            format!("{} has no function '{name}' (the DECLARE's ALIAS must be the exported name; many Windows functions end in A or W)", dll::library_base(lib) + ".dll")
        })?;
        let ptr = *sym;
        *CALLING.lock().unwrap_or_else(|e| e.into_inner()) = format!("{name} in {}", dll::library_base(lib));
        install_crash_filter();
        // SAFETY: a foreign function with the arguments its declaration
        // asks for; the program takes RapidQ's risk that they are right.
        let r = unsafe { call_table(ptr, &slots, spec.returns_float()) };
        CALLING.lock().unwrap_or_else(|e| e.into_inner()).clear();
        r
    })?;
    // What the DLL wrote into the program's memory.
    memory::read_back(blocks);
    // (BYREF numbers given as values: nothing to write back to)
    drop(temps);

    Ok(match (raw, spec.return_type.as_str()) {
        (Raw::Float(d), _) => v_dbl(d),
        (Raw::Int(_), "") => v_null(),
        (Raw::Int(n), "STRING") => {
            if n == 0 {
                v_str("")
            } else {
                // SAFETY: the DLL returned a C string's address (as the
                // DECLARE says); read to its NUL.
                let c = unsafe { CStr::from_ptr(n as *const c_char) };
                v_str(&crate::value::objects::codec::bytes_to_string(c.to_bytes()))
            }
        }
        (Raw::Int(n), "INT64") => v_int(n),
        (Raw::Int(n), "BYTE") => v_int((n & 0xFF) as i64),
        (Raw::Int(n), "WORD") => v_int((n & 0xFFFF) as i64),
        (Raw::Int(n), "SHORT") => v_int(n as i16 as i64),
        // (a 32-bit result: the register's upper bits are undefined)
        (Raw::Int(n), _) => v_int(n as i32 as i64),
    })
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
        let f: unsafe extern "system" fn($(slot_type!($k)),*) -> $ret = std::mem::transmute($ptr);
        f($(slot_value!($slots, $k, $i)),*)
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
        if what.is_empty() {
            // (not during a DLL call: Windows' own handling)
            return 0;
        }
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
        eprintln!("run-time error: the call to {what} crashed ({}); an argument wasn't the pointer or handle the function expects", detail.unwrap_or_default());
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
    }

    #[test]
    fn narrowing_and_temps() {
        assert_eq!(narrow(-1, &Param { type_name: "BYTE".into(), by_ref: false }), 255);
        assert_eq!(narrow(-1, &Param { type_name: "WORD".into(), by_ref: false }), 65535);
        assert_eq!(narrow(70000, &Param { type_name: "SHORT".into(), by_ref: false }), 4464);
        assert_eq!(narrow(-1, &Param { type_name: "LONG".into(), by_ref: false }), -1);
        assert_eq!(&temp_number(&v_int(64), "LONG")[..4], &64i32.to_le_bytes());
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
    }
}
