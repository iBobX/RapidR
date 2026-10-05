//! RapidR web runtime — the `Value` type, BASIC builtins, DOM-based component
//! system, and browser API wrappers compiled to WebAssembly.
//!
//! Generated Rust/WASM programs `use rapidr_runtime_web::prelude::*` and
//! operate on `Value` instances identical to the desktop runtime, but with
//! the GUI, network, and I/O layers replaced by browser APIs.

pub mod a11y_web;
mod builtins;
pub mod database_web;
pub mod datascience_web;
pub mod dialog_web;
/// QDXSCREEN / QDXTIMER in the browser (the DirectX lane's).
pub mod directx_web;

/// The INPUT statement's line (for `rapidr-vm-host-web`).
pub fn builtins_input_line() -> value::Value {
    builtins::rp_input_line()
}
mod file_io_web;
pub mod globals_web;
pub mod gui_web;
/// The UI kernel as the page's GUI host (`?host=kernel`, Stage W3).
#[cfg(feature = "kernel")]
pub mod kernel_web;
pub mod layout_web;
pub mod mdi_web;
pub mod menu_web;
pub mod network_web;
pub mod object_web;
pub mod scroll_web;
pub mod storage_web;
pub use rapidr_value as value;
pub use rapidr_rrcss::RR_BASE_CSS;

pub mod prelude {
    // Value type + constructors
    pub use crate::value::{rp_fixed_string, rp_new_array, v_bool, v_dbl, v_int, v_null, v_str, Value};
    // SUBI / FUNCTIONI arguments
    pub use crate::value::variadic;
    // DATA / READ / RESTORE
    pub use crate::value::data;
    pub use crate::value::numeric;
    pub use crate::value::memory;
    pub use crate::value::console;
    pub use crate::value::{input_value, obj_field, rp_inv, rp_new_object, rp_new_object_array, rp_redim, rp_shl, rp_shr, set_obj_field};

    // BASIC builtins (string / math / date / conversion / etc.)
    pub use crate::builtins::*;

    // File I/O stubs
    pub use crate::file_io_web::*;

    // Component system — identical function names as the desktop runtime
    pub use crate::object_web::{
        get_children_of, is_component_method, is_component_type, rp_bind_event,
        rp_bind_event_1, rp_bind_event_2, rp_bind_event_3, rp_bind_event_4,
        rp_bind_event_5, rp_bind_event_indirect, rp_clear_event_dispatcher,
        rp_set_event_dispatcher, rp_bind_event_closure, rp_bind_event_out, rp_bind_event_indirect_this, rp_comp_call, rp_comp_get, rp_comp_method, rp_comp_read, rp_comp_set, rp_comp_value, rp_component_array,
        rp_create_component, rp_fire_event, rp_fire_event_1, rp_fire_event_2,
        rp_fire_event_5, rp_run_app, rp_comp_get_all_properties,
    };

    // GUI helpers
    pub use crate::object_web::{gui_register_timer, set_theme};
    pub use crate::gui_web::{gui_web_finalize, gui_web_set_parent, gui_web_show_form};
}
