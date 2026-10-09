//! Dragging a QBUTTON, the two ways RapidQ's runtime (Delphi's VCL) has —
//! for every runtime (the UI kernel's input does the work: `drag.rs`
//! there).
//!
//! What RC.EXE's programs do (probes in the Windows VM, messages and the
//! real mouse):
//!
//! - **OnStartDrag bound** makes the button a drag source (the VCL's
//!   DragMode dmAutomatic): a left press on it starts a drag at once —
//!   OnStartDrag fires, OnMouseDown doesn't — and its release ends it:
//!   OnEndDrag fires, OnMouseUp and OnClick don't. Both events have no
//!   arguments. Nothing accepts a drop (RapidQ has no OnDragOver), so the
//!   mouse shows the no-drop pointer meanwhile. Binding only OnEndDrag
//!   changes nothing.
//! - **`StartDrag`** (the manual's "a drag button", called from
//!   OnMouseDown while the button is held) moves the control with the
//!   mouse until it's let go — Windows' own move of a window by its title
//!   bar (SC_MOVE), which RapidQ's StartDrag is: it returns then, Left and
//!   Top moved by what the mouse moved, and no OnMouseUp or OnClick
//!   follows. Called when no button is held it does nothing. Escape puts
//!   the control back. (A QCOOLBTN's or QOVALBTN's StartDrag ends RC.EXE's
//!   programs; RapidR moves them as a QBUTTON.)

use std::cell::RefCell;
use std::collections::HashSet;

thread_local! {
    /// Components with an OnStartDrag handler (lowercase names).
    static SOURCES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Component `name`'s OnStartDrag handler was bound: it's a drag source.
pub fn set_source(name: &str) {
    SOURCES.with(|s| s.borrow_mut().insert(name.to_ascii_lowercase()));
}

/// Whether a left press on `name` starts a drag (it has an OnStartDrag
/// handler).
pub fn is_source(name: &str) -> bool {
    SOURCES.with(|s| s.borrow().contains(&name.to_ascii_lowercase()))
}

/// The components that can be moved by `StartDrag` (RapidQ's types that
/// have it).
pub fn can_start_drag(type_name: &str) -> bool {
    matches!(type_name.to_ascii_uppercase().as_str(), "RBUTTON" | "RCOOLBTN" | "ROVALBTN")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources() {
        assert!(!is_source("b1"));
        set_source("B1");
        assert!(is_source("b1"));
        assert!(can_start_drag("RCoolBtn") && !can_start_drag("RLABEL"));
    }
}
