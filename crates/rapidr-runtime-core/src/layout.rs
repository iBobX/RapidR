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
//!
//! When all this happens is `rapidr_value::layout::engine`'s, the one
//! sequence the web runtime and the IDE's designer run too; this file is
//! this runtime's registry as its store ([`Rt`]).

use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use rapidr_value::layout::{anchor_rules, anchor_set, engine, splitter_drag, Align, AnchorRules, Constraints, Control, Rect, SplitterDrag, DEFAULT_ANCHORS};

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

/// `parent`'s client area changed size: its aligned children are laid out
/// again and its anchored ones follow.
pub fn client_changed(parent: &str) {
    engine::client_changed(&mut Rt, &parent.to_lowercase());
}

/// Moves `parent`'s anchored children to follow its client area
/// (`rapidr_value::layout::anchor_controls`).
pub fn reanchor(parent: &str) {
    engine::reanchor(&mut Rt, &parent.to_lowercase());
}

fn has_aligned_children(name: &str) -> bool {
    ALIGNED_PARENTS.with(|a| a.borrow().contains(&name.to_lowercase()))
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

/// This runtime's component registry as the layout engine's store.
struct Rt;

impl engine::LayoutStore for Rt {
    fn rect(&self, name: &str) -> Rect {
        rect_of(name)
    }
    fn align(&self, name: &str) -> Align {
        align_of(name)
    }
    fn visible(&self, name: &str) -> bool {
        visible(name)
    }
    fn anchors(&self, name: &str) -> i64 {
        anchors_of(name)
    }
    fn constraints(&self, name: &str) -> Constraints {
        constraints_of(name)
    }
    fn type_of(&self, name: &str) -> String {
        rp_comp_type(name)
    }
    fn parent_of(&self, name: &str) -> String {
        parent_of(name)
    }
    fn key(&self, name: &str) -> String {
        name.to_lowercase()
    }
    fn children_of(&self, parent: &str) -> Vec<String> {
        get_children_of(parent).into_iter().map(|(n, _)| n).collect()
    }
    fn client_rect(&self, parent: &str) -> Rect {
        client_rect(parent)
    }
    fn anchor_area(&self, parent: &str) -> (i64, i64) {
        anchor_parent_size(parent)
    }
    fn store_rect(&mut self, name: &str, r: Rect) {
        quietly(|| {
            rp_comp_set(name, "left", v_int(r.left));
            rp_comp_set(name, "top", v_int(r.top));
            rp_comp_set(name, "width", v_int(r.width));
            rp_comp_set(name, "height", v_int(r.height));
        });
    }
    fn rules(&self, name: &str) -> Option<AnchorRules> {
        anchor_rules(name)
    }
    fn set_rules(&mut self, name: &str, rules: Option<AnchorRules>) {
        anchor_set(name, rules);
    }
    fn has_aligned(&self, parent: &str) -> bool {
        has_aligned_children(parent)
    }
    fn mark_aligned(&mut self, parent: &str) {
        ALIGNED_PARENTS.with(|a| a.borrow_mut().insert(parent.to_lowercase()));
    }
    fn has_anchored(&self, parent: &str) -> bool {
        ANCHORED_PARENTS.with(|a| a.borrow().contains(&parent.to_lowercase()))
    }
    fn mark_anchored(&mut self, parent: &str) {
        ANCHORED_PARENTS.with(|a| a.borrow_mut().insert(parent.to_lowercase()));
    }
    fn moved(&mut self, _name: &str) {
        #[cfg(feature = "gui")]
        crate::ui::gui_apply_geometry(_name);
    }
    fn scroll_update(&mut self, name: &str) {
        crate::scroll::update(name);
    }
}

/// Called by `rp_comp_set` after it stored `prop` (lowercase) of `name`.
pub(crate) fn after_set(name: &str, prop: &str) {
    if busy() {
        return;
    }
    match prop {
        // (a QSCROLLBOX's edge: its inside changed)
        "borderstyle" => {
            crate::scroll::update(name);
            reanchor(name);
        }
        _ => engine::after_set(&mut Rt, &name.to_lowercase(), prop),
    }
}

fn controls_of(parent: &str) -> (Vec<String>, Vec<Control>) {
    engine::controls_of(&Rt, &parent.to_lowercase())
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
    let Some(i) = children.iter().position(|n| *n == splitter) else { return false };
    let min = match rp_comp_get(&splitter, "minsize") {
        Value::Null => 30,
        v => v.to_i64().max(0),
    };
    let drag = splitter_drag(client_rect(&parent), &controls, i, min);
    let found = drag.is_some();
    DRAG.with(|d| *d.borrow_mut() = drag.map(|g| (splitter, children[g.control].clone(), g)));
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
    engine::realign(&mut Rt, &parent.to_lowercase(), changed);
}
