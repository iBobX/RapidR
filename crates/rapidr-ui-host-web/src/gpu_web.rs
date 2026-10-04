//! (feature `gpu`) vello on the browser's WebGPU: the same display list,
//! drawn by the desktop host's GPU path (`gpu.rs`: the scene, area
//! anti-aliasing) through wgpu's WebGPU backend into a `<canvas>`'s
//! context. Where the browser has no WebGPU, [`SpikeGpu::create`] fails and
//! the page keeps the CPU renderer — which also stays what captures and
//! the pixel comparison read (as on the desktop).

use vello::util::{RenderContext, RenderSurface};
use vello::wgpu;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{now_ms, with_text, SpikeForm};

#[wasm_bindgen]
pub struct SpikeGpu {
    cx: RenderContext,
    surface: RenderSurface<'static>,
    renderer: vello::Renderer,
}

#[wasm_bindgen]
impl SpikeGpu {
    /// vello drawing into `canvas` (`width` × `height` device pixels) on
    /// the page's WebGPU adapter; an error where there is none.
    pub async fn create(canvas: HtmlCanvasElement, width: u32, height: u32) -> Result<SpikeGpu, JsValue> {
        let mut cx = RenderContext::new();
        let surface = cx.create_surface(wgpu::SurfaceTarget::Canvas(canvas), width.max(1), height.max(1), wgpu::PresentMode::AutoVsync).await.map_err(|e| JsValue::from_str(&format!("no WebGPU surface: {e}")))?;
        let renderer = crate::gpu::renderer(&cx.devices[surface.dev_id].device).map_err(|e| JsValue::from_str(&format!("vello can't run here: {e}")))?;
        Ok(SpikeGpu { cx, surface, renderer })
    }

    /// The adapter's name and backend (what the page reports).
    pub fn adapter(&self) -> String {
        format!("{:?}", self.cx.devices[self.surface.dev_id].device.adapter_info())
    }

    /// Draws `form` now; the milliseconds the CPU spent (the kernel's paint,
    /// the scene, encoding and submitting — the GPU's own work isn't
    /// waited for), as JSON.
    pub fn render(&mut self, form: &mut SpikeForm) -> Result<String, JsValue> {
        let t0 = now_ms();
        let list = form.display_list();
        let t1 = now_ms();
        let (w, h) = crate::canvas::device_size(&list);
        if (w, h) != (self.surface.config.width, self.surface.config.height) {
            self.cx.resize_surface(&mut self.surface, w, h);
        }
        let scene = with_text(|ts| crate::gpu::scene(&list, ts, form.ui()));
        let handle = &self.cx.devices[self.surface.dev_id];
        self.renderer.render_to_texture(&handle.device, &handle.queue, &scene, &self.surface.target_view, &crate::gpu::params(w, h)).map_err(|e| JsValue::from_str(&format!("vello: {e}")))?;
        let frame = match self.surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            other => return Err(JsValue::from_str(&format!("no frame: {other:?}"))),
        };
        let mut encoder = handle.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.surface.blitter.copy(&handle.device, &mut encoder, &self.surface.target_view, &target);
        handle.queue.submit([encoder.finish()]);
        handle.queue.present(frame);
        let t2 = now_ms();
        Ok(format!("{{\"paint\":{:.3},\"gpu\":{:.3},\"w\":{w},\"h\":{h}}}", t1 - t0, t2 - t1))
    }
}
