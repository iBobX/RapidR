//! QBUTTON, a Windows-classic push button (DrawFrameControl
//! DFCS_BUTTONPUSH): raised; framed in black when it has the focus or is
//! the form's Default button while no other button has it; pushed, a black
//! frame and a shadow line, its caption a pixel down and right. A click is
//! a press and a release on it, Space or Enter while it has the focus,
//! Enter anywhere on the form for its Default button, Escape for its
//! Cancel one, Alt + its caption's `&` letter.
//!
//! Kind (bkOK …), Default, Cancel and ModalResult are data
//! ([`ButtonData`]): the kernel only emits `Click`; runtime-core's OnClick
//! dispatch applies ModalResult / bkClose to the form, as today.

use rapidr_value::objects::a11y::{mnemonic, node_id, AccessNode, Action, Role};
use rapidr_value::objects::ops::Place;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::{caption, Painter, DARK, FACE, GRAY_TEXT, LIGHT, SHADOW};
use crate::store::{self, Store};
use crate::text::bgr_to_rgb;

/// A button under the mouse (additive: classic Windows has no hover).
const HOT_FACE: u32 = 0xE5F1FB;

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
        let data = ButtonData::of(cx.store, cx.id);
        p.fill((0, 0, w, h), if s.hover && !s.pressed && s.enabled { HOT_FACE } else { FACE });
        let mut r = (0, 0, w, h);
        if s.focused || s.pressed || s.default_frame {
            p.edge(r, &[0x000000], &[0x000000]);
            r = (1, 1, w - 2, h - 2);
        }
        if s.pressed {
            p.edge(r, &[SHADOW], &[SHADOW]);
        } else {
            p.edge(r, &[LIGHT], &[DARK, SHADOW]);
        }
        let shift = i64::from(s.pressed);
        let color = if s.enabled { bgr_to_rgb(cx.font.color) } else { GRAY_TEXT };
        caption(p, (shift, shift, w, h), &data.caption, &cx.font, color, Place::Center);
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
        let data = ButtonData::of(cx.store, cx.id);
        let mut n = AccessNode::new(node_id(cx.id), Role::Button);
        let (shown, mark) = mnemonic(&data.caption);
        n.name = shown;
        n.shortcut = mark.map(|m| m.1);
        n.actions = vec![Action::Click, Action::Focus];
        n.bounds = cx.rect;
        n
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
