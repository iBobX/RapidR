//! `--compare [prefix]` (Stage 0 spike B): the demo form after its script,
//! captured by the GPU renderer (vello on wgpu) and the CPU renderer
//! (vello_cpu) at 1x and 2x; prints per-channel difference statistics and
//! frame timings, and writes `<prefix>-{gpu,cpu}@<scale>x.bmp`.

use std::time::Instant;

use rapidr_value::objects::codec::{encode_bmp, Pixels};

use crate::cpu::CpuRenderer;
use crate::text::TextSystem;
use crate::{demo, render};

const FRAMES: usize = 100;

pub struct Diff {
    pub max: [u32; 3],
    pub over8: usize,
    pub differing: usize,
    pub total: usize,
}

pub fn diff(a: &Pixels, b: &Pixels) -> Diff {
    assert_eq!((a.width, a.height), (b.width, b.height), "sizes differ");
    let mut d = Diff { max: [0; 3], over8: 0, differing: 0, total: a.pixels.len() };
    for (p, q) in a.pixels.iter().zip(&b.pixels) {
        let mut worst = 0;
        for (c, m) in d.max.iter_mut().enumerate() {
            let x = (p >> (8 * c)) & 0xFF;
            let y = (q >> (8 * c)) & 0xFF;
            let e = x.abs_diff(y);
            *m = (*m).max(e);
            worst = worst.max(e);
        }
        if worst > 0 {
            d.differing += 1;
        }
        if worst > 8 {
            d.over8 += 1;
        }
    }
    d
}

/// `--diff A.bmp B.bmp`: the same statistics for two captures.
pub fn files(a: &str, b: &str) -> Result<bool, String> {
    let read = |p: &str| std::fs::read(p).map_err(|e| format!("{p}: {e}")).and_then(|d| rapidr_value::objects::codec::decode_bmp(&d));
    let (pa, pb) = (read(a)?, read(b)?);
    if (pa.width, pa.height) != (pb.width, pb.height) {
        println!("{a} {}x{} vs {b} {}x{}: sizes differ", pa.width, pa.height, pb.width, pb.height);
        return Ok(false);
    }
    let d = diff(&pa, &pb);
    println!("{a} vs {b}: {}x{}, max diff R {} G {} B {}, {} pixels differ, {} by >8", pa.width, pa.height, d.max[2], d.max[1], d.max[0], d.differing, d.over8);
    Ok(d.differing == 0)
}

pub fn run(prefix: &str) -> Result<(), String> {
    for scale in [1.0, 2.0] {
        let mut text = TextSystem::new();
        let mut form = demo::form();
        drop(render::scene(&mut form, &mut text, scale));
        demo::script(&mut form, &mut text, &mut 0);
        let (w, h) = render::device_size(&form, scale);

        let t = Instant::now();
        let mut gpu = render::Offscreen::new(w, h)?;
        let gpu_init = t.elapsed();
        let t = Instant::now();
        let gpu_px = gpu.capture(&render::scene(&mut form, &mut text, scale))?;
        let gpu_first = t.elapsed();
        let (gpu_avg, gpu_worst) = render::time(FRAMES, || {
            let scene = render::scene(&mut form, &mut text, scale);
            gpu.render(&scene).expect("render");
        });

        let t = Instant::now();
        let mut cpu = CpuRenderer::new(w, h);
        cpu.render_form(&mut form, &mut text, scale);
        let cpu_first = t.elapsed();
        let cpu_px = cpu.pixels();
        let (cpu_avg, cpu_worst) = render::time(FRAMES, || cpu.render_form(&mut form, &mut text, scale));

        let d = diff(&gpu_px, &cpu_px);
        let s = scale as u32;
        for (name, px) in [("gpu", &gpu_px), ("cpu", &cpu_px)] {
            let path = format!("{prefix}-{name}@{s}x.bmp");
            std::fs::write(&path, encode_bmp(px)).map_err(|e| format!("{path}: {e}"))?;
        }
        println!(
            "{s}x {w}x{h}: max diff R {} G {} B {}; pixels differing {} ({:.2}%), by >8 {} ({:.3}%)",
            d.max[2],
            d.max[1],
            d.max[0],
            d.differing,
            100.0 * d.differing as f64 / d.total as f64,
            d.over8,
            100.0 * d.over8 as f64 / d.total as f64
        );
        println!("    GPU: device+renderer {:.1} ms, first frame+readback {:.1} ms, {FRAMES} frames avg {:.2} ms (worst {:.2})", ms(gpu_init), ms(gpu_first), ms(gpu_avg), ms(gpu_worst));
        println!("    CPU: first frame {:.1} ms, {FRAMES} frames avg {:.2} ms (worst {:.2}) (scene build + raster)", ms(cpu_first), ms(cpu_avg), ms(cpu_worst));
    }
    Ok(())
}

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1e3
}
