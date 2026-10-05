//! The system tray (Windows' notification area): RapidQ programs put an
//! icon there with QNOTIFYICONDATA and Windows' `Shell_NotifyIcon` (the
//! manual: "its use is primarily restricted to the Shell_NotifyIcon API
//! call") and hear its clicks in their form's WndProc as the message they
//! chose (`uCallbackMessage`, wParam the icon's `uID`, lParam the mouse
//! message). RapidR keeps that one call (rapidr_ast turns a `DECLARE …
//! LIB "shell32" ALIAS "Shell_NotifyIconA"` into `__shell_notifyicon`)
//! and shows the icons on every platform: the notification area on
//! Windows, the menu bar's status items on macOS, StatusNotifierItem on
//! Linux, a small strip in the page on the web — the hosts read
//! [`icons`] and report clicks with [`clicked`].
//!
//! Like Windows: an icon is (hWnd, uID); NIM_ADD fails for one already
//! there, NIM_MODIFY and NIM_DELETE for one that isn't; the flags say
//! which of uCallbackMessage, hIcon and szTip count.

use std::cell::RefCell;

use crate::objects::record::Record;
use crate::Value;

pub const NIM_ADD: i64 = 0;
pub const NIM_MODIFY: i64 = 1;
pub const NIM_DELETE: i64 = 2;
pub const NIF_MESSAGE: i64 = 1;
pub const NIF_ICON: i64 = 2;
pub const NIF_TIP: i64 = 4;

/// The mouse messages a tray icon's clicks become (lParam).
pub const WM_MOUSEMOVE: i64 = 0x200;
pub const WM_LBUTTONDOWN: i64 = 0x201;
pub const WM_LBUTTONUP: i64 = 0x202;
pub const WM_LBUTTONDBLCLK: i64 = 0x203;
pub const WM_RBUTTONDOWN: i64 = 0x204;
pub const WM_RBUTTONUP: i64 = 0x205;
pub const WM_RBUTTONDBLCLK: i64 = 0x206;
pub const WM_MBUTTONDOWN: i64 = 0x207;
pub const WM_MBUTTONUP: i64 = 0x208;

/// One icon in the tray.
#[derive(Clone, Debug, PartialEq)]
pub struct TrayIcon {
    pub hwnd: i64,
    pub uid: i64,
    /// The message clicks become (NIF_MESSAGE).
    pub message: Option<i64>,
    /// The picture (NIF_ICON): `hIcon` as the program gave it.
    pub icon: Option<i64>,
    /// The tooltip (NIF_TIP).
    pub tip: Option<String>,
}

impl TrayIcon {
    /// Its key for the hosts (stable while it's there).
    pub fn key(&self) -> (i64, i64) {
        (self.hwnd, self.uid)
    }
}

#[derive(Default)]
struct Tray {
    icons: Vec<TrayIcon>,
    revision: u64,
}

thread_local! {
    static TRAY: RefCell<Tray> = RefCell::new(Tray::default());
    /// What a runtime that draws the tray itself runs when it changes (the
    /// web's strip; the desktop's hosts compare [`revision`] instead).
    static ON_CHANGE: std::cell::Cell<Option<fn()>> = const { std::cell::Cell::new(None) };
}

/// Runs `hook` after every change of the icons.
pub fn set_on_change(hook: fn()) {
    ON_CHANGE.with(|c| c.set(Some(hook)));
}

fn changed() {
    if let Some(hook) = ON_CHANGE.with(std::cell::Cell::get) {
        hook();
    }
}

/// `Shell_NotifyIcon(message, data)`: 1 when done, 0 when Windows would
/// have refused it.
pub fn shell_notify_icon(message: i64, data: &Record) -> i64 {
    let (hwnd, uid, flags) = (data.number("hwnd"), data.number("uid"), data.number("uflags"));
    let done = TRAY.with(|t| {
        let mut t = t.borrow_mut();
        let at = t.icons.iter().position(|i| i.hwnd == hwnd && i.uid == uid);
        let apply = |icon: &mut TrayIcon| {
            if flags & NIF_MESSAGE != 0 {
                icon.message = Some(data.number("ucallbackmessage"));
            }
            if flags & NIF_ICON != 0 {
                icon.icon = Some(data.number("hicon"));
            }
            if flags & NIF_TIP != 0 {
                icon.tip = Some(data.tip.clone());
            }
        };
        let done = match (message, at) {
            (NIM_ADD, None) => {
                let mut icon = TrayIcon { hwnd, uid, message: None, icon: None, tip: None };
                apply(&mut icon);
                t.icons.push(icon);
                true
            }
            (NIM_MODIFY, Some(i)) => {
                apply(&mut t.icons[i]);
                true
            }
            (NIM_DELETE, Some(i)) => {
                t.icons.remove(i);
                true
            }
            _ => false,
        };
        if done {
            t.revision += 1;
        }
        i64::from(done)
    });
    if done == 1 {
        changed();
    }
    done
}

/// The icons there now, in the order they were added.
pub fn icons() -> Vec<TrayIcon> {
    TRAY.with(|t| t.borrow().icons.clone())
}

/// Changes whenever the icons do (hosts compare it before syncing).
pub fn revision() -> u64 {
    TRAY.with(|t| t.borrow().revision)
}

/// Every icon gone (the program ends: Windows takes them away when the
/// window that owns them goes).
pub fn clear() {
    TRAY.with(|t| {
        let mut t = t.borrow_mut();
        if !t.icons.is_empty() {
            t.icons.clear();
            t.revision += 1;
        }
    });
}

/// A click on icon `key` as the mouse messages Windows sends for it, in
/// order: what the window's WndProc gets — (hWnd, uMsg, wParam, lParam)
/// for each; nothing when the icon has no NIF_MESSAGE (or is gone).
pub fn clicked(key: (i64, i64), mouse: &[i64]) -> Vec<[i64; 4]> {
    TRAY.with(|t| {
        let t = t.borrow();
        let Some(icon) = t.icons.iter().find(|i| i.key() == key) else { return Vec::new() };
        let Some(msg) = icon.message else { return Vec::new() };
        mouse.iter().map(|&m| [icon.hwnd, msg, icon.uid, m]).collect()
    })
}

/// The messages of a left click, a double click, a right click, a middle
/// click (what a host whose system reports only clicks sends).
pub const LEFT_CLICK: &[i64] = &[WM_LBUTTONDOWN, WM_LBUTTONUP];
pub const DOUBLE_CLICK: &[i64] = &[WM_LBUTTONDOWN, WM_LBUTTONUP, WM_LBUTTONDBLCLK, WM_LBUTTONUP];
pub const RIGHT_CLICK: &[i64] = &[WM_RBUTTONDOWN, WM_RBUTTONUP];
pub const MIDDLE_CLICK: &[i64] = &[WM_MBUTTONDOWN, WM_MBUTTONUP];

/// RapidR's own tray picture (a program whose icon is none RapidR can
/// show): a blue rounded square with a white R.
const DEFAULT_ICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect x="1" y="1" width="30" height="30" rx="6" fill="#1f6fd1"/><path d="M10 7h8.5a6 6 0 0 1 1.6 11.8L25 25h-4.6l-4.3-5.8H14V25h-4zM14 10.6v5.2h4.3a2.6 2.6 0 0 0 0-5.2z" fill="#fff"/></svg>"##;

/// The picture to show for `hIcon` — (width, height, RGBA): the icon a
/// handle stands for (an `Application.Icon` read, `crate::handles::
/// icon_handle`), else the application's icon (`Application.Icon` /
/// `IcoHandle`), else RapidR's own. (Windows shows an empty slot for an
/// icon it can't load; an empty status item can't even be clicked.)
pub fn picture(hicon: Option<i64>) -> (usize, usize, Vec<u8>) {
    let own = hicon.and_then(crate::handles::icon_source).filter(|s| !s.is_empty()).map(Value::String);
    let pixels = |v: Value| crate::objects::icon_pixels(&v).map(|(w, h, rgba, _)| (w, h, rgba));
    own.and_then(pixels)
        .or_else(|| crate::globals::application_icon().and_then(pixels))
        .or_else(|| pixels(Value::String(format!("{}{}", crate::objects::codec::SVG_DATA_URL, crate::objects::codec::base64_encode(DEFAULT_ICON_SVG.as_bytes())))))
        .unwrap_or((1, 1, vec![0, 0, 0, 0]))
}

/// What a host shows for an icon: (key, tooltip, picture (width, height,
/// RGBA)).
pub type Shown = ((i64, i64), String, (usize, usize, Vec<u8>));

/// What a host shows for each icon.
pub fn shown() -> Vec<Shown> {
    icons().into_iter().map(|i| (i.key(), i.tip.clone().unwrap_or_default(), picture(i.icon))).collect()
}

/// `__shell_notifyicon(message, data)`: the QNOTIFYICONDATA named by
/// `data` (its id).
pub fn shell_notify_icon_builtin(message: &Value, data: &Value) -> Value {
    let id = data.to_string_val();
    let done = crate::objects::with_record(&id, |r| shell_notify_icon(message.to_i64(), r)).unwrap_or(0);
    crate::v_int(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::record::{Kind, Record};
    use crate::{v_int, v_str};

    fn data(hwnd: i64, uid: i64, flags: i64, msg: i64, tip: &str) -> Record {
        let mut r = Record::new(Kind::NotifyIconData);
        for (p, v) in [("hwnd", hwnd), ("uid", uid), ("uflags", flags), ("ucallbackmessage", msg)] {
            r.set(p, &v_int(v));
        }
        r.set("sztip", &v_str(tip));
        r
    }

    #[test]
    fn add_modify_delete_as_windows() {
        clear();
        let a = data(10, 1, NIF_MESSAGE | NIF_TIP, 0x600, "Hello");
        assert_eq!(shell_notify_icon(NIM_ADD, &a), 1);
        assert_eq!(shell_notify_icon(NIM_ADD, &a), 0, "already there");
        assert_eq!(shell_notify_icon(NIM_MODIFY, &data(10, 2, NIF_TIP, 0, "x")), 0, "not there");
        assert_eq!(shell_notify_icon(NIM_MODIFY, &data(10, 1, NIF_TIP, 0, "Bye")), 1);
        assert_eq!(icons()[0].tip.as_deref(), Some("Bye"));
        assert_eq!(icons()[0].message, Some(0x600));
        assert_eq!(clicked((10, 1), LEFT_CLICK), vec![[10, 0x600, 1, WM_LBUTTONDOWN], [10, 0x600, 1, WM_LBUTTONUP]]);
        let r = revision();
        assert_eq!(shell_notify_icon(NIM_DELETE, &a), 1);
        assert!(revision() > r);
        assert!(icons().is_empty());
        assert!(clicked((10, 1), LEFT_CLICK).is_empty());
        // (no NIF_MESSAGE: clicks say nothing)
        assert_eq!(shell_notify_icon(NIM_ADD, &data(10, 3, NIF_TIP, 0x600, "")), 1);
        assert!(clicked((10, 3), RIGHT_CLICK).is_empty());
        clear();
    }

    #[test]
    fn every_icon_has_a_picture() {
        let (w, h, rgba) = picture(None);
        assert!(w >= 16 && h >= 16 && rgba.len() == w * h * 4);
    }
}
