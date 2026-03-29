//! RapidR runtime core — the `Value` type, BASIC builtins, and component system.
//!
//! Generated Rust programs `use rapidr_runtime_core::prelude::*` and
//! operate on `Value` instances instead of raw Rust types, preserving
//! BASIC semantics for arithmetic, string operations, comparisons, and
//! late-binding behaviour.

mod builtins;
mod file_io;
pub mod object;
mod value;

#[cfg(feature = "database")]
pub mod database;

#[cfg(feature = "network")]
pub mod network;

#[cfg(feature = "gui")]
pub mod gui;

#[cfg(feature = "datascience")]
pub mod datascience;

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod prelude {
    pub use crate::builtins::*;
    pub use crate::file_io::*;
    pub use crate::object::{
        is_component_method, is_component_type, rp_bind_event, rp_bind_event_1,
        rp_bind_event_2, rp_bind_event_3, rp_bind_event_4, rp_bind_event_5,
        rp_comp_get, rp_comp_method, rp_comp_set,
        rp_create_component, rp_fire_event, rp_fire_event_1, rp_fire_event_2, rp_fire_event_5,
        rp_run_app,
    };
    pub use crate::value::{v_bool, v_dbl, v_int, v_null, v_str, Value};

    #[cfg(feature = "gui")]
    pub use crate::gui::{set_theme, gui_register_timer};

    #[cfg(feature = "ffi")]
    pub use crate::ffi::{ffi_call, ffi_unload};
}
