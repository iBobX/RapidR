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

use rapidr_value::layout::{align_controls, splitter_drag, Align, Control, Rect, SplitterDrag};

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

/// Whether geometry is being stored internally (see [`quietly`]) rather than
/// changed by the program or the user.
pub fn is_quiet() -> bool {
    busy()
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

/// The client area of `parent`, in its children's coordinates: a form's
/// inside (frame and main menu excluded), any other container's whole size.
fn client_rect(parent: &str) -> Rect {
    // A tab control's: the area under its tabs (rapidr_value::objects::tabcontrol).
    if rapidr_value::objects::is_tabcontrol(parent) {
        let (w, h) = (rp_comp_get(parent, "width").to_i64(), rp_comp_get(parent, "height").to_i64());
        let font = rapidr_value::objects::font_from_props(parent, &|id, p| rp_comp_get(id, p));
        if let Some((x, y, w, h)) = rapidr_value::objects::with_tabcontrol(parent, |t| t.display(w, h, &font)) {
            return Rect::new(x, y, w, h);
        }
    }
    let (w, h) = if rp_comp_type(parent) == "RFORM" {
        crate::object::form_client(parent)
    } else {
        (rp_comp_get(parent, "width").to_i64(), rp_comp_get(parent, "height").to_i64())
    };
    Rect::new(0, 0, w, h)
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

fn controls_of(parent: &str) -> (Vec<(String, String)>, Vec<Control>) {
    let children = get_children_of(parent);
    let controls = children.iter().map(|(n, _)| Control { align: align_of(n), visible: visible(n), rect: rect_of(n) }).collect();
    (children, controls)
}

thread_local! {
    /// The QSPLITTER being dragged: its name, the control it resizes and how.
    static DRAG: RefCell<Option<(String, String, SplitterDrag)>> = const { RefCell::new(None) };
}

/// The user pressed the mouse on `splitter`: returns whether there's a
/// control next to it to resize (`rapidr_value::layout::splitter_drag`).
pub fn splitter_begin(splitter: &str) -> bool {
    let splitter = splitter.to_lowercase();
    let parent = parent_of(&splitter);
    let (children, controls) = controls_of(&parent);
    let Some(i) = children.iter().position(|(n, _)| *n == splitter) else { return false };
    let min = match rp_comp_get(&splitter, "minsize") {
        Value::Null => 30,
        v => v.to_i64().max(0),
    };
    let drag = splitter_drag(client_rect(&parent), &controls, i, min);
    let found = drag.is_some();
    DRAG.with(|d| *d.borrow_mut() = drag.map(|g| (splitter, children[g.control].0.clone(), g)));
    found
}

/// The mouse moved `delta` pixels (along the splitter's axis) since the
/// drag began: the neighbour's Width / Height follow, and the parent is
/// laid out again.
pub fn splitter_move(delta: i64) {
    let Some((_, target, drag)) = DRAG.with(|d| d.borrow().clone()) else { return };
    let size = drag.size_for(delta);
    let prop = if drag.horizontal { "width" } else { "height" };
    if rp_comp_get(&target, prop).to_i64() != size {
        rp_comp_set(&target, prop, v_int(size));
    }
}

/// The drag ended: the splitter's OnMoved fires.
pub fn splitter_end() {
    if let Some((splitter, _, _)) = DRAG.with(|d| d.borrow_mut().take()) {
        crate::object::rp_fire_event(&splitter, "onmoved");
    }
}

/// Lays out the aligned children of `parent` (see `rapidr_value::layout`);
/// `changed` is the child whose Align, size or visibility just changed.
pub fn realign(parent: &str, changed: Option<&str>) {
    let parent = parent.to_lowercase();
    if parent.is_empty() || !has_aligned_children(&parent) {
        return;
    }
    let (children, controls) = controls_of(&parent);
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
