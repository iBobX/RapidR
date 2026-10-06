//! QCHECKBOX, Windows-classic (DrawFrameControl DFCS_BUTTONCHECK): a
//! 13 × 13 box, sunken, white (the face while pressed or disabled), the
//! check mark black; the caption to its right, centred down, its `&`
//! letter underlined; the focus rectangle around the caption. A click (a
//! press and a release on it), Space, Alt + its letter or a screen
//! reader's click turns Checked over, then OnClick — Checked goes to the
//! store through [`KernelEvent::Set`](crate::KernelEvent::Set), so the
//! handler reads the new value. A fluent theme draws the box rounded, the
//! accent with a white check mark when checked, and a focus ring.

use rapidr_value::objects::a11y::{mnemonic, AccessNode, Action};
use rapidr_value::objects::ops::Place;
use rapidr_value::objects::text::text_size;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::{backdrop, caption, ink_of, Painter};
use crate::store::{self, Store};

/// The box's side.
pub const BOX: i64 = 13;
/// From the box to the caption.
pub const GAP: i64 = 5;

/// Windows' check mark: 7 columns of 3 pixels, its top left at (x, y).
pub fn check_mark(p: &mut Painter, x: i64, y: i64, color: u32) {
    if !p.one_to_one() {
        // (a high-DPI screen: the same band, its steps smoothed into
        // diagonals, at the screen's resolution)
        let (x, y) = (x as f64, y as f64);
        let points = vec![(x, y + 2.5), (x + 2.5, y + 5.0), (x + 7.0, y + 0.5), (x + 7.0, y + 2.5), (x + 2.5, y + 7.0), (x, y + 4.5)];
        p.op(rapidr_value::objects::ops::Op::Polygon { points, color });
        return;
    }
    for (c, top) in [2, 3, 4, 3, 2, 1, 0].into_iter().enumerate() {
        p.fill((x + c as i64, y + top, 1, 3), color);
    }
}

/// A sunken 13 × 13 box (a check box's) at (x, y), `well` inside.
pub fn sunken_box(p: &mut Painter, x: i64, y: i64, well: u32) {
    p.fill((x + 2, y + 2, BOX - 4, BOX - 4), well);
    p.sunken_edge((x, y, BOX, BOX));
}

/// A fluent theme's mark (a check box's `side`-pixel box, a radio
/// button's circle — `round`) at (x, y): the accent when on, else a rim
/// on the control's face; its fill under the mouse and pressed.
pub fn fluent_mark(p: &mut Painter, (x, y, side): (i64, i64, i64), round: bool, on: bool, s: super::State) {
    let t = p.theme();
    let radius = if round { side as f64 / 2.0 } else { (t.radius - 1.0).max(2.0) };
    let rect = (x, y, side, side);
    if on {
        let fill = match (s.enabled, s.pressed, s.hover) {
            (false, ..) => t.gray_text,
            (_, true, _) => t.accent_pressed,
            (_, _, true) => t.accent_hot,
            _ => t.accent,
        };
        p.round(rect, radius, Some(fill), None, 1.0);
        let ink = if s.enabled { t.accent_text } else { t.face };
        if round {
            // (the dot: bigger under the mouse, smaller pressed)
            let r = if s.pressed { 2.0 } else if s.hover { 3.0 } else { 2.5 };
            let c = side as f64 / 2.0;
            let d = (c - r).round() as i64;
            let size = side - 2 * d;
            p.round((x + d, y + d, size, size), size as f64 / 2.0, Some(ink), None, 1.0);
        } else {
            p.check_glyph(x as f64, y as f64, side as f64, ink);
        }
    } else {
        let fill = match (s.enabled, s.pressed, s.hover) {
            (false, ..) => t.control_disabled,
            (_, true, _) => t.control_pressed,
            (_, _, true) => t.control_hot,
            _ => t.control,
        };
        let rim = if s.enabled { t.border_strong } else { t.gray_text };
        p.round(rect, radius, Some(fill), Some(rim), 1.0);
    }
}

/// A check box's or radio button's caption, right of its mark (and the
/// focus rectangle around it).
pub fn caption_right(cx: &Cx, p: &mut Painter, mark: i64) {
    let (w, h) = (cx.width(), cx.height());
    let t = p.theme();
    let text = store::string(cx.store, cx.id, "caption");
    let mut color = ink_of(cx, backdrop(cx.store, cx.id));
    let x = mark + GAP;
    // (disabled, classic: embossed — white under the shadow grey, as
    // Windows' DrawState)
    if !cx.state.enabled && !t.fluent() {
        caption(p, (x + 1, 1, w - x, h), &text, &cx.font, t.light, Place::Left);
        color = t.shadow;
    }
    caption(p, (x, 0, w - x, h), &text, &cx.font, color, Place::Left);
    if cx.state.focused {
        let (tw, th) = text_size(&mnemonic(&text).0, &cx.font);
        if t.fluent() {
            // (a ring round the caption, clear of it and of the mark)
            let pad = t.focus_width as i64 + 2;
            let (fx, fh) = (x - pad, (th + 2 * pad).min(h));
            p.focus((fx, (h - fh) / 2, (tw + 2 * pad).min(w - fx).max(1), fh.max(1)));
        } else {
            let (fw, fh) = ((tw + 2).min(w - x + 1), th + 2);
            p.focus((x - 1, (h - th) / 2 - 1, fw.max(1), fh));
        }
    }
}

/// Whether a property reads as checked (-1, 1, True …).
pub fn checked(store: &dyn Store, id: &str) -> bool {
    store::flag(store, id, "checked", false)
}

pub struct CheckBox;

impl CheckBox {
    fn toggle(cx: &mut Cx) {
        let now = checked(cx.store, cx.id);
        let id = cx.id.to_string();
        cx.set(&id, "checked", i64::from(!now));
        cx.click();
    }
}

impl ComponentKind for CheckBox {
    fn name(&self) -> &'static str {
        "RCHECKBOX"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let h = cx.height();
        let s = cx.state;
        let t = p.theme();
        let y = (h - BOX) / 2;
        if t.fluent() {
            fluent_mark(p, (0, y, BOX), false, checked(cx.store, cx.id), s);
        } else {
            sunken_box(p, 0, y, if s.pressed || !s.enabled { t.face } else { t.window });
            if checked(cx.store, cx.id) {
                check_mark(p, 3, y + 3, if s.enabled { t.text } else { t.gray_text });
            }
        }
        caption_right(cx, p, BOX);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        match m.kind {
            MouseKind::Down => MouseOut { press: true, focus: None },
            MouseKind::Up if cx.state.held && m.inside => {
                Self::toggle(cx);
                MouseOut::default()
            }
            _ => MouseOut::default(),
        }
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if k.vk == 32 && !k.mods.alt && !k.mods.command {
            Self::toggle(cx);
            return true;
        }
        false
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        if action == Action::Click && cx.state.enabled {
            Self::toggle(cx);
            return true;
        }
        false
    }

    fn mnemonic(&self, store: &dyn Store, id: &str) -> Option<char> {
        mnemonic(&store::string(store, id, "caption")).1.map(|m| m.1)
    }

    fn mnemonic_clicks(&self) -> bool {
        true
    }

    fn activate(&self, cx: &mut Cx) {
        Self::toggle(cx);
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::input::Button;
    use rapidr_value::{v_int, v_str};

    use crate::{FormUi, KernelEvent, MemClipboard, MemStore, Mods, TextSystem};

    fn set(id: &str, value: i64) -> KernelEvent {
        KernelEvent::Set { id: id.into(), prop: "checked".into(), value }
    }

    #[test]
    fn a_click_space_and_alt_turn_it_over_before_onclick() {
        let mut s = MemStore::new();
        s.add("ckform", "RFORM", None);
        s.add("ck", "RCHECKBOX", Some("ckform")).set("ck", "caption", v_str("&Bold")).set("ck", "left", v_int(10)).set("ck", "top", v_int(10));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "ckform", false);
        f.paint(&s, &mut ts, 1.0);
        f.mouse_down(&s, &mut ts, 15.0, 20.0, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 15.0, 20.0, Button::Left, Mods::NONE);
        let ev: Vec<_> = f.take_events().into_iter().filter(|e| !matches!(e, KernelEvent::Mouse { .. })).collect();
        assert_eq!(ev, vec![set("ck", 1), KernelEvent::Click("ck".into())]);
        assert_eq!(f.focused(), Some("ck"));
        // (the store says Checked now: Space turns it off)
        s.set("ck", "checked", v_int(1));
        let mut clip = MemClipboard::default();
        f.key_down(&s, &mut ts, 32, " ", Mods::NONE, &mut clip);
        let ev: Vec<_> = f.take_events().into_iter().filter(|e| matches!(e, KernelEvent::Set { .. } | KernelEvent::Click(_))).collect();
        assert_eq!(ev, vec![set("ck", 0), KernelEvent::Click("ck".into())]);
        // Alt+B, its mnemonic.
        let alt = Mods { alt: true, ..Mods::NONE };
        f.key_down(&s, &mut ts, 66, "", alt, &mut clip);
        let ev: Vec<_> = f.take_events().into_iter().filter(|e| matches!(e, KernelEvent::Set { .. } | KernelEvent::Click(_))).collect();
        assert_eq!(ev, vec![set("ck", 0), KernelEvent::Click("ck".into())]);
        // A release off it is no click.
        f.mouse_down(&s, &mut ts, 15.0, 20.0, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 200.0, 200.0, Button::Left, Mods::NONE);
        assert!(f.take_events().iter().all(|e| matches!(e, KernelEvent::Mouse { .. })));
        let list = f.paint(&s, &mut ts, 1.0).dump();
        assert!(list.contains("\"Bold\""), "{list}");
    }
}
