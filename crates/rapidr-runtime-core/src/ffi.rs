//! FFI module — load shared libraries (.dylib on macOS, .dll on Windows, .so on Linux)
//! and call functions from them at runtime.
//!
//! Usage from RapidR:
//!   DECLARE FUNCTION MyFunc LIB "mylib.dylib" ALIAS "actual_func_name" (a AS INTEGER) AS INTEGER
//!   result = MyFunc(42)
//!
//! Calling convention notes:
//!   On x86_64 (SysV/Windows) and ARM64, integer and pointer arguments travel in integer
//!   registers (rdi/rsi/… or x0/x1/…) while floating-point (f64/f32) arguments travel in
//!   separate float registers (xmm0/xmm1/… or d0/d1/…).  The return value likewise lives in
//!   either an integer register (rax / x0) or a float register (xmm0 / d0) depending on the
//!   declared C return type.
//!
//!   To honour this ABI correctly, the Rust `extern "C"` function pointer type used for each
//!   call must exactly list `i64` for integer/pointer parameters and `f64` for double parameters
//!   (and the same for the return type).  The comprehensive dispatch matrix below handles all
//!   combinations of up to 4 parameters, each of which can be `i64` or `f64`, with either an
//!   `i64` or `f64` return type.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};

use libloading::{Library, Symbol};

use crate::value::{v_dbl, v_int, v_null, v_str, Value};

/// Cached loaded libraries (by path)
thread_local! {
    static LOADED_LIBS: RefCell<HashMap<String, Library>> = RefCell::new(HashMap::new());
}

/// Load a shared library if not already loaded
fn ensure_lib(lib_path: &str) -> Result<(), String> {
    LOADED_LIBS.with(|libs| {
        let mut map = libs.borrow_mut();
        if !map.contains_key(lib_path) {
            match unsafe { Library::new(lib_path) } {
                Ok(lib) => { map.insert(lib_path.to_string(), lib); Ok(()) }
                Err(e) => Err(format!("Failed to load library '{}': {}", lib_path, e))
            }
        } else {
            Ok(())
        }
    })
}

/// Raw result from the C call, before conversion to a RapidR Value.
enum RawResult {
    Int(i64),
    Dbl(f64),
}

/// Call an FFI function with the given arguments.
/// This is the generic entry point used by codegen-generated DECLARE stubs.
///
/// `lib_path`: path to the shared library (e.g., "libm.dylib")
/// `func_name`: the actual exported function name (the ALIAS)
/// `args`: the arguments from RapidR
/// `ret_type`: expected return type ("INTEGER", "DOUBLE", "STRING", or "")
pub fn ffi_call(lib_path: &str, func_name: &str, args: &[Value], ret_type: &str) -> Value {
    if let Err(e) = ensure_lib(lib_path) {
        eprintln!("[ERROR] FFI: {}", e);
        return v_null();
    }

    LOADED_LIBS.with(|libs| {
        let map = libs.borrow();
        let lib = match map.get(lib_path) {
            Some(l) => l,
            None => return v_null(),
        };

        let n = args.len();
        if n > 4 {
            eprintln!(
                "[WARN] FFI: function '{}' has {} args (max 4 supported; use RUSTSTART/RUSTEND for more)",
                func_name, n
            );
            return v_null();
        }

        // Keep CString values alive for the entire duration of the FFI call.
        let mut c_strings: Vec<CString> = Vec::new();

        // Prepare two parallel arrays: integer values (i64) and float values (f64).
        // `af[idx]` is true when args[idx] is a RapidR Double — those must be passed in
        // float registers, so we will use `f64` in the function-pointer type for that slot.
        let mut i_args = [0i64; 4];
        let mut f_args = [0f64; 4];
        let mut af = [false; 4];

        for (idx, arg) in args.iter().enumerate() {
            match arg {
                Value::Integer(v) => { i_args[idx] = *v; f_args[idx] = *v as f64; }
                Value::Double(v)  => { i_args[idx] = v.to_bits() as i64; f_args[idx] = *v; af[idx] = true; }
                Value::Boolean(v) => { let b = if *v { 1i64 } else { 0i64 }; i_args[idx] = b; f_args[idx] = b as f64; }
                Value::String(v)  => {
                    let cs = CString::new(v.as_str()).unwrap_or_default();
                    i_args[idx] = cs.as_ptr() as i64;
                    f_args[idx] = i_args[idx] as f64; // pointer as float (meaningless, but fills the slot)
                    c_strings.push(cs);
                }
                _ => {}
            }
        }

        // For ABI purposes, STRING return behaves like INTEGER (both use the integer return register).
        let abi_is_dbl = matches!(
            ret_type.to_ascii_uppercase().as_str(),
            "DOUBLE" | "SINGLE" | "CURRENCY"
        );

        // Convenience bindings so match arms are concise.
        let (i0, i1, i2, i3) = (i_args[0], i_args[1], i_args[2], i_args[3]);
        let (f0, f1, f2, f3) = (f_args[0], f_args[1], f_args[2], f_args[3]);
        let (a0, a1, a2, a3) = (af[0], af[1], af[2], af[3]);

        // Macro: look up the symbol and call it.  Returns Result<RawResult, String> so the
        // closure below can use `?` for clean error propagation.
        macro_rules! call_sym {
            // 0-arg form
            ($ft:ty, $res:path) => {{
                let s: Symbol<$ft> = unsafe { lib.get(func_name.as_bytes()) }
                    .map_err(|e| format!("symbol '{}' not found: {}", func_name, e))?;
                Ok($res(unsafe { s() }))
            }};
            // 1+-arg form
            ($ft:ty, $res:path, $($arg:expr),+) => {{
                let s: Symbol<$ft> = unsafe { lib.get(func_name.as_bytes()) }
                    .map_err(|e| format!("symbol '{}' not found: {}", func_name, e))?;
                Ok($res(unsafe { s($($arg),+) }))
            }};
        }

        // Comprehensive dispatch: (arg_count, a0_is_f64, a1_is_f64, a2_is_f64, a3_is_f64, return_is_f64)
        // Each arm uses the exact extern "C" fn signature so the compiler emits the correct
        // integer- vs. float-register usage for both parameters and return value.
        let raw_result: Result<RawResult, String> = (|| {
            match (n, a0, a1, a2, a3, abi_is_dbl) {
                // ── 0 arguments ──────────────────────────────────────────────────────────────
                (0, _, _, _, _, false) => call_sym!(unsafe extern "C" fn() -> i64, RawResult::Int),
                (0, _, _, _, _, true)  => call_sym!(unsafe extern "C" fn() -> f64, RawResult::Dbl),

                // ── 1 argument ───────────────────────────────────────────────────────────────
                (1, false, _, _, _, false) => call_sym!(unsafe extern "C" fn(i64) -> i64, RawResult::Int, i0),
                (1, false, _, _, _, true)  => call_sym!(unsafe extern "C" fn(i64) -> f64, RawResult::Dbl, i0),
                (1, true,  _, _, _, false) => call_sym!(unsafe extern "C" fn(f64) -> i64, RawResult::Int, f0),
                (1, true,  _, _, _, true)  => call_sym!(unsafe extern "C" fn(f64) -> f64, RawResult::Dbl, f0),

                // ── 2 arguments ──────────────────────────────────────────────────────────────
                (2, false, false, _, _, false) => call_sym!(unsafe extern "C" fn(i64, i64) -> i64, RawResult::Int, i0, i1),
                (2, false, false, _, _, true)  => call_sym!(unsafe extern "C" fn(i64, i64) -> f64, RawResult::Dbl, i0, i1),
                (2, false, true,  _, _, false) => call_sym!(unsafe extern "C" fn(i64, f64) -> i64, RawResult::Int, i0, f1),
                (2, false, true,  _, _, true)  => call_sym!(unsafe extern "C" fn(i64, f64) -> f64, RawResult::Dbl, i0, f1),
                (2, true,  false, _, _, false) => call_sym!(unsafe extern "C" fn(f64, i64) -> i64, RawResult::Int, f0, i1),
                (2, true,  false, _, _, true)  => call_sym!(unsafe extern "C" fn(f64, i64) -> f64, RawResult::Dbl, f0, i1),
                (2, true,  true,  _, _, false) => call_sym!(unsafe extern "C" fn(f64, f64) -> i64, RawResult::Int, f0, f1),
                (2, true,  true,  _, _, true)  => call_sym!(unsafe extern "C" fn(f64, f64) -> f64, RawResult::Dbl, f0, f1),

                // ── 3 arguments ──────────────────────────────────────────────────────────────
                (3, false, false, false, _, false) => call_sym!(unsafe extern "C" fn(i64, i64, i64) -> i64, RawResult::Int, i0, i1, i2),
                (3, false, false, false, _, true)  => call_sym!(unsafe extern "C" fn(i64, i64, i64) -> f64, RawResult::Dbl, i0, i1, i2),
                (3, false, false, true,  _, false) => call_sym!(unsafe extern "C" fn(i64, i64, f64) -> i64, RawResult::Int, i0, i1, f2),
                (3, false, false, true,  _, true)  => call_sym!(unsafe extern "C" fn(i64, i64, f64) -> f64, RawResult::Dbl, i0, i1, f2),
                (3, false, true,  false, _, false) => call_sym!(unsafe extern "C" fn(i64, f64, i64) -> i64, RawResult::Int, i0, f1, i2),
                (3, false, true,  false, _, true)  => call_sym!(unsafe extern "C" fn(i64, f64, i64) -> f64, RawResult::Dbl, i0, f1, i2),
                (3, false, true,  true,  _, false) => call_sym!(unsafe extern "C" fn(i64, f64, f64) -> i64, RawResult::Int, i0, f1, f2),
                (3, false, true,  true,  _, true)  => call_sym!(unsafe extern "C" fn(i64, f64, f64) -> f64, RawResult::Dbl, i0, f1, f2),
                (3, true,  false, false, _, false) => call_sym!(unsafe extern "C" fn(f64, i64, i64) -> i64, RawResult::Int, f0, i1, i2),
                (3, true,  false, false, _, true)  => call_sym!(unsafe extern "C" fn(f64, i64, i64) -> f64, RawResult::Dbl, f0, i1, i2),
                (3, true,  false, true,  _, false) => call_sym!(unsafe extern "C" fn(f64, i64, f64) -> i64, RawResult::Int, f0, i1, f2),
                (3, true,  false, true,  _, true)  => call_sym!(unsafe extern "C" fn(f64, i64, f64) -> f64, RawResult::Dbl, f0, i1, f2),
                (3, true,  true,  false, _, false) => call_sym!(unsafe extern "C" fn(f64, f64, i64) -> i64, RawResult::Int, f0, f1, i2),
                (3, true,  true,  false, _, true)  => call_sym!(unsafe extern "C" fn(f64, f64, i64) -> f64, RawResult::Dbl, f0, f1, i2),
                (3, true,  true,  true,  _, false) => call_sym!(unsafe extern "C" fn(f64, f64, f64) -> i64, RawResult::Int, f0, f1, f2),
                (3, true,  true,  true,  _, true)  => call_sym!(unsafe extern "C" fn(f64, f64, f64) -> f64, RawResult::Dbl, f0, f1, f2),

                // ── 4 arguments ──────────────────────────────────────────────────────────────
                (4, false, false, false, false, false) => call_sym!(unsafe extern "C" fn(i64, i64, i64, i64) -> i64, RawResult::Int, i0, i1, i2, i3),
                (4, false, false, false, false, true)  => call_sym!(unsafe extern "C" fn(i64, i64, i64, i64) -> f64, RawResult::Dbl, i0, i1, i2, i3),
                (4, false, false, false, true,  false) => call_sym!(unsafe extern "C" fn(i64, i64, i64, f64) -> i64, RawResult::Int, i0, i1, i2, f3),
                (4, false, false, false, true,  true)  => call_sym!(unsafe extern "C" fn(i64, i64, i64, f64) -> f64, RawResult::Dbl, i0, i1, i2, f3),
                (4, false, false, true,  false, false) => call_sym!(unsafe extern "C" fn(i64, i64, f64, i64) -> i64, RawResult::Int, i0, i1, f2, i3),
                (4, false, false, true,  false, true)  => call_sym!(unsafe extern "C" fn(i64, i64, f64, i64) -> f64, RawResult::Dbl, i0, i1, f2, i3),
                (4, false, false, true,  true,  false) => call_sym!(unsafe extern "C" fn(i64, i64, f64, f64) -> i64, RawResult::Int, i0, i1, f2, f3),
                (4, false, false, true,  true,  true)  => call_sym!(unsafe extern "C" fn(i64, i64, f64, f64) -> f64, RawResult::Dbl, i0, i1, f2, f3),
                (4, false, true,  false, false, false) => call_sym!(unsafe extern "C" fn(i64, f64, i64, i64) -> i64, RawResult::Int, i0, f1, i2, i3),
                (4, false, true,  false, false, true)  => call_sym!(unsafe extern "C" fn(i64, f64, i64, i64) -> f64, RawResult::Dbl, i0, f1, i2, i3),
                (4, false, true,  false, true,  false) => call_sym!(unsafe extern "C" fn(i64, f64, i64, f64) -> i64, RawResult::Int, i0, f1, i2, f3),
                (4, false, true,  false, true,  true)  => call_sym!(unsafe extern "C" fn(i64, f64, i64, f64) -> f64, RawResult::Dbl, i0, f1, i2, f3),
                (4, false, true,  true,  false, false) => call_sym!(unsafe extern "C" fn(i64, f64, f64, i64) -> i64, RawResult::Int, i0, f1, f2, i3),
                (4, false, true,  true,  false, true)  => call_sym!(unsafe extern "C" fn(i64, f64, f64, i64) -> f64, RawResult::Dbl, i0, f1, f2, i3),
                (4, false, true,  true,  true,  false) => call_sym!(unsafe extern "C" fn(i64, f64, f64, f64) -> i64, RawResult::Int, i0, f1, f2, f3),
                (4, false, true,  true,  true,  true)  => call_sym!(unsafe extern "C" fn(i64, f64, f64, f64) -> f64, RawResult::Dbl, i0, f1, f2, f3),
                (4, true,  false, false, false, false) => call_sym!(unsafe extern "C" fn(f64, i64, i64, i64) -> i64, RawResult::Int, f0, i1, i2, i3),
                (4, true,  false, false, false, true)  => call_sym!(unsafe extern "C" fn(f64, i64, i64, i64) -> f64, RawResult::Dbl, f0, i1, i2, i3),
                (4, true,  false, false, true,  false) => call_sym!(unsafe extern "C" fn(f64, i64, i64, f64) -> i64, RawResult::Int, f0, i1, i2, f3),
                (4, true,  false, false, true,  true)  => call_sym!(unsafe extern "C" fn(f64, i64, i64, f64) -> f64, RawResult::Dbl, f0, i1, i2, f3),
                (4, true,  false, true,  false, false) => call_sym!(unsafe extern "C" fn(f64, i64, f64, i64) -> i64, RawResult::Int, f0, i1, f2, i3),
                (4, true,  false, true,  false, true)  => call_sym!(unsafe extern "C" fn(f64, i64, f64, i64) -> f64, RawResult::Dbl, f0, i1, f2, i3),
                (4, true,  false, true,  true,  false) => call_sym!(unsafe extern "C" fn(f64, i64, f64, f64) -> i64, RawResult::Int, f0, i1, f2, f3),
                (4, true,  false, true,  true,  true)  => call_sym!(unsafe extern "C" fn(f64, i64, f64, f64) -> f64, RawResult::Dbl, f0, i1, f2, f3),
                (4, true,  true,  false, false, false) => call_sym!(unsafe extern "C" fn(f64, f64, i64, i64) -> i64, RawResult::Int, f0, f1, i2, i3),
                (4, true,  true,  false, false, true)  => call_sym!(unsafe extern "C" fn(f64, f64, i64, i64) -> f64, RawResult::Dbl, f0, f1, i2, i3),
                (4, true,  true,  false, true,  false) => call_sym!(unsafe extern "C" fn(f64, f64, i64, f64) -> i64, RawResult::Int, f0, f1, i2, f3),
                (4, true,  true,  false, true,  true)  => call_sym!(unsafe extern "C" fn(f64, f64, i64, f64) -> f64, RawResult::Dbl, f0, f1, i2, f3),
                (4, true,  true,  true,  false, false) => call_sym!(unsafe extern "C" fn(f64, f64, f64, i64) -> i64, RawResult::Int, f0, f1, f2, i3),
                (4, true,  true,  true,  false, true)  => call_sym!(unsafe extern "C" fn(f64, f64, f64, i64) -> f64, RawResult::Dbl, f0, f1, f2, i3),
                (4, true,  true,  true,  true,  false) => call_sym!(unsafe extern "C" fn(f64, f64, f64, f64) -> i64, RawResult::Int, f0, f1, f2, f3),
                (4, true,  true,  true,  true,  true)  => call_sym!(unsafe extern "C" fn(f64, f64, f64, f64) -> f64, RawResult::Dbl, f0, f1, f2, f3),

                _ => Err(format!("FFI: unsupported arg count {} for '{}'", n, func_name)),
            }
        })();

        // c_strings is still live here; drop happens after this point.
        drop(c_strings);

        match raw_result {
            Err(e) => { eprintln!("[ERROR] {}", e); v_null() }
            Ok(raw) => {
                match ret_type.to_ascii_uppercase().as_str() {
                    "INTEGER" | "LONG" | "INT64" | "DWORD" | "WORD" | "BYTE" => {
                        v_int(match raw { RawResult::Int(n) => n, RawResult::Dbl(d) => d as i64 })
                    }
                    "DOUBLE" | "SINGLE" | "CURRENCY" => {
                        v_dbl(match raw { RawResult::Dbl(d) => d, RawResult::Int(n) => n as f64 })
                    }
                    "STRING" => {
                        let ptr = match raw { RawResult::Int(n) => n, RawResult::Dbl(d) => d as i64 };
                        if ptr == 0 {
                            v_str("".into())
                        } else {
                            let cstr = unsafe { CStr::from_ptr(ptr as *const c_char) };
                            v_str(&cstr.to_string_lossy())
                        }
                    }
                    _ => match raw { RawResult::Int(n) => v_int(n), RawResult::Dbl(d) => v_dbl(d) },
                }
            }
        }
    })
}

/// Unload a specific library
pub fn ffi_unload(lib_path: &str) {
    LOADED_LIBS.with(|libs| {
        libs.borrow_mut().remove(lib_path);
    });
}
