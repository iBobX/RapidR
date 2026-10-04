//! The hosts `ui::step` pumps (Stage 0 spikes A and C).
//!
//! - [`WinitHost`]: winit driven by `pump_app_events` from an ordinary
//!   loop (never `run_app`, never `exit()`): one OS window per shown form,
//!   drawn by vello on the GPU or vello_cpu + softbuffer
//!   (`RAPIDR_RENDERER=cpu`), AccessKit, the muda menu bar, rfd async
//!   dialogs created inside the pump.
//! - [`HeadlessHost`]: no OS windows, no event loop, no NSApplication;
//!   `pump(t)` sleeps until `t`; the screen is a fixed 1920 x 1080.
//!
//! The callbacks ([`Shim`]) never run program code: they route input into
//! the kernel (which queues `HostEvent`s), render, and run `HostCmd`s.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::task::{Wake, Waker};
use std::time::{Duration, Instant};

use vello::util::{RenderContext, RenderSurface};
use vello::wgpu;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key as WKey, ModifiersState, NamedKey};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Window, WindowId};

use crate::cpu::CpuRenderer;
use crate::form::{Clipboard, Key, Mods};
use crate::kernel::{Dialog, FormId, Held, HostCmd, HostEvent, Kernel};
use crate::{a11y, menu, render};

/// The headless screen (logical pixels).
pub const HEADLESS_SCREEN: (i64, i64) = (1920, 1080);

pub trait Host {
    /// Runs the OS's event loop for up to `timeout` (None: until
    /// something happens), its callbacks writing into `k`.
    fn pump(&mut self, timeout: Option<Duration>, k: &mut Kernel);
    /// Device pixels per logical pixel for a form's window.
    fn scale(&self, f: FormId) -> f64;
    /// Wakes the pump (for futures: async dialogs).
    fn waker(&self) -> Waker;
    fn headless(&self) -> bool;
    /// Window commands that must run *outside* a pump, for the spike's
    /// "blocking rfd between pumps" check.
    fn name(&self) -> &'static str;
    /// A line of evidence about the host for the spike's report.
    fn report(&self) -> String {
        String::new()
    }
    /// The form's native view (macOS: its NSView), for the probes.
    fn native_view(&self, _f: FormId) -> Option<std::ptr::NonNull<std::ffi::c_void>> {
        None
    }
}

/// AppKit's NSApp global: nil until something makes the NSApplication
/// (winit's EventLoop, muda, rfd, arboard ...).
#[cfg(target_os = "macos")]
fn nsapp_exists() -> bool {
    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {
        static NSApp: *const std::ffi::c_void;
    }
    // SAFETY: a plain read of AppKit's global pointer on the main thread.
    unsafe { !NSApp.is_null() }
}

// ------------------------------------------------------------- headless --

pub struct HeadlessHost {
    scale: f64,
}

impl HeadlessHost {
    pub fn new(scale: f64) -> Self {
        HeadlessHost { scale }
    }
}

impl Host for HeadlessHost {
    fn pump(&mut self, timeout: Option<Duration>, k: &mut Kernel) {
        k.stats.pumps += 1;
        for cmd in std::mem::take(&mut k.cmds) {
            if let HostCmd::OpenFileDialog { id, .. } = cmd {
                // (no OS: RAPIDR_TEST_FILE_DIALOG answers, else Cancel)
                let answer = std::env::var_os("RAPIDR_TEST_FILE_DIALOG").map(Into::into);
                if let Some(d) = k.dialogs.iter_mut().find(|d| d.0 == id) {
                    d.1 = Dialog::Done(answer);
                }
                k.events.push(HostEvent::Wake);
            }
        }
        if !k.events.is_empty() {
            return;
        }
        match timeout {
            Some(t) => std::thread::sleep(t),
            None => {
                eprintln!("headless host: the program waits for input and nothing is scheduled (no timer, no script step)");
                std::process::exit(3);
            }
        }
    }
    fn scale(&self, _f: FormId) -> f64 {
        self.scale
    }
    fn waker(&self) -> Waker {
        Waker::noop().clone()
    }
    fn headless(&self) -> bool {
        true
    }
    fn name(&self) -> &'static str {
        "headless"
    }
    fn report(&self) -> String {
        #[cfg(target_os = "macos")]
        return format!(
            "headless: screen {}x{} at scale {}; NSApplication created: {} (NSApp is {})",
            HEADLESS_SCREEN.0,
            HEADLESS_SCREEN.1,
            self.scale,
            nsapp_exists(),
            if nsapp_exists() { "set" } else { "nil" }
        );
        #[cfg(not(target_os = "macos"))]
        String::from("headless: no event loop created")
    }
}

// ---------------------------------------------------------------- winit --

pub enum UserEvent {
    AccessKit(accesskit_winit::Event),
    Menu(muda::MenuEvent),
    Wake,
}

impl From<accesskit_winit::Event> for UserEvent {
    fn from(e: accesskit_winit::Event) -> Self {
        UserEvent::AccessKit(e)
    }
}

struct ProxyWaker(Mutex<EventLoopProxy<UserEvent>>);
impl Wake for ProxyWaker {
    fn wake(self: Arc<Self>) {
        if let Ok(p) = self.0.lock() {
            p.send_event(UserEvent::Wake).ok();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RendererKind {
    Gpu,
    Cpu,
}

#[allow(clippy::large_enum_variant)] // (one per window)
enum Surface {
    Gpu(RenderSurface<'static>),
    Cpu { surface: softbuffer::Surface<Arc<Window>, Arc<Window>>, r: CpuRenderer },
}

struct Win {
    window: Arc<Window>,
    surface: Surface,
    access: accesskit_winit::Adapter,
    cursor: (f64, f64),
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

/// Per-pump instrumentation.
#[derive(Default)]
struct PumpStats {
    resized: u32,
    redraws: u32,
    first: Option<(i64, i64)>,
    last: Option<(i64, i64)>,
}

struct State {
    proxy: EventLoopProxy<UserEvent>,
    kind: RendererKind,
    scale: Option<f64>,
    gpu: RenderContext,
    renderers: Vec<Option<vello::Renderer>>,
    wins: HashMap<FormId, Win>,
    ids: HashMap<WindowId, FormId>,
    menu: Option<menu::AppMenu>,
    mods: ModifiersState,
    clipboard: OsClipboard,
    pump: PumpStats,
}

pub struct WinitHost {
    event_loop: EventLoop<UserEvent>,
    state: State,
}

impl WinitHost {
    pub fn new(kind: RendererKind, scale: Option<f64>) -> Self {
        let event_loop = EventLoop::<UserEvent>::with_user_event().build().expect("event loop");
        let proxy = event_loop.create_proxy();
        WinitHost {
            event_loop,
            state: State {
                proxy,
                kind,
                scale,
                gpu: RenderContext::new(),
                renderers: Vec::new(),
                wins: HashMap::new(),
                ids: HashMap::new(),
                menu: None,
                mods: ModifiersState::empty(),
                clipboard: OsClipboard(arboard::Clipboard::new().ok()),
                pump: PumpStats::default(),
            },
        }
    }
}

impl Host for WinitHost {
    fn pump(&mut self, timeout: Option<Duration>, k: &mut Kernel) {
        // Forms the program changed since the last pump: repaint them.
        for (&f, w) in &self.state.wins {
            if k.forms[f].dirty {
                w.window.request_redraw();
            }
        }
        k.stats.pumps += 1;
        self.state.pump = PumpStats::default();
        let start = Instant::now();
        k.in_callback = true;
        let status = self.event_loop.pump_app_events(timeout, &mut Shim { s: &mut self.state, k });
        k.in_callback = false;
        if let winit::platform::pump_events::PumpStatus::Exit(code) = status {
            // (never asked for: the program ends by exiting the process)
            eprintln!("winit host: event loop exited ({code})");
            std::process::exit(code);
        }
        let took = start.elapsed();
        let over = took.saturating_sub(timeout.unwrap_or(Duration::ZERO));
        // (with no timeout a long pump is just waiting for input)
        if timeout.is_some() && over > Duration::from_millis(40) {
            let p = &self.state.pump;
            let held = Held { over, resized: p.resized, redraws: p.redraws, sizes: p.first.zip(p.last) };
            eprintln!(
                "[host] pump held {:.0} ms past its {:.0} ms timeout: {} Resized, {} RedrawRequested inside it{}",
                over.as_secs_f64() * 1e3,
                timeout.unwrap_or_default().as_secs_f64() * 1e3,
                held.resized,
                held.redraws,
                held.sizes.map_or(String::new(), |(a, b)| format!(" (client {}x{} -> {}x{}, laid out and painted live)", a.0, a.1, b.0, b.1))
            );
            k.stats.held_pumps.push(held);
        }
    }

    fn scale(&self, f: FormId) -> f64 {
        self.state.scale.or_else(|| self.state.wins.get(&f).map(|w| w.window.scale_factor())).unwrap_or(1.0)
    }

    fn waker(&self) -> Waker {
        Waker::from(Arc::new(ProxyWaker(Mutex::new(self.state.proxy.clone()))))
    }

    fn headless(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        match self.state.kind {
            RendererKind::Gpu => "winit+vello(gpu)",
            RendererKind::Cpu => "winit+vello_cpu+softbuffer",
        }
    }
    fn report(&self) -> String {
        #[cfg(target_os = "macos")]
        return format!("winit: NSApplication created: {}", nsapp_exists());
        #[cfg(not(target_os = "macos"))]
        String::new()
    }
    fn native_view(&self, f: FormId) -> Option<std::ptr::NonNull<std::ffi::c_void>> {
        ns_view(&self.state.wins.get(&f)?.window)
    }
}

/// A winit window's native view (macOS: its NSView).
pub fn ns_view(window: &Window) -> Option<std::ptr::NonNull<std::ffi::c_void>> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().ok()?.as_raw() {
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(h) => Some(h.ns_view),
        _ => None,
    }
}

/// The `ApplicationHandler` for one pump: the host's state and the
/// kernel, borrowed for the pump's duration.
struct Shim<'a> {
    s: &'a mut State,
    k: &'a mut Kernel,
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

impl Shim<'_> {
    fn mods(&self) -> Mods {
        let m = self.s.mods;
        if cfg!(target_os = "macos") {
            Mods { shift: m.shift_key(), command: m.super_key(), word: m.alt_key() }
        } else {
            Mods { shift: m.shift_key(), command: m.control_key(), word: m.control_key() }
        }
    }

    fn scale_of(&self, f: FormId) -> f64 {
        self.s.scale.or_else(|| self.s.wins.get(&f).map(|w| w.window.scale_factor())).unwrap_or(1.0)
    }

    /// Runs the program's window commands (an `ActiveEventLoop` is at hand).
    fn apply(&mut self, el: &ActiveEventLoop) {
        for cmd in std::mem::take(&mut self.k.cmds) {
            match cmd {
                HostCmd::Show(f) => self.show(el, f),
                HostCmd::Hide(f) => {
                    if let Some(w) = self.s.wins.remove(&f) {
                        self.s.ids.remove(&w.window.id());
                    }
                    // (the modal underneath gets the focus back)
                    if let Some(w) = self.k.modal.last().and_then(|m| self.s.wins.get(m)) {
                        w.window.focus_window();
                    }
                }
                HostCmd::SetSize(f) => {
                    let scale = self.scale_of(f);
                    if let Some(w) = self.s.wins.get(&f) {
                        let form = &self.k.forms[f].form;
                        let (pw, ph) = render::device_size(form, scale);
                        let _ = w.window.request_inner_size(PhysicalSize::new(pw, ph));
                    }
                }
                HostCmd::OpenFileDialog { id, parent } => {
                    // Inside the pump NSApp is running, so rfd makes a sheet
                    // and returns at once; its completion wakes the pump.
                    let mut d = rfd::AsyncFileDialog::new().set_title("Open (async)").add_filter("RapidQ source", &["bas", "inc"]);
                    if let Some(w) = self.s.wins.get(&parent) {
                        d = d.set_parent(&*w.window);
                    }
                    let fut = d.pick_file();
                    let fut = Box::pin(async move { fut.await.map(|h| h.path().to_path_buf()) });
                    if let Some(slot) = self.k.dialogs.iter_mut().find(|d| d.0 == id) {
                        slot.1 = Dialog::Pending(fut);
                    }
                    // (the program polls it after this pump)
                    self.k.events.push(HostEvent::Wake);
                }
                HostCmd::BlockingDialogInCallback => {
                    eprintln!("[host] blocking rfd dialog inside a winit callback (about_to_wait) ...");
                    let t = Instant::now();
                    let picked = rfd::FileDialog::new().set_title("Open (blocking, in a callback)").pick_file();
                    eprintln!("[host] ... returned after {:.0} ms: {picked:?}", t.elapsed().as_secs_f64() * 1e3);
                }
            }
        }
    }

    fn show(&mut self, el: &ActiveEventLoop, f: FormId) {
        if self.s.wins.contains_key(&f) {
            if let Some(w) = self.s.wins.get(&f) {
                w.window.focus_window();
            }
            return;
        }
        let form = &self.k.forms[f].form;
        let (w, h) = (form.width as f64, form.height as f64);
        let mut attrs = Window::default_attributes().with_title(form.caption.as_str()).with_visible(false);
        attrs = match self.s.scale {
            Some(s) => attrs.with_inner_size(PhysicalSize::new((w * s).round() as u32, (h * s).round() as u32)),
            None => attrs.with_inner_size(winit::dpi::LogicalSize::new(w, h)),
        };
        // (cascade, so the forms don't hide each other)
        attrs = attrs.with_position(winit::dpi::LogicalPosition::new(120.0 + 60.0 * f as f64, 120.0 + 50.0 * f as f64));
        let window = Arc::new(el.create_window(attrs).expect("window"));
        // (AccessKit's adapter must exist before the window shows)
        let access = accesskit_winit::Adapter::with_event_loop_proxy(el, &window, self.s.proxy.clone());
        if self.s.menu.is_none() {
            let proxy = self.s.proxy.clone();
            self.s.menu = Some(menu::install(move |e| drop(proxy.send_event(UserEvent::Menu(e))), &window));
        }
        window.set_ime_allowed(true);
        window.set_visible(true);
        let size = window.inner_size();
        let surface = match self.s.kind {
            RendererKind::Gpu => {
                let surface = pollster::block_on(self.s.gpu.create_surface(window.clone(), size.width.max(1), size.height.max(1), wgpu::PresentMode::AutoVsync)).expect("surface");
                let dev = surface.dev_id;
                self.s.renderers.resize_with(self.s.gpu.devices.len(), || None);
                if self.s.renderers[dev].is_none() {
                    self.s.renderers[dev] = Some(render::renderer(&self.s.gpu.devices[dev].device).expect("vello renderer"));
                }
                Surface::Gpu(surface)
            }
            RendererKind::Cpu => {
                let ctx = softbuffer::Context::new(window.clone()).expect("softbuffer context");
                let surface = softbuffer::Surface::new(&ctx, window.clone()).expect("softbuffer surface");
                Surface::Cpu { surface, r: CpuRenderer::new(size.width, size.height) }
            }
        };
        self.s.ids.insert(window.id(), f);
        window.request_redraw();
        self.s.wins.insert(f, Win { window, surface, access, cursor: (0.0, 0.0) });
    }

    fn redraw(&mut self, f: FormId) {
        let scale = self.scale_of(f);
        // (counted when asked for: an occluded GPU surface skips the frame)
        self.k.stats.redraws += 1;
        self.s.pump.redraws += 1;
        let State { wins, gpu, renderers, .. } = &mut *self.s;
        let Some(w) = wins.get_mut(&f) else { return };
        let k = &mut self.k.forms[f];
        let text = &mut self.k.text;
        k.dirty = false;
        match &mut w.surface {
            Surface::Gpu(surface) => {
                let scene = render::scene(&mut k.form, text, scale);
                let dev = surface.dev_id;
                let handle = &gpu.devices[dev];
                let Some(renderer) = renderers.get_mut(dev).and_then(Option::as_mut) else {
                    return;
                };
                let (pw, ph) = (surface.config.width, surface.config.height);
                if let Err(e) = renderer.render_to_texture(&handle.device, &handle.queue, &scene, &surface.target_view, &render::params(pw, ph)) {
                    eprintln!("[host] vello render failed: {e}");
                    return;
                }
                let frame = match surface.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                    other => {
                        if std::env::var_os("RAPIDR_HOST_LOG").is_some() {
                            eprintln!("[host] no surface texture for form {f}: {other:?}");
                        }
                        // (occluded / timed out: skip the frame; outdated / lost:
                        // reconfigure and try again)
                        if !matches!(other, wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout) {
                            gpu.configure_surface(surface);
                            w.window.request_redraw();
                        }
                        return;
                    }
                };
                let mut encoder = handle.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                let target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                surface.blitter.copy(&handle.device, &mut encoder, &surface.target_view, &target);
                handle.queue.submit([encoder.finish()]);
                handle.queue.present(frame);
            }
            Surface::Cpu { surface, r } => {
                let t = std::time::Instant::now();
                let size = w.window.inner_size();
                let (Some(pw), Some(ph)) = (std::num::NonZeroU32::new(size.width), std::num::NonZeroU32::new(size.height)) else {
                    return;
                };
                r.render(size.width, size.height, |c| k.form.paint(&mut crate::paint::Painter::new(c, text, scale)));
                if surface.resize(pw, ph).is_err() {
                    return;
                }
                let Ok(mut buf) = surface.buffer_mut() else {
                    return;
                };
                r.copy_to(&mut buf);
                if let Err(e) = buf.present() {
                    eprintln!("[host] softbuffer present failed: {e}");
                }
                self.k.stats.cpu_frames += 1;
                self.k.stats.cpu_frame_time += t.elapsed();
            }
        }
        let form = &k.form;
        w.access.update_if_active(|| a11y::tree(form, scale));
    }

    fn after_input(&mut self, f: FormId) {
        if self.k.forms[f].dirty {
            if let Some(w) = self.s.wins.get(&f) {
                w.window.request_redraw();
            }
        }
    }
}

impl ApplicationHandler<UserEvent> for Shim<'_> {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        self.apply(el);
    }

    fn new_events(&mut self, el: &ActiveEventLoop, _cause: winit::event::StartCause) {
        self.apply(el);
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        self.apply(el);
        el.set_control_flow(ControlFlow::Wait);
    }

    fn user_event(&mut self, _el: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::AccessKit(e) => {
                let Some(&f) = self.s.ids.get(&e.window_id) else {
                    return;
                };
                match e.window_event {
                    accesskit_winit::WindowEvent::InitialTreeRequested => {
                        let scale = self.scale_of(f);
                        let form = &self.k.forms[f].form;
                        if let Some(w) = self.s.wins.get_mut(&f) {
                            w.access.update_if_active(|| a11y::tree(form, scale));
                        }
                    }
                    accesskit_winit::WindowEvent::ActionRequested(req) => {
                        if self.k.accepts_input(f) {
                            let form = &mut self.k.forms[f].form;
                            let ev = a11y::request(form, &req).map(|r| a11y::apply(form, r)).unwrap_or_default();
                            self.k.forms[f].dirty = true;
                            self.k.events.extend(ev.into_iter().map(|e| HostEvent::Kernel(f, e)));
                            self.after_input(f);
                        }
                    }
                    accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
                }
            }
            UserEvent::Menu(e) => self.k.events.push(HostEvent::Menu(e.id.0)),
            UserEvent::Wake => self.k.events.push(HostEvent::Wake),
        }
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(&f) = self.s.ids.get(&id) else {
            return;
        };
        if let Some(w) = self.s.wins.get_mut(&f) {
            w.access.process_event(&w.window, &event);
        }
        let scale = self.scale_of(f);
        let user = !self.k.ignore_user;
        match event {
            WindowEvent::CloseRequested if user => self.k.close_box(f),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    if let Some(w) = self.s.wins.get_mut(&f) {
                        match &mut w.surface {
                            Surface::Gpu(s) => self.s.gpu.resize_surface(s, size.width, size.height),
                            Surface::Cpu { .. } => {}
                        }
                    }
                    let (lw, lh) = ((f64::from(size.width) / scale).round() as i64, (f64::from(size.height) / scale).round() as i64);
                    #[cfg(target_os = "macos")]
                    if let Some(v) = self.s.wins.get(&f).and_then(|w| crate::host::ns_view(&w.window)) {
                        if crate::macos_probe::in_live_resize(v) {
                            self.k.stats.live_resizes += 1;
                        }
                    }
                    self.k.resized(f, lw, lh);
                    let p = &mut self.s.pump;
                    p.resized += 1;
                    p.first.get_or_insert((lw, lh));
                    p.last = Some((lw, lh));
                    // (AppKit's live resize: draw now, inside the tracking loop)
                    self.redraw(f);
                }
            }
            WindowEvent::Occluded(false) => {
                if let Some(w) = self.s.wins.get(&f) {
                    w.window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.k.forms[f].dirty = true;
                self.after_input(f);
            }
            WindowEvent::RedrawRequested => {
                if std::env::var_os("RAPIDR_HOST_LOG").is_some() {
                    eprintln!("[host] RedrawRequested form {f}");
                }
                self.redraw(f)
            }
            WindowEvent::Focused(true) => {
                // A modal form keeps the focus (macOS has no owned windows).
                if let Some(&m) = self.k.modal.last() {
                    if m != f {
                        if let Some(w) = self.s.wins.get(&m) {
                            w.window.focus_window();
                        }
                    }
                }
            }
            WindowEvent::ModifiersChanged(m) => self.s.mods = m.state(),
            WindowEvent::CursorLeft { .. } if user => {
                self.k.forms[f].form.mouse_leave();
                self.k.forms[f].dirty = true;
                self.after_input(f);
            }
            WindowEvent::CursorMoved { position, .. } if user => {
                let (x, y) = (position.x / scale, position.y / scale);
                if let Some(w) = self.s.wins.get_mut(&f) {
                    w.cursor = (x, y);
                }
                self.k.mouse_move(f, x, y);
                self.after_input(f);
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if user => {
                let (x, y) = self.s.wins.get(&f).map_or((0.0, 0.0), |w| w.cursor);
                match state {
                    ElementState::Pressed => {
                        let m = self.mods();
                        self.k.mouse_down(f, x, y, m)
                    }
                    ElementState::Released => self.k.mouse_up(f, x, y),
                }
                self.after_input(f);
            }
            WindowEvent::KeyboardInput { event, .. } if user && event.state == ElementState::Pressed => {
                let k = key(&event.logical_key);
                let m = self.mods();
                self.k.key(f, k, m, event.text.as_deref(), &mut self.s.clipboard);
                self.after_input(f);
            }
            WindowEvent::Ime(Ime::Commit(s)) if user => {
                self.k.commit_text(f, &s);
                self.after_input(f);
            }
            _ => {}
        }
    }
}
