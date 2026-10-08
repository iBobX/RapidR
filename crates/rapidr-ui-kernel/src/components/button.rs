//! QBUTTON, a Windows-classic push button (DrawFrameControl
//! DFCS_BUTTONPUSH): raised; framed in black when it has the focus or is
//! the form's Default button while no other button has it; pushed, a black
//! frame and a shadow line, its caption a pixel down and right. A click is
//! a press and a release on it, Space or Enter while it has the focus,
//! Enter anywhere on the form for its Default button, Escape for its
//! Cancel one, Alt + its caption's `&` letter.
//!
//! A fluent theme draws it flat: a rounded face with a thin border (the
//! Default button in the accent), a focus ring round its edge.
//!
//! Kind (bkOK …), Default, Cancel and ModalResult are data
//! ([`ButtonData`]): the kernel only emits `Click`; runtime-core's OnClick
//! dispatch applies ModalResult / bkClose to the form, as today.

use rapidr_value::objects::a11y::{mnemonic, AccessNode, Action};
use rapidr_value::objects::ops::Place;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::{caption, ink_of, Painter};
use crate::store::{self, Store};

/// What a QBUTTON's properties mean.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonData {
    /// Its caption (Kind's when it has none).
    pub caption: String,
    /// bkCustom 0 … bkAll 10.
    pub kind: i64,
    /// Enter clicks it.
    pub default: bool,
    /// Escape clicks it.
    pub cancel: bool,
    /// What a click sets its form's ModalResult to (Kind's when 0).
    pub modal_result: i64,
}

impl ButtonData {
    pub fn of(store: &dyn Store, id: &str) -> ButtonData {
        let kind = store::int(store, id, "kind", 0);
        let by_kind = rapidr_value::events::button_kind(kind);
        let mut caption = store::string(store, id, "caption");
        if caption.is_empty() {
            caption = by_kind.map_or(String::new(), |k| k.0.to_string());
        }
        let mut modal_result = store::int(store, id, "modalresult", 0);
        if modal_result == 0 {
            modal_result = by_kind.map_or(0, |k| k.1);
        }
        ButtonData { caption, kind, default: store::flag(store, id, "default", false), cancel: store::flag(store, id, "cancel", false), modal_result }
    }

    /// A click closes its form (a ModalResult, or bkClose).
    pub fn closes_form(&self) -> bool {
        self.modal_result != 0 || self.kind == 6
    }
}

pub struct PushButton;

impl ComponentKind for PushButton {
    fn name(&self) -> &'static str {
        "RBUTTON"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let s = cx.state;
        let t = p.theme();
        let data = ButtonData::of(cx.store, cx.id);
        if t.fluent() {
            // (the Default button in the accent; under the mouse, pressed,
            // disabled: the theme's fills)
            let accent = data.default && s.enabled;
            let fill = match (accent, s.enabled, s.pressed, s.hover) {
                (_, false, _, _) => t.control_disabled,
                (true, _, true, _) => t.accent_pressed,
                (true, _, _, true) => t.accent_hot,
                (true, ..) => t.accent,
                (false, _, true, _) => t.control_pressed,
                (false, _, _, true) => t.control_hot,
                _ => t.control,
            };
            let border = if accent { fill } else if s.hover && s.enabled { t.border_hot } else { t.border };
            p.round((0, 0, w, h), t.radius, Some(fill), Some(border), 1.0);
            let color = if accent { t.accent_text } else { ink_of(cx, fill) };
            // (its glyph, BMP / BMPHandle, beside the caption: image.rs)
            let (at, place) = super::image::paint_glyph(cx, p, (0, 0, w, h), s.enabled, s.pressed, false).unwrap_or(((0, 0, w, h), Place::Center));
            caption(p, at, &data.caption, &cx.font, color, place);
            if s.focused {
                p.focus((0, 0, w, h));
            }
            return;
        }
        p.fill((0, 0, w, h), if s.hover && !s.pressed && s.enabled { t.hot } else { t.face });
        let mut r = (0, 0, w, h);
        if s.focused || s.pressed || s.default_frame {
            p.frame(r, t.frame);
            r = (1, 1, w - 2, h - 2);
        }
        if s.pressed {
            p.frame(r, t.shadow);
        } else {
            p.button_edge(r);
        }
        let shift = i64::from(s.pressed);
        let mut color = ink_of(cx, t.face);
        // (its glyph, BMP / BMPHandle, beside the caption, the two centred
        // together as Delphi's TBitBtn lays them out: image.rs)
        let ((x, y, cw, ch), place) = super::image::paint_glyph(cx, p, (shift, shift, w, h), s.enabled, s.pressed, false).unwrap_or(((shift, shift, w, h), Place::Center));
        if !s.enabled {
            // (embossed, as Windows' DrawState: white a pixel down and right,
            // the shadow over it)
            caption(p, (x + 1, y + 1, cw, ch), &data.caption, &cx.font, t.light, place);
            color = t.shadow;
        }
        caption(p, (x, y, cw, ch), &data.caption, &cx.font, color, place);
        if s.focused {
            p.focus((4, 4, w - 8, h - 8));
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        match m.kind {
            MouseKind::Down => MouseOut { press: true, focus: None },
            // (a click: let go on the button it was pressed on)
            MouseKind::Up if cx.state.held && m.inside => {
                cx.click();
                MouseOut::default()
            }
            _ => MouseOut::default(),
        }
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if matches!(k.vk, 13 | 32) && !k.mods.alt && !k.mods.command {
            cx.click();
            return true;
        }
        false
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        if action == Action::Click && cx.state.enabled {
            cx.click();
            return true;
        }
        false
    }

    fn mnemonic(&self, store: &dyn Store, id: &str) -> Option<char> {
        mnemonic(&ButtonData::of(store, id).caption).1.map(|m| m.1)
    }

    fn mnemonic_clicks(&self) -> bool {
        true
    }
}
