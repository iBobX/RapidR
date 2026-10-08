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

use crate::display::{DisplayList, Picture};
use crate::paint::Painter;
use crate::window_frame::{self, Button, Chrome, Metrics};

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

/// The title bar buttons shown, from the right — close, then maximize and
/// minimize (both shown when either is: the other greyed, as Windows draws
/// them); none without the system menu — and whether each is enabled.
fn buttons(f: Frame) -> Vec<(Button, bool)> {
    let mut b = Vec::new();
    if f.close {
        b.push((Button::Close, true));
        if f.minimize || f.maximize {
            b.push((Button::Maximize, f.maximize));
            b.push((Button::Minimize, f.minimize));
        }
    }
    b
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

/// [`paint`] with `p`: the frame with (0, 0) the window's top left — the
/// kernel's window frame (`window_frame`, a QFORMMDI child's too) in the
/// current theme: RapidR's title bar and thin glyph buttons, or Windows'
/// classic one with its 16 × 14 buttons, in the same 1-pixel border and
/// 29-pixel title bar.
pub fn paint_into(p: &mut Painter, look: &Look, size: (i64, i64)) {
    if !look.border {
        return;
    }
    let t = p.theme();
    if !look.caption {
        // (HideTitleBar: the border alone)
        let (w, h) = size;
        p.fill((0, 0, w, h), t.face);
        p.frame((0, 0, w, h), if t.fluent() { if look.active { t.caption } else { t.border } } else { t.dark_shadow });
        return;
    }
    // (the classic look's title in Windows' caption font, MS Sans Serif
    // bold; RapidR's in the chrome font, semibold)
    let base = if t.fluent() { rapidr_value::ide_theme::chrome_font(t) } else { Font::default() };
    let chrome = Chrome {
        title: look.title.clone(),
        active: look.active,
        maximized: look.maximized,
        buttons: buttons(look.frame),
        hot: None,
        pressed: None,
        icon: look.icon.as_ref().map(|(picture, revision)| ("rapidr:frame-icon".to_string(), *revision, picture.clone())),
        font: Font { styles: base.styles | 1, ..base },
    };
    window_frame::paint(p, size, &chrome, &Metrics::window(t, look.maximized));
}

/// The part of `look`'s frame (whole size `size`) at (x, y), logical; a
/// greyed button is the title bar's.
pub fn hit(look: &Look, size: (i64, i64), x: f64, y: f64) -> Part {
    if !look.border || !look.caption {
        return Part::None;
    }
    let m = Metrics::window(rapidr_value::theme::current(), look.maximized);
    match window_frame::hit(&m, size.0, &buttons(look.frame), x, y) {
        window_frame::Part::Button(Button::Close) => Part::Close,
        window_frame::Part::Button(Button::Maximize) => Part::Maximize,
        window_frame::Part::Button(Button::Minimize) => Part::Minimize,
        window_frame::Part::Title => Part::Title,
        window_frame::Part::None => Part::None,
    }
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
        // RapidR's look: buttons 40 wide across the title bar
        rapidr_value::theme::set(&rapidr_value::theme::RAPIDR);
        let l = look();
        assert_eq!(hit(&l, (320, 240), 315.0, 15.0), Part::Close);
        assert_eq!(hit(&l, (320, 240), 270.0, 15.0), Part::Maximize);
        assert_eq!(hit(&l, (320, 240), 230.0, 15.0), Part::Minimize);
        assert_eq!(hit(&l, (320, 240), 40.0, 15.0), Part::Title);
        assert_eq!(hit(&l, (320, 240), 40.0, 100.0), Part::None);
        // (a dialog's frame: a close box only)
        let d = Look { frame: Frame { resizable: false, close: true, minimize: false, maximize: false }, ..look() };
        assert_eq!(hit(&d, (320, 240), 270.0, 15.0), Part::Title);
        let list = paint(&l, (320, 240), 1.0);
        assert!(!list.items.is_empty());
        // the classic look: Windows' 16 × 14 buttons in the same frame
        rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
        assert_eq!(hit(&l, (320, 240), 310.0, 15.0), Part::Close);
        assert_eq!(hit(&l, (320, 240), 310.0, 4.0), Part::Title);
        assert_eq!(hit(&l, (320, 240), 290.0, 15.0), Part::Maximize);
        assert_eq!(hit(&l, (320, 240), 274.0, 15.0), Part::Minimize);
        assert_eq!(hit(&l, (320, 240), 250.0, 15.0), Part::Title);
    }

    #[test]
    fn frames_for_border_style_and_icons() {
        assert_eq!(frame_of(2, BI_DEFAULT), Frame::default());
        assert_eq!(frame_of(3, BI_DEFAULT), Frame { resizable: false, close: true, minimize: false, maximize: false });
        assert_eq!(frame_of(2, 0), Frame { resizable: true, close: false, minimize: false, maximize: false });
    }
}
