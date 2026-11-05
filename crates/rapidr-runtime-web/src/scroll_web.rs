//! A QFORM's and a QSCROLLBOX's scroll bars, as the desktop's: the shared
//! model and its engine (rapidr_value::scrollbars, `engine` — the one the
//! desktop's `scroll.rs` runs) on this runtime's registry ([`Rt`]), the
//! bars worked out from the stored properties. The UI kernel draws the bars
//! and takes the mouse on them.

use rapidr_value::scrollbars::engine::{self, ScrollStore};
use rapidr_value::scrollbars::Shift;

use crate::object_web::{get_children_of, rp_comp_get_stored, rp_comp_set, rp_comp_type};
use crate::value::Value;

/// This runtime's component registry as the bars see it.
struct Rt;

impl ScrollStore for Rt {
    fn type_of(&self, name: &str) -> String {
        rp_comp_type(name)
    }
    fn get(&self, name: &str, prop: &str) -> Value {
        rp_comp_get_stored(name, prop)
    }
    fn set(&self, name: &str, prop: &str, value: Value) {
        rp_comp_set(name, prop, value);
    }
    fn children(&self, name: &str) -> Vec<(String, String)> {
        get_children_of(name)
    }
    fn form_area(&self, form: &str) -> (i64, i64) {
        crate::layout_web::form_area(form)
    }
    fn has_aligned(&self, name: &str) -> bool {
        crate::layout_web::has_aligned_children(name)
    }
    fn realign(&self, name: &str) {
        crate::layout_web::realign(name, None);
    }
    fn quietly(&self, f: &mut dyn FnMut()) {
        crate::layout_web::quietly(f);
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

pub fn get(name: &str, prop: &str) -> Option<Value> {
    engine::get(&Rt, name, prop)
}

pub fn set(name: &str, prop: &str, val: &Value) -> bool {
    engine::set(&Rt, name, prop, val)
}
