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

/// A link's text is this far inside its rectangle, on each side: the focus
/// ring has room round the letters.
const LINK_PAD: i64 = 4;

/// (RapidR's) The program gave the label `TabStop = 1`: it's a link and
/// takes the focus.
fn tab_stop(store: &dyn Store, id: &str) -> bool {
    store::flag(store, id, "tabstop", false)
}

/// (POLISH-2) A WordWrap label's lines as last wrapped, and what for: its
/// caption, font, width and the screen's scale.
pub struct Wrapped {
    key: (String, rapidr_value::objects::font::Font, i64, u64),
    lines: Vec<String>,
}

/// The lines of `text` wrapped to `width` as they are drawn: each measured
/// by the text system that draws it, at the scale it's drawn at (a modern
/// theme's face at 2× rounds each glyph to the device's pixels — measured
/// any other way, a line could come out wider than the label and lose its
/// last letter). Kept until the caption, font, width or scale change.
fn drawn_lines(cx: &mut Cx, text: &str, width: i64) -> Vec<String> {
    let scale = if cx.scale > 0.0 { cx.scale } else { 1.0 };
    let key = (text.to_string(), cx.font.clone(), width, scale.to_bits());
    if let Some(w) = cx.ui.wrapped.as_ref().filter(|w| w.key == key) {
        return w.lines.clone();
    }
    let lines = wrap_as_drawn(&mut *cx.text, &cx.font, text, width, scale);
    cx.ui.wrapped = Some(Box::new(Wrapped { key, lines: lines.clone() }));
    lines
}

/// `text`'s lines wrapped to `width` logical pixels, measured by `ts` at
/// `scale` as the renderer lays each line out.
pub fn wrap_as_drawn(ts: &mut crate::text::TextSystem, font: &rapidr_value::objects::font::Font, text: &str, width: i64, scale: f64) -> Vec<String> {
    rapidr_value::autosize::lines_by(text, Some(width as f64), &mut |s| if s.is_empty() { 0.0 } else { f64::from(ts.layout(s, font, 0, scale as f32).full_width()) / scale })
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
        // (RapidR's TabStop: a link's text sits LINK_PAD inside its rectangle,
        // so the focus ring round it — not round the label's whole, wider
        // rectangle — has room off the letters, kept inside the label)
        let link = tab_stop(cx.store, cx.id);
        let pad = if link { LINK_PAD } else { 0 };
        if link && cx.state.focused {
            let tw = text_size(&mnemonic(&text).0, &cx.font).0.min(w - 2 * pad).max(0);
            let x = match place(cx.store, cx.id) {
                Place::TopRight => w - pad - tw,
                Place::TopCenter => (w - tw) / 2,
                _ => pad,
            };
            let (l, r) = ((x - LINK_PAD).max(0), (x + tw + LINK_PAD).min(w));
            p.focus_ring((l, 0, (r - l).max(1), h), false);
        }
        if !wrap && !text.contains(['\r', '\n']) {
            caption(p, (pad, 0, w - 2 * pad, h), &text, &cx.font, color, place(cx.store, cx.id));
            return;
        }
        // (DrawText: a line at each line break, and with WordWrap broken
        // at spaces to fit the width — as AutoSize measured it,
        // rapidr_value::autosize — one under the other)
        let line_h = rapidr_value::objects::text::text_size(" ", &cx.font).1.max(1);
        // (POLISH-2: wrapped where the text, as drawn, fits — inside a
        // link's padding too)
        let lines = if wrap { drawn_lines(cx, &text, w - 2 * pad) } else { rapidr_value::autosize::lines(&text, &cx.font, None) };
        for (k, line) in lines.iter().enumerate() {
            let top = k as i64 * line_h;
            if top >= h {
                break;
            }
            // (the `&`s are gone from the lines: drawn as they are)
            let shown = line.replace('&', "&&");
            caption(p, (pad, top, w - 2 * pad, line_h), &shown, &cx.font, color, place(cx.store, cx.id));
        }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }

    fn mnemonic(&self, store: &dyn Store, id: &str) -> Option<char> {
        mnemonic(&store::string(store, id, "caption")).1.map(|m| m.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::TextSystem;
    use rapidr_value::objects::font::Font;

    /// (POLISH-2) A WordWrap label in a modern theme's face at 2×: every
    /// line, as the renderer lays it out at that scale, fits the label's
    /// width — the wrap measured with the drawing's own metrics (each glyph
    /// on the device's pixels), not the 1× sums that came out narrower and
    /// cut a word's last letter.
    #[test]
    fn wrapped_lines_fit_as_drawn_at_2x() {
        let mut ts = TextSystem::new();
        let captions = [
            "Choose a model first (the server lists its own): start the server and Look Again if the list is empty.",
            "Send shares your message, the chat so far, RapidR's instructions and what the assistant reads of the open project.",
            "mmmm wwww MMMM WWWW mmmm wwww MMMM WWWW mmmm wwww",
        ];
        let (mut checked, mut old_too_wide) = (0, 0);
        for name in ["Inter", "RapidR Sans"] {
            for size in [8, 10] {
                let font = Font { name: name.into(), size, color: 0, styles: 0 };
                for caption in captions {
                    for width in (90..=300).step_by(15) {
                        for scale in [2.0, 1.0] {
                            let drawn = |ts: &mut TextSystem, line: &str| f64::from(ts.layout(line, &font, 0, scale as f32).width()) / scale;
                            // (a word wider than the label is a line of its own, as DrawText's)
                            let fits = |ts: &mut TextSystem, line: &str| !line.trim().contains(' ') || drawn(ts, line) <= width as f64 + 1e-6;
                            for line in wrap_as_drawn(&mut ts, &font, caption, width, scale) {
                                assert!(fits(&mut ts, &line), "{name} {size} at {scale}×: {line:?} is {} wide in {width}", drawn(&mut ts, &line));
                                checked += 1;
                            }
                            // (what the 1× sums wrapped, drawn at this scale)
                            for line in rapidr_value::autosize::lines(caption, &font, Some(width)) {
                                old_too_wide += usize::from(!fits(&mut ts, &line));
                            }
                        }
                    }
                }
            }
        }
        assert!(checked > 200);
        eprintln!("(the 1× wrap drew {old_too_wide} lines wider than their label)");
    }
}
