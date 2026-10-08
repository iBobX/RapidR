//! The system's menus (muda), where the system has them:
//!
//! - **macOS' menu bar**: the key window's form's QMAINMENU (unless it's
//!   drawn in the window: `RAPIDR_MENU=window`), built from the shared
//!   model (`rapidr_value::objects::menu`) after the application menu macOS
//!   expects first (Hide, Hide Others, Show All, Quit — Quit closes the
//!   key window's form, as its close box would). Built again when the
//!   model changes or another form becomes key. ShortCuts are its key
//!   equivalents (the kernel leaves them alone: `MenuUi::system_bar`).
//! - **Context menus** for `QPOPUPMENU.Popup` on macOS and Windows (a
//!   `HostCmd::Popup`, inside a pump: menu tracking holds the pump; only the
//!   program's timers run inside it, through the menu's tick — tracking.rs).
//!
//! muda calls its handler from inside AppKit's / Win32's menu handling:
//! the pick is queued here (and the pump woken through the event loop
//! proxy); the host turns it into `KernelEvent::MenuPick` for the form
//! after the menu closes, and runtime-core fires OnClick after the pump.
//! Elsewhere (Linux: muda needs GTK) the kernel draws every menu.

use std::sync::Mutex;

use rapidr_ui_kernel::KernelEvent;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::winit_host::UserEvent;
use crate::{Desktop, HostEvent};

/// Picks muda reported: (form, item); item `QUIT` is the application
/// menu's Quit.
static PICKS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

/// The application menu's Quit.
pub const QUIT: &str = "\u{1}quit";

/// A muda id for item `item` of form `form`'s menus.
#[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
fn id_of(form: &str, item: &str) -> String {
    format!("{form}\u{2}{item}")
}

/// Whether this host shows pop-up menus itself.
pub fn native_popups() -> bool {
    cfg!(any(target_os = "macos", target_os = "windows"))
}

/// The picks since the last call, as the program hears them.
pub fn take_picks(desk: &mut Desktop) {
    let picks = std::mem::take(&mut *PICKS.lock().unwrap_or_else(|e| e.into_inner()));
    for (form, item) in picks {
        let ev = if item == QUIT { KernelEvent::Close(form.clone()) } else { KernelEvent::MenuPick(item) };
        desk.events.push(HostEvent::Kernel(form, ev));
    }
}

/// The system's menus this process shows. (`Default`: none yet — what the
/// host's state holds while its menus are lent to a context menu's call.)
#[derive(Default)]
pub struct NativeMenus {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    imp: imp::Menus,
}

impl NativeMenus {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let proxy = Mutex::new(proxy);
            muda::MenuEvent::set_event_handler(Some(move |e: muda::MenuEvent| {
                if let Some((form, item)) = e.id.0.split_once('\u{2}') {
                    PICKS.lock().unwrap_or_else(|p| p.into_inner()).push((form.to_string(), item.to_string()));
                }
                if let Ok(p) = proxy.lock() {
                    p.send_event(UserEvent::Wake).ok();
                }
            }));
            NativeMenus { imp: imp::Menus::default() }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = proxy;
            NativeMenus {}
        }
    }

    /// macOS: the menu bar shows key form `key`'s main menu (built again if
    /// it or the model changed); forms whose main menu it is know it.
    pub fn sync(&mut self, desk: &mut Desktop, store: &dyn rapidr_ui_kernel::Store, key: Option<&str>) {
        #[cfg(target_os = "macos")]
        self.imp.sync(desk, store, key);
        #[cfg(not(target_os = "macos"))]
        let _ = (desk, store, key);
    }

    /// Pop-up menu `menu` at (x, y) of `window`'s inside (logical), for
    /// form `form`: returns when it closes (a pick is queued).
    pub fn popup(&mut self, window: &Window, form: &str, menu: &str, x: i64, y: i64) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        self.imp.popup(window, form, menu, x, y);
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let _ = (window, form, menu, x, y);
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod imp {
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{CheckMenuItem, ContextMenu, IsMenuItem, MenuItem, PredefinedMenuItem, Submenu};
    use rapidr_value::objects::menu::{self, Shortcut};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::Window;

    use super::id_of;
    #[cfg(target_os = "macos")]
    use super::QUIT;
    #[cfg(target_os = "macos")]
    use crate::Desktop;

    #[derive(Default)]
    pub struct Menus {
        /// The menu bar (kept alive while it shows).
        #[cfg(target_os = "macos")]
        bar: Option<muda::Menu>,
        /// What the bar was built for: the key form, its main menu, the
        /// model's revision.
        #[cfg(target_os = "macos")]
        built: Option<(String, Option<String>, u64)>,
    }

    /// A ShortCut as muda's accelerator.
    fn accelerator(sc: Option<Shortcut>) -> Option<Accelerator> {
        let sc = sc?;
        let code = match sc.vk {
            8 => Code::Backspace,
            9 => Code::Tab,
            13 => Code::Enter,
            27 => Code::Escape,
            32 => Code::Space,
            33 => Code::PageUp,
            34 => Code::PageDown,
            35 => Code::End,
            36 => Code::Home,
            37 => Code::ArrowLeft,
            38 => Code::ArrowUp,
            39 => Code::ArrowRight,
            40 => Code::ArrowDown,
            45 => Code::Insert,
            46 => Code::Delete,
            48 => Code::Digit0,
            49 => Code::Digit1,
            50 => Code::Digit2,
            51 => Code::Digit3,
            52 => Code::Digit4,
            53 => Code::Digit5,
            54 => Code::Digit6,
            55 => Code::Digit7,
            56 => Code::Digit8,
            57 => Code::Digit9,
            65..=90 => {
                const LETTERS: [Code; 26] = [
                    Code::KeyA, Code::KeyB, Code::KeyC, Code::KeyD, Code::KeyE, Code::KeyF, Code::KeyG, Code::KeyH, Code::KeyI, Code::KeyJ, Code::KeyK, Code::KeyL, Code::KeyM,
                    Code::KeyN, Code::KeyO, Code::KeyP, Code::KeyQ, Code::KeyR, Code::KeyS, Code::KeyT, Code::KeyU, Code::KeyV, Code::KeyW, Code::KeyX, Code::KeyY, Code::KeyZ,
                ];
                LETTERS[(sc.vk - 65) as usize]
            }
            112..=123 => {
                const F: [Code; 12] = [Code::F1, Code::F2, Code::F3, Code::F4, Code::F5, Code::F6, Code::F7, Code::F8, Code::F9, Code::F10, Code::F11, Code::F12];
                F[(sc.vk - 112) as usize]
            }
            _ => return None,
        };
        let mut mods = Modifiers::empty();
        // ("Ctrl+" is the command key on a Mac, as its menus' keys are)
        if sc.ctrl {
            mods |= if cfg!(target_os = "macos") { Modifiers::META } else { Modifiers::CONTROL };
        }
        if sc.shift {
            mods |= Modifiers::SHIFT;
        }
        if sc.alt {
            mods |= Modifiers::ALT;
        }
        Some(Accelerator::new(mods, code))
    }

    /// The items under `parent` (a menu or an item), as muda's, into
    /// `out` (separators folded in).
    fn fill(form: &str, parent: &str, out: &dyn Fn(&dyn IsMenuItem)) {
        for id in menu::children(parent) {
            let Some(n) = menu::with(&id, |n| n.clone()) else { continue };
            if n.caption == "-" {
                out(&PredefinedMenuItem::separator());
                continue;
            }
            let mid = id_of(form, &id);
            if !menu::children(&id).is_empty() {
                let sub = Submenu::with_id(mid, menu::split_caption(&n.caption).0, n.enabled);
                fill(form, &id, &|item| {
                    sub.append(item).ok();
                });
                out(&sub);
            } else if n.checked {
                out(&CheckMenuItem::with_id(mid, menu::split_caption(&n.caption).0, n.enabled, true, accelerator(menu::parse_shortcut(&n.shortcut))));
            } else {
                out(&MenuItem::with_id(mid, menu::split_caption(&n.caption).0, n.enabled, accelerator(menu::parse_shortcut(&n.shortcut))));
            }
        }
    }

    impl Menus {
        #[cfg(target_os = "macos")]
        pub fn sync(&mut self, desk: &mut Desktop, store: &dyn rapidr_ui_kernel::Store, key: Option<&str>) {
            // (forms with a main menu not drawn in their window: the bar's)
            for f in desk.forms.values_mut() {
                let system = f.ui.menu_offset == 0 && f.ui.main_menu(store).is_some();
                f.ui.menus.system_bar = system;
            }
            let Some(key) = key else { return };
            let Some(form) = desk.forms.get(key) else { return };
            let main = form.ui.main_menu(store).filter(|_| form.ui.menus.system_bar);
            let want = (key.to_string(), main.clone(), menu::revision());
            if self.built.as_ref() == Some(&want) {
                return;
            }
            let bar = muda::Menu::new();
            let app = Submenu::new("RapidR", true);
            let quit = MenuItem::with_id(id_of(key, QUIT), "Quit", true, Some(Accelerator::new(Modifiers::META, Code::KeyQ)));
            app.append_items(&[&PredefinedMenuItem::hide(None), &PredefinedMenuItem::hide_others(None), &PredefinedMenuItem::show_all(None), &PredefinedMenuItem::separator(), &quit]).ok();
            bar.append(&app).ok();
            if let Some(main) = &main {
                fill(key, main, &|item| {
                    bar.append(item).ok();
                });
            }
            bar.init_for_nsapp();
            self.bar = Some(bar);
            self.built = Some(want);
        }

        pub fn popup(&mut self, window: &Window, form: &str, menu_id: &str, x: i64, y: i64) {
            let m = Submenu::new("", true);
            fill(form, menu_id, &|item| {
                m.append(item).ok();
            });
            let at = Some(muda::dpi::Position::Logical(muda::dpi::LogicalPosition::new(x as f64, y as f64)));
            let Ok(handle) = window.window_handle() else { return };
            match handle.as_raw() {
                #[cfg(target_os = "macos")]
                // SAFETY: the view is the live window's, on the main thread.
                RawWindowHandle::AppKit(h) => unsafe {
                    // (its NSMenu while it's tracked: a tick may close it,
                    // tracking.rs)
                    crate::tracking::set_popup_menu(muda::ContextMenu::ns_menu(&m));
                    m.show_context_menu_for_nsview(h.ns_view.as_ptr(), at);
                    crate::tracking::set_popup_menu(std::ptr::null_mut());
                },
                #[cfg(target_os = "windows")]
                // SAFETY: the handle is the live window's, on its thread.
                RawWindowHandle::Win32(h) => unsafe {
                    m.show_context_menu_for_hwnd(h.hwnd.get(), at);
                },
                _ => {}
            }
            // (a macOS bar built again later: a context menu leaves it be)
            #[cfg(target_os = "macos")]
            let _ = &self.bar;
        }
    }
}
