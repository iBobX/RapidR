//! Drawing a form's ops with vello, at device resolution (principle 6):
//! what's drawn is given in RapidQ's logical pixels (1/96 inch) and every
//! edge is put on the screen's own pixels — a 1-pixel bevel line is
//! `scale` device pixels wide and lands exactly on them, text is shaped
//! and placed at the screen's resolution, polygons and glyphs are vector
//! paths. Nothing is drawn at 96 dpi and enlarged.
//!
//! The ops are the shared models' own (`rapidr_value::objects::tabcontrol::Op`,
//! `trackbar::Shape`), the ones the FLTK runtime and the web runtime draw:
//! this host only renders them.

use parley::{Layout, PositionedLayoutItem};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::tabcontrol::{Op, Rect};
use rapidr_value::objects::trackbar::Shape;
use vello::kurbo::{Affine, BezPath, Diagonal2, Rect as KRect, Stroke};
use vello::peniko::{Color, Fill};
use vello::{FontEmbolden, Glyph, Scene};

use crate::text::{Ink, TextSystem};

/// 0xRRGGBB as a vello colour.
pub fn color(rgb: u32) -> Color {
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// Where text sits in its rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    /// Centred both ways (a button's caption, a tab's).
    Center,
    /// At the top left (a QLABEL's caption, Windows' DrawText default).
    TopLeft,
}

/// Draws into a vello scene in logical pixels, from an origin (the
/// component being drawn), at `scale` device pixels per logical pixel.
pub struct Painter<'a> {
    pub scene: &'a mut Scene,
    pub text: &'a mut TextSystem,
    pub scale: f64,
    origin: (i64, i64),
}

impl<'a> Painter<'a> {
    pub fn new(scene: &'a mut Scene, text: &'a mut TextSystem, scale: f64) -> Self {
        Painter { scene, text, scale, origin: (0, 0) }
    }

    /// Draws what `f` draws with (0, 0) at `origin` (a component's Left / Top).
    pub fn at(&mut self, origin: (i64, i64), f: impl FnOnce(&mut Painter)) {
        let was = self.origin;
        self.origin = (was.0 + origin.0, was.1 + origin.1);
        f(self);
        self.origin = was;
    }

    /// A logical coordinate on the device's pixel grid.
    fn dev(&self, v: i64) -> f64 {
        (v as f64 * self.scale).round()
    }

    /// A logical rectangle on the device's pixels (its edges snapped).
    pub fn device_rect(&self, r: Rect) -> KRect {
        let (x, y, w, h) = r;
        let (ox, oy) = self.origin;
        KRect::new(self.dev(ox + x), self.dev(oy + y), self.dev(ox + x + w), self.dev(oy + y + h))
    }

    pub fn fill(&mut self, r: Rect, rgb: u32) {
        if r.2 <= 0 || r.3 <= 0 {
            return;
        }
        let rect = self.device_rect(r);
        self.scene.fill(Fill::NonZero, Affine::IDENTITY, color(rgb), None, &rect);
    }

    /// A 3D frame: lines from the outside in, top / left in `light`,
    /// bottom / right in `dark` (Windows' DrawEdge).
    pub fn edge(&mut self, r: Rect, light: &[u32], dark: &[u32]) {
        let (x, y, w, h) = r;
        for (k, c) in light.iter().enumerate() {
            let k = k as i64;
            self.fill((x + k, y + k, w - 2 * k, 1), *c);
            self.fill((x + k, y + k, 1, h - 2 * k), *c);
        }
        for (k, c) in dark.iter().enumerate() {
            let k = k as i64;
            self.fill((x + k, y + h - 1 - k, w - 2 * k, 1), *c);
            self.fill((x + w - 1 - k, y + k, 1, h - 2 * k), *c);
        }
    }

    /// Windows' DrawFocusRect: a dotted rectangle, every other pixel.
    pub fn focus(&mut self, r: Rect) {
        let (x, y, w, h) = r;
        if w <= 0 || h <= 0 {
            return;
        }
        for i in (0..w).step_by(2) {
            self.fill((x + i, y, 1, 1), 0x000000);
            self.fill((x + i, y + h - 1, 1, 1), 0x000000);
        }
        for j in (0..h).step_by(2) {
            self.fill((x, y + j, 1, 1), 0x000000);
            self.fill((x + w - 1, y + j, 1, 1), 0x000000);
        }
    }

    /// A polygon (or with two points a line) of logical points; outlines
    /// are whole device pixels wide, on the pixel grid (as the web's
    /// `shape-rendering="crispEdges"`).
    pub fn shape(&mut self, shape: &Shape) {
        let lw = self.scale.round().max(1.0);
        let (ox, oy) = (self.origin.0 as f64, self.origin.1 as f64);
        let snap = |v: f64| ((v) * self.scale).floor() + lw / 2.0;
        let mut path = BezPath::new();
        for (i, (x, y)) in shape.points.iter().enumerate() {
            let p = (snap(ox + x), snap(oy + y));
            if i == 0 { path.move_to(p) } else { path.line_to(p) }
        }
        if shape.points.len() > 2 {
            path.close_path();
            if let Some(f) = shape.fill {
                self.scene.fill(Fill::NonZero, Affine::IDENTITY, color(f), None, &path);
            }
        }
        if let Some(s) = shape.stroke {
            self.scene.stroke(&Stroke::new(lw), Affine::IDENTITY, color(s), None, &path);
        }
    }

    /// A filled polygon, exactly where its logical points fall.
    pub fn polygon(&mut self, points: &[(f64, f64)], rgb: u32) {
        let (ox, oy) = (self.origin.0 as f64, self.origin.1 as f64);
        let mut path = BezPath::new();
        for (i, (x, y)) in points.iter().enumerate() {
            let p = ((ox + x) * self.scale, (oy + y) * self.scale);
            if i == 0 { path.move_to(p) } else { path.line_to(p) }
        }
        path.close_path();
        self.scene.fill(Fill::NonZero, Affine::IDENTITY, color(rgb), None, &path);
    }

    /// `text` in `font` and `rgb` in a logical rectangle, turned `angle`
    /// degrees (90: reading upward, as the shared models' `Op::Text`).
    pub fn text(&mut self, r: Rect, text: &str, font: &Font, rgb: u32, angle: i32, place: Place) {
        if text.is_empty() {
            return;
        }
        let layout = self.text.layout(text, font, rgb, self.scale as f32);
        let (lw, lh) = (f64::from(layout.width()), f64::from(layout.height()));
        let rect = self.device_rect(r);
        let transform = if angle == 0 {
            let x = match place {
                Place::Center => rect.x0 + ((rect.width() - lw) / 2.0).round(),
                Place::TopLeft => rect.x0,
            };
            let y = match place {
                Place::TopLeft => rect.y0,
                _ => rect.y0 + ((rect.height() - lh) / 2.0).round(),
            };
            Affine::translate((x, y))
        } else {
            let c = rect.center();
            Affine::translate((c.x, c.y)) * Affine::rotate(-f64::from(angle).to_radians()) * Affine::translate((-(lw / 2.0).round(), -(lh / 2.0).round()))
        };
        self.layout(&layout, transform);
    }

    /// Draws a parley layout (device pixels) with `transform`, in its own
    /// colours.
    pub fn layout(&mut self, layout: &Layout<Ink>, transform: Affine) {
        self.draw_layout(layout, transform, None);
    }

    /// Draws a parley layout all in one colour (selected text).
    pub fn layout_in(&mut self, layout: &Layout<Ink>, transform: Affine, rgb: u32) {
        self.draw_layout(layout, transform, Some(rgb));
    }

    fn draw_layout(&mut self, layout: &Layout<Ink>, transform: Affine, ink: Option<u32>) {
        // (hinting only applies when glyphs aren't turned)
        let upright = transform.as_coeffs()[1] == 0.0 && transform.as_coeffs()[2] == 0.0;
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else { continue };
                let run = glyph_run.run();
                let style = glyph_run.style();
                let synthesis = run.synthesis();
                let size = run.font_size();
                let mut draw = self
                    .scene
                    .draw_glyphs(run.font())
                    .brush(color(ink.unwrap_or(style.brush.0)))
                    .hint(upright)
                    .transform(transform)
                    .glyph_transform(synthesis.skew().map(|a| Affine::skew(f64::from(a).to_radians().tan(), 0.0)))
                    .font_size(size)
                    .normalized_coords(run.normalized_coords());
                if synthesis.embolden() {
                    let amount = f64::from(size) / 48.0;
                    draw = draw.font_embolden(FontEmbolden::new(Diagonal2::new(amount, amount)));
                }
                draw.draw(Fill::NonZero, glyph_run.positioned_glyphs().map(|g| Glyph { id: g.id, x: g.x, y: g.y }));
                // Underline / StrikeOut: a line from the run's metrics.
                let metrics = run.metrics();
                let x0 = f64::from(glyph_run.offset());
                let x1 = x0 + f64::from(glyph_run.advance());
                let base = f64::from(glyph_run.baseline());
                for (deco, offset, thickness) in [
                    (&style.underline, metrics.underline_offset, metrics.underline_size),
                    (&style.strikethrough, metrics.strikethrough_offset, metrics.strikethrough_size),
                ] {
                    if let Some(d) = deco {
                        let t = f64::from(d.size.unwrap_or(thickness)).max(1.0);
                        let y = base - f64::from(d.offset.unwrap_or(offset));
                        self.scene.fill(Fill::NonZero, transform, color(ink.unwrap_or(d.brush.0)), None, &KRect::new(x0, y, x1, y + t));
                    }
                }
            }
        }
    }

    /// The shared models' ops (a QTABCONTROL's, a form's scroll bars').
    pub fn ops(&mut self, ops: &[Op]) {
        for op in ops {
            match op {
                Op::Fill { rect, color } => self.fill(*rect, *color),
                Op::Text { rect, text, angle, font, color } => self.text(*rect, text, font, *color, *angle, Place::Center),
                Op::Focus { rect } => self.focus(*rect),
                Op::Arrow { points, color } => self.polygon(points, *color),
            }
        }
    }

    /// Clips what `f` draws to a logical rectangle.
    pub fn clipped(&mut self, r: Rect, f: impl FnOnce(&mut Painter)) {
        let rect = self.device_rect(r);
        self.scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &rect);
        f(self);
        self.scene.pop_layer();
    }

    /// The device position of a logical point (for layouts drawn directly).
    pub fn device_point(&self, x: i64, y: i64) -> (f64, f64) {
        (self.dev(self.origin.0 + x), self.dev(self.origin.1 + y))
    }
}
