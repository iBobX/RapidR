//! RapidR web runtime — the `Value` type, BASIC builtins, DOM-based component
//! system, and browser API wrappers compiled to WebAssembly.
//!
//! Generated Rust/WASM programs `use rapidr_runtime_web::prelude::*` and
//! operate on `Value` instances identical to the desktop runtime, but with
//! the GUI, network, and I/O layers replaced by browser APIs.

mod builtins;
pub mod database_web;
pub mod datascience_web;
mod file_io_web;
pub mod gui_web;
pub mod network_web;
pub mod object_web;
pub mod storage_web;
pub use rapidr_value as value;
pub use rapidr_rrcss::RR_BASE_CSS;

pub mod prelude {
    // Value type + constructors
    pub use crate::value::{v_bool, v_dbl, v_int, v_null, v_str, Value};

    // BASIC builtins (string / math / date / conversion / etc.)
    pub use crate::builtins::*;

    // File I/O stubs
    pub use crate::file_io_web::*;

    // Component system — identical function names as the desktop runtime
    pub use crate::object_web::{
        get_children_of, is_component_method, is_component_type, rp_bind_event,
        rp_bind_event_1, rp_bind_event_2, rp_bind_event_3, rp_bind_event_4,
        rp_bind_event_5, rp_bind_event_indirect, rp_clear_event_dispatcher,
        rp_set_event_dispatcher, rp_comp_get, rp_comp_method, rp_comp_set,
        rp_create_component, rp_fire_event, rp_fire_event_1, rp_fire_event_2,
        rp_fire_event_5, rp_run_app,
    };

    // GUI helpers
    pub use crate::object_web::{gui_register_timer, set_theme};
    pub use crate::gui_web::{gui_web_finalize, gui_web_set_parent, gui_web_show_form};
}
