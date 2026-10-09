//! RapidQ's form members that only some programs use — HideTitleBar,
//! ShowTitleBar, ShapeForm, QFORM's MDI methods (Cascade, Tile,
//! ArrangeIcons, Next, Previous) and properties (MDIChildCount, TileMode),
//! and a button's StartDrag — on the desktop. The work is shared with the
//! web runtime (`form_members_web.rs`): `rapidr_value::layout`, `shape`,
//! `mdi` and `drag`, `rapidr_ui_app::forms`, the UI kernel.

use crate::value::{v_int, v_null, Value};

/// A form member's or StartDrag's call; `None` when `method` isn't one of
/// them for `comp_type`.
pub fn method(name: &str, comp_type: &str, method: &str, args: &[Value]) -> Option<Value> {
    let form = matches!(comp_type, "RFORM" | "RFORMMDI");
    match method {
        "hidetitlebar" | "showtitlebar" if form => title_bar(name, method == "showtitlebar"),
        "shapeform" if form => shape_form(name, args),
        m if form && rapidr_value::mdi::qform_method(m, 0).is_some() => {
            // (a QFORM has no MDI children: nothing to arrange; a QFORMMDI's)
            if rapidr_value::mdi::is_mdi(name) {
                let tile = tile_mode(name).to_i64();
                if let Some(mdi) = rapidr_value::mdi::qform_method(m, tile) {
                    crate::mdi::method(name, mdi, &[]);
                }
            }
        }
        "startdrag" if rapidr_value::drag::can_start_drag(comp_type) => {
            #[cfg(feature = "gui")]
            crate::ui::gui_start_drag(name);
        }
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
    crate::object::stored(&name.to_lowercase(), "tilemode").unwrap_or(v_int(0))
}

fn title_bar(name: &str, show: bool) {
    #[cfg(feature = "gui")]
    crate::ui::gui_title_bar(name, show);
    #[cfg(not(feature = "gui"))]
    {
        use crate::object::{rp_comp_get, rp_comp_set};
        let border_style = rp_comp_get(name, "borderstyle").to_i64();
        if let Some(h) = rapidr_value::layout::title_bar_height_change(name, show, border_style, rp_comp_get(name, "height").to_i64()) {
            rp_comp_set(name, "height", v_int(h));
        }
    }
}

/// `Form.ShapeForm(Filename$|Resource, TransparentColor&)`: the outline
/// (rapidr_value::shape), shown by the host. A bitmap that can't be read
/// leaves the form as it was (RapidQ's runtime raises "Cannot open file
/// …", which a GUI program's message box reports before going on).
fn shape_form(name: &str, args: &[Value]) {
    let source = args.first().cloned().unwrap_or(Value::String(String::new()));
    let transparent = args.get(1).map_or(0xFF_FFFF, Value::to_i64);
    match rapidr_value::shape::shape_form(name, &source, transparent) {
        Ok(_) => {
            #[cfg(feature = "gui")]
            crate::ui::gui_shape_changed(name);
        }
        Err(e) => eprintln!("[rapidr] {name}.ShapeForm: {e}"),
    }
}
