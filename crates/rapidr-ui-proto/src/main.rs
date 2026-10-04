//! Prototype of RapidR's own desktop host (ROADMAP Phase 1B, "new desktop
//! host"): a RapidQ QFORM drawn by RapidR's UI kernel instead of FLTK, on
//!
//! - `winit` — the window, mouse, keyboard and input methods;
//! - `vello` on `wgpu` — every pixel, as GPU vector drawing at the
//!   screen's resolution (logical pixels laid out, device pixels drawn);
//! - `parley` — text shaping, layout and editing, from RapidR's built-in
//!   Liberation fonts (the metrics `TextWidth` reports);
//! - `AccessKit` — the accessibility tree screen readers read and operate;
//! - `muda` / `rfd` / `arboard` — the native menu bar, file dialogs and
//!   clipboard, where users notice native.
//!
//! The components are the shared models FLTK and the web runtime already
//! draw (`rapidr_value::objects`): this host renders their ops and routes
//! input to them. It sits next to the FLTK runtime and changes nothing in it.
//!
//! `rapidr-ui-proto [--capture out.bmp] [--bench]` — `RAPIDR_SCALE` forces
//! the scale (device pixels per logical pixel, as the runtime's), and
//! `RAPIDR_CAPTURE=out.bmp` is `--capture`: one frame rendered offscreen
//! and written as a BMP, then exit. `--bench` prints the time from start
//! to the first frame and the average frame over 200 redraws.

mod a11y;
mod demo;
mod form;
mod menu;
mod paint;
mod render;
#[cfg(test)]
mod tests;
mod text;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use vello::util::{RenderContext, RenderSurface};
use vello::wgpu;
use vello::Renderer;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key as WKey, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

use form::{Clipboard, Event, Form, Key, Mods};
use text::TextSystem;

/// Windows' caret blink time (GetCaretBlinkTime).
const BLINK: Duration = Duration::from_millis(530);
/// Redraws `--bench` times.
const BENCH_FRAMES: usize = 200;

struct Options {
    capture: Option<PathBuf>,
    bench: bool,
    /// `--script`: scripted input before the capture.
    script: bool,
    /// RAPIDR_SCALE: the scale, whatever the screen's.
    scale: Option<f64>,
}

fn options() -> Options {
    let mut capture = std::env::var_os("RAPIDR_CAPTURE").map(PathBuf::from);
    let (mut bench, mut script) = (false, false);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--capture" => capture = args.next().map(PathBuf::from),
            "--bench" => bench = true,
            "--script" => script = true,
            _ => {
                eprintln!("usage: rapidr-ui-proto [--capture out.bmp [--script]] [--bench]");
                std::process::exit(2);
            }
        }
    }
    let scale = std::env::var("RAPIDR_SCALE").ok().and_then(|s| s.parse::<f64>().ok()).filter(|s| *s > 0.0 && *s <= 8.0);
    Options { capture, bench, script, scale }
}

fn main() {
    let start = Instant::now();
    let opts = options();
    if let Some(path) = &opts.capture {
        if let Err(e) = capture(path, opts.scale.unwrap_or(1.0), &opts, start) {
            eprintln!("rapidr-ui-proto: {e}");
            std::process::exit(1);
        }
        return;
    }
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().expect("event loop");
    let mut app = App::new(opts, start, event_loop.create_proxy());
    event_loop.run_app(&mut app).expect("event loop");
}

/// `--capture`: one frame offscreen, written as a BMP.
fn capture(path: &PathBuf, scale: f64, opts: &Options, start: Instant) -> Result<(), String> {
    let bench = opts.bench;
    let mut text = TextSystem::new();
    let mut form = demo::form();
    if opts.script {
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

/// What reaches the event loop from other threads.
enum UserEvent {
    AccessKit(accesskit_winit::Event),
    Menu(muda::MenuEvent),
}

impl From<accesskit_winit::Event> for UserEvent {
    fn from(e: accesskit_winit::Event) -> Self {
        UserEvent::AccessKit(e)
    }
}

/// The OS clipboard through arboard.
struct OsClipboard(Option<arboard::Clipboard>);

impl Clipboard for OsClipboard {
    fn get_text(&mut self) -> Option<String> {
        self.0.as_mut()?.get_text().ok()
    }
    fn set_text(&mut self, text: &str) {
        if let Some(c) = self.0.as_mut() {
            c.set_text(text).ok();
        }
    }
}

struct Shown {
    window: Arc<Window>,
    surface: RenderSurface<'static>,
    access: accesskit_winit::Adapter,
    _menu: menu::AppMenu,
}

#[derive(Default)]
struct Bench {
    frames: usize,
    total: Duration,
    scene: Duration,
    worst: Duration,
}

struct App {
    opts: Options,
    start: Instant,
    proxy: EventLoopProxy<UserEvent>,
    ctx: RenderContext,
    /// One renderer per device (by the device's index in `ctx`).
    renderers: Vec<Option<Renderer>>,
    shown: Option<Shown>,
    form: Form,
    text: TextSystem,
    clipboard: OsClipboard,
    clicks: u32,
    mods: ModifiersState,
    cursor: (f64, f64),
    next_blink: Instant,
    first_frame: bool,
    bench: Option<Bench>,
}

impl App {
    fn new(opts: Options, start: Instant, proxy: EventLoopProxy<UserEvent>) -> Self {
        let bench = opts.bench.then(Bench::default);
        App {
            opts,
            start,
            proxy,
            ctx: RenderContext::new(),
            renderers: Vec::new(),
            shown: None,
            form: demo::form(),
            text: TextSystem::new(),
            clipboard: OsClipboard(arboard::Clipboard::new().ok()),
            clicks: 0,
            mods: ModifiersState::empty(),
            cursor: (0.0, 0.0),
            next_blink: Instant::now() + BLINK,
            first_frame: true,
            bench,
        }
    }

    /// Device pixels per logical pixel: RAPIDR_SCALE, else the screen's.
    fn scale(&self) -> f64 {
        self.opts.scale.or_else(|| self.shown.as_ref().map(|s| s.window.scale_factor())).unwrap_or(1.0)
    }

    fn mods(&self) -> Mods {
        let m = self.mods;
        if cfg!(target_os = "macos") {
            Mods { shift: m.shift_key(), command: m.super_key(), word: m.alt_key() }
        } else {
            Mods { shift: m.shift_key(), command: m.control_key(), word: m.control_key() }
        }
    }

    /// The program's handlers ran; the screen and the accessibility tree
    /// follow.
    fn after(&mut self, events: Vec<Event>, redraw: bool) {
        if !events.is_empty() {
            demo::handle(&mut self.form, &events, &mut self.clicks);
        }
        if redraw || !events.is_empty() {
            self.next_blink = Instant::now() + BLINK;
            self.update_access();
            if let Some(s) = &self.shown {
                s.window.request_redraw();
            }
        }
    }

    fn update_access(&mut self) {
        let scale = self.scale();
        let form = &self.form;
        if let Some(s) = &mut self.shown {
            s.access.update_if_active(|| a11y::tree(form, scale));
        }
    }

    fn redraw(&mut self) {
        let frame_start = Instant::now();
        let scale = self.scale();
        let t = Instant::now();
        let scene = render::scene(&mut self.form, &mut self.text, scale);
        let scene_time = t.elapsed();
        let Some(s) = &mut self.shown else { return };
        let dev = s.surface.dev_id;
        let handle = &self.ctx.devices[dev];
        let Some(renderer) = self.renderers.get_mut(dev).and_then(Option::as_mut) else { return };
        let (w, h) = (s.surface.config.width, s.surface.config.height);
        if renderer.render_to_texture(&handle.device, &handle.queue, &scene, &s.surface.target_view, &render::params(w, h)).is_err() {
            return;
        }
        let frame = match s.surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => {
                self.ctx.configure_surface(&s.surface);
                s.window.request_redraw();
                return;
            }
        };
        let mut encoder = handle.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        s.surface.blitter.copy(&handle.device, &mut encoder, &s.surface.target_view, &target);
        handle.queue.submit([encoder.finish()]);
        handle.queue.present(frame);
        if let Some(b) = &mut self.bench {
            // (the GPU's work counted too)
            handle.device.poll(wgpu::PollType::wait_indefinitely()).ok();
            if self.first_frame {
                println!("startup to first frame (window): {:.1} ms", ms(self.start.elapsed()));
            } else {
                let d = frame_start.elapsed();
                b.frames += 1;
                b.total += d;
                b.scene += scene_time;
                b.worst = b.worst.max(d);
            }
            s.window.request_redraw();
        }
        self.first_frame = false;
    }
}

/// The kernel's key for winit's.
fn key(k: &WKey) -> Key {
    match k {
        WKey::Named(n) => match n {
            NamedKey::Tab => Key::Tab,
            NamedKey::Enter => Key::Enter,
            NamedKey::Space => Key::Space,
            NamedKey::Escape => Key::Escape,
            NamedKey::ArrowLeft => Key::Left,
            NamedKey::ArrowRight => Key::Right,
            NamedKey::ArrowUp => Key::Up,
            NamedKey::ArrowDown => Key::Down,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            _ => Key::Other,
        },
        WKey::Character(s) => match s.chars().next() {
            Some(' ') => Key::Space,
            Some(c) => Key::Char(c.to_lowercase().next().unwrap_or(c)),
            None => Key::Other,
        },
        _ => Key::Other,
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.shown.is_some() {
            return;
        }
        let (w, h) = (self.form.width as f64, self.form.height as f64);
        let mut attrs = Window::default_attributes().with_title(self.form.caption.as_str()).with_visible(false);
        attrs = match self.opts.scale {
            Some(s) => attrs.with_inner_size(PhysicalSize::new((w * s).round() as u32, (h * s).round() as u32)),
            None => attrs.with_inner_size(LogicalSize::new(w, h)),
        };
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        // (AccessKit's adapter must exist before the window shows)
        let access = accesskit_winit::Adapter::with_event_loop_proxy(event_loop, &window, self.proxy.clone());
        let proxy = self.proxy.clone();
        let menu = menu::install(move |e| drop(proxy.send_event(UserEvent::Menu(e))), &window);
        window.set_ime_allowed(true);
        window.set_visible(true);
        let size = window.inner_size();
        let present = if self.opts.bench { wgpu::PresentMode::AutoNoVsync } else { wgpu::PresentMode::AutoVsync };
        let surface = pollster::block_on(self.ctx.create_surface(window.clone(), size.width.max(1), size.height.max(1), present)).expect("surface");
        let dev = surface.dev_id;
        self.renderers.resize_with(self.ctx.devices.len(), || None);
        if self.renderers[dev].is_none() {
            self.renderers[dev] = Some(render::renderer(&self.ctx.devices[dev].device).expect("vello renderer"));
        }
        self.shown = Some(Shown { window: window.clone(), surface, access, _menu: menu });
        window.request_redraw();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::AccessKit(e) => match e.window_event {
                accesskit_winit::WindowEvent::InitialTreeRequested => self.update_access(),
                accesskit_winit::WindowEvent::ActionRequested(req) => {
                    let events = a11y::request(&self.form, &req).map(|r| a11y::apply(&mut self.form, r)).unwrap_or_default();
                    self.after(events, true);
                }
                accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
            },
            UserEvent::Menu(e) => {
                if e.id == menu::EXIT {
                    event_loop.exit();
                } else if e.id == menu::OPEN {
                    let picked = rfd::FileDialog::new().set_title("Open").add_filter("RapidQ source", &["bas", "inc"]).pick_file();
                    let msg = picked.map_or("Open: cancelled".to_string(), |p| format!("Open: {}", p.display()));
                    demo::log(&mut self.form, &msg);
                    self.after(Vec::new(), true);
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(s) = &mut self.shown {
            s.access.process_event(&s.window, &event);
        }
        let scale = self.scale();
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(s) = &mut self.shown {
                    if size.width > 0 && size.height > 0 {
                        self.ctx.resize_surface(&mut s.surface, size.width, size.height);
                        // (a sizeable form: its client area follows the window)
                        self.form.width = (f64::from(size.width) / scale).round() as i64;
                        self.form.height = (f64::from(size.height) / scale).round() as i64;
                    }
                }
                self.after(Vec::new(), true);
            }
            WindowEvent::ScaleFactorChanged { .. } => self.after(Vec::new(), true),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x / scale, position.y / scale);
                let (redraw, events) = self.form.mouse_move(self.cursor.0, self.cursor.1, &mut self.text);
                self.after(events, redraw);
            }
            WindowEvent::CursorLeft { .. } => {
                self.form.mouse_leave();
                self.after(Vec::new(), true);
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let (x, y) = self.cursor;
                let events = match state {
                    ElementState::Pressed => self.form.mouse_down(x, y, self.mods(), &mut self.text),
                    ElementState::Released => self.form.mouse_up(x, y),
                };
                self.after(events, true);
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let k = key(&event.logical_key);
                let events = self.form.key(k, self.mods(), event.text.as_deref(), &mut self.text, &mut self.clipboard);
                self.after(events, true);
            }
            WindowEvent::Ime(Ime::Commit(s)) => {
                let events = self.form.commit_text(&s, &mut self.text);
                self.after(events, true);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(b) = self.bench.take_if(|b| b.frames >= BENCH_FRAMES) {
            let n = b.frames as u32;
            let (w, h) = self.shown.as_ref().map_or((0, 0), |s| (s.surface.config.width, s.surface.config.height));
            println!("{} frames in the window at {w}x{h} (no vsync requested): average {:.2} ms (scene {:.2} ms), slowest {:.2} ms", b.frames, ms(b.total / n), ms(b.scene / n), ms(b.worst));
            event_loop.exit();
            return;
        }
        if self.bench.is_some() {
            event_loop.set_control_flow(ControlFlow::Poll);
            return;
        }
        // The caret blinks while an edit has the focus.
        let editing = self.form.focus.is_some_and(|i| matches!(self.form.components[i].kind, form::Kind::Edit(_)));
        if !editing {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        if Instant::now() >= self.next_blink {
            self.form.caret_on = !self.form.caret_on;
            self.next_blink = Instant::now() + BLINK;
            if let Some(s) = &self.shown {
                s.window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_blink));
    }
}
