//! Prototype of RapidR's own desktop host (ROADMAP Phase 1B, "new desktop
//! host"): RapidQ QFORMs drawn by RapidR's UI kernel instead of FLTK, on
//!
//! - `winit` — the windows, mouse, keyboard and input methods, driven by
//!   `pump_app_events` from the program's own loop (`ui::step`), never by
//!   `run_app`: generated native code is ordinary blocking Rust;
//! - `vello` on `wgpu` — every pixel, as GPU vector drawing at the
//!   screen's resolution; or `vello_cpu` + `softbuffer` on the CPU
//!   (`RAPIDR_RENDERER=cpu`, and every capture);
//! - `parley` — text shaping, layout and editing, from RapidR's built-in
//!   Liberation fonts (the metrics `TextWidth` reports);
//! - `AccessKit` — the accessibility tree screen readers read and operate;
//! - `muda` / `rfd` / `arboard` — the native menu bar, file dialogs and
//!   clipboard, where users notice native.
//!
//! Usage:
//!
//! - `rapidr-ui-proto [--script | --script-file F] [--shots PREFIX]` — the
//!   spike program (`program.rs`): Form1 shown, Form2 shown modally, a
//!   modal inside a handler, a 50 ms timer, an async file dialog, a menu.
//!   `--script` drives it with the built-in script.
//! - `RAPIDR_CAPTURE=PREFIX` (without `RAPIDR_CAPTURE_WINDOWS`): the same
//!   program on the headless host — no OS window, no event loop;
//!   `capture` script steps write `PREFIX-<n>.bmp` with the CPU renderer.
//! - `--capture out.bmp [--script] [--bench]` — the demo form rendered
//!   offscreen on the GPU once (and timed), then exit.
//! - `--compare PREFIX` — the demo form, GPU vs CPU renderer, 1x and 2x.
//!
//! `RAPIDR_SCALE` forces the scale (device pixels per logical pixel),
//! `RAPIDR_TEST_FILE_DIALOG=path` answers file dialogs without UI,
//! `RAPIDR_HOST_TRACE=1` prints winit's own warnings.

mod a11y;
mod compare;
mod cpu;
mod demo;
mod form;
mod host;
mod kernel;
#[cfg(target_os = "macos")]
mod macos_probe;
mod menu;
mod paint;
mod program;
mod render;
mod script;
#[cfg(test)]
mod tests;
mod text;
mod ui;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use text::TextSystem;

/// Redraws `--bench` times.
const BENCH_FRAMES: usize = 200;

fn usage() -> ! {
    eprintln!("usage: rapidr-ui-proto [--script | --script-file F] [--shots PREFIX] | --capture out.bmp [--script] [--bench] | --compare PREFIX");
    std::process::exit(2);
}

fn main() {
    let start = Instant::now();
    if std::env::var_os("RAPIDR_HOST_TRACE").is_some() {
        tracing_subscriber::fmt().with_max_level(tracing_subscriber::filter::LevelFilter::DEBUG).with_writer(std::io::stderr).init();
    }
    let mut capture: Option<PathBuf> = None;
    let (mut bench, mut script, mut script_file, mut shots) = (false, false, None::<String>, None::<String>);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--capture" => capture = Some(args.next().map(PathBuf::from).unwrap_or_else(|| usage())),
            "--bench" => bench = true,
            "--script" => script = true,
            "--script-file" => script_file = Some(args.next().unwrap_or_else(|| usage())),
            "--shots" => shots = Some(args.next().unwrap_or_else(|| usage())),
            "--diff" => {
                let (a, b) = (args.next().unwrap_or_else(|| usage()), args.next().unwrap_or_else(|| usage()));
                match compare::files(&a, &b) {
                    Ok(same) => std::process::exit(if same { 0 } else { 1 }),
                    Err(e) => {
                        eprintln!("rapidr-ui-proto: {e}");
                        std::process::exit(2);
                    }
                }
            }
            "--compare" => {
                let prefix = args.next().unwrap_or_else(|| "compare".into());
                if let Err(e) = compare::run(&prefix) {
                    eprintln!("rapidr-ui-proto: {e}");
                    std::process::exit(1);
                }
                return;
            }
            _ => usage(),
        }
    }
    let scale = std::env::var("RAPIDR_SCALE").ok().and_then(|s| s.parse::<f64>().ok()).filter(|s| *s > 0.0 && *s <= 8.0);
    if let Some(path) = &capture {
        if let Err(e) = offscreen(path, scale.unwrap_or(1.0), script, bench, start) {
            eprintln!("rapidr-ui-proto: {e}");
            std::process::exit(1);
        }
        return;
    }

    let script = match (script_file, script) {
        (Some(f), _) => Some(std::fs::read_to_string(&f).unwrap_or_else(|e| panic!("{f}: {e}"))),
        (None, true) => Some(program::SCRIPT.to_string()),
        _ => None,
    }
    .map(|s| script::Script::parse(&s).unwrap_or_else(|e| panic!("{e}")));
    let env_capture = std::env::var("RAPIDR_CAPTURE").ok().filter(|s| !s.is_empty());
    let headless = env_capture.is_some() && std::env::var_os("RAPIDR_CAPTURE_WINDOWS").is_none();
    let host: Box<dyn host::Host> = if headless {
        Box::new(host::HeadlessHost::new(scale.unwrap_or(1.0)))
    } else {
        let kind = match std::env::var("RAPIDR_RENDERER").as_deref() {
            Ok("cpu") => host::RendererKind::Cpu,
            _ => host::RendererKind::Gpu,
        };
        Box::new(host::WinitHost::new(kind, scale))
    };
    eprintln!("[host] {} (startup {:.0} ms)", host.name(), start.elapsed().as_secs_f64() * 1e3);
    ui::init(host, script, shots.or(env_capture));
    program::main();
}

/// `--capture`: one frame of the demo form offscreen on the GPU, as a BMP.
fn offscreen(path: &PathBuf, scale: f64, script: bool, bench: bool, start: Instant) -> Result<(), String> {
    let mut text = TextSystem::new();
    let mut form = demo::form();
    if script {
        // (laid out at the capture's scale first, as a shown window is)
        drop(render::scene(&mut form, &mut text, scale));
        demo::script(&mut form, &mut text, &mut 0);
    }
    let (w, h) = render::device_size(&form, scale);
    let mut gpu = render::Offscreen::new(w, h)?;
    let scene = render::scene(&mut form, &mut text, scale);
    let pixels = gpu.capture(&scene)?;
    if bench {
        println!("startup to first frame (offscreen, incl. readback): {:.1} ms", start.elapsed().as_secs_f64() * 1e3);
        let (scene_avg, _) = render::time(BENCH_FRAMES, || drop(render::scene(&mut form, &mut text, scale)));
        let (avg, worst) = render::time(BENCH_FRAMES, || {
            let scene = render::scene(&mut form, &mut text, scale);
            gpu.render(&scene).expect("render");
        });
        println!("{BENCH_FRAMES} frames offscreen at {w}x{h}: average {:.2} ms (scene {:.2} ms), slowest {:.2} ms", ms(avg), ms(scene_avg), ms(worst));
    }
    std::fs::write(path, rapidr_value::objects::codec::encode_bmp(&pixels)).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{}: {w}x{h} (form {}x{} at scale {scale})", path.display(), form.width, form.height);
    Ok(())
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}
