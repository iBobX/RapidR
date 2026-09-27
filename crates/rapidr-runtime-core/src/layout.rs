//! `Align` and live geometry in the desktop runtime.
//!
//! Layout works on the component registry (Left / Top / Width / Height),
//! with `rapidr_value::layout` placing the aligned children, so a program
//! reads the aligned size right away (`Grid.Align = alClient : PRINT
//! Grid.Width`), before or after the form is shown. When the widgets exist,
//! `gui::gui_apply_geometry` moves them.
//!
//! A container is laid out again when an aligned child's Align, position,
//! size or visibility changes, when the container itself is resized (by the
//! program, or by the user resizing a form), and when a form gets its main
//! menu (which takes the top of its client area).

use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use rapidr_value::layout::{align_controls, Align, Control, Rect};

use crate::object::{get_children_of, rp_comp_get, rp_comp_set, rp_comp_type};
use crate::value::{v_int, Value};

thread_local! {
    /// Set while layout stores the positions it computed (they must not
    /// trigger another layout), or while widgets are built.
    static BUSY: Cell<u32> = const { Cell::new(0) };
    /// Containers with at least one aligned child.
    static ALIGNED_PARENTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Runs `f` without geometry stores moving widgets or laying anything out
/// (widget construction sets temporary absolute positions).
pub fn quietly<R>(f: impl FnOnce() -> R) -> R {
    BUSY.with(|b| b.set(b.get() + 1));
    let r = f();
    BUSY.with(|b| b.set(b.get() - 1));
    r
}

fn busy() -> bool {
    BUSY.with(Cell::get) > 0
}

fn align_of(name: &str) -> Align {
    Align::from_value(rp_comp_get(name, "align").to_i64())
}

fn parent_of(name: &str) -> String {
    rp_comp_get(name, "parent").to_string_val().to_lowercase()
}

fn visible(name: &str) -> bool {
    match rp_comp_get(name, "visible") {
        Value::Null => true,
        v => v.to_bool(),
    }
}

fn rect_of(name: &str) -> Rect {
    let n = |p: &str| rp_comp_get(name, p).to_i64();
    Rect::new(n("left"), n("top"), n("width"), n("height"))
}

fn has_aligned_children(name: &str) -> bool {
    ALIGNED_PARENTS.with(|a| a.borrow().contains(&name.to_lowercase()))
}

fn mark(parent: &str) {
    if !parent.is_empty() {
        ALIGNED_PARENTS.with(|a| a.borrow_mut().insert(parent.to_lowercase()));
    }
}

/// The client area of `parent`, in its children's coordinates: the whole
/// component, less a form's in-window main menu.
fn client_rect(parent: &str) -> Rect {
    let n = |p: &str| rp_comp_get(parent, p).to_i64();
    #[cfg(feature = "gui")]
    let menu = crate::gui::menu_offset(parent) as i64;
    #[cfg(not(feature = "gui"))]
    let menu = 0;
    Rect::new(0, 0, n("width"), (n("height") - menu).max(0))
}

/// Called by `rp_comp_set` after it stored `prop` (lowercase) of `name`.
pub(crate) fn after_set(name: &str, prop: &str) {
    if busy() {
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
            #[cfg(feature = "gui")]
            if prop != "visible" {
                crate::gui::gui_apply_geometry(name);
            }
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

/// Lays out the aligned children of `parent` (see `rapidr_value::layout`);
/// `changed` is the child whose Align, size or visibility just changed.
pub fn realign(parent: &str, changed: Option<&str>) {
    let parent = parent.to_lowercase();
    if parent.is_empty() || !has_aligned_children(&parent) {
        return;
    }
    let children = get_children_of(&parent);
    let controls: Vec<Control> =
        children.iter().map(|(n, _)| Control { align: align_of(n), visible: visible(n), rect: rect_of(n) }).collect();
    let changed = changed.map(str::to_lowercase).and_then(|c| children.iter().position(|(n, _)| *n == c));
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
        #[cfg(feature = "gui")]
        crate::gui::gui_apply_geometry(&name);
        if resized {
            realign(&name, None);
        }
    }
}
