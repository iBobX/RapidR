//! A form's outline on the desktop (`Form.ShapeForm`, `rapidr_value::shape`):
//! the window shows — and takes the mouse — only where the bitmap's pixels
//! aren't its transparent colour. The bitmap's pixel (x, y) is the
//! window's (x, y) from its top left corner, the frame RapidR accounts
//! (`rapidr_value::layout::form_frame`) included, as RapidQ's runtime gives
//! a window its region; a shaped window has no system frame of its own (the
//! outline is its edge).
//!
//! - **Windows**: the window's region (SetWindowRgn), as RapidQ's runtime.
//! - **macOS**: a mask on the window's view, the window not opaque: what's
//!   outside is the desktop, and the mouse goes through it there.
//! - **X11**: the window's bounding shape (XFixes' shape region).
//! - **Wayland** has no way for a client to cut its window: the window
//!   keeps its rectangle there (`apply` says false).

use winit::window::Window;

/// Rectangles in the window's inside (logical pixels).
pub type Rects = [(i64, i64, i64, i64)];

/// The outline of form `form`, if it has one, as rectangles in its
/// window's inside: the shape's moved by where the inside starts in the
/// accounted frame (`frame_style`'s inset).
pub fn rects_of(form: &str, frame_style: i64) -> Option<Vec<(i64, i64, i64, i64)>> {
    let shape = rapidr_value::shape::get(form)?;
    let (fw, fh) = rapidr_value::layout::form_frame(frame_style);
    let (ox, oy) = (fw / 2, fh - fw / 2);
    Some(shape.rects().into_iter().map(|(x, y, w, h)| (x - ox, y - oy, w, h)).collect())
}

/// Gives `window` the outline `rects` (`None`: its whole rectangle again);
/// `scale` is its device pixels per logical pixel. Whether the platform
/// could.
pub fn apply(window: &Window, rects: Option<&Rects>, scale: f64) -> bool {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = window.window_handle() else { return false };
    let _ = (rects, scale);
    match handle.as_raw() {
        #[cfg(target_os = "windows")]
        RawWindowHandle::Win32(h) => {
            win32::apply(h.hwnd.get(), rects, scale);
            true
        }
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(h) => {
            appkit::apply(h.ns_view.as_ptr(), rects);
            true
        }
        #[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
        RawWindowHandle::Xlib(h) => {
            use winit::raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
            let display = window.display_handle().ok().and_then(|d| match d.as_raw() {
                RawDisplayHandle::Xlib(x) => x.display,
                _ => None,
            });
            match display {
                Some(display) => x11::apply(display.as_ptr(), h.window, rects, scale),
                None => false,
            }
        }
        _ => false,
    }
}

/// Logical rectangles in device pixels (edges rounded, so neighbours
/// still touch).
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn device(rects: &Rects, scale: f64) -> Vec<(i32, i32, i32, i32)> {
    rects
        .iter()
        .map(|&(x, y, w, h)| {
            let (x0, y0) = ((x as f64 * scale).round() as i32, (y as f64 * scale).round() as i32);
            let (x1, y1) = (((x + w) as f64 * scale).round() as i32, ((y + h) as f64 * scale).round() as i32);
            (x0, y0, x1, y1)
        })
        .collect()
}

#[cfg(target_os = "windows")]
mod win32 {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, RGN_OR};

    pub fn apply(hwnd: HWND, rects: Option<&super::Rects>, scale: f64) {
        // SAFETY: the window is live and ours; every region made here is
        // either deleted or handed to the system (SetWindowRgn owns it).
        unsafe {
            let Some(rects) = rects else {
                SetWindowRgn(hwnd, 0, 1);
                return;
            };
            let region = CreateRectRgn(0, 0, 0, 0);
            for (x0, y0, x1, y1) in super::device(rects, scale) {
                let part = CreateRectRgn(x0, y0, x1, y1);
                CombineRgn(region, region, part, RGN_OR);
                DeleteObject(part);
            }
            SetWindowRgn(hwnd, region, 1);
        }
    }
}

#[cfg(target_os = "macos")]
mod appkit {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    pub fn apply(ns_view: *mut std::ffi::c_void, rects: Option<&super::Rects>) {
        let view = ns_view.cast::<AnyObject>();
        // SAFETY: the view is the live window's, on the main thread (the
        // host's pump); the layers made here are autoreleased and kept by
        // the view's layer as its mask.
        unsafe {
            let window: *mut AnyObject = msg_send![view, window];
            if window.is_null() {
                return;
            }
            let _: () = msg_send![view, setWantsLayer: true];
            let layer: *mut AnyObject = msg_send![view, layer];
            if layer.is_null() {
                return;
            }
            let Some(rects) = rects else {
                let _: () = msg_send![layer, setMask: std::ptr::null_mut::<AnyObject>()];
                let _: () = msg_send![window, setOpaque: true];
                let _: () = msg_send![window, setHasShadow: true];
                return;
            };
            let bounds: NSRect = msg_send![layer, bounds];
            let mask: *mut AnyObject = msg_send![class!(CALayer), layer];
            let _: () = msg_send![mask, setFrame: bounds];
            let black: *mut AnyObject = msg_send![class!(NSColor), blackColor];
            let cg: *const std::ffi::c_void = msg_send![black, CGColor];
            // (the outline's rows count from the top: so do the view's
            // layer's when it shows flipped — winit's view is — else from
            // the bottom)
            let top_down: bool = msg_send![layer, contentsAreFlipped];
            let height = bounds.size.height;
            for &(x, y, w, h) in rects {
                let part: *mut AnyObject = msg_send![class!(CALayer), layer];
                let _: () = msg_send![part, setBackgroundColor: cg];
                let top = if top_down { y as f64 } else { height - (y + h) as f64 };
                let frame = NSRect::new(NSPoint::new(x as f64, top), NSSize::new(w as f64, h as f64));
                let _: () = msg_send![part, setFrame: frame];
                let _: () = msg_send![mask, addSublayer: part];
            }
            let _: () = msg_send![layer, setMask: mask];
            let clear: *mut AnyObject = msg_send![class!(NSColor), clearColor];
            let _: () = msg_send![window, setBackgroundColor: clear];
            let _: () = msg_send![window, setOpaque: false];
            let _: () = msg_send![window, setHasShadow: false];
        }
    }
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
mod x11 {
    use x11_dl::xfixes::Xlib_xfixes;
    use x11_dl::xlib::{Display, XRectangle, Xlib};

    /// XFixes' shape kinds (X Shape extension: ShapeBounding, ShapeInput).
    const SHAPE_BOUNDING: i32 = 0;
    const SHAPE_INPUT: i32 = 2;

    pub fn apply(display: *mut std::ffi::c_void, window: std::ffi::c_ulong, rects: Option<&super::Rects>, scale: f64) -> bool {
        let (Ok(fixes), Ok(xlib)) = (Xlib_xfixes::open(), Xlib::open()) else { return false };
        let display = display.cast::<Display>();
        // SAFETY: the display and window are the live window's (winit's
        // Xlib connection); the region made here is destroyed after use.
        unsafe {
            match rects {
                None => {
                    (fixes.XFixesSetWindowShapeRegion)(display, window, SHAPE_BOUNDING, 0, 0, 0);
                    (fixes.XFixesSetWindowShapeRegion)(display, window, SHAPE_INPUT, 0, 0, 0);
                }
                Some(rects) => {
                    let mut xr: Vec<XRectangle> = super::device(rects, scale)
                        .into_iter()
                        .filter(|r| r.2 > r.0 && r.3 > r.1)
                        .map(|(x0, y0, x1, y1)| XRectangle { x: x0.clamp(i16::MIN as i32, i16::MAX as i32) as i16, y: y0.clamp(i16::MIN as i32, i16::MAX as i32) as i16, width: (x1 - x0).min(u16::MAX as i32) as u16, height: (y1 - y0).min(u16::MAX as i32) as u16 })
                        .collect();
                    let region = (fixes.XFixesCreateRegion)(display, xr.as_mut_ptr(), xr.len() as i32);
                    (fixes.XFixesSetWindowShapeRegion)(display, window, SHAPE_BOUNDING, 0, 0, region);
                    (fixes.XFixesSetWindowShapeRegion)(display, window, SHAPE_INPUT, 0, 0, region);
                    (fixes.XFixesDestroyRegion)(display, region);
                }
            }
            (xlib.XFlush)(display);
        }
        true
    }
}
