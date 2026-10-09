//! A button dragged (`rapidr_value::drag` has what RapidQ's runtime does):
//!
//! - a **drag source** — a QBUTTON with an OnStartDrag handler: a left
//!   press on it fires OnStartDrag instead of OnMouseDown (no focus, no
//!   pushed look), the mouse shows the no-drop pointer, and the release —
//!   or Escape — fires OnEndDrag instead of OnMouseUp and OnClick;
//! - a **move** — `StartDrag` while a mouse button is held: the control
//!   follows the mouse (its Left / Top, as the program sets them) until
//!   the release, which nothing else hears; Escape puts it back.

use crate::input::KernelEvent;
use crate::store::Store;
use crate::tree::FormUi;

/// A drag in progress.
#[derive(Clone, Debug, PartialEq)]
pub enum Drag {
    /// OnStartDrag's drag of `id`, until the release.
    Source { id: String },
    /// `StartDrag`'s move of `id`: where the mouse was (the window's
    /// inside) and the control's Left / Top then, and where it is now.
    Move { id: String, from: (f64, f64), start: (i64, i64), at: (i64, i64) },
}

impl FormUi {
    /// A left press on node `i`: a drag source's drag starts (OnStartDrag).
    /// Whether it did (the press is the drag's, nothing else's).
    pub(crate) fn drag_press(&mut self, store: &dyn Store, i: usize) -> bool {
        let n = &self.nodes[i];
        if n.type_name != "RBUTTON" || !rapidr_value::drag::is_source(&n.id) || store.type_of(&n.id).is_empty() {
            return false;
        }
        let id = n.id.clone();
        self.capture = Some(Some(i));
        self.pressed = None;
        self.drag = Some(Drag::Source { id: id.clone() });
        self.events.push(KernelEvent::Fire { id, event: "onstartdrag".into(), args: Vec::new() });
        self.dirty = true;
        true
    }

    /// `X.StartDrag`: control `id` moves with the mouse while a button is
    /// held; whether it does (no button held: nothing happens).
    pub fn start_move(&mut self, id: &str) -> bool {
        if self.capture.is_none() || self.drag.is_some() {
            return false;
        }
        let Some(i) = self.index_of(id) else { return false };
        let (left, top, _, _) = self.nodes[i].rect;
        self.pressed = None;
        self.capture = Some(Some(i));
        self.drag = Some(Drag::Move { id: self.nodes[i].id.clone(), from: self.mouse_at, start: (left, top), at: (left, top) });
        self.dirty = true;
        true
    }

    /// Whether a drag or move is in progress (StartDrag's wait lasts that
    /// long).
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Whether a drag source is being dragged (the no-drop pointer).
    pub fn dragging_source(&self) -> bool {
        matches!(self.drag, Some(Drag::Source { .. }))
    }

    /// The mouse moved to (x, y) during a drag: a moved control follows.
    /// Whether a drag took the move.
    pub(crate) fn drag_move(&mut self, x: f64, y: f64) -> bool {
        let Some(drag) = self.drag.as_mut() else { return false };
        if let Drag::Move { id, from, start, at } = drag {
            let to = (start.0 + (x - from.0).round() as i64, start.1 + (y - from.1).round() as i64);
            if to != *at {
                *at = to;
                let id = id.clone();
                self.move_to(&id, to);
            }
        }
        true
    }

    /// Control `id`'s Left / Top set as the user moved it (the program's
    /// store gets them after the pump, as a property the user changed).
    fn move_to(&mut self, id: &str, (left, top): (i64, i64)) {
        self.events.push(KernelEvent::Set { id: id.to_string(), prop: "left".into(), value: left });
        self.events.push(KernelEvent::Set { id: id.to_string(), prop: "top".into(), value: top });
        // (drawn there at once: the store follows after the pump)
        if let Some(i) = self.index_of(id) {
            let n = &mut self.nodes[i];
            let (dx, dy) = (left - n.rect.0, top - n.rect.1);
            n.rect.0 = left;
            n.rect.1 = top;
            n.abs.0 += dx;
            n.abs.1 += dy;
        }
        self.dirty = true;
    }

    /// The mouse let go during a drag: it ends (a source's OnEndDrag).
    /// Whether a drag took the release.
    pub(crate) fn drag_release(&mut self) -> bool {
        let Some(drag) = self.drag.take() else { return false };
        self.capture = None;
        self.pressed = None;
        self.dirty = true;
        if let Drag::Source { id } = drag {
            self.events.push(KernelEvent::Fire { id, event: "onenddrag".into(), args: Vec::new() });
        }
        true
    }

    /// Escape during a drag: a source's drag is cancelled (OnEndDrag all
    /// the same), a moved control goes back. Whether a drag took the key.
    pub(crate) fn drag_escape(&mut self) -> bool {
        match self.drag.clone() {
            None => false,
            Some(Drag::Move { id, start, at, .. }) => {
                if at != start {
                    self.move_to(&id, start);
                }
                self.drag = None;
                self.capture = None;
                true
            }
            Some(Drag::Source { .. }) => self.drag_release(),
        }
    }
}

