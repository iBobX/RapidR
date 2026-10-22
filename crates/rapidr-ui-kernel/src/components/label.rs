//! QLABEL: its Caption at the top of its rectangle, Alignment
//! (taLeftJustify 0, taRightJustify 1, taCenter 2), cut at its edges as
//! Windows' static control; a Color fills it when the program set one;
//! `&` marks a mnemonic that focuses the next component.
//!
//! (RapidR's) With `TabStop = 1` a label is a link: Tab reaches it, the
//! theme's focus ring shows around its text, Enter or Space clicks it
//! (OnClick), a screen reader announces a link named by its caption (or
//! AccessibleName) — what RapidR Studio's Recent list is. QLABEL has no
//! TabStop: RapidQ's labels never take the focus, and a label that isn't
//! given it doesn't.

use rapidr_value::objects::a11y::{mnemonic, AccessNode, Action};
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::Value;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::{backdrop, caption, ink_of, Painter};
use crate::store::{self, Store};
use crate::text::bgr_to_rgb;

pub struct Label;

/// (RapidR's) The program gave the label `TabStop = 1`: it's a link and
/// takes the focus.
fn tab_stop(store: &dyn Store, id: &str) -> bool {
    store::flag(store, id, "tabstop", false)
}

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

    fn focusable(&self, store: &dyn Store, id: &str) -> bool {
        tab_stop(store, id)
    }

    // (RapidR's TabStop: Enter or Space clicks it, as a link)
    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if matches!(k.vk, 13 | 32) && !k.mods.alt && !k.mods.command && tab_stop(cx.store, cx.id) {
            cx.click();
            return true;
        }
        false
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        if action == Action::Click && cx.state.enabled && tab_stop(cx.store, cx.id) {
            cx.click();
            return true;
        }
        false
    }

    /// An AutoSize label is RapidQ's size in every theme (its text measured
    /// as RC.EXE measures it, `rapidr_value::autosize`); where the theme's
    /// face is wider (Inter), its text runs on past its edge — to the right,
    /// the left or both, as its Alignment puts it — rather than being cut.
    fn room(&self, store: &dyn Store, id: &str, font: &rapidr_value::objects::font::Font, w: i64, h: i64) -> Rect {
        if !rapidr_value::autosize::autosize_on(&store.get(id, "autosize")) {
            return (0, 0, w, h);
        }
        let text = store::string(store, id, "caption");
        let wrap = store::flag(store, id, "wordwrap", false);
        let lines = rapidr_value::autosize::lines(&text, font, wrap.then_some(w));
        let size = |s: &str| rapidr_value::objects::text::text_size(s, font);
        let tw = lines.iter().map(|l| size(l).0).max().unwrap_or(0);
        let th = size(" ").1 * lines.len() as i64;
        let (dw, dh) = (if tw > w { tw - w + 1 } else { 0 }, (th - h).max(0));
        match store::int(store, id, "alignment", 0) {
            1 => (-dw, 0, w + dw, h + dh),
            2 => (-dw / 2 - 1, 0, w + dw + 2, h + dh),
            _ => (0, 0, w + dw, h + dh),
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        // (the input lane's: OnClick, OnDblClick in RapidQ's order)
        super::canvas::click_or_double(cx, m)
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        if let v @ (Value::Integer(_) | Value::Double(_)) = cx.store.get(cx.id, "color") {
            // (under all its text: where the theme's face runs on past the
            // label's edge too)
            p.fill(self.room(cx.store, cx.id, &cx.font, w, h), bgr_to_rgb(v.to_i64()));
        }
        let text = store::string(cx.store, cx.id, "caption");
        let color = ink_of(cx, backdrop(cx.store, cx.id));
        let wrap = store::flag(cx.store, cx.id, "wordwrap", false);
        // (RapidR's TabStop: the focus ring round the link's text — not the
        // label's whole, wider rectangle —, a little off it, kept inside)
        if cx.state.focused && tab_stop(cx.store, cx.id) {
            let tw = text_size(&mnemonic(&text).0, &cx.font).0.min(w);
            let x = match place(cx.store, cx.id) {
                Place::TopRight => w - tw,
                Place::TopCenter => (w - tw) / 2,
                _ => 0,
            };
            let (l, r) = ((x - 3).max(0), (x + tw + 3).min(w));
            p.focus_ring((l, 0, (r - l).max(1), h), false);
        }
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
