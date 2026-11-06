//! Everything of the JIT that needs `unsafe`, in one place
//! (docs/jit-design.md §7): the memory compiled code lives in, the
//! start-up check that this process may run code it makes, the call into
//! an entry form, and the helpers compiled code calls. Each `unsafe` block
//! says why it is sound.
//!
//! **W^X**: compiled code is never writable and executable at once. Off
//! macOS the code generator's own memory does it (pages written, then made
//! read + execute before anything runs). On macOS the hardened runtime
//! allows only `MAP_JIT` memory, written while this thread's write
//! protection is off (`pthread_jit_write_protect_np(0)`: not executable
//! for this thread meanwhile) and executable once it is back on — the
//! memory below. The stock memory would be killed there even with the
//! `allow-jit` entitlement (docs/legal/review-2026-11-05-cranelift.md §7).

use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

use rapidr_value::BasicArray;

use crate::kinds::{bits_of, value_of, K};

/// What compiled code is handed: the first three fields at the offsets
/// `compile.rs` reads (`CTX_GLOBALS`, `CTX_INTERRUPT`, `CTX_STACK_LIMIT`);
/// the rest only the helpers touch.
#[repr(C)]
pub struct Ctx {
    /// The module's globals buffer (one `u64` per global compiled code
    /// uses), alive for the call.
    pub globals: *mut u64,
    /// The debugger's interrupt flag (a byte; `NOT_INTERRUPTED` outside
    /// debug mode).
    pub interrupt: *const u8,
    /// The lowest stack pointer compiled code makes a call at (SEC-50):
    /// below it the interpreter makes the call (its frames are on the
    /// heap; its stack overflow error at `MAX_CALL_DEPTH`). The higher of
    /// the thread's stack end + [`STACK_MARGIN`] + the largest frame
    /// compiled for the module (the callee's whole frame fits above the
    /// margin) and the entry's stack pointer − the smallest compiled frame
    /// (at least [`MIN_FRAME`]) × the frames the VM has room for (each
    /// compiled call takes at least that much: never more frames than
    /// `MAX_CALL_DEPTH` allows).
    pub stack_limit: u64,
    /// The exits taken, innermost frame first: record and values.
    pub exits: Vec<(u32, Vec<u64>)>,
    pub garrays: Vec<Option<Rc<RefCell<BasicArray>>>>,
    pub larrays: Vec<Rc<RefCell<BasicArray>>>,
}

// (compile.rs's offsets are these)
const _: () = {
    assert!(std::mem::offset_of!(Ctx, globals) == crate::compile::CTX_GLOBALS as usize);
    assert!(std::mem::offset_of!(Ctx, interrupt) == crate::compile::CTX_INTERRUPT as usize);
    assert!(std::mem::offset_of!(Ctx, stack_limit) == crate::compile::CTX_STACK_LIMIT as usize);
};

/// The interrupt byte outside debug mode: never raised.
pub static NOT_INTERRUPTED: u8 = 0;

/// The interrupt flag's byte (an `AtomicBool` is one byte, 0 or 1, the
/// same layout as `bool`).
pub fn interrupt_byte(flag: &std::sync::atomic::AtomicBool) -> *const u8 {
    flag.as_ptr() as *const u8
}

// ---------- the thread's machine stack (SEC-50, docs/jit-design.md §12.7) ----------

/// Stack kept free below the lowest frame compiled code pushes: the helpers
/// it calls run there (an exit's copy, an array's borrow, a maths builtin,
/// a caught panic's hook), and the system's own reserve (Windows keeps the
/// last pages of a stack, glibc may count its guard inside).
pub const STACK_MARGIN: usize = 256 << 10;

/// The least machine stack a compiled call takes: the return address and
/// the frame pointer the callee saves (every compiled function sets up its
/// frame: `preserve_frame_pointers`).
pub const MIN_FRAME: usize = 16;

/// The lowest address of this thread's stack (asked once per thread);
/// `None` where it can't be known — then nothing compiled runs.
pub fn stack_end() -> Option<usize> {
    thread_local! {
        static END: std::cell::Cell<Option<Option<usize>>> = const { std::cell::Cell::new(None) };
    }
    END.with(|c| {
        if let Some(e) = c.get() {
            return e;
        }
        let e = stack::end();
        c.set(Some(e));
        e
    })
}

/// About the stack pointer here (a local's address, a little below the
/// caller's frame).
#[inline(never)]
pub fn stack_here() -> usize {
    let probe = 0u8;
    std::hint::black_box(&probe) as *const u8 as usize
}

#[cfg(target_os = "macos")]
mod stack {
    pub fn end() -> Option<usize> {
        // SAFETY: both ask about the calling thread (`pthread_self` is
        // always valid for it) and only read its record; no memory of ours
        // is written.
        let (top, size) = unsafe {
            let t = libc::pthread_self();
            (libc::pthread_get_stackaddr_np(t) as usize, libc::pthread_get_stacksize_np(t))
        };
        top.checked_sub(size).filter(|&e| e > 0 && size > 0)
    }
}

#[cfg(target_os = "linux")]
mod stack {
    pub fn end() -> Option<usize> {
        // SAFETY: `attr` is filled by `pthread_getattr_np` for the calling
        // thread, read only by the attribute getters into our locals, and
        // destroyed once — only when it was filled.
        unsafe {
            let mut attr: libc::pthread_attr_t = std::mem::zeroed();
            if libc::pthread_getattr_np(libc::pthread_self(), &mut attr) != 0 {
                return None;
            }
            let mut addr: *mut libc::c_void = std::ptr::null_mut();
            let mut size: libc::size_t = 0;
            let mut guard: libc::size_t = 0;
            let ok = libc::pthread_attr_getstack(&attr, &mut addr, &mut size) == 0;
            if libc::pthread_attr_getguardsize(&attr, &mut guard) != 0 {
                guard = 0;
            }
            libc::pthread_attr_destroy(&mut attr);
            if !ok || addr.is_null() || size == 0 {
                return None;
            }
            // (glibc before 2.27 counts the guard inside the stack: kept
            // out either way)
            (addr as usize).checked_add(guard)
        }
    }
}

#[cfg(windows)]
mod stack {
    use std::ffi::c_void;

    /// `MEMORY_BASIC_INFORMATION` (64-bit Windows: the JIT is built for
    /// 64-bit only).
    #[repr(C)]
    struct MemoryBasicInformation {
        base_address: *mut c_void,
        allocation_base: *mut c_void,
        allocation_protect: u32,
        partition_id: u16,
        region_size: usize,
        state: u32,
        protect: u32,
        kind: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn VirtualQuery(address: *const c_void, buffer: *mut MemoryBasicInformation, length: usize) -> usize;
    }

    pub fn end() -> Option<usize> {
        let probe = 0u8;
        let at = std::hint::black_box(&probe) as *const u8 as *const c_void;
        let mut info = std::mem::MaybeUninit::<MemoryBasicInformation>::zeroed();
        // SAFETY: VirtualQuery writes at most `length` bytes — the
        // struct's own size — into `info`, which has the documented
        // layout; `at` is a local's address, on this thread's stack. (On
        // every Windows the JIT runs on, Windows 7 included.)
        let n = unsafe { VirtualQuery(at, info.as_mut_ptr(), std::mem::size_of::<MemoryBasicInformation>()) };
        if n == 0 {
            return None;
        }
        // SAFETY: zeroed, then filled by the call that succeeded.
        let info = unsafe { info.assume_init() };
        // (a stack is one reservation: its allocation base is its lowest
        // address, whatever of it is committed yet)
        Some(info.allocation_base as usize).filter(|&e| e > 0 && e < at as usize)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
mod stack {
    pub fn end() -> Option<usize> {
        None
    }
}

/// Runs entry form `code` on `ctx` and `locals`.
pub fn call_entry(code: *const u8, ctx: &mut Ctx, locals: &mut [u64]) -> u32 {
    // SAFETY: `code` is the address of a function this crate compiled with
    // `compile::entry_signature` — the platform's C calling convention,
    // `(ctx, locals) -> u32` — and finalized (made executable) in a
    // `JITModule` the engine owns and keeps alive while it can run.
    // `compile.rs` reads and writes through `locals` only at offsets
    // `8 × slot` for slots below the function's `n_locals`, which the
    // verifier checked against the module, and `locals` has `n_locals`
    // entries (the engine sizes it); it reads `ctx`'s three first fields
    // and passes `ctx` to the helpers, which the glue doesn't touch until
    // this call returns. The globals pointer in `ctx` is a live buffer of
    // the size compiled code indexes (`Env::globals`, grown only between
    // runs).
    unsafe {
        let f: extern "C" fn(*mut Ctx, *mut u64) -> u32 = std::mem::transmute(code);
        f(ctx as *mut Ctx, locals.as_mut_ptr())
    }
}

// ---------- the helpers (compiled code calls them; `compile::Helpers`) ----------

/// An exit: its record and the values compiled code wrote out.
pub extern "C" fn rapidr_jit_exit(ctx: *mut Ctx, record: u32, values: *const u64, n: u32) {
    // SAFETY: `ctx` is the context `call_entry` passed in (alive, used by
    // nothing else during the call); `values` points to `n` u64 compiled
    // code stored in a stack slot of `8 × max(n, 1)` bytes it made for this
    // exit (compile.rs `exit_now`).
    let (ctx, values) = unsafe { (&mut *ctx, std::slice::from_raw_parts(values, n as usize)) };
    ctx.exits.push((record, values.to_vec()));
}

/// The array of a handle.
fn array(ctx: &Ctx, table: u32, handle: u32) -> Option<&Rc<RefCell<BasicArray>>> {
    match table {
        0 => ctx.garrays.get(handle as usize).and_then(Option::as_ref),
        _ => ctx.larrays.get(handle as usize),
    }
}

/// `arr(i)` (one index) as `rapidr_vm::fast::aget1`'s fast path: 0 and the
/// element's bits in `out` when it is in range and of kind `expect`; else
/// 1 (the interpreter answers, or raises the error).
pub extern "C" fn rapidr_jit_aget(ctx: *mut Ctx, table: u32, handle: u32, index: i64, expect: u32, out: *mut u64) -> u32 {
    // SAFETY: `ctx` as in `rapidr_jit_exit`; `out` is an 8-byte stack slot
    // compiled code made for this call (compile.rs `aget`).
    let (ctx, out) = unsafe { (&*ctx, &mut *out) };
    let got = catch_unwind(AssertUnwindSafe(|| {
        let a = array(ctx, table, handle)?.try_borrow().ok()?;
        let [(lo, hi)] = a.bounds[..] else { return None };
        if index < lo || index > hi {
            return None;
        }
        let v = a.data.get(index.wrapping_sub(lo) as usize)?;
        (K::of(v).code() == expect).then(|| bits_of(v))
    }));
    match got {
        Ok(Some(bits)) => {
            *out = bits;
            0
        }
        _ => 1,
    }
}

/// `arr(i) = v` (one index) as `fast::aset1`'s fast path: 0 when stored;
/// 1 — nothing stored — for anything else.
pub extern "C" fn rapidr_jit_aset(ctx: *mut Ctx, table: u32, handle: u32, index: i64, kind: u32, bits: u64) -> u32 {
    // SAFETY: `ctx` as in `rapidr_jit_exit`.
    let ctx = unsafe { &*ctx };
    let done = catch_unwind(AssertUnwindSafe(|| {
        let mut a = array(ctx, table, handle)?.try_borrow_mut().ok()?;
        let [(lo, hi)] = a.bounds[..] else { return None };
        if index < lo || index > hi {
            return None;
        }
        let slot = a.data.get_mut(index.wrapping_sub(lo) as usize)?;
        rapidr_vm::fast::put(slot, value_of(K::from_code(kind), bits));
        Some(())
    }));
    match done {
        Ok(Some(())) => 0,
        _ => 1,
    }
}

/// `ToNum code` (`fast::to_num`) of a scalar: the result's bits (its kind
/// is the code's: Double / Single a Double, the others an Integer).
pub extern "C" fn rapidr_jit_conv(code: u32, kind: u32, bits: u64) -> u64 {
    let v = value_of(K::from_code(kind), bits);
    catch_unwind(AssertUnwindSafe(|| rapidr_vm::fast::to_num(v, code as u8).map(|r| bits_of(&r)).unwrap_or(0))).unwrap_or(0)
}

/// A pure maths builtin (`fast::call_direct`): the result's bits in `out`
/// and its kind's code as the answer (an unexpected one: compiled code
/// leaves, the interpreter calls it again — it is pure).
pub extern "C" fn rapidr_jit_math(code: u32, kind: u32, bits: u64, out: *mut u64) -> u32 {
    // SAFETY: `out` is an 8-byte stack slot compiled code made for this
    // call (compile.rs `math`).
    let out = unsafe { &mut *out };
    let v = value_of(K::from_code(kind), bits);
    match catch_unwind(AssertUnwindSafe(|| rapidr_vm::fast::call_direct(code as u8, std::slice::from_ref(&v)))) {
        Ok(r) => {
            *out = bits_of(&r);
            K::of(&r).code()
        }
        Err(_) => u32::MAX,
    }
}

/// SHL / SHR (`fast::call_direct` with two arguments): as
/// [`rapidr_jit_math`].
pub extern "C" fn rapidr_jit_math2(code: u32, ka: u32, a: u64, kb: u32, b: u64, out: *mut u64) -> u32 {
    // SAFETY: `out` is an 8-byte stack slot compiled code made for this
    // call (compile.rs `math`).
    let out = unsafe { &mut *out };
    let args = [value_of(K::from_code(ka), a), value_of(K::from_code(kb), b)];
    match catch_unwind(AssertUnwindSafe(|| rapidr_vm::fast::call_direct(code as u8, &args))) {
        Ok(r) => {
            *out = bits_of(&r);
            K::of(&r).code()
        }
        Err(_) => u32::MAX,
    }
}

/// `a ^ b` (`Value::power`).
pub extern "C" fn rapidr_jit_powf(a: f64, b: f64) -> f64 {
    a.powf(b)
}

/// The helpers by the names `compile::Helpers::declare` imports.
pub fn helper_symbols() -> Vec<(&'static str, *const u8)> {
    vec![
        ("rapidr_jit_exit", rapidr_jit_exit as *const u8),
        ("rapidr_jit_aget", rapidr_jit_aget as *const u8),
        ("rapidr_jit_aset", rapidr_jit_aset as *const u8),
        ("rapidr_jit_conv", rapidr_jit_conv as *const u8),
        ("rapidr_jit_math", rapidr_jit_math as *const u8),
        ("rapidr_jit_math2", rapidr_jit_math2 as *const u8),
        ("rapidr_jit_powf", rapidr_jit_powf as *const u8),
    ]
}

// ---------- where compiled code lives ----------

/// The most machine code a process makes (docs/jit-design.md §8): past
/// it, nothing new is compiled.
pub const CODE_CAP: usize = 16 << 20;

/// Whether this process may run code it makes, and if not why: asked
/// once, at start (`rapidr run --verbose` says it).
pub fn probe() -> Result<(), String> {
    imp::probe()
}

/// The memory a module's compiled code goes in (`None`: the code
/// generator's own — off macOS).
pub fn memory_provider() -> Result<Option<Box<dyn cranelift_jit::JITMemoryProvider + Send>>, String> {
    imp::provider()
}

/// Frees a module's compiled code.
pub fn free(jm: cranelift_jit::JITModule) {
    // SAFETY: called only when the engine that owns `jm` is dropped — the
    // VM it served has finished with it: no compiled code of it runs or
    // can be called any more (entries hold indices into the engine, not
    // pointers that outlive it).
    unsafe { jm.free_memory() }
}

#[cfg(target_os = "macos")]
mod imp {
    use cranelift_jit::{BranchProtection, JITMemoryKind, JITMemoryProvider};
    use cranelift_module::ModuleResult;
    use std::io;

    extern "C" {
        #[cfg(target_arch = "aarch64")]
        fn pthread_jit_write_protect_np(enabled: libc::c_int);
        fn sys_icache_invalidate(start: *mut libc::c_void, len: libc::size_t);
    }

    /// One MAP_JIT region, bump-allocated: code only (no data).
    pub struct MapJit {
        base: *mut u8,
        cap: usize,
        used: usize,
        writing: bool,
    }

    // SAFETY: the region is owned by this value alone; the JIT moves it
    // with the module to the thread that compiles and runs it (one).
    unsafe impl Send for MapJit {}

    fn map(len: usize) -> io::Result<*mut u8> {
        // SAFETY: an anonymous private mapping of `len` bytes, no address
        // asked for: it touches no memory of the process; failure is
        // reported (EINVAL under the hardened runtime without the
        // allow-jit entitlement).
        let p = unsafe {
            libc::mmap(std::ptr::null_mut(), len, libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC, libc::MAP_PRIVATE | libc::MAP_ANON | libc::MAP_JIT, -1, 0)
        };
        if p == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        Ok(p as *mut u8)
    }

    /// This thread may write MAP_JIT memory (`false`: may execute it).
    fn writable(on: bool) {
        // SAFETY: switches this thread's view of every MAP_JIT region
        // between writable and executable; the JIT makes no call into
        // compiled code while it is writable (allocation to finalize, all
        // inside one compile on this thread).
        #[cfg(target_arch = "aarch64")]
        unsafe {
            pthread_jit_write_protect_np(if on { 0 } else { 1 })
        };
        #[cfg(not(target_arch = "aarch64"))]
        let _ = on;
    }

    pub fn probe() -> Result<(), String> {
        let page = 1 << 14;
        let p = map(page).map_err(|e| format!("this process may not make code executable ({e}): signed with the hardened runtime and without the com.apple.security.cs.allow-jit entitlement"))?;
        // SAFETY: `p` is the mapping just made, of `page` bytes, unused.
        unsafe { libc::munmap(p as *mut libc::c_void, page) };
        Ok(())
    }

    pub fn provider() -> Result<Option<Box<dyn JITMemoryProvider + Send>>, String> {
        let base = map(super::CODE_CAP).map_err(|e| format!("no memory for compiled code ({e})"))?;
        Ok(Some(Box::new(MapJit { base, cap: super::CODE_CAP, used: 0, writing: false })))
    }

    impl JITMemoryProvider for MapJit {
        fn allocate(&mut self, size: usize, align: u64, kind: JITMemoryKind) -> io::Result<*mut u8> {
            if !matches!(kind, JITMemoryKind::Executable) {
                return Err(io::Error::new(io::ErrorKind::Unsupported, "only code goes in MAP_JIT memory"));
            }
            let a = (align as usize).max(16);
            let start = self.used.div_ceil(a) * a;
            if self.base.is_null() || start.checked_add(size).is_none_or(|end| end > self.cap) {
                return Err(io::Error::new(io::ErrorKind::OutOfMemory, "the compiled code's memory is full"));
            }
            if !self.writing {
                writable(true);
                self.writing = true;
            }
            self.used = start + size;
            // SAFETY: `start + size <= cap`: inside the region.
            Ok(unsafe { self.base.add(start) })
        }

        unsafe fn free_memory(&mut self) {
            if !self.base.is_null() {
                // SAFETY (the caller's): no code in the region runs any more.
                unsafe { libc::munmap(self.base as *mut libc::c_void, self.cap) };
                self.base = std::ptr::null_mut();
            }
        }

        fn finalize(&mut self, _branch_protection: BranchProtection) -> ModuleResult<()> {
            if self.writing {
                // SAFETY: the region's first `used` bytes are mapped; the
                // instruction cache drops what it held of them before this
                // thread executes the new code.
                unsafe { sys_icache_invalidate(self.base as *mut libc::c_void, self.used) };
                writable(false);
                self.writing = false;
            }
            Ok(())
        }
    }

    impl Drop for MapJit {
        fn drop(&mut self) {
            // (never left writable)
            if self.writing {
                writable(false);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn probe() -> Result<(), String> {
        let page = region::page::size();
        let mut mem = region::alloc(page, region::Protection::READ_WRITE).map_err(|e| format!("no memory for compiled code ({e})"))?;
        // SAFETY: `mem` is the allocation just made, `page` bytes, unused;
        // making it read + execute touches nothing else (SELinux's
        // execmem refusal comes back as an error here, not a kill).
        unsafe { region::protect(mem.as_mut_ptr::<u8>(), page, region::Protection::READ_EXECUTE) }.map_err(|e| format!("this process may not make code executable ({e})"))?;
        Ok(())
    }

    pub fn provider() -> Result<Option<Box<dyn cranelift_jit::JITMemoryProvider + Send>>, String> {
        Ok(None)
    }
}
