//! A QFORM's and a QSCROLLBOX's scroll bars in this runtime: the shared
//! model (rapidr_value::scrollbars) works out the ranges and the bars from
//! the components; scrolling moves the components (their Left / Top), as
//! Delphi's ScrollBy; gui.rs draws the bars and passes them the mouse.

use std::cell::Cell;

use rapidr_value::layout::Align;
use rapidr_value::scrollbars::{self, Child, Shift};

use crate::object::{get_children_of, rp_comp_get, rp_comp_set, rp_comp_type};
use crate::value::v_int;

thread_local! {
    /// Set while the bars are worked out (moving components mustn't start
    /// it again).
    static UPDATING: Cell<bool> = const { Cell::new(false) };
}

/// Whether `name` scrolls (a QFORM, a QSCROLLBOX — not a QFORMMDI: its
/// children's MDI client area is Windows' own).
pub fn scrolls(name: &str) -> bool {
    !name.is_empty() && scrollbars::scrolls(&rp_comp_type(name)) && !rapidr_value::mdi::is_mdi(name)
}

/// A QSCROLLBOX's edge (BorderStyle bsSingle, its default: 2 pixels).
pub fn border(name: &str) -> i64 {
    if rp_comp_type(name) != "RSCROLLBOX" {
        return 0;
    }
    match rp_comp_get(name, "borderstyle") {
        crate::value::Value::Null => 2,
        v if v.to_i64() == 0 => 0,
        _ => 2,
    }
}

/// The area the bars and the components share: a form's inside, a scroll
/// box's inside its edge.
pub fn area(name: &str) -> (i64, i64) {
    if rp_comp_type(name) == "RFORM" {
        return crate::object::form_area(name);
    }
    let b = border(name);
    ((rp_comp_get(name, "width").to_i64() - 2 * b).max(0), (rp_comp_get(name, "height").to_i64() - 2 * b).max(0))
}

/// The client area: the area less the bars shown (ClientWidth / Height).
pub fn client(name: &str) -> (i64, i64) {
    let (w, h) = area(name);
    scrollbars::with(name, |s| s.client(w, h)).unwrap_or((w, h))
}

/// The components that take room (menus, timers and the like don't).
fn children(name: &str) -> Vec<(String, Child)> {
    get_children_of(name)
        .into_iter()
        .filter(|(c, t)| !matches!(t.as_str(), "RMAINMENU" | "RPOPUPMENU" | "RMENUITEM") && !matches!(rp_comp_get(c, "width"), crate::value::Value::Null))
        .map(|(c, _)| {
            let n = |p: &str| rp_comp_get(&c, p).to_i64();
            let child = Child {
                left: n("left"),
                top: n("top"),
                width: n("width"),
                height: n("height"),
                align: Align::from_value(n("align")),
                visible: crate::layout::visible(&c),
            };
            (c, child)
        })
        .collect()
}

/// The bars worked out again for `name` (its components or size changed,
/// the program set a Position or AutoScroll …): the components moved if a
/// position had to change, laid out again if the client area did.
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
        // (a bar came or went: the client area changed)
        crate::layout::realign(name, None);
    }
    UPDATING.with(|u| u.set(false));
    #[cfg(feature = "desktop-ui")]
    crate::ui::redraw_widget(name);
}

/// The components moved by a scroll: Left by -dx, Top by -dy.
pub fn move_children(name: &str, (dx, dy): Shift) {
    if (dx, dy) == (0, 0) {
        return;
    }
    let kids = children(name);
    crate::layout::quietly(|| {
        for (c, k) in &kids {
            if dx != 0 {
                rp_comp_set(c, "left", v_int(k.left - dx));
            }
            if dy != 0 {
                rp_comp_set(c, "top", v_int(k.top - dy));
            }
        }
    });
    #[cfg(feature = "desktop-ui")]
    for (c, _) in &kids {
        crate::ui::gui_apply_geometry(c);
    }
}

/// The user scrolled (`shift` from the model): the components move, the
/// bars are drawn again.
pub fn user_scrolled(name: &str, shift: Shift) {
    move_children(name, shift);
    #[cfg(feature = "desktop-ui")]
    crate::ui::redraw_widget(name);
}

/// The program reading AutoScroll, HorzPosition, … of a QFORM / QSCROLLBOX.
pub fn get(name: &str, prop: &str) -> Option<crate::value::Value> {
    if !scrolls(name) {
        return None;
    }
    scrollbars::with_mut(name, |s| s.get(prop))
}

/// The program setting one: the bars and the components follow.
pub fn set(name: &str, prop: &str, val: &crate::value::Value) -> bool {
    if !scrolls(name) || scrollbars::with_mut(name, |s| s.set(prop, val)).is_none() {
        return false;
    }
    update(name);
    true
}
