//! RapidQ's form members that only some programs use — HideTitleBar,
//! ShowTitleBar, ShapeForm, QFORM's MDI methods (Cascade, Tile,
//! ArrangeIcons, Next, Previous) and properties (MDIChildCount, TileMode),
//! and a button's StartDrag — in the browser: the desktop's
//! (runtime-core's `form_members.rs`) over the same shared work
//! (`rapidr_value::layout`, `shape`, `mdi` and `drag`,
//! `rapidr_ui_app::forms`, the UI kernel).

use crate::object_web::rp_comp_get_stored;
use crate::value::{v_int, v_null, Value};

/// A form member's or StartDrag's call; `None` when `method` isn't one of
/// them for `comp_type`.
pub fn method(name: &str, comp_type: &str, method: &str, args: &[Value]) -> Option<Value> {
    let form = matches!(comp_type, "RFORM" | "RFORMMDI");
    match method {
        "hidetitlebar" | "showtitlebar" if form => crate::kernel_web::title_bar(name, method == "showtitlebar"),
        "shapeform" if form => {
            let source = args.first().cloned().unwrap_or(Value::String(String::new()));
            let transparent = args.get(1).map_or(0xFF_FFFF, Value::to_i64);
            match rapidr_value::shape::shape_form(name, &source, transparent) {
                Ok(_) => crate::kernel_web::shape_changed(name),
                Err(e) => web_sys::console::warn_1(&format!("[rapidr] {name}.ShapeForm: {e}").into()),
            }
        }
        m if form && rapidr_value::mdi::qform_method(m, 0).is_some() => {
            // (a QFORM has no MDI children: nothing to arrange; a QFORMMDI's)
            if rapidr_value::mdi::is_mdi(name) {
                let tile = tile_mode(name).to_i64();
                if let Some(mdi) = rapidr_value::mdi::qform_method(m, tile) {
                    crate::mdi_web::method(name, mdi, &[]);
                }
            }
        }
        "startdrag" if rapidr_value::drag::can_start_drag(comp_type) => crate::kernel_web::start_drag(name),
        _ => return None,
    }
    Some(v_null())
}

/// A form property of these; `None` when `prop` isn't one.
pub fn get(name: &str, comp_type: &str, prop: &str) -> Option<Value> {
    if !matches!(comp_type, "RFORM" | "RFORMMDI") {
        return None;
    }
    match prop {
        // (a QFORM's: none — RapidQ's FormStyle is fsNormal only)
        "mdichildcount" => Some(v_int(rapidr_value::mdi::get(name, "childcount").map_or(0, |v| v.to_i64()))),
        "tilemode" => Some(tile_mode(name)),
        _ => None,
    }
}

/// TileMode: tbHorizontal (0) until the program sets it.
fn tile_mode(name: &str) -> Value {
    match rp_comp_get_stored(name, "tilemode") {
        Value::Null => v_int(0),
        v => v,
    }
}
