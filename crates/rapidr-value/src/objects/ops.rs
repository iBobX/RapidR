//! What a component draws, as data: one op vocabulary for every host that
//! draws the shared models — RapidR's own UI kernel (`rapidr-ui-kernel`),
//! whose hosts render [`Op`]s with vello on the GPU or the CPU, and the web
//! runtime. Coordinates are RapidQ's logical pixels (1/96 inch)
//! in the component's own space; a host puts every edge on the device's
//! pixels (a 1-pixel bevel line is `scale` device pixels wide).
//!
//! [`ModelOp`] is the subset the shared models emit today (QTABCONTROL's
//! tabs, the scroll bars): `tabcontrol::Op` is it, re-exported, so the
//! runtimes that match on those four ops keep drawing exactly what they
//! draw. [`Op`] is its superset, the kernel's vocabulary, and every
//! `ModelOp` converts into it unchanged ([`From`]).

use super::font::Font;
use super::trackbar::Shape;

/// A rectangle (x, y, width, height).
pub type Rect = (i64, i64, i64, i64);

/// What the shared models draw (`tabcontrol::Op`): fills, centred text, a
/// focus rectangle and arrows.
#[derive(Clone, Debug, PartialEq)]
pub enum ModelOp {
    Fill { rect: Rect, color: u32 },
    /// `text` centred in `rect`, turned `angle` degrees (90: reading upward,
    /// -90: downward), in `font`, `color` (0xRRGGBB).
    Text { rect: Rect, text: String, angle: i32, font: Font, color: u32 },
    /// A dotted focus rectangle.
    Focus { rect: Rect },
    /// A filled triangle (a scroll button's arrow).
    Arrow { points: [(f64, f64); 3], color: u32 },
    /// Windows' 50 % pattern (a scroll bar's track): [`Op::Checker`].
    Checker { rect: Rect, a: u32, b: u32 },
}

/// Where text sits in its rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    /// Centred both ways (a button's caption, a tab's).
    Center,
    /// At the top left (a QLABEL's caption, Windows' DrawText default).
    TopLeft,
    /// At the top, centred across (QLABEL Alignment = taCenter).
    TopCenter,
    /// At the top right (QLABEL Alignment = taRightJustify).
    TopRight,
    /// At the left, centred down (a check box's caption).
    Left,
}

/// What to draw: [`ModelOp`]'s ops and the kernel's own.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Fill { rect: Rect, color: u32 },
    /// A 3D frame, as Windows' DrawEdge: lines from the outside in, the top
    /// and left in `light`, the bottom and right in `dark` (0xRRGGBB each).
    Edge { rect: Rect, light: Vec<u32>, dark: Vec<u32> },
    /// A line one logical pixel wide between two pixel centres (x.5: the
    /// pixel x), both ends' pixels included (a mnemonic's underline, a
    /// separator).
    Line { from: (f64, f64), to: (f64, f64), color: u32 },
    /// A polygon or a line (a QTRACKBAR's channel, ticks, thumb).
    Shape(Shape),
    /// `text` in `font` and `color` placed in `rect`, turned `angle`
    /// degrees (90: reading upward). Drawn without kerning, as GDI's
    /// TextOut: as wide as `text::text_size` measures it.
    Text { rect: Rect, text: String, font: Font, color: u32, angle: i32, place: Place },
    /// Windows' DrawFocusRect: a dotted rectangle, every other pixel.
    Focus { rect: Rect },
    /// A filled triangle.
    Arrow { points: [(f64, f64); 3], color: u32 },
    /// A filled polygon exactly where its points fall (logical pixel
    /// coordinates, off the grid where they are), smooth at any scale: the
    /// classic look's round shapes drawn at a high-DPI screen's resolution
    /// (a radio button's well, an oval button); a chart's areas, pie slices
    /// and markers (RPLOT).
    Polygon { points: Vec<(f64, f64)>, color: u32 },
    /// Windows' 50 % pattern (a scroll bar's track, a toggled button's
    /// face): the pixels (x, y) of `rect` with x + y odd in `a`, the others
    /// in `b` — counted from the origin (the component's corner), each a
    /// whole logical pixel at any scale.
    Checker { rect: Rect, a: u32, b: u32 },
    /// A rounded rectangle, smooth (anti-aliased, off the pixel grid where
    /// it curves): filled with `fill`, its border `width` pixels wide
    /// inside its edge in `stroke` — the fluent themes' buttons, boxes and
    /// focus rings (`rapidr_value::theme`); a circle when `radius` is half
    /// its side.
    Round { rect: Rect, radius: f64, fill: Option<u32>, stroke: Option<u32>, width: f64 },
    /// Line segments through `points` (pixel coordinates), `width` pixels
    /// wide with round joins and ends, smooth: a fluent check mark, a
    /// chevron.
    Stroke { points: Vec<(f64, f64)>, color: u32, width: f64 },
    /// A bitmap model's picture (a QCANVAS, a QIMAGE, a form's surface) by
    /// its object id, scaled into `rect`; `revision` tells a host when to
    /// upload it again.
    Image { source: String, revision: u64, rect: Rect },
    /// Clips what follows to `rect`, until the matching [`Op::ClipPop`].
    ClipPush { rect: Rect },
    /// Clips what follows to a polygon (logical points, as [`Op::Polygon`]'s),
    /// until the matching [`Op::ClipPop`]: a pie gauge's done part.
    ClipPolygon { points: Vec<(f64, f64)> },
    ClipPop,
}

impl From<ModelOp> for Op {
    fn from(op: ModelOp) -> Op {
        match op {
            ModelOp::Fill { rect, color } => Op::Fill { rect, color },
            ModelOp::Text { rect, text, angle, font, color } => Op::Text { rect, text, font, color, angle, place: Place::Center },
            ModelOp::Focus { rect } => Op::Focus { rect },
            ModelOp::Arrow { points, color } => Op::Arrow { points, color },
            ModelOp::Checker { rect, a, b } => Op::Checker { rect, a, b },
        }
    }
}

/// The shared models' ops as the kernel's.
pub fn lift(ops: Vec<ModelOp>) -> Vec<Op> {
    ops.into_iter().map(Op::from).collect()
}

/// An [`Op::Edge`] as the 1-pixel fills it is made of (for hosts that only
/// fill rectangles).
pub fn edge_fills(rect: Rect, light: &[u32], dark: &[u32]) -> Vec<(Rect, u32)> {
    let (x, y, w, h) = rect;
    let mut out = Vec::new();
    for (k, c) in light.iter().enumerate() {
        let k = k as i64;
        out.push(((x + k, y + k, w - 2 * k, 1), *c));
        out.push(((x + k, y + k, 1, h - 2 * k), *c));
    }
    for (k, c) in dark.iter().enumerate() {
        let k = k as i64;
        out.push(((x + k, y + h - 1 - k, w - 2 * k, 1), *c));
        out.push(((x + w - 1 - k, y + k, 1, h - 2 * k), *c));
    }
    out.retain(|((_, _, w, h), _)| *w > 0 && *h > 0);
    out
}

/// An [`Op::Focus`] as the 1-pixel dots it is made of: every other pixel,
/// as DrawFocusRect.
pub fn focus_dots(rect: Rect) -> Vec<(i64, i64)> {
    let (x, y, w, h) = rect;
    let mut out = Vec::new();
    if w <= 0 || h <= 0 {
        return out;
    }
    for i in (0..w).step_by(2) {
        out.push((x + i, y));
        out.push((x + i, y + h - 1));
    }
    for j in (0..h).step_by(2) {
        out.push((x, y + j));
        out.push((x + w - 1, y + j));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_ops_convert_unchanged() {
        let font = Font::default();
        let t: Op = ModelOp::Text { rect: (1, 2, 3, 4), text: "a".into(), angle: 90, font: font.clone(), color: 7 }.into();
        assert_eq!(t, Op::Text { rect: (1, 2, 3, 4), text: "a".into(), font, color: 7, angle: 90, place: Place::Center });
        assert_eq!(Op::from(ModelOp::Fill { rect: (0, 0, 1, 1), color: 1 }), Op::Fill { rect: (0, 0, 1, 1), color: 1 });
    }

    #[test]
    fn edges_and_dots() {
        let f = edge_fills((0, 0, 10, 5), &[1, 2], &[3]);
        assert_eq!(f.len(), 6);
        assert!(f.contains(&((0, 4, 10, 1), 3)));
        assert!(f.contains(&((1, 1, 8, 1), 2)));
        assert_eq!(focus_dots((0, 0, 4, 2)).len(), 6);
    }
}
