//! QUPDOWN (RapidR's RUPDOWN): Windows' up-down control — two arrow
//! buttons stacked (side by side with Orientation = udHorizontal, 1), each
//! raised with a black arrow and pushed while held. A click on one, or the
//! arrow keys while it has the focus, steps Position by Increment (1)
//! between Min and Max (round with Wrap); the new Position goes to the
//! store ([`KernelEvent::Set`](crate::KernelEvent::Set)) before OnClick.

use rapidr_value::objects::a11y::{node_id, AccessNode, Action, Numeric, Role};
use rapidr_value::objects::ops::{Op, Rect};

use super::progress::range;
use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::{Painter, DARK, FACE, GRAY_TEXT, LIGHT, SHADOW};
use crate::store::{self, Store};

pub struct UpDown;

fn horizontal(store: &dyn Store, id: &str) -> bool {
    store::int(store, id, "orientation", 0) == 1
}

/// Its two buttons: up (or right), down (or left).
fn halves(store: &dyn Store, id: &str, w: i64, h: i64) -> [Rect; 2] {
    if horizontal(store, id) {
        let half = w / 2;
        [(half, 0, w - half, h), (0, 0, half, h)]
    } else {
        let half = h / 2;
        [(0, 0, w, half), (0, half, w, h - half)]
    }
}

/// One step up (`up`) or down: whether Position changed (stored, OnClick).
fn step(cx: &mut Cx, up: bool) -> bool {
    let (min, max, pos) = range(cx.store, cx.id);
    let inc = store::int(cx.store, cx.id, "increment", 1).max(1);
    let wrap = store::flag(cx.store, cx.id, "wrap", false);
    let mut next = if up { pos + inc } else { pos - inc };
    if next > max {
        next = if wrap { min } else { max };
    } else if next < min {
        next = if wrap { max } else { min };
    }
    let id = cx.id.to_string();
    if next != pos {
        cx.set(&id, "position", next);
    }
    cx.click();
    next != pos
}

impl ComponentKind for UpDown {
    fn name(&self) -> &'static str {
        "RUPDOWN"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let across = horizontal(cx.store, cx.id);
        let color = if cx.state.enabled { 0x000000 } else { GRAY_TEXT };
        for (k, r) in halves(cx.store, cx.id, w, h).into_iter().enumerate() {
            let pushed = cx.state.held && cx.ui.part == Some(k) && cx.state.hover;
            p.fill(r, FACE);
            if pushed {
                p.edge(r, &[SHADOW], &[SHADOW]);
            } else {
                p.edge(r, &[FACE, LIGHT], &[DARK, SHADOW]);
            }
            let d = if pushed { 1.0 } else { 0.0 };
            let (cx0, cy0) = (r.0 as f64 + r.2 as f64 / 2.0 + d, r.1 as f64 + r.3 as f64 / 2.0 + d);
            let s = ((r.2.min(r.3) as f64) / 4.0).clamp(1.5, 4.0);
            let points = match (across, k) {
                (false, 0) => [(cx0 - s, cy0 + s / 2.0), (cx0 + s, cy0 + s / 2.0), (cx0, cy0 - s / 2.0)],
                (false, _) => [(cx0 - s, cy0 - s / 2.0), (cx0 + s, cy0 - s / 2.0), (cx0, cy0 + s / 2.0)],
                (true, 0) => [(cx0 - s / 2.0, cy0 - s), (cx0 - s / 2.0, cy0 + s), (cx0 + s / 2.0, cy0)],
                (true, _) => [(cx0 + s / 2.0, cy0 - s), (cx0 + s / 2.0, cy0 + s), (cx0 - s / 2.0, cy0)],
            };
            p.op(Op::Arrow { points, color });
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let (w, h) = (cx.width(), cx.height());
        let at = |x: f64, y: f64| halves(cx.store, cx.id, w, h).iter().position(|r| x >= r.0 as f64 && y >= r.1 as f64 && x < (r.0 + r.2) as f64 && y < (r.1 + r.3) as f64);
        match m.kind {
            MouseKind::Down => {
                cx.ui.part = at(m.x, m.y);
                MouseOut { press: true, focus: None }
            }
            MouseKind::Up => {
                let part = cx.ui.part.take();
                if cx.state.held && m.inside && part.is_some() && at(m.x, m.y) == part {
                    step(cx, part == Some(0));
                }
                MouseOut::default()
            }
            _ => MouseOut::default(),
        }
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if k.mods.alt || k.mods.command {
            return false;
        }
        match k.vk {
            38 | 39 => step(cx, true),
            37 | 40 => step(cx, false),
            _ => return false,
        };
        true
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Slider);
        let (min, max, pos) = range(cx.store, cx.id);
        n.numeric = Some(Numeric { value: pos as f64, min: min as f64, max: max as f64, step: store::int(cx.store, cx.id, "increment", 1).max(1) as f64, jump: 10.0 });
        n.actions = vec![Action::Increment, Action::Decrement, Action::Focus];
        n.bounds = cx.rect;
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        match action {
            Action::Increment => step(cx, true),
            Action::Decrement => step(cx, false),
            _ => return false,
        };
        true
    }
}

#[cfg(test)]
mod tests {
    use rapidr_value::input::Button;
    use rapidr_value::v_int;

    use crate::{FormUi, KernelEvent, MemClipboard, MemStore, Mods, TextSystem};

    #[test]
    fn its_arrows_step_position() {
        let mut s = MemStore::new();
        s.add("udform", "RFORM", None);
        s.add("ud", "RUPDOWN", Some("udform")).set("ud", "width", v_int(17)).set("ud", "height", v_int(24)).set("ud", "position", v_int(99));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "udform", false);
        f.paint(&s, &mut ts, 1.0);
        // The upper half: 99 → 100; the top is Max, then nothing changes.
        f.mouse_down(&s, &mut ts, 8.0, 4.0, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 8.0, 4.0, Button::Left, Mods::NONE);
        let set = |v| KernelEvent::Set { id: "ud".into(), prop: "position".into(), value: v };
        let ev: Vec<_> = f.take_events().into_iter().filter(|e| !matches!(e, KernelEvent::Mouse { .. })).collect();
        assert_eq!(ev, vec![set(100), KernelEvent::Click("ud".into())]);
        s.set("ud", "position", v_int(100));
        let mut clip = MemClipboard::default();
        f.key_down(&s, &mut ts, 38, "", Mods::NONE, &mut clip);
        assert!(!f.take_events().iter().any(|e| matches!(e, KernelEvent::Set { .. })));
        f.key_down(&s, &mut ts, 40, "", Mods::NONE, &mut clip);
        assert!(f.take_events().contains(&set(99)));
    }
}
