//! A QFORM's and a QSCROLLBOX's scroll bars in this runtime: the shared
//! model and its engine (rapidr_value::scrollbars, `engine`: the bars from
//! the components, worked out at once or once before anything reads them —
//! the web runtime runs the same) on this runtime's registry ([`Rt`]).
//! Scrolling moves the components (their Left / Top), as RapidQ's
//! ScrollBy; the UI kernel draws the bars and passes them the mouse.

use rapidr_value::scrollbars::engine::{self, ScrollStore};
use rapidr_value::scrollbars::Shift;

use crate::object::{get_children_of, rp_comp_get, rp_comp_set, rp_comp_type};
use crate::value::Value;

/// This runtime's component registry as the bars see it.
struct Rt;

impl ScrollStore for Rt {
    fn type_of(&self, name: &str) -> String {
        rp_comp_type(name)
    }
    fn get(&self, name: &str, prop: &str) -> Value {
        rp_comp_get(name, prop)
    }
    fn set(&self, name: &str, prop: &str, value: Value) {
        rp_comp_set(name, prop, value);
    }
    fn children(&self, name: &str) -> Vec<(String, String)> {
        get_children_of(name)
    }
    fn form_area(&self, form: &str) -> (i64, i64) {
        crate::object::form_area(form)
    }
    fn has_aligned(&self, name: &str) -> bool {
        crate::layout::has_aligned_children(name)
    }
    fn realign(&self, name: &str) {
        crate::layout::realign(name, None);
    }
    fn quietly(&self, f: &mut dyn FnMut()) {
        crate::layout::quietly(f);
    }
    fn moved(&self, _name: &str) {
        #[cfg(feature = "gui")]
        crate::ui::gui_apply_geometry(_name);
    }
    fn redraw(&self, _name: &str) {
        #[cfg(feature = "gui")]
        crate::ui::redraw_widget(_name);
    }
}

/// `name`'s components moved or resized, or it did (`engine::changed`).
pub fn changed(name: &str) {
    engine::changed(&Rt, name);
}

/// The bars put off worked out now: before the program reads anything,
/// before a container's first aligned component, before the windows sync.
pub fn flush() {
    engine::flush(&Rt);
}

pub fn scrolls(name: &str) -> bool {
    engine::scrolls(&Rt, name)
}

pub fn area(name: &str) -> (i64, i64) {
    engine::area(&Rt, name)
}

/// ClientWidth / ClientHeight: the area less the bars shown.
pub fn client(name: &str) -> (i64, i64) {
    engine::client(&Rt, name)
}

/// The bars worked out again for `name` now.
pub fn update(name: &str) {
    engine::update(&Rt, name);
}

pub fn user_scrolled(name: &str, shift: Shift) {
    engine::user_scrolled(&Rt, name, shift);
}

/// The program reading AutoScroll, HorzPosition, … of a QFORM / QSCROLLBOX.
pub fn get(name: &str, prop: &str) -> Option<Value> {
    engine::get(&Rt, name, prop)
}

/// The program setting one: the bars and the components follow.
pub fn set(name: &str, prop: &str, val: &Value) -> bool {
    engine::set(&Rt, name, prop, val)
}
