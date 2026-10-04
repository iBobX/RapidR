//! QTABCONTROL: the shared model (`rapidr_value::objects::tabcontrol`)
//! lays its tabs out, draws them as ops, picks a tab under a click or the
//! arrow keys and follows the mouse (HotTrack), as the web runtime
//! routes it. Its components are ordinary children of the control: they
//! are placed and drawn by the kernel's tree, over it.

use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::objects::ops::lift;
use rapidr_value::objects::{form_color, with_tabcontrol, with_tabcontrol_mut};
use rapidr_value::v_int;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::Painter;

pub struct Tabs;

impl ComponentKind for Tabs {
    fn name(&self) -> &'static str {
        "RTABCONTROL"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let color = form_color(&cx.store.get(cx.id, "color"));
        let ops = with_tabcontrol(cx.id, |t| t.ops(w, h, &cx.font, color, cx.state.enabled, cx.state.focused)).unwrap_or_default();
        p.ops(lift(ops));
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width(), cx.height());
        let font = cx.font.clone();
        match m.kind {
            MouseKind::Down => match with_tabcontrol_mut(cx.id, |t| t.mouse_down(m.x as i64, m.y as i64, w, h, &font)).flatten() {
                Some((changed, focus)) => {
                    if changed {
                        cx.change();
                    }
                    MouseOut { press: false, focus: Some(focus) }
                }
                // (not on a tab: the area is its components')
                None => MouseOut { press: false, focus: Some(false) },
            },
            MouseKind::Move => {
                let at = m.inside.then_some((m.x as i64, m.y as i64));
                with_tabcontrol_mut(cx.id, |t| t.mouse_move(at, w, h, &font));
                MouseOut::default()
            }
            MouseKind::Leave => {
                with_tabcontrol_mut(cx.id, |t| t.mouse_move(None, w, h, &font));
                MouseOut::default()
            }
            MouseKind::Up => MouseOut::default(),
        }
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if !(37..=40).contains(&k.vk) || k.mods.alt || k.mods.command {
            return false;
        }
        let (w, h, font) = (cx.width(), cx.height(), cx.font.clone());
        if with_tabcontrol_mut(cx.id, |t| t.key(k.vk, w, h, &font)).unwrap_or(false) {
            cx.change();
        }
        true
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let Some(k) = part else { return false };
        if !matches!(action, Action::Click | Action::ScrollIntoView) {
            return false;
        }
        let (w, h, font) = (cx.width(), cx.height(), cx.font.clone());
        // (a click at the tab, as the user's; one scrolled out of view is
        // picked as the arrow keys would)
        let changed = with_tabcontrol_mut(cx.id, |t| match t.tab_rect(k, w, h, &font) {
            Some((x, y, tw, th)) => t.mouse_down(x + tw / 2, y + th / 2, w, h, &font).is_some_and(|r| r.0),
            None if k < t.tabs.len() && t.index != k as i64 => {
                t.set("tabindex", &v_int(k as i64));
                true
            }
            None => false,
        });
        if changed.unwrap_or(false) {
            cx.change();
        }
        true
    }
}
