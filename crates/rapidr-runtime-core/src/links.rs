//! (I7) Linked data on the desktop — native builds and the interpreter:
//! the shared model (`rapidr_value::datascience::link`, the web's and the
//! designer's too) says what changed; this applies it with the runtime's
//! own means — a bound control's property set (`rp_comp_set`), a grid drawn
//! again, an event fired, a message on stderr.
//!
//! Called after every component property set and method call
//! (`rp_comp_set`, `rp_comp_method`): when nothing data-related changed it
//! is one flag test.

use std::cell::Cell;

use rapidr_value::datascience::link::{self, Update};

thread_local! {
    /// Inside [`apply`] (a bound control's own set comes back here).
    static APPLYING: Cell<bool> = const { Cell::new(false) };
    static INSTALLED: Cell<bool> = const { Cell::new(false) };
}

/// The runtime's side of the model, once: RJSON documents as sources
/// (called as an RJSON is made, before anything can read it).
pub(crate) fn install() {
    if !INSTALLED.with(|i| i.replace(true)) {
        link::set_json_reader(crate::object::json_document);
    }
}

/// What the linked data changed, carried out (bound controls, grids,
/// events, messages) until nothing is left (a handler may change more).
pub fn apply() {
    if APPLYING.with(Cell::get) || !link::pending() {
        return;
    }
    install();
    APPLYING.with(|a| a.set(true));
    for _ in 0..8 {
        let updates = link::sync();
        if updates.is_empty() {
            break;
        }
        for u in updates {
            match u {
                Update::Set(id, prop, value) => crate::object::rp_comp_set(&id, &prop, value),
                #[cfg(feature = "gui")]
                Update::Redraw(id) => crate::ui::redraw_widget(&id),
                #[cfg(not(feature = "gui"))]
                Update::Redraw(_) => {}
                Update::Fire(id, event, args) => crate::object::rp_fire_event_args(&id, &event, &args),
                Update::Warn(text) => eprintln!("[WARN] {text}"),
            }
        }
    }
    APPLYING.with(|a| a.set(false));
}
