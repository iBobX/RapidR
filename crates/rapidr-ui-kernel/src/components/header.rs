//! QHEADER: the shared model (`rapidr_value::objects::header::Header`)
//! holds the sections and takes the mouse (a click on a section, a drag of
//! its right edge); its faces are painted on the header's own drawing
//! surface (a canvas bitmap: what OnDrawSection draws on, what `Pixel`
//! reads) by runtime-core before the pump, which also fires OnDrawSection
//! for the owner-drawn sections (`objects::paint_header`, as FLTK's
//! `header_refresh`). The kernel shows that surface and passes the mouse
//! on: OnSectionClick (Index), OnSectionTrack (Index, Width, State),
//! OnSectionResize (Index).

use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::objects::header::{Action as HeaderAction, TS_END};
use rapidr_value::objects::{with_canvas, with_header};
use rapidr_value::v_int;

use super::list::{fire, picture_of};
use super::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::paint::{Painter, FACE};
use crate::store::Store;

pub struct HeaderBar;

/// The model's actions, for the program.
fn actions(cx: &mut Cx, actions: Vec<HeaderAction>) {
    for a in actions {
        match a {
            HeaderAction::Click(i) => fire(cx, "onsectionclick", vec![v_int(i as i64)]),
            HeaderAction::Track(i, width, state) => {
                fire(cx, "onsectiontrack", vec![v_int(i as i64), v_int(width), v_int(state)]);
                if state == TS_END {
                    fire(cx, "onsectionresize", vec![v_int(i as i64)]);
                }
            }
        }
    }
}

impl ComponentKind for HeaderBar {
    fn name(&self) -> &'static str {
        "RHEADER"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        p.fill((0, 0, w, h), FACE);
        // (its surface, painted with its faces and what OnDrawSection drew;
        // at the header's size once runtime-core painted it)
        let shown = with_canvas(cx.id, w, h, |b| b.display_rgba());
        if let Some(px) = shown.filter(|px| px.0 > 0) {
            p.picture(&format!("{}#surface", cx.id), 0, picture_of(px), (0, 0, w, h));
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let x = m.x.floor() as i64;
        let acts = with_header(cx.id, |h| match m.kind {
            MouseKind::Down => h.press(x),
            MouseKind::Move if m.captured => h.drag_to(x),
            MouseKind::Up => h.release(x),
            _ => Vec::new(),
        })
        .unwrap_or_default();
        actions(cx, acts);
        MouseOut { press: false, focus: Some(false) }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        match (action, part) {
            (Action::Click, Some(i)) => {
                let clickable = with_header(cx.id, |h| h.sections.get(i).is_some_and(|s| s.allow_click)).unwrap_or(false);
                if clickable {
                    actions(cx, vec![HeaderAction::Click(i)]);
                }
                true
            }
            _ => false,
        }
    }
}
