//! (I7) Linked data in the browser: the shared model
//! (`rapidr_value::datascience::link`, the desktop's and the designer's
//! too) says what changed; this applies it with the page's own means — a
//! bound control's property set (`rp_comp_set`), the windows drawn again,
//! an event fired, a message on the console.
//!
//! Called after every component property set and method call: when
//! nothing data-related changed it is one flag test.

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
        link::set_json_reader(crate::object_web::json_document);
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
                Update::Set(id, prop, value) => crate::object_web::rp_comp_set(&id.to_uppercase(), &prop, value),
                Update::Redraw(_) => crate::kernel_web::redraw(),
                Update::Fire(id, event, args) => crate::object_web::rp_fire_event_args(&id.to_uppercase(), &event, &args),
                Update::Warn(text) => web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&format!("[WARN] {text}"))),
            }
        }
    }
    APPLYING.with(|a| a.set(false));
}
