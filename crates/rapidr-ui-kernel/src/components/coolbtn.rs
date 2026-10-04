//! QCOOLBTN, a Windows-classic speed button: raised (Flat: no frame until
//! the mouse is over it, then a thin raised one), sunken while pressed
//! and while Down, its caption a pixel down and right then. It never takes
//! the focus (a graphic control, as Delphi's TSpeedButton). Buttons
//! sharing a parent and a non-zero GroupIndex are a group
//! (`rapidr_value::toggle_group`, as FLTK's and the web's): a click puts
//! one down and the others up; the one down comes up only with AllowAllUp;
//! GroupIndex 0 never stays down. The new Down values go to the store
//! ([`KernelEvent::Set`](crate::KernelEvent::Set)) before OnClick.
//!
//! Its BMP glyph (`BMP` / `BMPHandle`, `NumBMPs`, `Layout`, `Spacing`) is
//! drawn beside the caption by `image::paint_glyph` (the surfaces lane's).

use rapidr_value::objects::a11y::{mnemonic, node_id, AccessNode, Action, Role};
use rapidr_value::objects::ops::Place;
use rapidr_value::toggle_group::{self, Member};

use super::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::paint::{caption, Painter, DARK, FACE, GRAY_TEXT, LIGHT, SHADOW};
use crate::store::{self, Store};
use crate::text::bgr_to_rgb;

/// Whether a component is a toggle button (a group's member).
pub fn is_toggle(type_name: &str) -> bool {
    type_name.eq_ignore_ascii_case("RCOOLBTN") || type_name.eq_ignore_ascii_case("ROVALBTN")
}

/// Down, as the store keeps it.
pub fn down(store: &dyn Store, id: &str) -> bool {
    store::flag(store, id, "down", false)
}

/// The toggle buttons sharing `id`'s parent.
pub fn members(store: &dyn Store, id: &str) -> Vec<Member> {
    let parent = store::string(store, id, "parent");
    store
        .children(&parent)
        .into_iter()
        .filter(|(_, t)| is_toggle(t))
        .map(|(n, _)| Member { group: store::int(store, &n, "groupindex", 0), down: down(store, &n), name: n.to_lowercase() })
        .collect()
}

/// The user clicked toggle button `cx.id`: its group's new Down values
/// (stored), then OnClick.
pub fn press(cx: &mut Cx) {
    let allow_all_up = store::flag(cx.store, cx.id, "allowallup", false);
    let changes = toggle_group::press(cx.id, allow_all_up, &members(cx.store, cx.id));
    for (name, down) in changes {
        cx.set(&name, "down", if down { -1 } else { 0 });
    }
    cx.click();
}

/// A toggle button for screen readers: a button, pressed or not (checked).
pub fn describe_toggle(cx: &Cx) -> AccessNode {
    let mut n = AccessNode::new(node_id(cx.id), Role::Button);
    let (shown, mark) = mnemonic(&store::string(cx.store, cx.id, "caption"));
    n.name = shown;
    n.shortcut = mark.map(|m| m.1);
    if store::int(cx.store, cx.id, "groupindex", 0) != 0 {
        n.states.checked = Some(down(cx.store, cx.id));
    }
    n.actions = vec![Action::Click];
    n.bounds = cx.rect;
    n
}

/// A press and a release on a toggle button.
pub fn toggle_mouse(cx: &mut Cx, m: &MouseIn) -> MouseOut {
    match m.kind {
        MouseKind::Down => MouseOut { press: true, focus: Some(false) },
        MouseKind::Up if cx.state.held && m.inside => {
            press(cx);
            MouseOut::default()
        }
        _ => MouseOut::default(),
    }
}

pub struct CoolBtn;

impl ComponentKind for CoolBtn {
    fn name(&self) -> &'static str {
        "RCOOLBTN"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let s = cx.state;
        let is_down = down(cx.store, cx.id);
        let sunk = s.pressed || is_down;
        let flat = store::flag(cx.store, cx.id, "flat", false);
        // (down and not held: Windows' dithered face, lighter)
        p.fill((0, 0, w, h), if is_down && !s.pressed { 0xF8F8F8 } else { FACE });
        let r = (0, 0, w, h);
        match (flat, sunk) {
            (false, false) => p.edge(r, &[LIGHT], &[DARK, SHADOW]),
            (false, true) => p.edge(r, &[SHADOW, DARK], &[LIGHT, FACE]),
            (true, true) => p.edge(r, &[SHADOW], &[LIGHT]),
            (true, false) if s.hover && s.enabled => p.edge(r, &[LIGHT], &[SHADOW]),
            (true, false) => {}
        }
        let d = i64::from(sunk);
        let text = store::string(cx.store, cx.id, "caption");
        // (its BMP glyph, the caption beside it: image.rs)
        let ((cx0, cy0, cw, ch), place) = super::image::paint_glyph(cx, p, (d, d, w, h), s.enabled, s.pressed, down(cx.store, cx.id)).unwrap_or(((d, d, w, h), Place::Center));
        if s.enabled {
            caption(p, (cx0, cy0, cw, ch), &text, &cx.font, bgr_to_rgb(cx.font.color), place);
        } else {
            caption(p, (cx0 + 1, cy0 + 1, cw, ch), &text, &cx.font, LIGHT, place);
            caption(p, (cx0, cy0, cw, ch), &text, &cx.font, GRAY_TEXT, place);
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

#[cfg(test)]
mod tests {
    use rapidr_value::input::Button;
    use rapidr_value::{v_int, v_str};

    use crate::{FormUi, KernelEvent, MemStore, Mods, TextSystem};

    fn set(id: &str, value: i64) -> KernelEvent {
        KernelEvent::Set { id: id.into(), prop: "down".into(), value }
    }

    /// coolbtn_group.bas's buttons A (down), B, C in group 1, D in none, E
    /// in group 2 with AllowAllUp.
    fn store() -> MemStore {
        let mut s = MemStore::new();
        s.add("cbform", "RFORM", None);
        for (i, (id, group)) in [("cba", 1), ("cbb", 1), ("cbc", 1), ("cbd", 0), ("cbe", 2)].into_iter().enumerate() {
            s.add(id, "RCOOLBTN", Some("cbform")).set(id, "caption", v_str(id)).set(id, "left", v_int(10 + 80 * i as i64)).set(id, "top", v_int(10)).set(id, "groupindex", v_int(group));
        }
        s.set("cba", "down", v_int(-1)).set("cbe", "allowallup", v_int(-1));
        s
    }

    fn click(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, x: f64) -> Vec<KernelEvent> {
        f.mouse_down(s, ts, x, 20.0, Button::Left, Mods::NONE);
        f.mouse_up(s, ts, x, 20.0, Button::Left, Mods::NONE);
        f.take_events().into_iter().filter(|e| !matches!(e, KernelEvent::Mouse { .. })).collect()
    }

    #[test]
    fn a_group_keeps_one_down() {
        let mut s = store();
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "cbform", false);
        f.paint(&s, &mut ts, 1.0);
        // B: down, A up; then OnClick. Nothing took the focus.
        assert_eq!(click(&mut f, &s, &mut ts, 95.0), vec![set("cbb", -1), set("cba", 0), KernelEvent::Click("cbb".into())]);
        assert_eq!(f.focused(), None);
        s.set("cbb", "down", v_int(-1)).set("cba", "down", v_int(0));
        // B again (no AllowAllUp): stays; D (group 0): only OnClick.
        assert_eq!(click(&mut f, &s, &mut ts, 95.0), vec![KernelEvent::Click("cbb".into())]);
        assert_eq!(click(&mut f, &s, &mut ts, 255.0), vec![KernelEvent::Click("cbd".into())]);
        // E (AllowAllUp): down, then up.
        assert_eq!(click(&mut f, &s, &mut ts, 335.0), vec![set("cbe", -1), KernelEvent::Click("cbe".into())]);
        s.set("cbe", "down", v_int(-1));
        assert_eq!(click(&mut f, &s, &mut ts, 335.0), vec![set("cbe", 0), KernelEvent::Click("cbe".into())]);
        // Drawn: B sunken (its edge starts grey).
        let list = f.paint(&s, &mut ts, 1.0).dump();
        assert!(list.contains("edge 0,0 25x25 #808080/#404040 #ffffff/#f0f0f0 @90,10"), "{list}");
    }
}
