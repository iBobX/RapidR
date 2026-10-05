//! A QFORM's and a QSCROLLBOX's scroll bars (RapidQ manual: AutoScroll,
//! HorzIncrement / HorzMargin / HorzPosition / HorzRange / HorzTracking /
//! HorzVisible and their Vert… twins), as Delphi's TScrollingWinControl
//! has them — shared by the desktop and web runtimes, which draw the same
//! [`Scroller::ops`] and move the same components.
//!
//! With AutoScroll a bar's Range reaches the farthest visible component
//! (one not aligned, or aligned alLeft / alTop; alRight / alBottom ones add
//! their size) plus the Margin; a bar shows when its Range is more than the
//! client area (the other bar, once shown, takes 17 pixels of it). Scrolling
//! moves the components (Delphi's ScrollBy): a component's Left / Top are
//! where it is now in the client area. Position stays within
//! 0..Range - client size; with no bar it's 0. The arrows move Increment
//! pixels, a click in the track a page (Delphi's PageIncrement, 80), the
//! thumb drags (with Tracking the components follow it, without it they
//! move when it's let go); the wheel scrolls three Increments.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::layout::Align;
use crate::objects::tabcontrol::Op;
use crate::{v_int, Value};

/// A scroll bar's thickness (Windows' SM_CXVSCROLL).
pub const BAR: i64 = 17;
/// A click in the track moves this far (Delphi's PageIncrement).
const PAGE: i64 = 80;
/// The thumb's smallest length.
const MIN_THUMB: i64 = 8;
/// Largest range kept (a component far away can't make it overflow).
const MAX_RANGE: i64 = 1 << 30;


#[derive(Clone, Debug)]
pub struct Axis {
    pub increment: i64,
    pub margin: i64,
    pub position: i64,
    pub range: i64,
    pub tracking: bool,
    pub visible: bool,
    /// Shown now (its Range is more than the client area).
    pub shown: bool,
    /// A Position the program set, applied (kept in range, the components
    /// moved) by the next [`Scroller::update`].
    pending: Option<i64>,
}

impl Default for Axis {
    fn default() -> Self {
        Axis { increment: 8, margin: 0, position: 0, range: 0, tracking: false, visible: true, shown: false, pending: None }
    }
}

/// A part of a bar the mouse is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Back,
    Forward,
    PageBack,
    PageForward,
    Thumb,
}

#[derive(Clone, Debug)]
struct Drag {
    vertical: bool,
    /// Where the mouse went down (along the bar) and the position then.
    from: i64,
    start: i64,
    /// The position the thumb shows while dragged (without Tracking the
    /// components stay until it's let go).
    at: i64,
}

#[derive(Clone, Debug)]
pub struct Scroller {
    pub auto: bool,
    pub horz: Axis,
    pub vert: Axis,
    /// The part held down (vertical bar?, part): it repeats while held.
    pub pressed: Option<(bool, Part)>,
    drag: Option<Drag>,
    pub revision: u64,
}

impl Default for Scroller {
    fn default() -> Self {
        Scroller { auto: true, horz: Axis::default(), vert: Axis::default(), pressed: None, drag: None, revision: 0 }
    }
}

/// A component inside (in the client area's coordinates, as scrolled now).
#[derive(Clone, Copy, Debug)]
pub struct Child {
    pub left: i64,
    pub top: i64,
    pub width: i64,
    pub height: i64,
    pub align: Align,
    pub visible: bool,
}

/// How far the components must move: (dx, dy) — each component's Left
/// changes by -dx, Top by -dy.
pub type Shift = (i64, i64);

/// A bar's rectangle (x, y, w, h) in the container's client coordinates.
type Rect = (i64, i64, i64, i64);

impl Scroller {
    fn changed(&mut self) {
        self.revision += 1;
    }

    fn axis(&self, vertical: bool) -> &Axis {
        if vertical { &self.vert } else { &self.horz }
    }

    fn axis_mut(&mut self, vertical: bool) -> &mut Axis {
        if vertical { &mut self.vert } else { &mut self.horz }
    }

    /// The client area of a `w` × `h` area once the shown bars are taken
    /// off (ClientWidth / ClientHeight).
    pub fn client(&self, w: i64, h: i64) -> (i64, i64) {
        ((w - if self.vert.shown { BAR } else { 0 }).max(0), (h - if self.horz.shown { BAR } else { 0 }).max(0))
    }

    /// Works out the ranges (AutoScroll) and which bars show, for a `w` ×
    /// `h` area and its components; then keeps the positions in range.
    /// Returns how far the components must move.
    pub fn update(&mut self, w: i64, h: i64, children: &[Child]) -> Shift {
        if self.auto {
            let (mut right, mut bottom, mut right_edge, mut bottom_edge) = (0i64, 0i64, 0i64, 0i64);
            for c in children.iter().filter(|c| c.visible) {
                match c.align {
                    Align::None | Align::Left => right = right.max(self.horz.position + c.left + c.width),
                    Align::Right => right_edge += c.width,
                    _ => {}
                }
                match c.align {
                    Align::None | Align::Top => bottom = bottom.max(self.vert.position + c.top + c.height),
                    Align::Bottom => bottom_edge += c.height,
                    _ => {}
                }
            }
            self.horz.range = (right + right_edge + self.horz.margin).clamp(0, MAX_RANGE);
            self.vert.range = (bottom + bottom_edge + self.vert.margin).clamp(0, MAX_RANGE);
        }
        // (each bar shown takes room from the other's side)
        let mut v = self.vert.visible && self.vert.range > h;
        let hz = self.horz.visible && self.horz.range > w - if v { BAR } else { 0 };
        if hz && !v {
            v = self.vert.visible && self.vert.range > h - BAR;
        }
        if (v, hz) != (self.vert.shown, self.horz.shown) {
            self.vert.shown = v;
            self.horz.shown = hz;
            self.changed();
        }
        self.clamp(w, h)
    }

    /// The furthest a position can go on an axis.
    fn max_position(&self, vertical: bool, w: i64, h: i64) -> i64 {
        let (cw, ch) = self.client(w, h);
        let a = self.axis(vertical);
        if !a.shown {
            return 0;
        }
        (a.range - if vertical { ch } else { cw }).max(0)
    }

    /// Positions kept in range: how far the components must move.
    fn clamp(&mut self, w: i64, h: i64) -> Shift {
        let mut shift = (0, 0);
        for vertical in [false, true] {
            let max = self.max_position(vertical, w, h);
            let a = self.axis_mut(vertical);
            let p = a.pending.take().unwrap_or(a.position).clamp(0, max);
            let d = p - a.position;
            a.position = p;
            if vertical { shift.1 = d } else { shift.0 = d }
        }
        if shift != (0, 0) {
            self.changed();
        }
        shift
    }

    /// Scrolls an axis to `to` (kept in range): how far the components move.
    fn scroll_to(&mut self, vertical: bool, to: i64, w: i64, h: i64) -> Shift {
        let max = self.max_position(vertical, w, h);
        let a = self.axis_mut(vertical);
        let to = to.clamp(0, max);
        let d = to - a.position;
        a.position = to;
        if d != 0 {
            self.changed();
        }
        if vertical { (0, d) } else { (d, 0) }
    }

    /// A bar's rectangle, if it shows (a theme drawing its own bars).
    pub fn bar(&self, vertical: bool, w: i64, h: i64) -> Option<Rect> {
        let (cw, ch) = self.client(w, h);
        if vertical {
            self.vert.shown.then_some((cw, 0, BAR, ch))
        } else {
            self.horz.shown.then_some((0, ch, cw, BAR))
        }
    }

    /// The thumb's start and length along a bar `len` long (None: no room).
    pub fn thumb(&self, vertical: bool, len: i64, w: i64, h: i64) -> Option<(i64, i64)> {
        let track = len - 2 * BAR;
        if track < MIN_THUMB {
            return None;
        }
        let (cw, ch) = self.client(w, h);
        let page = if vertical { ch } else { cw };
        let a = self.axis(vertical);
        let range = a.range.max(1);
        let size = (track * page / range).clamp(MIN_THUMB, track);
        let max = self.max_position(vertical, w, h);
        let pos = match &self.drag {
            Some(d) if d.vertical == vertical => d.at,
            _ => a.position,
        };
        let at = if max > 0 { (track - size) * pos.clamp(0, max) / max } else { 0 };
        Some((BAR + at, size))
    }

    /// The bar part at a point of the client area (with the bars).
    fn part_at(&self, x: i64, y: i64, w: i64, h: i64) -> Option<(bool, Part)> {
        for vertical in [true, false] {
            let Some((bx, by, bw, bh)) = self.bar(vertical, w, h) else { continue };
            if x < bx || x >= bx + bw || y < by || y >= by + bh {
                continue;
            }
            let (along, len) = if vertical { (y - by, bh) } else { (x - bx, bw) };
            let part = if along < BAR {
                Part::Back
            } else if along >= len - BAR {
                Part::Forward
            } else {
                match self.thumb(vertical, len, w, h) {
                    Some((t, _)) if along < t => Part::PageBack,
                    Some((t, s)) if along >= t + s => Part::PageForward,
                    Some(_) => Part::Thumb,
                    None => return Some((vertical, Part::PageForward)),
                }
            };
            return Some((vertical, part));
        }
        None
    }

    /// Whether (x, y) is on a bar (or the corner between them).
    pub fn on_bars(&self, x: i64, y: i64, w: i64, h: i64) -> bool {
        let (cw, ch) = self.client(w, h);
        (self.vert.shown || self.horz.shown) && x < w && y < h && (x >= cw || y >= ch)
    }

    /// What a part does once (an arrow, a page).
    fn step(&mut self, vertical: bool, part: Part, w: i64, h: i64) -> Shift {
        let a = self.axis(vertical);
        let to = match part {
            Part::Back => a.position - a.increment,
            Part::Forward => a.position + a.increment,
            Part::PageBack => a.position - PAGE,
            Part::PageForward => a.position + PAGE,
            Part::Thumb => return (0, 0),
        };
        self.scroll_to(vertical, to, w, h)
    }

    /// The mouse pressed at (x, y): `None` if not on a bar; else how far
    /// the components move (an arrow or a page scrolls at once, and again
    /// while held: [`Scroller::repeat`]; the thumb starts a drag).
    pub fn mouse_down(&mut self, x: i64, y: i64, w: i64, h: i64) -> Option<Shift> {
        if !self.on_bars(x, y, w, h) {
            return None;
        }
        let Some((vertical, part)) = self.part_at(x, y, w, h) else { return Some((0, 0)) };
        self.pressed = Some((vertical, part));
        self.changed();
        if part == Part::Thumb {
            let a = self.axis(vertical);
            self.drag = Some(Drag { vertical, from: if vertical { y } else { x }, start: a.position, at: a.position });
            return Some((0, 0));
        }
        Some(self.step(vertical, part, w, h))
    }

    /// The part held still does its step again (while the mouse stays on
    /// it, as Windows' bars repeat): how far the components move.
    pub fn repeat(&mut self, x: i64, y: i64, w: i64, h: i64) -> Shift {
        match self.pressed {
            Some((vertical, part)) if part != Part::Thumb && self.part_at(x, y, w, h) == Some((vertical, part)) => self.step(vertical, part, w, h),
            _ => (0, 0),
        }
    }

    /// The mouse dragged to (x, y): the thumb follows (the components too,
    /// with Tracking).
    pub fn mouse_drag(&mut self, x: i64, y: i64, w: i64, h: i64) -> Shift {
        let Some(d) = self.drag.clone() else { return (0, 0) };
        let Some((_, _, bw, bh)) = self.bar(d.vertical, w, h) else { return (0, 0) };
        let len = if d.vertical { bh } else { bw };
        let Some((_, size)) = self.thumb(d.vertical, len, w, h) else { return (0, 0) };
        let room = (len - 2 * BAR - size).max(1);
        let max = self.max_position(d.vertical, w, h);
        let moved = (if d.vertical { y } else { x }) - d.from;
        let to = (d.start + moved * max / room).clamp(0, max);
        if let Some(dr) = self.drag.as_mut() {
            if dr.at != to {
                dr.at = to;
                self.revision += 1;
            }
        }
        if self.axis(d.vertical).tracking {
            self.scroll_to(d.vertical, to, w, h)
        } else {
            (0, 0)
        }
    }

    /// The mouse let go: a dragged thumb's place is where the components
    /// go (without Tracking).
    pub fn mouse_up(&mut self, w: i64, h: i64) -> Shift {
        let drag = self.drag.take();
        if self.pressed.take().is_some() {
            self.changed();
        }
        match drag {
            Some(d) => self.scroll_to(d.vertical, d.at, w, h),
            None => (0, 0),
        }
    }

    /// The wheel turned `notches` (positive: down / right): three
    /// Increments each, the vertical bar if shown, else the horizontal one.
    pub fn wheel(&mut self, notches: i64, horizontal: bool, w: i64, h: i64) -> Shift {
        let vertical = !horizontal && self.vert.shown;
        if !self.axis(vertical).shown {
            return (0, 0);
        }
        let a = self.axis(vertical);
        let to = a.position + notches.clamp(-100, 100) * 3 * a.increment;
        self.scroll_to(vertical, to, w, h)
    }

    /// What to draw: the bars (and the corner between them) of a `w` × `h`
    /// area, in its coordinates — Windows' classic bars in the current
    /// theme's colours (`crate::theme`; a fluent theme draws its own thin
    /// ones from [`Scroller::bar`] and [`Scroller::thumb`]).
    pub fn ops(&self, w: i64, h: i64) -> Vec<Op> {
        let th = crate::theme::current();
        let mut out = Vec::new();
        let (cw, ch) = self.client(w, h);
        if self.vert.shown && self.horz.shown {
            out.push(Op::Fill { rect: (cw, ch, BAR, BAR), color: th.face });
        }
        for vertical in [false, true] {
            let Some((bx, by, bw, bh)) = self.bar(vertical, w, h) else { continue };
            let pressed = |p: Part| self.pressed == Some((vertical, p));
            out.push(Op::Fill { rect: (bx, by, bw, bh), color: th.track });
            let len = if vertical { bh } else { bw };
            let at = |along: i64, size: i64| if vertical { (bx, by + along, BAR, size) } else { (bx + along, by, size, BAR) };
            // the page parts held down show darker
            if let Some((t, s)) = self.thumb(vertical, len, w, h) {
                if pressed(Part::PageBack) {
                    out.push(Op::Fill { rect: at(BAR, t - BAR), color: th.track_pressed });
                }
                if pressed(Part::PageForward) {
                    out.push(Op::Fill { rect: at(t + s, len - BAR - t - s), color: th.track_pressed });
                }
                button(&mut out, at(t, s), false);
            }
            for (part, along) in [(Part::Back, 0), (Part::Forward, len - BAR)] {
                let r = at(along, BAR.min(len));
                let down = pressed(part);
                button(&mut out, r, down);
                let (x, y, rw, rh) = r;
                let (cx, cy) = (x as f64 + rw as f64 / 2.0 + f64::from(u8::from(down)), y as f64 + rh as f64 / 2.0 + f64::from(u8::from(down)));
                let s = 3.5;
                let points = match (vertical, part == Part::Forward) {
                    (false, false) => [(cx + s / 2.0, cy - s), (cx + s / 2.0, cy + s), (cx - s / 2.0 - 1.0, cy)],
                    (false, true) => [(cx - s / 2.0, cy - s), (cx - s / 2.0, cy + s), (cx + s / 2.0 + 1.0, cy)],
                    (true, false) => [(cx - s, cy + s / 2.0), (cx + s, cy + s / 2.0), (cx, cy - s / 2.0 - 1.0)],
                    (true, true) => [(cx - s, cy - s / 2.0), (cx + s, cy - s / 2.0), (cx, cy + s / 2.0 + 1.0)],
                };
                out.push(Op::Arrow { points, color: th.text });
            }
        }
        out
    }

    /// The program reading AutoScroll, HorzPosition, … (`None`: not one).
    pub fn get(&self, prop: &str) -> Option<Value> {
        let flag = |b: bool| v_int(if b { -1 } else { 0 });
        if prop == "autoscroll" {
            return Some(flag(self.auto));
        }
        let (a, rest) = if let Some(r) = prop.strip_prefix("horz") {
            (&self.horz, r)
        } else if let Some(r) = prop.strip_prefix("vert") {
            (&self.vert, r)
        } else {
            return None;
        };
        Some(match rest {
            "increment" => v_int(a.increment),
            "margin" => v_int(a.margin),
            "position" => v_int(a.pending.unwrap_or(a.position)),
            "range" => v_int(a.range),
            "tracking" => flag(a.tracking),
            "visible" => flag(a.visible),
            _ => return None,
        })
    }

    /// The program setting one: `None` if not one; else whether the bars
    /// must be worked out again (and the position applied: the runtime
    /// calls [`Scroller::update`], then moves the components).
    pub fn set(&mut self, prop: &str, val: &Value) -> Option<()> {
        let n = val.to_i64();
        if prop == "autoscroll" {
            self.auto = val.to_bool();
        } else {
            let (vertical, rest) = if let Some(r) = prop.strip_prefix("horz") {
                (false, r)
            } else if let Some(r) = prop.strip_prefix("vert") {
                (true, r)
            } else {
                return None;
            };
            let auto = self.auto;
            let a = self.axis_mut(vertical);
            match rest {
                "increment" => a.increment = n.clamp(1, 32_767),
                "margin" => a.margin = n.clamp(0, MAX_RANGE),
                // (kept in range, the components moved: by `update`)
                "position" => a.pending = Some(n),
                // (AutoScroll works it out again)
                "range" if !auto => a.range = n.clamp(0, MAX_RANGE),
                "range" => {}
                "tracking" => a.tracking = val.to_bool(),
                "visible" => a.visible = val.to_bool(),
                _ => return None,
            }
        }
        self.changed();
        Some(())
    }
}

/// A raised button (pushed: flat with a shadow line), as the bars' parts.
fn button(out: &mut Vec<Op>, r: Rect, pushed: bool) {
    let (x, y, w, h) = r;
    if w <= 0 || h <= 0 {
        return;
    }
    let th = crate::theme::current();
    let (face, light, shadow, dark) = (th.face, th.light, th.shadow, th.dark_shadow);
    out.push(Op::Fill { rect: r, color: face });
    if pushed {
        for (rr, c) in [((x, y, w, 1), shadow), ((x, y, 1, h), shadow), ((x, y + h - 1, w, 1), shadow), ((x + w - 1, y, 1, h), shadow)] {
            out.push(Op::Fill { rect: rr, color: c });
        }
        return;
    }
    for (rr, c) in [
        ((x, y, w, 1), face),
        ((x, y, 1, h), face),
        ((x + 1, y + 1, w - 2, 1), light),
        ((x + 1, y + 1, 1, h - 2), light),
        ((x, y + h - 1, w, 1), dark),
        ((x + w - 1, y, 1, h), dark),
        ((x + 1, y + h - 2, w - 2, 1), shadow),
        ((x + w - 2, y + 1, 1, h - 2), shadow),
    ] {
        if rr.2 > 0 && rr.3 > 0 {
            out.push(Op::Fill { rect: rr, color: c });
        }
    }
}

thread_local! {
    /// Each scrolling container's bars (lowercase names).
    static SCROLLERS: RefCell<HashMap<String, Scroller>> = RefCell::new(HashMap::new());
}

/// Whether components of this type scroll (QFORM, QSCROLLBOX).
pub fn scrolls(type_name: &str) -> bool {
    matches!(type_name.to_ascii_uppercase().as_str(), "RFORM" | "RSCROLLBOX")
}

/// A container's bars, to read (`None`: it has none yet).
pub fn with<R>(name: &str, f: impl FnOnce(&Scroller) -> R) -> Option<R> {
    SCROLLERS.with(|s| s.borrow().get(&name.to_lowercase()).map(f))
}

/// A container's bars, to change (made the first time).
pub fn with_mut<R>(name: &str, f: impl FnOnce(&mut Scroller) -> R) -> R {
    SCROLLERS.with(|s| f(s.borrow_mut().entry(name.to_lowercase()).or_default()))
}

/// A container gone: its bars too.
pub fn remove(name: &str) {
    SCROLLERS.with(|s| {
        s.borrow_mut().remove(&name.to_lowercase());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child(left: i64, top: i64, width: i64, height: i64) -> Child {
        Child { left, top, width, height, align: Align::None, visible: true }
    }

    #[test]
    fn auto_range_bars_and_positions() {
        let mut s = Scroller::default();
        // everything fits: no bars
        assert_eq!(s.update(300, 200, &[child(10, 10, 100, 50)]), (0, 0));
        assert!(!s.vert.shown && !s.horz.shown);
        assert_eq!(s.client(300, 200), (300, 200));
        // a button below the bottom: a vertical bar, the client narrower
        let kids = [child(10, 10, 100, 50), child(10, 400, 80, 25)];
        s.update(300, 200, &kids);
        assert!(s.vert.shown && !s.horz.shown);
        assert_eq!((s.vert.range, s.client(300, 200)), (425, (283, 200)));
        // scrolled to the end; then the components move back when it fits
        s.set("vertposition", &v_int(10_000));
        assert_eq!(s.update(300, 200, &kids), (0, 225));
        assert_eq!(s.get("vertposition").unwrap().to_i64(), 225);
        let moved = [child(10, 10 - 225, 100, 50)];
        assert_eq!(s.update(300, 200, &moved), (0, -225), "no bar: position 0");
        // a wide one too: both bars, each taking room from the other
        let wide = [child(0, 0, 290, 195)];
        s.update(300, 200, &wide);
        assert!(!s.vert.shown && !s.horz.shown);
        let wide = [child(0, 0, 310, 190)];
        s.update(300, 200, &wide);
        assert!(s.horz.shown && s.vert.shown, "the horizontal bar leaves 183 high: 190 doesn't fit");
        // alRight adds its width; aligned alClient ones don't count
        let mut s = Scroller::default();
        let kids = [Child { align: Align::Client, ..child(0, 0, 900, 900) }, Child { align: Align::Right, ..child(0, 0, 50, 10) }, child(280, 0, 10, 10)];
        s.update(300, 200, &kids);
        assert_eq!(s.horz.range, 340);
    }

    #[test]
    fn mouse_and_tracking() {
        let mut s = Scroller::default();
        let kids = [child(0, 0, 100, 1000)];
        s.update(300, 200, &kids);
        // the down arrow: an Increment; the track below the thumb: a page
        assert_eq!(s.mouse_down(290, 195, 300, 200), Some((0, 8)));
        assert_eq!(s.repeat(290, 195, 300, 200), (0, 8));
        assert_eq!(s.mouse_up(300, 200), (0, 0));
        assert_eq!(s.mouse_down(290, 150, 300, 200), Some((0, 80)));
        s.mouse_up(300, 200);
        assert_eq!(s.mouse_down(100, 100, 300, 200), None, "not on a bar");
        // the thumb: without Tracking the components move when let go
        let (t, _) = s.thumb(true, 200, 300, 200).unwrap();
        assert_eq!(s.mouse_down(290, t + 2, 300, 200), Some((0, 0)));
        assert_eq!(s.mouse_drag(290, t + 52, 300, 200), (0, 0));
        let (dx, dy) = s.mouse_up(300, 200);
        assert!(dx == 0 && dy > 0);
        // with Tracking they follow
        s.set("verttracking", &v_int(1));
        let (t, _) = s.thumb(true, 200, 300, 200).unwrap();
        s.mouse_down(290, t + 2, 300, 200);
        assert!(s.mouse_drag(290, t + 22, 300, 200).1 > 0);
        s.mouse_up(300, 200);
        assert_eq!(s.wheel(-1, false, 300, 200).1, -24);
        assert!(!s.ops(300, 200).is_empty());
    }
}
