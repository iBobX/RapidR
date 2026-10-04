//! QCHECKBOX, Windows-classic (DrawFrameControl DFCS_BUTTONCHECK): a
//! 13 × 13 box, sunken, white (the face while pressed or disabled), the
//! check mark black; the caption to its right, centred down, its `&`
//! letter underlined; the focus rectangle around the caption. A click (a
//! press and a release on it), Space, Alt + its letter or a screen
//! reader's click turns Checked over, then OnClick — Checked goes to the
//! store through [`KernelEvent::Set`](crate::KernelEvent::Set), so the
//! handler reads the new value.

use rapidr_value::objects::a11y::{mnemonic, AccessNode, Action};
use rapidr_value::objects::ops::Place;
use rapidr_value::objects::text::text_size;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::{caption, Painter, DARK, FACE, GRAY_TEXT, LIGHT, SHADOW};
use crate::store::{self, Store};
use crate::text::bgr_to_rgb;

/// The box's side.
pub const BOX: i64 = 13;
/// From the box to the caption.
pub const GAP: i64 = 4;

/// Windows' check mark: 7 columns of 3 pixels, its top left at (x, y).
pub fn check_mark(p: &mut Painter, x: i64, y: i64, color: u32) {
    for (c, top) in [2, 3, 4, 3, 2, 1, 0].into_iter().enumerate() {
        p.fill((x + c as i64, y + top, 1, 3), color);
    }
}

/// A sunken 13 × 13 box (a check box's) at (x, y), `well` inside.
pub fn sunken_box(p: &mut Painter, x: i64, y: i64, well: u32) {
    p.fill((x + 2, y + 2, BOX - 4, BOX - 4), well);
    p.edge((x, y, BOX, BOX), &[SHADOW, DARK], &[LIGHT, FACE]);
}

/// A check box's or radio button's caption, right of its mark (and the
/// focus rectangle around it).
pub fn caption_right(cx: &Cx, p: &mut Painter, mark: i64) {
    let (w, h) = (cx.width(), cx.height());
    let text = store::string(cx.store, cx.id, "caption");
    let color = if cx.state.enabled { bgr_to_rgb(cx.font.color) } else { GRAY_TEXT };
    let x = mark + GAP;
    if !cx.state.enabled {
        caption(p, (x + 1, 1, w - x, h), &text, &cx.font, LIGHT, Place::Left);
    }
    caption(p, (x, 0, w - x, h), &text, &cx.font, color, Place::Left);
    if cx.state.focused {
        let (tw, th) = text_size(&mnemonic(&text).0, &cx.font);
        let (fw, fh) = ((tw + 2).min(w - x + 1), th + 2);
        p.focus((x - 1, (h - th) / 2 - 1, fw.max(1), fh));
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
        let y = (h - BOX) / 2;
        sunken_box(p, 0, y, if s.pressed || !s.enabled { FACE } else { LIGHT });
        if checked(cx.store, cx.id) {
            check_mark(p, 3, y + 3, if s.enabled { 0x000000 } else { GRAY_TEXT });
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
