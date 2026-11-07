//! (RapidR's) BringToFront / SendToBack in the kernel: a parent's children
//! are drawn and take the mouse in the order the program stacked them
//! (rapidr_value::zorder) — a panel brought to the front covers the
//! controls under it and takes their clicks; sent back, it is under them;
//! its own children come with it.

use rapidr_value::input::Button;
use rapidr_value::objects::ops::Op;
use rapidr_value::{v_int, v_str, zorder};

use crate::{FormUi, Item, KernelEvent, MemStore, Mods, TextSystem};

const NONE: Mods = Mods::NONE;
/// The drawer's colour (BGR, as RapidQ's): something no theme draws.
const DRAWER: u32 = 0x00_33_66_99;

fn place(s: &mut MemStore, id: &str, (l, t, w, h): (i64, i64, i64, i64)) {
    s.set(id, "left", v_int(l)).set(id, "top", v_int(t)).set(id, "width", v_int(w)).set(id, "height", v_int(h));
}

/// A form with a drawer panel (made first: under) holding a button, and a
/// button over where the drawer is (made after: on top).
fn world() -> (MemStore, FormUi, TextSystem) {
    let mut s = MemStore::new();
    s.add("zform", "RFORM", None).set("zform", "clientwidth", v_int(320)).set("zform", "clientheight", v_int(200));
    s.add("drawer", "RPANEL", Some("zform"));
    place(&mut s, "drawer", (0, 0, 160, 200));
    s.set("drawer", "color", v_int(DRAWER as i64)).set("drawer", "bevelouter", v_int(0));
    s.add("inside", "RBUTTON", Some("drawer")).set("inside", "caption", v_str("Inside"));
    place(&mut s, "inside", (10, 150, 100, 30));
    s.add("under", "RBUTTON", Some("zform")).set("under", "caption", v_str("Under"));
    place(&mut s, "under", (20, 20, 100, 30));
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "zform", true);
    drop(f.paint(&s, &mut ts, 1.0));
    (s, f, ts)
}

/// Where in the display list the drawer's fill and the "Under" button's
/// caption are drawn: (fill, caption) — the later one is on top.
fn drawn(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem) -> (usize, usize) {
    let list = f.paint(s, ts, 1.0);
    let want = crate::paint::color_of(s, "drawer").expect("the drawer has a colour");
    let fill = list.items.iter().position(|i| matches!(i, Item::Op { op: Op::Fill { color, .. }, .. } if *color == want)).expect("the drawer is drawn");
    let caption = list.items.iter().position(|i| matches!(i, Item::Op { op: Op::Text { text, .. }, .. } if text == "Under")).expect("the button is drawn");
    (fill, caption)
}

fn click(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, x: f64, y: f64) -> Vec<KernelEvent> {
    f.take_events();
    f.mouse_move(s, ts, x, y, NONE);
    f.mouse_down(s, ts, x, y, Button::Left, NONE);
    f.mouse_up(s, ts, x, y, Button::Left, NONE);
    f.take_events()
}

fn clicked(events: &[KernelEvent], id: &str) -> bool {
    events.iter().any(|e| matches!(e, KernelEvent::Click(c) if c == id))
}

#[test]
fn a_panel_brought_to_the_front_covers_its_siblings_and_takes_their_clicks() {
    let (mut s, mut f, mut ts) = world();
    // (as made: the button over the drawer, drawn after it, clicked)
    let (fill, caption) = drawn(&mut f, &s, &mut ts);
    assert!(fill < caption, "the drawer under the button: fill {fill}, caption {caption}");
    assert_eq!(f.hit(40.0, 30.0).map(|i| f.nodes[i].id.as_str()), Some("under"));
    assert!(clicked(&click(&mut f, &s, &mut ts, 40.0, 30.0), "under"));
    // BringToFront: the drawer and its button over the other button
    s.set("drawer", zorder::KEY, v_int(zorder::next(true)));
    f.rebuild(&s);
    let (fill, caption) = drawn(&mut f, &s, &mut ts);
    assert!(fill > caption, "the drawer over the button: fill {fill}, caption {caption}");
    assert_eq!(f.hit(40.0, 30.0).map(|i| f.nodes[i].id.as_str()), Some("drawer"));
    let events = click(&mut f, &s, &mut ts, 40.0, 30.0);
    assert!(!clicked(&events, "under"), "the covered button hears nothing: {events:?}");
    // (its own children came with it)
    assert_eq!(f.hit(50.0, 160.0).map(|i| f.nodes[i].id.as_str()), Some("inside"));
    assert!(clicked(&click(&mut f, &s, &mut ts, 50.0, 160.0), "inside"));
    // (where it doesn't cover them, the others still take the mouse)
    assert_eq!(f.hit(200.0, 100.0), None);
    // SendToBack: under the button again
    s.set("drawer", zorder::KEY, v_int(zorder::next(false)));
    f.rebuild(&s);
    let (fill, caption) = drawn(&mut f, &s, &mut ts);
    assert!(fill < caption, "sent back: fill {fill}, caption {caption}");
    assert!(clicked(&click(&mut f, &s, &mut ts, 40.0, 30.0), "under"));
}

/// The button brought to the front, then the drawer: the last brought is on
/// top; Tab (no TabOrder) follows the stack.
#[test]
fn the_last_brought_to_the_front_is_on_top() {
    let (mut s, mut f, mut ts) = world();
    s.set("under", zorder::KEY, v_int(zorder::next(true)));
    s.set("drawer", zorder::KEY, v_int(zorder::next(true)));
    f.rebuild(&s);
    assert_eq!(f.hit(40.0, 30.0).map(|i| f.nodes[i].id.as_str()), Some("drawer"));
    let order: Vec<&str> = f.tab_order(&s).into_iter().map(|i| f.nodes[i].id.as_str()).collect();
    assert_eq!(order, vec!["under", "inside"]);
    s.set("under", zorder::KEY, v_int(zorder::next(true)));
    f.rebuild(&s);
    assert_eq!(f.hit(40.0, 30.0).map(|i| f.nodes[i].id.as_str()), Some("under"));
    drop(f.paint(&s, &mut ts, 1.0));
}
