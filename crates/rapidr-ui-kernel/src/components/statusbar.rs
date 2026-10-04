//! QSTATUSBAR (docked at the bottom by its Align, as in RapidQ:
//! runtime-core's layout): its panels left to right, each `Panel(i).Width`
//! pixels (100 by default; the last one takes the rest), each a thin
//! sunken box with its Caption on one line — or its SimpleText in one box
//! when SimplePanel is set or it has no panels; as the FLTK and web
//! runtimes lay it out.
//!
//! (the input lane's) Its **size grip** (Delphi's TStatusBar.SizeGrip,
//! RapidQ's SizeGrip, True by default): on a sizeable form, docked at its
//! bottom, the bar's bottom-right square shows Windows' classic grip (the
//! last box ends before it), and a drag from it resizes the window — the
//! grip takes the press before the components, as Windows' HTBOTTOMRIGHT
//! (no OnMouseDown); the host resizes the window as the user's drag of its
//! border would (`Container::Resize` → `HostCmd::Resize`), so OnResize and
//! Width / Height follow.

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::layout::{self, STATUS_GRIP};
use rapidr_value::objects::a11y::{node_id, AccessNode, Role};
use rapidr_value::objects::ops::{Place, Rect};

use super::form::Container;
use super::{ComponentKind, Cx};
use crate::input::KernelEvent;
use crate::paint::{Painter, FACE, GRAY_TEXT, LIGHT, SHADOW};
use crate::store::{self, Store};
use crate::text::bgr_to_rgb;
use crate::tree::FormUi;

pub struct StatusBar;

/// (the input lane's) Whether status bar `id` shows its size grip: SizeGrip
/// (True unless set), the form its parent, sizeable, the bar at its bottom.
pub fn has_grip(store: &dyn Store, id: &str) -> bool {
    let parent = store::string(store, id, "parent");
    let on_form = !parent.is_empty() && matches!(store.type_of(&parent).to_ascii_uppercase().as_str(), "RFORM" | "RFORMMDI");
    let align = layout::Align::from_value(store::int(store, id, "align", layout::default_align("RSTATUSBAR").value()));
    layout::status_grip(store::flag(store, id, "sizegrip", true), on_form, store::int(store, &parent, "borderstyle", 2), align)
}

/// Its boxes (in its own pixels) and their texts.
pub fn panels(store: &dyn Store, id: &str, w: i64, h: i64) -> Vec<(Rect, String)> {
    let count = store::int(store, id, "panelcount", 0).clamp(0, 256);
    // (a box 2 pixels narrower than its panel; 1 pixel in from the bar's
    // edges; the last ends before the size grip)
    let boxed = |x: i64, pw: i64| (x, 2, (pw - 2).max(0), (h - 3).max(0));
    let w = if has_grip(store, id) { w - STATUS_GRIP } else { w };
    if count == 0 || store::flag(store, id, "simplepanel", false) {
        return vec![(boxed(1, w), store::string(store, id, "simpletext"))];
    }
    let mut out = Vec::new();
    let mut x = 1;
    for i in 0..count {
        let width = store::int(store, id, &format!("panel({i}).width"), 0);
        let pw = if i == count - 1 { (w + 1 - x).max(0) } else if width > 0 { width.min(10_000) } else { 100 };
        out.push((boxed(x, pw), store::string(store, id, &format!("panel({i}).caption"))));
        x += pw;
    }
    out
}

/// (the input lane's) Windows' classic size grip in a bar `w` × `h`: three
/// raised ridges across its bottom-right corner, each a white line over two
/// grey ones (DrawFrameControl's DFCS_SCROLLSIZEGRIP).
fn paint_grip(p: &mut Painter, w: i64, h: i64) {
    let (cx, cy) = (w - 1, h - 1);
    for base in [1, 5, 9] {
        for (d, color) in [(base, SHADOW), (base + 1, SHADOW), (base + 2, LIGHT)] {
            // (the pixels d steps from the corner along the anti-diagonal)
            for k in 0..=d {
                let (x, y) = (cx - d + k, cy - k);
                if x >= w - STATUS_GRIP + 2 && y >= h - STATUS_GRIP + 2 {
                    p.fill((x, y, 1, 1), color);
                }
            }
        }
    }
}

thread_local! {
    /// (the input lane's) A size grip held, per form: the mouse's offset
    /// from the window's inside's bottom-right.
    static GRIP: RefCell<HashMap<String, (f64, f64)>> = RefCell::new(HashMap::new());
}

/// (the input lane's) A press at (x, y) of form `f`'s inside on a status
/// bar's size grip: its drag starts. Whether it was on one (then nothing
/// else hears the press, as Windows' sizing border).
pub(crate) fn grip_down(f: &mut FormUi, store: &dyn Store, x: f64, y: f64) -> bool {
    let hit = (0..f.nodes.len()).rev().find(|&i| {
        let n = &f.nodes[i];
        let (bx, by, bw, bh) = n.abs;
        n.shown
            && n.enabled
            && n.type_name == "RSTATUSBAR"
            && x >= (bx + bw - STATUS_GRIP) as f64
            && y >= (by + bh - STATUS_GRIP) as f64
            && x < (bx + bw) as f64
            && y < (by + bh) as f64
            && has_grip(store, &n.id)
    });
    let Some(i) = hit else { return false };
    // (the bar is docked at the bottom, its full width: its bottom-right is
    // the window's inside's)
    let (bx, by, bw, bh) = f.nodes[i].abs;
    GRIP.with(|g| g.borrow_mut().insert(f.form.clone(), ((bx + bw) as f64 - x, (by + bh) as f64 - y)));
    true
}

/// (the input lane's) The mouse moved to (x, y) with a grip held: the
/// window's inside asked to grow or shrink with it. Whether a grip is held.
pub(crate) fn grip_drag(f: &mut FormUi, x: f64, y: f64) -> bool {
    let Some((ox, oy)) = GRIP.with(|g| g.borrow().get(&f.form).copied()) else { return false };
    let (w, h) = ((x + ox).round() as i64, (y + oy).round() as i64);
    f.events.push(KernelEvent::Container(Container::Resize { form: f.form.clone(), w: w.max(1), h: h.max(1) }));
    true
}

/// (the input lane's) A release: the grip let go. Whether one was held.
pub(crate) fn grip_up(f: &mut FormUi) -> bool {
    GRIP.with(|g| g.borrow_mut().remove(&f.form)).is_some()
}

impl ComponentKind for StatusBar {
    fn name(&self) -> &'static str {
        "RSTATUSBAR"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        p.fill((0, 0, w, h), FACE);
        let color = if cx.state.enabled { bgr_to_rgb(cx.font.color) } else { GRAY_TEXT };
        for ((x, y, bw, bh), text) in panels(cx.store, cx.id, w, h) {
            if bw <= 0 || bh <= 0 {
                continue;
            }
            p.edge((x, y, bw, bh), &[SHADOW], &[LIGHT]);
            // (the caption 3 pixels in, cut at the box's inside)
            let inside = (x + 1, y + 1, (bw - 2).max(0), (bh - 2).max(0));
            p.clipped(inside, |p| p.text((x + 3, y, (bw - 6).max(0), bh), &text, &cx.font, color, Place::Left));
        }
        if has_grip(cx.store, cx.id) {
            paint_grip(p, w, h);
        }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Group);
        n.bounds = cx.rect;
        let (x0, y0, w, h) = cx.rect;
        for (k, ((x, y, bw, bh), text)) in panels(cx.store, cx.id, w, h).into_iter().enumerate() {
            let mut l = AccessNode::new(node_id(&format!("{}.panel({k})", cx.id)), Role::Label);
            l.name = text;
            l.bounds = (x0 + x, y0 + y, bw, bh);
            n.children.push(l);
        }
        n
    }
}

#[cfg(test)]
mod tests {
    //! (the input lane's) The size grip, headless.

    use rapidr_value::input::{Button, Mouse};
    use rapidr_value::{v_int, v_str};

    use super::super::form::Container;
    use crate::{FormUi, KernelEvent, MemStore, Mods, Op, TextSystem};

    /// A sizeable 300 × 200 form with a bar docked at its bottom.
    fn form(tag: &str) -> MemStore {
        let (f, b) = (format!("{tag}f"), format!("{tag}b"));
        let mut s = MemStore::new();
        s.add(&f, "RFORM", None).set(&f, "clientwidth", v_int(300)).set(&f, "clientheight", v_int(200));
        s.add(&b, "RSTATUSBAR", Some(&f)).set(&b, "align", v_int(2)).set(&b, "top", v_int(176)).set(&b, "width", v_int(300)).set(&b, "height", v_int(24));
        s.set(&b, "simpletext", v_str("ready"));
        s
    }

    fn press_drag(s: &MemStore, tag: &str) -> Vec<KernelEvent> {
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(s, &format!("{tag}f"), false);
        drop(f.paint(s, &mut ts, 1.0));
        f.mouse_down(s, &mut ts, 295.0, 196.0, Button::Left, Mods::NONE);
        f.mouse_move(s, &mut ts, 345.0, 236.0, Mods::NONE);
        f.mouse_up(s, &mut ts, 345.0, 236.0, Button::Left, Mods::NONE);
        f.take_events()
    }

    #[test]
    fn the_size_grip_resizes_the_window_and_is_drawn() {
        let s = form("sg1");
        // (the grip takes the press — no OnMouseDown — and the drag asks the
        // host for the window's inside 50 wider, 40 higher)
        let events = press_drag(&s, "sg1");
        assert_eq!(events, vec![KernelEvent::Container(Container::Resize { form: "sg1f".into(), w: 350, h: 240 })]);
        // (drawn: the ridges' white at the corner; the box ends before it)
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "sg1f", false);
        let list = f.paint(&s, &mut ts, 1.0);
        let lit = |x: i64, y: i64| list.items.iter().any(|i| matches!(i, crate::Item::Op { origin, op: Op::Fill { rect, color: crate::paint::LIGHT } } if (origin.0 + rect.0, origin.1 + rect.1) == (x, y) && rect.2 == 1));
        assert!(lit(299 - 3, 199), "a ridge's white pixel");
        assert_eq!(super::panels(&s, "sg1b", 300, 24)[0].0 .2, 300 - 16 - 2);
    }

    #[test]
    fn no_grip_without_sizegrip_a_sizeable_form_or_the_bottom() {
        for (k, change) in ["sizegrip", "borderstyle", "align"].into_iter().enumerate() {
            let tag = format!("sg2{k}");
            let mut s = form(&tag);
            match change {
                "sizegrip" => s.set(&format!("{tag}b"), "sizegrip", v_int(0)),
                "borderstyle" => s.set(&format!("{tag}f"), "borderstyle", v_int(3)),
                _ => s.set(&format!("{tag}b"), "align", v_int(1)),
            };
            let events = press_drag(&s, &tag);
            assert!(events.iter().any(|e| matches!(e, KernelEvent::Mouse { kind: Mouse::Down, .. })), "{change}: the bar's own press");
            assert!(!events.iter().any(|e| matches!(e, KernelEvent::Container(_))), "{change}: no resize");
        }
    }
}
