//! (C-SYS) A cursor the program put in `Screen.Cursors(i)` — a Windows
//! HCURSOR from `LoadCursorFromFile` / `LoadCursor` (DLL calls,
//! docs/windows-dll-calls.md) — shown by winit: its pixels drawn once into
//! a 32-bit bitmap on black and on white (which gives every pixel's colour
//! and alpha, whatever the cursor's format: colour, monochrome, the first
//! frame of an animated one) and made a winit custom cursor, kept per
//! handle.

use std::cell::RefCell;
use std::collections::HashMap;

use winit::event_loop::ActiveEventLoop;
use winit::window::{CustomCursor, CustomCursorSource};

thread_local! {
    static MADE: RefCell<HashMap<i64, Option<CustomCursor>>> = RefCell::new(HashMap::new());
}

/// winit's cursor for HCURSOR `handle` (`None` when it isn't a cursor).
pub fn custom(el: &ActiveEventLoop, handle: i64) -> Option<CustomCursor> {
    if let Some(made) = MADE.with(|m| m.borrow().get(&handle).cloned()) {
        return made;
    }
    let made = source(handle).map(|s| el.create_custom_cursor(s));
    MADE.with(|m| m.borrow_mut().insert(handle, made.clone()));
    made
}

/// The cursor's pixels (RGBA, straight alpha) and hotspot.
fn source(handle: i64) -> Option<CustomCursorSource> {
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetObjectW, SelectObject, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{DrawIconEx, GetIconInfo, DI_NORMAL, HICON, ICONINFO};
    if handle == 0 {
        return None;
    }
    let icon = handle as HICON;
    // SAFETY: plain GDI / USER calls on a handle the program got from
    // Windows; each failure is checked, every object made here is deleted,
    // and the DIB's bits are read only while it is alive, within its
    // w × h × 4 bytes.
    unsafe {
        let mut info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(icon, &mut info) == 0 {
            return None;
        }
        let mut bm: BITMAP = std::mem::zeroed();
        let probe = if info.hbmColor != 0 { info.hbmColor } else { info.hbmMask };
        GetObjectW(probe, std::mem::size_of::<BITMAP>() as i32, &mut bm as *mut BITMAP as *mut _);
        let (w, h) = (bm.bmWidth, if info.hbmColor != 0 { bm.bmHeight } else { bm.bmHeight / 2 });
        let (hx, hy) = (info.xHotspot, info.yHotspot);
        if info.hbmColor != 0 {
            DeleteObject(info.hbmColor);
        }
        if info.hbmMask != 0 {
            DeleteObject(info.hbmMask);
        }
        if !(1..=256).contains(&w) || !(1..=256).contains(&h) {
            return None;
        }
        let dc = CreateCompatibleDC(0);
        if dc == 0 {
            return None;
        }
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB as u32,
            ..std::mem::zeroed()
        };
        let n = (w * h) as usize;
        let draw = |fill: u8| -> Option<Vec<u8>> {
            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let dib = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &mut bits, 0, 0);
            if dib == 0 || bits.is_null() {
                return None;
            }
            let old = SelectObject(dc, dib);
            std::ptr::write_bytes(bits as *mut u8, fill, n * 4);
            DrawIconEx(dc, 0, 0, icon, w, h, 0, 0, DI_NORMAL);
            let out = std::slice::from_raw_parts(bits as *const u8, n * 4).to_vec();
            SelectObject(dc, old);
            DeleteObject(dib);
            Some(out)
        };
        let on_black = draw(0);
        let on_white = draw(255);
        DeleteDC(dc);
        let (b, wt) = (on_black?, on_white?);
        let mut rgba = vec![0u8; n * 4];
        for i in 0..n {
            let p = i * 4;
            // alpha from how much the background shows through (the
            // largest channel difference); colour un-premultiplied
            let diff = (0..3).map(|c| wt[p + c] as i32 - b[p + c] as i32).max().unwrap_or(255).clamp(0, 255);
            let a = 255 - diff;
            rgba[p + 3] = a as u8;
            if a > 0 {
                for c in 0..3 {
                    // (BGRA → RGBA)
                    let v = (b[p + (2 - c)] as i32 * 255 / a).min(255);
                    rgba[p + c] = v as u8;
                }
            }
        }
        CustomCursor::from_rgba(rgba, w as u16, h as u16, (hx as i32).clamp(0, w - 1) as u16, (hy as i32).clamp(0, h - 1) as u16).ok()
    }
}
