//! The CPU renderer (Stage 0 spike B): the same painting calls rasterized
//! by vello_cpu (vello's sparse-strips CPU renderer, same peniko / kurbo /
//! skrifa as the GPU one) instead of on the GPU. It is what captures use
//! (deterministic, no GPU or window server needed: the headless host), and
//! the fallback where wgpu finds no adapter; a window shows its frames
//! through softbuffer (`RAPIDR_RENDERER=cpu`).

use rapidr_value::objects::codec::Pixels;
use vello::kurbo::{Affine, BezPath, Rect as KRect, Stroke};
use vello_cpu::{Pixmap, RenderContext, RenderMode, Resources};

use crate::form::Form;
use crate::paint::{color, Canvas, GlyphRun, Painter};
use crate::text::TextSystem;

/// A vello_cpu render context being drawn into.
pub struct CpuCanvas<'a> {
    pub ctx: &'a mut RenderContext,
    pub res: &'a mut Resources,
}

impl Canvas for CpuCanvas<'_> {
    fn fill_rect(&mut self, transform: Affine, rgb: u32, rect: &KRect) {
        self.ctx.set_transform(transform);
        self.ctx.set_paint(color(rgb));
        self.ctx.fill_rect(rect);
    }
    fn fill_path(&mut self, transform: Affine, rgb: u32, path: &BezPath) {
        self.ctx.set_transform(transform);
        self.ctx.set_paint(color(rgb));
        self.ctx.fill_path(path);
    }
    fn stroke_path(&mut self, width: f64, rgb: u32, path: &BezPath) {
        self.ctx.set_transform(Affine::IDENTITY);
        self.ctx.set_paint(color(rgb));
        self.ctx.set_stroke(Stroke::new(width));
        self.ctx.stroke_path(path);
    }
    fn push_clip(&mut self, transform: Affine, rect: &KRect) {
        self.ctx.set_transform(transform);
        self.ctx.push_clip_rect(rect);
    }
    fn pop_clip(&mut self) {
        self.ctx.pop_clip();
    }
    fn glyphs(&mut self, run: &GlyphRun, glyphs: &[(u32, f32, f32)]) {
        self.ctx.set_transform(run.transform);
        self.ctx.set_paint(color(run.rgb));
        let mut b = self.ctx.glyph_run(self.res, run.font).font_size(run.size).hint(run.hint).normalized_coords(run.coords);
        if let Some(t) = run.glyph_transform {
            b = b.glyph_transform(t);
        }
        if let Some(a) = run.embolden {
            b = b.font_embolden(glifo::FontEmbolden::new(vello::kurbo::Diagonal2::new(a, a)));
        }
        b.fill_glyphs(glyphs.iter().map(|&(id, x, y)| vello_cpu::Glyph { id, x, y })).ok();
    }
}

/// vello_cpu with its persistent resources (glyph caches) and a target.
pub struct CpuRenderer {
    ctx: RenderContext,
    res: Resources,
    pub pixmap: Pixmap,
}

impl CpuRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        let (w, h) = (clamp16(width), clamp16(height));
        CpuRenderer { ctx: RenderContext::new(w, h), res: Resources::new(), pixmap: Pixmap::new(w, h) }
    }

    /// Paints with `paint` into the pixmap (resized to w × h first).
    pub fn render(&mut self, width: u32, height: u32, paint: impl FnOnce(&mut dyn Canvas)) {
        let (w, h) = (clamp16(width), clamp16(height));
        if (w, h) != (self.ctx.width(), self.ctx.height()) {
            self.ctx = RenderContext::new(w, h);
            self.pixmap = Pixmap::new(w, h);
        } else {
            self.ctx.reset();
        }
        paint(&mut CpuCanvas { ctx: &mut self.ctx, res: &mut self.res });
        self.ctx.flush();
        let settings = vello_cpu::RasterizerSettings { render_mode: RenderMode::OptimizeSpeed, target_init: vello_cpu::TargetInit::Clear(color(crate::form::FACE)), ..Default::default() };
        self.ctx.render_with(&mut self.pixmap, &mut self.res, settings);
    }

    /// The form at `scale` into the pixmap.
    pub fn render_form(&mut self, form: &mut Form, text: &mut TextSystem, scale: f64) {
        let (w, h) = crate::render::device_size(form, scale);
        self.render(w, h, |c| form.paint(&mut Painter::new(c, text, scale)));
    }

    /// The pixmap as RapidQ pixels (&HBBGGRR; opaque, so premultiplied =
    /// straight).
    pub fn pixels(&self) -> Pixels {
        let (w, h) = (self.pixmap.width() as usize, self.pixmap.height() as usize);
        let pixels = self.pixmap.data().iter().map(|p| u32::from(p.r) | u32::from(p.g) << 8 | u32::from(p.b) << 16).collect();
        Pixels { width: w, height: h, pixels }
    }

    /// Into a softbuffer frame (0x00RRGGBB per pixel).
    pub fn copy_to(&self, out: &mut [u32]) {
        for (o, p) in out.iter_mut().zip(self.pixmap.data()) {
            *o = u32::from(p.r) << 16 | u32::from(p.g) << 8 | u32::from(p.b);
        }
    }
}

fn clamp16(v: u32) -> u16 {
    v.clamp(1, u32::from(u16::MAX)) as u16
}

/// One capture with the CPU renderer (what `RAPIDR_CAPTURE` writes).
pub fn capture(form: &mut Form, text: &mut TextSystem, scale: f64) -> Pixels {
    let (w, h) = crate::render::device_size(form, scale);
    let mut r = CpuRenderer::new(w, h);
    r.render_form(form, text, scale);
    r.pixels()
}
