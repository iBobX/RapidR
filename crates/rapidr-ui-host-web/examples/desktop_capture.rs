//! The desktop's pixels of the spike's forms, for the pixel comparison
//! (`tests/web_host_spike.mjs`): each form of `forms::FORMS` built as the
//! browser builds it, painted by the kernel and rasterized by the desktop
//! host's own CPU path (`rapidr_ui_host_winit::cpu::capture`, what the
//! headless host's `RAPIDR_CAPTURE` saves) at 1× and 2×.
//!
//! Twice: with the system's fonts as fallbacks (`<dir>/desktop`, what the
//! desktop shows: CJK and symbols the built-in Liberation fonts lack come
//! from the system), and with the built-in fonts alone
//! (`<dir>/desktop-builtin`, what a browser has: wasm can't read the
//! system's fonts).
//!
//! Usage: `cargo run -p rapidr-ui-host-web --example desktop_capture --release -- <dir>`
//! writes `<dir>/desktop*/<form>@<scale>x.rgba`: width and height (u32,
//! little endian), then the pixels as RGBA.

use std::sync::Arc;

use parley::fontique::{Blob, Collection, CollectionOptions};
use rapidr_ui_host_web::forms;
use rapidr_ui_kernel::{FormUi, TextSystem};

/// The kernel's text system; without the system's fonts, the built-in
/// ones alone (as `TextSystem::new` registers them).
fn text_system(system_fonts: bool) -> TextSystem {
    if system_fonts {
        return TextSystem::new();
    }
    let mut font_cx = parley::FontContext { collection: Collection::new(CollectionOptions { shared: false, system_fonts: false }), source_cache: Default::default() };
    for data in rapidr_value::objects::text::BUILTIN_FONTS {
        font_cx.collection.register_fonts(Blob::new(Arc::new(data)), None);
    }
    TextSystem { font_cx, layout_cx: parley::LayoutContext::new() }
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "target/web-host-spike".into());
    for (sub, system_fonts) in [("desktop", true), ("desktop-builtin", false)] {
        let out_dir = format!("{dir}/{sub}");
        std::fs::create_dir_all(&out_dir).expect("output directory");
        let mut ts = text_system(system_fonts);
        for scale in [1.0, 2.0] {
            for &(name, id) in forms::FORMS.iter().chain([&forms::BIG]) {
                // (canvases draw at the screen's scale, as the browser's do)
                rapidr_value::objects::bitmap::set_display_scale(scale);
                let store = forms::build(name).expect("a spike form");
                // (as the web host builds it: an in-window menu bar, the
                // caret steadily on)
                let mut ui = FormUi::build(&store, id, true);
                ui.blinks = false;
                let t0 = std::time::Instant::now();
                let list = ui.paint(&store, &mut ts, scale);
                let px = rapidr_ui_host_winit::cpu::capture(&list, &mut ts, &ui);
                let ms = t0.elapsed().as_secs_f64() * 1000.0;
                let mut out = Vec::with_capacity(8 + px.pixels.len() * 4);
                out.extend_from_slice(&(px.width as u32).to_le_bytes());
                out.extend_from_slice(&(px.height as u32).to_le_bytes());
                for p in &px.pixels {
                    // (&HBBGGRR → RGBA)
                    out.extend_from_slice(&[(*p & 0xFF) as u8, (*p >> 8 & 0xFF) as u8, (*p >> 16 & 0xFF) as u8, 0xFF]);
                }
                let file = format!("{out_dir}/{name}@{scale}x.rgba");
                std::fs::write(&file, out).expect("write the capture");
                // The desktop's per-frame cost, as the browser's is measured
                // (60 frames: the kernel's paint, then vello_cpu).
                let (w, h) = rapidr_ui_host_winit::canvas::device_size(&list);
                let mut r = rapidr_ui_host_winit::cpu::CpuRenderer::new(w, h);
                let (mut paint, mut raster) = (0.0, 0.0);
                for _ in 0..60 {
                    let t = std::time::Instant::now();
                    let list = ui.paint(&store, &mut ts, scale);
                    let t1 = std::time::Instant::now();
                    r.render(w, h, &list, &mut ts, &ui);
                    paint += (t1 - t).as_secs_f64() * 1000.0 / 60.0;
                    raster += t1.elapsed().as_secs_f64() * 1000.0 / 60.0;
                }
                println!("{file}: {}x{} (first frame {ms:.2} ms; then kernel paint {paint:.3} + vello_cpu {raster:.3} ms a frame)", px.width, px.height);
            }
        }
    }
}
