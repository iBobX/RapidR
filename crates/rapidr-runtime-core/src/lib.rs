//! RapidR runtime core — the `Value` type, BASIC builtins, and component system.
//!
//! Generated Rust programs `use rapidr_runtime_core::prelude::*` and
//! operate on `Value` instances instead of raw Rust types, preserving
//! BASIC semantics for arithmetic, string operations, comparisons, and
//! late-binding behaviour.

mod builtins;
mod file_io;
pub mod object;
pub mod layout;
pub mod scroll;
pub mod mdi;
pub mod globals;
pub(crate) mod sound;
/// QDXJOYSTICK's gamepads (the DirectX lane's).
pub mod joystick;
pub mod terminal;
pub use rapidr_value as value;

#[cfg(feature = "database")]
pub mod database;

#[cfg(feature = "network")]
pub mod network;

/// The desktop host: the UI kernel's windows.
#[cfg(feature = "gui")]
pub mod ui;

/// QDXSCREEN / QDXTIMER on the desktop (the DirectX lane's).
#[cfg(feature = "gui")]
pub mod directx;

#[cfg(feature = "datascience")]
pub mod datascience;

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod prelude {
    pub use crate::builtins::*;
    pub use crate::file_io::*;
    pub use crate::sound::rp_playwav;
    pub use crate::object::{
        is_component_method, is_component_type, rp_bind_event, rp_bind_event_1,
        rp_bind_event_2, rp_bind_event_3, rp_bind_event_4, rp_bind_event_5,
        rp_bind_event_indirect, rp_clear_event_dispatcher, rp_mark_shutting_down,
        rp_set_event_dispatcher, rp_stop_all_timers,
        rp_bind_event_closure, rp_bind_event_out, rp_bind_event_indirect_this, rp_comp_get, rp_comp_method, rp_comp_set, rp_comp_value, rp_component_array,
        rp_create_component, rp_fire_event, rp_fire_event_1, rp_fire_event_2, rp_fire_event_5,
        rp_run_app,
    };
    pub use crate::value::{rp_fixed_string, rp_new_array, v_bool, v_dbl, v_int, v_null, v_str, Value};
    // SUBI / FUNCTIONI arguments
    pub use crate::value::variadic;
    // DATA / READ / RESTORE
    pub use crate::value::data;
    pub use crate::value::console;
    pub use crate::value::numeric;
    pub use crate::value::memory;
    pub use crate::value::{input_value, obj_field, rp_inv, rp_last_of_type, rp_new_object, rp_new_object_array, rp_redim, rp_shl, rp_shr, set_obj_field};

    #[cfg(feature = "gui")]
    pub use crate::ui::{set_theme, gui_register_timer};

    #[cfg(feature = "ffi")]
    pub use crate::ffi::{ffi_call, ffi_unload};
}
