//! `Align` in the web runtime: the desktop runtime's `layout.rs` on this
//! runtime's registry, with the same placement (`rapidr_value::layout`).
//! Laying out stores the children's Left / Top / Width / Height, which
//! moves their elements.
//!
//! A container is laid out again when an aligned child's Align, position,
//! size or visibility changes, when the container is resized (by the
//! program, or by maximizing / restoring a form), and when a form gets its
//! main menu. Anchored children follow their parent's client area at the
//! same moments (as on the desktop: [`client_changed`]). When all this
//! happens is `rapidr_value::layout::engine`'s — the one sequence the
//! desktop runtime and the IDE's designer run too; this file is this
//! runtime's registry as its store ([`Rt`]).

use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use rapidr_value::layout::{anchor_rules, anchor_set, engine, splitter_drag, Align, AnchorRules, Constraints, Control, Rect, SplitterDrag, DEFAULT_ANCHORS};

use crate::object_web::{get_children_of, rp_comp_get_stored, rp_comp_set, rp_comp_type};
use crate::value::{v_int, Value};

/// A form's main menu, above its client area (the UI kernel's in-window
/// bar); the frame is `rapidr_value::layout::form_frame`.
pub const MENU_HEIGHT: i64 = rapidr_value::layout::MAIN_MENU_HEIGHT;

thread_local! {
    static BUSY: Cell<u32> = const { Cell::new(0) };
    static ALIGNED_PARENTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    /// Containers with at least one anchored child (Anchors not the default).
    static ANCHORED_PARENTS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Whether geometry is being stored internally (see [`quietly`]).
pub fn is_quiet() -> bool {
    BUSY.with(Cell::get) > 0
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

/// A component's Anchors (akLeft + akTop until set).
fn anchors_of(name: &str) -> i64 {
    match rp_comp_get_stored(name, "anchors") {
        Value::Null => DEFAULT_ANCHORS,
        v => v.to_i64(),
    }
}

/// A component's MinWidth … MaxHeight.
pub fn constraints_of(name: &str) -> Constraints {
    Constraints::of(|p| rp_comp_get_stored(name, p).to_i64())
}

/// The client area `parent`'s anchored children follow (as on the
/// desktop): a form's or scroll box's inside (its scroll bars not
/// excluded), a tab control's page, any other container's whole size.
fn anchor_parent_size(parent: &str) -> (i64, i64) {
    if rp_comp_type(parent) == "RFORM" || crate::scroll_web::scrolls(parent) {
        return crate::scroll_web::area(parent);
    }
    let r = client_rect(parent);
    (r.width, r.height)
}

/// `parent`'s client area changed size: its aligned children are laid out
/// again and its anchored ones follow.
pub fn client_changed(parent: &str) {
    engine::client_changed(&mut Rt, &parent.to_uppercase());
}

/// Moves `parent`'s anchored children to follow its client area
/// (`rapidr_value::layout::anchor_controls`).
pub fn reanchor(parent: &str) {
    engine::reanchor(&mut Rt, &parent.to_uppercase());
}

fn has_menu(form: &str) -> bool {
    get_children_of(form).iter().any(|(_, t)| t == "RMAINMENU")
}

/// A form's ClientWidth / ClientHeight: its size less its frame and menu.
pub fn form_client(form: &str) -> (i64, i64) {
    crate::scroll_web::client(form)
}

/// A form's inside: its frame and menu excluded (its scroll bars not).
pub fn form_area(form: &str) -> (i64, i64) {
    let n = |p: &str| rp_comp_get_stored(form, p);
    let border_style = match n("borderstyle") {
        Value::Null => 2,
        v => v.to_i64(),
    };
    let menu = if has_menu(form) { MENU_HEIGHT } else { 0 };
    rapidr_value::layout::form_client_size(n("width").to_i64(), n("height").to_i64(), rapidr_value::layout::frame_style(form, border_style), menu)
}

/// The Width / Height giving a form this client size.
pub fn form_outer(form: &str, client_width: i64, client_height: i64) -> (i64, i64) {
    let border_style = match rp_comp_get_stored(form, "borderstyle") {
        Value::Null => 2,
        v => v.to_i64(),
    };
    let menu = if has_menu(form) { MENU_HEIGHT } else { 0 };
    rapidr_value::layout::form_outer_size(client_width, client_height, rapidr_value::layout::frame_style(form, border_style), menu)
}

/// The client area of `parent` in its children's coordinates: a form's
/// inside; any other container's whole size.
pub fn client_rect(parent: &str) -> Rect {
    // A form's / scroll box's: Delphi's (AdjustClientRect) — the scrolled
    // area, as large as the ranges.
    if crate::scroll_web::scrolls(parent) {
        let (cw, ch) = crate::scroll_web::client(parent);
        let (hp, vp, hr, vr) = rapidr_value::scrollbars::with(parent, |s| (s.horz.position, s.vert.position, s.horz.range, s.vert.range)).unwrap_or_default();
        return Rect::new(-hp, -vp, cw.max(hr), ch.max(vr));
    }
    // A tab control's: the area under its tabs (rapidr_value::objects::tabcontrol).
    if rapidr_value::objects::is_tabcontrol(parent) {
        let n = |p: &str| rp_comp_get_stored(parent, p).to_i64();
        let font = rapidr_value::objects::font_from_props(parent, &|id, p| crate::object_web::rp_comp_get(id, p));
        if let Some((x, y, w, h)) = rapidr_value::objects::with_tabcontrol(parent, |t| t.display(n("width"), n("height"), &font)) {
            return Rect::new(x, y, w, h);
        }
    }
    if rp_comp_type(parent) == "RFORM" {
        let (w, h) = form_client(parent);
        return Rect::new(0, 0, w, h);
    }
    let n = |p: &str| rp_comp_get_stored(parent, p).to_i64();
    // (a panel's: inside its bevels — rapidr_value::objects::bevel)
    if rp_comp_type(parent) == "RPANEL" {
        let i = rapidr_value::objects::bevel::client_inset(&|p| rp_comp_get_stored(parent, p));
        return Rect::new(i, i, (n("width") - 2 * i).max(0), (n("height") - 2 * i).max(0));
    }
    Rect::new(0, 0, n("width"), n("height"))
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
        name.to_uppercase()
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
        ALIGNED_PARENTS.with(|a| a.borrow().contains(&parent.to_uppercase()))
    }
    fn mark_aligned(&mut self, parent: &str) {
        ALIGNED_PARENTS.with(|a| a.borrow_mut().insert(parent.to_uppercase()));
    }
    fn has_anchored(&self, parent: &str) -> bool {
        ANCHORED_PARENTS.with(|a| a.borrow().contains(&parent.to_uppercase()))
    }
    fn mark_anchored(&mut self, parent: &str) {
        ANCHORED_PARENTS.with(|a| a.borrow_mut().insert(parent.to_uppercase()));
    }
    fn scroll_update(&mut self, name: &str) {
        crate::scroll_web::update(name);
    }
    // (RapidQ aligns nothing before the form's first Show: rapidr_value::
    // layout's align_controls_unshown; ui-app's forms mark the first Show)
    fn unshown(&self, parent: &str) -> bool {
        crate::object_web::form_of(parent).is_some_and(|f| !crate::object_web::rp_comp_get_stored(&f, "__shownonce").to_bool())
    }
}

/// Called by `rp_comp_set` after it stored `prop` (lowercase) of `name`.
pub fn after_set(name: &str, prop: &str) {
    if is_quiet() {
        return;
    }
    match prop {
        "borderstyle" if rp_comp_type(name) == "RFORM" => client_changed(name),
        // (a QSCROLLBOX's edge: its inside changed)
        "borderstyle" => {
            crate::scroll_web::update(name);
            reanchor(name);
        }
        _ => engine::after_set(&mut Rt, &name.to_uppercase(), prop),
    }
}

fn controls_of(parent: &str) -> (Vec<String>, Vec<Control>) {
    engine::controls_of(&Rt, &parent.to_uppercase())
}

thread_local! {
    /// The QSPLITTER being dragged: its name, the control it resizes and how.
    static DRAG: RefCell<Option<(String, String, SplitterDrag)>> = const { RefCell::new(None) };
}

/// The user pressed the mouse on `splitter`: returns whether there's a
/// control next to it to resize (as on the desktop).
pub fn splitter_begin(splitter: &str) -> bool {
    let splitter = splitter.to_uppercase();
    let parent = parent_of(&splitter);
    let (children, controls) = controls_of(&parent);
    let Some(i) = children.iter().position(|n| *n == splitter) else { return false };
    let min = match rp_comp_get_stored(&splitter, "minsize") {
        Value::Null => 30,
        v => v.to_i64().max(0),
    };
    let drag = splitter_drag(client_rect(&parent), &controls, i, min);
    let found = drag.is_some();
    DRAG.with(|d| *d.borrow_mut() = drag.map(|g| (splitter, children[g.control].clone(), g)));
    found
}

/// The mouse moved `delta` pixels since the drag began.
pub fn splitter_move(delta: i64) {
    let Some((_, target, drag)) = DRAG.with(|d| d.borrow().clone()) else { return };
    let size = drag.size_for(delta);
    let prop = if drag.horizontal { "width" } else { "height" };
    if rp_comp_get_stored(&target, prop).to_i64() != size {
        rp_comp_set(&target, prop, v_int(size));
    }
}

/// The drag ended: the splitter's OnMoved fires.
pub fn splitter_end() {
    if let Some((splitter, _, _)) = DRAG.with(|d| d.borrow_mut().take()) {
        crate::object_web::rp_fire_event(&splitter, "onmoved");
    }
}

/// Lays out the aligned children of `parent`; `changed` is the child whose
/// Align, size or visibility just changed.
pub fn realign(parent: &str, changed: Option<&str>) {
    engine::realign(&mut Rt, &parent.to_uppercase(), changed);
}

/// (a QLABEL's AutoSize, `rapidr_value::autosize`, as on the desktop)
/// Before `rp_comp_set` stores `prop` of `name`: what can resize labels — a
/// label's Caption, AutoSize, WordWrap; any component's font or Parent (the
/// fonts of the labels it is or holds) — for [`labels_after_set`].
pub fn labels_before_set(name: &str, prop: &str) -> Option<rapidr_value::autosize::Before> {
    let t = rp_comp_type(name);
    if t.is_empty() || t == "RUDT" || rapidr_value::objects::is_object_type(&t) {
        return None;
    }
    rapidr_value::autosize::before_set(name, &t, &prop.to_ascii_lowercase(), &rp_comp_get_stored, &get_children_of, &label_font)
}

/// The property is stored: each AutoSize label it changed takes its text's
/// size (Left too, right-aligned).
pub fn labels_after_set(name: &str, before: rapidr_value::autosize::Before) {
    let get = |i: &str, p: &str| crate::object_web::rp_comp_get(i, p);
    for label in rapidr_value::autosize::changed_labels(name, before, &rp_comp_get_stored, &label_font) {
        if let Some(r) = rapidr_value::autosize::label_bounds(&label, &get) {
            for (p, v) in [("left", r.left), ("width", r.width), ("height", r.height)] {
                if rp_comp_get_stored(&label, p).to_i64() != v {
                    rp_comp_set(&label, p, v_int(v));
                }
            }
        }
    }
}

/// The font label `name` is drawn in, its Font.Color the one it reads.
fn label_font(name: &str) -> rapidr_value::autosize::FontKey {
    (rapidr_value::objects::font_from_props(name, &|i, p| crate::object_web::rp_comp_get(i, p)), crate::object_web::program_font_color(name).to_i64())
}
