//! QSPLITTER: a raised bar between aligned components; dragging it
//! resizes the component it sits against (runtime-core's
//! `layout::splitter_*`, host-neutral: the kernel queues the drag as
//! [`Container`] events — begin, the distance from the press, end with
//! OnMoved — and runtime-core lays the form out after the pump). The
//! distance is taken in the window, so it holds while the splitter itself
//! moves under the mouse.

use std::cell::Cell;

use rapidr_value::objects::a11y::{node_id, AccessNode, Orientation, Role};

use super::form::Container;
use super::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::input::KernelEvent;
use crate::paint::{Painter, FACE, LIGHT, SHADOW};
use crate::store::{self, Store};

pub struct Splitter;

thread_local! {
    /// Where the splitter being dragged was pressed (in the window: along
    /// its axis).
    static FROM: Cell<Option<f64>> = const { Cell::new(None) };
}

/// It moves up and down (Align alTop 1 / alBottom 2), else left and right.
pub fn vertical(store: &dyn Store, id: &str) -> bool {
    matches!(store::int(store, id, "align", 3), 1 | 2)
}

impl ComponentKind for Splitter {
    fn name(&self) -> &'static str {
        "RSPLITTER"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        false
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        p.fill((0, 0, w, h), FACE);
        p.edge((0, 0, w, h), &[LIGHT], &[SHADOW]);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let along = |m: &MouseIn| if vertical(cx.store, cx.id) { m.y + cx.rect.1 as f64 } else { m.x + cx.rect.0 as f64 };
        match m.kind {
            MouseKind::Down => {
                FROM.with(|f| f.set(Some(along(m))));
                cx.events.push(KernelEvent::Container(Container::SplitBegin(cx.id.to_string())));
            }
            MouseKind::Move if m.captured => {
                if let Some(from) = FROM.with(Cell::get) {
                    let delta = (along(m) - from).round() as i64;
                    cx.events.push(KernelEvent::Container(Container::SplitMove(delta)));
                }
            }
            MouseKind::Up => {
                if FROM.with(|f| f.take()).is_some() {
                    cx.events.push(KernelEvent::Container(Container::SplitEnd));
                }
            }
            _ => {}
        }
        MouseOut { press: false, focus: Some(false) }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::Unknown);
        n.orientation = Some(if vertical(cx.store, cx.id) { Orientation::Vertical } else { Orientation::Horizontal });
        n.bounds = cx.rect;
        n
    }
}
