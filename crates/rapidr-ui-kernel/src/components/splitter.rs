//! QSPLITTER: a raised bar between aligned components; dragging it
//! resizes the component it sits against (runtime-core's
//! `layout::splitter_*`, host-neutral: the kernel queues the drag as
//! [`Container`] events — begin, the distance from the press, end with
//! OnMoved — and runtime-core lays the form out after the pump). The
//! distance is taken in the window, so it holds while the splitter itself
//! moves under the mouse.

use std::cell::Cell;

use rapidr_value::input::Cursor;
use rapidr_value::objects::a11y::AccessNode;

use super::form::Container;
use super::{ComponentKind, Cx, MouseIn, MouseKind, MouseOut};
use crate::input::KernelEvent;
use crate::paint::Painter;
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
        let t = p.theme();
        p.fill((0, 0, w, h), t.face);
        if !t.fluent() {
            // (the classic look: RapidQ's raised bar)
            p.thin_raised((0, 0, w, h));
            return;
        }
        // (RapidR's look: the dock manager's splitters' — a line along the
        // middle and a grip at rest, the accent's under the mouse or held:
        // it says "drag me". Its place and size are the program's.)
        use rapidr_value::dock::look::{palette, splitter_ops, SplitterLook};
        let look = if cx.state.hover || cx.state.held { SplitterLook::Hot } else { SplitterLook::Rest };
        let mut ops = Vec::new();
        splitter_ops(&mut ops, (0, 0, w, h), !vertical(cx.store, cx.id), &palette(t), look);
        p.ops(ops);
    }

    /// Between two columns (or two rows): the resize pointer, `crHSplit` /
    /// `crVSplit`.
    fn pointer(&self, cx: &mut Cx, _x: i64, _y: i64) -> Cursor {
        if vertical(cx.store, cx.id) {
            Cursor::RowResize
        } else {
            Cursor::ColResize
        }
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
        super::shared_describe(cx, self.name())
    }
}
