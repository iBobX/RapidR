//! QGLASSFRAME (UtilMind's TGlassy in RapidQ's runtime;
//! `rapidr_value::objects::glass`): the glass's colour (TransparentColor)
//! laid over what's under it at `100 − Transparency` percent — what's
//! under it being its parent's background (RapidR's windows aren't
//! see-through). Moveable: dragging it with the left button moves its form.
//! Takes no focus; clicks as a graphic control's.

use rapidr_value::objects::a11y::AccessNode;
use rapidr_value::objects::glass;

use super::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::input::KernelEvent;
use crate::paint::{color_of, Painter};
use crate::store::{self, Store};

pub struct GlassFrame;

/// The colour under it: its parent's (up the parents to one set), else the
/// theme's face.
fn under(store: &dyn Store, id: &str, face: u32) -> u32 {
    let mut at = store::string(store, id, "parent");
    for _ in 0..64 {
        if at.is_empty() {
            break;
        }
        if let Some(c) = color_of(store, &at) {
            return c;
        }
        let next = store::string(store, &at, "parent");
        if next.eq_ignore_ascii_case(&at) {
            break;
        }
        at = next;
    }
    face
}

/// Its form (up the parents to a QFORM).
fn form_of(store: &dyn Store, id: &str) -> Option<String> {
    let mut at = id.to_string();
    for _ in 0..64 {
        let parent = store::string(store, &at, "parent");
        if parent.is_empty() {
            return None;
        }
        if store.type_of(&parent).eq_ignore_ascii_case("RFORM") {
            return Some(parent.to_lowercase());
        }
        at = parent;
    }
    None
}

impl ComponentKind for GlassFrame {
    fn name(&self) -> &'static str {
        "RGLASSFRAME"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let t = p.theme();
        let under = under(cx.store, cx.id, t.face);
        let tint = store::int(cx.store, cx.id, "transparentcolor", 0);
        let transparency = store::int(cx.store, cx.id, "transparency", glass::TRANSPARENCY);
        p.fill((0, 0, cx.width(), cx.height()), glass::shade(under, tint, transparency));
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let left = m.button == rapidr_value::input::Button::Left;
        // (Moveable: the form follows the drag — the press point stays
        // under the mouse, so each move is its distance from it)
        if store::int(cx.store, cx.id, "moveable", 1) != 0 {
            match m.kind {
                MouseKind::Down if left => cx.ui.part = Some(((m.x.max(0.0) as usize) << 16) | (m.y.max(0.0) as usize & 0xFFFF)),
                MouseKind::Move if m.captured && cx.ui.part.is_some() => {
                    let p = cx.ui.part.unwrap_or(0);
                    let (dx, dy) = (m.x as i64 - (p >> 16) as i64, m.y as i64 - (p & 0xFFFF) as i64);
                    if let (Some(form), true) = (form_of(cx.store, cx.id), dx != 0 || dy != 0) {
                        let (l, tp) = (store::int(cx.store, &form, "left", 0), store::int(cx.store, &form, "top", 0));
                        cx.events.push(KernelEvent::Set { id: form.clone(), prop: "left".into(), value: l + dx });
                        cx.events.push(KernelEvent::Set { id: form, prop: "top".into(), value: tp + dy });
                    }
                }
                MouseKind::Up => cx.ui.part = None,
                _ => {}
            }
        }
        super::canvas::click_or_double(cx, m)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }
}
