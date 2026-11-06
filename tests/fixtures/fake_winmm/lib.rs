//! A stand-in for Windows' winmm.dll MIDI input (see Cargo.toml): one input
//! device whose "driver", a thread of its own, plays three SysEx messages.
//! For each it takes the oldest buffer midiInAddBuffer queued, writes the
//! message into the buffer `lpData` names, sets `dwBytesRecorded` and
//! MHDR_DONE in the header — all after midiInAddBuffer returned — and posts
//! MM_MIM_LONGDATA (wParam the device, lParam the header) to the window
//! midiInOpen was given; an MM_MIM_DATA note before each. As a real driver,
//! it keeps its own data in the header's reserved fields (written by
//! midiInPrepareHeader) and refuses a header whose reserved fields were
//! overwritten, or that is still queued.
//!
//! The header is RapidQ's 32-bit MIDIHDR as RapidQ programs declare it
//! (`lpData AS LONG`, … `dwReserved AS STRING * 64`: winmm.inc), which a
//! real winmm reads that way on 32-bit Windows. `FAKE_WINMM_LOG` names a
//! file where it writes what it did.

use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const MM_MIM_DATA: u32 = 0x3C3;
const MM_MIM_LONGDATA: u32 = 0x3C4;
const MHDR_DONE: u32 = 1;
const MHDR_PREPARED: u32 = 2;
const MHDR_INQUEUE: u32 = 4;
const MMSYSERR_NOERROR: u32 = 0;
const MMSYSERR_BADDEVICEID: u32 = 2;
const MMSYSERR_INVALHANDLE: u32 = 5;
const MMSYSERR_INVALPARAM: u32 = 11;
const MIDIERR_UNPREPARED: u32 = 64;
const MIDIERR_STILLPLAYING: u32 = 65;
const DEVICE: usize = 0x51;

// RapidQ's 32-bit MIDIHDR
const LP_DATA: usize = 0;
const BUFFER_LENGTH: usize = 4;
const BYTES_RECORDED: usize = 8;
const FLAGS: usize = 16;
const RESERVED: usize = 24;
const DW_RESERVED: usize = 32;
/// What the driver keeps in `reserved` and at the start of `dwReserved`
/// (a NUL first: the program's STRING * 64 field reads "").
const MINE: u32 = 0xC0FF_EE00;
const PRIVATE: [u8; 8] = [0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];

const MESSAGES: [&[u8]; 3] = [
    &[0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7],
    &[0xF0, 0x7E, 0x00, 0x06, 0x02, 0x41, 0x10, 0x42, 0x00, 0x01, 0x00, 0x00, 0x00, 0xF7],
    &[0xF0, 0x41, 0x10, 0x42, 0x12, 0x40, 0x00, 0x7F, 0x00, 0x41, 0xF7],
];

struct State {
    hwnd: isize,
    queue: VecDeque<usize>,
}

static STATE: Mutex<State> = Mutex::new(State { hwnd: 0, queue: VecDeque::new() });
static RUNNING: AtomicBool = AtomicBool::new(false);

#[link(name = "user32")]
extern "system" {
    fn PostMessageA(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
}

fn log(text: &str) {
    if let Some(path) = std::env::var_os("FAKE_WINMM_LOG") {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{text}");
        }
    }
}

unsafe fn get(hdr: usize, at: usize) -> u32 {
    std::ptr::read_volatile((hdr + at) as *const u32)
}

unsafe fn set(hdr: usize, at: usize, v: u32) {
    std::ptr::write_volatile((hdr + at) as *mut u32, v)
}

#[no_mangle]
pub extern "system" fn midiInGetNumDevs() -> u32 {
    1
}

/// MIDIINCAPS as RapidQ programs declare it: WORD wMid, WORD wPid, LONG vDriverVersion,
/// STRING * 32 szPname, DWORD dwSupport.
#[no_mangle]
pub unsafe extern "system" fn midiInGetDevCapsA(id: usize, caps: *mut u8, size: u32) -> u32 {
    if id != 0 || caps.is_null() || size < 44 {
        return MMSYSERR_BADDEVICEID;
    }
    std::ptr::write_bytes(caps, 0, 44);
    let name = b"RapidR test MIDI in";
    std::ptr::copy_nonoverlapping(name.as_ptr(), caps.add(8), name.len());
    MMSYSERR_NOERROR
}

#[no_mangle]
pub unsafe extern "system" fn midiInOpen(phmi: *mut u64, id: usize, callback: isize, _instance: usize, _flags: u32) -> u32 {
    if id != 0 || phmi.is_null() {
        return MMSYSERR_BADDEVICEID;
    }
    let mut s = STATE.lock().unwrap();
    s.hwnd = callback;
    s.queue.clear();
    std::ptr::write_unaligned(phmi, DEVICE as u64);
    log(&format!("open: window {callback:#x}"));
    MMSYSERR_NOERROR
}

#[no_mangle]
pub unsafe extern "system" fn midiInPrepareHeader(h: usize, hdr: usize, size: u32) -> u32 {
    if h != DEVICE {
        return MMSYSERR_INVALHANDLE;
    }
    if hdr == 0 || size < 64 {
        return MMSYSERR_INVALPARAM;
    }
    set(hdr, FLAGS, get(hdr, FLAGS) | MHDR_PREPARED);
    set(hdr, RESERVED, MINE);
    std::ptr::copy_nonoverlapping(PRIVATE.as_ptr(), (hdr + DW_RESERVED) as *mut u8, PRIVATE.len());
    log(&format!("prepare: header {hdr:#x}, buffer {:#x} ({} bytes)", get(hdr, LP_DATA), get(hdr, BUFFER_LENGTH)));
    MMSYSERR_NOERROR
}

#[no_mangle]
pub unsafe extern "system" fn midiInUnprepareHeader(h: usize, hdr: usize, _size: u32) -> u32 {
    if h != DEVICE || hdr == 0 {
        return MMSYSERR_INVALHANDLE;
    }
    set(hdr, FLAGS, get(hdr, FLAGS) & !MHDR_PREPARED);
    MMSYSERR_NOERROR
}

#[no_mangle]
pub unsafe extern "system" fn midiInAddBuffer(h: usize, hdr: usize, _size: u32) -> u32 {
    if h != DEVICE || hdr == 0 {
        return MMSYSERR_INVALHANDLE;
    }
    let flags = get(hdr, FLAGS);
    if flags & MHDR_PREPARED == 0 {
        log("add: refused, not prepared");
        return MIDIERR_UNPREPARED;
    }
    if flags & MHDR_INQUEUE != 0 {
        log("add: refused, still queued (MIDIERR_STILLPLAYING)");
        return MIDIERR_STILLPLAYING;
    }
    let mut private = [0u8; 8];
    std::ptr::copy_nonoverlapping((hdr + DW_RESERVED) as *const u8, private.as_mut_ptr(), 8);
    if get(hdr, RESERVED) != MINE || private != PRIVATE {
        log("add: refused, the driver's reserved fields were overwritten");
        return MMSYSERR_INVALPARAM;
    }
    set(hdr, FLAGS, (flags | MHDR_INQUEUE) & !MHDR_DONE);
    set(hdr, BYTES_RECORDED, 0);
    STATE.lock().unwrap().queue.push_back(hdr);
    log("add: queued");
    MMSYSERR_NOERROR
}

#[no_mangle]
pub extern "system" fn midiInStart(h: usize) -> u32 {
    if h != DEVICE {
        return MMSYSERR_INVALHANDLE;
    }
    if RUNNING.swap(true, Ordering::SeqCst) {
        return MMSYSERR_NOERROR;
    }
    log("start");
    std::thread::spawn(|| {
        for (k, message) in MESSAGES.iter().enumerate() {
            std::thread::sleep(std::time::Duration::from_millis(300));
            if !RUNNING.load(Ordering::SeqCst) {
                return;
            }
            let (hwnd, hdr) = {
                let mut s = STATE.lock().unwrap();
                (s.hwnd, s.queue.pop_front())
            };
            // SAFETY: the window midiInOpen was given.
            unsafe { PostMessageA(hwnd, MM_MIM_DATA, DEVICE, 0x40_3C90 + k as isize) };
            let Some(hdr) = hdr else {
                log(&format!("message {}: no buffer queued, dropped", k + 1));
                continue;
            };
            // SAFETY: a header the program prepared and queued, and the
            // buffer it names (the program's memory: RapidR's are real
            // addresses on Windows).
            unsafe {
                let data = get(hdr, LP_DATA) as usize;
                let n = message.len().min(get(hdr, BUFFER_LENGTH) as usize);
                std::ptr::copy_nonoverlapping(message.as_ptr(), data as *mut u8, n);
                set(hdr, BYTES_RECORDED, n as u32);
                set(hdr, FLAGS, (get(hdr, FLAGS) | MHDR_DONE) & !MHDR_INQUEUE);
                log(&format!("message {}: {n} bytes into {data:#x}", k + 1));
                PostMessageA(hwnd, MM_MIM_LONGDATA, DEVICE, hdr as isize);
            }
        }
        log("played all");
    });
    MMSYSERR_NOERROR
}

#[no_mangle]
pub extern "system" fn midiInStop(h: usize) -> u32 {
    if h != DEVICE {
        return MMSYSERR_INVALHANDLE;
    }
    RUNNING.store(false, Ordering::SeqCst);
    MMSYSERR_NOERROR
}

#[no_mangle]
pub unsafe extern "system" fn midiInReset(h: usize) -> u32 {
    if h != DEVICE {
        return MMSYSERR_INVALHANDLE;
    }
    RUNNING.store(false, Ordering::SeqCst);
    let mut s = STATE.lock().unwrap();
    while let Some(hdr) = s.queue.pop_front() {
        set(hdr, FLAGS, (get(hdr, FLAGS) | MHDR_DONE) & !MHDR_INQUEUE);
    }
    MMSYSERR_NOERROR
}

#[no_mangle]
pub extern "system" fn midiInClose(h: usize) -> u32 {
    if h != DEVICE {
        return MMSYSERR_INVALHANDLE;
    }
    log("close");
    MMSYSERR_NOERROR
}

#[no_mangle]
pub extern "system" fn midiOutGetNumDevs() -> u32 {
    0
}

#[no_mangle]
pub extern "system" fn midiOutGetDevCapsA(_id: usize, _caps: usize, _size: u32) -> u32 {
    MMSYSERR_BADDEVICEID
}

#[no_mangle]
pub extern "system" fn midiOutOpen(_phmo: usize, _id: usize, _callback: usize, _instance: usize, _flags: u32) -> u32 {
    MMSYSERR_BADDEVICEID
}

#[no_mangle]
pub extern "system" fn midiOutShortMsg(_h: usize, _msg: u32) -> u32 {
    MMSYSERR_INVALHANDLE
}

#[no_mangle]
pub extern "system" fn midiOutLongMsg(_h: usize, _hdr: usize, _size: u32) -> u32 {
    MMSYSERR_INVALHANDLE
}

#[no_mangle]
pub extern "system" fn midiOutPrepareHeader(_h: usize, _hdr: usize, _size: u32) -> u32 {
    MMSYSERR_INVALHANDLE
}

#[no_mangle]
pub extern "system" fn midiOutUnprepareHeader(_h: usize, _hdr: usize, _size: u32) -> u32 {
    MMSYSERR_INVALHANDLE
}

#[no_mangle]
pub extern "system" fn midiOutReset(_h: usize) -> u32 {
    MMSYSERR_INVALHANDLE
}

#[no_mangle]
pub extern "system" fn midiOutClose(_h: usize) -> u32 {
    MMSYSERR_INVALHANDLE
}
