//! RapidR's desktop host for the UI kernel (`docs/desktop-host-plan.md`
//! §1.5): the program's forms as windows the kernel draws.
//!
//! - [`WinitHost`](winit_host::WinitHost): winit driven by
//!   `pump_app_events` from an ordinary loop — never `run_app`, never
//!   `exit()`; one `EventLoop` per process, made on the main thread when the
//!   first window is needed. A window per shown form, drawn by vello on the
//!   GPU or by vello_cpu through softbuffer (`RAPIDR_RENDERER=cpu|gpu`; the
//!   CPU when wgpu finds no GPU, or only a software one), an AccessKit
//!   adapter each.
//! - [`HeadlessHost`](headless::HeadlessHost): no OS windows, no event
//!   loop, no NSApplication; `pump(t)` sleeps until `t`; the screen is a
//!   fixed 1920 × 1080. GUI tests run on it (`RAPIDR_CAPTURE` without
//!   `RAPIDR_CAPTURE_WINDOWS`).
//!
//! The host never calls program code — it can't: this crate doesn't depend
//! on runtime-core. Its callbacks route input into the kernel
//! ([`Desktop`]), render, and run the program's [`HostCmd`]s; what the
//! program must hear about is queued as [`HostEvent`]s, which runtime-core
//! dispatches after the pump returns. Captures always use the CPU renderer
//! ([`capture`]). The drawing itself — display lists to vello scenes and
//! vello_cpu pixmaps — is `rapidr-ui-render`'s, which the web host uses
//! too.

pub mod a11y;
pub mod dialogs;
pub mod headless;
pub mod menu;
pub mod platform;
// (ShapeForm: a form's outline on its window)
pub mod shape;
pub mod tracking;
// (the system tray: rapidr_value::tray)
pub mod tray;
pub mod winit_host;
// (C-SYS: Screen.Cursors' Windows cursors)
#[cfg(target_os = "windows")]
mod wincursor;

use std::task::Waker;
use std::time::Duration;

// (Stage W3: `Desktop` and its queues are the hosts' shared ones, in
// rapidr-ui-app; re-exported under their old names.)
pub use rapidr_ui_app::desktop::{Desktop, Form, HostCmd, HostEvent, Icon, Source, WindowSpec};
pub use dialogs::FileRequest;
pub use platform::Frame;
use rapidr_ui_kernel::Store;
use rapidr_value::objects::codec::Pixels;

/// The headless screen (logical pixels).
pub const HEADLESS_SCREEN: (i64, i64) = (1920, 1080);

/// A desktop host: the window system, pumped by runtime-core's `step`.
pub trait Host {
    /// Runs the window system for up to `timeout` (`None`: until something
    /// happens). Its callbacks run `desk`'s commands, route input into it
    /// (reading the program's components through `store`) and render; what
    /// the program must hear about is left in `desk.events`.
    fn pump(&mut self, timeout: Option<Duration>, desk: &mut Desktop, store: &dyn Store);
    /// The screen's size (logical pixels).
    fn screen(&self) -> (i64, i64);
    /// The screen less the task bar / menu bar.
    fn work_area(&self) -> (i64, i64) {
        self.screen()
    }
    fn monitors(&self) -> i64 {
        1
    }
    /// The mouse on the screen (logical pixels), as last seen.
    fn mouse(&self) -> (i64, i64) {
        (0, 0)
    }
    /// The scale a new window would show at (device pixels per logical
    /// pixel).
    fn default_scale(&self) -> f64;
    /// Wakes the pump (for futures: async dialogs).
    fn waker(&self) -> Waker;
    fn headless(&self) -> bool;
    /// (the DirectX lane's) The program is the active application: one of
    /// its windows has the keyboard (QDXTIMER's ActiveOnly). A host without
    /// a system (headless) always is.
    fn active(&self) -> bool {
        true
    }
    fn name(&self) -> &'static str;
    /// Shows pop-up menus itself ([`HostCmd::Popup`]: macOS' and Windows'
    /// context menus); else the kernel draws them.
    fn native_menus(&self) -> bool {
        false
    }
    /// Open / Save dialog `id`'s answer ([`HostCmd::FileDialog`]) once it's
    /// closed: the paths picked (none: cancelled); `None` while it's open.
    /// A host without the system's dialogs cancels them.
    fn file_dialog(&mut self, _id: u64) -> Option<Vec<String>> {
        Some(Vec::new())
    }
    /// (timers during native menu tracking) What runs while the system
    /// holds a pump (a native menu tracked, Windows' size / move loop):
    /// runtime-core's due timers, the program's changes into the kernel
    /// ([`tracking`]). A host whose pumps are never held ignores it.
    fn set_tracking_hook(&mut self, _hook: tracking::Hook) {}
    /// (a GUI test's `__hold_ms`) The next pump held for `hold`, as a native
    /// menu the user keeps open would hold it: the headless host pretends
    /// ([`tracking::simulate_hold`]); a real one has its user.
    fn hold(&mut self, _hold: Duration) {}
    /// (the system tray) Shows the program's tray icons
    /// (`rapidr_value::tray::shown`) — called when they changed. A host
    /// without a system (headless) keeps none.
    fn tray_sync(&mut self, _icons: &[tray::Shown]) {}
    /// The tray icons' clicks since the last turn: (icon, Windows' mouse
    /// messages).
    fn tray_clicks(&mut self) -> Vec<((i64, i64), Vec<i64>)> {
        Vec::new()
    }
}

/// Which renderer windows use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererKind {
    Gpu,
    Cpu,
}

impl RendererKind {
    /// `RAPIDR_RENDERER=cpu|gpu` (default: the GPU).
    pub fn from_env() -> RendererKind {
        match std::env::var("RAPIDR_RENDERER") {
            Ok(v) if v.trim().eq_ignore_ascii_case("cpu") => RendererKind::Cpu,
            _ => RendererKind::Gpu,
        }
    }

    /// `RAPIDR_RENDERER=gpu`: the GPU even when it's a software one.
    pub fn gpu_asked() -> bool {
        matches!(std::env::var("RAPIDR_RENDERER"), Ok(v) if v.trim().eq_ignore_ascii_case("gpu"))
    }
}

/// The host for this process: headless, or winit windows (`scale`: a
/// forced scale, `RAPIDR_SCALE`).
pub fn new_host(headless: bool, scale: Option<f64>) -> Box<dyn Host> {
    if !headless {
        match winit_host::WinitHost::new(RendererKind::from_env(), scale) {
            Ok(host) => return Box::new(host),
            // (no display: a Linux server, SSH, a CI machine — the program
            // runs on, its windows unseen, as the headless host keeps them)
            Err(e) => eprintln!("[rapidr] no windows can be shown ({e}); the program runs without them"),
        }
    }
    Box::new(headless::HeadlessHost::new(scale.unwrap_or(1.0)))
}

/// Form `id` drawn by the CPU renderer at its scale (what `RAPIDR_CAPTURE`
/// saves); `None` if there's no such form.
pub fn capture(desk: &mut Desktop, store: &dyn Store, id: &str) -> Option<Pixels> {
    let Desktop { forms, text, .. } = desk;
    let f = forms.get_mut(&id.to_lowercase())?;
    let list = f.ui.paint(store, text, f.scale);
    Some(rapidr_ui_render::cpu::capture(&list, text, &f.ui))
}

#[cfg(test)]
mod tests;
