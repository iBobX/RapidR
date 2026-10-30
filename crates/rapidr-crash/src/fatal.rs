//! The fatal signals (Unix): a stack overflow (SIGSEGV or SIGBUS on the
//! stack's guard page), another memory fault, an illegal instruction, an
//! arithmetic trap, an abort. The process can't go on, and a signal
//! handler may do very little: no allocation, no lock. So the report is a
//! smaller one, written here with nothing but system calls into a buffer
//! set aside for it — the kind, where the RapidR program was (the
//! interpreter's last call into the runtime, the event being handled: the
//! map of the program is read as it is, never changed), what Studio was
//! doing, the last calls and log lines — then the signal goes on to the
//! handler that was there before (std's, which prints "has overflowed its
//! stack" and aborts), or to the system's.
//!
//! No backtrace: taking one isn't safe here (the report says so).

#![allow(unsafe_code)]

use std::ffi::CString;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

use crate::{place, slots, CALLS, DOING, FILE, LOG, SYSTEM_TEXT, VERSION_TEXT, WROTE};

const SIGNALS: [libc::c_int; 5] = [libc::SIGSEGV, libc::SIGBUS, libc::SIGILL, libc::SIGFPE, libc::SIGABRT];

/// The handlers that were there before ours, by [`SIGNALS`]' order.
static mut OLD: [std::mem::MaybeUninit<libc::sigaction>; 5] = [const { std::mem::MaybeUninit::uninit() }; 5];
static OLD_SET: AtomicBool = AtomicBool::new(false);
/// The folder, as a C string, and as people read it (`~`).
static DIR: OnceLock<(CString, String)> = OnceLock::new();
/// The main thread's stack: (lowest address, highest).
static MAIN_LO: AtomicUsize = AtomicUsize::new(0);
static MAIN_HI: AtomicUsize = AtomicUsize::new(0);
static MAIN_THREAD: AtomicUsize = AtomicUsize::new(0);
/// A handler is running (a second fault inside it goes straight on).
static HANDLING: AtomicBool = AtomicBool::new(false);

/// The report's buffer (the handler's alone: [`HANDLING`]).
const BUF_LEN: usize = 24 * 1024;
static mut BUF: [u8; BUF_LEN] = [0; BUF_LEN];

pub(crate) fn install(dir: Option<&Path>) {
    if let Some(d) = dir {
        let shown = crate::scrub::home(&d.to_string_lossy());
        if let Ok(c) = CString::new(d.to_string_lossy().as_bytes()) {
            let _ = DIR.set((c, shown));
        }
    }
    // SAFETY: plain libc calls on this thread's own handle; `install` runs
    // on the main thread (the CLI's main), whose stack is what we note.
    unsafe {
        let me = libc::pthread_self();
        MAIN_THREAD.store(me as usize, Ordering::Relaxed);
        #[cfg(target_os = "macos")]
        {
            if libc::pthread_main_np() == 1 {
                let hi = libc::pthread_get_stackaddr_np(me) as usize;
                let size = libc::pthread_get_stacksize_np(me);
                MAIN_LO.store(hi.saturating_sub(size), Ordering::Relaxed);
                MAIN_HI.store(hi, Ordering::Relaxed);
            }
        }
        #[cfg(target_os = "linux")]
        {
            let mut attr: libc::pthread_attr_t = std::mem::zeroed();
            if libc::pthread_getattr_np(me, &mut attr) == 0 {
                let mut addr: *mut libc::c_void = std::ptr::null_mut();
                let mut size: libc::size_t = 0;
                if libc::pthread_attr_getstack(&attr, &mut addr, &mut size) == 0 {
                    MAIN_LO.store(addr as usize, Ordering::Relaxed);
                    MAIN_HI.store(addr as usize + size, Ordering::Relaxed);
                }
                libc::pthread_attr_destroy(&mut attr);
            }
        }
        let old = &mut *std::ptr::addr_of_mut!(OLD);
        for (i, &sig) in SIGNALS.iter().enumerate() {
            let mut new: libc::sigaction = std::mem::zeroed();
            new.sa_sigaction = handler as *const () as usize;
            new.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
            libc::sigemptyset(&mut new.sa_mask);
            libc::sigaction(sig, &new, old[i].as_mut_ptr());
        }
        OLD_SET.store(true, Ordering::Release);
    }
}

fn index_of(sig: libc::c_int) -> Option<usize> {
    SIGNALS.iter().position(|&s| s == sig)
}

extern "C" fn handler(sig: libc::c_int, info: *mut libc::siginfo_t, ctx: *mut libc::c_void) {
    if !HANDLING.swap(true, Ordering::AcqRel) && !WROTE.load(Ordering::Acquire) {
        // SAFETY: the buffer is this handler's alone (HANDLING); `info` is
        // the kernel's.
        unsafe { write_report(sig, info) };
    }
    // On to what was there before.
    let Some(i) = index_of(sig) else { return };
    if !OLD_SET.load(Ordering::Acquire) {
        return;
    }
    // SAFETY: OLD was filled before the handler was installed; sigaction
    // and the previous handler are what the system would have called.
    unsafe {
        let old = (*std::ptr::addr_of!(OLD))[i].assume_init();
        let h = old.sa_sigaction;
        if h == libc::SIG_DFL || h == libc::SIG_IGN {
            // (the system's: put back, and the fault happens again on
            // return — or abort raises again — and ends the process)
            libc::sigaction(sig, &old, std::ptr::null_mut());
            if sig != libc::SIGABRT && h == libc::SIG_IGN {
                let mut dfl: libc::sigaction = std::mem::zeroed();
                dfl.sa_sigaction = libc::SIG_DFL;
                libc::sigaction(sig, &dfl, std::ptr::null_mut());
            }
            return;
        }
        if old.sa_flags & libc::SA_SIGINFO != 0 {
            let f: extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void) = std::mem::transmute(h);
            f(sig, info, ctx);
        } else {
            let f: extern "C" fn(libc::c_int) = std::mem::transmute(h);
            f(sig);
        }
    }
}

/// Appends to the buffer, never past its end.
struct W {
    len: usize,
}

impl W {
    unsafe fn buf() -> &'static mut [u8; BUF_LEN] {
        &mut *std::ptr::addr_of_mut!(BUF)
    }
    fn s(&mut self, b: &[u8]) {
        // SAFETY: the handler's own buffer.
        let buf = unsafe { Self::buf() };
        let n = b.len().min(BUF_LEN - self.len);
        buf[self.len..self.len + n].copy_from_slice(&b[..n]);
        self.len += n;
    }
    fn n(&mut self, mut v: u64, width: usize) {
        let mut d = [0u8; 20];
        let mut i = d.len();
        loop {
            i -= 1;
            d[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        for _ in (d.len() - i)..width {
            self.s(b"0");
        }
        self.s(&d[i..]);
    }
    fn field(&mut self, key: &[u8], value: &[u8]) {
        if !value.is_empty() {
            self.s(key);
            self.s(b": ");
            self.s(value);
            self.s(b"\n");
        }
    }
    fn slot<const N: usize>(&mut self, key: &[u8], slot: &slots::Slot<N>) {
        let mut b = [0u8; 512];
        let n = slot.read_into(&mut b);
        self.field(key, &b[..n]);
    }
    fn section(&mut self, title: &[u8]) {
        self.s(b"\n");
        self.s(title);
        self.s(b"\n");
        for _ in 0..title.len() {
            self.s(b"-");
        }
        self.s(b"\n");
    }
}

/// The address a memory fault was at (0 when not known).
unsafe fn fault_address(info: *mut libc::siginfo_t) -> usize {
    if info.is_null() {
        return 0;
    }
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    {
        (*info).si_addr as usize
    }
    // (glibc and musl: si_signo, si_errno, si_code, then the union, whose
    // first field for SIGSEGV / SIGBUS is the address — at 16 on 64-bit)
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "freebsd")))]
    {
        let at = if cfg!(target_pointer_width = "64") { 16 } else { 12 };
        std::ptr::read_unaligned((info as *const u8).add(at).cast::<usize>())
    }
}

/// The smaller report, into the buffer, then into its file.
unsafe fn write_report(sig: libc::c_int, info: *mut libc::siginfo_t) {
    let addr = fault_address(info);
    let main = libc::pthread_self() as usize == MAIN_THREAD.load(Ordering::Relaxed);
    let (lo, hi) = (MAIN_LO.load(Ordering::Relaxed), MAIN_HI.load(Ordering::Relaxed));
    let overflow = (sig == libc::SIGSEGV || sig == libc::SIGBUS) && main && lo != 0 && addr < lo + 64 * 1024 && addr + (1 << 20) > lo && addr < hi;
    let kind: &[u8] = if overflow { b"stack-overflow" } else { b"fatal" };
    let app = crate::app();
    let who: &[u8] = match app {
        crate::App::Studio => b"RapidR Studio",
        crate::App::Program => b"Your program",
        crate::App::Tool => b"RapidR",
    };
    let what: &[u8] = if overflow {
        b"the stack overflowed (calls went too deep)"
    } else {
        match sig {
            libc::SIGSEGV => b"a memory fault (SIGSEGV); it may be a stack overflow on another thread",
            libc::SIGBUS => b"a memory fault (SIGBUS)",
            libc::SIGILL => b"an illegal instruction (SIGILL)",
            libc::SIGFPE => b"an arithmetic trap (SIGFPE)",
            _ => b"the process was aborted (SIGABRT)",
        }
    };
    let mut w = W { len: 0 };
    w.s(b"RapidR crash report\n===================\n\n");
    w.s(who);
    w.s(b" stopped unexpectedly: ");
    w.s(what);
    w.s(b".\n\n");
    w.field(b"Kind", kind);
    w.field(b"App", app.name().as_bytes());
    w.field(b"Message", what);
    // (where the RapidR program was: the map is read as it is)
    let place = place::current();
    let mut ev = [0u8; 96];
    let mut comp = [0u8; 96];
    let (depth, en, cn) = place::innermost_event(&mut ev, &mut comp);
    if place.is_some() || depth > 0 {
        w.s(b"Where: ");
        if let Some(p) = place {
            if !p.file.is_empty() {
                w.s(p.file.as_bytes());
                w.s(b" ");
            }
            w.s(b"line ");
            w.n(u64::from(p.line), 0);
            if p.routine.is_empty() {
                w.s(b", in the main program");
            } else {
                w.s(if p.function { b", in FUNCTION " } else { b", in SUB " });
                w.s(p.routine.as_bytes());
            }
        }
        if depth > 0 {
            w.s(b" (");
            // (`ontimer` as `OnTimer`, roughly: no allocation here)
            let e = &mut ev[..en];
            if e.len() > 2 && e[..2].eq_ignore_ascii_case(b"on") {
                e[0] = b'O';
                e[2] = e[2].to_ascii_uppercase();
            }
            w.s(e);
            if cn > 0 {
                w.s(b" of ");
                w.s(std::str::from_utf8(&comp[..cn]).map_or(&comp[..cn], |c| place::spelled(c).as_bytes()));
            }
            if depth > 1 {
                w.s(b", ");
                w.n(depth as u64, 0);
                w.s(b" handlers deep");
            }
            w.s(b")");
        }
        w.s(b"\n");
    }
    w.slot(b"Doing", &DOING);
    w.slot(b"File", &FILE);
    w.field(b"Thread", if main { b"main" } else { b"another thread" });
    let now = libc::time(std::ptr::null_mut()).max(0) as u64;
    let (y, mo, d, h, mi, s) = crate::report::utc(now);
    w.s(b"When: ");
    w.n(y.max(0) as u64, 4);
    w.s(b"-");
    w.n(u64::from(mo), 2);
    w.s(b"-");
    w.n(u64::from(d), 2);
    w.s(b" ");
    w.n(u64::from(h), 2);
    w.s(b":");
    w.n(u64::from(mi), 2);
    w.s(b":");
    w.n(u64::from(s), 2);
    w.s(b" UTC\n");
    w.slot(b"RapidR", &VERSION_TEXT);
    w.slot(b"System", &SYSTEM_TEXT);
    let pid = libc::getpid() as u64;
    w.s(b"Process: ");
    w.n(pid, 0);
    w.s(b"\n");
    w.section(b"Rust backtrace");
    w.s(b"(not taken: after a fatal signal it isn't safe to; the place above is where the program was)\n");
    w.section(b"Last AI and MCP calls (names only)");
    let before = w.len;
    CALLS.each(|l| {
        w.s(l);
        w.s(b"\n");
    });
    if w.len == before {
        w.s(b"(none)\n");
    }
    w.section(b"Last log lines");
    let before = w.len;
    LOG.each(|l| {
        w.s(l);
        w.s(b"\n");
    });
    if w.len == before {
        w.s(b"(none)\n");
    }
    w.section(b"Privacy");
    w.s(crate::report::PRIVACY.as_bytes());
    w.s(b"\n");
    let len = w.len;

    // The file: <dir>/<stamp>-<app>-<pid>-<kind>.txt
    let Some((dir, shown)) = DIR.get() else { return };
    let mut name = W { len: 0 };
    // (the name goes after the report in the same buffer)
    name.len = len;
    name.s(dir.as_bytes());
    name.s(b"/");
    let name_from = name.len;
    name.n(y.max(0) as u64, 4);
    name.n(u64::from(mo), 2);
    name.n(u64::from(d), 2);
    name.s(b"-");
    name.n(u64::from(h), 2);
    name.n(u64::from(mi), 2);
    name.n(u64::from(s), 2);
    name.s(b"-");
    name.s(crate::report::app_slug(app.name()).as_bytes());
    name.s(b"-");
    name.n(pid, 0);
    name.s(b"-");
    name.s(kind);
    name.s(b".txt\0");
    let buf = W::buf();
    let path = buf[len..name.len].as_ptr() as *const libc::c_char;
    let fd = libc::open(path, libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o600 as libc::c_uint);
    if fd < 0 {
        return;
    }
    let mut at = 0;
    while at < len {
        let n = libc::write(fd, buf[at..len].as_ptr() as *const libc::c_void, len - at);
        if n <= 0 {
            break;
        }
        at += n as usize;
    }
    libc::close(fd);
    WROTE.store(true, Ordering::Release);
    let say = |b: &[u8]| {
        libc::write(2, b.as_ptr() as *const libc::c_void, b.len());
    };
    say(b"[rapidr] A crash report was written (it stays on this computer): ");
    say(shown.as_bytes());
    say(b"/");
    say(&buf[len + (name_from - len)..name.len - 1]);
    say(b"\n");
}
