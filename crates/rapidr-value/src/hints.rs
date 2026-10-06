//! Hints as RapidQ's runtime (Delphi's VCL) keeps them, for every runtime:
//! a component's `Hint` is "short|long" (the short part shows in the
//! tooltip, the long part is the application's hint), and a form's
//! `OnHint (Hint AS STRING)` is the application's one hint event.
//!
//! What RC.EXE's programs do (a probe in the Windows VM moving the real
//! mouse): the mouse coming onto a component whose hint differs from the
//! last one fires OnHint with the **long** hint — whatever its ShowHint —
//! and moving onto the form's open area (no hint) fires it with "". The
//! hint of a component without one is its parent's (up to the form). As
//! the VCL's `Application.OnHint`, there is one handler for the whole
//! program: the one bound last, whichever form it was bound on, hears the
//! hints of every form ([`set_receiver`]).

use std::cell::RefCell;

thread_local! {
    /// The form whose OnHint handler was bound last.
    static RECEIVER: RefCell<Option<String>> = const { RefCell::new(None) };
    /// The application's hint now (the last OnHint's argument).
    static CURRENT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// A form's OnHint handler was bound: it hears every hint from now on.
pub fn set_receiver(form: &str) {
    RECEIVER.with(|r| *r.borrow_mut() = Some(form.to_ascii_lowercase()));
}

/// The form whose OnHint hears the hints (none bound: nobody).
pub fn receiver() -> Option<String> {
    RECEIVER.with(|r| r.borrow().clone())
}

/// The application's hint becomes `long`: whether it changed (OnHint
/// fires only then).
pub fn change(long: &str) -> bool {
    CURRENT.with(|c| {
        let mut c = c.borrow_mut();
        if *c == long {
            false
        } else {
            *c = long.to_string();
            true
        }
    })
}

/// The part of a hint the tooltip shows: before the first `|` (all of it
/// without one) — the VCL's GetShortHint.
pub fn short_hint(hint: &str) -> &str {
    hint.split_once('|').map_or(hint, |(s, _)| s)
}

/// The application's part of a hint: after the first `|` (all of it
/// without one) — the VCL's GetLongHint.
pub fn long_hint(hint: &str) -> &str {
    hint.split_once('|').map_or(hint, |(_, l)| l)
}

/// The VCL's hint timings (Application.HintPause, HintHidePause), in
/// milliseconds: a tooltip shows after the mouse rests this long on a
/// component, and goes after the second.
pub const HINT_PAUSE_MS: i64 = 500;
pub const HINT_HIDE_PAUSE_MS: i64 = 2500;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_and_long() {
        assert_eq!((short_hint("short|long text"), long_hint("short|long text")), ("short", "long text"));
        assert_eq!((short_hint("second"), long_hint("second")), ("second", "second"));
        assert_eq!((short_hint("a|b|c"), long_hint("a|b|c")), ("a", "b|c"));
        assert_eq!((short_hint(""), long_hint("")), ("", ""));
    }

    #[test]
    fn changes_only() {
        assert!(change("long text"));
        assert!(!change("long text"));
        assert!(change(""));
        assert!(!change(""));
        set_receiver("Form2");
        assert_eq!(receiver().as_deref(), Some("form2"));
    }
}
