//! vello on wgpu: a form's scene rendered into a window's surface, or
//! offscreen into a texture read back as RapidQ pixels (`--capture`, the
//! headless check, and what a form's `GetImage` / screenshot tools would
//! use).

use std::time::{Duration, Instant};

use rapidr_value::objects::codec::Pixels;
use vello::util::RenderContext;
use vello::wgpu;
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene};

use crate::form::{Form, FACE};
use crate::paint::{color, Painter};
use crate::text::TextSystem;

/// A renderer for a device: only area anti-aliasing is compiled (the
/// others' shaders cost startup time and the kernel doesn't use them).
pub fn renderer(device: &wgpu::Device) -> Result<Renderer, String> {
    Renderer::new(device, RendererOptions { antialiasing_support: AaSupport::area_only(), ..Default::default() }).map_err(|e| e.to_string())
}

pub fn params(width: u32, height: u32) -> RenderParams {
    RenderParams { base_color: color(FACE), width, height, antialiasing_method: AaConfig::Area }
}

/// The form drawn at `scale` (device pixels per logical pixel).
pub fn scene(form: &mut Form, text: &mut TextSystem, scale: f64) -> Scene {
    let mut scene = Scene::new();
    form.paint(&mut Painter::new(&mut scene, text, scale));
    scene
}

/// The form's client area in device pixels at `scale`.
pub fn device_size(form: &Form, scale: f64) -> (u32, u32) {
    (((form.width as f64) * scale).round().max(1.0) as u32, ((form.height as f64) * scale).round().max(1.0) as u32)
}

/// Rendering without a window.
pub struct Offscreen {
    ctx: RenderContext,
    dev: usize,
    renderer: Renderer,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

impl Offscreen {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let mut ctx = RenderContext::new();
        let dev = pollster::block_on(ctx.device(None)).ok_or("no GPU adapter (wgpu found none)")?;
        let device = &ctx.devices[dev].device;
        let renderer = renderer(device)?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rapidr capture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Offscreen { ctx, dev, renderer, texture, view, width, height })
    }

    /// Renders and waits for the GPU (what a frame costs, end to end).
    pub fn render(&mut self, scene: &Scene) -> Result<(), String> {
        let handle = &self.ctx.devices[self.dev];
        self.renderer.render_to_texture(&handle.device, &handle.queue, scene, &self.view, &params(self.width, self.height)).map_err(|e| e.to_string())?;
        handle.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Renders and reads the pixels back (&HBBGGRR, as `codec::Pixels`).
    pub fn capture(&mut self, scene: &Scene) -> Result<Pixels, String> {
        self.render(scene)?;
        let handle = &self.ctx.devices[self.dev];
        let (w, h) = (self.width, self.height);
        let row = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = handle.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rapidr capture readback"),
            size: u64::from(row * h),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = handle.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &self.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: None } },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        handle.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        handle.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
        let data = slice.get_mapped_range().map_err(|e| e.to_string())?;
        let mut pixels = Vec::with_capacity((w * h) as usize);
        for y in 0..h as usize {
            for px in data[y * row as usize..][..w as usize * 4].as_chunks::<4>().0 {
                pixels.push(u32::from(px[0]) | u32::from(px[1]) << 8 | u32::from(px[2]) << 16);
            }
        }
        Ok(Pixels { width: w as usize, height: h as usize, pixels })
    }
}

/// Times `n` runs of `f`: (average, slowest).
pub fn time(n: usize, mut f: impl FnMut()) -> (Duration, Duration) {
    let mut total = Duration::ZERO;
    let mut worst = Duration::ZERO;
    for _ in 0..n {
        let t = Instant::now();
        f();
        let d = t.elapsed();
        total += d;
        worst = worst.max(d);
    }
    (total / n.max(1) as u32, worst)
}
