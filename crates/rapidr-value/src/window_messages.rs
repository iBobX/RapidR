//! A form's WndProc on Windows (RapidQ's `QFORM.WndProc = Handler`, the
//! event `OnWndProc` after `rapidr_ast::tray_calls`): the messages Windows
//! sends and posts to the form's window, handed to the handler as
//! (hWnd, uMsg, wParam, lParam). RC.EXE, checked in the Windows VM
//! (2026-10-18), hands over every message its form's window procedure
//! gets: the system's (WM_SIZE, WM_MOVE, WM_ACTIVATE, WM_PAINT, the mouse
//! and the keys …) and the ones a program or a driver posts or sends
//! (WM_USER + n, winmm's MM_MIM_DATA): uMsg, wParam and lParam as 32-bit
//! signed numbers, the handler's result ignored, Windows' own processing
//! going on as usual.
//!
//! The desktop host on Windows listens to the windows of the forms named
//! here ([`listen`]: the runtimes call it when a form's OnWndProc is
//! bound) and keeps what [`heard`] lets through; runtime-core hands it to
//! the handler after the host's turn. Other systems have no Windows
//! messages: there a form's WndProc hears only its system tray icon's
//! clicks (`crate::tray`).

use std::cell::{Cell, RefCell};
use std::collections::HashSet;

/// Windows' WM_PAINT.
pub const WM_PAINT: u32 = 0x000F;
/// Windows' WM_USER: the first of a window's own messages.
pub const WM_USER: u32 = 0x0400;

thread_local! {
    static LISTENING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static REVISION: Cell<u64> = const { Cell::new(0) };
}

/// Form `form`'s WndProc was bound: its window's messages are wanted.
pub fn listen(form: &str) {
    let fresh = LISTENING.with(|l| l.borrow_mut().insert(form.to_ascii_lowercase()));
    if fresh {
        REVISION.with(|r| r.set(r.get() + 1));
    }
}

/// Whether form `form`'s window's messages are wanted.
pub fn listens(form: &str) -> bool {
    LISTENING.with(|l| l.borrow().contains(&form.to_ascii_lowercase()))
}

/// Changes with every form [`listen`] adds (the host looks again then).
pub fn revision() -> u64 {
    REVISION.with(Cell::get)
}

/// Whether message `msg`, arriving at a form's window, reaches its WndProc.
/// Every message does, as in RapidQ, except one: a WM_PAINT with nothing
/// to paint (`update` false: no part of the window invalid). That is
/// RapidR's own redraw — RapidR draws a form's controls in the form's
/// window, so a control changed by the handler asks the window to paint
/// again; RapidQ's controls are windows of their own and paint themselves
/// without telling the form. Handed over, it would come back for ever to a
/// handler that changes a control on every message. A WM_PAINT the system
/// sends because part of the window must be painted again (shown,
/// uncovered, made bigger) reaches it.
pub fn heard(msg: u32, update: bool) -> bool {
    msg != WM_PAINT || update
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_is_heard_but_rapidrs_own_repaints() {
        // (the system's, the program's, a driver's: as RC.EXE's QFORM hears them)
        for msg in [0x0005, 0x0003, 0x0006, 0x0014, 0x0024, 0x0046, 0x0047, 0x0083, 0x0084, 0x0085, 0x0100, 0x0102, 0x0200, 0x0020, 0x03C3, WM_USER, WM_USER + 1, 0x7FFF, 0xC123] {
            assert!(heard(msg, false), "{msg:#x}");
            assert!(heard(msg, true), "{msg:#x}");
        }
        // (WM_PAINT: when the system wants part of the window painted, not
        // when RapidR redraws its controls)
        assert!(heard(WM_PAINT, true));
        assert!(!heard(WM_PAINT, false));
    }

    #[test]
    fn forms_listen_by_name_and_the_revision_moves_once() {
        assert!(!listens("wpform"));
        let r = revision();
        listen("WPForm");
        assert!(listens("wpform") && listens("WPFORM"));
        assert_eq!(revision(), r + 1);
        listen("wpform");
        assert_eq!(revision(), r + 1);
    }
}
