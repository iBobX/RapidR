//! QSCROLLBOX, and the scroll bars of a form and of a scroll box: the
//! shared model (`rapidr_value::scrollbars`, which the FLTK and web
//! runtimes draw and route too) works out the bars and draws them as ops;
//! the kernel draws them over the components and gives them the mouse
//! first — a press on a bar never reaches the component under it, nor
//! OnMouseDown. Scrolling moves the components (their Left / Top, as
//! Delphi's ScrollBy): the kernel queues a
//! [`Container::Scrolled`] and runtime-core moves them after the pump.
//!
//! A scroll box is its edge (BorderStyle bsSingle, its default: Windows'
//! sunken client edge, 2 pixels) and, inside it, its components' area —
//! their Left / Top count from the inside of the edge — which its bars
//! share.

use std::cell::RefCell;

use rapidr_value::objects::a11y::{node_id, AccessNode, Role};
use rapidr_value::objects::ops::{lift, Rect};
use rapidr_value::scrollbars::{self, BAR};
use rapidr_value::Value;

use super::form::Container;
use super::{ComponentKind, Cx, MouseIn, MouseOut};
use crate::input::KernelEvent;
use crate::paint::{Painter, DARK, FACE, LIGHT, SHADOW};
use crate::store::Store;
use crate::text::bgr_to_rgb;
use crate::tree::FormUi;

pub struct ScrollBox;

/// A QSCROLLBOX's edge: 2 pixels (bsSingle, its default), 0 with
/// BorderStyle bsNone (runtime-core's `scroll::border`).
pub fn border(store: &dyn Store, id: &str) -> i64 {
    match store.get(id, "borderstyle") {
        Value::Null => 2,
        v if v.to_i64() == 0 => 0,
        _ => 2,
    }
}

/// What the bars of `id` (a form or a scroll box) take of its area: the
/// vertical bar's width, the horizontal bar's height.
pub fn bars_taken(id: &str) -> (i64, i64) {
    scrollbars::with(id, |s| (if s.vert.shown { BAR } else { 0 }, if s.horz.shown { BAR } else { 0 })).unwrap_or((0, 0))
}

/// The bars of `id` drawn for an area `w` × `h` (nothing without a bar).
fn bar_ops(id: &str, w: i64, h: i64, p: &mut Painter) {
    if let Some(ops) = scrollbars::with(id, |s| (s.vert.shown || s.horz.shown).then(|| s.ops(w, h))).flatten() {
        p.clipped((0, 0, w, h), |p| p.ops(lift(ops)));
    }
}

/// A form's bars (its client area at (0, `menu`), `area` with the bars).
pub fn paint_form_bars(form: &str, area: (i64, i64), p: &mut Painter, menu: i64) {
    p.at((0, menu), |p| bar_ops(form, area.0, area.1, p));
}

impl ComponentKind for ScrollBox {
    fn name(&self) -> &'static str {
        "RSCROLLBOX"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        // (the input lane's: OnClick, OnDblClick in the VCL's order)
        super::canvas::click_or_double(cx, m)
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let color = rapidr_value::objects::form_color(&cx.store.get(cx.id, "color"));
        p.fill((0, 0, w, h), bgr_to_rgb(color));
        if border(cx.store, cx.id) > 0 {
            // (Windows' client edge: sunken, two lines)
            p.edge((0, 0, w, h), &[SHADOW, DARK], &[LIGHT, FACE]);
        }
    }

    fn client_area(&self, store: &dyn Store, id: &str, w: i64, h: i64) -> Rect {
        let b = border(store, id);
        (b, b, (w - 2 * b).max(0), (h - 2 * b).max(0))
    }

    fn paint_over(&self, store: &dyn Store, id: &str, w: i64, h: i64, p: &mut Painter) {
        let b = border(store, id);
        p.at((b, b), |p| bar_ops(id, (w - 2 * b).max(0), (h - 2 * b).max(0), p));
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Pane);
        n.bounds = cx.rect;
        n
    }
}

/// The bars held down: their form, their container, its area's corner in
/// the window's client area (logical).
struct Held {
    form: String,
    name: String,
    origin: (i64, i64),
    /// Where the mouse is (an arrow held repeats while it's on it).
    at: (i64, i64),
}

thread_local! {
    static HELD: RefCell<Option<Held>> = const { RefCell::new(None) };
}

/// The containers in form `f` that scroll, innermost (smallest) first,
/// then the form: (id, its area's corner in the client area, its area).
fn scrollers(f: &FormUi, store: &dyn Store) -> Vec<(String, (i64, i64), (i64, i64))> {
    let mut found: Vec<(i64, String, (i64, i64), (i64, i64))> = Vec::new();
    for n in &f.nodes {
        if n.type_name != "RSCROLLBOX" || !n.shown {
            continue;
        }
        let b = border(store, &n.id);
        let (w, h) = ((n.abs.2 - 2 * b).max(0), (n.abs.3 - 2 * b).max(0));
        found.push((w * h, n.id.clone(), (n.abs.0 + b, n.abs.1 + b), (w, h)));
    }
    found.sort_by_key(|f| f.0);
    let mut out: Vec<_> = found.into_iter().map(|(_, n, o, a)| (n, o, a)).collect();
    out.push((f.form.clone(), (0, f.menu_offset), f.client));
    out
}

/// (x, y) of the client area in a container's area at `origin`.
fn local(origin: (i64, i64), x: f64, y: f64) -> (i64, i64) {
    ((x - origin.0 as f64).floor() as i64, (y - origin.1 as f64).floor() as i64)
}

fn scrolled(f: &mut FormUi, id: &str, (dx, dy): (i64, i64)) {
    f.dirty = true;
    if (dx, dy) != (0, 0) {
        f.events.push(KernelEvent::Container(Container::Scrolled { id: id.to_string(), dx, dy }));
    }
}

/// A press at (x, y) of form `f`'s client area: whether a scroll bar took
/// it (an arrow or the track scrolls at once; the thumb starts a drag).
pub(crate) fn bars_down(f: &mut FormUi, store: &dyn Store, x: f64, y: f64) -> bool {
    for (name, origin, (w, h)) in scrollers(f, store) {
        let (lx, ly) = local(origin, x, y);
        if lx < 0 || ly < 0 || lx >= w || ly >= h || !scrollbars::with(&name, |s| s.on_bars(lx, ly, w, h)).unwrap_or(false) {
            continue;
        }
        let shift = scrollbars::with_mut(&name, |s| s.mouse_down(lx, ly, w, h)).unwrap_or((0, 0));
        HELD.with(|h| *h.borrow_mut() = Some(Held { form: f.form.clone(), name: name.clone(), origin, at: (lx, ly) }));
        scrolled(f, &name, shift);
        // (an arrow or the track held repeats: Windows' 400 ms, then 50 ms)
        f.wakes.bars = Some(crate::tick::now() + crate::tick::REPEAT_DELAY);
        return true;
    }
    false
}

/// The held bar's container and its area now.
fn held(f: &FormUi, store: &dyn Store) -> Option<(String, (i64, i64), (i64, i64))> {
    let (name, origin) = HELD.with(|h| h.borrow().as_ref().filter(|h| h.form == f.form).map(|h| (h.name.clone(), h.origin)))?;
    let area = scrollers(f, store).into_iter().find(|s| s.0 == name).map_or((0, 0), |s| s.2);
    Some((name, origin, area))
}

/// The mouse moved while a bar is held: whether it was (the thumb follows).
pub(crate) fn bars_drag(f: &mut FormUi, store: &dyn Store, x: f64, y: f64) -> bool {
    let Some((name, origin, (w, h))) = held(f, store) else { return false };
    let (lx, ly) = local(origin, x, y);
    HELD.with(|h| {
        if let Some(h) = h.borrow_mut().as_mut() {
            h.at = (lx, ly);
        }
    });
    let shift = scrollbars::with_mut(&name, |s| s.mouse_drag(lx, ly, w, h));
    scrolled(f, &name, shift);
    true
}

/// The mouse let go: whether a bar was held (a dragged thumb's place is
/// where the components go, without Tracking).
pub(crate) fn bars_up(f: &mut FormUi, store: &dyn Store) -> bool {
    let Some((name, _, (w, h))) = held(f, store) else { return false };
    HELD.with(|h| h.borrow_mut().take());
    f.wakes.bars = None;
    let shift = scrollbars::with_mut(&name, |s| s.mouse_up(w, h));
    scrolled(f, &name, shift);
    true
}

/// A held arrow or track's repeat came (tick.rs): it steps again while the
/// mouse stays on it, then again every 50 ms.
pub(crate) fn bars_repeat(f: &mut FormUi, store: &dyn Store) {
    let Some((name, _, (w, h))) = held(f, store) else { return };
    let at = HELD.with(|h| h.borrow().as_ref().map_or((0, 0), |h| h.at));
    let shift = scrollbars::with_mut(&name, |s| s.repeat(at.0, at.1, w, h));
    scrolled(f, &name, shift);
    f.wakes.bars = Some(crate::tick::now() + crate::tick::REPEAT);
}

/// The wheel at (x, y) of form `f`'s client area, `notches` whole notches:
/// the innermost scroll box (or the form) under it whose bars show
/// scrolls; whether one did.
pub(crate) fn bars_wheel(f: &mut FormUi, store: &dyn Store, x: f64, y: f64, notches: i64, horizontal: bool) -> bool {
    for (name, origin, (w, h)) in scrollers(f, store) {
        let (lx, ly) = local(origin, x, y);
        let over = lx >= 0 && ly >= 0 && lx < w && ly < h;
        if !over || !scrollbars::with(&name, |s| s.vert.shown || s.horz.shown).unwrap_or(false) {
            continue;
        }
        let shift = scrollbars::with_mut(&name, |s| s.wheel(notches, horizontal, w, h));
        scrolled(f, &name, shift);
        return true;
    }
    false
}
