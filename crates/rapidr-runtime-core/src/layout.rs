//! `Align` and live geometry in the desktop runtime.
//!
//! Layout works on the component registry (Left / Top / Width / Height),
//! with `rapidr_value::layout` placing the aligned children, so a program
//! reads the aligned size right away (`Grid.Align = alClient : PRINT
//! Grid.Width`), before or after the form is shown. When the widgets exist,
//! `ui::gui_apply_geometry` moves them.
//!
//! A container is laid out again when an aligned child's Align, position,
//! size or visibility changes, when the container itself is resized (by the
//! program, or by the user resizing a form), and when a form gets its main
//! menu (which takes the top of its client area).
//!
//! Anchors (a RapidR extension, from Delphi): a child whose Anchors aren't
//! the default records where it is ([`rapidr_value::layout::AnchorRules`])
//! when its Anchors are set and whenever the program places it, and follows
//! its parent's client area each time that changes — the same moments that
//! lay out the aligned children ([`client_changed`]).

use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use rapidr_value::layout::{align_controls, anchor_controls, anchor_record, anchor_rules, splitter_drag, Align, Constraints, Control, Rect, SplitterDrag, DEFAULT_ANCHORS};

use crate::object::{get_children_of, rp_comp_get, rp_comp_set, rp_comp_type};
use crate::value::{v_int, Value};

thread_local! {
    /// Set while layout stores the positions it computed (they must not
    /// trigger another layout), or while widgets are built.
    static BUSY: Cell<u32> = const { Cell::new(0) };
    /// Containers with at least one aligned child.
    static ALIGNED_PARENTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    /// Containers with at least one anchored child (Anchors not the default).
    static ANCHORED_PARENTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
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

pub(crate) fn align_of(name: &str) -> Align {
    Align::from_value(rp_comp_get(name, "align").to_i64())
}

pub(crate) fn parent_of(name: &str) -> String {
    rp_comp_get(name, "parent").to_string_val().to_lowercase()
}

pub(crate) fn visible(name: &str) -> bool {
    match rp_comp_get(name, "visible") {
        Value::Null => true,
        v => v.to_bool(),
    }
}

fn rect_of(name: &str) -> Rect {
    let n = |p: &str| rp_comp_get(name, p).to_i64();
    Rect::new(n("left"), n("top"), n("width"), n("height"))
}

/// A component's Anchors (akLeft + akTop until set).
fn anchors_of(name: &str) -> i64 {
    match crate::object::stored(name, "anchors") {
        None | Some(Value::Null) => DEFAULT_ANCHORS,
        Some(v) => v.to_i64(),
    }
}

/// A component's MinWidth … MaxHeight.
pub(crate) fn constraints_of(name: &str) -> Constraints {
    Constraints::of(|p| crate::object::stored_int(name, p))
}

/// The client area `parent`'s anchored children follow: a form's or
/// scroll box's inside (its scroll bars not excluded, so bars coming and
/// going move nothing), a tab control's page, any other container's whole
/// size.
fn anchor_parent_size(parent: &str) -> (i64, i64) {
    if rp_comp_type(parent) == "RFORM" || crate::scroll::scrolls(parent) {
        return crate::scroll::area(parent);
    }
    let r = client_rect(parent);
    (r.width, r.height)
}

/// Records where `name` is for its Anchors (they changed, or the program
/// placed it): from now on it follows its parent.
fn anchor_here(name: &str) {
    let anchors = anchors_of(name);
    if anchors == DEFAULT_ANCHORS && anchor_rules(name).is_none() {
        return;
    }
    let parent = parent_of(name);
    let size = if parent.is_empty() { (0, 0) } else { anchor_parent_size(&parent) };
    anchor_record(name, anchors, rect_of(name), size);
    if anchors != DEFAULT_ANCHORS && !parent.is_empty() {
        ANCHORED_PARENTS.with(|a| a.borrow_mut().insert(parent));
    }
}

/// `parent`'s client area changed size: its aligned children are laid out
/// again and its anchored ones follow.
pub fn client_changed(parent: &str) {
    realign(parent, None);
    reanchor(parent);
}

/// Moves `parent`'s anchored children to follow its client area
/// (`rapidr_value::layout::anchor_controls`).
pub fn reanchor(parent: &str) {
    let parent = parent.to_lowercase();
    if parent.is_empty() || !ANCHORED_PARENTS.with(|a| a.borrow().contains(&parent)) {
        return;
    }
    let children = get_children_of(&parent);
    let list: Vec<_> = children
        .iter()
        .map(|(n, _)| {
            let rules = anchor_rules(n).filter(|r| r.anchors() == anchors_of(n));
            (rules, rect_of(n), align_of(n), constraints_of(n))
        })
        .collect();
    let moves = anchor_controls(anchor_parent_size(&parent), &list);
    if moves.is_empty() {
        return;
    }
    quietly(|| {
        for (i, r) in &moves {
            let name = &children[*i].0;
            rp_comp_set(name, "left", v_int(r.left));
            rp_comp_set(name, "top", v_int(r.top));
            rp_comp_set(name, "width", v_int(r.width));
            rp_comp_set(name, "height", v_int(r.height));
        }
    });
    for (i, r) in moves {
        let name = &children[i].0;
        #[cfg(feature = "gui")]
        crate::ui::gui_apply_geometry(name);
        if (r.width, r.height) != (list[i].1.width, list[i].1.height) {
            client_changed(name);
            crate::scroll::update(name);
        }
    }
    crate::scroll::update(&parent);
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
    // A form's / scroll box's: Delphi's (AdjustClientRect) — the scrolled
    // area, as large as the ranges.
    if crate::scroll::scrolls(parent) {
        let (cw, ch) = crate::scroll::client(parent);
        let (hp, vp, hr, vr) = rapidr_value::scrollbars::with(parent, |s| (s.horz.position, s.vert.position, s.horz.range, s.vert.range)).unwrap_or_default();
        return Rect::new(-hp, -vp, cw.max(hr), ch.max(vr));
    }
    let (w, h) = (rp_comp_get(parent, "width").to_i64(), rp_comp_get(parent, "height").to_i64());
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
        "anchors" => anchor_here(name),
        "left" | "top" | "width" | "height" | "visible" => {
            #[cfg(feature = "gui")]
            if prop != "visible" {
                crate::ui::gui_apply_geometry(name);
            }
            if prop != "visible" {
                anchor_here(name);
            }
            if align_of(name) != Align::None {
                realign(&parent_of(name), Some(name));
            }
            if matches!(prop, "width" | "height") {
                client_changed(name);
                crate::scroll::update(name);
            }
        }
        "parent" => {
            let parent = parent_of(name);
            anchor_here(name);
            if align_of(name) != Align::None {
                mark(&parent);
                realign(&parent, Some(name));
            } else if rp_comp_type(name) == "RMAINMENU" {
                client_changed(&parent);
            }
        }
        // (a QSCROLLBOX's edge: its inside changed)
        "borderstyle" => {
            crate::scroll::update(name);
            reanchor(name);
        }
        _ => {}
    }
    // A scrolling parent's bars follow its components (scroll.rs).
    if matches!(prop, "align" | "left" | "top" | "width" | "height" | "visible" | "parent") {
        crate::scroll::update(&parent_of(name));
    }
}

fn controls_of(parent: &str) -> (Vec<(String, String)>, Vec<Control>) {
    let children = get_children_of(parent);
    let controls = children.iter().map(|(n, _)| Control { align: align_of(n), visible: visible(n), rect: rect_of(n), constraints: constraints_of(n) }).collect();
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
        crate::ui::gui_apply_geometry(&name);
        if resized {
            client_changed(&name);
        }
    }
}

/// (a QLABEL's AutoSize, `rapidr_value::autosize`) Before `rp_comp_set`
/// stores `prop` of `name`: what can resize labels — a label's Caption,
/// AutoSize, WordWrap; any component's font or Parent (the fonts of the
/// labels it is or holds) — for [`labels_after_set`].
pub(crate) fn labels_before_set(name: &str, prop: &str) -> Option<rapidr_value::autosize::Before> {
    let t = rp_comp_type(name);
    if t.is_empty() || t == "RUDT" || rapidr_value::objects::is_object_type(&t) {
        return None;
    }
    rapidr_value::autosize::before_set(name, &t, &prop.to_ascii_lowercase(), &stored_or_null, &get_children_of, &label_font)
}

/// The property is stored: each AutoSize label it changed takes its text's
/// size (Left too, right-aligned).
pub(crate) fn labels_after_set(name: &str, before: rapidr_value::autosize::Before) {
    for label in rapidr_value::autosize::changed_labels(name, before, &stored_or_null, &label_font) {
        if let Some(r) = rapidr_value::autosize::label_bounds(&label, &|i, p| rp_comp_get(i, p)) {
            for (p, v) in [("left", r.left), ("width", r.width), ("height", r.height)] {
                if rp_comp_get(&label, p).to_i64() != v {
                    rp_comp_set(&label, p, v_int(v));
                }
            }
        }
    }
}

fn stored_or_null(name: &str, prop: &str) -> Value {
    crate::object::stored(name, prop).unwrap_or(Value::Null)
}

/// The font label `name` is drawn in, its Font.Color the one it reads.
fn label_font(name: &str) -> rapidr_value::autosize::FontKey {
    (rapidr_value::objects::font_from_props(name, &|i, p| rp_comp_get(i, p)), crate::object::program_font_color(name).to_i64())
}
