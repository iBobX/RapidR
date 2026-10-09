//! QFORMMDI's child windows. The shared model (`rapidr_value::mdi`, the
//! web's too) keeps the children, their places, states and z-order;
//! runtime-core's `mdi::Runtime` gives each child a frame component
//! (`RMDICHILD`: `caption`, `active`, `childstate`, `__form`,
//! `__component`) and places the child's component inside it. The kernel
//! draws a frame as it draws every window (`window_frame`: Windows'
//! classic MDI child, or RapidR's window), stacks the
//! frames and their components in the model's z-order, and turns the
//! mouse on a frame into the model's actions ([`Container::Mdi`]: a
//! button, a press that raises it, a drag by the title bar, or by any edge
//! or corner of its sizing border as Windows' (`mdi::edges_at`), which
//! runtime-core applies after the pump with
//! the program's events (OnChildActive, OnChildClose …).

use std::cell::RefCell;

use rapidr_value::input::Cursor;
use rapidr_value::mdi::{self, Action, BORDER, TITLE_HEIGHT};
use rapidr_value::objects::a11y::AccessNode;
use rapidr_value::objects::font::Font;

use super::form::Container;
use super::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::input::KernelEvent;
use crate::paint::Painter;
use crate::window_frame::{self, Button, Chrome, Metrics};
use crate::store::{self, Store};

pub struct ChildFrame;

/// `parent`'s children for the tree: a QFORMMDI's child frames and their
/// components over its other components, in the model's z-order (the last
/// on top).
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

/// A frame being dragged: by its title bar (moved: `edges` none) or an
/// edge / corner (resized); the mouse's place in the window then, and the
/// frame's Left, Top, Width, Height then.
struct Drag {
    frame: String,
    edges: Option<mdi::Edges>,
    mouse: (f64, f64),
    start: (i64, i64, i64, i64),
}

/// The edges under (x, y) of frame `id` (a normal one: a maximized or
/// minimized child has no sizing border), for the press and the pointer.
pub fn edges_at(store: &dyn Store, id: &str, w: i64, h: i64, x: i64, y: i64) -> Option<mdi::Edges> {
    if store::int(store, id, "childstate", 0) != 0 {
        return None;
    }
    mdi::edges_at(w, h, x, y)
}

thread_local! {
    static DRAG: RefCell<Option<Drag>> = const { RefCell::new(None) };
}

impl ComponentKind for ChildFrame {
    fn name(&self) -> &'static str {
        "RMDICHILD"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let t = p.theme();
        // (the frame every window the kernel draws has: the classic look's
        // title in the window's font, RapidR's in the chrome font)
        let base = if t.fluent() { rapidr_value::ide_theme::chrome_font(t) } else { cx.font.clone() };
        let chrome = Chrome {
            title: store::string(cx.store, cx.id, "caption"),
            active: store::flag(cx.store, cx.id, "active", false),
            maximized: store::int(cx.store, cx.id, "childstate", 0) == 2,
            buttons: vec![(Button::Close, true), (Button::Maximize, true), (Button::Minimize, true)],
            font: Font { styles: base.styles | 1, ..base },
            ..Chrome::default()
        };
        window_frame::paint(p, (cx.width(), cx.height()), &chrome, &Metrics::mdi());
    }

    /// A child's sizing border, every edge and corner as Windows': ↔ ↕ ⤡ ⤢
    /// — kept while one is being dragged, wherever the mouse goes. (A
    /// maximized or minimized child has none; its title bar, being moved,
    /// keeps the arrow.)
    fn pointer(&self, cx: &mut Cx, x: i64, y: i64) -> Cursor {
        let held = DRAG.with(|d| d.borrow().as_ref().filter(|d| d.frame == cx.id).map(|d| d.edges));
        let edges = match held {
            Some(edges) => edges,
            None => edges_at(cx.store, cx.id, cx.width(), cx.height(), x, y),
        };
        match edges.map(|e| e.pointer()) {
            Some("we") => Cursor::SizeWE,
            Some("ns") => Cursor::SizeNS,
            Some("nwse") => Cursor::SizeNWSE,
            Some(_) => Cursor::SizeNESW,
            None => Cursor::Default,
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
                // (the input lane's: a double click on the title bar maximizes
                // or restores it, as Windows' frame)
                if y < BORDER + TITLE_HEIGHT && m.double() {
                    act(cx, Action::ToggleMaximize);
                    return MouseOut { press: false, focus: Some(false) };
                }
                act(cx, Action::Activate);
                let edges = edges_at(cx.store, cx.id, w, h, x, y);
                if edges.is_some() || y < BORDER + TITLE_HEIGHT {
                    let start = (store::int(cx.store, cx.id, "left", 0), store::int(cx.store, cx.id, "top", 0), w, h);
                    DRAG.with(|d| *d.borrow_mut() = Some(Drag { frame: cx.id.to_string(), edges, mouse: abs(m), start }));
                }
            }
            MouseKind::Move if m.captured => {
                let Some((edges, from, start)) = DRAG.with(|d| d.borrow().as_ref().filter(|d| d.frame == cx.id).map(|d| (d.edges, d.mouse, d.start))) else {
                    return MouseOut::default();
                };
                let (ax, ay) = abs(m);
                let (dx, dy) = ((ax - from.0).round() as i64, (ay - from.1).round() as i64);
                act(cx, match edges {
                    Some(e) => {
                        let (l, t, w, h) = mdi::resized(start, e, dx, dy);
                        Action::Resize(l, t, w, h)
                    }
                    None => Action::Move(start.0 + dx, start.1 + dy),
                });
            }
            MouseKind::Up => DRAG.with(|d| *d.borrow_mut() = None),
            _ => {}
        }
        MouseOut { press: false, focus: Some(false) }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        super::shared_describe(cx, self.name())
    }
}
