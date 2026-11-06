//! An RFormEx's window look on the desktop (`rapidr_value::formex`,
//! `WindowSpec::alpha` …; docs/rformex-design.md §5):
//!
//! - **On top** (fsStayOnTop): winit's always-on-top level (Windows,
//!   macOS, X11; on Wayland the compositor decides).
//! - **No taskbar button** (RemoveTaskBar): Windows only.
//! - **Opacity** (Transparent): a layered window's alpha on Windows,
//!   NSWindow's alphaValue on macOS; not on Linux yet.
//! - **The background a hole** (TransparentClient): the form's colour as a
//!   layered window's colour key on Windows; on macOS the window's outline
//!   made of the frame's pixels that aren't that colour (the window draws on
//!   the CPU then: its pixels are read — [`mask`]); not on Linux yet.
//!
//! (Linux: `_NET_WM_WINDOW_OPACITY` and an XFixes shape were tried on X11
//! through XWayland, 2026-11-06, and showed nothing there, so neither is
//! claimed: the window stays as it was until they are seen working on a real
//! X11 session.) What a system can't do is said once on stderr, as
//! ShapeForm's outline.

use rapidr_ui_app::desktop::WindowSpec;
use winit::window::{Window, WindowLevel};

/// The colour form `form`'s background is drawn in (0xRRGGBB): its Color,
/// else the theme's face — the colour TransparentClient makes a hole.
pub fn key_of(store: &dyn rapidr_ui_kernel::Store, form: &str) -> u32 {
    rapidr_ui_kernel::paint::color_of(store, form).unwrap_or_else(|| rapidr_value::theme::current().face)
}

/// Whether the spec asks anything of the window's look (an ordinary
/// window's is left alone: nothing is set on it).
pub fn asks(spec: &WindowSpec) -> bool {
    spec.alpha.is_some() || spec.color_key || spec.on_top || spec.skip_taskbar
}

/// Whether the window must draw on the CPU for its look: a colour key
/// reads (macOS) or is matched against (Windows' layered window, which a
/// GPU swap chain doesn't take) the frame's pixels.
pub fn needs_cpu(spec: &WindowSpec) -> bool {
    spec.color_key && keys_colours()
}

/// Whether this system's windows can have a colour key here (Windows,
/// macOS).
pub fn keys_colours() -> bool {
    cfg!(any(target_os = "windows", target_os = "macos"))
}

/// The window's look applied: on top, taskbar, opacity and (Windows) the
/// colour key `key` (0xRRGGBB, the colour the form's background is drawn
/// in). macOS' and X11's colour key is the outline [`mask`] makes.
pub fn apply(window: &Window, spec: &WindowSpec, key: u32) {
    window.set_window_level(if spec.on_top { WindowLevel::AlwaysOnTop } else { WindowLevel::Normal });
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::WindowExtWindows;
        window.set_skip_taskbar(spec.skip_taskbar);
        win32::layered(window, spec.alpha, spec.color_key.then_some(key));
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = key;
        if !opacity(window, spec.alpha) && spec.alpha.is_some() {
            once("Transparent: windows here can't be made see-through yet (Linux): it stays opaque");
        }
        if spec.color_key && !keys_colours() {
            no_mask();
        }
    }
}

fn once(message: &str) {
    thread_local! {
        static SAID: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    SAID.with(|s| {
        let mut s = s.borrow_mut();
        if !s.iter().any(|m| m == message) {
            eprintln!("[rapidr] {message}");
            s.push(message.to_string());
        }
    });
}

/// The colour key on a system that can't cut a window here (Linux): said
/// once.
pub fn no_mask() {
    once("TransparentClient: windows here can't have holes cut in them yet (Linux): it keeps its background");
}

/// (macOS) Whole-window opacity; whether the system could.
#[cfg(not(target_os = "windows"))]
fn opacity(window: &Window, alpha: Option<u8>) -> bool {
    #[cfg(target_os = "macos")]
    {
        use objc2::msg_send;
        use objc2::runtime::AnyObject;
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let Ok(handle) = window.window_handle() else { return false };
        let RawWindowHandle::AppKit(h) = handle.as_raw() else { return false };
        let a = f64::from(alpha.unwrap_or(255)) / 255.0;
        let view = h.ns_view.as_ptr().cast::<AnyObject>();
        // SAFETY: the live window's view, on the main thread (the host's
        // pump).
        unsafe {
            let w: *mut AnyObject = msg_send![view, window];
            if !w.is_null() {
                let _: () = msg_send![w, setAlphaValue: a];
            }
        }
        true
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, alpha);
        false
    }
}

/// Rectangles (x, y, width, height) in a window's inside, logical pixels.
pub type Outline = Vec<(i64, i64, i64, i64)>;

/// The outline of the frame's pixels that aren't `key` (0xRRGGBB): rows of
/// runs merged into rectangles, in the window's inside (logical pixels:
/// device pixels / `scale`, edges rounded outwards). `rgba` is the frame
/// (`width` × `height` device pixels, 4 bytes each).
pub fn mask(rgba: &[u8], width: usize, height: usize, key: u32, scale: f64) -> Outline {
    let (kr, kg, kb) = ((key >> 16) as u8, (key >> 8) as u8, key as u8);
    let s = if scale > 0.0 { scale } else { 1.0 };
    // (per logical row: the runs of logical columns that show)
    let rows = ((height as f64) / s).ceil() as usize;
    let mut out: Vec<(i64, i64, i64, i64)> = Vec::new();
    let mut open: Vec<(i64, i64, i64)> = Vec::new(); // (x0, x1, top): rectangles still growing
    let mut prev: Vec<(i64, i64)> = Vec::new();
    for ly in 0..=rows {
        let mut runs: Vec<(i64, i64)> = Vec::new();
        if ly < rows {
            let (dy0, dy1) = (((ly as f64) * s).floor() as usize, ((((ly + 1) as f64) * s).ceil() as usize).min(height));
            let mut keep = vec![false; width];
            for dy in dy0..dy1 {
                let row = &rgba[dy * width * 4..(dy + 1) * width * 4];
                for (x, p) in row.as_chunks::<4>().0.iter().enumerate() {
                    if !(p[0] == kr && p[1] == kg && p[2] == kb) {
                        keep[x] = true;
                    }
                }
            }
            let mut start = None;
            for (x, &k) in keep.iter().enumerate() {
                match (k, start) {
                    (true, None) => start = Some(x),
                    (false, Some(x0)) => {
                        runs.push(((x0 as f64 / s).floor() as i64, (x as f64 / s).ceil() as i64));
                        start = None;
                    }
                    _ => {}
                }
            }
            if let Some(x0) = start {
                runs.push(((x0 as f64 / s).floor() as i64, (width as f64 / s).ceil() as i64));
            }
            // (neighbouring runs that touch once rounded: one)
            runs.dedup_by(|b, a| {
                if b.0 <= a.1 {
                    a.1 = a.1.max(b.1);
                    true
                } else {
                    false
                }
            });
        }
        if runs != prev {
            for (x0, x1, top) in open.drain(..) {
                out.push((x0, top, x1 - x0, ly as i64 - top));
            }
            open = runs.iter().map(|&(a, b)| (a, b, ly as i64)).collect();
            prev = runs;
        }
    }
    out
}

#[cfg(target_os = "windows")]
mod win32 {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, GWL_EXSTYLE, LWA_ALPHA, LWA_COLORKEY, WS_EX_LAYERED};
    use winit::window::Window;

    fn hwnd_of(window: &Window) -> Option<HWND> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        match window.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(w) => Some(w.hwnd.get() as HWND),
            _ => None,
        }
    }

    /// The window layered with `alpha` and the colour key `key` (0xRRGGBB),
    /// or not layered when it has neither.
    pub fn layered(window: &Window, alpha: Option<u8>, key: Option<u32>) {
        let Some(hwnd) = hwnd_of(window) else { return };
        // SAFETY: the live window, ours; the style change and the layered
        // attributes take plain values.
        unsafe {
            let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            if alpha.is_none() && key.is_none() {
                if style & WS_EX_LAYERED != 0 {
                    SetWindowLongW(hwnd, GWL_EXSTYLE, (style & !WS_EX_LAYERED) as i32);
                }
                return;
            }
            SetWindowLongW(hwnd, GWL_EXSTYLE, (style | WS_EX_LAYERED) as i32);
            let mut flags = 0;
            if alpha.is_some() {
                flags |= LWA_ALPHA;
            }
            // (COLORREF is 0x00BBGGRR)
            let colorref = key.map_or(0, |k| ((k & 0xFF) << 16) | (k & 0xFF00) | ((k >> 16) & 0xFF));
            if key.is_some() {
                flags |= LWA_COLORKEY;
            }
            SetLayeredWindowAttributes(hwnd, colorref, alpha.unwrap_or(255), flags);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mask;

    fn frame(w: usize, h: usize, key: [u8; 3], solid: &[(usize, usize)]) -> Vec<u8> {
        let mut v = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let c = if solid.contains(&(x, y)) { [10, 20, 30] } else { key };
                v.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        v
    }

    #[test]
    fn the_key_colour_is_a_hole() {
        // (a 2 × 2 block at (1, 1) on a key-coloured 4 × 4 frame)
        let f = frame(4, 4, [0xE5, 0xFD, 0xFF], &[(1, 1), (2, 1), (1, 2), (2, 2)]);
        assert_eq!(mask(&f, 4, 4, 0xE5FDFF, 1.0), vec![(1, 1, 2, 2)]);
        // (at 2x: the same block in logical pixels, a 2 × 2 frame)
        assert_eq!(mask(&f, 4, 4, 0xE5FDFF, 2.0), vec![(0, 0, 2, 2)]);
        // (all key: nothing shows)
        assert!(mask(&frame(3, 3, [1, 2, 3], &[]), 3, 3, 0x010203, 1.0).is_empty());
    }
}
