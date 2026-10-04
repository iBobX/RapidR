//! The CPU renderer: display lists rasterized by vello_cpu (vello's
//! sparse-strips CPU renderer, on the same peniko / kurbo / skrifa as the
//! GPU one). Captures always use it (deterministic; no GPU or window server
//! needed: the headless host); a window shows its frames through softbuffer
//! where wgpu finds no GPU, or with `RAPIDR_RENDERER=cpu`.

use rapidr_ui_kernel::display::{DisplayList, Picture};
use rapidr_ui_kernel::{FormUi, TextSystem};
use rapidr_value::objects::codec::Pixels;
use vello::kurbo::{Affine, BezPath, Rect as KRect, Stroke};
use vello_cpu::{Pixmap, RenderContext, RenderMode, Resources};

use crate::canvas::{color, draw_list, Canvas, GlyphRun, BACKGROUND};

/// A vello_cpu render context being drawn into.
struct CpuCanvas<'a> {
    ctx: &'a mut RenderContext,
    res: &'a mut Resources,
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
    fn image(&mut self, rect: &KRect, picture: &Picture) {
        let (w, h) = (picture.width, picture.height);
        if w == 0 || h == 0 || w > usize::from(u16::MAX) || h > usize::from(u16::MAX) || picture.rgba.len() != w * h * 4 {
            return;
        }
        let meta = vello_cpu::PixelMetadata { may_have_transparency: true, alpha_type: vello_cpu::peniko::ImageAlphaType::Alpha };
        let pixmap = Pixmap::from_parts(picture.rgba.clone(), w as u16, h as u16, meta);
        let sampler = vello_cpu::peniko::ImageSampler::new().with_quality(image_quality(rect, w, h));
        let image = vello_cpu::Image { image: vello_cpu::ImageSource::Pixmap(std::sync::Arc::new(pixmap)), sampler };
        self.ctx.set_transform(Affine::translate((rect.x0, rect.y0)) * Affine::scale_non_uniform(rect.width() / w as f64, rect.height() / h as f64));
        self.ctx.set_paint_transform(Affine::IDENTITY);
        self.ctx.set_paint(image);
        self.ctx.fill_rect(&KRect::new(0.0, 0.0, w as f64, h as f64));
    }
}

/// Pixel for pixel when the picture is drawn at its own size (crisp), else
/// smoothed.
pub(crate) fn image_quality(rect: &KRect, w: usize, h: usize) -> vello_cpu::peniko::ImageQuality {
    if (rect.width() - w as f64).abs() < 0.5 && (rect.height() - h as f64).abs() < 0.5 {
        vello_cpu::peniko::ImageQuality::Low
    } else {
        vello_cpu::peniko::ImageQuality::Medium
    }
}

/// vello_cpu with its persistent resources (glyph caches) and a target.
pub struct CpuRenderer {
    ctx: RenderContext,
    res: Resources,
    pub pixmap: Pixmap,
}

fn clamp16(v: u32) -> u16 {
    v.clamp(1, u32::from(u16::MAX)) as u16
}

impl CpuRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        let (w, h) = (clamp16(width), clamp16(height));
        CpuRenderer { ctx: RenderContext::new(w, h), res: Resources::new(), pixmap: Pixmap::new(w, h) }
    }

    /// `list` drawn into the pixmap, `width` × `height` device pixels.
    pub fn render(&mut self, width: u32, height: u32, list: &DisplayList, text: &mut TextSystem, form: &FormUi) {
        let (w, h) = (clamp16(width), clamp16(height));
        if (w, h) != (self.ctx.width(), self.ctx.height()) {
            self.ctx = RenderContext::new(w, h);
            self.pixmap = Pixmap::new(w, h);
        } else {
            self.ctx.reset();
        }
        draw_list(&mut CpuCanvas { ctx: &mut self.ctx, res: &mut self.res }, text, list, form);
        self.ctx.flush();
        let settings = vello_cpu::RasterizerSettings { render_mode: RenderMode::OptimizeSpeed, target_init: vello_cpu::TargetInit::Clear(color(BACKGROUND)), ..Default::default() };
        self.ctx.render_with(&mut self.pixmap, &mut self.res, settings);
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

/// One capture with the CPU renderer (what `RAPIDR_CAPTURE` writes).
pub fn capture(list: &DisplayList, text: &mut TextSystem, form: &FormUi) -> Pixels {
    let (w, h) = crate::canvas::device_size(list);
    let mut r = CpuRenderer::new(w, h);
    r.render(w, h, list, text, form);
    r.pixels()
}
