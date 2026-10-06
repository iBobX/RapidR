//! QLABEL: its Caption at the top of its rectangle, Alignment
//! (taLeftJustify 0, taRightJustify 1, taCenter 2), cut at its edges as
//! Windows' static control; a Color fills it when the program set one;
//! `&` marks a mnemonic that focuses the next component.

use rapidr_value::objects::a11y::{mnemonic, AccessNode};
use rapidr_value::objects::ops::Place;
use rapidr_value::Value;

use super::{ComponentKind, Cx, MouseIn, MouseOut};
use crate::paint::{backdrop, caption, ink_of, Painter};
use crate::store::{self, Store};
use crate::text::bgr_to_rgb;

pub struct Label;

/// Where its caption sits for its Alignment.
pub fn place(store: &dyn Store, id: &str) -> Place {
    match store::int(store, id, "alignment", 0) {
        1 => Place::TopRight,
        2 => Place::TopCenter,
        _ => Place::TopLeft,
    }
}

impl ComponentKind for Label {
    fn name(&self) -> &'static str {
        "RLABEL"
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
        if let v @ (Value::Integer(_) | Value::Double(_)) = cx.store.get(cx.id, "color") {
            p.fill((0, 0, w, h), bgr_to_rgb(v.to_i64()));
        }
        let text = store::string(cx.store, cx.id, "caption");
        let color = ink_of(cx, backdrop(cx.store, cx.id));
        let wrap = store::flag(cx.store, cx.id, "wordwrap", false);
        if !wrap && !text.contains(['\r', '\n']) {
            caption(p, (0, 0, w, h), &text, &cx.font, color, place(cx.store, cx.id));
            return;
        }
        // (DrawText: a line at each line break, and with WordWrap broken
        // at spaces to fit the width — as AutoSize measured it,
        // rapidr_value::autosize — one under the other)
        let line_h = rapidr_value::objects::text::text_size(" ", &cx.font).1.max(1);
        let lines = rapidr_value::autosize::lines(&text, &cx.font, wrap.then_some(w));
        for (k, line) in lines.iter().enumerate() {
            let top = k as i64 * line_h;
            if top >= h {
                break;
            }
            // (the `&`s are gone from the lines: drawn as they are)
            let shown = line.replace('&', "&&");
            caption(p, (0, top, w, line_h), &shown, &cx.font, color, place(cx.store, cx.id));
        }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }

    fn mnemonic(&self, store: &dyn Store, id: &str) -> Option<char> {
        mnemonic(&store::string(store, id, "caption")).1.map(|m| m.1)
    }
}
