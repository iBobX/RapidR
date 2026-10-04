//! Timers while the system holds the pump (docs/desktop-host-plan.md,
//! "Timers during native menu tracking").
//!
//! Some of the system's own loops run inside a pump and don't give it back
//! until the user lets go: a native menu tracked — macOS' menu bar (AppKit's
//! event-tracking loop, inside `NSApp.run`), a context menu
//! ([`HostCmd::Popup`](crate::HostCmd::Popup): `popUpMenuPositioningItem:` /
//! `TrackPopupMenu`, inside `apply`) — and Windows' modal size / move loop.
//! runtime-core's step, where timers fire, waits meanwhile. RapidQ's timers
//! keep firing there (Windows' modal loops dispatch WM_TIMER, and WM_PAINT
//! shows what they changed), so RapidR's do too:
//!
//! - **The tracking timer.** While a pump runs, a timer the system's loops
//!   dispatch — macOS: a `CFRunLoopTimer` in the common modes (menu
//!   tracking's `NSEventTrackingRunLoopMode` is one); Windows: a thread
//!   timer with a TIMERPROC, which every modal loop's DispatchMessage calls
//!   — is armed for when the pump should be back (its deadline, plus
//!   [`GRACE`]). If it fires, the pump overstayed: the system holds it
//!   ([`held`]).
//! - **A tick** ([`Hook`]): runtime-core's turn — its due timers fire, their
//!   handlers run as from a step (native ones directly, the VM's through the
//!   wait it's in), what they changed goes into the kernel — then the host
//!   draws the forms whose display changed. The timer is armed again for the
//!   deadline the tick answered ([`Turn::next`]).
//! - **Where a tick runs: never on top of one of the host's callbacks**,
//!   which hold its state. Inside the host's own context menu (`apply`'s
//!   Popup: the callback waits in the menu) the menu's tick runs: the
//!   callback lent it to [`with_popup_tick`]. Anywhere else (macOS' menu
//!   bar, Windows' size / move loop) the timer wakes the event loop with the
//!   waker ([`set_waker`]: the winit host's `UserEvent::Tick`), and winit
//!   delivers it as soon as no callback runs — still inside the system's
//!   loop (winit's run-loop observers are in the common modes; Windows'
//!   modal loops dispatch winit's wake-up message).
//! - While held, the host keeps for later what would start another of the
//!   system's loops (a context menu, an Open / Save sheet) and doesn't
//!   rebuild macOS' menu bar under the user's mouse; a tick may ask for the
//!   held loop to end ([`Turn::end_loop`]: a dialog the program showed
//!   meanwhile waits for it — macOS' `cancelTracking`, Windows' `EndMenu`,
//!   as Windows closes a menu when a dialog takes the focus).
//!
//! The headless host never holds a pump; a GUI test asks it to pretend
//! ([`simulate_hold`]: `__hold_ms`), ticking through the same hook.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use rapidr_ui_kernel::Store;

use crate::Desktop;

/// runtime-core's turn inside a held pump (`Host::set_tracking_hook`): the
/// due timers fired, their handlers run, what they changed put into the
/// kernel's forms.
pub type Hook = Box<dyn FnMut(&mut Desktop, &dyn Store) -> Turn>;

/// What a tick answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Turn {
    /// When the next tick is due (the program's next timer, the kernel's
    /// next deadline); `None`: nothing is scheduled.
    pub next: Option<Instant>,
    /// Something waits for the held loop to end (a dialog the program
    /// showed meanwhile): the menu is closed.
    pub end_loop: bool,
}

/// How long past its deadline a pump may run before it counts as held (a
/// pump returns at its deadline; the system's loops don't).
pub const GRACE: Duration = Duration::from_millis(8);

/// A tick that couldn't run where it was due (the waker's event not
/// delivered yet): the timer tries again this much later.
const RETRY: Duration = Duration::from_millis(50);

type Tick = *mut (dyn FnMut() + 'static);

thread_local! {
    /// A pump is in progress, due back at this deadline (`None`: until
    /// something happens).
    static PUMP: Cell<Option<Option<Instant>>> = const { Cell::new(None) };
    /// The system holds the pump in progress.
    static HELD: Cell<bool> = const { Cell::new(false) };
    /// A tick runs (the timer doesn't start another inside it).
    static TICKING: Cell<bool> = const { Cell::new(false) };
    /// The waker was used and its tick hasn't run yet.
    static WOKEN: Cell<bool> = const { Cell::new(false) };
    /// The pump in progress was asked to come back at its deadline.
    static STOPPED: Cell<bool> = const { Cell::new(false) };
    /// The host's context menu's own tick, while it's tracked.
    static POPUP_TICK: Cell<Option<Tick>> = const { Cell::new(None) };
    static WAKER: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
    /// A tick's panic, for the pump to go on with once it's back (it can't
    /// unwind through the system's loop).
    static PANIC: RefCell<Option<Box<dyn Any + Send>>> = const { RefCell::new(None) };
}

/// The system holds the pump in progress: a native menu is tracked (or
/// Windows' size / move loop runs), and ticks run the program's timers.
pub fn held() -> bool {
    HELD.with(Cell::get)
}

/// How the timer wakes the event loop for a tick outside the host's
/// context menu (the winit host: an event loop proxy's `UserEvent::Tick`).
pub(crate) fn set_waker(waker: Box<dyn Fn()>) {
    WAKER.with(|w| *w.borrow_mut() = Some(waker));
}

/// A pump in progress (from [`begin_pump`]); dropping it — the pump is
/// back — disarms the timer.
pub(crate) struct Pump(());

impl Drop for Pump {
    fn drop(&mut self) {
        sys::arm(None);
        PUMP.with(|p| p.set(None));
        HELD.with(|h| h.set(false));
        WOKEN.with(|w| w.set(false));
    }
}

/// A pump starts, due back at `deadline` (`None`: when something happens —
/// nothing is scheduled, so nothing will need a tick): the timer armed for
/// `deadline` + [`GRACE`] — or for `deadline` itself when it's the pump's
/// only alarm clock ([`OWN_DEADLINE`]: its firing wakes the loop, which
/// then gives the pump back).
pub(crate) fn begin_pump(deadline: Option<Instant>) -> Pump {
    HELD.with(|h| h.set(false));
    WOKEN.with(|w| w.set(false));
    due_at(deadline);
    Pump(())
}

/// The pump in progress is due back at `deadline`: the timer armed for it.
fn due_at(deadline: Option<Instant>) {
    PUMP.with(|p| p.set(Some(deadline)));
    STOPPED.with(|s| s.set(false));
    sys::arm(deadline.map(|d| if OWN_DEADLINE { d } else { d + GRACE }));
}

/// Whether the tracking timer is the pump's own alarm clock (the winit host
/// pumps without a timeout and the timer's firing wakes the loop): on macOS,
/// where winit's own wake-up timer, once its deadline has passed inside a
/// loop that doesn't stop (menu tracking), fires without pause — the
/// process spun at 100 % with a menu open, and every turn queued another of
/// winit's stop events for after it.
pub const OWN_DEADLINE: bool = cfg!(target_os = "macos");

/// The panic a tick stashed, if any (resumed once the pump is back).
pub(crate) fn take_panic() -> Option<Box<dyn Any + Send>> {
    PANIC.with(|p| p.borrow_mut().take())
}

/// Runs `f` — the host's context menu, tracked until the user lets go —
/// with `tick` as the menu's tick: the timer runs it when due, the system
/// holding the pump meanwhile.
pub(crate) fn with_popup_tick<R>(tick: &mut dyn FnMut(), f: impl FnOnce() -> R) -> R {
    let p: *mut (dyn FnMut() + '_) = tick;
    // SAFETY: only the lifetime is erased. The pointer is cleared before
    // this returns (`Restore`, unwinding too), so it never outlives `tick`'s
    // borrow; it's called only while `f` runs, never inside itself
    // (`TICKING`).
    let p: Tick = unsafe { std::mem::transmute::<*mut (dyn FnMut() + '_), Tick>(p) };
    struct Restore(Option<Tick>, bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            POPUP_TICK.with(|t| t.set(self.0));
            HELD.with(|h| h.set(self.1));
            // (the menu closed: the pump, overdue by now, comes back — its
            // deadline is now, not a hold)
            if let Some(Some(_)) = PUMP.with(Cell::get) {
                due_at(Some(Instant::now()));
            }
        }
    }
    let _restore = Restore(POPUP_TICK.with(|t| t.replace(Some(p))), held());
    f()
}

/// Runs the tick `f` (no other starts inside it); a panic is stashed and the
/// held loop ended, so the pump comes back and unwinds there.
fn ticking(f: impl FnOnce()) {
    TICKING.with(|t| t.set(true));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    TICKING.with(|t| t.set(false));
    if let Err(p) = r {
        PANIC.with(|s| *s.borrow_mut() = Some(p));
        sys::arm(None);
        sys::end_loop();
    }
}

/// The tracking timer fired (the platform's callback, from the system's
/// loop): the pump overstayed, so it's held — a tick runs (the host's
/// context menu's own), or the waker asks for one.
pub(crate) fn fired() {
    let Some(deadline) = PUMP.with(Cell::get) else { return };
    if TICKING.with(Cell::get) {
        return;
    }
    if let Some(tick) = POPUP_TICK.with(Cell::get) {
        hold("the context menu");
        // SAFETY: lent by `with_popup_tick`, still running below (it clears
        // the pointer before returning); `TICKING` keeps it from re-entering.
        ticking(|| unsafe { (*tick)() });
        return;
    }
    let now = Instant::now();
    let Some(d) = deadline else { return };
    // (the pump's alarm clock rings: it's given back now — unless the
    // system holds it, which the next firing tells)
    if OWN_DEADLINE && !STOPPED.with(|s| s.replace(true)) {
        sys::stop_pump();
        return sys::arm(Some(now.max(d) + GRACE));
    }
    // (early: a timer fires at or after its date, but be sure)
    if now < d + GRACE {
        return sys::arm(Some(d + GRACE));
    }
    hold("the system's loop (the menu bar, a size / move)");
    if !WOKEN.with(|w| w.replace(true)) {
        WAKER.with(|w| {
            if let Some(w) = w.borrow().as_ref() {
                w();
            }
        });
    }
    // (until the tick runs and arms it for its own deadline)
    sys::arm(Some(now + RETRY));
}

/// The pump is held from now (`RAPIDR_HOST_LOG` says by what, once a pump).
fn hold(by: &str) {
    if !HELD.with(|h| h.replace(true)) && std::env::var_os("RAPIDR_HOST_LOG").is_some() {
        eprintln!("[rapidr] {by} holds the pump: the program's timers tick from the tracking timer");
    }
}

/// The waker's event arrived (the winit host's `UserEvent::Tick`): whether
/// to tick now — the pump is still held and no tick runs.
pub(crate) fn woken() -> bool {
    WOKEN.with(|w| w.set(false));
    held() && !TICKING.with(Cell::get)
}

/// A tick outside the host's context menu (delivered by the waker): `f`
/// runs it, panics stashed as the context menu's are.
pub(crate) fn run_tick(f: impl FnOnce()) {
    ticking(f);
}

/// After a tick: the timer armed for its next deadline, the held loop ended
/// if something waits for that.
pub(crate) fn after_tick(turn: Turn) {
    if PUMP.with(Cell::get).is_none() {
        return;
    }
    sys::arm(turn.next);
    if turn.end_loop {
        sys::end_loop();
    }
}

/// The headless host's pretend hold (a GUI test's `__hold_ms`): the pump
/// is held for `hold`, ticking through `tick` as the tracking timer would —
/// at the pump's own deadline (`first`), then at each tick's answer.
pub fn simulate_hold(hold: Duration, first: Option<Instant>, mut tick: impl FnMut() -> Turn) {
    let end = Instant::now() + hold;
    // (no system loop, no platform timer: a panic just unwinds; a request
    // to end the loop has nothing to end)
    struct Back;
    impl Drop for Back {
        fn drop(&mut self) {
            PUMP.with(|p| p.set(None));
            HELD.with(|h| h.set(false));
            TICKING.with(|t| t.set(false));
        }
    }
    let _back = Back;
    PUMP.with(|p| p.set(Some(first)));
    HELD.with(|h| h.set(true));
    let mut next = first;
    loop {
        let now = Instant::now();
        let at = next.map_or(end, |n| n.min(end));
        if at > now {
            std::thread::sleep(at - now);
        }
        if Instant::now() >= end {
            return;
        }
        TICKING.with(|t| t.set(true));
        next = tick().next;
        TICKING.with(|t| t.set(false));
    }
}

/// The platform's timer the system's loops dispatch, and how a held loop
/// is ended.
#[cfg(target_os = "macos")]
mod sys {
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::time::Instant;

    use core_foundation_sys::date::CFAbsoluteTimeGetCurrent;
    use core_foundation_sys::runloop::{kCFRunLoopCommonModes, CFRunLoopAddTimer, CFRunLoopGetMain, CFRunLoopTimerCreate, CFRunLoopTimerRef, CFRunLoopTimerSetNextFireDate};
    use objc2::runtime::AnyObject;
    use objc2::{msg_send, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType};
    use objc2_foundation::NSPoint;

    thread_local! {
        static TIMER: Cell<CFRunLoopTimerRef> = const { Cell::new(std::ptr::null_mut()) };
        /// The context menu tracked now (its `NSMenu`), to cancel.
        pub(super) static POPUP_MENU: Cell<*mut c_void> = const { Cell::new(std::ptr::null_mut()) };
    }

    /// "Never" for a run-loop timer.
    const FAR: f64 = f64::MAX;

    extern "C" fn fire(_timer: CFRunLoopTimerRef, _info: *mut c_void) {
        super::fired();
    }

    /// The timer (made once, on the main run loop, in the common modes:
    /// AppKit's event-tracking and modal-panel modes run it too).
    fn timer() -> CFRunLoopTimerRef {
        TIMER.with(|t| {
            if t.get().is_null() {
                // SAFETY: a repeating timer (an interval far off: its next
                // date is always set explicitly) with no context, added to
                // the main run loop for good — never released or invalidated.
                unsafe {
                    let timer = CFRunLoopTimerCreate(std::ptr::null(), FAR, 1.0e9, 0, 0, fire, std::ptr::null_mut());
                    CFRunLoopAddTimer(CFRunLoopGetMain(), timer, kCFRunLoopCommonModes);
                    t.set(timer);
                }
            }
            t.get()
        })
    }

    pub fn arm(at: Option<Instant>) {
        // (nothing to disarm before the first arming)
        if at.is_none() && TIMER.with(Cell::get).is_null() {
            return;
        }
        let date = match at {
            // SAFETY: CFAbsoluteTimeGetCurrent has no preconditions.
            Some(at) => (unsafe { CFAbsoluteTimeGetCurrent() }) + at.saturating_duration_since(Instant::now()).as_secs_f64(),
            None => FAR,
        };
        // SAFETY: the timer is live (never released).
        unsafe { CFRunLoopTimerSetNextFireDate(timer(), date) };
    }

    /// The menu bar's and the tracked context menu's tracking cancelled
    /// (`-[NSMenu cancelTracking]`: the menu closes, its loop ends).
    pub fn end_loop() {
        let Some(mtm) = MainThreadMarker::new() else { return };
        if let Some(main) = NSApplication::sharedApplication(mtm).mainMenu() {
            main.cancelTracking();
        }
        let popup = POPUP_MENU.with(Cell::get).cast::<AnyObject>();
        if !popup.is_null() {
            // SAFETY: muda's NSMenu, set only while it's shown (menu.rs), on
            // the main thread.
            unsafe {
                let _: () = msg_send![popup, cancelTracking];
            }
        }
    }

    /// The pump's deadline (the timer is its alarm clock): `NSApp.run`
    /// stops after the event at hand — one posted, as winit does — so the
    /// pump comes back whether the loop slept or was busy.
    pub fn stop_pump() {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let app = NSApplication::sharedApplication(mtm);
        app.stop(None);
        let event = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(NSEventType::ApplicationDefined, NSPoint::new(0.0, 0.0), NSEventModifierFlags(0), 0.0, 0, None, 0, 0, 0);
        if let Some(event) = event {
            app.postEvent_atStart(&event, true);
        }
    }
}

#[cfg(target_os = "windows")]
mod sys {
    use std::cell::Cell;
    use std::time::Instant;

    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{EndMenu, KillTimer, SetTimer, USER_TIMER_MINIMUM};

    thread_local! {
        /// The thread timer's id (0: none).
        static ID: Cell<usize> = const { Cell::new(0) };
    }

    unsafe extern "system" fn fire(_hwnd: HWND, _msg: u32, _id: usize, _time: u32) {
        super::fired();
    }

    pub fn arm(at: Option<Instant>) {
        match at {
            Some(at) => {
                let ms = at.saturating_duration_since(Instant::now()).as_millis().clamp(u128::from(USER_TIMER_MINIMUM), u128::from(u32::MAX >> 1)) as u32;
                // SAFETY: a thread timer (no window): its WM_TIMER, dispatched
                // by whatever loop runs, calls `fire`; the id given back
                // replaces the one set before.
                let id = unsafe { SetTimer(0, ID.with(Cell::get), ms, Some(fire)) };
                ID.with(|i| i.set(id));
            }
            None => {
                let id = ID.with(|i| i.replace(0));
                if id != 0 {
                    // SAFETY: the thread timer set above.
                    unsafe { KillTimer(0, id) };
                }
            }
        }
    }

    /// The menu tracked by this thread closed (`EndMenu`); Windows' size /
    /// move loop ends when the user lets go.
    pub fn end_loop() {
        // SAFETY: EndMenu takes no arguments; nothing happens without a menu.
        unsafe { EndMenu() };
    }

    /// (winit's own timeout gives Windows' pumps back: see `OWN_DEADLINE`)
    pub fn stop_pump() {}
}

/// Elsewhere no system loop holds the pump (the kernel draws the menus; X11
/// and Wayland resize without a modal loop): nothing to arm.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod sys {
    use std::time::Instant;

    pub fn arm(_at: Option<Instant>) {}
    pub fn end_loop() {}
    pub fn stop_pump() {}
}

/// macOS: the context menu muda shows now (its `NSMenu`), for
/// [`Turn::end_loop`]; null when it's gone.
#[cfg(target_os = "macos")]
pub(crate) fn set_popup_menu(menu: *mut std::ffi::c_void) {
    sys::POPUP_MENU.with(|m| m.set(menu));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pump_is_held_only_once_it_overstays() {
        let _p = begin_pump(Some(Instant::now()));
        assert!(!held());
        // (the timer can't fire before its date: called early, it waits)
        PUMP.with(|p| p.set(Some(Some(Instant::now() + Duration::from_secs(5)))));
        fired();
        assert!(!held());
        PUMP.with(|p| p.set(Some(Some(Instant::now() - Duration::from_secs(1)))));
        let woke = std::rc::Rc::new(Cell::new(0));
        let w = woke.clone();
        set_waker(Box::new(move || w.set(w.get() + 1)));
        fired();
        assert!(held());
        // (one wake-up until its tick ran)
        fired();
        assert_eq!(woke.get(), 1);
        assert!(woken());
        assert!(!WOKEN.with(Cell::get));
        drop(_p);
        assert!(!held());
        assert!(!woken(), "no tick once the pump is back");
        WAKER.with(|w| w.borrow_mut().take());
    }

    #[test]
    fn the_context_menus_own_tick_runs_inside_it_never_inside_itself() {
        let _p = begin_pump(Some(Instant::now()));
        let mut ticks = 0;
        with_popup_tick(&mut || {
            ticks += 1;
            assert!(held());
            // (a timer firing inside the tick doesn't start another)
            fired();
        }, || {
            fired();
            fired();
        });
        assert_eq!(ticks, 2);
        assert!(!held(), "held only while the menu is");
        assert!(POPUP_TICK.with(Cell::get).is_none());
        // (no pump: nothing ticks)
        drop(_p);
        let mut n = 0;
        with_popup_tick(&mut || n += 1, fired);
        assert_eq!(n, 0);
    }

    #[test]
    fn a_ticks_panic_waits_for_the_pump_to_come_back() {
        let _p = begin_pump(Some(Instant::now()));
        with_popup_tick(&mut || panic!("in a tick"), fired);
        assert!(!TICKING.with(Cell::get));
        assert!(take_panic().is_some());
        assert!(take_panic().is_none());
    }

    #[test]
    fn a_pretend_hold_ticks_at_each_answer_until_it_ends() {
        let start = Instant::now();
        let mut at = Vec::new();
        simulate_hold(Duration::from_millis(120), Some(start), || {
            assert!(held());
            at.push(start.elapsed());
            Turn { next: Some(Instant::now() + Duration::from_millis(30)), end_loop: false }
        });
        assert!(!held());
        assert!((3..=5).contains(&at.len()), "{at:?}");
        assert!(start.elapsed() >= Duration::from_millis(120));
    }
}
