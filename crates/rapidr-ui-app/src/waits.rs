//! The waits a VM serves itself — their bookkeeping, which never blocks
//! (moved from runtime-core's `ui/kernel.rs`). An interpreter can't wait
//! inside a builtin while its handlers run: with cooperative waits on
//! ([`set_cooperative`]), ShowModal, DOEVENTS and a native Popup leave a
//! [`Wait`] here and return; the VM then serves the innermost one, a turn at
//! a time, until it's [`over`] (the desktop's `gui_pump_wait`). A native
//! build waits in place instead (the runtime's own loop).
//!
//! The web's VM runs in time slices and suspends on a ShowModal: the same
//! stack tells its page loop what the VM waits for (docs/web-host-plan.md
//! §3.2, W4).

use std::cell::RefCell;

use crate::forms;

/// A wait the VM serves itself.
#[derive(Clone, Debug, PartialEq)]
pub enum Wait {
    /// `Form.ShowModal`: until the form is closed.
    Form(String),
    /// The program's main event loop: until no window is left.
    App,
    // (timers during native menu tracking: the VM waits between
    // instructions, so it can lend itself to a tracking tick — rather than
    // inside a builtin, where its handlers could only queue)
    /// `PopupMenu.Popup` with the host's context menu: one turn, the menu
    /// shown (and tracked) inside it, its pick dispatched.
    Popup,
    /// `DOEVENTS`: one turn.
    Once,
}

#[derive(Default)]
struct Waits {
    cooperative: bool,
    waits: Vec<Wait>,
    started: bool,
}

thread_local! {
    static WAITS: RefCell<Waits> = RefCell::new(Waits::default());
}

fn ws<R>(f: impl FnOnce(&mut Waits) -> R) -> R {
    WAITS.with(|w| f(&mut w.borrow_mut()))
}

/// For the bytecode VM: `ShowModal` returns at once and leaves its wait to
/// the VM.
pub fn set_cooperative(on: bool) {
    ws(|s| s.cooperative = on);
}

pub fn cooperative() -> bool {
    ws(|s| s.cooperative)
}

/// The operation running left wait `w` to the VM ([`take_started`] says
/// so once).
pub fn start(w: Wait) {
    ws(|s| {
        s.waits.push(w);
        s.started = true;
    });
}

/// Whether the last operation started a wait (asked once per operation).
pub fn take_started() -> bool {
    ws(|s| std::mem::replace(&mut s.started, false))
}

/// The program's main event loop begins (after the main program): a wait
/// for its windows, under any it starts.
pub fn begin_app() {
    ws(|s| s.waits.push(Wait::App));
}

/// The innermost wait, if it's one turn (DOEVENTS', Popup's), taken off:
/// whether it was Popup's.
pub fn take_turn() -> Option<bool> {
    ws(|s| match s.waits.last() {
        Some(Wait::Popup | Wait::Once) => Some(matches!(s.waits.pop(), Some(Wait::Popup))),
        _ => None,
    })
}

/// Whether the innermost wait is over: none left, its modal form closed,
/// no window left for the main loop.
pub fn over() -> bool {
    let last = ws(|s| s.waits.last().cloned());
    match last {
        None => true,
        Some(Wait::Form(name)) => !forms::form_shown(&name),
        Some(Wait::App) => !forms::any_shown(),
        Some(Wait::Popup | Wait::Once) => true,
    }
}

/// The innermost wait, taken off.
pub fn pop() -> Option<Wait> {
    ws(|s| s.waits.pop())
}
