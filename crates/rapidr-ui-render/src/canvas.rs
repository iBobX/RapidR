//! A kernel display list drawn at device resolution: what the kernel gives
//! in RapidQ's logical pixels (1/96 inch) lands on the screen's own pixels —
//! a 1-pixel bevel line is `scale` device pixels wide and exactly on them,
//! text is shaped and placed at the screen's resolution, polygons and glyphs
//! are vector paths. Nothing is drawn at 96 dpi and enlarged.
//!
//! The renderers implement [`Canvas`] (vello's GPU `Scene` in `gpu.rs`,
//! vello_cpu's context in `cpu.rs`), so a form paints the same calls into
//! either, on the desktop and in the browser. (The geometry and paint types
//! are kurbo's and peniko's as vello_cpu re-exports them: the very crates
//! vello uses.)

use std::collections::HashMap;
use std::sync::Arc;

use parley::{Layout, PositionedLayoutItem};
use rapidr_ui_kernel::display::{DisplayList, Item, Picture, TextItem};
use rapidr_ui_kernel::{FormUi, Ink, TextSystem};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{edge_fills, focus_dots, Op, Place, Rect};
use rapidr_value::objects::trackbar::Shape;
use vello_cpu::kurbo::{Affine, BezPath, Rect as KRect, RoundedRect, Shape as _};
use vello_cpu::peniko::{Color, FontData};

/// What a renderer draws into: the few primitives a display list needs
/// (device pixels; non-zero fill).
pub trait Canvas {
    fn fill_rect(&mut self, transform: Affine, rgb: u32, rect: &KRect);
    fn fill_path(&mut self, transform: Affine, rgb: u32, path: &BezPath);
    /// An outline `width` device pixels wide.
    fn stroke_path(&mut self, width: f64, rgb: u32, path: &BezPath);
    fn push_clip(&mut self, transform: Affine, rect: &KRect);
    /// Clips to a path (device pixels) until the matching `pop_clip`.
    fn push_clip_path(&mut self, path: &BezPath);
    fn pop_clip(&mut self);
    /// What follows drawn `alpha` (0–1) opaque, as one layer, until
    /// `pop_fade`.
    fn push_fade(&mut self, alpha: f32);
    fn pop_fade(&mut self);
    fn glyphs(&mut self, run: &GlyphRun, glyphs: &[(u32, f32, f32)]);
    /// A picture scaled into `rect` (device pixels), smoothly; `source` and
    /// `revision` name it in the picture cache (`images.rs`).
    fn image(&mut self, rect: &KRect, source: &str, revision: u64, picture: &Arc<Picture>);
}

/// A run of glyphs of one font, size and colour.
pub struct GlyphRun<'a> {
    pub font: &'a FontData,
    pub size: f32,
    pub rgb: u32,
    pub hint: bool,
    pub transform: Affine,
    /// Synthetic italic (a horizontal skew).
    pub glyph_transform: Option<Affine>,
    pub coords: &'a [i16],
    /// Synthetic bold: how far outlines grow sideways and up and down
    /// (device pixels).
    pub embolden: Option<(f64, f64)>,
}

/// 0xRRGGBB as a vello colour.
pub fn color(rgb: u32) -> Color {
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// The window's background before anything is drawn (Windows' button face).
pub const BACKGROUND: u32 = rapidr_ui_kernel::paint::FACE;

/// Draws into a canvas in logical pixels, from an origin (the item being
/// drawn), at `scale` device pixels per logical pixel.
pub struct Painter<'a> {
    pub canvas: &'a mut dyn Canvas,
    pub text: &'a mut TextSystem,
    pub scale: f64,
    origin: (i64, i64),
    /// Where the logical (0, 0) is on the device (a zoomed part: the list's
    /// `Item::Zoom`; else the corner).
    at: (f64, f64),
    /// The display list's pictures (what its `Op::Image`s name).
    pub images: Option<&'a HashMap<String, Arc<Picture>>>,
}

impl<'a> Painter<'a> {
    pub fn new(canvas: &'a mut dyn Canvas, text: &'a mut TextSystem, scale: f64) -> Self {
        Painter { canvas, text, scale, origin: (0, 0), at: (0.0, 0.0), images: None }
    }

    /// A logical coordinate on the device's pixel grid.
    fn dev(&self, v: i64) -> f64 {
        (v as f64 * self.scale).round()
    }

    /// A logical point (from the corner, not snapped) on the device.
    fn pt(&self, x: f64, y: f64) -> (f64, f64) {
        (self.at.0 + x * self.scale, self.at.1 + y * self.scale)
    }

    /// A logical rectangle on the device's pixels (its edges snapped).
    pub fn device_rect(&self, r: Rect) -> KRect {
        let (x, y, w, h) = r;
        let (ox, oy) = self.origin;
        let (ax, ay) = self.at;
        KRect::new(ax + self.dev(ox + x), ay + self.dev(oy + y), ax + self.dev(ox + x + w), ay + self.dev(oy + y + h))
    }

    pub fn fill(&mut self, r: Rect, rgb: u32) {
        if r.2 <= 0 || r.3 <= 0 {
            return;
        }
        let rect = self.device_rect(r);
        self.canvas.fill_rect(Affine::IDENTITY, rgb, &rect);
    }

    /// A line one logical pixel wide between pixel centres, both ends'
    /// pixels included (`Op::Line`): a fill when it's straight.
    fn line(&mut self, from: (f64, f64), to: (f64, f64), rgb: u32) {
        let (x0, y0, x1, y1) = (from.0.floor() as i64, from.1.floor() as i64, to.0.floor() as i64, to.1.floor() as i64);
        if y0 == y1 || x0 == x1 {
            self.fill((x0.min(x1), y0.min(y1), (x1 - x0).abs() + 1, (y1 - y0).abs() + 1), rgb);
            return;
        }
        let (ox, oy) = (self.origin.0 as f64, self.origin.1 as f64);
        let mut path = BezPath::new();
        path.move_to(self.pt(ox + from.0, oy + from.1));
        path.line_to(self.pt(ox + to.0, oy + to.1));
        self.canvas.stroke_path(self.scale.round().max(1.0), rgb, &path);
    }

    /// A polygon (or with two points a line) of logical points; outlines
    /// are whole device pixels wide, on the pixel grid (as the web's
    /// `shape-rendering="crispEdges"`).
    fn shape(&mut self, shape: &Shape) {
        let lw = self.scale.round().max(1.0);
        let (ox, oy) = (self.origin.0 as f64, self.origin.1 as f64);
        let snap = |v: f64| (v * self.scale).floor() + lw / 2.0;
        let mut path = BezPath::new();
        for (i, (x, y)) in shape.points.iter().enumerate() {
            let p = (self.at.0 + snap(ox + x), self.at.1 + snap(oy + y));
            if i == 0 {
                path.move_to(p)
            } else {
                path.line_to(p)
            }
        }
        if shape.points.len() > 2 {
            path.close_path();
            if let Some(f) = shape.fill {
                self.canvas.fill_path(Affine::IDENTITY, f, &path);
            }
        }
        if let Some(s) = shape.stroke {
            self.canvas.stroke_path(lw, s, &path);
        }
    }

    /// Windows' 50 % pattern in a logical rectangle (`Op::Checker`): a
    /// picture of it at the device's resolution — each logical pixel a
    /// whole block of device pixels, drawn pixel for pixel, so crisp —
    /// kept between frames by its size, phase and colours.
    fn checker(&mut self, r: Rect, a: u32, b: u32) {
        let (x, y, w, h) = r;
        if w <= 0 || h <= 0 {
            return;
        }
        if a == b {
            self.fill(r, b);
            return;
        }
        let dr = self.device_rect(r);
        let (pw, ph) = (dr.width().round().max(1.0) as usize, dr.height().round().max(1.0) as usize);
        let phase = (x + y).rem_euclid(2);
        let source = format!("checker:{pw}x{ph}:{w}x{h}:{phase}:{a:06x}:{b:06x}");
        let (sx, sy) = (pw as f64 / w as f64, ph as f64 / h as f64);
        let mut rgba = Vec::with_capacity(pw * ph * 4);
        for py in 0..ph {
            let ly = (py as f64 / sy).floor() as i64;
            for px in 0..pw {
                let lx = (px as f64 / sx).floor() as i64;
                let c = if (lx + ly + phase) % 2 == 1 { a } else { b };
                rgba.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
            }
        }
        let picture = Arc::new(Picture { width: pw, height: ph, rgba });
        self.canvas.image(&dr, &source, 0, &picture);
    }

    /// A filled polygon, exactly where its logical points fall.
    fn polygon(&mut self, points: &[(f64, f64)], rgb: u32) {
        let (ox, oy) = (self.origin.0 as f64, self.origin.1 as f64);
        let mut path = BezPath::new();
        for (i, (x, y)) in points.iter().enumerate() {
            let p = self.pt(ox + x, oy + y);
            if i == 0 {
                path.move_to(p)
            } else {
                path.line_to(p)
            }
        }
        path.close_path();
        self.canvas.fill_path(Affine::IDENTITY, rgb, &path);
    }

    /// A rounded rectangle (`Op::Round`): its rectangle on the device's
    /// pixels, its curves smooth; the border inside its edge, whole device
    /// pixels wide where the width allows (so its straight sides are sharp).
    fn round(&mut self, r: Rect, radius: f64, fill: Option<u32>, stroke: Option<u32>, width: f64) {
        if r.2 <= 0 || r.3 <= 0 {
            return;
        }
        let rect = self.device_rect(r);
        let radius = (radius * self.scale).max(0.0);
        if let Some(f) = fill {
            let path = RoundedRect::from_rect(rect, radius).to_path(0.1);
            self.canvas.fill_path(Affine::IDENTITY, f, &path);
        }
        if let Some(s) = stroke {
            let w = (width * self.scale).round().max(1.0);
            let inner = rect.inset(-w / 2.0);
            if inner.width() > 0.0 && inner.height() > 0.0 {
                let path = RoundedRect::from_rect(inner, (radius - w / 2.0).max(0.0)).to_path(0.1);
                self.canvas.stroke_path(w, s, &path);
            }
        }
    }

    /// Line segments through logical points (`Op::Stroke`), `width`
    /// logical pixels wide, round-joined, smooth.
    fn polyline(&mut self, points: &[(f64, f64)], rgb: u32, width: f64) {
        if points.len() < 2 {
            return;
        }
        let (ox, oy) = (self.origin.0 as f64, self.origin.1 as f64);
        let mut path = BezPath::new();
        for (i, (x, y)) in points.iter().enumerate() {
            let p = self.pt(ox + x, oy + y);
            if i == 0 {
                path.move_to(p)
            } else {
                path.line_to(p)
            }
        }
        self.canvas.stroke_path(width * self.scale, rgb, &path);
    }

    /// `text` in `font` and `rgb` in a logical rectangle, turned `angle`
    /// degrees (90: reading upward).
    fn text(&mut self, r: Rect, text: &str, font: &Font, rgb: u32, angle: i32, place: Place) {
        if text.is_empty() {
            return;
        }
        let layout = self.text.layout(text, font, rgb, self.scale as f32);
        let (lw, lh) = (f64::from(layout.width()), f64::from(layout.height()));
        let rect = self.device_rect(r);
        let transform = if angle == 0 {
            let x = match place {
                Place::Center | Place::TopCenter => rect.x0 + ((rect.width() - lw) / 2.0).round(),
                Place::TopRight => rect.x1 - lw.round(),
                Place::TopLeft | Place::Left => rect.x0,
            };
            let y = match place {
                Place::TopLeft | Place::TopCenter | Place::TopRight => rect.y0,
                Place::Center | Place::Left => rect.y0 + ((rect.height() - lh) / 2.0).round(),
            };
            Affine::translate((x, y))
        } else {
            let c = rect.center();
            Affine::translate((c.x, c.y)) * Affine::rotate(-f64::from(angle).to_radians()) * Affine::translate((-(lw / 2.0).round(), -(lh / 2.0).round()))
        };
        draw_layout(self.canvas, &layout, transform, None);
    }

    /// One op, from the current origin.
    pub fn op(&mut self, op: &Op) {
        match op {
            Op::Fill { rect, color } => self.fill(*rect, *color),
            Op::Edge { rect, light, dark } => {
                for (r, c) in edge_fills(*rect, light, dark) {
                    self.fill(r, c);
                }
            }
            Op::Line { from, to, color } => self.line(*from, *to, *color),
            Op::Shape(s) => self.shape(s),
            Op::Text { rect, text, font, color, angle, place } => self.text(*rect, text, font, *color, *angle, *place),
            Op::Focus { rect } => {
                for (x, y) in focus_dots(*rect) {
                    self.fill((x, y, 1, 1), 0x000000);
                }
            }
            Op::Arrow { points, color } => self.polygon(points, *color),
            Op::Checker { rect, a, b } => self.checker(*rect, *a, *b),
            Op::Round { rect, radius, fill, stroke, width } => self.round(*rect, *radius, *fill, *stroke, *width),
            Op::Stroke { points, color, width } => self.polyline(points, *color, *width),
            Op::Polygon { points, color } => {
                if points.len() >= 3 {
                    self.polygon(points, *color)
                }
            }
            // (a picture the display list carries: a component's own, or a
            // program's bitmap by object id with its drawing revision)
            Op::Image { source, revision, rect } => {
                if let Some(pic) = self.images.and_then(|m| m.get(source)) {
                    let r = self.device_rect(*rect);
                    self.canvas.image(&r, source, *revision, pic);
                }
            }
            Op::ClipPush { rect } => {
                let r = self.device_rect(*rect);
                self.canvas.push_clip(Affine::IDENTITY, &r);
            }
            Op::ClipPolygon { points } => {
                let (ox, oy) = (self.origin.0 as f64, self.origin.1 as f64);
                let mut path = BezPath::new();
                for (i, (x, y)) in points.iter().enumerate() {
                    let p = self.pt(ox + x, oy + y);
                    if i == 0 {
                        path.move_to(p)
                    } else {
                        path.line_to(p)
                    }
                }
                path.close_path();
                self.canvas.push_clip_path(&path);
            }
            Op::ClipPop => self.canvas.pop_clip(),
            Op::Fade { alpha } => self.canvas.push_fade(f32::from(*alpha) / 255.0),
            Op::FadePop => self.canvas.pop_fade(),
        }
    }
}

/// Draws a parley layout (device pixels) with `transform`, in its own
/// colours or all in `ink`.
pub fn draw_layout(canvas: &mut dyn Canvas, layout: &Layout<Ink>, transform: Affine, ink: Option<u32>) {
    // (hinting only applies when glyphs aren't turned)
    let upright = transform.as_coeffs()[1] == 0.0 && transform.as_coeffs()[2] == 0.0;
    for line in layout.lines() {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyph_run) = item else { continue };
            let run = glyph_run.run();
            let style = glyph_run.style();
            let synthesis = run.synthesis();
            let size = run.font_size();
            let glyphs: Vec<(u32, f32, f32)> = glyph_run.positioned_glyphs().map(|g| (g.id, g.x, g.y)).collect();
            canvas.glyphs(
                &GlyphRun {
                    font: run.font(),
                    size,
                    rgb: ink.unwrap_or(style.brush.0),
                    hint: upright,
                    transform,
                    glyph_transform: synthesis.skew().map(|a| Affine::skew(f64::from(a).to_radians().tan(), 0.0)),
                    coords: run.normalized_coords(),
                    // (a synthetic bold as heavy as a real one: about a pixel wider at
                    // 8 pt, as Windows' bold is, a little taller)
                    embolden: synthesis.embolden().then(|| (f64::from(size) / 24.0, f64::from(size) / 96.0)),
                },
                &glyphs,
            );
            // Underline / StrikeOut: a line from the run's metrics.
            let metrics = run.metrics();
            let x0 = f64::from(glyph_run.offset());
            let x1 = x0 + f64::from(glyph_run.advance());
            let base = f64::from(glyph_run.baseline());
            for (deco, offset, thickness) in [(&style.underline, metrics.underline_offset, metrics.underline_size), (&style.strikethrough, metrics.strikethrough_offset, metrics.strikethrough_size)] {
                if let Some(d) = deco {
                    let t = f64::from(d.size.unwrap_or(thickness)).max(1.0);
                    let y = base - f64::from(d.offset.unwrap_or(offset));
                    canvas.fill_rect(transform, ink.unwrap_or(d.brush.0), &KRect::new(x0, y, x1, y + t));
                }
            }
        }
    }
}

/// An editor's text: the selection's highlight, the layout, the selected
/// glyphs again in the highlight's text colour, the composition's
/// underlines, the caret.
fn editor(canvas: &mut dyn Canvas, t: &TextItem, layout: &Layout<Ink>) {
    let at = Affine::translate(t.origin);
    let dev = |(x0, y0, x1, y1): (f64, f64, f64, f64)| KRect::new(x0, y0, x1, y1);
    for r in &t.selection {
        canvas.fill_rect(at, t.highlight, &dev(*r));
    }
    draw_layout(canvas, layout, at, None);
    for r in &t.selection {
        canvas.push_clip(at, &dev(*r));
        draw_layout(canvas, layout, at, Some(t.highlight_text));
        canvas.pop_clip();
    }
    for r in &t.underlines {
        canvas.fill_rect(at, t.caret_color, &dev(*r));
    }
    if let Some(c) = t.caret {
        canvas.fill_rect(at, t.caret_color, &dev(c));
    }
}

/// Draws `list` (form `form`'s, for its editors' layouts) into `canvas`.
pub fn draw_list(canvas: &mut dyn Canvas, text: &mut TextSystem, list: &DisplayList, form: &FormUi) {
    let mut p = Painter::new(canvas, text, list.scale);
    p.images = Some(&list.images);
    for item in &list.items {
        match item {
            Item::Op { origin, op } => {
                p.origin = *origin;
                p.op(op);
            }
            Item::Zoom(z) => {
                (p.scale, p.at) = z.map_or((list.scale, (0.0, 0.0)), |z| (z.scale, z.at));
            }
            Item::Text(t) => {
                if let Some(layout) = form.editor_layout_at(&t.node, t.para) {
                    editor(p.canvas, t, layout);
                }
            }
        }
    }
    crate::images::end_frame();
}

/// A display list's size in device pixels.
pub fn device_size(list: &DisplayList) -> (u32, u32) {
    let (w, h) = list.size;
    (((w as f64) * list.scale).round().max(1.0) as u32, ((h as f64) * list.scale).round().max(1.0) as u32)
}
