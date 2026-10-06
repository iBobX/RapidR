//! QRADIOBUTTON, Windows-classic (DFCS_BUTTONRADIO): a 12 × 12 round
//! well, its two-tone sunken rim, the dot black; the caption to its right
//! as a check box's. The radio buttons sharing a parent are one group:
//! a click (a press and a release on it), Space, Alt + its letter or a
//! screen reader's click checks it and unchecks the others, then OnClick
//! (the new Checked values go to the store first, through
//! [`KernelEvent::Set`](crate::KernelEvent::Set)). A fluent theme draws a
//! smooth circle: the accent with a dot when checked.

use rapidr_value::objects::a11y::{mnemonic, AccessNode, Action};
use rapidr_value::objects::trackbar::Shape;

use super::check::{caption_right, checked, fluent_mark};
use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::Painter;
use crate::store::{self, Store};

/// The well's diameter.
pub const DIAMETER: i64 = 12;

/// A filled circle (or a pie from `from` to `to` degrees, counter-clockwise
/// from 3 o'clock, y up) of radius `r` around (cx, cy), as a shape on the
/// pixel grid (its outline drawn in its colour too, so its edge pixels are
/// whole). Points are pixel indices: the circle through pixels' centres.
pub fn disc(cx: f64, cy: f64, r: f64, from: f64, to: f64, color: u32) -> Shape {
    oval(cx, cy, r, r, from, to, color)
}

/// [`disc`] for an ellipse of radii `rx`, `ry`.
pub fn oval(cx: f64, cy: f64, rx: f64, ry: f64, from: f64, to: f64, color: u32) -> Shape {
    let full = (to - from).abs() >= 360.0;
    let steps = ((rx.max(ry) * 6.0).ceil() as usize).clamp(12, 96);
    let mut points = Vec::with_capacity(steps + 2);
    if !full {
        points.push((cx, cy));
    }
    for k in 0..=steps {
        let a = (from + (to - from) * k as f64 / steps as f64).to_radians();
        points.push((cx + rx * a.cos(), cy - ry * a.sin()));
    }
    if full {
        points.pop();
    }
    Shape { points, fill: Some(color), stroke: Some(color) }
}

/// The classic well, 12 × 12, as Windows draws it at 1× (DFCS_BUTTONRADIO;
/// RapidQ's capture, tests/visual/rapidq/checks_radios): the rim's outer
/// ring the shadow above (`o`) and white below (`h`), its inner ring the
/// dark shadow above (`i`) and COLOR_3DLIGHT below (`l`), the well (`w`).
const WELL: [&str; 12] = [
    "....oooo....",
    "..ooiiiioo..",
    ".oiiwwwwiih.",
    ".oiwwwwwwlh.",
    "oiwwwwwwwwlh",
    "oiwwwwwwwwlh",
    "oiwwwwwwwwlh",
    "oiwwwwwwwwlh",
    ".oiwwwwwwlh.",
    ".ollwwwwllh.",
    "..hhllllhh..",
    "....hhhh....",
];

/// The dot of a checked one, 4 × 4 at (4, 4) in the well.
const DOT: [&str; 4] = [".xx.", "xxxx", "xxxx", ".xx."];

/// The round well at (x, y) (`well` inside), and its dot in `dot`: pixel
/// for pixel at 1×; at a high-DPI screen's resolution the same rings as
/// smooth shapes — the light from the top left, the rings split from the
/// bottom left to the top right.
pub fn round_well(p: &mut Painter, x: i64, y: i64, well: u32, dot: Option<u32>) {
    let t = p.theme();
    if p.one_to_one() {
        p.pixels(x, y, &WELL, &[('o', t.shadow), ('i', t.dark_shadow), ('l', t.light3d), ('h', t.light), ('w', well)]);
        if let Some(c) = dot {
            p.pixels(x + 4, y + 4, &DOT, &[('x', c)]);
        }
        return;
    }
    let c = (x as f64 + 6.0, y as f64 + 6.0);
    p.sector(c, 6.0, 45.0, 225.0, t.shadow);
    p.sector(c, 6.0, 225.0, 405.0, t.light);
    p.sector(c, 5.0, 45.0, 225.0, t.dark_shadow);
    p.sector(c, 5.0, 225.0, 405.0, t.light3d);
    p.sector(c, 4.0, 0.0, 360.0, well);
    if let Some(color) = dot {
        p.sector(c, 2.0, 0.0, 360.0, color);
    }
}

pub struct RadioButton;

impl RadioButton {
    /// Checks it; the others of its parent's come off; OnClick.
    fn choose(cx: &mut Cx) {
        let parent = store::string(cx.store, cx.id, "parent");
        let others: Vec<String> = cx
            .store
            .children(&parent)
            .into_iter()
            .filter(|(id, t)| t.eq_ignore_ascii_case("RRADIOBUTTON") && !id.eq_ignore_ascii_case(cx.id) && checked(cx.store, id))
            .map(|(id, _)| id)
            .collect();
        let me = cx.id.to_string();
        if !checked(cx.store, &me) {
            cx.set(&me, "checked", 1);
        }
        for o in others {
            cx.set(&o, "checked", 0);
        }
        cx.click();
    }
}

impl ComponentKind for RadioButton {
    fn name(&self) -> &'static str {
        "RRADIOBUTTON"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let h = cx.height();
        let s = cx.state;
        let t = p.theme();
        let y = (h - DIAMETER) / 2;
        if t.fluent() {
            fluent_mark(p, (0, y, DIAMETER), true, checked(cx.store, cx.id), s);
        } else {
            let dot = checked(cx.store, cx.id).then_some(if s.enabled { t.text } else { t.shadow });
            round_well(p, 1, y, if s.pressed || !s.enabled { t.face } else { t.window }, dot);
        }
        caption_right(cx, p, DIAMETER + 1);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        match m.kind {
            MouseKind::Down => MouseOut { press: true, focus: None },
            MouseKind::Up if cx.state.held && m.inside => {
                Self::choose(cx);
                MouseOut::default()
            }
            _ => MouseOut::default(),
        }
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if k.vk == 32 && !k.mods.alt && !k.mods.command {
            Self::choose(cx);
            return true;
        }
        false
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        if action == Action::Click && cx.state.enabled {
            Self::choose(cx);
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
        Self::choose(cx);
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::input::Button;
    use rapidr_value::{v_int, v_str};

    use crate::{FormUi, KernelEvent, MemStore, Mods, TextSystem};

    fn set(id: &str, value: i64) -> KernelEvent {
        KernelEvent::Set { id: id.into(), prop: "checked".into(), value }
    }

    #[test]
    fn a_group_is_its_parents_radio_buttons() {
        let mut s = MemStore::new();
        s.add("rbform", "RFORM", None);
        s.add("rbpanel", "RPANEL", Some("rbform")).set("rbpanel", "left", v_int(0)).set("rbpanel", "top", v_int(100));
        for (i, id) in ["rb1", "rb2"].iter().enumerate() {
            s.add(id, "RRADIOBUTTON", Some("rbform")).set(id, "caption", v_str(id)).set(id, "top", v_int(i as i64 * 30));
        }
        s.add("rb3", "RRADIOBUTTON", Some("rbpanel")).set("rb3", "checked", v_int(1));
        s.set("rb1", "checked", v_int(1));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "rbform", false);
        f.paint(&s, &mut ts, 1.0);
        // rb2 clicked: it's checked, rb1 (same parent) comes off, rb3 (another) stays.
        f.mouse_down(&s, &mut ts, 5.0, 40.0, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 5.0, 40.0, Button::Left, Mods::NONE);
        let ev: Vec<_> = f.take_events().into_iter().filter(|e| !matches!(e, KernelEvent::Mouse { .. })).collect();
        assert_eq!(ev, vec![set("rb2", 1), set("rb1", 0), KernelEvent::Click("rb2".into())]);
        let list = f.paint(&s, &mut ts, 2.0).dump();
        assert!(list.contains("polygon"), "{list}");
    }
}
