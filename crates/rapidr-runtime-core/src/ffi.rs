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
//! 16 arguments, of which up to 8 DOUBLE / SINGLE — no C library, no
//! assembler.

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

/// Why `lib` didn't load, in words: the DLL file isn't there, or it is a
/// 32-bit one (most DLLs shipped with RapidQ programs), which a 64-bit
/// program can't load.
fn library_error(lib: &str, e: &libloading::Error) -> String {
    use std::error::Error as _;
    let mut text = e.to_string();
    if let Some(s) = e.source() {
        text = format!("{text}: {s}");
    }
    if let Some(m) = dll_file(lib).and_then(|p| pe_machine(&p)) {
        if m == PE_I386 {
            return format!("'{lib}' is a 32-bit DLL; RapidR programs are 64-bit, so Windows can't load it (a 64-bit build of that DLL is needed)");
        }
    }
    if cfg!(windows) && (text.contains("os error 126") || dll_file(lib).is_none()) {
        return format!("can't find the DLL '{lib}': it isn't one of Windows' and isn't beside the program");
    }
    format!("can't load the library '{lib}': {text}")
}

/// The PE machine of a 32-bit x86 DLL.
const PE_I386: u16 = 0x014C;

/// The file `LIB "…"` names, where Windows looks for a DLL of the
/// program's: as written (a path), else in the current folder or beside
/// the program, with `.dll` added when it has no extension.
fn dll_file(lib: &str) -> Option<std::path::PathBuf> {
    let name = lib.trim_matches('"');
    let mut names = vec![name.to_string()];
    if std::path::Path::new(name).extension().is_none() {
        names.push(format!("{name}.dll"));
    }
    let mut dirs = vec![std::path::PathBuf::new()];
    if let Some(d) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        dirs.push(d);
    }
    dirs.iter().flat_map(|d| names.iter().map(move |n| d.join(n))).find(|p| p.is_file())
}

/// A PE file's machine field (`0x14C` x86, `0x8664` x64, `0xAA64` ARM64).
fn pe_machine(path: &std::path::Path) -> Option<u16> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let mut mz = [0u8; 0x40];
    f.read_exact(&mut mz).ok()?;
    if &mz[..2] != b"MZ" {
        return None;
    }
    let pe = u32::from_le_bytes([mz[0x3C], mz[0x3D], mz[0x3E], mz[0x3F]]);
    f.seek(SeekFrom::Start(pe as u64)).ok()?;
    let mut head = [0u8; 6];
    f.read_exact(&mut head).ok()?;
    (&head[..4] == b"PE\0\0").then(|| u16::from_le_bytes([head[4], head[5]]))
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
    // A sandboxed run (what RapidR starts for code it didn't get from the
    // user's own hands — docs/security-audit.md SEC-19 / SEC-10) calls no
    // DLL at all.
    if sandboxed() {
        return Err(dll::sandboxed_error(name));
    }
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
        if p.is_float() && !matches!(p.type_name.as_str(), "DOUBLE" | "SINGLE") {
            return Err(format!("'{name}': a {} passed by value isn't supported; declare the parameter AS DOUBLE", p.type_name));
        }
        if matches!(a, Value::String(_)) && p.is_float() {
            return Err(format!("'{name}': a string was given for the {} parameter", p.type_name));
        }
    }
    // CallWindowProc given the program's own memory as the procedure: x86
    // machine code a RapidQ program wrote into a string or a buffer (its
    // way to run assembler), which no 64-bit program can run.
    if matches!(name.to_ascii_lowercase().as_str(), "callwindowproca" | "callwindowprocw") {
        if let Some(a) = args.first() {
            if matches!(a, Value::String(_)) || memory::bytes_from(a.to_i64()).is_ok() {
                return Err(format!("'{name}' is given the program's own memory to run: machine code for 32-bit x86 processors (RapidQ's way to run assembler), which a 64-bit program can't run"));
            }
        }
    }
    let floats = spec.params.iter().filter(|p| p.is_float()).count();
    if floats > MAX_FLOAT_ARGS {
        return Err(format!("'{name}': RapidR calls DLL functions of up to {MAX_FLOAT_ARGS} DOUBLE / SINGLE arguments; this one has {floats}"));
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
    // BYREF LONGs that are a variable of their own: where a DLL may write a
    // 64-bit pointer (a handle's or an object's out-parameter)
    let mut out_longs: Vec<usize> = Vec::new();
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
            // (a SINGLE: its 32 bits in the low half of the float slot,
            // which is all the callee reads of a float register or of its
            // 8-byte stack slot — on x64 and ARM64 alike)
            _ if p.is_float() && p.type_name == "SINGLE" => Slot::Float(f64::from_bits((a.to_f64() as f32).to_bits() as u64)),
            _ if p.is_float() => Slot::Float(a.to_f64()),
            _ => {
                let v = a.to_i64();
                if let Some(ptr) = memory::real_pointer(v, &blocks) {
                    if p.by_ref && p.is_numeric() && dll::numeric_size(&p.type_name) == 4 && memory::variable_len(v) == Some(4) && memory::room_at(v, &blocks) >= 8 {
                        out_longs.push(ptr);
                    }
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

    // The function's address, looked up with the library table borrowed
    // only for that: the call itself may lead back here (Windows sends the
    // program's windows messages while SetWindowPos runs, and a handler
    // may call a DLL), and the library stays loaded (only UNLOADLIBRARY
    // drops it, never inside a call of its own).
    let ptr = with_library(lib, |library| {
        // SAFETY: looking a symbol up only reads the library's export
        // table; the address is used as a function by `call_table` below.
        let sym: libloading::Symbol<*const ()> = unsafe { library.get(name.as_bytes()) }.map_err(|_| {
            format!("{} has no function '{name}' (the DECLARE's ALIAS must be the exported name; many Windows functions end in A or W)", dll::library_base(lib) + ".dll")
        })?;
        Ok(*sym)
    })?;
    let raw = {
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
        // (a SINGLE result is the low 32 bits of the float register)
        let r = r.map(|raw| match raw {
            Raw::Float(d) if spec.return_type == "SINGLE" => Raw::Float(f32::from_bits(d.to_bits() as u32) as f64),
            other => other,
        });
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
    }?;
    // A 64-bit pointer written into a BYREF LONG (`AVIFileOpen(pfile, …)`,
    // `CoCreateInstance(…, ppv)`): the DLL wrote 8 bytes where the program
    // has 4, the upper half into the room after the variable. The variable
    // gets the pointer's 32-bit stand-in, which turns back into the pointer
    // when the program hands it to a DLL (`long_result`).
    for ptr in &out_longs {
        // SAFETY: `ptr` is the start of a VARPTR'd LONG's real memory, with
        // at least 8 bytes of it (`room_at`), ours for the process's life.
        let full = unsafe { std::ptr::read_unaligned(*ptr as *const i64) };
        if full >> 32 != 0 {
            let stand_in = long_result(full);
            if dll::is_pointer_stand_in(stand_in) {
                // SAFETY: as above, the same 8 bytes.
                unsafe { std::ptr::write_unaligned(*ptr as *mut i64, stand_in as i32 as u32 as i64) };
            }
        }
    }
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

/// Whether this run is sandboxed (`RAPIDR_SANDBOX` set, to anything but
/// empty or 0): no DLL is loaded or called.
fn sandboxed() -> bool {
    std::env::var("RAPIDR_SANDBOX").is_ok_and(|v| !v.is_empty() && v != "0")
}

/// 64-bit pointers DLLs returned for 32-bit results: by stand-in, and the
/// stand-in of each.
struct Pointers {
    by_index: Vec<i64>,
    index: HashMap<i64, usize>,
}

static POINTERS: std::sync::Mutex<Option<Pointers>> = std::sync::Mutex::new(None);

use dll::{POINTER_STAND_INS as MAX_HANDLES, POINTER_STAND_IN_BASE as HANDLE_BASE};

/// A DLL's result as RapidQ's 32-bit LONG: the value, when it is one (a
/// number, a handle — Windows keeps USER / GDI / kernel handles within 32
/// bits); a 64-bit pointer becomes a stand-in that turns back into the
/// pointer when the program hands it to a DLL again. (A 32-bit result
/// whose register's upper half the function left as it was isn't a
/// pointer: only an address that is mapped memory of the process gets a
/// stand-in.)
fn long_result(n: i64) -> i64 {
    let low = n as i32 as i64;
    if low == n || (n >> 32) == 0 || !is_mapped(n) {
        return low;
    }
    let mut guard = POINTERS.lock().unwrap_or_else(|e| e.into_inner());
    let p = guard.get_or_insert_with(|| Pointers { by_index: Vec::new(), index: HashMap::new() });
    let i = match p.index.get(&n) {
        Some(&i) => i,
        None if p.by_index.len() < MAX_HANDLES => {
            p.by_index.push(n);
            p.index.insert(n, p.by_index.len() - 1);
            p.by_index.len() - 1
        }
        None => return low,
    };
    HANDLE_BASE + i as i64
}

/// The pointer a stand-in from [`long_result`] stands for.
fn pointer_of_handle(v: i64) -> Option<i64> {
    let i = usize::try_from(v - HANDLE_BASE).ok().filter(|&i| i < MAX_HANDLES)?;
    POINTERS.lock().unwrap_or_else(|e| e.into_inner()).as_ref()?.by_index.get(i).copied()
}

/// Whether `addr` lies in memory the process has (reserved, committed or
/// an image's): what a pointer a DLL returned points into.
#[cfg(windows)]
fn is_mapped(addr: i64) -> bool {
    use windows_sys::Win32::System::Memory::{VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_FREE};
    // SAFETY: a plain-data struct zeroed, then VirtualQuery describes the
    // address's range into it (its size passed); it reads no memory at
    // `addr`, whatever that is.
    unsafe {
        let mut info: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
        let n = VirtualQuery(addr as usize as *const core::ffi::c_void, &mut info, std::mem::size_of::<MEMORY_BASIC_INFORMATION>());
        n != 0 && info.State != MEM_FREE
    }
}

#[cfg(not(windows))]
fn is_mapped(_addr: i64) -> bool {
    true
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
            _ => long_mixed!($ptr, $s, $ret),
        }
    }};
}

/// The longest DOUBLE / SINGLE argument list a call may have (the float
/// registers of the conventions that pass floats apart from integers).
const MAX_FLOAT_ARGS: usize = 8;

/// 64-bit Windows on x64: arguments by position — the first four in RCX /
/// RDX / R8 / R9 or XMM0–3 by their type, the rest on the stack, 8 bytes
/// each whatever the type (a float's bits as they are). So a call of 5 to
/// 16 arguments is the first four's types and integers after them.
#[cfg(all(windows, target_arch = "x86_64"))]
macro_rules! win64_tail {
    ($ptr:expr, $s:expr, $ret:ty; $a:ident, $b:ident, $c:ident, $d:ident) => {
        match $s.len() {
            5 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4),
            6 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5),
            7 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5, I 6),
            8 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5, I 6, I 7),
            9 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5, I 6, I 7, I 8),
            10 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5, I 6, I 7, I 8, I 9),
            11 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10),
            12 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11),
            13 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11, I 12),
            14 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11, I 12, I 13),
            15 => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11, I 12, I 13, I 14),
            _ => call_as!($ptr, $s, $ret; $a 0, $b 1, $c 2, $d 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11, I 12, I 13, I 14, I 15),
        }
    };
}

#[cfg(all(windows, target_arch = "x86_64"))]
macro_rules! long_mixed {
    ($ptr:expr, $s:expr, $ret:ty) => {{
        let f = |i: usize| matches!($s.get(i), Some(Slot::Float(_)));
        match (f(0), f(1), f(2), f(3)) {
            (false, false, false, false) => win64_tail!($ptr, $s, $ret; I, I, I, I),
            (false, false, false, true) => win64_tail!($ptr, $s, $ret; I, I, I, F),
            (false, false, true, false) => win64_tail!($ptr, $s, $ret; I, I, F, I),
            (false, false, true, true) => win64_tail!($ptr, $s, $ret; I, I, F, F),
            (false, true, false, false) => win64_tail!($ptr, $s, $ret; I, F, I, I),
            (false, true, false, true) => win64_tail!($ptr, $s, $ret; I, F, I, F),
            (false, true, true, false) => win64_tail!($ptr, $s, $ret; I, F, F, I),
            (false, true, true, true) => win64_tail!($ptr, $s, $ret; I, F, F, F),
            (true, false, false, false) => win64_tail!($ptr, $s, $ret; F, I, I, I),
            (true, false, false, true) => win64_tail!($ptr, $s, $ret; F, I, I, F),
            (true, false, true, false) => win64_tail!($ptr, $s, $ret; F, I, F, I),
            (true, false, true, true) => win64_tail!($ptr, $s, $ret; F, I, F, F),
            (true, true, false, false) => win64_tail!($ptr, $s, $ret; F, F, I, I),
            (true, true, false, true) => win64_tail!($ptr, $s, $ret; F, F, I, F),
            (true, true, true, false) => win64_tail!($ptr, $s, $ret; F, F, F, I),
            (true, true, true, true) => win64_tail!($ptr, $s, $ret; F, F, F, F),
        }
    }};
}

/// ARM64 (Windows, macOS, Linux) and x64 outside Windows: integers and
/// floats go in registers of their own, each kind in its order (X0–X7 /
/// V0–V7; RDI… / XMM0–7), the integers past the registers on the stack in
/// their order. So any call with up to 8 floats is one signature: the
/// integers, then the floats, each kind in its order (registers the callee
/// doesn't read are ignored, and the caller clears the stack).
#[cfg(not(all(windows, target_arch = "x86_64")))]
macro_rules! long_mixed {
    ($ptr:expr, $s:expr, $ret:ty) => {{
        let mut ints = [0i64; 16];
        let mut floats = [0f64; MAX_FLOAT_ARGS];
        let (mut ni, mut nf) = (0, 0);
        for slot in $s.iter() {
            match *slot {
                Slot::Int(n) => {
                    ints[ni] = n;
                    ni += 1;
                }
                Slot::Float(x) => {
                    floats[nf] = x;
                    nf += 1;
                }
            }
        }
        let split = [
            Slot::Int(ints[0]), Slot::Int(ints[1]), Slot::Int(ints[2]), Slot::Int(ints[3]),
            Slot::Int(ints[4]), Slot::Int(ints[5]), Slot::Int(ints[6]), Slot::Int(ints[7]),
            Slot::Int(ints[8]), Slot::Int(ints[9]), Slot::Int(ints[10]), Slot::Int(ints[11]),
            Slot::Int(ints[12]), Slot::Int(ints[13]), Slot::Int(ints[14]), Slot::Int(ints[15]),
            Slot::Float(floats[0]), Slot::Float(floats[1]), Slot::Float(floats[2]), Slot::Float(floats[3]),
            Slot::Float(floats[4]), Slot::Float(floats[5]), Slot::Float(floats[6]), Slot::Float(floats[7]),
        ];
        call_as!($ptr, split, $ret; I 0, I 1, I 2, I 3, I 4, I 5, I 6, I 7, I 8, I 9, I 10, I 11, I 12, I 13, I 14, I 15,
            F 16, F 17, F 18, F 19, F 20, F 21, F 22, F 23)
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
    // SAFETY (of the signature): Windows calls the filter with a pointer
    // to the exception's record or null; the body only reads it through
    // `as_ref`, prints and returns, touching no program state.
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
        // (an address in this program's own image: mapped memory, above
        // 4 GB on every 64-bit system with ASLR)
        let p = long_result as fn(i64) -> i64 as usize as i64;
        assert!(p >> 32 != 0, "{p:#x}");
        let h = long_result(p);
        assert!(h < 0 && h >= HANDLE_BASE, "{h}");
        assert_eq!((long_result(p), pointer_of_handle(h)), (h, Some(p)));
        assert_eq!(pointer_of_handle(-1), None);
        assert_eq!(pointer_of_handle(12345), None);
        // Flags stay flags while stand-ins exist: GENERIC_READ OR
        // GENERIC_WRITE (0xC0000000) is no pointer.
        assert_eq!(pointer_of_handle(0xC000_0000u32 as i32 as i64), None);
        assert_eq!(pointer_of_handle(HANDLE_BASE + MAX_HANDLES as i64), None);
        if cfg!(windows) {
            // (a 32-bit result over a register's stale upper half: no
            // memory there, so the value itself)
            assert_eq!(long_result(0x1234_0000_0005), 5);
        }
    }

    /// Long calls mixing integers, DOUBLEs and SINGLEs (GDI+'s
    /// `GdipDrawBezier` has 2 handles and 8 SINGLEs), through the table, to
    /// a function of exactly that signature.
    #[test]
    fn long_mixed_calls() {
        extern "system" fn probe(a: i32, b: f64, c: i32, d: f32, e: i64, f: f64, g: f32, h: i32, i: f64, j: f32, k: i32) -> f64 {
            a as f64 + b * 10.0 + c as f64 * 100.0 + d as f64 * 1000.0 + e as f64 * 1e4 + f * 1e5 + g as f64 * 1e6 + h as f64 * 1e7 + i * 1e8 + j as f64 * 1e9 + k as f64 * 1e10
        }
        let spec = Spec::parse("DOUBLE|LONG:v,DOUBLE:v,LONG:v,SINGLE:v,LONG:v,DOUBLE:v,SINGLE:v,LONG:v,DOUBLE:v,SINGLE:v,LONG:v");
        let single = |x: f32| Slot::Float(f64::from_bits(x.to_bits() as u64));
        let slots = [Slot::Int(1), Slot::Float(2.0), Slot::Int(3), single(4.0), Slot::Int(5), Slot::Float(6.0), single(7.0), Slot::Int(8), Slot::Float(9.0), single(1.0), Slot::Int(2)];
        assert!(spec.returns_float());
        // SAFETY: `probe` has exactly the signature the slots describe.
        let r = unsafe { call_table(probe as *const (), &slots, true) }.unwrap();
        let Raw::Float(r) = r else { panic!("not a float") };
        assert_eq!(r, 21_987_654_321.0);
        extern "system" fn bezier(g: i64, pen: i64, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32) -> i32 {
            (g + pen) as i32 + (x1 + 2.0 * y1 + 3.0 * x2 + 4.0 * y2 + 5.0 * x3 + 6.0 * y3 + 7.0 * x4 + 8.0 * y4) as i32
        }
        let slots = [Slot::Int(10), Slot::Int(20), single(1.0), single(1.0), single(1.0), single(1.0), single(1.0), single(1.0), single(1.0), single(1.0)];
        // SAFETY: as above.
        let r = unsafe { call_table(bezier as *const (), &slots, false) }.unwrap();
        assert!(matches!(r, Raw::Int(n) if n as i32 == 66), "bezier");
        // More floats than registers: refused before any call.
        let nine = format!("LONG|{}", vec!["DOUBLE:v"; 9].join(","));
        let e = dll_call(if cfg!(windows) { "msvcrt" } else if cfg!(target_os = "macos") { "libSystem.B.dylib" } else { "libc.so.6" }, "abs", &nine, &vec![v_dbl(1.0); 9]).unwrap_err();
        assert!(e.contains("up to 8 DOUBLE / SINGLE arguments"), "{e}");
    }

    /// A 64-bit pointer a DLL writes into a BYREF LONG becomes the
    /// variable's stand-in, and the stand-in turns back into the pointer
    /// when the program hands it to the DLL again.
    #[test]
    fn pointers_written_into_byref_longs() {
        let p = memory::varptr_var("main:outptr", &v_int(0), "LONG").unwrap();
        if cfg!(windows) {
            // GetModuleHandleExA(0, "kernel32", BYREF hModule): a 64-bit HMODULE
            let r = dll_call("kernel32", "GetModuleHandleExA", "LONG|LONG:v,STRING:v,LONG:r", &[v_int(0), v_str("kernel32"), p.clone()]).unwrap();
            assert_eq!(r, v_int(1));
        } else {
            // posix_memalign(BYREF memptr, 16, 64): a 64-bit heap pointer
            let lib = if cfg!(target_os = "macos") { "libSystem.B.dylib" } else { "libc.so.6" };
            let r = dll_call(lib, "posix_memalign", "LONG|LONG:r,LONG:v,LONG:v", &[p.clone(), v_int(16), v_int(64)]).unwrap();
            assert_eq!(r, v_int(0));
        }
        let h = memory::sync("main:outptr", &v_int(0), "LONG").to_i64();
        assert!(dll::is_pointer_stand_in(h), "{h:#x}");
        let real = pointer_of_handle(h).unwrap();
        assert!(real >> 32 != 0, "{real:#x}");
        if cfg!(windows) {
            let f = dll_call("kernel32", "GetProcAddress", "INT64|LONG:v,STRING:v", &[v_int(h), v_str("GetTickCount")]).unwrap();
            assert_ne!(f, v_int(0));
        } else {
            let lib = if cfg!(target_os = "macos") { "libSystem.B.dylib" } else { "libc.so.6" };
            dll_call(lib, "free", "|LONG:v", &[v_int(h)]).unwrap();
        }
        // PEEK of the stand-in: not the program's memory, said so.
        let e = memory::peek(None, &v_int(h)).unwrap_err();
        assert!(e.contains("memory a DLL returned"), "{e}");
    }

    /// CallWindowProc given the program's own memory (x86 machine code a
    /// RapidQ program wrote) is refused before anything runs.
    #[test]
    fn machine_code_in_the_programs_memory_is_refused() {
        let code = memory::varptr_var("main:asm", &v_str("\u{55}\u{8B}\u{EC}"), "STRING").unwrap();
        for a in [code, v_str("\u{C3}")] {
            let e = dll_call("user32", "CallWindowProcA", "LONG|LONG:v,LONG:v,LONG:v,LONG:v,LONG:v", &[a, v_int(0), v_int(0), v_int(0), v_int(0)]).unwrap_err();
            if cfg!(windows) {
                assert!(e.contains("machine code for 32-bit x86"), "{e}");
            }
        }
    }

    /// The DLL files' own format read from their headers.
    #[test]
    fn pe_machines() {
        let dir = std::env::temp_dir().join(format!("rapidr-pe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut pe = vec![0u8; 0x200];
        pe[..2].copy_from_slice(b"MZ");
        pe[0x3C] = 0x80;
        pe[0x80..0x84].copy_from_slice(b"PE\0\0");
        pe[0x84..0x86].copy_from_slice(&PE_I386.to_le_bytes());
        let f = dir.join("old32.dll");
        std::fs::write(&f, &pe).unwrap();
        assert_eq!(pe_machine(&f), Some(PE_I386));
        std::fs::write(&f, b"not a dll").unwrap();
        assert_eq!(pe_machine(&f), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// SINGLE arguments and results: the float's 32 bits in the float slot.
    #[test]
    fn single_arguments_and_results() {
        let lib = if cfg!(windows) { "ucrtbase" } else if cfg!(target_os = "macos") { "libSystem.B.dylib" } else { "libm.so.6" };
        assert_eq!(dll_call(lib, "sqrtf", "SINGLE|SINGLE:v", &[v_dbl(6.25)]).unwrap(), v_dbl(2.5));
        assert_eq!(dll_call(lib, "fmaxf", "SINGLE|SINGLE:v,SINGLE:v", &[v_dbl(-1.5), v_dbl(0.25)]).unwrap(), v_dbl(0.25));
        assert_eq!(dll_call(lib, "ldexpf", "SINGLE|SINGLE:v,LONG:v", &[v_dbl(0.75), v_int(4)]).unwrap(), v_dbl(12.0));
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
