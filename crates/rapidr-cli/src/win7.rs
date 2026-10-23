//! Windows 7 (and 8): the few functions only newer Windows has.
//!
//! The `*-win7-windows-gnu` builds (tools/win7/build.sh) import five
//! functions Windows 7 does not have: `RoGetActivationFactory` and
//! `RoOriginateErrorW` (api-ms-win-core-winrt-*), `PathCchStripPrefix`
//! (api-ms-win-core-path), and `CoIncrementMTAUsage` /
//! `CoCreateFreeThreadedMarshaler` / `CoTaskMemFree` from `combase.dll`
//! (Windows 7 keeps those in `ole32.dll`, minus the first). They come from
//! the `windows` / `windows-sys` crates (error info, WinRT, COM) and from
//! arboard's clipboard file list. Imported the usual way, Windows 7 refuses
//! to start the program ("api-ms-win-core-winrt-l1-1-0.dll is missing").
//!
//! So build.rs asks the linker to load those DLLs on first use
//! (`--delayload`), and this module is what answers when they are not
//! there: `combase.dll` is read from `ole32.dll` instead, and the
//! functions Windows 7 lacks get a stand-in that returns "not available"
//! (the callers all treat that as an ordinary error). On Windows 8 and
//! later nothing here ever runs: the real functions are found.

use std::ffi::{c_char, c_void, CStr};

type Farproc = *mut c_void;

/// `DelayLoadProc` (delayimp.h): the function being resolved.
#[repr(C)]
struct DelayLoadProc {
    by_name: i32,
    name: *const c_char,
}

/// `DelayLoadInfo` (delayimp.h).
#[repr(C)]
struct DelayLoadInfo {
    cb: u32,
    pidd: *const c_void,
    ppfn: *mut Farproc,
    dll: *const c_char,
    proc_: DelayLoadProc,
    module: *mut c_void,
    function: Farproc,
    last_error: u32,
}

type Hook = unsafe extern "system" fn(u32, *const DelayLoadInfo) -> Farproc;

extern "C" {
    /// The delay-load helper's failure hook (mingw-w64's libmingwex).
    static mut __pfnDliFailureHook2: Option<Hook>;
}

extern "system" {
    fn LoadLibraryA(name: *const c_char) -> *mut c_void;
}

const DLI_FAIL_LOAD_LIB: u32 = 4;
const DLI_FAIL_GET_PROC: u32 = 5;

const E_NOTIMPL: i32 = 0x8000_4001_u32 as i32;
const REGDB_E_CLASSNOTREG: i32 = 0x8004_0154_u32 as i32;

// The stand-ins: "not available" in each function's own kind of answer.
unsafe extern "system" fn ro_originate_error(_error: i32, _max: u32, _message: *const u16) -> i32 {
    0 // BOOL FALSE: no WinRT error object was made (the Error is still returned)
}
unsafe extern "system" fn ro_get_activation_factory(_class: *mut c_void, _iid: *const c_void, factory: *mut *mut c_void) -> i32 {
    if !factory.is_null() {
        *factory = std::ptr::null_mut();
    }
    REGDB_E_CLASSNOTREG // no WinRT here
}
unsafe extern "system" fn hresult_not_implemented() -> i32 {
    E_NOTIMPL
}

unsafe extern "system" fn hook(notify: u32, info: *const DelayLoadInfo) -> Farproc {
    let info = &*info;
    match notify {
        // The DLL is not on this Windows: a module that has the function
        // (combase.dll's are in ole32.dll), or any loaded one so that the
        // next step (the function) is asked about, and answered below.
        DLI_FAIL_LOAD_LIB => {
            let dll = CStr::from_ptr(info.dll).to_bytes().to_ascii_lowercase();
            let stand_in: &CStr = if dll == b"combase.dll" { c"ole32.dll" } else if dll.starts_with(b"api-ms-win-core-") { c"kernel32.dll" } else { return std::ptr::null_mut() };
            LoadLibraryA(stand_in.as_ptr())
        }
        // The function is not in the DLL: its stand-in, if it is one of ours.
        DLI_FAIL_GET_PROC if info.proc_.by_name != 0 => match CStr::from_ptr(info.proc_.name).to_bytes() {
            b"RoOriginateErrorW" => ro_originate_error as Farproc,
            b"RoGetActivationFactory" => ro_get_activation_factory as Farproc,
            b"PathCchStripPrefix" | b"CoIncrementMTAUsage" => hresult_not_implemented as Farproc,
            _ => std::ptr::null_mut(),
        },
        _ => std::ptr::null_mut(),
    }
}

/// Called first thing in `main`, before any thread starts.
pub fn install() {
    unsafe { __pfnDliFailureHook2 = Some(hook) };
}
