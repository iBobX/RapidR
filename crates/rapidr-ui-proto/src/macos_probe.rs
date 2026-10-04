//! Spike A probes on macOS: AppKit events synthesized as `NSEvent`s and
//! posted into the application's queue (`-[NSApplication postEvent:atStart:]`,
//! thread-safe), so they reach the app exactly as real ones do — through
//! `NSApp.run` inside a pump, `sendEvent:`, AppKit's own tracking loops
//! (live resize), key equivalents (menus). This works with the screen
//! locked, where CGEvent-level input goes to the login window instead.
//! Dialogs are cancelled with `-[NSSavePanel cancel:]` from a main-queue
//! block (GCD's main queue is serviced in modal and tracking run-loop
//! modes too).

use std::ffi::c_void;
use std::ptr::NonNull;
use std::time::Duration;

use objc2::encode::{Encode, Encoding};
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::{class, msg_send};
use objc2_foundation::NSString;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Pt {
    x: f64,
    y: f64,
}
unsafe impl Encode for Pt {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Sz {
    w: f64,
    h: f64,
}
unsafe impl Encode for Sz {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Rc {
    o: Pt,
    s: Sz,
}
unsafe impl Encode for Rc {
    const ENCODING: Encoding = Encoding::Struct("CGRect", &[Pt::ENCODING, Sz::ENCODING]);
}

const LEFT_DOWN: usize = 1;
const LEFT_UP: usize = 2;
const LEFT_DRAGGED: usize = 6;
const KEY_DOWN: usize = 10;
const KEY_UP: usize = 11;
const CMD: usize = 1 << 20;

/// What a window is, for posting events to it from another thread.
#[derive(Clone, Copy, Debug)]
pub struct WinInfo {
    pub number: isize,
    /// The frame's size (points, title bar included).
    pub frame: (f64, f64),
}

/// The NSWindow of an NSView (winit's AppKit handle), on the main thread.
pub fn window_info(ns_view: NonNull<c_void>) -> Option<WinInfo> {
    unsafe {
        let view = ns_view.as_ptr() as *mut AnyObject;
        let window: *mut AnyObject = msg_send![view, window];
        if window.is_null() {
            return None;
        }
        let number: isize = msg_send![window, windowNumber];
        let frame: Rc = msg_send![window, frame];
        Some(WinInfo { number, frame: (frame.s.w, frame.s.h) })
    }
}

/// `-[NSWindow inLiveResize]` for a view's window (main thread).
pub fn in_live_resize(ns_view: NonNull<c_void>) -> bool {
    unsafe {
        let view = ns_view.as_ptr() as *mut AnyObject;
        let window: *mut AnyObject = msg_send![view, window];
        if window.is_null() {
            return false;
        }
        let b: Bool = msg_send![window, inLiveResize];
        b.as_bool()
    }
}

fn uptime() -> f64 {
    unsafe {
        let pi: *mut AnyObject = msg_send![class!(NSProcessInfo), processInfo];
        msg_send![pi, systemUptime]
    }
}

fn post(ev: *mut AnyObject) {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, postEvent: ev, atStart: Bool::NO];
    }
}

fn mouse(kind: usize, win: &WinInfo, x: f64, y: f64) {
    unsafe {
        let ev: *mut AnyObject = msg_send![class!(NSEvent),
            mouseEventWithType: kind,
            location: Pt { x, y },
            modifierFlags: 0usize,
            timestamp: uptime(),
            windowNumber: win.number,
            context: std::ptr::null_mut::<AnyObject>(),
            eventNumber: 0isize,
            clickCount: 1isize,
            pressure: if kind == LEFT_UP { 0.0f32 } else { 1.0f32 }];
        post(ev);
    }
}

fn key(kind: usize, win: &WinInfo, chars: &str, code: u16, flags: usize) {
    unsafe {
        let s = NSString::from_str(chars);
        let ev: *mut AnyObject = msg_send![class!(NSEvent),
            keyEventWithType: kind,
            location: Pt { x: 0.0, y: 0.0 },
            modifierFlags: flags,
            timestamp: uptime(),
            windowNumber: win.number,
            context: std::ptr::null_mut::<AnyObject>(),
            characters: &*s,
            charactersIgnoringModifiers: &*s,
            isARepeat: Bool::NO,
            keyCode: code];
        post(ev);
    }
}

/// A user's drag of the window's right edge by `dx` points over `ms`
/// (mouse down on the edge, drags, up), posted from a thread.
pub fn live_resize(win: WinInfo, dx: f64, ms: u64) {
    std::thread::spawn(move || {
        // window coordinates: origin at the frame's bottom left; the right
        // edge's grab zone is a few points either side of it
        let (x0, y) = (win.frame.0 - 1.0, win.frame.1 / 2.0);
        mouse(LEFT_DOWN, &win, x0, y);
        let n = (ms / 16).max(4);
        for k in 1..=n {
            std::thread::sleep(Duration::from_millis(ms / n));
            mouse(LEFT_DRAGGED, &win, x0 + dx * k as f64 / n as f64, y);
        }
        std::thread::sleep(Duration::from_millis(30));
        mouse(LEFT_UP, &win, x0 + dx, y);
        eprintln!("[probe] live resize: posted mouse down, {n} drags over {ms} ms, up");
    });
}

/// Cmd+<c> to a window (a menu's key equivalent), from a thread.
pub fn key_equivalent(win: WinInfo, c: char, code: u16) {
    std::thread::spawn(move || {
        let s = c.to_string();
        key(KEY_DOWN, &win, &s, code, CMD);
        key(KEY_UP, &win, &s, code, CMD);
    });
}

/// After `ms`, on the main queue: every open/save panel cancelled
/// (sheets and app-modal runModal alike).
pub fn cancel_panels_after(ms: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(ms));
        dispatch2::DispatchQueue::main().exec_async(|| unsafe {
            let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
            let windows: *mut AnyObject = msg_send![app, windows];
            let n: usize = msg_send![windows, count];
            let panel_class: &AnyClass = class!(NSSavePanel);
            let mut cancelled = 0;
            for i in 0..n {
                let w: *mut AnyObject = msg_send![windows, objectAtIndex: i];
                let is_panel: Bool = msg_send![w, isKindOfClass: panel_class];
                let visible: Bool = msg_send![w, isVisible];
                if is_panel.as_bool() && visible.as_bool() {
                    let _: () = msg_send![w, cancel: std::ptr::null_mut::<AnyObject>()];
                    cancelled += 1;
                }
            }
            let modal: *mut AnyObject = msg_send![app, modalWindow];
            eprintln!("[probe] main queue ran (in whatever run-loop mode): cancelled {cancelled} panel(s); modal window was {}", if modal.is_null() { "none" } else { "set" });
        });
    });
}

/// A user opening a menu: from a main-queue block (so from `NSApp.run`
/// inside a pump, no winit callback on the stack, as a click on the menu
/// bar or a right-click would be) a pop-up menu tracks synchronously in
/// AppKit's event-tracking loop; after `ms` another main-queue block
/// (serviced in that mode too) cancels it.
pub fn menu_tracking(ns_view: NonNull<c_void>, ms: u64) {
    let addr = ns_view.as_ptr() as usize;
    dispatch2::DispatchQueue::main().exec_async(move || unsafe {
        let view = addr as *mut AnyObject;
        let menu: *mut AnyObject = msg_send![class!(NSMenu), new];
        let title = NSString::from_str("Probe item");
        let empty = NSString::from_str("");
        let _: *mut AnyObject = msg_send![menu, addItemWithTitle: &*title, action: std::ptr::null::<c_void>(), keyEquivalent: &*empty];
        let maddr = menu as usize;
        let window: *mut AnyObject = msg_send![view, window];
        let number: isize = msg_send![window, windowNumber];
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done2 = done.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(ms));
            // (1) a main-queue block; (2) an Escape posted to the queue;
            // (3) report if neither ended the tracking
            dispatch2::DispatchQueue::main().exec_async(move || {
                let menu = maddr as *mut AnyObject;
                let _: () = msg_send![menu, cancelTracking];
                eprintln!("[probe] main-queue block ran during menu tracking: cancelTracking sent");
            });
            std::thread::sleep(Duration::from_millis(300));
            if !done2.load(std::sync::atomic::Ordering::SeqCst) {
                let win = WinInfo { number, frame: (0.0, 0.0) };
                key(KEY_DOWN, &win, "\u{1b}", 53, 0);
                key(KEY_UP, &win, "\u{1b}", 53, 0);
                eprintln!("[probe] menu still tracking 300 ms after the main-queue cancel: posted Escape");
            }
            for s in 1..=5 {
                std::thread::sleep(Duration::from_secs(1));
                if done2.load(std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                eprintln!("[probe] menu still tracking {s} s after the cancels (the pump is held; no step runs)");
            }
        });
        let t = std::time::Instant::now();
        let _: Bool = msg_send![menu, popUpMenuPositioningItem: std::ptr::null_mut::<AnyObject>(), atLocation: Pt { x: 20.0, y: 20.0 }, inView: view];
        done.store(true, std::sync::atomic::Ordering::SeqCst);
        eprintln!("[probe] pop-up menu tracking returned after {:.0} ms", t.elapsed().as_secs_f64() * 1e3);
    });
}

/// After `ms`, on the main queue (so: while a modal dialog runs): the
/// window of `view` resized by AppKit and marked for display, to see
/// what winit does with the events this makes.
pub fn poke_after(view: NonNull<c_void>, ms: u64) {
    let addr = view.as_ptr() as usize;
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(ms));
        dispatch2::DispatchQueue::main().exec_async(move || unsafe {
            let view = addr as *mut AnyObject;
            let window: *mut AnyObject = msg_send![view, window];
            let frame: Rc = msg_send![view, frame];
            let _: () = msg_send![window, setContentSize: Sz { w: frame.s.w + 24.0, h: frame.s.h }];
            let _: () = msg_send![view, setNeedsDisplay: Bool::YES];
            eprintln!("[probe] poked the window during the modal: content size {}x{} -> {}x{}", frame.s.w, frame.s.h, frame.s.w + 24.0, frame.s.h);
        });
    });
}
