//! What the platform answers that winit has no API for, and RapidQ's
//! window properties in winit's terms:
//!
//! - **The mouse anywhere on the screen** (Screen.MouseX / MouseY,
//!   MOUSEX / MOUSEY): macOS `NSEvent.mouseLocation`, Windows
//!   `GetCursorPos`, X11 `XQueryPointer`. Wayland has no such thing (a
//!   client only sees the pointer over its own surfaces): there, and when
//!   the call fails, the host's last-seen position stands.
//! - **The work area** (the screen less the menu bar / task bar):
//!   macOS `NSScreen.visibleFrame`, Windows `SPI_GETWORKAREA`; elsewhere the
//!   screen (X11's `_NET_WORKAREA` isn't read).
//! - **Cursors**: Screen.Cursor, else the component's Cursor (crDefault:
//!   the component's own — an edit's I-beam, a splitter's or a header
//!   section edge's resize arrows), as winit's `CursorIcon`.
//! - **The frame**: BorderStyle and BorderIcons as decorations, resizing
//!   and the title bar's buttons ([`Frame`]).

use rapidr_ui_kernel::store::{self, Store};
use rapidr_value::input::Cursor;
use winit::window::{CursorIcon, WindowButtons};

use crate::Desktop;

// ------------------------------------------------------------- the frame --

/// A form's window frame as winit can show it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The user may resize it (bsSizeable, bsSizeToolWin).
    pub resizable: bool,
    /// The title bar's buttons.
    pub close: bool,
    pub minimize: bool,
    pub maximize: bool,
}

impl Default for Frame {
    fn default() -> Self {
        Frame { resizable: true, close: true, minimize: true, maximize: true }
    }
}

/// BorderIcons' bits (biSystemMenu 0, biMinimize 1, biMaximize 2, biHelp 3).
pub const BI_DEFAULT: i64 = 0b0111;

/// The frame for BorderStyle `style` (bsNone 0, bsSingle 1, bsSizeable 2,
/// bsDialog 3, bsToolWindow 4, bsSizeToolWin 5) and BorderIcons `icons`
/// (as Windows draws them): without biSystemMenu no button at all; a dialog
/// or tool window has no minimize / maximize; only bsSizeable and
/// bsSizeToolWin resize. biHelp has no counterpart.
pub fn frame_of(style: i64, icons: i64) -> Frame {
    let system = icons & 1 != 0;
    let full = matches!(style, 1 | 2);
    Frame { resizable: matches!(style, 2 | 5), close: system, minimize: system && full && icons & 2 != 0, maximize: system && full && icons & 4 != 0 }
}

impl Frame {
    pub fn buttons(self) -> WindowButtons {
        let mut b = WindowButtons::empty();
        if self.close {
            b |= WindowButtons::CLOSE;
        }
        if self.minimize {
            b |= WindowButtons::MINIMIZE;
        }
        if self.maximize {
            b |= WindowButtons::MAXIMIZE;
        }
        b
    }
}

// --------------------------------------------------------------- cursors --

/// winit's cursor for RapidQ's (`None`: hidden, crNone).
pub fn cursor_icon(c: Cursor) -> Option<CursorIcon> {
    Some(match c {
        Cursor::None => return None,
        Cursor::Default | Cursor::Arrow => CursorIcon::Default,
        Cursor::Cross => CursorIcon::Crosshair,
        Cursor::IBeam => CursorIcon::Text,
        Cursor::Move => CursorIcon::Move,
        Cursor::SizeNESW => CursorIcon::NeswResize,
        Cursor::SizeNS => CursorIcon::NsResize,
        Cursor::SizeNWSE => CursorIcon::NwseResize,
        Cursor::SizeWE => CursorIcon::EwResize,
        Cursor::UpArrow => CursorIcon::NResize,
        Cursor::Wait => CursorIcon::Wait,
        Cursor::NoDrop => CursorIcon::NotAllowed,
        Cursor::Help => CursorIcon::Help,
        Cursor::Hand => CursorIcon::Pointer,
        Cursor::Progress => CursorIcon::Progress,
    })
}

/// The pointer over form `form` at `(x, y)` of its inside: Screen.Cursor
/// (`desk.screen_cursor`), else the Cursor of the component under the
/// mouse (the form's over its client area); crDefault: the component's
/// own (an enabled edit's I-beam, a splitter's resize arrows, a header's
/// or list view header's section edge), else the arrow.
pub fn cursor_at(desk: &Desktop, store: &dyn Store, form: &str, (x, y): (f64, f64)) -> Cursor {
    if desk.screen_cursor != 0 {
        return Cursor::of(desk.screen_cursor);
    }
    let Some(f) = desk.forms.get(form) else { return Cursor::Default };
    let node = f.ui.hover.and_then(|i| f.ui.nodes.get(i));
    // (the input lane's: a status bar's size grip is the window's sizing
    // corner — Windows' HTBOTTOMRIGHT arrow, whatever the bar's Cursor)
    let grip = rapidr_value::layout::STATUS_GRIP;
    if let Some(n) = node.filter(|n| n.type_name == "RSTATUSBAR" && x >= (n.abs.0 + n.abs.2 - grip) as f64 && y >= (n.abs.1 + n.abs.3 - grip) as f64) {
        if rapidr_ui_kernel::components::statusbar::has_grip(store, &n.id) {
            return Cursor::SizeNWSE;
        }
    }
    let id = node.map_or(f.ui.form.as_str(), |n| n.id.as_str());
    let code = store::int(store, id, "cursor", 0);
    if code != 0 {
        return Cursor::of(code);
    }
    let Some(n) = node else { return Cursor::Default };
    let (lx, ly) = ((x as i64) - n.abs.0, (y as i64) - n.abs.1);
    match n.type_name.as_str() {
        "REDIT" | "RMEMO" | "RRICHEDIT" if n.enabled => Cursor::IBeam,
        "RSPLITTER" if rapidr_ui_kernel::components::splitter::vertical(store, &n.id) => Cursor::SizeNS,
        "RSPLITTER" => Cursor::SizeWE,
        "RHEADER" if rapidr_value::objects::with_header(&n.id, |h| h.on_grip(lx)).unwrap_or(false) => Cursor::SizeWE,
        "RLISTVIEW" if rapidr_value::objects::with_listview(&n.id, |l| l.on_grip(lx, ly)).unwrap_or(false) => Cursor::SizeWE,
        _ => Cursor::Default,
    }
}

// ------------------------------------------------------- the screen, mouse --

/// The mouse on the screen (logical pixels, the primary screen's top left
/// the origin), or `None` where the platform can't say (Wayland).
pub fn global_mouse() -> Option<(f64, f64)> {
    imp::global_mouse()
}

/// The primary screen less the menu bar / task bar (logical pixels), or
/// `None` where it isn't known.
pub fn work_area() -> Option<(i64, i64)> {
    imp::work_area()
}

/// (kernel themes) How the system looks — (dark, high contrast) — for
/// `$THEME auto` (`rapidr_value::theme::auto`): macOS' appearance and its
/// Increase Contrast setting; Windows' app mode (AppsUseLightTheme) and
/// its high contrast; elsewhere GTK_THEME's name (…:dark, HighContrast…).
pub fn system_look() -> (bool, bool) {
    imp::system_look()
}

// (the dialogs lane's)
/// A message box's sound, as Windows' MessageBeep plays it for its icon
/// (`None`: the default beep): Windows' system sounds, macOS' alert sound
/// (NSBeep, one for all), X11's bell. Wayland has none.
pub fn beep(icon: Option<rapidr_value::dialogs::MsgIcon>) {
    imp::beep(icon);
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSScreen};

    pub fn beep(_icon: Option<rapidr_value::dialogs::MsgIcon>) {
        objc2_app_kit::NSBeep();
    }

    pub fn system_look() -> (bool, bool) {
        use objc2_foundation::{NSString, NSUserDefaults};
        // (the user's appearance: "Dark" when it's dark, absent when light)
        let style = NSUserDefaults::standardUserDefaults().stringForKey(&NSString::from_str("AppleInterfaceStyle"));
        let dark = style.is_some_and(|s| s.to_string().eq_ignore_ascii_case("dark"));
        let contrast = objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldIncreaseContrast();
        (dark, contrast)
    }

    /// The primary screen (the menu bar's: the origin of screen coordinates).
    fn primary(mtm: MainThreadMarker) -> Option<objc2::rc::Retained<NSScreen>> {
        NSScreen::screens(mtm).firstObject()
    }

    pub fn global_mouse() -> Option<(f64, f64)> {
        let mtm = MainThreadMarker::new()?;
        // (Cocoa's origin is the primary screen's bottom left, y upwards)
        let height = primary(mtm)?.frame().size.height;
        let p = NSEvent::mouseLocation();
        Some((p.x, height - p.y))
    }

    pub fn work_area() -> Option<(i64, i64)> {
        let mtm = MainThreadMarker::new()?;
        let r = primary(mtm)?.visibleFrame();
        Some((r.size.width.round() as i64, r.size.height.round() as i64))
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SystemParametersInfoW, SPI_GETWORKAREA};

    /// The primary monitor's scale (winit makes the process per-monitor DPI
    /// aware: these calls answer in device pixels).
    fn scale() -> f64 {
        crate::platform::primary_scale()
    }

    pub fn beep(icon: Option<rapidr_value::dialogs::MsgIcon>) {
        use rapidr_value::dialogs::MsgIcon;
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONASTERISK, MB_ICONEXCLAMATION, MB_ICONHAND, MB_ICONQUESTION, MB_OK};
        let kind = match icon {
            Some(MsgIcon::Error) => MB_ICONHAND,
            Some(MsgIcon::Question) => MB_ICONQUESTION,
            Some(MsgIcon::Warning) => MB_ICONEXCLAMATION,
            Some(MsgIcon::Information) => MB_ICONASTERISK,
            None => MB_OK,
        };
        // SAFETY: MessageBeep takes a sound type; it only queues a sound.
        unsafe { windows_sys::Win32::System::Diagnostics::Debug::MessageBeep(kind) };
    }

    pub fn system_look() -> (bool, bool) {
        use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
        use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
        use windows_sys::Win32::UI::WindowsAndMessaging::SPI_GETHIGHCONTRAST;
        let mut hc = HIGHCONTRASTW { cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32, dwFlags: 0, lpszDefaultScheme: std::ptr::null_mut() };
        // SAFETY: SPI_GETHIGHCONTRAST fills the HIGHCONTRASTW `hc` points to
        // (its cbSize set).
        let contrast = unsafe { SystemParametersInfoW(SPI_GETHIGHCONTRAST, hc.cbSize, (&mut hc as *mut HIGHCONTRASTW).cast(), 0) } != 0 && hc.dwFlags & HCF_HIGHCONTRASTON != 0;
        let (mut light, mut size) = (1u32, std::mem::size_of::<u32>() as u32);
        // SAFETY: wide, NUL-terminated key and value names; a DWORD and its
        // size for the value.
        let read = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                windows_sys::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                windows_sys::w!("AppsUseLightTheme"),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut light as *mut u32).cast(),
                &mut size,
            )
        };
        (read == 0 && light == 0, contrast)
    }

    pub fn global_mouse() -> Option<(f64, f64)> {
        let mut p = POINT { x: 0, y: 0 };
        // SAFETY: `p` is a valid POINT for the call to fill.
        (unsafe { GetCursorPos(&mut p) } != 0).then(|| (f64::from(p.x) / scale(), f64::from(p.y) / scale()))
    }

    pub fn work_area() -> Option<(i64, i64)> {
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: SPI_GETWORKAREA fills the RECT `r` points to.
        let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut r as *mut RECT).cast(), 0) } != 0;
        let s = scale();
        ok.then(|| ((f64::from(r.right - r.left) / s).round() as i64, (f64::from(r.bottom - r.top) / s).round() as i64))
    }
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
mod imp {
    use std::sync::OnceLock;

    use x11_dl::xlib::{self, Xlib};

    /// libX11 and the display, opened once (`None` on Wayland or without
    /// an X server).
    struct X {
        lib: Xlib,
        display: *mut xlib::Display,
    }

    // SAFETY: only used from the main thread (the event loop's); the
    // pointer is never freed.
    unsafe impl Send for X {}
    unsafe impl Sync for X {}

    fn x() -> Option<&'static X> {
        static X11: OnceLock<Option<X>> = OnceLock::new();
        X11.get_or_init(|| {
            if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("DISPLAY").is_none() {
                return None;
            }
            let lib = Xlib::open().ok()?;
            // SAFETY: XOpenDisplay(NULL) opens $DISPLAY; checked for NULL.
            let display = unsafe { (lib.XOpenDisplay)(std::ptr::null()) };
            (!display.is_null()).then_some(X { lib, display })
        })
        .as_ref()
    }

    pub fn global_mouse() -> Option<(f64, f64)> {
        // (XWayland only knows the pointer over X windows: not asked)
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            return None;
        }
        let x = x()?;
        let (mut root, mut child) = (0, 0);
        let (mut rx, mut ry, mut wx, mut wy, mut mask) = (0, 0, 0, 0, 0);
        // SAFETY: a display opened above; out-parameters are valid locals.
        let ok = unsafe {
            let root_window = (x.lib.XDefaultRootWindow)(x.display);
            (x.lib.XQueryPointer)(x.display, root_window, &mut root, &mut child, &mut rx, &mut ry, &mut wx, &mut wy, &mut mask)
        } != 0;
        let s = crate::platform::primary_scale();
        ok.then(|| (f64::from(rx) / s, f64::from(ry) / s))
    }

    pub fn work_area() -> Option<(i64, i64)> {
        None
    }

    pub fn system_look() -> (bool, bool) {
        super::gtk_look(&std::env::var("GTK_THEME").unwrap_or_default())
    }

    pub fn beep(_icon: Option<rapidr_value::dialogs::MsgIcon>) {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            return;
        }
        if let Some(x) = x() {
            // SAFETY: a display opened above; XBell at the base volume.
            unsafe {
                (x.lib.XBell)(x.display, 0);
                (x.lib.XFlush)(x.display);
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", all(unix, not(target_os = "android")))))]
mod imp {
    pub fn global_mouse() -> Option<(f64, f64)> {
        None
    }
    pub fn work_area() -> Option<(i64, i64)> {
        None
    }
    pub fn beep(_icon: Option<rapidr_value::dialogs::MsgIcon>) {}
    pub fn system_look() -> (bool, bool) {
        (false, false)
    }
}

/// (dark, high contrast) as a GTK theme's name says (`Adwaita:dark`,
/// `Yaru-dark`, `HighContrastInverse` …).
#[allow(dead_code)] // (Linux's answer; the tests read it everywhere)
fn gtk_look(name: &str) -> (bool, bool) {
    let n = name.to_ascii_lowercase();
    let contrast = n.contains("highcontrast") || n.contains("high-contrast");
    (n.ends_with(":dark") || n.contains("-dark") || n.contains("inverse") || n.contains("dark"), contrast)
}

thread_local! {
    static PRIMARY_SCALE: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
}

/// The primary monitor's scale, as the winit host last saw it (device
/// pixels per logical pixel).
#[allow(dead_code)] // (macOS answers in points already)
pub(crate) fn primary_scale() -> f64 {
    PRIMARY_SCALE.with(std::cell::Cell::get)
}

pub(crate) fn set_primary_scale(s: f64) {
    if s > 0.0 {
        PRIMARY_SCALE.with(|p| p.set(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_for_border_style_and_icons() {
        // bsSizeable with every icon: all buttons, resizable
        assert_eq!(frame_of(2, BI_DEFAULT), Frame::default());
        // DelBorderIcons(biMaximize), AddBorderIcons(biHelp): 11
        assert_eq!(frame_of(2, 11), Frame { resizable: true, close: true, minimize: true, maximize: false });
        // bsSingle: fixed size; bsDialog: no minimize / maximize
        assert_eq!(gtk_look("Adwaita"), (false, false));
        assert_eq!(gtk_look("Adwaita:dark"), (true, false));
        assert_eq!(gtk_look("HighContrastInverse"), (true, true));
        assert_eq!(gtk_look("HighContrast"), (false, true));
        assert_eq!(frame_of(1, BI_DEFAULT), Frame { resizable: false, ..Frame::default() });
        assert_eq!(frame_of(3, BI_DEFAULT), Frame { resizable: false, close: true, minimize: false, maximize: false });
        // no biSystemMenu: no buttons at all; bsSizeToolWin resizes
        assert_eq!(frame_of(2, 6).buttons(), WindowButtons::empty());
        assert_eq!(frame_of(5, BI_DEFAULT), Frame { resizable: true, close: true, minimize: false, maximize: false });
    }

    #[test]
    fn cursors() {
        assert_eq!(cursor_icon(Cursor::of(-21)), Some(CursorIcon::Pointer));
        assert_eq!(cursor_icon(Cursor::of(-14)), Some(CursorIcon::EwResize));
        assert_eq!(cursor_icon(Cursor::of(-1)), None);
        assert_eq!(cursor_icon(Cursor::of(0)), Some(CursorIcon::Default));
    }
}
