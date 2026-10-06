//! QSCROLLBAR (RapidR's RSCROLLBAR, Delphi's TScrollBar): a Windows scroll
//! bar on its own — Kind 0 (sbHorizontal) or 1 (sbVertical), Position
//! between Min and Max (less a PageSize but one), SmallChange for an arrow
//! or an arrow key, LargeChange for a click in the track or Page Up / Page
//! Down, Home / End to the ends; the thumb drags. Drawn by the shared
//! scroll-bar model (`rapidr_value::scrollbars`, its [`Alone`] bar) through
//! [`crate::paint::bar_ops`], so it looks like every other scroll bar the
//! kernel draws, in every theme; disabled, its arrows grey and no thumb
//! (Windows'). A held arrow or page repeats ([`crate::tick`]'s delay and
//! rate). The new Position goes to the store
//! ([`KernelEvent::Set`](crate::KernelEvent::Set)) before OnChange.

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::objects::a11y::{AccessNode, Action};
use rapidr_value::scrollbars::{Alone, Part, Scroller, Values};

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::Clipboard;
use crate::paint::Painter;
use crate::store::{self, Store};
use crate::tick::{now, REPEAT, REPEAT_DELAY};

pub struct ScrollBar;

/// A bar held down with the mouse.
#[derive(Clone, Copy, Debug)]
struct Held {
    part: Part,
    /// Where the mouse is now (the component's pixels).
    x: f64,
    y: f64,
    /// A thumb drag: where along the bar it was pressed, the Position
    /// then, and the Position it shows now.
    from: i64,
    start: i64,
    at: i64,
}

thread_local! {
    /// The bars held down now, by id.
    static HELD: RefCell<HashMap<String, Held>> = RefCell::new(HashMap::new());
}

fn held(id: &str) -> Option<Held> {
    HELD.with(|h| h.borrow().get(id).copied())
}

fn set_held(id: &str, h: Option<Held>) {
    HELD.with(|m| {
        let mut m = m.borrow_mut();
        match h {
            Some(h) => m.insert(id.to_string(), h),
            None => m.remove(id),
        };
    });
}

fn vertical(store: &dyn Store, id: &str) -> bool {
    store::int(store, id, "kind", 0) == 1
}

/// Min, Max, PageSize (RapidQ's default 1) and Position, as stored.
fn values(store: &dyn Store, id: &str) -> Values {
    let min = store::int(store, id, "min", 0);
    let max = store::int(store, id, "max", 100).max(min);
    Values { min, max, page: store::int(store, id, "pagesize", 1).max(0), position: store::int(store, id, "position", 0) }
}

/// Its length along the bar and the mouse's place along it.
fn along(cx: &Cx, x: f64, y: f64) -> (i64, i64) {
    if vertical(cx.store, cx.id) {
        (cx.height(), y.floor() as i64)
    } else {
        (cx.width(), x.floor() as i64)
    }
}

/// Position moved to `to` (kept in range): stored, then OnChange — when it
/// changed.
fn move_to(cx: &mut Cx, to: i64) -> bool {
    let v = values(cx.store, cx.id);
    let (from, to) = (v.clamp(v.position), v.clamp(to));
    if to == from {
        return false;
    }
    let id = cx.id.to_string();
    cx.set(&id, "position", to);
    cx.change();
    true
}

/// What an arrow or a page part does once.
fn step(cx: &mut Cx, part: Part) -> bool {
    let v = values(cx.store, cx.id);
    let small = store::int(cx.store, cx.id, "smallchange", 1);
    let large = store::int(cx.store, cx.id, "largechange", 1);
    let pos = v.clamp(v.position);
    match part {
        Part::Back => move_to(cx, pos - small),
        Part::Forward => move_to(cx, pos + small),
        Part::PageBack => move_to(cx, pos - large),
        Part::PageForward => move_to(cx, pos + large),
        Part::Thumb => false,
    }
}

/// The part under the mouse at (x, y).
fn part_at(cx: &Cx, x: f64, y: f64) -> Option<Part> {
    let (len, a) = along(cx, x, y);
    let across = if vertical(cx.store, cx.id) { x } else { y };
    let thick = if vertical(cx.store, cx.id) { cx.width() } else { cx.height() };
    if across < 0.0 || across >= thick as f64 {
        return None;
    }
    values(cx.store, cx.id).part_at(a, len)
}

impl ComponentKind for ScrollBar {
    fn name(&self) -> &'static str {
        "RSCROLLBAR"
    }

    fn paint_over(&self, _store: &dyn Store, _id: &str, w: i64, h: i64, p: &mut Painter) {
        // (classic: a thin sunken line round it, a pixel outside its
        // rectangle, as RapidQ's TScrollBar shows on Windows — the shadow
        // above and left, white below and right)
        if !p.fluent() && w > 0 && h > 0 {
            p.thin_sunken((-1, -1, w + 2, h + 2));
        }
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let vert = vertical(cx.store, cx.id);
        let len = if vert { h } else { w };
        let enabled = cx.state.enabled;
        let held = held(cx.id).filter(|_| cx.state.held);
        let mut v = values(cx.store, cx.id);
        if let Some(Held { part: Part::Thumb, at, .. }) = held {
            v.position = at;
        }
        let thumb = if enabled { v.thumb(len) } else { None };
        // (an arrow or a page part shows pushed while the mouse is on it)
        let pressed = held.and_then(|hd| (hd.part == Part::Thumb || part_at(cx, hd.x, hd.y) == Some(hd.part)).then_some((vert, hd.part)));
        let mut sc = Scroller::default();
        sc.alone = Some(Alone { vertical: vert, thumb, disabled: !enabled });
        sc.pressed = pressed;
        p.ops(crate::paint::bar_ops(&sc, w, h));
        if cx.state.focused && p.fluent() {
            p.focus((0, 0, w, h));
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        match m.kind {
            MouseKind::Down => {
                let Some(part) = part_at(cx, m.x, m.y) else { return MouseOut::default() };
                let (_, a) = along(cx, m.x, m.y);
                let v = values(cx.store, cx.id);
                let pos = v.clamp(v.position);
                set_held(cx.id, Some(Held { part, x: m.x, y: m.y, from: a, start: pos, at: pos }));
                if part != Part::Thumb {
                    step(cx, part);
                    cx.ui.wake = Some(now() + REPEAT_DELAY);
                }
                MouseOut { press: true, focus: None }
            }
            MouseKind::Move if m.captured => {
                let Some(mut hd) = held(cx.id) else { return MouseOut::default() };
                hd.x = m.x;
                hd.y = m.y;
                if hd.part == Part::Thumb {
                    let (len, a) = along(cx, m.x, m.y);
                    let to = values(cx.store, cx.id).dragged(hd.start, a - hd.from, len);
                    if to != hd.at {
                        hd.at = to;
                        let id = cx.id.to_string();
                        cx.set(&id, "position", to);
                        cx.change();
                    }
                }
                set_held(cx.id, Some(hd));
                MouseOut::default()
            }
            MouseKind::Up => {
                set_held(cx.id, None);
                cx.ui.wake = None;
                MouseOut::default()
            }
            _ => MouseOut::default(),
        }
    }

    /// A held arrow or page part repeats while the mouse is on it (a page
    /// stops where the thumb reaches the mouse: the part under it is the
    /// thumb then).
    fn tick(&self, cx: &mut Cx) {
        let Some(hd) = held(cx.id) else { return };
        if !cx.state.held || hd.part == Part::Thumb {
            return;
        }
        if part_at(cx, hd.x, hd.y) == Some(hd.part) {
            step(cx, hd.part);
        }
        cx.ui.wake = Some(now() + REPEAT);
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        if k.mods.alt || k.mods.command {
            return false;
        }
        let v = values(cx.store, cx.id);
        match k.vk {
            37 | 38 => step(cx, Part::Back),
            39 | 40 => step(cx, Part::Forward),
            33 => step(cx, Part::PageBack),
            34 => step(cx, Part::PageForward),
            36 => move_to(cx, v.min),
            35 => move_to(cx, v.last()),
            _ => return false,
        };
        true
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }

    fn access(&self, cx: &mut Cx, action: Action, _part: Option<usize>, value: Option<&AccessValue>) -> bool {
        match (action, value) {
            // (as the arrow keys would)
            (Action::Increment, _) => step(cx, Part::Forward),
            (Action::Decrement, _) => step(cx, Part::Back),
            (Action::SetValue, Some(AccessValue::Number(n))) => move_to(cx, n.round() as i64),
            _ => return false,
        };
        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rapidr_value::input::Button;
    use rapidr_value::objects::a11y::{node_id, Action, Orientation, Role};
    use rapidr_value::v_int;

    use crate::tick::{now, set_test_now, REPEAT_DELAY};
    use crate::{FormUi, KernelEvent, MemClipboard, MemStore, Mods, Op, TextSystem};

    fn set(v: i64) -> KernelEvent {
        KernelEvent::Set { id: "sb".into(), prop: "position".into(), value: v }
    }

    fn change() -> KernelEvent {
        KernelEvent::Change("sb".into())
    }

    /// A 200 × 17 horizontal bar at (10, 10), Position 30 (ranges.bas's
    /// `Sh`).
    fn bar() -> (MemStore, FormUi, TextSystem) {
        let mut s = MemStore::new();
        s.add("sbform", "RFORM", None);
        s.add("sb", "RSCROLLBAR", Some("sbform"));
        for (p, v) in [("left", 10), ("top", 10), ("width", 200), ("height", 17), ("position", 30)] {
            s.set("sb", p, v_int(v));
        }
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "sbform", false);
        f.paint(&s, &mut ts, 1.0);
        (s, f, ts)
    }

    fn events(f: &mut FormUi) -> Vec<KernelEvent> {
        f.take_events().into_iter().filter(|e| matches!(e, KernelEvent::Set { .. } | KernelEvent::Change(_))).collect()
    }

    fn click(s: &MemStore, f: &mut FormUi, ts: &mut TextSystem, x: f64, y: f64) {
        f.mouse_down(s, ts, x, y, Button::Left, Mods::NONE);
        f.mouse_up(s, ts, x, y, Button::Left, Mods::NONE);
    }

    #[test]
    fn arrows_and_track_step_position() {
        let (mut s, mut f, mut ts) = bar();
        s.set("sb", "smallchange", v_int(2)).set("sb", "largechange", v_int(10));
        // The right arrow: SmallChange on; then the left one back.
        click(&s, &mut f, &mut ts, 205.0, 18.0);
        assert_eq!(events(&mut f), vec![set(32), change()]);
        s.set("sb", "position", v_int(32));
        click(&s, &mut f, &mut ts, 15.0, 18.0);
        assert_eq!(events(&mut f), vec![set(30), change()]);
        s.set("sb", "position", v_int(30));
        // The track before the thumb (at 64..72 along): a page back; after
        // it a page on.
        click(&s, &mut f, &mut ts, 40.0, 18.0);
        assert_eq!(events(&mut f), vec![set(20), change()]);
        s.set("sb", "position", v_int(20));
        click(&s, &mut f, &mut ts, 150.0, 18.0);
        assert_eq!(events(&mut f), vec![set(30), change()]);
        // At Min the left arrow changes nothing (no events).
        s.set("sb", "position", v_int(0));
        click(&s, &mut f, &mut ts, 15.0, 18.0);
        assert_eq!(events(&mut f), vec![]);
    }

    #[test]
    fn a_held_arrow_repeats() {
        let (mut s, mut f, mut ts) = bar();
        let t0 = now();
        set_test_now(Some(t0));
        f.mouse_down(&s, &mut ts, 205.0, 18.0, Button::Left, Mods::NONE);
        assert_eq!(events(&mut f), vec![set(31), change()]);
        s.set("sb", "position", v_int(31));
        f.tick(&s, &mut ts, t0 + REPEAT_DELAY + Duration::from_millis(1));
        assert_eq!(events(&mut f), vec![set(32), change()]);
        f.mouse_up(&s, &mut ts, 205.0, 18.0, Button::Left, Mods::NONE);
        assert_eq!(f.next_wake(), None);
        set_test_now(None);
    }

    #[test]
    fn the_thumb_drags_and_keys_move_it() {
        let (mut s, mut f, mut ts) = bar();
        // The thumb is 8 wide at 64 along (Windows': 17 + 158 × 30 / 100):
        // 79 more pixels is half the room, 50 more.
        f.mouse_down(&s, &mut ts, 78.0, 18.0, Button::Left, Mods::NONE);
        f.mouse_move(&s, &mut ts, 157.0, 18.0, Mods::NONE);
        assert_eq!(events(&mut f), vec![set(80), change()]);
        f.mouse_move(&s, &mut ts, 157.0, 20.0, Mods::NONE);
        assert_eq!(events(&mut f), vec![], "no change, no event");
        f.mouse_up(&s, &mut ts, 157.0, 20.0, Button::Left, Mods::NONE);
        s.set("sb", "position", v_int(80));
        // It has the focus now: End, Home, Page Down, an arrow.
        let mut clip = MemClipboard::default();
        f.key_down(&s, &mut ts, 35, "", Mods::NONE, &mut clip);
        assert_eq!(events(&mut f), vec![set(100), change()]);
        f.key_down(&s, &mut ts, 36, "", Mods::NONE, &mut clip);
        assert_eq!(events(&mut f), vec![set(0), change()]);
        s.set("sb", "largechange", v_int(25));
        f.key_down(&s, &mut ts, 34, "", Mods::NONE, &mut clip);
        assert_eq!(events(&mut f), vec![set(100), change()]);
        f.key_down(&s, &mut ts, 37, "", Mods::NONE, &mut clip);
        assert_eq!(events(&mut f), vec![set(79), change()]);
    }

    #[test]
    fn drawn_as_windows_classic_bar() {
        let (mut s, mut f, mut ts) = bar();
        let ops = |f: &mut FormUi, s: &MemStore, ts: &mut TextSystem| {
            let list = f.paint(s, ts, 1.0);
            list.items.iter().filter_map(|i| match i {
                crate::display::Item::Op { origin, op } if *origin == (10, 10) => Some(op.clone()),
                _ => None,
            }).collect::<Vec<_>>()
        };
        let enabled = ops(&mut f, &s, &mut ts);
        let arrows = enabled.iter().filter(|o| matches!(o, Op::Arrow { .. })).count();
        assert_eq!(arrows, 2);
        // (the thumb's EDGE_RAISED: COLOR_3DLIGHT at its top left, 64 along)
        let t = rapidr_value::theme::current();
        if !t.fluent() {
            assert!(enabled.contains(&Op::Fill { rect: (64, 0, 8, 1), color: t.light3d }));
            // the track's checks: white where x + y is odd
            assert!(enabled.iter().any(|o| matches!(o, Op::Checker { rect: (0, 0, 200, 17), a, b } if *a == t.light && *b == t.face)), "{enabled:?}");
        }
        // Disabled: no thumb, each arrow embossed (two triangles).
        s.set("sb", "enabled", v_int(0));
        let mut f = FormUi::build(&s, "sbform", false);
        let disabled = ops(&mut f, &s, &mut ts);
        if !t.fluent() {
            assert_eq!(disabled.iter().filter(|o| matches!(o, Op::Arrow { .. })).count(), 4);
            assert!(!disabled.contains(&Op::Fill { rect: (64, 0, 8, 1), color: t.light3d }));
        }
    }

    #[test]
    fn described_as_a_slider() {
        let (mut s, mut f, mut ts) = bar();
        s.set("sb", "kind", v_int(1)).set("sb", "width", v_int(17)).set("sb", "height", v_int(130));
        let mut f2 = FormUi::build(&s, "sbform", false);
        let tree = f2.access_tree(&s, &mut ts);
        let n = tree.find(node_id("sb")).unwrap();
        let num = n.numeric.unwrap();
        assert_eq!((n.role, num.value, num.min, num.max, n.orientation), (Role::Slider, 30.0, 0.0, 100.0, Some(Orientation::Vertical)));
        // a screen reader's Increment: as the arrow
        assert!(f.access_action(&s, &mut ts, node_id("sb"), Action::Increment, None));
        assert_eq!(events(&mut f), vec![set(31), change()]);
    }
}
