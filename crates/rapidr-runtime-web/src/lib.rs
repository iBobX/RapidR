//! RapidR web runtime — the `Value` type, BASIC builtins, the component
//! registry, and browser API wrappers compiled to WebAssembly.
//!
//! Generated Rust/WASM programs `use rapidr_runtime_web::prelude::*` and
//! operate on `Value` instances identical to the desktop runtime. The
//! program's windows are the UI kernel's, drawn on the page as on the
//! desktop (`kernel_web`, docs/web-host-plan.md); network and I/O go through
//! browser APIs.

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
/// QOPENDIALOG / QSAVEDIALOG on the web: the user's real files, through the
/// browser's own pickers.
pub mod file_picker_web;
pub mod globals_web;
/// The I/O and media objects' devices (QCOMPORT, QDOWNLOAD, …).
pub mod io_web;
/// The media objects' devices (QMIDI, QWAVE).
pub mod media_web;
/// The UI kernel as the page's GUI host: the program's windows.
pub mod kernel_web;
pub mod layout_web;
pub mod mdi_web;
// (HideTitleBar, ShapeForm, QFORM's MDI members, StartDrag)
mod form_members_web;
// (I1: RDOCKMANAGER — rapidr_value::dock)
pub mod dock_web;
// (I1 / L-PANELS: RapidR Studio's panels — rapidr_value::panels)
pub mod panels_web;
pub mod studio_web;
pub use rapidr_studio;
pub mod network_web;
pub mod object_web;
/// The web-only components (RWEBVIEW, RDOM, media) as elements over
/// the UI kernel's canvases.
pub mod overlay_web;
/// The fallback fonts (Noto: symbols, CJK) fetched as text needs them.
pub mod fonts_web;
/// The page itself: its document, new elements.
pub mod page_web;
pub mod scroll_web;
pub mod storage_web;
pub mod tray_web;
/// RJAVASCRIPT, RWEBSTORAGE, RWEBNOTIFICATION, RWEBGEOLOCATION, RROUTER.
pub mod webapi_web;
pub use rapidr_value as value;

pub mod prelude {
    // Value type + constructors
    pub use crate::value::{rp_fixed_string, rp_new_array, v_bool, v_dbl, v_int, v_null, v_str, Value};
    // SUBI / FUNCTIONI arguments
    pub use crate::value::variadic;
    // DATA / READ / RESTORE
    pub use crate::value::data;
    pub use crate::value::numeric;
    pub use crate::value::tray;
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
        rp_fire_event_5, rp_comp_get_all_properties,
    };

    // GUI helpers
    pub use crate::object_web::{gui_register_timer, set_theme};
    // (the program's main code done: its windows shown; then the program's
    // end unless a form or a dialog is waited for)
    pub use crate::kernel_web::{finalize as gui_finalize, main_ended as gui_main_ended};
}
