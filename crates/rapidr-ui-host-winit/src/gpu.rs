//! vello on wgpu: a display list as a vello scene, rendered into a
//! window's surface.

use rapidr_ui_kernel::display::{DisplayList, Picture};
use rapidr_ui_kernel::{FormUi, TextSystem};
use vello::kurbo::{Affine, BezPath, Diagonal2, Rect as KRect, Stroke};
use vello::peniko::Fill;
use vello::wgpu;
use vello::{AaConfig, AaSupport, FontEmbolden, Glyph, RenderParams, Renderer, RendererOptions, Scene};

use crate::canvas::{color, draw_list, Canvas, GlyphRun, BACKGROUND};

/// The GPU renderer's canvas: a vello scene.
impl Canvas for Scene {
    fn fill_rect(&mut self, transform: Affine, rgb: u32, rect: &KRect) {
        self.fill(Fill::NonZero, transform, color(rgb), None, rect);
    }
    fn fill_path(&mut self, transform: Affine, rgb: u32, path: &BezPath) {
        self.fill(Fill::NonZero, transform, color(rgb), None, path);
    }
    fn stroke_path(&mut self, width: f64, rgb: u32, path: &BezPath) {
        self.stroke(&Stroke::new(width), Affine::IDENTITY, color(rgb), None, path);
    }
    fn push_clip(&mut self, transform: Affine, rect: &KRect) {
        self.push_clip_layer(Fill::NonZero, transform, rect);
    }
    fn pop_clip(&mut self) {
        self.pop_layer();
    }
    fn glyphs(&mut self, run: &GlyphRun, glyphs: &[(u32, f32, f32)]) {
        let mut draw = self
            .draw_glyphs(run.font)
            .brush(color(run.rgb))
            .hint(run.hint)
            .transform(run.transform)
            .glyph_transform(run.glyph_transform)
            .font_size(run.size)
            .normalized_coords(run.coords);
        if let Some(a) = run.embolden {
            draw = draw.font_embolden(FontEmbolden::new(Diagonal2::new(a, a)));
        }
        draw.draw(Fill::NonZero, glyphs.iter().map(|&(id, x, y)| Glyph { id, x, y }));
    }
    fn image(&mut self, rect: &KRect, picture: &Picture) {
        use vello::peniko::{Blob, ImageAlphaType, ImageBrush, ImageData, ImageFormat, ImageQuality};
        let (w, h) = (picture.width, picture.height);
        if w == 0 || h == 0 || picture.rgba.len() != w * h * 4 {
            return;
        }
        let data = ImageData { data: Blob::new(std::sync::Arc::new(picture.rgba.clone())), format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::Alpha, width: w as u32, height: h as u32 };
        let crisp = (rect.width() - w as f64).abs() < 0.5 && (rect.height() - h as f64).abs() < 0.5;
        let brush = ImageBrush::new(data).with_quality(if crisp { ImageQuality::Low } else { ImageQuality::Medium });
        self.draw_image(&brush, Affine::translate((rect.x0, rect.y0)) * Affine::scale_non_uniform(rect.width() / w as f64, rect.height() / h as f64));
    }
}

/// A renderer for a device: only area anti-aliasing is compiled (the
/// others' shaders cost startup time and the kernel doesn't use them).
pub fn renderer(device: &wgpu::Device) -> Result<Renderer, String> {
    Renderer::new(device, RendererOptions { antialiasing_support: AaSupport::area_only(), ..Default::default() }).map_err(|e| e.to_string())
}

pub fn params(width: u32, height: u32) -> RenderParams {
    RenderParams { base_color: color(BACKGROUND), width, height, antialiasing_method: AaConfig::Area }
}

/// The display list as a scene.
pub fn scene(list: &DisplayList, text: &mut TextSystem, form: &FormUi) -> Scene {
    let mut scene = Scene::new();
    draw_list(&mut scene, text, list, form);
    scene
}
