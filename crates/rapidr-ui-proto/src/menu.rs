//! The native menu bar (muda), where users notice: on macOS the screen's
//! menu bar (with the application menu macOS expects first), on Windows
//! the window's own menu. A QMAINMENU would be built the same way from the
//! shared menu model (`rapidr_value::objects::menu`).

use muda::accelerator::Accelerator;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

pub const OPEN: &str = "file.open";
pub const EXIT: &str = "file.exit";

/// Keeps the menu alive (muda menus are dropped with their owner).
pub struct AppMenu {
    _menu: Menu,
}

/// Builds File > Open… / Exit and hands each click to `on_event` (called on
/// the UI thread on macOS and Windows).
pub fn install(on_event: impl Fn(MenuEvent) + Send + Sync + 'static, _window: &winit::window::Window) -> AppMenu {
    let menu = Menu::new();
    let open = MenuItem::with_id(OPEN, "&Open…", true, "CmdOrCtrl+O".parse::<Accelerator>().ok());
    let exit = MenuItem::with_id(EXIT, "E&xit", true, None);
    let file = Submenu::with_items("&File", true, &[&open, &PredefinedMenuItem::separator(), &exit]).expect("File menu");
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
