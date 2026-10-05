//! QPANEL as RapidQ (Delphi's TPanel) draws it: its Color, its bevels
//! from the shared model (`rapidr_value::objects::bevel`: BevelOuter,
//! BorderWidth, BevelInner, each BevelWidth one-pixel frames; a raised
//! outer bevel by default) — the same frames the web runtime draws — and
//! its Caption on one line inside them, centred (Alignment
//! taLeftJustify 0 / taRightJustify 1 / taCenter 2), under its
//! components. A container: its components are the tree's children.

use rapidr_value::objects::a11y::{mnemonic, AccessNode};
use rapidr_value::objects::bevel;
use rapidr_value::objects::ops::Place;
use rapidr_value::objects::text::text_size;

use super::{ComponentKind, Cx, MouseIn, MouseOut};
use crate::paint::{caption, color_of, ink_of, Painter};
use crate::store::{self, Store};

pub struct Panel;

/// A bevel property, its default until the program sets it.
fn prop(store: &dyn Store, id: &str, name: &str) -> i64 {
    store::int(store, id, name, bevel::default(name).unwrap_or(0))
}

/// The panel's frames (outermost first).
pub fn frames(store: &dyn Store, id: &str) -> Vec<bevel::Frame> {
    bevel::frames(prop(store, id, "bevelouter"), prop(store, id, "bevelinner"), prop(store, id, "bevelwidth"), prop(store, id, "borderwidth"))
}

impl ComponentKind for Panel {
    fn name(&self) -> &'static str {
        "RPANEL"
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
        let t = p.theme();
        let back = color_of(cx.store, cx.id).unwrap_or(t.face);
        p.fill((0, 0, w, h), back);
        let frames = frames(cx.store, cx.id);
        for f in &frames {
            let i = f.inset;
            if w - 2 * i < 2 || h - 2 * i < 2 {
                break;
            }
            // (the model's light and dark lines: the theme's; a fluent
            // theme's bevels are thin lines of its border)
            let (lit, shaded) = match (t.fluent(), f.top_left == bevel::LIGHT) {
                (true, _) => (t.border, t.border),
                (false, true) => (t.light, t.shadow),
                (false, false) => (t.shadow, t.light),
            };
            p.edge((i, i, w - 2 * i, h - 2 * i), &[lit], &[shaded]);
        }
        let text = store::string(cx.store, cx.id, "caption");
        if text.is_empty() {
            return;
        }
        // (inside the bevels, as TPanel's DrawText)
        let i = frames.last().map_or(0, |f| f.inset + 1);
        let (x, y, iw, ih) = (i, i, (w - 2 * i).max(0), (h - 2 * i).max(0));
        let color = ink_of(cx, back);
        let (shown, _) = mnemonic(&text);
        let tw = text_size(&shown, &cx.font).0;
        let rect = match store::int(cx.store, cx.id, "alignment", 2) {
            0 => (x, y, iw, ih),
            1 => (x + iw - tw, y, tw, ih),
            _ => (x, y, iw, ih),
        };
        let place = if store::int(cx.store, cx.id, "alignment", 2) == 2 { Place::Center } else { Place::Left };
        p.clipped((x, y, iw, ih), |p| caption(p, rect, &text, &cx.font, color, place));
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }
}
