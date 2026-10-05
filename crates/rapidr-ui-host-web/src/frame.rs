//! A window's frame on the page, drawn by the kernel (docs/web-host-plan.md
//! §3.4): the desktop's windows get theirs from the window system; the
//! web host's windows are canvases on the page, so their title bar, border
//! and title bar buttons are drawn here — with the kernel's [`Painter`] in
//! the kernel's theme, as a QFORMMDI child's frame (`components/mdi.rs`):
//! the active window's title bar in the caption colours, the others in the
//! inactive ones, the buttons the program's BorderStyle / BorderIcons
//! leave ([`Frame`]).
//!
//! The metrics are the ones every runtime accounts a form's frame with
//! (`rapidr_value::layout::form_frame`: a 1-pixel border and a 29-pixel
//! title bar), so a form's Width / Height and ClientWidth / ClientHeight
//! are what they are on the desktop.

use rapidr_ui_app::desktop::Frame;
use rapidr_ui_kernel::display::DisplayList;
use rapidr_ui_kernel::paint::Painter;
use rapidr_ui_kernel::Rect;
use rapidr_value::layout::{FORM_BORDER, FORM_CAPTION};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Place;

/// What a window's frame shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub title: String,
    /// The frontmost window (it has the keyboard).
    pub active: bool,
    /// A frame at all (BorderStyle <> bsNone).
    pub border: bool,
    pub frame: Frame,
    pub maximized: bool,
    /// The theme's generation (`rapidr_value::theme::generation`): drawn
    /// again in a new one.
    pub theme: u64,
    /// The form's icon (its IcoHandle / Icon, else the application's), at
    /// the title bar's left as Windows draws it.
    pub icon: Option<rapidr_ui_app::desktop::Icon>,
}

/// A button's width on the title bar.
const BUTTON_W: i64 = 28;

/// Where the inside (the client canvas) starts in the window: (left, top).
pub fn inset(border: bool) -> (i64, i64) {
    if border {
        (FORM_BORDER, FORM_BORDER + FORM_CAPTION)
    } else {
        (0, 0)
    }
}

/// The window's whole size for an inside of `inside` (logical pixels).
pub fn outer(inside: (i64, i64), border: bool) -> (i64, i64) {
    let (fw, fh) = rapidr_value::layout::form_frame(if border { 2 } else { 0 });
    (inside.0 + fw, inside.1 + fh)
}

/// The title bar buttons shown: (slot, part) from the right — close, then
/// maximize and minimize (both shown when either is: the other greyed, as
/// Windows draws them); none without the system menu.
fn buttons(f: Frame) -> Vec<(i64, Part)> {
    let mut b = Vec::new();
    if f.close {
        b.push((0, Part::Close));
        if f.minimize || f.maximize {
            b.push((1, Part::Maximize));
            b.push((2, Part::Minimize));
        }
    }
    b
}

/// Title bar button `slot`'s rectangle (from the right) in a `w` wide window.
fn button_rect(w: i64, slot: i64) -> Rect {
    (w - FORM_BORDER - (slot + 1) * BUTTON_W, FORM_BORDER + 3, BUTTON_W - 2, FORM_CAPTION - 6)
}

fn enabled(look: &Look, part: Part) -> bool {
    match part {
        Part::Close => look.frame.close,
        Part::Maximize => look.frame.maximize,
        Part::Minimize => look.frame.minimize,
        _ => false,
    }
}

/// The window's frame drawn, `size` its whole size (logical), for a screen
/// of `scale` device pixels per logical pixel. The inside is left as the
/// face colour (the client canvas covers it).
pub fn paint(look: &Look, size: (i64, i64), scale: f64) -> DisplayList {
    let mut list = DisplayList { size, scale, ..Default::default() };
    if !look.border {
        return list;
    }
    let (w, h) = size;
    let mut p = Painter::new(&mut list);
    let t = p.theme();
    let (bar, ink) = if look.active { (t.caption, t.caption_text) } else { (t.inactive_caption, t.inactive_caption_text) };
    p.fill((0, 0, w, h), t.face);
    p.frame((0, 0, w, h), if t.fluent() { if look.active { t.caption } else { t.border } } else { t.dark_shadow });
    let (bx, by, bw, bh) = (FORM_BORDER, FORM_BORDER, (w - 2 * FORM_BORDER).max(0), FORM_CAPTION);
    p.fill((bx, by, bw, bh), bar);
    let shown = buttons(look.frame);
    // (the icon, 16 × 16, then the title after it)
    let mut text_x = bx + 8;
    if let Some(icon) = &look.icon {
        let picture = rapidr_ui_kernel::display::Picture { width: icon.width as usize, height: icon.height as usize, rgba: icon.rgba.clone() };
        let revision = icon.rgba.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3));
        p.picture("rapidr:frame-icon", revision, std::sync::Arc::new(picture), (bx + 6, by + (bh - 16) / 2, 16, 16));
        text_x += 20;
    }
    let room = (bw - shown.len() as i64 * BUTTON_W - 10 - (text_x - bx - 8)).max(0);
    let font = Font { name: "Arial".into(), size: 9, color: rapidr_value::theme::bgr(ink) as i64, styles: 1 };
    p.clipped((text_x - 2, by, room, bh), |p| p.text((text_x, by, room, bh), &look.title, &font, ink, Place::Left));
    for (slot, part) in shown {
        let r = button_rect(w, slot);
        if r.0 <= bx + 6 {
            continue;
        }
        let on = enabled(look, part);
        let glyph_ink = if t.fluent() { ink } else if on { t.text } else { t.gray_text };
        if !t.fluent() {
            p.fill(r, t.face);
            p.edge(r, &[t.light, t.face], &[t.dark_shadow, t.shadow]);
        }
        glyph(&mut p, part, r, look.maximized, if on { glyph_ink } else { t.gray_text });
    }
    list
}

/// A title bar button's glyph (×, □ / restore, _), as a QFORMMDI child's.
fn glyph(p: &mut Painter, part: Part, (x, y, w, h): Rect, maximized: bool, ink: u32) {
    let fluent = p.fluent();
    let (cx, cy) = (x + w / 2, y + h / 2);
    let (fx, fy) = (cx as f64, cy as f64);
    match part {
        Part::Close if fluent => {
            p.stroke(&[(fx - 4.0, fy - 4.0), (fx + 4.0, fy + 4.0)], ink, 1.0);
            p.stroke(&[(fx - 4.0, fy + 4.0), (fx + 4.0, fy - 4.0)], ink, 1.0);
        }
        Part::Close => {
            for d in [0.0, 1.0] {
                let (l, t) = ((cx - 4) as f64 + d, (cy - 4) as f64);
                p.line((l + 0.5, t + 0.5), (l + 7.5, t + 7.5), ink);
                p.line((l + 0.5, t + 7.5), (l + 7.5, t + 0.5), ink);
            }
        }
        Part::Maximize if maximized => {
            for (bx, by) in [(cx - 2, cy - 5), (cx - 5, cy - 2)] {
                if !fluent {
                    p.fill((bx, by, 7, 7), p.theme().face);
                }
                p.edge((bx, by, 7, 7), &[ink], &[ink]);
                p.fill((bx, by + 1, 7, 1), ink);
            }
        }
        Part::Maximize if fluent => p.ring((cx - 4, cy - 4, 9, 9), 1.5, ink, 1.0),
        Part::Maximize => {
            p.edge((cx - 5, cy - 5, 10, 9), &[ink], &[ink]);
            p.fill((cx - 5, cy - 4, 10, 1), ink);
        }
        Part::Minimize if fluent => p.fill((cx - 4, cy, 9, 1), ink),
        Part::Minimize => p.fill((cx - 4, cy + 2, 7, 2), ink),
        _ => {}
    }
}

/// What's at (x, y) of a window's frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Close,
    Maximize,
    Minimize,
    /// The title bar (dragged: the window moves; double-clicked: it's
    /// maximized or restored).
    Title,
    None,
}

/// The part of `look`'s frame (whole size `size`) at (x, y), logical.
pub fn hit(look: &Look, size: (i64, i64), x: f64, y: f64) -> Part {
    if !look.border {
        return Part::None;
    }
    let (w, _) = size;
    let (xi, yi) = (x.floor() as i64, y.floor() as i64);
    if !(FORM_BORDER..FORM_BORDER + FORM_CAPTION).contains(&yi) {
        return Part::None;
    }
    for (slot, part) in buttons(look.frame) {
        let (bx, by, bw, bh) = button_rect(w, slot);
        if xi >= bx && xi < bx + bw && yi >= by && yi < by + bh {
            return if enabled(look, part) { part } else { Part::None };
        }
    }
    Part::Title
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> Look {
        Look { title: "Form1".into(), active: true, border: true, frame: Frame::default(), maximized: false, theme: 0, icon: None }
    }

    #[test]
    fn the_frame_is_every_runtimes() {
        assert_eq!(outer((318, 209), true), (320, 240));
        assert_eq!(inset(true), (1, 30));
        assert_eq!(outer((100, 50), false), (100, 50));
    }

    #[test]
    fn buttons_and_the_title_bar() {
        let l = look();
        assert_eq!(hit(&l, (320, 240), 315.0, 15.0), Part::Close);
        assert_eq!(hit(&l, (320, 240), 287.0, 15.0), Part::Maximize);
        assert_eq!(hit(&l, (320, 240), 259.0, 15.0), Part::Minimize);
        assert_eq!(hit(&l, (320, 240), 40.0, 15.0), Part::Title);
        assert_eq!(hit(&l, (320, 240), 40.0, 100.0), Part::None);
        // (a dialog's frame: a close box only)
        let d = Look { frame: Frame { resizable: false, close: true, minimize: false, maximize: false }, ..look() };
        assert_eq!(hit(&d, (320, 240), 287.0, 15.0), Part::Title);
        let list = paint(&l, (320, 240), 1.0);
        assert!(!list.items.is_empty());
    }
}
