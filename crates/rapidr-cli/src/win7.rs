//! Windows 7 (and 8): the few functions only newer Windows has.
//!
//! The `*-win7-windows-gnu` builds (tools/win7/build.sh) use six functions
//! Windows 7 does not have: `RoGetActivationFactory` and `RoOriginateErrorW`
//! (WinRT, `combase.dll`), `PathCchStripPrefix` (`kernelbase.dll`), and
//! `CoIncrementMTAUsage`, `CoCreateFreeThreadedMarshaler`, `CoTaskMemFree`
//! (the `windows` / `windows-sys` crates import these from `combase.dll`;
//! Windows 7 keeps the last two in `ole32.dll` and has no `CoIncrementMTAUsage`
//! at all). They come from the `windows` / `windows-sys` crates (error info,
//! WinRT, COM) and from arboard's clipboard file list. Imported the usual
//! way, Windows 7 refuses to start the program ("api-ms-win-core-winrt-l1-1-0.dll
//! is missing").
//!
//! Nothing here is delay-loaded and no helper from any toolchain is used:
//! the program's import table never names those DLLs. With
//! `--cfg windows_raw_dylib` a call to such a function goes through a pointer
//! named `__imp_<Function>` (the import-table slot). This module defines
//! those six pointers itself, each pointing at a small function of ours.
//! The linker takes an object's own definition before anything in a library,
//! so the import library's entry (and with it the DLL) never gets in. On the
//! first call the function asks `GetProcAddress` for the real one in the
//! DLLs that can have it (`combase.dll`, `kernelbase.dll`, `ole32.dll`); on
//! Windows 8 and later it is found and called. On Windows 7 it is not there,
//! and the function answers as "not available" in its own kind of answer
//! (an error code the callers all treat as an ordinary error).
//!
//! Written from the public documentation of `GetModuleHandleW`,
//! `LoadLibraryW`, `GetProcAddress`, `GetSystemDirectoryW` and of the six
//! functions themselves (their signatures), and from the PE/COFF import
//! naming (`__imp_` pointers). See tools/win7/clean-room.md.

use std::arch::global_asm;
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
}

const E_NOTIMPL: i32 = 0x8000_4001_u32 as i32;
const REGDB_E_CLASSNOTREG: i32 = 0x8004_0154_u32 as i32;

/// One function that may not be there: the DLLs to look in, in order, and
/// what was found (the address; `ABSENT` once asked and not there).
struct Import {
    name: &'static CStr,
    dlls: &'static [&'static str],
    address: AtomicPtr<c_void>,
    /// Which of `dlls` had it (for the report), 0 = none.
    found_in: AtomicUsize,
}

/// "Asked, and Windows does not have it" (never a real address).
const ABSENT: *mut c_void = 1 as *mut c_void;

impl Import {
    const fn new(name: &'static CStr, dlls: &'static [&'static str]) -> Import {
        Import { name, dlls, address: AtomicPtr::new(std::ptr::null_mut()), found_in: AtomicUsize::new(0) }
    }

    /// The real function, or `None` on a Windows that does not have it.
    fn get(&self) -> Option<*mut c_void> {
        let mut p = self.address.load(Ordering::Acquire);
        if p.is_null() {
            p = ABSENT;
            for (i, dll) in self.dlls.iter().enumerate() {
                let found = unsafe { find(dll, self.name) };
                if !found.is_null() {
                    p = found;
                    self.found_in.store(i + 1, Ordering::Relaxed);
                    break;
                }
            }
            // (two threads asking at once find the same thing)
            self.address.store(p, Ordering::Release);
        }
        if p == ABSENT { None } else { Some(p) }
    }

    /// For `rapidr __win7_check`: where the function was found.
    fn describe(&self) -> String {
        let name = self.name.to_string_lossy();
        match self.get() {
            Some(_) => format!("{name}: found in {}", self.dlls[self.found_in.load(Ordering::Relaxed) - 1]),
            None => format!("{name}: not on this Windows (the stand-in answers)"),
        }
    }
}

/// `name` in the DLL called `dll`: if it is already loaded, that one;
/// otherwise the copy in Windows' own folder (never a same-named file next
/// to the program).
unsafe fn find(dll: &str, name: &CStr) -> *mut c_void {
    let mut wide = [0u16; 32];
    if dll.len() >= wide.len() {
        return std::ptr::null_mut();
    }
    for (slot, unit) in wide.iter_mut().zip(dll.encode_utf16()) {
        *slot = unit;
    }
    let mut module = GetModuleHandleW(wide.as_ptr());
    if module.is_null() {
        let mut path = [0u16; 320];
        let n = GetSystemDirectoryW(path.as_mut_ptr(), 260) as usize;
        if n == 0 || n > 260 {
            return std::ptr::null_mut();
        }
        path[n] = u16::from(b'\\');
        for (slot, unit) in path[n + 1..].iter_mut().zip(dll.encode_utf16()) {
            *slot = unit;
        }
        module = LoadLibraryW(path.as_ptr());
    }
    if module.is_null() {
        std::ptr::null_mut()
    } else {
        GetProcAddress(module, name.as_ptr())
    }
}

/// The `__imp_$name` slot: one pointer-sized word holding `$shim`'s address.
macro_rules! slot {
    ($name:literal, $shim:ident, $word:literal) => {
        global_asm!(
            ".section .data",
            ".p2align 3",
            concat!(".globl __imp_", $name),
            concat!("__imp_", $name, ":"),
            concat!($word, " {shim}"),
            shim = sym $shim,
            options(att_syntax),
        );
    };
}
#[cfg(target_arch = "x86")]
macro_rules! slot_for_arch {
    ($name:literal, $shim:ident) => {
        slot!($name, $shim, ".long");
    };
}
#[cfg(target_arch = "x86_64")]
macro_rules! slot_for_arch {
    ($name:literal, $shim:ident) => {
        slot!($name, $shim, ".quad");
    };
}

/// One import: the function of ours that stands for `$name`, and the
/// `__imp_$name` pointer to it (what the callers load their target from).
macro_rules! import {
    ($cell:ident, $shim:ident, $name:literal, [$($dll:literal),+], fn($($arg:ident: $ty:ty),*) -> $ret:ty, absent: $absent:expr) => {
        static $cell: Import = Import::new(
            match CStr::from_bytes_with_nul(concat!($name, "\0").as_bytes()) {
                Ok(name) => name,
                Err(_) => panic!("name"),
            },
            &[$($dll),+],
        );

        #[allow(unused_variables)]
        unsafe extern "system" fn $shim($($arg: $ty),*) -> $ret {
            match $cell.get() {
                Some(real) => {
                    let real: unsafe extern "system" fn($($ty),*) -> $ret = std::mem::transmute(real);
                    real($($arg),*)
                }
                None => $absent,
            }
        }

        slot_for_arch!($name, $shim);
    };
}

// The stand-ins: "not available" in each function's own kind of answer.
import!(RO_GET_ACTIVATION_FACTORY, ro_get_activation_factory, "RoGetActivationFactory", ["combase.dll"],
    fn(class: *mut c_void, iid: *const c_void, factory: *mut *mut c_void) -> i32,
    absent: {
        if !factory.is_null() {
            *factory = std::ptr::null_mut();
        }
        REGDB_E_CLASSNOTREG // no WinRT here
    });
import!(RO_ORIGINATE_ERROR, ro_originate_error, "RoOriginateErrorW", ["combase.dll"],
    fn(error: i32, max: u32, message: *const u16) -> i32,
    absent: 0 /* BOOL FALSE: no WinRT error object was made (the Error is still returned) */);
import!(PATH_CCH_STRIP_PREFIX, path_cch_strip_prefix, "PathCchStripPrefix", ["kernelbase.dll", "api-ms-win-core-path-l1-1-0.dll"],
    fn(path: *mut u16, len: usize) -> i32,
    absent: E_NOTIMPL);
import!(CO_INCREMENT_MTA_USAGE, co_increment_mta_usage, "CoIncrementMTAUsage", ["ole32.dll", "combase.dll"],
    fn(cookie: *mut *mut c_void) -> i32,
    absent: E_NOTIMPL);
import!(CO_CREATE_FREE_THREADED_MARSHALER, co_create_free_threaded_marshaler, "CoCreateFreeThreadedMarshaler", ["ole32.dll", "combase.dll"],
    fn(outer: *mut c_void, marshaler: *mut *mut c_void) -> i32,
    absent: E_NOTIMPL);
import!(CO_TASK_MEM_FREE, co_task_mem_free, "CoTaskMemFree", ["ole32.dll", "combase.dll"],
    fn(memory: *mut c_void) -> (),
    absent: ());

/// `rapidr __win7_check`: where each of the six was found on this Windows,
/// and, for one that is not there, what its stand-in answers (it is called
/// once, with harmless arguments).
pub fn report() -> String {
    let mut lines: Vec<String> = [&RO_GET_ACTIVATION_FACTORY, &RO_ORIGINATE_ERROR, &PATH_CCH_STRIP_PREFIX, &CO_INCREMENT_MTA_USAGE, &CO_CREATE_FREE_THREADED_MARSHALER, &CO_TASK_MEM_FREE]
        .iter()
        .map(|i| i.describe())
        .collect();
    unsafe {
        let mut out: *mut c_void = std::ptr::null_mut();
        if RO_GET_ACTIVATION_FACTORY.get().is_none() {
            let hr = ro_get_activation_factory(std::ptr::null_mut(), std::ptr::null(), &mut out);
            lines.push(format!("stand-in RoGetActivationFactory -> {hr:#010x}, factory {:?}", out));
        }
        if RO_ORIGINATE_ERROR.get().is_none() {
            lines.push(format!("stand-in RoOriginateErrorW -> {}", ro_originate_error(E_NOTIMPL, 0, std::ptr::null())));
        }
        if PATH_CCH_STRIP_PREFIX.get().is_none() {
            lines.push(format!("stand-in PathCchStripPrefix -> {:#010x}", path_cch_strip_prefix(std::ptr::null_mut(), 0)));
        }
        if CO_INCREMENT_MTA_USAGE.get().is_none() {
            lines.push(format!("stand-in CoIncrementMTAUsage -> {:#010x}", co_increment_mta_usage(&mut out)));
        }
        if CO_CREATE_FREE_THREADED_MARSHALER.get().is_none() {
            lines.push(format!("stand-in CoCreateFreeThreadedMarshaler -> {:#010x}", co_create_free_threaded_marshaler(std::ptr::null_mut(), &mut out)));
        }
    }
    lines.join("\n")
}
