//! The kernel's view of the program's components: `rapidr_ui_kernel::Store`
//! over the component registry (`rp_comp_get`, `rp_comp_type`,
//! `get_children_of`). Reading never runs program code, so the host's
//! callbacks may read through it inside a pump.
//!
//! A property the program never set reads as `Null` where the kernel needs
//! to tell (it then uses RapidQ's default): a QLABEL's Color is the one
//! the registry fills in at creation (white), which the kernel would paint
//! as a background a label never has, so it reads as `Null` there.

use rapidr_ui_kernel::Store;
use rapidr_value::objects::font::Font;
use rapidr_value::Value;

use crate::object::{get_children_of, rp_comp_get, rp_comp_type};

/// The runtime's components, as the kernel reads them.
pub struct RtStore;

/// A kernel-drawn dialog's component (`rapidr:…`): its own store answers
/// (the open dialogs are `rapidr_ui_app::dialogs`').
fn dialog<R>(id: &str, f: impl FnOnce(&dyn Store) -> R) -> Option<R> {
    if !rapidr_ui_kernel::dialogs::is_dialog(id) {
        return None;
    }
    rapidr_ui_app::dialogs::with_store(id, f)
}

impl Store for RtStore {
    fn get(&self, id: &str, prop: &str) -> Value {
        if let Some(v) = dialog(id, |s| s.get(id, prop)) {
            return v;
        }
        // (a Color the program never set reads Null here — the kernel's
        // default: a form's and a panel's face, no background for a label —
        // while the program reads RapidQ's clBtnFace / clWindow / its
        // parent's: object.rs `program_color`)
        rp_comp_get(id, prop)
    }

    fn type_of(&self, id: &str) -> String {
        if let Some(t) = dialog(id, |s| s.type_of(id)) {
            return t;
        }
        rp_comp_type(id).to_ascii_uppercase()
    }

    fn children(&self, id: &str) -> Vec<(String, String)> {
        if let Some(c) = dialog(id, |s| s.children(id)) {
            return c;
        }
        get_children_of(id)
    }

    fn font(&self, id: &str) -> Font {
        if let Some(f) = dialog(id, |s| s.font(id)) {
            return f;
        }
        rapidr_value::objects::font_from_props(id, &|i, p| rp_comp_get(i, p))
    }
}

/// A true / false property as the kernel reads them (`default` when unset).
pub fn flag(id: &str, prop: &str, default: bool) -> bool {
    rapidr_ui_kernel::store::flag(&RtStore, id, prop, default)
}
