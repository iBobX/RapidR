//! A form's window frame as the kernel draws it, in the theme: the title
//! bar (the active window's in the caption colours, the others' in the
//! inactive ones), the border, the form's icon and the title bar buttons
//! its BorderStyle / BorderIcons leave ([`Frame`]). The desktop's windows
//! get theirs from the window system; this one is drawn where the kernel
//! draws the whole window — the web host's windows (canvases on a page) and
//! the form designer (RDESIGNSURFACE shows the designed form in its frame).
//!
//! The metrics are the ones every runtime accounts a form's frame with
//! (`rapidr_value::layout::form_frame`: a 1-pixel border and a 29-pixel
//! title bar), so a form's Width / Height and ClientWidth / ClientHeight
//! are what they are on the desktop.

use std::sync::Arc;

use rapidr_value::layout::{FORM_BORDER, FORM_CAPTION};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};

use crate::display::{DisplayList, Picture};
use crate::paint::Painter;

/// A form's window frame: what the window system (the desktop's) or a
/// kernel-drawn frame shows.
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

/// What a window's frame shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Look {
    pub title: String,
    /// The frontmost window (it has the keyboard).
    pub active: bool,
    /// A frame at all (BorderStyle <> bsNone).
    pub border: bool,
    /// Its title bar (`HideTitleBar` takes it away: the border alone).
    pub caption: bool,
    pub frame: Frame,
    pub maximized: bool,
    /// The form's icon (its IcoHandle / Icon, else the application's), at
    /// the title bar's left as Windows draws it: its picture and a number
    /// that changes when its pixels do.
    pub icon: Option<(Arc<Picture>, u64)>,
}

/// A button's width on the title bar.
const BUTTON_W: i64 = 28;

/// Where the inside (the client area) starts in the window: (left, top)
/// — `caption`: with its title bar.
pub fn inset(border: bool, caption: bool) -> (i64, i64) {
    match (border, caption) {
        (true, true) => (FORM_BORDER, FORM_BORDER + FORM_CAPTION),
        (true, false) => (FORM_BORDER, FORM_BORDER),
        _ => (0, 0),
    }
}

/// The window's whole size for an inside of `inside` (logical pixels).
pub fn outer(inside: (i64, i64), border: bool, caption: bool) -> (i64, i64) {
    let style = match (border, caption) {
        (true, true) => 2,
        (true, false) => rapidr_value::layout::FRAME_NO_CAPTION,
        _ => 0,
    };
    let (fw, fh) = rapidr_value::layout::form_frame(style);
    (inside.0 + fw, inside.1 + fh)
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
/// face colour (the client area covers it).
pub fn paint(look: &Look, size: (i64, i64), scale: f64) -> DisplayList {
    let mut list = DisplayList { size, scale, ..Default::default() };
    let mut p = Painter::new(&mut list);
    paint_into(&mut p, look, size);
    list
}

/// [`paint`] with `p`: the frame with (0, 0) the window's top left.
pub fn paint_into(p: &mut Painter, look: &Look, size: (i64, i64)) {
    if !look.border {
        return;
    }
    let (w, h) = size;
    let t = p.theme();
    let (bar, ink) = if look.active { (t.caption, t.caption_text) } else { (t.inactive_caption, t.inactive_caption_text) };
    p.fill((0, 0, w, h), t.face);
    p.frame((0, 0, w, h), if t.fluent() { if look.active { t.caption } else { t.border } } else { t.dark_shadow });
    if !look.caption {
        // (HideTitleBar: the border alone)
        return;
    }
    let (bx, by, bw, bh) = (FORM_BORDER, FORM_BORDER, (w - 2 * FORM_BORDER).max(0), FORM_CAPTION);
    p.fill((bx, by, bw, bh), bar);
    let shown = buttons(look.frame);
    // (the icon, 16 × 16, then the title after it)
    let mut text_x = bx + 8;
    if let Some((picture, revision)) = &look.icon {
        p.picture("rapidr:frame-icon", *revision, picture.clone(), (bx + 6, by + (bh - 16) / 2, 16, 16));
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
        glyph(p, part, r, look.maximized, if on { glyph_ink } else { t.gray_text });
    }
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

/// The part of `look`'s frame (whole size `size`) at (x, y), logical.
pub fn hit(look: &Look, size: (i64, i64), x: f64, y: f64) -> Part {
    if !look.border || !look.caption {
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

/// A form's icon as its frame shows it (its IcoHandle / Icon, else the
/// application's), with a number that changes when its pixels do.
pub fn icon_of(store: &dyn crate::Store, form: &str) -> Option<(Arc<Picture>, u64)> {
    let own = ["icohandle", "icon"].into_iter().map(|prop| store.get(form, prop)).find(rapidr_value::objects::has_icon);
    let (w, h, rgba, _) = own.or_else(rapidr_value::globals::application_icon).and_then(|v| rapidr_value::objects::icon_pixels(&v))?;
    let revision = icon_revision(&rgba);
    Some((Arc::new(Picture { width: w, height: h, rgba }), revision))
}

/// An icon's pixels' number (FNV-1a), for the display list's picture cache.
pub fn icon_revision(rgba: &[u8]) -> u64 {
    rgba.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> Look {
        Look { title: "Form1".into(), active: true, border: true, caption: true, frame: Frame::default(), maximized: false, icon: None }
    }

    #[test]
    fn the_frame_is_every_runtimes() {
        assert_eq!(outer((318, 209), true, true), (320, 240));
        assert_eq!(inset(true, true), (1, 30));
        assert_eq!(outer((100, 50), false, true), (100, 50));
        // (HideTitleBar: the border alone, no title bar to press)
        assert_eq!(outer((318, 209), true, false), (320, 211));
        assert_eq!(inset(true, false), (1, 1));
        let bare = Look { caption: false, ..look() };
        assert_eq!(hit(&bare, (320, 211), 315.0, 15.0), Part::None);
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

    #[test]
    fn frames_for_border_style_and_icons() {
        assert_eq!(frame_of(2, BI_DEFAULT), Frame::default());
        assert_eq!(frame_of(3, BI_DEFAULT), Frame { resizable: false, close: true, minimize: false, maximize: false });
        assert_eq!(frame_of(2, 0), Frame { resizable: true, close: false, minimize: false, maximize: false });
    }
}
