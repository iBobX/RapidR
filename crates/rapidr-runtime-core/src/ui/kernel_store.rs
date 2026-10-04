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

/// The Color the registry gives a new QLABEL, QFORM and QPANEL
/// (`RpComponent::new`), which isn't painted: read as unset (a label
/// has no background; a form and a panel are the button face) — unless the
/// program set it (`__colorset`, object.rs), white included.
const LABEL_DEFAULT_COLOR: i64 = 0xFFFFFF;

/// A kernel-drawn dialog's component (`rapidr:…`): its own store answers
/// (the dialogs lane's `kernel/dialogs.rs`).
fn dialog<R>(id: &str, f: impl FnOnce(&dyn Store) -> R) -> Option<R> {
    if !rapidr_ui_kernel::dialogs::is_dialog(id) {
        return None;
    }
    super::kernel::dialogs::with_store(id, f)
}

impl Store for RtStore {
    fn get(&self, id: &str, prop: &str) -> Value {
        if let Some(v) = dialog(id, |s| s.get(id, prop)) {
            return v;
        }
        let v = rp_comp_get(id, prop);
        if prop.eq_ignore_ascii_case("color")
            && matches!(v, Value::Integer(LABEL_DEFAULT_COLOR))
            && matches!(rp_comp_type(id).as_str(), "RLABEL" | "RFORM" | "RPANEL")
            && !rp_comp_get(id, "__colorset").to_bool()
        {
            return Value::Null;
        }
        v
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
