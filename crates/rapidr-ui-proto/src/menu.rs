//! The native menu bar (muda), where users notice: on macOS the screen's
//! menu bar (with the application menu macOS expects first), on Windows
//! the window's own menu. A QMAINMENU would be built the same way from the
//! shared menu model (`rapidr_value::objects::menu`).
//!
//! muda calls `on_event` on the UI thread from inside AppKit's menu
//! handling (inside a pump); the host forwards it through the event loop
//! proxy, so it reaches the program as a `HostEvent::Menu` after the pump.

use muda::accelerator::Accelerator;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

/// Async file dialog (a sheet, created inside a pump).
pub const OPEN: &str = "file.open";
/// Spike: rfd's async dialog created between pumps (falls back to blocking).
pub const OPEN_BETWEEN: &str = "file.open_between";
/// Spike: a blocking rfd dialog between pumps.
pub const BLOCKING: &str = "file.blocking";
/// Spike: a blocking rfd dialog inside a winit callback.
pub const BLOCKING_CB: &str = "file.blocking_cb";
/// Does nothing but reach the program (Cmd+P).
pub const PING: &str = "file.ping";
pub const EXIT: &str = "file.exit";

/// Keeps the menu alive (muda menus are dropped with their owner).
pub struct AppMenu {
    _menu: Menu,
}

/// Builds the File menu and hands each click to `on_event` (called on
/// the UI thread on macOS and Windows).
pub fn install(on_event: impl Fn(MenuEvent) + Send + Sync + 'static, _window: &winit::window::Window) -> AppMenu {
    let menu = Menu::new();
    let accel = |s: &str| s.parse::<Accelerator>().ok();
    let open = MenuItem::with_id(OPEN, "&Open… (async)", true, accel("CmdOrCtrl+O"));
    let between = MenuItem::with_id(OPEN_BETWEEN, "Open… (async, created between pumps)", true, None);
    let blocking = MenuItem::with_id(BLOCKING, "Open… (blocking, between pumps)", true, None);
    let blocking_cb = MenuItem::with_id(BLOCKING_CB, "Open… (blocking, inside a callback)", true, None);
    let ping = MenuItem::with_id(PING, "Ping", true, accel("CmdOrCtrl+P"));
    let exit = MenuItem::with_id(EXIT, "E&xit", true, None);
    let file = Submenu::with_items("&File", true, &[&open, &between, &blocking, &blocking_cb, &ping, &PredefinedMenuItem::separator(), &exit]).expect("File menu");
    #[cfg(target_os = "macos")]
    {
        // (macOS: the first menu is the application's, named after it)
        let app = Submenu::with_items("RapidR", true, &[&PredefinedMenuItem::about(None, None), &PredefinedMenuItem::separator(), &PredefinedMenuItem::quit(None)]).expect("app menu");
        menu.append(&app).expect("app menu");
        menu.append(&file).expect("File menu");
        menu.init_for_nsapp();
    }
    #[cfg(target_os = "windows")]
    {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        menu.append(&file).expect("File menu");
        if let Ok(RawWindowHandle::Win32(h)) = _window.window_handle().map(|h| h.as_raw()) {
            // SAFETY: the handle is the live window's, on its own thread.
            unsafe { menu.init_for_hwnd(h.hwnd.get()).ok() };
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // (Linux: muda needs a GTK window; the kernel would draw the bar itself)
        menu.append(&file).expect("File menu");
    }
    MenuEvent::set_event_handler(Some(on_event));
    AppMenu { _menu: menu }
}
