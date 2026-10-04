//! QFORMMDI's child windows. The shared model (`rapidr_value::mdi`, the
//! web's too) keeps the children, their places, states and z-order;
//! runtime-core's `mdi::Runtime` gives each child a frame component
//! (`RMDICHILD`: `caption`, `active`, `childstate`, `__form`,
//! `__component`) and places the child's component inside it. The kernel
//! draws a frame as a Windows-classic MDI child (a raised border, the
//! title bar — dark blue when active, grey otherwise — with the title in
//! white bold and the minimize / maximize / close buttons), stacks the
//! frames and their components in the model's z-order, and turns the
//! mouse on a frame into the model's actions ([`Container::Mdi`]: a
//! button, a press that raises it, a drag by the title bar or the
//! bottom-right corner), which runtime-core applies after the pump with
//! the program's events (OnChildActive, OnChildClose …).

use std::cell::RefCell;

use rapidr_value::mdi::{self, Action, BORDER, TITLE_HEIGHT};
use rapidr_value::objects::a11y::{node_id, AccessNode, Role};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Place;

use super::form::Container;
use super::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::input::KernelEvent;
use crate::paint::{Painter, DARK, FACE, LIGHT, SHADOW};
use crate::store::{self, Store};

pub struct ChildFrame;

/// The title bar's colours (COLOR_ACTIVECAPTION, COLOR_INACTIVECAPTION).
const ACTIVE: u32 = 0x0A246A;
const INACTIVE: u32 = 0x808080;
/// The bottom-right corner that resizes it.
const GRIP: i64 = 12;

/// `parent`'s children for the tree: a QFORMMDI's child frames and their
/// components over its other components, in the model's z-order (the last
/// on top), as runtime-core's `stack_widgets` stacks FLTK's widgets.
pub fn stacked(parent: &str, children: Vec<(String, String)>) -> Vec<(String, String)> {
    if !mdi::is_mdi(parent) {
        return children;
    }
    let order: Vec<String> = mdi::frames(parent).iter().flat_map(|f| [mdi::frame_name(parent, &f.component), f.component.to_ascii_lowercase()]).collect();
    let rank = |id: &str| order.iter().position(|o| o.eq_ignore_ascii_case(id));
    let (mut on_top, mut out): (Vec<_>, Vec<_>) = children.into_iter().partition(|(id, _)| rank(id).is_some());
    on_top.sort_by_key(|(id, _)| rank(id));
    out.extend(on_top);
    out
}

/// A frame being dragged: by its title bar (moved) or its corner
/// (resized); the mouse's place in the window then, and the frame's Left /
/// Top (or Width / Height) then.
struct Drag {
    frame: String,
    resize: bool,
    mouse: (f64, f64),
    start: (i64, i64),
}

thread_local! {
    static DRAG: RefCell<Option<Drag>> = const { RefCell::new(None) };
}

/// The title bar's rectangle in a `w` wide frame.
fn title_bar(w: i64) -> (i64, i64, i64, i64) {
    (BORDER, BORDER, (w - 2 * BORDER).max(0), TITLE_HEIGHT - 2)
}

/// Caption button `slot` (0 close, 1 maximize, 2 minimize from the right).
fn button_rect(w: i64, slot: i64) -> (i64, i64, i64, i64) {
    (w - BORDER - (slot + 1) * (TITLE_HEIGHT - 2) + 1, BORDER + 2, TITLE_HEIGHT - 5, TITLE_HEIGHT - 6)
}

fn glyph(p: &mut Painter, slot: i64, (x, y, w, h): (i64, i64, i64, i64), maximized: bool) {
    let black = 0x000000;
    let (cx, cy) = (x + w / 2, y + h / 2);
    match slot {
        // ×: two lines, two pixels thick
        0 => {
            for d in [0.0, 1.0] {
                let (l, t) = ((cx - 4) as f64 + d, (cy - 4) as f64);
                p.line((l + 0.5, t + 0.5), (l + 7.5, t + 7.5), black);
                p.line((l + 0.5, t + 7.5), (l + 7.5, t + 0.5), black);
            }
        }
        1 if maximized => {
            // restore: two windows, the back one up and right
            for (bx, by) in [(cx - 2, cy - 5), (cx - 5, cy - 2)] {
                p.fill((bx, by, 7, 7), FACE);
                p.edge((bx, by, 7, 7), &[black], &[black]);
                p.fill((bx, by + 1, 7, 1), black);
            }
        }
        1 => {
            p.edge((cx - 5, cy - 5, 10, 9), &[black], &[black]);
            p.fill((cx - 5, cy - 4, 10, 1), black);
        }
        _ => p.fill((cx - 4, cy + 2, 7, 2), black),
    }
}

impl ComponentKind for ChildFrame {
    fn name(&self) -> &'static str {
        "RMDICHILD"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let active = store::flag(cx.store, cx.id, "active", false);
        let maximized = store::int(cx.store, cx.id, "childstate", 0) == 2;
        p.fill((0, 0, w, h), FACE);
        // (a window's raised border)
        p.edge((0, 0, w, h), &[FACE, LIGHT], &[DARK, SHADOW]);
        let (bx, by, bw, bh) = title_bar(w);
        p.fill((bx, by, bw, bh), if active { ACTIVE } else { INACTIVE });
        let title = store::string(cx.store, cx.id, "caption");
        let font = Font { styles: cx.font.styles | 1, color: 0xFFFFFF, ..cx.font.clone() };
        let room = (bw - 3 * (TITLE_HEIGHT - 2) - 6).max(0);
        p.clipped((bx + 2, by, room, bh), |p| p.text((bx + 3, by, room, bh), &title, &font, 0xFFFFFF, Place::Left));
        for slot in 0..3 {
            let r = button_rect(w, slot);
            if r.0 <= bx {
                continue;
            }
            p.fill(r, FACE);
            p.edge(r, &[LIGHT, FACE], &[DARK, SHADOW]);
            glyph(p, slot, r, maximized);
        }
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let form = store::string(cx.store, cx.id, "__form");
        let component = store::string(cx.store, cx.id, "__component");
        let (w, h) = (cx.width(), cx.height());
        let (ox, oy) = (cx.rect.0 as f64, cx.rect.1 as f64);
        let abs = |m: &MouseIn| (m.x + ox, m.y + oy);
        let act = |cx: &mut Cx, action: Action| cx.events.push(KernelEvent::Container(Container::Mdi { form: form.clone(), component: component.clone(), action }));
        match m.kind {
            MouseKind::Down => {
                let (x, y) = (m.x.floor() as i64, m.y.floor() as i64);
                if let Some(action) = mdi::button_at(w, x, y) {
                    act(cx, action);
                    return MouseOut { press: false, focus: Some(false) };
                }
                act(cx, Action::Activate);
                let corner = x >= w - GRIP && y >= h - GRIP;
                if corner || y < BORDER + TITLE_HEIGHT {
                    let start = if corner { (w, h) } else { (store::int(cx.store, cx.id, "left", 0), store::int(cx.store, cx.id, "top", 0)) };
                    DRAG.with(|d| *d.borrow_mut() = Some(Drag { frame: cx.id.to_string(), resize: corner, mouse: abs(m), start }));
                }
            }
            MouseKind::Move if m.captured => {
                let Some((resize, from, start)) = DRAG.with(|d| d.borrow().as_ref().filter(|d| d.frame == cx.id).map(|d| (d.resize, d.mouse, d.start))) else {
                    return MouseOut::default();
                };
                let (ax, ay) = abs(m);
                let (dx, dy) = ((ax - from.0).round() as i64, (ay - from.1).round() as i64);
                act(cx, if resize { Action::Resize(start.0 + dx, start.1 + dy) } else { Action::Move(start.0 + dx, start.1 + dy) });
            }
            MouseKind::Up => DRAG.with(|d| *d.borrow_mut() = None),
            _ => {}
        }
        MouseOut { press: false, focus: Some(false) }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Window);
        n.name = store::string(cx.store, cx.id, "caption");
        n.states.focused = store::flag(cx.store, cx.id, "active", false);
        n.bounds = cx.rect;
        n
    }
}
