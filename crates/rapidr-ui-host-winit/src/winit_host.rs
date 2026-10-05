//! winit driven by `pump_app_events` from runtime-core's `step` — never
//! `run_app`, never `exit()` (an exited loop can't come back; the program
//! ends by ending the process). One OS window per shown form, made inside
//! a pump (where an `ActiveEventLoop` is at hand), its AccessKit adapter
//! before it shows; drawn by vello on the GPU, or vello_cpu + softbuffer.
//!
//! The callbacks ([`Shim`]) never run program code: they run the program's
//! [`HostCmd`]s, route input into the kernel ([`Desktop`]), render, and
//! record resizes, moves and scale changes. An occluded (or timed-out)
//! frame is skipped without asking for another; the window asks again when
//! it shows (Stage 0).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::task::{Wake, Waker};
use std::time::Duration;

use rapidr_ui_kernel::{Mods, Store};
use rapidr_ui_render::cpu::CpuRenderer;
use rapidr_ui_render::gpu;
use rapidr_value::input::Button;
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key as WKey, ModifiersState, NamedKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowId};

use crate::menu::NativeMenus;
use crate::{a11y, Desktop, Host, HostCmd, HostEvent, RendererKind, Source};

pub enum UserEvent {
    AccessKit(accesskit_winit::Event),
    Wake,
    /// (timers during native menu tracking) The system holds the pump: a
    /// tick is due (tracking.rs).
    Tick,
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

#[allow(clippy::large_enum_variant)] // (one per window)
enum Surface {
    Gpu(RenderSurface<'static>),
    Cpu { surface: softbuffer::Surface<Arc<Window>, Arc<Window>>, r: CpuRenderer },
}

struct Win {
    window: Arc<Window>,
    surface: Surface,
    access: accesskit_winit::Adapter,
    sent: a11y::Sent,
    /// The mouse in the window's inside (logical).
    cursor: (f64, f64),
    /// The pointer shown over it (platform.rs; set only when it changes).
    pointer: Option<rapidr_value::input::Cursor>,
    /// Input methods allowed (an edit has the focus).
    ime: bool,
}

struct State {
    proxy: EventLoopProxy<UserEvent>,
    kind: RendererKind,
    /// `RAPIDR_SCALE`: every window at this scale.
    forced: Option<f64>,
    gpu: RenderContext,
    renderers: Vec<Option<vello::Renderer>>,
    wins: HashMap<String, Win>,
    ids: HashMap<WindowId, String>,
    mods: ModifiersState,
    /// The primary monitor (logical size, scale) and how many there are.
    screen: Option<((i64, i64), f64, i64)>,
    /// The mouse on the screen (logical), as last seen.
    mouse: (i64, i64),
    /// The system's menus (menu.rs) and the form whose window has the
    /// keyboard (macOS' menu bar shows its main menu).
    menus: NativeMenus,
    key_form: Option<String>,
    /// Open / Save dialogs (dialogs.rs) and the waker their completion
    /// wakes the pump with.
    dialogs: crate::dialogs::Dialogs,
    waker: Waker,
    /// (timers during native menu tracking) runtime-core's turn while the
    /// system holds a pump.
    hook: Option<crate::tracking::Hook>,
}

pub struct WinitHost {
    event_loop: EventLoop<UserEvent>,
    state: State,
}

impl WinitHost {
    /// The process's event loop (made once, on the main thread); an error
    /// when the system has no display to give it.
    pub fn new(kind: RendererKind, forced: Option<f64>) -> Result<Self, winit::error::EventLoopError> {
        let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
        let proxy = event_loop.create_proxy();
        let menus = NativeMenus::new(proxy.clone());
        let waker = Waker::from(Arc::new(ProxyWaker(Mutex::new(proxy.clone()))));
        // (a held pump's tick, outside the host's context menu: delivered by
        // winit as soon as no callback runs — tracking.rs)
        let ticker = proxy.clone();
        crate::tracking::set_waker(Box::new(move || {
            ticker.send_event(UserEvent::Tick).ok();
        }));
        let gpu = RenderContext::new();
        // A GPU that's software — Windows' WARP, Mesa's llvmpipe / lavapipe,
        // SwiftShader: virtual machines, remote desktops, servers — runs
        // vello's compute shaders slower than vello_cpu draws, and WARP
        // crashes in them (an access violation in d3d10warp.dll on Windows
        // 11 ARM in Parallels): the CPU draws instead.
        let kind = if kind == RendererKind::Gpu && !RendererKind::gpu_asked() && software_gpu_only(&gpu.instance) {
            RendererKind::Cpu
        } else {
            kind
        };
        Ok(WinitHost {
            event_loop,
            state: State {
                proxy,
                kind,
                forced,
                gpu,
                renderers: Vec::new(),
                wins: HashMap::new(),
                ids: HashMap::new(),
                mods: ModifiersState::empty(),
                screen: None,
                mouse: (0, 0),
                menus,
                key_form: None,
                dialogs: crate::dialogs::Dialogs::default(),
                waker,
                hook: None,
            },
        })
    }

    fn monitor(&self) -> ((i64, i64), f64, i64) {
        self.state.screen.unwrap_or(((1920, 1080), 1.0, 1))
    }
}

/// Whether every adapter wgpu finds is software (or there's none).
fn software_gpu_only(instance: &wgpu::Instance) -> bool {
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    adapters.iter().all(|a| a.get_info().device_type == wgpu::DeviceType::Cpu)
}

impl Host for WinitHost {
    fn pump(&mut self, timeout: Option<Duration>, desk: &mut Desktop, store: &dyn Store) {
        // Forms the program changed since the last pump: drawn again.
        for (f, w) in &self.state.wins {
            if desk.forms.get(f).is_some_and(|k| k.shown && k.ui.dirty) {
                w.window.request_redraw();
            }
        }
        // (timers during native menu tracking: if the pump overstays its
        // deadline, the system holds it, and the program's timers tick from
        // the tracking timer — tracking.rs)
        let held = crate::tracking::begin_pump(timeout.map(|t| std::time::Instant::now() + t));
        // (macOS: the tracking timer is the pump's alarm clock — winit waits
        // for anything, the timer's firing wakes the loop and the pump comes
        // back; winit's own timer would spin inside a tracked menu)
        let winit_timeout = if crate::tracking::OWN_DEADLINE { timeout.filter(Duration::is_zero) } else { timeout };
        let status = self.event_loop.pump_app_events(winit_timeout, &mut Shim { s: &mut self.state, desk, store });
        drop(held);
        if let Some(panic) = crate::tracking::take_panic() {
            std::panic::resume_unwind(panic);
        }
        if let PumpStatus::Exit(code) = status {
            // (never asked for: the program ends by ending the process)
            eprintln!("[rapidr] winit host: the event loop exited ({code})");
            std::process::exit(code);
        }
    }

    fn screen(&self) -> (i64, i64) {
        self.monitor().0
    }

    fn monitors(&self) -> i64 {
        self.monitor().2
    }

    fn work_area(&self) -> (i64, i64) {
        crate::platform::work_area().unwrap_or_else(|| self.monitor().0)
    }

    /// The mouse anywhere on the screen (platform.rs); where the platform
    /// can't say (Wayland), as last seen over the program's windows.
    fn mouse(&self) -> (i64, i64) {
        crate::platform::global_mouse().map_or(self.state.mouse, |(x, y)| (x.round() as i64, y.round() as i64))
    }

    fn default_scale(&self) -> f64 {
        self.state.forced.unwrap_or(self.monitor().1)
    }

    fn waker(&self) -> Waker {
        Waker::from(Arc::new(ProxyWaker(Mutex::new(self.state.proxy.clone()))))
    }

    fn headless(&self) -> bool {
        false
    }

    fn native_menus(&self) -> bool {
        crate::menu::native_popups()
    }

    fn file_dialog(&mut self, id: u64) -> Option<Vec<String>> {
        let State { dialogs, waker, .. } = &mut self.state;
        dialogs.take(id, waker)
    }

    fn set_tracking_hook(&mut self, hook: crate::tracking::Hook) {
        self.state.hook = Some(hook);
    }

    fn name(&self) -> &'static str {
        match self.state.kind {
            RendererKind::Gpu => "winit+vello",
            RendererKind::Cpu => "winit+vello_cpu",
        }
    }
}

/// The `ApplicationHandler` for one pump: the host's state, the kernel's
/// forms and the program's store, borrowed for the pump.
struct Shim<'a> {
    s: &'a mut State,
    desk: &'a mut Desktop,
    store: &'a dyn Store,
}

/// Windows' virtual-key code for a winit key.
fn vk_of(key: &WKey) -> i64 {
    match key {
        WKey::Named(n) => match n {
            NamedKey::Backspace => 8,
            NamedKey::Tab => 9,
            NamedKey::Enter => 13,
            NamedKey::Shift => 16,
            NamedKey::Control => 17,
            NamedKey::Alt => 18,
            NamedKey::Pause => 19,
            NamedKey::CapsLock => 20,
            NamedKey::Escape => 27,
            NamedKey::Space => 32,
            NamedKey::PageUp => 33,
            NamedKey::PageDown => 34,
            NamedKey::End => 35,
            NamedKey::Home => 36,
            NamedKey::ArrowLeft => 37,
            NamedKey::ArrowUp => 38,
            NamedKey::ArrowRight => 39,
            NamedKey::ArrowDown => 40,
            NamedKey::PrintScreen => 44,
            NamedKey::Insert => 45,
            NamedKey::Delete => 46,
            NamedKey::Super | NamedKey::Meta => 91,
            NamedKey::ContextMenu => 93,
            NamedKey::NumLock => 144,
            NamedKey::ScrollLock => 145,
            NamedKey::F1 => 112,
            NamedKey::F2 => 113,
            NamedKey::F3 => 114,
            NamedKey::F4 => 115,
            NamedKey::F5 => 116,
            NamedKey::F6 => 117,
            NamedKey::F7 => 118,
            NamedKey::F8 => 119,
            NamedKey::F9 => 120,
            NamedKey::F10 => 121,
            NamedKey::F11 => 122,
            NamedKey::F12 => 123,
            _ => 0,
        },
        WKey::Character(s) => s.chars().next().and_then(rapidr_value::input::vk_of_char).unwrap_or(0),
        _ => 0,
    }
}

impl Shim<'_> {
    fn mods(&self) -> Mods {
        let m = self.s.mods;
        let (shift, ctrl, alt) = (m.shift_key(), m.control_key(), m.alt_key());
        if cfg!(target_os = "macos") {
            Mods { shift, ctrl, alt, command: m.super_key(), word: alt }
        } else {
            Mods { shift, ctrl, alt, command: ctrl, word: ctrl }
        }
    }

    /// Form `f`'s window's inside is `size` now: the surface follows, and
    /// the kernel hears it as the window system's resize. (Also when
    /// `request_inner_size` answers that it applied a size at once — as on
    /// Wayland — since then no `Resized` follows.)
    fn size_applied(&mut self, f: &str, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        let scale = self.scale_of(f);
        if let Some(w) = self.s.wins.get_mut(f) {
            if let Surface::Gpu(s) = &mut w.surface {
                self.s.gpu.resize_surface(s, size.width, size.height);
            }
        }
        let (lw, lh) = ((f64::from(size.width) / scale).round() as i64, (f64::from(size.height) / scale).round() as i64);
        self.desk.resized(f, lw, lh);
        if let Some(w) = self.s.wins.get(f) {
            w.window.request_redraw();
        }
    }

    /// Form `f`'s window's scale.
    fn scale_of(&self, f: &str) -> f64 {
        self.s.forced.or_else(|| self.s.wins.get(f).map(|w| w.window.scale_factor())).unwrap_or(1.0)
    }

    fn note_monitor(&mut self, el: &ActiveEventLoop) {
        if self.s.screen.is_some() {
            return;
        }
        let count = el.available_monitors().count().max(1) as i64;
        if let Some(m) = el.primary_monitor().or_else(|| el.available_monitors().next()) {
            let f = m.scale_factor();
            let size = m.size().to_logical::<f64>(f);
            self.s.screen = Some(((size.width.round() as i64, size.height.round() as i64), f, count));
            crate::platform::set_primary_scale(f);
        }
    }

    /// Runs the program's window commands.
    fn apply(&mut self, el: &ActiveEventLoop) {
        self.note_monitor(el);
        // (timers during native menu tracking: while the system holds the
        // pump — macOS' menu bar tracked, winit's observers calling in — no
        // second loop of the system's starts inside it; a context menu or a
        // sheet waits for the next pump)
        let held = crate::tracking::held();
        let mut later = Vec::new();
        for cmd in std::mem::take(&mut self.desk.cmds) {
            match cmd {
                cmd @ (HostCmd::Popup { .. } | HostCmd::FileDialog { .. }) if held => later.push(cmd),
                HostCmd::Show(f) => self.show(el, &f),
                HostCmd::Hide(f) => {
                    if let Some(w) = self.s.wins.get(&f) {
                        w.window.set_visible(false);
                    }
                    // (the modal underneath gets the focus back)
                    if let Some(w) = self.desk.modal.last().and_then(|m| self.s.wins.get(m)) {
                        w.window.focus_window();
                    }
                }
                HostCmd::Title(f) => {
                    if let (Some(w), Some(k)) = (self.s.wins.get(&f), self.desk.forms.get(&f)) {
                        w.window.set_title(&k.spec.title);
                    }
                }
                HostCmd::Size(f) => {
                    let scale = self.scale_of(&f);
                    if let (Some(w), Some(k)) = (self.s.wins.get(&f), self.desk.forms.get(&f)) {
                        let (lw, lh) = k.spec.size;
                        if let Some(size) = w.window.request_inner_size(self.inner_size(lw, lh, scale)) {
                            self.size_applied(&f, size);
                        }
                    }
                }
                HostCmd::Position(f) => {
                    if let (Some(w), Some(k)) = (self.s.wins.get(&f), self.desk.forms.get(&f)) {
                        if let Some((x, y)) = k.spec.position {
                            w.window.set_outer_position(LogicalPosition::new(x as f64, y as f64));
                        }
                    }
                }
                HostCmd::Border(f) => {
                    if let (Some(w), Some(k)) = (self.s.wins.get(&f), self.desk.forms.get_mut(&f)) {
                        w.window.set_decorations(k.spec.border);
                        w.window.set_resizable(k.spec.frame.resizable);
                        w.window.set_enabled_buttons(k.spec.frame.buttons());
                        k.ui.system_corner = system_corner(k.spec.border);
                        k.ui.dirty = true;
                    }
                }
                HostCmd::Icon(f) => {
                    if let (Some(w), Some(k)) = (self.s.wins.get(&f), self.desk.forms.get(&f)) {
                        w.window.set_window_icon(icon(k.spec.icon.as_ref()));
                    }
                }
                HostCmd::Minimize(f) => {
                    if let Some(w) = self.s.wins.get(&f) {
                        w.window.set_minimized(true);
                    }
                }
                HostCmd::Popup { form, menu, x, y } => {
                    if let Some(w) = self.s.wins.get(&form) {
                        let window = w.window.clone();
                        // (the menu holds the pump until the user lets go:
                        // its own tick runs the program's timers meanwhile —
                        // this callback's state lent to it, the menus taken
                        // out for the call)
                        let mut menus = std::mem::take(&mut self.s.menus);
                        crate::tracking::with_popup_tick(&mut || self.tracking_tick(), || menus.popup(&window, &form, &menu, x, y));
                        self.s.menus = menus;
                    }
                }
                // (the dialogs lane's: made here, inside the pump — a sheet on
                // macOS; the program asks for the answer after each step)
                HostCmd::FileDialog { id, form, req } => {
                    let parent = form.and_then(|f| self.s.wins.get(&f)).map(|w| w.window.clone());
                    let State { dialogs, waker, .. } = &mut *self.s;
                    dialogs.open(id, &req, parent.as_deref(), waker);
                    self.desk.events.push(HostEvent::Wake);
                }
                // (the input lane's: a size grip dragged — the system resizes,
                // and its Resized is the user's)
                HostCmd::Resize { form, w: lw, h: lh } => {
                    let scale = self.scale_of(&form);
                    if let Some(w) = self.s.wins.get(&form) {
                        if let Some(size) = w.window.request_inner_size(self.inner_size(lw, lh, scale)) {
                            self.size_applied(&form, size);
                        }
                    }
                }
                // (the WindowState lane's: the system maximizes, minimizes
                // or restores it; what it became comes back through
                // `note_state`)
                HostCmd::State(f) => {
                    if let (Some(w), Some(k)) = (self.s.wins.get(&f), self.desk.forms.get_mut(&f)) {
                        apply_state(&w.window, k.spec.state);
                        k.state = k.spec.state;
                    }
                }
                HostCmd::Forget(f) => {
                    if let Some(w) = self.s.wins.remove(&f) {
                        self.s.ids.remove(&w.window.id());
                        w.window.set_visible(false);
                    }
                    if self.s.key_form.as_deref() == Some(f.as_str()) {
                        self.s.key_form = None;
                    }
                    if let Some(w) = self.desk.modal.last().and_then(|m| self.s.wins.get(m)) {
                        w.window.focus_window();
                    }
                }
            }
        }
        if !later.is_empty() {
            later.append(&mut self.desk.cmds);
            self.desk.cmds = later;
        }
        // (menus: macOS' bar for the key form — not built again under the
        // user's mouse; picks made meanwhile)
        if !held {
            let key = self.s.key_form.clone();
            self.s.menus.sync(self.desk, self.store, key.as_deref());
        }
        crate::menu::take_picks(self.desk);
    }

    /// (timers during native menu tracking) A tick while the system holds
    /// the pump: runtime-core's turn — the due timers' handlers, what they
    /// changed into the kernel — then the forms whose display changed drawn
    /// now (the menu is over them, the next pump far off). Window commands
    /// wait for the next `apply`.
    fn tracking_tick(&mut self) {
        let Some(mut hook) = self.s.hook.take() else { return };
        let turn = hook(self.desk, self.store);
        self.s.hook = Some(hook);
        let dirty: Vec<String> = self.s.wins.keys().filter(|f| self.desk.forms.get(*f).is_some_and(|k| k.shown && k.ui.dirty)).cloned().collect();
        for f in dirty {
            self.redraw(&f);
        }
        crate::tracking::after_tick(turn);
    }

    /// A window's inside for a logical size: at the forced scale in device
    /// pixels, else logical (the window system scales it).
    fn inner_size(&self, w: i64, h: i64, _scale: f64) -> winit::dpi::Size {
        let (w, h) = (w.max(1) as f64, h.max(1) as f64);
        match self.s.forced {
            Some(s) => PhysicalSize::new((w * s).round() as u32, (h * s).round() as u32).into(),
            None => LogicalSize::new(w, h).into(),
        }
    }

    fn show(&mut self, el: &ActiveEventLoop, f: &str) {
        self.s.key_form = Some(f.to_string());
        if let Some(w) = self.s.wins.get(f) {
            w.window.set_visible(true);
            w.window.focus_window();
            w.window.request_redraw();
            return;
        }
        let Some(k) = self.desk.forms.get(f) else { return };
        let spec = k.spec.clone();
        let mut attrs = Window::default_attributes()
            .with_title(spec.title.as_str())
            .with_visible(false)
            .with_decorations(spec.border)
            .with_resizable(spec.frame.resizable)
            .with_enabled_buttons(spec.frame.buttons())
            .with_inner_size(self.inner_size(spec.size.0, spec.size.1, 1.0))
            .with_window_icon(icon(spec.icon.as_ref()))
            // (the WindowState lane's: a form shown maximized)
            .with_maximized(spec.state == 2);
        if let Some((x, y)) = spec.position {
            attrs = attrs.with_position(LogicalPosition::new(x as f64, y as f64));
        }
        let window = match el.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("[rapidr] can't make a window for {f}: {e}");
                return;
            }
        };
        // (AccessKit's adapter must exist before the window shows. A screen
        // reader's first question is answered at once, with the form as it
        // is now — the full tree follows with the next pump, as it did alone)
        let scale = self.s.forced.unwrap_or_else(|| window.scale_factor());
        let first = {
            let Desktop { forms, text, .. } = &mut *self.desk;
            forms.get_mut(f).and_then(|k| a11y::update(&k.ui.access_tree(self.store, text), scale, &mut a11y::Sent::default(), true))
        };
        let initial = a11y::FirstTree::new(first, window.id(), self.s.proxy.clone());
        let access = accesskit_winit::Adapter::with_mixed_handlers(el, &window, initial, self.s.proxy.clone());
        window.set_visible(true);
        window.focus_window();
        let size = window.inner_size();
        let surface = self.surface(&window, size);
        self.s.ids.insert(window.id(), f.to_string());
        // (the WindowState lane's: shown minimized)
        if spec.state == 1 {
            window.set_minimized(true);
        }
        let scale = self.s.forced.unwrap_or_else(|| window.scale_factor());
        if let Some(k) = self.desk.forms.get_mut(f) {
            k.scale = scale;
            k.ui.dirty = true;
            k.state = spec.state;
            k.ui.system_corner = system_corner(spec.border);
        }
        window.request_redraw();
        self.s.wins.insert(f.to_string(), Win { window, surface, access, sent: a11y::Sent::default(), cursor: (0.0, 0.0), pointer: None, ime: false });
    }

    /// The window's surface: the GPU's unless it has none (or the CPU was
    /// asked for).
    fn surface(&mut self, window: &Arc<Window>, size: PhysicalSize<u32>) -> Surface {
        if self.s.kind == RendererKind::Gpu {
            match pollster::block_on(self.s.gpu.create_surface(window.clone(), size.width.max(1), size.height.max(1), wgpu::PresentMode::AutoVsync)) {
                Ok(surface) => {
                    let dev = surface.dev_id;
                    self.s.renderers.resize_with(self.s.gpu.devices.len(), || None);
                    if self.s.renderers[dev].is_none() {
                        match gpu::renderer(&self.s.gpu.devices[dev].device) {
                            Ok(r) => self.s.renderers[dev] = Some(r),
                            Err(e) => {
                                eprintln!("[rapidr] vello can't run on this GPU ({e}): drawing on the CPU");
                                self.s.kind = RendererKind::Cpu;
                            }
                        }
                    }
                    if self.s.kind == RendererKind::Gpu {
                        return Surface::Gpu(surface);
                    }
                }
                Err(e) => {
                    eprintln!("[rapidr] no GPU surface ({e}): drawing on the CPU");
                    self.s.kind = RendererKind::Cpu;
                }
            }
        }
        let ctx = softbuffer::Context::new(window.clone()).expect("softbuffer context");
        let surface = softbuffer::Surface::new(&ctx, window.clone()).expect("softbuffer surface");
        Surface::Cpu { surface, r: CpuRenderer::new(size.width, size.height) }
    }

    fn redraw(&mut self, f: &str) {
        let scale = self.scale_of(f);
        let State { wins, gpu, renderers, .. } = &mut *self.s;
        let Some(w) = wins.get_mut(f) else { return };
        let Desktop { forms, text, .. } = &mut *self.desk;
        let Some(k) = forms.get_mut(f) else { return };
        k.scale = scale;
        let list = k.ui.paint(self.store, text, scale);
        match &mut w.surface {
            Surface::Gpu(surface) => {
                let scene = gpu::scene(&list, text, &k.ui);
                let dev = surface.dev_id;
                let handle = &gpu.devices[dev];
                let Some(renderer) = renderers.get_mut(dev).and_then(Option::as_mut) else { return };
                let (pw, ph) = (surface.config.width, surface.config.height);
                if let Err(e) = renderer.render_to_texture(&handle.device, &handle.queue, &scene, &surface.target_view, &gpu::params(pw, ph)) {
                    eprintln!("[rapidr] vello render failed: {e}");
                    return;
                }
                let frame = match surface.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                    other => {
                        if std::env::var_os("RAPIDR_HOST_LOG").is_some() {
                            eprintln!("[rapidr] {f}: no frame from the GPU surface ({other:?})");
                        }
                        // (occluded / timed out: skip the frame, without asking
                        // for another; outdated / lost: configure and ask again)
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
                let size = w.window.inner_size();
                let (Some(pw), Some(ph)) = (std::num::NonZeroU32::new(size.width), std::num::NonZeroU32::new(size.height)) else { return };
                r.render(size.width, size.height, &list, text, &k.ui);
                if surface.resize(pw, ph).is_err() {
                    return;
                }
                let Ok(mut buf) = surface.buffer_mut() else { return };
                r.copy_to(&mut buf);
                if let Err(e) = buf.present() {
                    eprintln!("[rapidr] softbuffer present failed: {e}");
                }
            }
        }
        // (input methods only while an edit has the focus, so other
        // components' keys aren't swallowed; their window at the caret)
        let wants = k.ui.wants_ime(self.store);
        if wants != w.ime {
            w.window.set_ime_allowed(wants);
            w.ime = wants;
        }
        if wants {
            if let Some((x, y, cw, ch)) = k.ui.ime_area(self.store, text) {
                w.window.set_ime_cursor_area(LogicalPosition::new(x as f64, y as f64), LogicalSize::new(cw.max(1) as f64, ch.max(1) as f64));
            }
        }
        // (a screen reader listening: the nodes that changed)
        let (ui, sent, store) = (&mut k.ui, &mut w.sent, self.store);
        w.access.update_if_active(|| {
            let tree = ui.access_tree(store, text);
            a11y::update(&tree, scale, sent, false).unwrap_or_else(|| a11y::unchanged(sent))
        });
    }

    /// The pointer over form `f`'s window at `at` (platform.rs), set when
    /// it changes.
    fn pointer(&mut self, f: &str, at: (f64, f64)) {
        let c = crate::platform::cursor_at(self.desk, self.store, f, at);
        if let Some(w) = self.s.wins.get_mut(f) {
            if w.pointer != Some(c) {
                w.pointer = Some(c);
                match crate::platform::cursor_icon(c) {
                    Some(icon) => {
                        w.window.set_cursor_visible(true);
                        w.window.set_cursor(icon);
                    }
                    None => w.window.set_cursor_visible(false),
                }
            }
        }
    }

    /// (the WindowState lane's) What form `f`'s window is now — minimized,
    /// maximized or neither — told to the program when it changed (the
    /// user's own maximize, restore or minimize).
    fn note_state(&mut self, f: &str) {
        let Some(w) = self.s.wins.get(f) else { return };
        let now = if w.window.is_minimized() == Some(true) {
            1
        } else if w.window.is_maximized() {
            2
        } else {
            0
        };
        self.desk.window_state(f, now);
    }

    fn after_input(&mut self, f: &str) {
        if self.desk.forms.get(f).is_some_and(|k| k.ui.dirty) {
            if let Some(w) = self.s.wins.get(f) {
                w.window.request_redraw();
            }
        }
    }
}

/// Whether a window's bottom-right corner is the system's: macOS rounds a
/// titled window's corners off (by about 16 points on macOS 26) and resizes
/// it from them itself — a status bar's size grip drawn there would be cut
/// off (`FormUi::system_corner`).
fn system_corner(border: bool) -> bool {
    cfg!(target_os = "macos") && border
}

/// (the WindowState lane's) wsNormal 0 / wsMinimized 1 / wsMaximized 2 as
/// the system's own: restored, minimized, maximized.
fn apply_state(window: &Window, state: i64) {
    match state {
        1 => window.set_minimized(true),
        2 => {
            window.set_minimized(false);
            window.set_maximized(true);
        }
        _ => {
            window.set_minimized(false);
            window.set_maximized(false);
        }
    }
}

fn icon(icon: Option<&crate::Icon>) -> Option<winit::window::Icon> {
    let i = icon?;
    winit::window::Icon::from_rgba(i.rgba.clone(), i.width, i.height).ok()
}

fn button(b: MouseButton) -> Option<Button> {
    match b {
        MouseButton::Left => Some(Button::Left),
        MouseButton::Right => Some(Button::Right),
        MouseButton::Middle => Some(Button::Middle),
        _ => None,
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
                let Some(f) = self.s.ids.get(&e.window_id).cloned() else { return };
                match e.window_event {
                    accesskit_winit::WindowEvent::InitialTreeRequested => {
                        let scale = self.scale_of(&f);
                        let Desktop { forms, text, .. } = &mut *self.desk;
                        if let (Some(k), Some(w)) = (forms.get_mut(&f), self.s.wins.get_mut(&f)) {
                            let tree = k.ui.access_tree(self.store, text);
                            let sent = &mut w.sent;
                            w.access.update_if_active(|| a11y::update(&tree, scale, sent, true).expect("a full tree"));
                        }
                    }
                    accesskit_winit::WindowEvent::ActionRequested(req) => {
                        if let Some((target, action, value)) = a11y::request(&req) {
                            self.desk.access_action(self.store, &f, target, action, value);
                            self.after_input(&f);
                        }
                    }
                    accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
                }
            }
            UserEvent::Wake => self.desk.events.push(HostEvent::Wake),
            // (timers during native menu tracking: a held pump's tick, no
            // other callback on the stack)
            UserEvent::Tick => {
                if crate::tracking::woken() {
                    crate::tracking::run_tick(|| self.tracking_tick());
                }
            }
        }
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(f) = self.s.ids.get(&id).cloned() else { return };
        if let Some(w) = self.s.wins.get_mut(&f) {
            w.access.process_event(&w.window, &event);
        }
        let scale = self.scale_of(&f);
        let store = self.store;
        match event {
            WindowEvent::CloseRequested => self.desk.close_box(&f, Source::User),
            WindowEvent::Resized(size) => {
                self.size_applied(&f, size);
                self.note_state(&f);
            }
            WindowEvent::Moved(p) => {
                if let Some(w) = self.s.wins.get(&f) {
                    let pos = p.to_logical::<f64>(w.window.scale_factor());
                    self.desk.moved(&f, pos.x.round() as i64, pos.y.round() as i64);
                }
                self.note_state(&f);
            }
            WindowEvent::Occluded(occluded) => {
                if !occluded {
                    if let Some(w) = self.s.wins.get(&f) {
                        w.window.request_redraw();
                    }
                }
                // (minimized or back: the WindowState lane's)
                self.note_state(&f);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if self.s.forced.is_none() {
                    self.desk.scale_changed(&f, scale_factor);
                }
                self.after_input(&f);
            }
            WindowEvent::RedrawRequested => self.redraw(&f),
            WindowEvent::Focused(false) => {
                // (a kernel-drawn menu closes when its window loses the
                // keyboard, as Windows' menus do)
                if let Some(k) = self.desk.forms.get_mut(&f) {
                    if k.ui.menu_open() {
                        k.ui.close_menus();
                    }
                }
                self.after_input(&f);
            }
            WindowEvent::Focused(true) => {
                self.s.key_form = Some(f.clone());
                self.note_state(&f);
                // A modal form keeps the focus (macOS has no owned windows).
                if let Some(m) = self.desk.modal.last() {
                    if *m != f {
                        if let Some(w) = self.s.wins.get(m) {
                            w.window.focus_window();
                        }
                    }
                }
            }
            WindowEvent::ModifiersChanged(m) => self.s.mods = m.state(),
            WindowEvent::CursorLeft { .. } => {
                self.desk.mouse_leave(store, &f, Source::User);
                self.after_input(&f);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x / scale, position.y / scale);
                if let Some(w) = self.s.wins.get_mut(&f) {
                    w.cursor = (x, y);
                    if let Ok(p) = w.window.inner_position() {
                        let p = p.to_logical::<f64>(w.window.scale_factor());
                        self.s.mouse = ((p.x + x).round() as i64, (p.y + y).round() as i64);
                    }
                }
                let m = self.mods();
                self.desk.mouse_move(store, &f, x, y, m, Source::User);
                self.pointer(&f, (x, y));
                self.after_input(&f);
            }
            WindowEvent::MouseInput { state, button: b, .. } => {
                let Some(b) = button(b) else { return };
                let (x, y) = self.s.wins.get(&f).map_or((0.0, 0.0), |w| w.cursor);
                let m = self.mods();
                match state {
                    ElementState::Pressed => self.desk.mouse_down(store, &f, (x, y), b, m, Source::User),
                    ElementState::Released => self.desk.mouse_up(store, &f, (x, y), b, m, Source::User),
                }
                self.after_input(&f);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // (winit: positive moves the content right / down, i.e. the
                // view up; RapidR's notches are positive down. A touchpad's
                // pixels: 48 logical a notch, three lines)
                let (dx, dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (-f64::from(x), -f64::from(y)),
                    MouseScrollDelta::PixelDelta(p) => {
                        let p = p.to_logical::<f64>(scale);
                        (-p.x / 48.0, -p.y / 48.0)
                    }
                };
                let at = self.s.wins.get(&f).map_or((0.0, 0.0), |w| w.cursor);
                let m = self.mods();
                self.desk.mouse_wheel(store, &f, at, (dx, dy), m, Source::User);
                self.after_input(&f);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let vk = vk_of(&event.logical_key);
                let m = self.mods();
                if event.state == ElementState::Pressed {
                    let text = event.text.as_deref().filter(|t| !t.chars().any(char::is_control)).unwrap_or("");
                    self.desk.key_down(store, &f, vk, text, m, Source::User);
                } else {
                    self.desk.key_up(&f, vk, m, Source::User);
                }
                self.after_input(&f);
            }
            WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                self.desk.ime_preedit(store, &f, &text, cursor, Source::User);
                self.after_input(&f);
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.desk.ime_commit(store, &f, &text, Source::User);
                self.after_input(&f);
            }
            _ => {}
        }
    }
}
