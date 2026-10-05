//! QGROUPBOX as Windows' group box (BS_GROUPBOX) draws it: an etched
//! frame (a shadow line, a light line inside it) whose top runs through
//! the middle of its Caption's line, the caption at 8 pixels from the
//! left over its own background. A container: its components are the
//! tree's children, Left / Top counted from its corner. A fluent theme's
//! frame is a thin rounded line.

use rapidr_value::objects::a11y::{mnemonic, AccessNode};
use rapidr_value::objects::ops::Place;
use rapidr_value::objects::text::text_size;

use super::{ComponentKind, Cx, MouseIn, MouseOut};
use crate::paint::{caption, color_of, ink_of, Painter};
use crate::store::{self, Store};

pub struct GroupBox;

/// Where the caption starts.
const CAPTION_LEFT: i64 = 8;

impl ComponentKind for GroupBox {
    fn name(&self) -> &'static str {
        "RGROUPBOX"
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
        let back = color_of(cx.store, cx.id).unwrap_or_else(|| crate::paint::backdrop(cx.store, cx.id));
        p.fill((0, 0, w, h), back);
        let text = store::string(cx.store, cx.id, "caption");
        let (shown, _) = mnemonic(&text);
        let (tw, th) = text_size(if shown.is_empty() { "X" } else { &shown }, &cx.font);
        // (the frame's top through the caption's middle)
        let top = th / 2;
        if t.fluent() {
            p.ring((0, top, w, h - top), t.radius, t.border, 1.0);
        } else {
            p.edge((0, top, w, h - top), &[t.shadow, t.light], &[t.light, t.shadow]);
        }
        if shown.is_empty() {
            return;
        }
        // (the caption over the frame, a pixel of background each side)
        let tw = tw.min(w - CAPTION_LEFT - 2).max(0);
        p.fill((CAPTION_LEFT - 2, 0, tw + 4, th), back);
        let color = ink_of(cx, back);
        p.clipped((CAPTION_LEFT, 0, tw, th), |p| caption(p, (CAPTION_LEFT, 0, tw, th), &text, &cx.font, color, Place::TopLeft));
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }
}
