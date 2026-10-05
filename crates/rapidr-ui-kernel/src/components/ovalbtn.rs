//! QOVALBTN: "like QCOOLBTN, except that the button looks round/oval" —
//! an ellipse filling its rectangle in Color (the face when 0), its rim lit
//! from the top left (ColorHighlight above, ColorShadow below; swapped
//! while pressed or Down), the caption centred (a pixel down and right
//! when sunk). A toggle button as a QCOOLBTN is: GroupIndex, Down,
//! AllowAllUp through `rapidr_value::toggle_group`, Down stored before
//! OnClick; it never takes the focus.

use rapidr_value::objects::a11y::{mnemonic, AccessNode, Action};
use rapidr_value::objects::ops::Place;

use super::coolbtn::{describe_toggle, down, press, toggle_mouse};
use super::radio::oval;
use super::{ComponentKind, Cx, MouseIn, MouseOut};
use crate::a11y::AccessValue;
use crate::paint::{caption, ink_of, Painter};
use crate::store::{self, Store};
use crate::text::bgr_to_rgb;

/// A colour property (0xBBGGRR) as RGB, `default` when unset or 0.
fn color(store: &dyn Store, id: &str, prop: &str, default: u32) -> u32 {
    match store::int(store, id, prop, 0) {
        0 => default,
        c => bgr_to_rgb(c),
    }
}

pub struct OvalBtn;

impl ComponentKind for OvalBtn {
    fn name(&self) -> &'static str {
        "ROVALBTN"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        if w < 2 || h < 2 {
            return;
        }
        let s = cx.state;
        let t = p.theme();
        let sunk = s.pressed || down(cx.store, cx.id);
        // (a fluent theme: its control face, a thin rim, the accent when down)
        let (face0, lit0, shade0) = if t.fluent() {
            let face = if down(cx.store, cx.id) { t.accent } else if s.pressed { t.control_pressed } else if s.hover && s.enabled { t.control_hot } else { t.control };
            (face, t.border, t.border)
        } else {
            (t.face, t.light, t.shadow)
        };
        let face = color(cx.store, cx.id, "color", face0);
        let (mut lit, mut shade) = (color(cx.store, cx.id, "colorhighlight", lit0), color(cx.store, cx.id, "colorshadow", shade0));
        if sunk {
            std::mem::swap(&mut lit, &mut shade);
        }
        // (pixel indices: the ellipse through the edge pixels' centres)
        let (ox, oy, rx, ry) = ((w - 1) as f64 / 2.0, (h - 1) as f64 / 2.0, (w - 1) as f64 / 2.0, (h - 1) as f64 / 2.0);
        p.shape(oval(ox, oy, rx, ry, 45.0, 225.0, lit));
        p.shape(oval(ox, oy, rx, ry, 225.0, 405.0, shade));
        p.shape(oval(ox, oy, (rx - 2.0).max(0.0), (ry - 2.0).max(0.0), 0.0, 360.0, face));
        let d = i64::from(sunk && !t.fluent());
        let text = store::string(cx.store, cx.id, "caption");
        // (its BMP glyph, the caption beside it: image.rs)
        let ((cx0, cy0, cw, ch), place) = super::image::paint_glyph(cx, p, (d, d, w, h), s.enabled, s.pressed, down(cx.store, cx.id)).unwrap_or(((d, d, w, h), Place::Center));
        if s.enabled {
            let ink = if t.fluent() && face == t.accent { t.accent_text } else { ink_of(cx, face) };
            caption(p, (cx0, cy0, cw, ch), &text, &cx.font, ink, place);
        } else {
            if !t.fluent() {
                caption(p, (cx0 + 1, cy0 + 1, cw, ch), &text, &cx.font, t.light, place);
            }
            caption(p, (cx0, cy0, cw, ch), &text, &cx.font, t.gray_text, place);
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        toggle_mouse(cx, m)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        describe_toggle(cx)
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        if action == Action::Click && cx.state.enabled {
            press(cx);
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
        press(cx);
    }
}
