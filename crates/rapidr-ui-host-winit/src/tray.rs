//! The system tray on the desktop (`rapidr_value::tray`: the icons a
//! program put there with QNOTIFYICONDATA and Shell_NotifyIcon): the
//! menu bar's status items on macOS, the notification area on Windows,
//! StatusNotifierItem (D-Bus, through zbus: no system package) on Linux and
//! the BSDs. The host syncs them from [`Tray::sync`] when they change and
//! hands the clicks back as Windows' mouse messages ([`Tray::clicks`]):
//! Windows reports them as they are; macOS a click's release (left, right,
//! the double click's second), StatusNotifierItem Activate (left),
//! SecondaryActivate (middle) and ContextMenu (right) — each the messages a
//! Windows click sends.

use std::sync::{Arc, Mutex};
use std::task::Waker;

use rapidr_value::tray;

/// An icon to show: (its key, tooltip, picture (width, height, RGBA)).
pub type Shown = tray::Shown;

/// A click: (the icon's key, Windows' mouse messages).
pub type Click = ((i64, i64), Vec<i64>);

/// Clicks waiting for the program: (icon, Windows' mouse messages); a
/// click wakes the pump (it may come from inside the system's loop, or
/// another thread: D-Bus).
pub struct Pending {
    list: Mutex<Vec<Click>>,
    waker: Waker,
}

impl Pending {
    pub fn push(&self, key: (i64, i64), mouse: Vec<i64>) {
        self.list.lock().unwrap_or_else(|e| e.into_inner()).push((key, mouse));
        self.waker.wake_by_ref();
    }
}

pub type Clicks = Arc<Pending>;

/// The platform's tray, as the host keeps it.
pub struct Tray {
    clicks: Clicks,
    #[allow(dead_code)]
    waker: Waker,
    shown: Vec<Shown>,
    #[cfg(target_os = "macos")]
    mac: mac::Items,
    #[cfg(target_os = "windows")]
    win: win::Items,
    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
    sni: sni::Items,
}

impl Tray {
    pub fn new(waker: Waker) -> Tray {
        Tray {
            clicks: Arc::new(Pending { list: Mutex::new(Vec::new()), waker: waker.clone() }),
            waker,
            shown: Vec::new(),
            #[cfg(target_os = "macos")]
            mac: mac::Items::default(),
            #[cfg(target_os = "windows")]
            win: win::Items::default(),
            #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
            sni: sni::Items::default(),
        }
    }

    /// Shows `icons` (the program's, in order); icons gone are taken away.
    pub fn sync(&mut self, icons: &[Shown]) {
        if icons == self.shown.as_slice() {
            return;
        }
        #[cfg(target_os = "macos")]
        self.mac.sync(icons, &self.clicks);
        #[cfg(target_os = "windows")]
        self.win.sync(icons, &self.clicks);
        #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
        self.sni.sync(icons, &self.clicks, &self.waker);
        self.shown = icons.to_vec();
    }

    /// The clicks since the last turn.
    pub fn clicks(&mut self) -> Vec<Click> {
        std::mem::take(&mut *self.clicks.list.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

/// A click the platform reports as a whole: Windows' messages for it.
#[allow(dead_code)]
fn messages(left: bool, right: bool, double: bool) -> Vec<i64> {
    if right {
        tray::RIGHT_CLICK.to_vec()
    } else if !left {
        tray::MIDDLE_CLICK.to_vec()
    } else if double {
        // (the first click of the pair was reported already)
        vec![tray::WM_LBUTTONDBLCLK, tray::WM_LBUTTONUP]
    } else {
        tray::LEFT_CLICK.to_vec()
    }
}

// ----------------------------------------------------------------- macOS --

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::Cell;

    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject};
    use objc2::{define_class, msg_send, sel, AllocAnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSApplication, NSEventMask, NSEventType, NSImage, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
    use objc2_foundation::{NSData, NSSize, NSString};

    use super::{messages, Clicks, Shown};

    pub struct TargetIvars {
        key: Cell<(i64, i64)>,
        clicks: Clicks,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements; Target has no Drop.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "RapidRTrayTarget"]
        #[ivars = TargetIvars]
        struct Target;

        impl Target {
            #[unsafe(method(clicked:))]
            fn clicked(&self, _sender: Option<&AnyObject>) {
                let app = NSApplication::sharedApplication(self.mtm());
                let (left, right, double) = match app.currentEvent() {
                    Some(e) => {
                        let t = e.r#type();
                        let right = t == NSEventType::RightMouseUp || t == NSEventType::RightMouseDown;
                        let left = t == NSEventType::LeftMouseUp || t == NSEventType::LeftMouseDown;
                        let n = e.clickCount();
                        (left, right, left && n >= 2 && n % 2 == 0)
                    }
                    None => (true, false, false),
                };
                let ivars = self.ivars();
                ivars.clicks.push(ivars.key.get(), messages(left, right, double));
            }
        }
    );

    impl Target {
        fn new(mtm: MainThreadMarker, key: (i64, i64), clicks: Clicks) -> Retained<Target> {
            let this = Target::alloc(mtm).set_ivars(TargetIvars { key: Cell::new(key), clicks });
            unsafe { msg_send![super(this), init] }
        }
    }

    /// An icon's status item and the object its button calls.
    struct Item {
        key: (i64, i64),
        item: Retained<NSStatusItem>,
        _target: Retained<Target>,
        shown: (String, (usize, usize, Vec<u8>)),
    }

    #[derive(Default)]
    pub struct Items {
        items: Vec<Item>,
    }

    /// A picture as a PNG for NSImage (RGBA, straight alpha).
    fn png(w: usize, h: usize, rgba: &[u8]) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header().ok()?;
            writer.write_image_data(rgba).ok()?;
        }
        Some(out)
    }

    fn show(mtm: MainThreadMarker, item: &NSStatusItem, tip: &str, picture: &(usize, usize, Vec<u8>)) {
        let Some(button) = item.button(mtm) else { return };
        let (w, h, rgba) = picture;
        if let Some(bytes) = png(*w, *h, rgba) {
            let data = NSData::with_bytes(&bytes);
            if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
                // (the menu bar's icons are 18 points)
                image.setSize(NSSize::new(18.0, 18.0));
                button.setImage(Some(&image));
            }
        }
        let tip = NSString::from_str(tip);
        button.setToolTip(if tip.length() == 0 { None } else { Some(&tip) });
    }

    impl Items {
        pub fn sync(&mut self, icons: &[Shown], clicks: &Clicks) {
            let Some(mtm) = MainThreadMarker::new() else { return };
            let bar = NSStatusBar::systemStatusBar();
            // (gone)
            self.items.retain(|i| {
                let keep = icons.iter().any(|(k, _, _)| *k == i.key);
                if !keep {
                    bar.removeStatusItem(&i.item);
                }
                keep
            });
            for (key, tip, picture) in icons {
                match self.items.iter_mut().find(|i| i.key == *key) {
                    Some(i) => {
                        if i.shown != (tip.clone(), picture.clone()) {
                            show(mtm, &i.item, tip, picture);
                            i.shown = (tip.clone(), picture.clone());
                        }
                    }
                    None => {
                        let item = bar.statusItemWithLength(NSVariableStatusItemLength);
                        let target = Target::new(mtm, *key, clicks.clone());
                        if let Some(button) = item.button(mtm) {
                            unsafe {
                                button.setTarget(Some(&target));
                                button.setAction(Some(sel!(clicked:)));
                            }
                            button.sendActionOn(NSEventMask::LeftMouseUp | NSEventMask::RightMouseUp | NSEventMask::OtherMouseUp);
                        }
                        show(mtm, &item, tip, picture);
                        self.items.push(Item { key: *key, item, _target: target, shown: (tip.clone(), picture.clone()) });
                    }
                }
            }
        }
    }

    impl Drop for Items {
        fn drop(&mut self) {
            let bar = NSStatusBar::systemStatusBar();
            for i in &self.items {
                bar.removeStatusItem(&i.item);
            }
        }
    }
}

// --------------------------------------------------------------- Windows --

#[cfg(target_os = "windows")]
mod win {
    use std::cell::RefCell;

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{CreateBitmap, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Shell::{Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW};
    use windows_sys::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, CreateWindowExW, DefWindowProcW, DestroyIcon, RegisterClassW, HICON, HWND_MESSAGE, ICONINFO, WM_APP, WNDCLASSW};

    use super::{Clicks, Shown};

    /// The message the notification area sends the hidden window.
    const CALLBACK: u32 = WM_APP + 0x51;

    thread_local! {
        /// (the window procedure's view: our uID → the icon's key; the clicks)
        static ROUTE: RefCell<(Vec<(u32, (i64, i64))>, Option<Clicks>)> = const { RefCell::new((Vec::new(), None)) };
    }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if msg == CALLBACK {
            let mouse = (lparam & 0xFFFF) as i64;
            ROUTE.with(|r| {
                let r = r.borrow();
                if let (Some(&(_, key)), Some(clicks)) = (r.0.iter().find(|(id, _)| *id == wparam as u32), &r.1) {
                    clicks.push(key, vec![mouse]);
                }
            });
            return 0;
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// An HICON of a picture (32-bit with alpha).
    unsafe fn icon(w: usize, h: usize, rgba: &[u8]) -> HICON {
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w as i32,
            biHeight: -(h as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB as u32,
            ..std::mem::zeroed()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let color = CreateDIBSection(0, &info, DIB_RGB_COLORS, &mut bits, 0, 0);
        if color == 0 || bits.is_null() {
            return 0;
        }
        let dst = std::slice::from_raw_parts_mut(bits as *mut u8, w * h * 4);
        for (d, s) in dst.chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
            // (BGRA, premultiplied as Windows' icons want it)
            let a = u32::from(s[3]);
            d[0] = (u32::from(s[2]) * a / 255) as u8;
            d[1] = (u32::from(s[1]) * a / 255) as u8;
            d[2] = (u32::from(s[0]) * a / 255) as u8;
            d[3] = s[3];
        }
        let mask = CreateBitmap(w as i32, h as i32, 1, 1, std::ptr::null());
        let ii = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
        let hicon = CreateIconIndirect(&ii);
        DeleteObject(color);
        DeleteObject(mask);
        hicon
    }

    struct Item {
        key: (i64, i64),
        id: u32,
        hicon: HICON,
        shown: (String, (usize, usize, Vec<u8>)),
    }

    #[derive(Default)]
    pub struct Items {
        window: HWND,
        items: Vec<Item>,
        next_id: u32,
    }

    impl Items {
        fn window(&mut self) -> HWND {
            if self.window == 0 {
                unsafe {
                    let class = wide("RapidRTray");
                    let hinstance = GetModuleHandleW(std::ptr::null());
                    let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinstance, lpszClassName: class.as_ptr(), ..std::mem::zeroed() };
                    RegisterClassW(&wc);
                    self.window = CreateWindowExW(0, class.as_ptr(), std::ptr::null(), 0, 0, 0, 0, 0, HWND_MESSAGE, 0, hinstance, std::ptr::null());
                }
            }
            self.window
        }

        fn notify(&mut self, message: u32, item: &Item) {
            let hwnd = self.window();
            unsafe {
                let mut data: NOTIFYICONDATAW = std::mem::zeroed();
                data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                data.hWnd = hwnd;
                data.uID = item.id;
                data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
                data.uCallbackMessage = CALLBACK;
                data.hIcon = item.hicon;
                for (d, s) in data.szTip.iter_mut().zip(item.shown.0.encode_utf16().take(127)) {
                    *d = s;
                }
                Shell_NotifyIconW(message, &data);
            }
        }

        pub fn sync(&mut self, icons: &[Shown], clicks: &Clicks) {
            ROUTE.with(|r| r.borrow_mut().1 = Some(clicks.clone()));
            let gone: Vec<usize> = (0..self.items.len()).filter(|&i| !icons.iter().any(|(k, _, _)| *k == self.items[i].key)).collect();
            for i in gone.into_iter().rev() {
                let item = self.items.remove(i);
                self.notify(NIM_DELETE, &item);
                unsafe { DestroyIcon(item.hicon) };
            }
            for (key, tip, picture) in icons {
                let shown = (tip.clone(), picture.clone());
                match self.items.iter().position(|i| i.key == *key) {
                    Some(i) if self.items[i].shown != shown => {
                        let old = self.items[i].hicon;
                        self.items[i].hicon = unsafe { icon(picture.0, picture.1, &picture.2) };
                        self.items[i].shown = shown;
                        let item = std::mem::replace(&mut self.items[i], Item { key: *key, id: 0, hicon: 0, shown: Default::default() });
                        self.notify(NIM_MODIFY, &item);
                        self.items[i] = item;
                        unsafe { DestroyIcon(old) };
                    }
                    Some(_) => {}
                    None => {
                        self.next_id += 1;
                        let item = Item { key: *key, id: self.next_id, hicon: unsafe { icon(picture.0, picture.1, &picture.2) }, shown };
                        self.notify(NIM_ADD, &item);
                        self.items.push(item);
                    }
                }
            }
            let route: Vec<(u32, (i64, i64))> = self.items.iter().map(|i| (i.id, i.key)).collect();
            ROUTE.with(|r| r.borrow_mut().0 = route);
        }
    }

    impl Drop for Items {
        fn drop(&mut self) {
            for item in std::mem::take(&mut self.items) {
                self.notify(NIM_DELETE, &item);
                unsafe { DestroyIcon(item.hicon) };
            }
        }
    }
}

// ---------------------------------------------- Linux: StatusNotifierItem --

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
mod sni {
    use std::task::Waker;

    use super::{messages, Clicks, Shown};

    /// One icon on the session bus (org.kde.StatusNotifierItem).
    struct Item {
        key: (i64, i64),
        title: String,
        pixmap: Vec<(i32, i32, Vec<u8>)>,
        clicks: Clicks,
        waker: Waker,
    }

    impl Item {
        fn click(&self, left: bool, right: bool) {
            self.clicks.push(self.key, messages(left, right, false));
        }
    }

    #[zbus::interface(name = "org.kde.StatusNotifierItem")]
    impl Item {
        #[zbus(property)]
        fn category(&self) -> String {
            "ApplicationStatus".into()
        }
        #[zbus(property)]
        fn id(&self) -> String {
            format!("rapidr-{}-{}", self.key.0, self.key.1)
        }
        #[zbus(property)]
        fn title(&self) -> String {
            self.title.clone()
        }
        #[zbus(property)]
        fn status(&self) -> String {
            "Active".into()
        }
        #[zbus(property)]
        fn window_id(&self) -> i32 {
            0
        }
        #[zbus(property)]
        fn icon_name(&self) -> String {
            String::new()
        }
        #[zbus(property)]
        fn icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
            self.pixmap.clone()
        }
        #[zbus(property)]
        fn tool_tip(&self) -> (String, Vec<(i32, i32, Vec<u8>)>, String, String) {
            (String::new(), Vec::new(), self.title.clone(), String::new())
        }
        #[zbus(property)]
        fn item_is_menu(&self) -> bool {
            false
        }
        #[zbus(property)]
        fn menu(&self) -> zbus::zvariant::OwnedObjectPath {
            zbus::zvariant::OwnedObjectPath::try_from("/NO_DBUSMENU").unwrap_or_default()
        }
        fn activate(&self, _x: i32, _y: i32) {
            self.click(true, false);
        }
        fn secondary_activate(&self, _x: i32, _y: i32) {
            self.click(false, false);
        }
        fn context_menu(&self, _x: i32, _y: i32) {
            self.click(false, true);
        }
        fn scroll(&self, _delta: i32, _orientation: String) {}
    }

    /// A picture as StatusNotifierItem's ARGB32, big-endian.
    fn pixmap(w: usize, h: usize, rgba: &[u8]) -> (i32, i32, Vec<u8>) {
        let argb = rgba.chunks_exact(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
        (w as i32, h as i32, argb)
    }

    #[derive(Default)]
    pub struct Items {
        items: Vec<((i64, i64), (String, (usize, usize, Vec<u8>)), zbus::blocking::Connection)>,
        failed: bool,
    }

    impl Items {
        fn register(key: (i64, i64), tip: &str, picture: &(usize, usize, Vec<u8>), clicks: &Clicks, waker: &Waker) -> zbus::Result<zbus::blocking::Connection> {
            let item = Item { key, title: tip.to_string(), pixmap: vec![pixmap(picture.0, picture.1, &picture.2)], clicks: clicks.clone(), waker: waker.clone() };
            let name = format!("org.kde.StatusNotifierItem-{}-{}", std::process::id(), key.1.rem_euclid(1 << 30));
            let conn = zbus::blocking::connection::Builder::session()?.name(name.as_str())?.serve_at("/StatusNotifierItem", item)?.build()?;
            conn.call_method(Some("org.kde.StatusNotifierWatcher"), "/StatusNotifierWatcher", Some("org.kde.StatusNotifierWatcher"), "RegisterStatusNotifierItem", &(name.as_str(),))?;
            Ok(conn)
        }

        pub fn sync(&mut self, icons: &[Shown], clicks: &Clicks, waker: &Waker) {
            // (an icon changed or gone: its connection dropped, which takes
            // it off the bus; a changed one registers again)
            self.items.retain(|(k, shown, _)| icons.iter().any(|(key, tip, picture)| key == k && shown.0 == *tip && shown.1 == *picture));
            for (key, tip, picture) in icons {
                if self.failed || self.items.iter().any(|(k, _, _)| k == key) {
                    continue;
                }
                match Self::register(*key, tip, picture, clicks, waker) {
                    Ok(conn) => self.items.push((*key, (tip.clone(), picture.clone()), conn)),
                    Err(e) => {
                        // (no session bus, or no tray to show it: said once)
                        eprintln!("[rapidr] the system tray isn't available ({e})");
                        self.failed = true;
                    }
                }
            }
        }
    }
}
