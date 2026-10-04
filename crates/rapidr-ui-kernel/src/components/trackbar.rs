//! QTRACKBAR: the shared model (`rapidr_value::objects::trackbar`) does
//! everything — its shapes, a press beside the thumb (a page toward it) or
//! on it (a drag), the keys (arrows, Page Up / Down, Home / End) — as the
//! FLTK and web runtimes route it; OnChange when the user moved it.

use rapidr_value::objects::a11y::{node_id, AccessNode, Action};
use rapidr_value::objects::{with_trackbar, with_trackbar_mut};
use rapidr_value::v_int;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::Painter;

pub struct Trackbar;

impl ComponentKind for Trackbar {
    fn name(&self) -> &'static str {
        "RTRACKBAR"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
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
        let (w, h) = (cx.width(), cx.height());
        let mut n = with_trackbar(cx.id, |t| t.describe(node_id(cx.id), w, h)).unwrap_or_else(|| AccessNode::new(node_id(cx.id), rapidr_value::objects::a11y::Role::Slider));
        n.actions.push(Action::Focus);
        n.offset(cx.rect.0, cx.rect.1);
        n
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
