//! Snapping and smart guides (docs/ide-plan.md I4): pure functions on
//! rectangles in a container's client coordinates.
//!
//! A component being dragged (or one of its edges, resizing) snaps to the
//! nearest of, within [`Snapper::threshold`] pixels on each axis:
//!
//! * its siblings' edges and centres ([`GuideKind::Edge`],
//!   [`GuideKind::Centre`]), and their text **baselines** (a label's to an
//!   edit's: [`GuideKind::Baseline`]);
//! * its parent's edges and centre lines (centring);
//! * **margins**: [`Snapper::margin`] pixels from the parent's edges and
//!   from a sibling ([`GuideKind::Margin`]);
//! * **equal spacing**: the gap to a neighbour in the same row / column
//!   equal to a gap between two other siblings there
//!   ([`GuideKind::Spacing`], the distance shown).
//!
//! Where nothing is near, it snaps to the grid. Every guide that holds at
//! the snapped place is returned to be drawn (a thin line, a distance
//! label for margins and spacing). Alt / Option suspends snapping: the
//! caller passes `free`.

use crate::layout::Rect;

/// What a guide lines up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuideKind {
    Edge,
    Centre,
    Baseline,
    Margin,
    /// Equal gaps of this many pixels.
    Spacing(i64),
}

/// A guide to draw: a vertical line at x = `at` from y = `from` to `to`
/// (`vertical`), else a horizontal one at y = `at` from x = `from` to `to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guide {
    pub vertical: bool,
    pub at: i64,
    pub from: i64,
    pub to: i64,
    pub kind: GuideKind,
}

/// A sibling the moving component lines up with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub rect: Rect,
    /// Its text's baseline, from its top (None: no text line).
    pub baseline: Option<i64>,
}

/// Which edges a resize moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges {
    pub left: bool,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
}

/// The snapping settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapper {
    pub grid: i64,
    pub snap_to_grid: bool,
    pub snap_to_guides: bool,
    /// How near (logical pixels, at any zoom) a guide pulls.
    pub threshold: i64,
    /// The platform's spacing: from the parent's edges and between
    /// siblings.
    pub margin: i64,
}

impl Default for Snapper {
    fn default() -> Self {
        Snapper { grid: 8, snap_to_grid: true, snap_to_guides: true, threshold: 5, margin: 8 }
    }
}

/// The result: where the component goes, and the guides that hold there.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapped {
    pub rect: Rect,
    pub guides: Vec<Guide>,
}

/// One candidate line on an axis: the moving feature's offset from the
/// rectangle's start (0, half, the size; a baseline), the line, its kind
/// and the extent of what it lines up with (for drawing).
#[derive(Clone, Copy, Debug)]
struct Line {
    offset: i64,
    at: i64,
    kind: GuideKind,
    span: (i64, i64),
}

fn snap_grid(v: i64, grid: i64) -> i64 {
    if grid <= 1 {
        return v;
    }
    (v as f64 / grid as f64).round() as i64 * grid
}

impl Snapper {
    /// A drag: the component would be at `proposed` (its parent's client
    /// area `area` wide / high, its siblings `others`; `baseline` its own
    /// text's baseline from its top). `free`: snapping suspended (Alt).
    pub fn snap_move(&self, proposed: Rect, baseline: Option<i64>, others: &[Target], area: (i64, i64), free: bool) -> Snapped {
        if free {
            return Snapped { rect: proposed, guides: Vec::new() };
        }
        let mut r = proposed;
        let mut guides = Vec::new();
        for vertical in [true, false] {
            let lines = self.lines(r, baseline, others, area, vertical, None);
            let (pos, size) = if vertical { (r.left, r.width) } else { (r.top, r.height) };
            let best = if self.snap_to_guides { self.best(&lines, pos) } else { None };
            let new_pos = match best {
                Some(d) => pos + d,
                None if self.snap_to_grid => snap_grid(pos, self.grid),
                None => pos,
            };
            if vertical {
                r.left = new_pos;
            } else {
                r.top = new_pos;
            }
            let _ = size;
        }
        if self.snap_to_guides {
            for vertical in [true, false] {
                let lines = self.lines(r, baseline, others, area, vertical, None);
                guides.extend(self.holding(r, &lines, vertical));
            }
        }
        Snapped { rect: r, guides }
    }

    /// A resize: the moving `edges` of `proposed` snap (the others stay).
    pub fn snap_resize(&self, proposed: Rect, edges: Edges, others: &[Target], area: (i64, i64), free: bool) -> Snapped {
        if free {
            return Snapped { rect: proposed, guides: Vec::new() };
        }
        let mut r = proposed;
        // (each moving edge as a feature at offset 0 of a zero-size rect)
        for (on, vertical, far) in [(edges.left, true, false), (edges.right, true, true), (edges.top, false, false), (edges.bottom, false, true)] {
            if !on {
                continue;
            }
            let edge = match (vertical, far) {
                (true, false) => r.left,
                (true, true) => r.left + r.width,
                (false, false) => r.top,
                (false, true) => r.top + r.height,
            };
            let lines = self.lines(r, None, others, area, vertical, Some(far));
            let best = if self.snap_to_guides { self.best(&lines, edge) } else { None };
            let to = match best {
                Some(d) => edge + d,
                None if self.snap_to_grid => snap_grid(edge, self.grid),
                None => edge,
            };
            match (vertical, far) {
                (true, false) => {
                    r.width += r.left - to;
                    r.left = to;
                }
                (true, true) => r.width = to - r.left,
                (false, false) => {
                    r.height += r.top - to;
                    r.top = to;
                }
                (false, true) => r.height = to - r.top,
            }
        }
        r.width = r.width.max(1);
        r.height = r.height.max(1);
        let mut guides = Vec::new();
        if self.snap_to_guides {
            for (on, vertical, far) in [(edges.left, true, false), (edges.right, true, true), (edges.top, false, false), (edges.bottom, false, true)] {
                if on {
                    let lines = self.lines(r, None, others, area, vertical, Some(far));
                    guides.extend(self.holding(r, &lines, vertical));
                }
            }
        }
        Snapped { rect: r, guides }
    }

    /// The smallest move (within the threshold) that puts a feature on a
    /// line.
    fn best(&self, lines: &[Line], pos: i64) -> Option<i64> {
        lines.iter().map(|l| l.at - (pos + l.offset)).filter(|d| d.abs() <= self.threshold).min_by_key(|d| d.abs())
    }

    /// The guides that hold with the component at `r`.
    fn holding(&self, r: Rect, lines: &[Line], vertical: bool) -> Vec<Guide> {
        let pos = if vertical { r.left } else { r.top };
        let (lo, hi) = if vertical { (r.top, r.top + r.height) } else { (r.left, r.left + r.width) };
        let mut out: Vec<Guide> = Vec::new();
        for l in lines.iter().filter(|l| pos + l.offset == l.at) {
            let g = match l.kind {
                // (a spacing guide is the gaps themselves, drawn across)
                GuideKind::Spacing(_) => Guide { vertical: !vertical, at: l.span.0, from: l.span.1, to: l.at, kind: l.kind },
                GuideKind::Margin => Guide { vertical: !vertical, at: (lo + hi) / 2, from: l.span.0, to: l.span.1, kind: l.kind },
                _ => Guide { vertical, at: l.at, from: lo.min(l.span.0), to: hi.max(l.span.1), kind: l.kind },
            };
            if !out.contains(&g) {
                out.push(g);
            }
        }
        out
    }

    /// The lines a feature of the component at `r` can snap to on one axis
    /// (`vertical`: x positions). `edge`: only one edge moves (resizing:
    /// Some(far edge?)), whose offset is then 0 at that edge.
    fn lines(&self, r: Rect, baseline: Option<i64>, others: &[Target], area: (i64, i64), vertical: bool, edge: Option<bool>) -> Vec<Line> {
        let (pos, size) = if vertical { (r.left, r.width) } else { (r.top, r.height) };
        let (cross_lo, cross_hi) = if vertical { (r.top, r.top + r.height) } else { (r.left, r.left + r.width) };
        let full = if vertical { area.0 } else { area.1 };
        let cross_full = if vertical { area.1 } else { area.0 };
        // The moving features: (offset, is it the near / centre / far one)
        let features: Vec<(i64, u8)> = match edge {
            None => vec![(0, 0), (size / 2, 1), (size, 2)],
            Some(false) => vec![(0, 0)],
            Some(true) => vec![(size, 2)],
        };
        let mut out = Vec::new();
        let ext = |o: &Rect| if vertical { (o.top, o.top + o.height) } else { (o.left, o.left + o.width) };
        for o in others {
            let (opos, osize) = if vertical { (o.rect.left, o.rect.width) } else { (o.rect.top, o.rect.height) };
            let span = ext(&o.rect);
            for &(f, which) in &features {
                for (t, kind) in [(opos, GuideKind::Edge), (opos + osize / 2, GuideKind::Centre), (opos + osize, GuideKind::Edge)] {
                    // (centres line up with centres only)
                    if (which == 1) != (kind == GuideKind::Centre) {
                        continue;
                    }
                    out.push(Line { offset: f, at: t, kind, span });
                }
            }
            // Margins between siblings that face each other on this axis.
            let overlaps = { let (a, b) = ext(&o.rect); a < cross_hi && b > cross_lo };
            if overlaps {
                if features.iter().any(|&(_, w)| w == 0) {
                    out.push(Line { offset: 0, at: opos + osize + self.margin, kind: GuideKind::Margin, span: (opos + osize, opos + osize + self.margin) });
                }
                if features.iter().any(|&(_, w)| w == 2) {
                    out.push(Line { offset: size, at: opos - self.margin, kind: GuideKind::Margin, span: (opos - self.margin, opos) });
                }
            }
            // Baselines (horizontal lines: the y axis).
            if !vertical && edge.is_none() {
                if let (Some(mine), Some(theirs)) = (baseline, o.baseline) {
                    out.push(Line { offset: mine, at: o.rect.top + theirs, kind: GuideKind::Baseline, span: (o.rect.left, o.rect.left + o.rect.width) });
                }
            }
        }
        // The parent: its edges, its centre line, its margins.
        for &(f, which) in &features {
            match which {
                0 => {
                    out.push(Line { offset: f, at: 0, kind: GuideKind::Edge, span: (0, cross_full) });
                    out.push(Line { offset: f, at: self.margin, kind: GuideKind::Margin, span: (0, self.margin) });
                }
                1 => out.push(Line { offset: f, at: full / 2, kind: GuideKind::Centre, span: (0, cross_full) }),
                _ => {
                    out.push(Line { offset: f, at: full, kind: GuideKind::Edge, span: (0, cross_full) });
                    out.push(Line { offset: f, at: full - self.margin, kind: GuideKind::Margin, span: (full - self.margin, full) });
                }
            }
        }
        // Equal spacing in the row (column) the component is in.
        if edge.is_none() {
            let mut row: Vec<Rect> = others.iter().map(|o| o.rect).filter(|o| { let (a, b) = ext(o); a < cross_hi && b > cross_lo }).collect();
            row.sort_by_key(|o| if vertical { o.left } else { o.top });
            let start = |o: &Rect| if vertical { o.left } else { o.top };
            let end = |o: &Rect| if vertical { o.left + o.width } else { o.top + o.height };
            let mid = |o: &Rect| if vertical { o.top + o.height / 2 } else { o.left + o.width / 2 };
            let gaps: Vec<(i64, &Rect, &Rect)> = row.windows(2).filter_map(|w| { let g = start(&w[1]) - end(&w[0]); (g > 0).then_some((g, &w[0], &w[1])) }).collect();
            // (neighbours: the nearest before and after the component)
            let before = row.iter().filter(|o| end(o) <= pos + self.threshold).max_by_key(|o| end(o));
            let after = row.iter().filter(|o| start(o) >= pos + size - self.threshold).min_by_key(|o| start(o));
            for &(g, a, _) in &gaps {
                if let Some(n) = before {
                    out.push(Line { offset: 0, at: end(n) + g, kind: GuideKind::Spacing(g), span: ((mid(a) + mid(n)) / 2, end(n)) });
                }
                if let Some(n) = after {
                    out.push(Line { offset: size, at: start(n) - g, kind: GuideKind::Spacing(g), span: ((mid(a) + mid(n)) / 2, start(n) - g) });
                }
            }
        }
        let _ = pos;
        out
    }
}

/// Where a component's text baseline is, from its top: a label's text sits
/// at its top; a button's, an edit's, a check box's … centred in it.
pub fn baseline(type_name: &str, height: i64, font: &crate::objects::font::Font) -> Option<i64> {
    let (ascent, line) = crate::objects::text::line_metrics(font);
    match type_name.to_ascii_uppercase().as_str() {
        "RLABEL" => Some(ascent.round() as i64),
        "RBUTTON" | "REDIT" | "RCOMBOBOX" | "RCHECKBOX" | "RRADIOBUTTON" | "RCOOLBTN" | "RSPINEDIT" => Some(((height as f32 - line) / 2.0 + ascent).round() as i64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(l: i64, tp: i64, w: i64, h: i64) -> Target {
        Target { rect: Rect::new(l, tp, w, h), baseline: None }
    }

    #[test]
    fn edges_and_centres_of_siblings() {
        let s = Snapper::default();
        let others = [t(100, 100, 80, 24)];
        // 3 px right of the sibling's left edge: pulled onto it.
        let r = s.snap_move(Rect::new(103, 200, 80, 24), None, &others, (400, 300), false);
        assert_eq!(r.rect.left, 100);
        assert!(r.guides.iter().any(|g| g.vertical && g.at == 100 && g.kind == GuideKind::Edge && g.from == 100 && g.to == 224), "{:?}", r.guides);
        // centres: a 40 wide one whose centre is 2 px off the sibling's
        let r = s.snap_move(Rect::new(122, 200, 40, 24), None, &others, (400, 300), false);
        assert_eq!(r.rect.left, 120);
        assert!(r.guides.iter().any(|g| g.vertical && g.at == 140 && g.kind == GuideKind::Centre));
        // too far (6 px): the grid instead
        let r = s.snap_move(Rect::new(106, 203, 80, 24), None, &others, (400, 300), false);
        assert_eq!((r.rect.left, r.rect.top), (104, 200));
        // Alt: nothing
        let r = s.snap_move(Rect::new(103, 203, 80, 24), None, &others, (400, 300), true);
        assert_eq!((r.rect.left, r.rect.top, r.guides.len()), (103, 203, 0));
    }

    #[test]
    fn the_parent_centre_and_margins() {
        let s = Snapper::default();
        // centring: 100 wide in 400 → left 150
        let r = s.snap_move(Rect::new(152, 40, 100, 20), None, &[], (400, 300), false);
        assert_eq!(r.rect.left, 150);
        assert!(r.guides.iter().any(|g| g.vertical && g.at == 200 && g.kind == GuideKind::Centre && (g.from, g.to) == (0, 300)));
        // the margin from the parent's right edge: 8 px
        let r = s.snap_move(Rect::new(310, 41, 80, 20), None, &[], (400, 300), false);
        assert_eq!(r.rect.left + r.rect.width, 392);
        assert!(r.guides.iter().any(|g| g.kind == GuideKind::Margin && !g.vertical && (g.from, g.to) == (392, 400)));
        // the margin from a sibling: 8 px to its right
        let r = s.snap_move(Rect::new(195, 101, 50, 24), None, &[t(100, 100, 90, 24)], (400, 300), false);
        assert_eq!(r.rect.left, 198);
        assert!(r.guides.iter().any(|g| g.kind == GuideKind::Margin && (g.from, g.to) == (190, 198)));
    }

    #[test]
    fn baselines_and_equal_spacing() {
        let s = Snapper::default();
        // a label (baseline 11 from its top) next to an edit (baseline 16)
        let edit = Target { rect: Rect::new(100, 100, 120, 25), baseline: Some(16) };
        let r = s.snap_move(Rect::new(20, 103, 60, 30), Some(11), &[edit], (400, 300), false);
        assert_eq!(r.rect.top, 105, "baselines at 116");
        assert!(r.guides.iter().any(|g| !g.vertical && g.at == 116 && g.kind == GuideKind::Baseline));
        // three in a row 20 apart; the fourth dropped 22 after the third
        let row = [t(10, 50, 40, 20), t(70, 50, 40, 20), t(130, 50, 40, 20)];
        let r = s.snap_move(Rect::new(192, 52, 40, 20), None, &row, (400, 300), false);
        assert_eq!(r.rect.left, 190);
        assert!(r.guides.iter().any(|g| g.kind == GuideKind::Spacing(20)), "{:?}", r.guides);
    }

    #[test]
    fn resizing_snaps_the_moving_edge_only() {
        let s = Snapper::default();
        let others = [t(200, 90, 50, 50)];
        let right = Edges { right: true, ..Edges::default() };
        let r = s.snap_resize(Rect::new(10, 100, 187, 30), right, &others, (400, 300), false);
        assert_eq!((r.rect.left, r.rect.width), (10, 190), "to the sibling's left edge");
        let r = s.snap_resize(Rect::new(10, 100, 183, 30), right, &others, (400, 300), false);
        assert_eq!(r.rect.width, 182, "8 px before it (margin)");
        let left = Edges { left: true, ..Edges::default() };
        let r = s.snap_resize(Rect::new(13, 100, 100, 30), left, &[], (400, 300), false);
        assert_eq!((r.rect.left, r.rect.width), (8, 105), "the parent's margin");
    }

    #[test]
    fn three_hundred_components_snap_fast() {
        let s = Snapper::default();
        let others: Vec<Target> = (0..300).map(|i| t((i % 20) * 37, (i / 20) * 29, 30, 22)).collect();
        let start = std::time::Instant::now();
        for k in 0..50 {
            let _ = s.snap_move(Rect::new(100 + k, 120 + k, 60, 22), Some(15), &others, (800, 600), false);
        }
        let per = start.elapsed() / 50;
        // (the plan: < 2 ms per move; generous for debug builds)
        assert!(per.as_millis() < 20, "{per:?} per move");
    }
}
