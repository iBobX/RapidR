//! QTRACKBAR: the shared model (`rapidr_value::objects::trackbar`) does
//! everything — its shapes, a press beside the thumb (a page toward it) or
//! on it (a drag), the keys (arrows, Page Up / Down, Home / End) — as the
//! web runtime routes it; OnChange when the user moved it. A fluent theme
//! draws it from the model's parts ([`TrackBar::parts`]): a thin rounded
//! rail, the accent up to the thumb, a round thumb with the accent in it.
//!
//! [`TrackBar::parts`]: rapidr_value::objects::trackbar::TrackBar::parts

use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::objects::{with_trackbar, with_trackbar_mut};
use rapidr_value::v_int;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::Painter;

pub struct Trackbar;

/// A fluent theme's track bar from the model's parts.
fn paint_fluent(cx: &Cx, p: &mut Painter, parts: &rapidr_value::objects::trackbar::Parts) {
    let t = p.theme();
    let s = cx.state;
    let vertical = parts.vertical;
    // (along, across) → the control's (x, y)
    let rect = |a0: f64, c0: f64, a1: f64, c1: f64| {
        let (a0, a1) = (a0.min(a1).round() as i64, a0.max(a1).round() as i64);
        let (c0, c1) = (c0.round() as i64, c1.round() as i64);
        if vertical { (c0, a0, c1 - c0, a1 - a0) } else { (a0, c0, a1 - a0, c1 - c0) }
    };
    let mid = parts.middle;
    let (rail, value) = if s.enabled { (t.channel, t.accent) } else { (t.slider_disabled, t.gray_text) };
    p.round(rect(parts.channel.0, mid - 2.0, parts.channel.1, mid + 2.0), 2.0, Some(rail), None, 1.0);
    // (the value: from Min to the thumb; a selection range over it)
    let (from, to) = parts.range.unwrap_or((parts.start, parts.thumb));
    if to > from {
        p.round(rect(from, mid - 2.0, to, mid + 2.0), 2.0, Some(value), None, 1.0);
    }
    let tick = if s.enabled { t.ticks } else { t.ticks_disabled };
    for &(a, long) in &parts.ticks {
        let len = if long { 4.0 } else { 3.0 };
        for (from, dir) in [(parts.ticks_before, -1.0), (parts.ticks_after, 1.0)] {
            let Some(c) = from else { continue };
            let (c0, c1) = (c, c + dir * len);
            let pts = if vertical { [(c0, a), (c1, a)] } else { [(a, c0), (a, c1)] };
            p.stroke(&pts, tick, 1.0);
        }
    }
    // (the thumb: a circle on the rail, the accent's dot in it — bigger
    // under the mouse, smaller held)
    let d = 18i64;
    let (ax, ay) = if vertical { (mid, parts.thumb) } else { (parts.thumb, mid) };
    let (x0, y0) = ((ax - d as f64 / 2.0).round() as i64, (ay - d as f64 / 2.0).round() as i64);
    let fill = if t.dark { t.slider_edge } else { t.control };
    p.round((x0, y0, d, d), d as f64 / 2.0, Some(fill), Some(t.border), 1.0);
    let dot = if !s.enabled || cx.ui.dragging { 8 } else if s.hover { 12 } else { 10 };
    let k = (d - dot) / 2;
    p.round((x0 + k, y0 + k, dot, dot), dot as f64 / 2.0, Some(if s.enabled { t.slider } else { t.slider_disabled }), None, 1.0);
    if s.focused {
        let r = t.focus_width as i64 + 1;
        p.focus((x0 - r, y0 - r, d + 2 * r, d + 2 * r));
    }
}

impl ComponentKind for Trackbar {
    fn name(&self) -> &'static str {
        "RTRACKBAR"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        if p.fluent() {
            if let Some(parts) = with_trackbar(cx.id, |t| t.parts(w as f64, h as f64)) {
                paint_fluent(cx, p, &parts);
            }
            return;
        }
        let shapes = with_trackbar(cx.id, |t| t.shapes(w as f64, h as f64, cx.state.enabled)).unwrap_or_default();
        for s in shapes {
            p.shape(s);
        }
        if cx.state.focused {
            p.focus((0, 0, w, h));
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width() as f64, cx.height() as f64);
        match m.kind {
            MouseKind::Down => {
                let (drag, changed) = with_trackbar_mut(cx.id, |t| t.mouse_down(m.x, m.y, w, h)).unwrap_or_default();
                cx.ui.dragging = drag;
                if changed {
                    cx.change();
                }
            }
            MouseKind::Move if m.captured && cx.ui.dragging => {
                if with_trackbar_mut(cx.id, |t| t.drag(m.x, m.y, w, h)).unwrap_or(false) {
                    cx.change();
                }
            }
            MouseKind::Up => cx.ui.dragging = false,
            _ => {}
        }
        MouseOut::default()
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if !(33..=40).contains(&k.vk) || k.mods.alt || k.mods.command {
            return false;
        }
        if with_trackbar_mut(cx.id, |t| t.key(k.vk)).unwrap_or(false) {
            cx.change();
        }
        true
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, value: Option<&AccessValue>) -> bool {
        let changed = match (action, value) {
            // (as the arrow keys would)
            (Action::Increment, _) => with_trackbar_mut(cx.id, |t| t.key(39)),
            (Action::Decrement, _) => with_trackbar_mut(cx.id, |t| t.key(37)),
            (Action::SetValue, Some(AccessValue::Number(v))) => with_trackbar_mut(cx.id, |t| {
                let before = t.position;
                t.set("position", &v_int(v.round() as i64));
                t.position != before
            }),
            _ => return false,
        };
        if changed.unwrap_or(false) {
            cx.change();
        }
        true
    }
}
