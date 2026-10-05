//! The waits a VM serves itself — their bookkeeping, which never blocks
//! (moved from runtime-core's `ui/kernel.rs`). An interpreter can't wait
//! inside a builtin while its handlers run: with cooperative waits on
//! ([`set_cooperative`]), ShowModal, DOEVENTS, a Popup, a dialog
//! (MESSAGEBOX … , the file / colour / font dialogs: `dialogs.rs`) and
//! INPUT$ leave a [`Wait`] here and return; the VM then serves the
//! innermost one, a turn at a time — its timers' handlers run between the
//! turns — until it's [`over`] or [`answered`] (the desktop's
//! `gui_pump_wait`), and the wait's result replaces the builtin's. A native
//! build waits in place instead (the runtime's own loop).
//!
//! The web's VM runs in time slices and suspends on a ShowModal: the same
//! stack tells its page loop what the VM waits for (docs/web-host-plan.md
//! §3.2, W4).

use std::cell::RefCell;

use rapidr_ui_kernel::tick::Instant;
use rapidr_value::{v_int, v_null, Value};

use crate::{dialogs, forms, Program, Windows};

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
    /// `DOEVENTS`, called then: a turn — and the timers due then fire
    /// before it ends, each once the handler before it has run
    /// (`timers::due_since`).
    Once(Instant),
    /// A dialog the program waits for (`dialogs::Pending::Open`): until it
    /// answers; its result is the builtin's (`dialogs::finished`).
    Dialog(u64),
    /// INPUT$'s: until a key reaches INKEY$'s queue (1), or no window is
    /// left (0).
    Key,
    /// `PopupMenu.Popup` with a pop-up menu the kernel draws on `form`
    /// while the program waits (a real screen without native context
    /// menus): until it closes, its pick dispatched.
    Menu(String),
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

/// For the bytecode VM: `ShowModal` (and DOEVENTS, Popup, the dialogs,
/// INPUT$) returns at once and leaves its wait to the VM.
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

/// The innermost wait if it's a turn's (DOEVENTS', Popup's): the runtime
/// takes it off ([`pop`]) when the turn is over.
pub fn turn() -> Option<Wait> {
    ws(|s| s.waits.last().filter(|w| matches!(w, Wait::Popup | Wait::Once(_))).cloned())
}

/// Whether the innermost wait is over: none left, its modal form closed,
/// no window left for the main loop. (A wait for an answer ends with its
/// result: [`answered`].)
pub fn over() -> bool {
    let last = ws(|s| s.waits.last().cloned());
    match last {
        None => true,
        Some(Wait::Form(name)) => !forms::modal_waits(&name),
        Some(Wait::App) => !forms::any_shown(),
        Some(Wait::Popup | Wait::Once(_)) => true,
        Some(Wait::Dialog(_) | Wait::Key | Wait::Menu(_)) => false,
    }
}

/// The innermost wait's result, taken off, if it waits for an answer that
/// has come: a dialog's (the builtin's result, `dialogs::finished`),
/// INPUT$'s (1: a key in INKEY$'s queue; 0: no window left), a kernel-drawn
/// menu's (closed). `None`: no such wait, or no answer yet.
pub fn answered<R: Program + Windows>(rt: R) -> Option<Value> {
    let result = match ws(|s| s.waits.last().cloned())? {
        Wait::Dialog(id) => dialogs::finished(id)?,
        Wait::Key if rapidr_value::console::key_waiting() => v_int(1),
        Wait::Key if !forms::any_shown() => v_int(0),
        Wait::Menu(form) if !rt.popup_open(&form) || !forms::form_shown(&form) => v_null(),
        _ => return None,
    };
    pop();
    Some(result)
}

/// The innermost wait, taken off.
pub fn pop() -> Option<Wait> {
    ws(|s| s.waits.pop())
}
