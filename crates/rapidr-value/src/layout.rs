//! `Align` (RapidQ manual, Appendix A: "Align determines how the control
//! aligns within its parent control"): RapidQ's components are Delphi VCL
//! controls, so aligned children are laid out as Delphi's
//! `TWinControl.AlignControls` does. Both runtimes call [`align_controls`]
//! with a container's client rectangle and its children, and move the
//! children to the rectangles it returns.
//!
//! * The aligns are handled in the order alTop, alBottom, alLeft, alRight,
//!   alClient. Each control takes a strip off the edge of what's left of the
//!   client area — keeping its height (alTop / alBottom) or width (alLeft /
//!   alRight) — and alClient controls fill what remains.
//! * Controls with the same align are ordered by position (for alTop, the
//!   one nearer the top first; for alBottom, the one nearer the bottom
//!   first; likewise left / right). Among equal positions, creation order
//!   for alTop / alLeft and the reverse for alBottom / alRight (Delphi
//!   compares with `<` and `>=`). The control whose Align, size or
//!   visibility just changed comes first. So `Splitter (alLeft)` created before `Tree (alLeft)` ends
//!   up to the right of the tree, as in RapidQ.
//! * Invisible controls and those with alNone are left alone.

/// RAPIDQ.INC: `alNone = 0`, `alTop = 1`, `alBottom = 2`, `alLeft = 3`,
/// `alRight = 4`, `alClient = 5`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    None,
    Top,
    Bottom,
    Left,
    Right,
    Client,
}

impl Align {
    pub fn from_value(n: i64) -> Align {
        match n {
            1 => Align::Top,
            2 => Align::Bottom,
            3 => Align::Left,
            4 => Align::Right,
            5 => Align::Client,
            _ => Align::None,
        }
    }

    pub fn value(self) -> i64 {
        self as i64
    }
}

/// The Align a component type starts with (QSTATUSBAR docks at the bottom,
/// QSPLITTER at the left, as in RapidQ), for RapidR's type names.
pub fn default_align(type_name: &str) -> Align {
    match type_name.to_ascii_uppercase().as_str() {
        "RSTATUSBAR" => Align::Bottom,
        "RSPLITTER" => Align::Left,
        _ => Align::None,
    }
}

/// A form's frame, the same in both runtimes. As in RapidQ (Delphi),
/// `Width` / `Height` are the whole window — caption and borders included —
/// and `ClientWidth` / `ClientHeight` the area inside, below the main menu:
/// a 29px caption, a 1px border on every side, and no frame at all with
/// `BorderStyle = bsNone` (0). The desktop's window manager draws its own
/// frame, so there the frame is only accounted for, keeping a form's inside
/// the size it has in the browser.
pub const FORM_CAPTION: i64 = 29;
pub const FORM_BORDER: i64 = 1;

/// The frame around a form's inside: (left + right, caption + top +
/// bottom), for its BorderStyle.
pub fn form_frame(border_style: i64) -> (i64, i64) {
    if border_style == 0 {
        (0, 0)
    } else {
        (2 * FORM_BORDER, FORM_CAPTION + 2 * FORM_BORDER)
    }
}

/// A form's client size for its Width / Height, BorderStyle and the height
/// of its main menu.
pub fn form_client_size(width: i64, height: i64, border_style: i64, menu: i64) -> (i64, i64) {
    let (fw, fh) = form_frame(border_style);
    ((width - fw).max(0), (height - fh - menu).max(0))
}

/// The Width / Height that give a form this client size (ClientWidth =,
/// ClientHeight =).
pub fn form_outer_size(client_width: i64, client_height: i64, border_style: i64, menu: i64) -> (i64, i64) {
    let (fw, fh) = form_frame(border_style);
    (client_width.max(0) + fw, client_height.max(0) + fh + menu)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub left: i64,
    pub top: i64,
    pub width: i64,
    pub height: i64,
}

impl Rect {
    pub fn new(left: i64, top: i64, width: i64, height: i64) -> Rect {
        Rect { left, top, width, height }
    }

    fn right(&self) -> i64 {
        self.left + self.width
    }

    fn bottom(&self) -> i64 {
        self.top + self.height
    }
}

/// A child of the container being laid out.
#[derive(Clone, Copy, Debug)]
pub struct Control {
    pub align: Align,
    pub visible: bool,
    pub rect: Rect,
}

/// Lays out `controls` (in creation order) inside `client` (in the
/// children's coordinates); `changed` is the index of the control whose
/// Align, size or visibility just changed, if any. Returns the new
/// rectangle of every aligned, visible control, as `(index, rect)`.
pub fn align_controls(client: Rect, controls: &[Control], changed: Option<usize>) -> Vec<(usize, Rect)> {
    // What's left of the client area, as edges.
    let (mut left, mut top, mut right, mut bottom) = (client.left, client.top, client.right(), client.bottom());
    let mut out = Vec::new();
    for align in [Align::Top, Align::Bottom, Align::Left, Align::Right, Align::Client] {
        let takes_part = |i: usize| controls[i].align == align && controls[i].visible;
        let mut order: Vec<usize> = Vec::new();
        if let Some(c) = changed.filter(|&c| c < controls.len() && takes_part(c)) {
            order.push(c);
        }
        for i in (0..controls.len()).filter(|&i| takes_part(i) && Some(i) != changed) {
            let r = controls[i].rect;
            let at = order
                .iter()
                .position(|&j| {
                    let o = controls[j].rect;
                    match align {
                        Align::Top => r.top < o.top,
                        Align::Bottom => r.bottom() >= o.bottom(),
                        Align::Left => r.left < o.left,
                        Align::Right => r.right() >= o.right(),
                        _ => false,
                    }
                })
                .unwrap_or(order.len());
            order.insert(at, i);
        }
        for i in order {
            let r = controls[i].rect;
            let (w, h) = (r.width.max(0), r.height.max(0));
            let placed = match align {
                Align::Top => {
                    let p = Rect::new(left, top, right - left, h);
                    top += h;
                    p
                }
                Align::Bottom => {
                    bottom -= h;
                    Rect::new(left, bottom, right - left, h)
                }
                Align::Left => {
                    let p = Rect::new(left, top, w, bottom - top);
                    left += w;
                    p
                }
                Align::Right => {
                    right -= w;
                    Rect::new(right, top, w, bottom - top)
                }
                _ => Rect::new(left, top, right - left, bottom - top),
            };
            // Never a negative size when the client area runs out.
            out.push((i, Rect { width: placed.width.max(0), height: placed.height.max(0), ..placed }));
        }
    }
    out
}

/// Dragging a QSPLITTER (Delphi's TSplitter, which RapidQ wraps): the
/// control just outside the splitter's anchored edge — left of an alLeft
/// splitter, above an alTop one, … — gets wider / taller as the splitter
/// moves away from that edge, never below `MinSize` nor leaving less than
/// `MinSize` for the rest of the client area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitterDrag {
    /// Index (in the controls given to [`splitter_drag`]) of the control
    /// being resized.
    pub control: usize,
    /// True when it's the control's Width that changes (alLeft / alRight).
    pub horizontal: bool,
    start: i64,
    /// +1 when moving right / down grows the control (alLeft / alTop).
    sign: i64,
    min: i64,
    max: i64,
}

impl SplitterDrag {
    /// The control's size once the mouse moved `delta` pixels (along the
    /// splitter's axis) from where the drag started.
    pub fn size_for(&self, delta: i64) -> i64 {
        (self.start + self.sign * delta).min(self.max).max(self.min)
    }
}

/// Starts dragging splitter `splitter` among its parent's `controls`, in a
/// client area `client` wide / high; `None` if nothing is there to resize
/// (or the splitter isn't aligned).
pub fn splitter_drag(client: Rect, controls: &[Control], splitter: usize, min_size: i64) -> Option<SplitterDrag> {
    let sp = controls.get(splitter)?;
    let r = sp.rect;
    let (x, y, horizontal, sign) = match sp.align {
        Align::Left => (r.left - 1, r.top, true, 1),
        Align::Right => (r.right(), r.top, true, -1),
        Align::Top => (r.left, r.top - 1, false, 1),
        Align::Bottom => (r.left, r.bottom(), false, -1),
        _ => return None,
    };
    let inside = |c: &Rect| {
        // A control of no width / height still counts on its splitter side.
        let (mut left, mut right, mut top, mut bottom) = (c.left, c.right(), c.top, c.bottom());
        if c.width == 0 {
            if matches!(sp.align, Align::Left | Align::Top) { left -= 1 } else { right += 1 }
        }
        if c.height == 0 {
            if matches!(sp.align, Align::Left | Align::Top) { top -= 1 } else { bottom += 1 }
        }
        (left..right).contains(&x) && (top..bottom).contains(&y)
    };
    let control = (0..controls.len()).find(|&i| i != splitter && controls[i].visible && inside(&controls[i].rect))?;
    let size = |c: &Control| if horizontal { c.rect.width } else { c.rect.height };
    let edge_aligns: [Align; 2] = if horizontal { [Align::Left, Align::Right] } else { [Align::Top, Align::Bottom] };
    let taken: i64 = controls.iter().filter(|c| c.visible && edge_aligns.contains(&c.align)).map(size).sum();
    let room = if horizontal { client.width } else { client.height };
    let start = size(&controls[control]);
    let max = room - min_size - taken + start;
    Some(SplitterDrag { control, horizontal, start, sign, min: min_size.min(max.max(0)), max: max.max(0) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(align: Align, left: i64, top: i64, width: i64, height: i64) -> Control {
        Control { align, visible: true, rect: Rect::new(left, top, width, height) }
    }

    fn laid(client: Rect, controls: &[Control], changed: Option<usize>) -> Vec<Rect> {
        let mut rects: Vec<Rect> = controls.iter().map(|c| c.rect).collect();
        for (i, r) in align_controls(client, controls, changed) {
            rects[i] = r;
        }
        rects
    }

    #[test]
    fn edges_then_client() {
        // A toolbar, a status bar, a tree on the left, a panel on the right
        // and an editor filling the rest of a 400×300 client area.
        let controls = [
            c(Align::Client, 0, 0, 10, 10),
            c(Align::Top, 5, 5, 50, 30),
            c(Align::Bottom, 0, 0, 50, 20),
            c(Align::Left, 0, 0, 100, 10),
            c(Align::Right, 0, 0, 60, 10),
            c(Align::None, 7, 8, 9, 10),
        ];
        let r = laid(Rect::new(0, 0, 400, 300), &controls, None);
        assert_eq!(r[1], Rect::new(0, 0, 400, 30));
        assert_eq!(r[2], Rect::new(0, 280, 400, 20));
        assert_eq!(r[3], Rect::new(0, 30, 100, 250));
        assert_eq!(r[4], Rect::new(340, 30, 60, 250));
        assert_eq!(r[0], Rect::new(100, 30, 240, 250));
        assert_eq!(r[5], Rect::new(7, 8, 9, 10), "alNone is left alone");
    }

    #[test]
    fn same_align_by_position_changed_first() {
        // Splitter (alLeft, width 5) created before Tree (alLeft, width
        // 200): aligning the tree puts it first, the splitter after it...
        let controls = [c(Align::Left, 0, 0, 5, 10), c(Align::Left, 0, 0, 200, 10), c(Align::Client, 0, 0, 1, 1)];
        let r = laid(Rect::new(0, 0, 500, 300), &controls, Some(1));
        assert_eq!(r[1].left, 0);
        assert_eq!(r[0].left, 200);
        assert_eq!(r[2], Rect::new(205, 0, 295, 300));
        // ...and from then on their positions keep that order.
        let placed = [c(Align::Left, r[0].left, 0, 5, 300), c(Align::Left, r[1].left, 0, 200, 300), c(Align::Client, 0, 0, 1, 1)];
        let again = laid(Rect::new(0, 0, 800, 300), &placed, None);
        assert_eq!((again[1].left, again[0].left), (0, 200));
        // Two alTop controls: the one nearer the top first; equal tops keep
        // creation order.
        let tops = [c(Align::Top, 0, 50, 10, 20), c(Align::Top, 0, 10, 10, 30), c(Align::Top, 0, 10, 10, 5)];
        let t = laid(Rect::new(0, 0, 100, 100), &tops, None);
        assert_eq!((t[1].top, t[2].top, t[0].top), (0, 30, 35));
        // Two alBottom controls at the same place: the later one is placed
        // first, at the very bottom.
        let bottoms = [c(Align::Bottom, 0, 0, 10, 20), c(Align::Bottom, 0, 0, 10, 20)];
        let b = laid(Rect::new(0, 0, 100, 100), &bottoms, None);
        assert_eq!((b[1].top, b[0].top), (80, 60));
    }

    #[test]
    fn invisible_and_overflowing() {
        let mut hidden = c(Align::Top, 0, 0, 10, 40);
        hidden.visible = false;
        let controls = [hidden, c(Align::Top, 0, 0, 10, 70), c(Align::Bottom, 0, 0, 10, 70), c(Align::Client, 0, 0, 1, 1)];
        let r = laid(Rect::new(0, 10, 100, 100), &controls, None);
        assert_eq!(r[0], Rect::new(0, 0, 10, 40), "an invisible control isn't moved");
        assert_eq!(r[1], Rect::new(0, 10, 100, 70));
        assert_eq!(r[2], Rect::new(0, 40, 100, 70));
        assert_eq!(r[3].height, 0, "no negative size when the space runs out");
    }

    #[test]
    fn splitter_resizes_its_neighbour() {
        // Tree (alLeft, 200) | Splitter (alLeft, 5) | Memo (alClient) in 500.
        let client = Rect::new(0, 0, 500, 300);
        let controls = [c(Align::Left, 0, 0, 200, 300), c(Align::Left, 200, 0, 5, 300), c(Align::Client, 205, 0, 295, 300)];
        let d = splitter_drag(client, &controls, 1, 30).unwrap();
        assert_eq!((d.control, d.horizontal), (0, true));
        assert_eq!(d.size_for(50), 250);
        assert_eq!(d.size_for(-500), 30, "MinSize");
        assert_eq!(d.size_for(1000), 500 - 30 - 5, "leaves MinSize for the rest");
        // An alBottom splitter above an alBottom panel: dragging up grows it.
        let controls = [c(Align::Client, 0, 0, 500, 195), c(Align::Bottom, 0, 195, 500, 5), c(Align::Bottom, 0, 200, 500, 100)];
        let d = splitter_drag(client, &controls, 1, 30).unwrap();
        assert_eq!((d.control, d.horizontal), (2, false));
        assert_eq!(d.size_for(-40), 140);
        // Nothing next to it, or not aligned: no drag.
        assert!(splitter_drag(client, &[c(Align::Left, 0, 0, 5, 300)], 0, 30).is_none());
        assert!(splitter_drag(client, &[c(Align::None, 0, 0, 5, 300)], 0, 30).is_none());
    }

    #[test]
    fn form_frame_sizes() {
        assert_eq!(form_client_size(400, 300, 2, 0), (398, 269));
        assert_eq!(form_client_size(400, 300, 2, 28), (398, 241));
        assert_eq!(form_client_size(400, 300, 0, 0), (400, 300), "bsNone has no frame");
        assert_eq!(form_outer_size(398, 241, 2, 28), (400, 300));
        assert_eq!(form_client_size(1, 1, 2, 0), (0, 0));
    }

    #[test]
    fn values_and_defaults() {
        for n in 0..=5 {
            assert_eq!(Align::from_value(n).value(), n);
        }
        assert_eq!(Align::from_value(99), Align::None);
        assert_eq!(default_align("RStatusBar"), Align::Bottom);
        assert_eq!(default_align("RSPLITTER"), Align::Left);
        assert_eq!(default_align("RBUTTON"), Align::None);
    }
}
