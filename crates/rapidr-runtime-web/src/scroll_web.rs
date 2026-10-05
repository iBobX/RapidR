//! A QFORM's and a QSCROLLBOX's scroll bars, as the desktop's
//! (rapidr-runtime-core's scroll.rs): the shared model
//! (rapidr_value::scrollbars) works out the ranges and the bars; scrolling
//! moves the components (their Left / Top). The UI kernel draws the bars
//! and takes the mouse on them.

use std::cell::Cell;

use rapidr_value::layout::Align;
use rapidr_value::scrollbars::{self, Child, Shift};

use crate::object_web::{get_children_of, rp_comp_get_stored, rp_comp_set, rp_comp_type};
use crate::value::{v_int, Value};

thread_local! {
    static UPDATING: Cell<bool> = const { Cell::new(false) };
}

/// (not a QFORMMDI: its children's MDI client area is Windows' own)
pub fn scrolls(name: &str) -> bool {
    !name.is_empty() && scrollbars::scrolls(&rp_comp_type(name)) && !rapidr_value::mdi::is_mdi(name)
}

/// A QSCROLLBOX's edge (bsSingle, its default: 2 pixels).
pub fn border(name: &str) -> i64 {
    if rp_comp_type(name) != "RSCROLLBOX" {
        return 0;
    }
    match rp_comp_get_stored(name, "borderstyle") {
        Value::Null => 2,
        v if v.to_i64() == 0 => 0,
        _ => 2,
    }
}

/// The area the bars and the components share.
pub fn area(name: &str) -> (i64, i64) {
    if rp_comp_type(name) == "RFORM" {
        return crate::layout_web::form_area(name);
    }
    let b = border(name);
    let n = |p: &str| rp_comp_get_stored(name, p).to_i64();
    ((n("width") - 2 * b).max(0), (n("height") - 2 * b).max(0))
}

/// ClientWidth / ClientHeight: the area less the bars shown.
pub fn client(name: &str) -> (i64, i64) {
    let (w, h) = area(name);
    scrollbars::with(name, |s| s.client(w, h)).unwrap_or((w, h))
}

fn children(name: &str) -> Vec<(String, Child)> {
    get_children_of(name)
        .into_iter()
        .filter(|(c, t)| !matches!(t.as_str(), "RMAINMENU" | "RPOPUPMENU" | "RMENUITEM") && !matches!(rp_comp_get_stored(c, "width"), Value::Null))
        .map(|(c, _)| {
            let n = |p: &str| rp_comp_get_stored(&c, p).to_i64();
            let visible = match rp_comp_get_stored(&c, "visible") {
                Value::Null => true,
                v => v.to_bool(),
            };
            let child = Child { left: n("left"), top: n("top"), width: n("width"), height: n("height"), align: Align::from_value(n("align")), visible };
            (c, child)
        })
        .collect()
}

/// The bars worked out again for `name` (as the desktop's `scroll::update`).
pub fn update(name: &str) {
    if !scrolls(name) || UPDATING.with(Cell::get) {
        return;
    }
    UPDATING.with(|u| u.set(true));
    for _ in 0..2 {
        let (w, h) = area(name);
        let kids: Vec<Child> = children(name).into_iter().map(|(_, c)| c).collect();
        let before = scrollbars::with(name, |s| (s.horz.shown, s.vert.shown));
        let shift = scrollbars::with_mut(name, |s| s.update(w, h, &kids));
        move_children(name, shift);
        let after = scrollbars::with(name, |s| (s.horz.shown, s.vert.shown));
        if before == after {
            break;
        }
        crate::layout_web::realign(name, None);
    }
    UPDATING.with(|u| u.set(false));
}

/// The components moved by a scroll: Left by -dx, Top by -dy.
fn move_children(name: &str, (dx, dy): Shift) {
    if (dx, dy) == (0, 0) {
        return;
    }
    let kids = children(name);
    crate::layout_web::quietly(|| {
        for (c, k) in &kids {
            if dx != 0 {
                rp_comp_set(c, "left", v_int(k.left - dx));
            }
            if dy != 0 {
                rp_comp_set(c, "top", v_int(k.top - dy));
            }
        }
    });
}

pub fn user_scrolled(name: &str, shift: Shift) {
    move_children(name, shift);
}

pub fn get(name: &str, prop: &str) -> Option<Value> {
    if !scrolls(name) {
        return None;
    }
    scrollbars::with_mut(name, |s| s.get(prop))
}

pub fn set(name: &str, prop: &str, val: &Value) -> bool {
    if !scrolls(name) || scrollbars::with_mut(name, |s| s.set(prop, val)).is_none() {
        return false;
    }
    update(name);
    true
}
