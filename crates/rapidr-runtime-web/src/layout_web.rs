//! `Align` in the web runtime: the desktop runtime's `layout.rs` on this
//! runtime's registry, with the same placement (`rapidr_value::layout`).
//! Laying out stores the children's Left / Top / Width / Height, which
//! moves their elements.
//!
//! A container is laid out again when an aligned child's Align, position,
//! size or visibility changes, when the container is resized (by the
//! program, or by maximizing / restoring a form), and when a form gets its
//! main menu.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use rapidr_value::layout::{align_controls, Align, Control, Rect};

use crate::object_web::{get_children_of, rp_comp_get_stored, rp_comp_set, rp_comp_type};
use crate::value::{v_int, Value};

/// A web form's title bar, and its main menu, above the client area
/// (gui_web's `create_form` / `create_mainmenu`).
pub const TITLE_HEIGHT: i64 = 29;
pub const MENU_HEIGHT: i64 = 28;

thread_local! {
    static BUSY: Cell<u32> = const { Cell::new(0) };
    static ALIGNED_PARENTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Runs `f` without its geometry stores laying anything out.
pub fn quietly<R>(f: impl FnOnce() -> R) -> R {
    BUSY.with(|b| b.set(b.get() + 1));
    let r = f();
    BUSY.with(|b| b.set(b.get() - 1));
    r
}

fn align_of(name: &str) -> Align {
    Align::from_value(rp_comp_get_stored(name, "align").to_i64())
}

fn parent_of(name: &str) -> String {
    rp_comp_get_stored(name, "parent").to_string_val().to_uppercase()
}

fn visible(name: &str) -> bool {
    match rp_comp_get_stored(name, "visible") {
        Value::Null => true,
        v => v.to_bool(),
    }
}

fn rect_of(name: &str) -> Rect {
    let n = |p: &str| rp_comp_get_stored(name, p).to_i64();
    Rect::new(n("left"), n("top"), n("width"), n("height"))
}

fn has_aligned_children(name: &str) -> bool {
    ALIGNED_PARENTS.with(|a| a.borrow().contains(&name.to_uppercase()))
}

fn mark(parent: &str) {
    if !parent.is_empty() {
        ALIGNED_PARENTS.with(|a| a.borrow_mut().insert(parent.to_uppercase()));
    }
}

fn has_menu(form: &str) -> bool {
    get_children_of(form).iter().any(|(_, t)| t == "RMAINMENU")
}

/// The client area of `parent` in its children's coordinates: a form's
/// size less its title bar and menu; any other container's whole size.
pub fn client_rect(parent: &str) -> Rect {
    let n = |p: &str| rp_comp_get_stored(parent, p).to_i64();
    let top = if rp_comp_type(parent) == "RFORM" { TITLE_HEIGHT + if has_menu(parent) { MENU_HEIGHT } else { 0 } } else { 0 };
    Rect::new(0, 0, n("width"), (n("height") - top).max(0))
}

/// Called by `rp_comp_set` after it stored `prop` (lowercase) of `name`.
pub fn after_set(name: &str, prop: &str) {
    if BUSY.with(Cell::get) > 0 {
        return;
    }
    match prop {
        "align" => {
            let parent = parent_of(name);
            if align_of(name) != Align::None {
                mark(&parent);
            }
            realign(&parent, Some(name));
        }
        "left" | "top" | "width" | "height" | "visible" => {
            if align_of(name) != Align::None {
                realign(&parent_of(name), Some(name));
            }
            if matches!(prop, "width" | "height") {
                realign(name, None);
            }
        }
        "parent" => {
            let parent = parent_of(name);
            if align_of(name) != Align::None {
                mark(&parent);
                realign(&parent, Some(name));
            } else if rp_comp_type(name) == "RMAINMENU" {
                realign(&parent, None);
            }
        }
        _ => {}
    }
}

/// Lays out the aligned children of `parent`; `changed` is the child whose
/// Align, size or visibility just changed.
pub fn realign(parent: &str, changed: Option<&str>) {
    let parent = parent.to_uppercase();
    if parent.is_empty() || !has_aligned_children(&parent) {
        return;
    }
    let children = get_children_of(&parent);
    let controls: Vec<Control> =
        children.iter().map(|(n, _)| Control { align: align_of(n), visible: visible(n), rect: rect_of(n) }).collect();
    let changed = changed.map(str::to_uppercase).and_then(|c| children.iter().position(|(n, _)| *n == c));
    let moves: Vec<(String, Rect, bool)> = align_controls(client_rect(&parent), &controls, changed)
        .into_iter()
        .filter(|(i, r)| *r != controls[*i].rect)
        .map(|(i, r)| {
            let old = controls[i].rect;
            (children[i].0.clone(), r, (old.width, old.height) != (r.width, r.height))
        })
        .collect();
    quietly(|| {
        for (name, r, _) in &moves {
            rp_comp_set(name, "left", v_int(r.left));
            rp_comp_set(name, "top", v_int(r.top));
            rp_comp_set(name, "width", v_int(r.width));
            rp_comp_set(name, "height", v_int(r.height));
        }
    });
    for (name, _, resized) in moves {
        if resized {
            realign(&name, None);
        }
    }
}
