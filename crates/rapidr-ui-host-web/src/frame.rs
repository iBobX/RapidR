//! A window's frame on the page (docs/web-host-plan.md §3.4): the
//! desktop's windows get theirs from the window system; the web host's
//! windows are canvases on the page, so their title bar, border and title
//! bar buttons are drawn here — by the kernel's own window frame
//! (`rapidr_ui_kernel::window_frame`, a QFORMMDI child's too), in the
//! kernel's theme: the active window's title bar in the caption colours,
//! the others in the inactive ones, the buttons the program's BorderStyle /
//! BorderIcons leave ([`Frame`]).
//!
//! The metrics are the ones every runtime accounts a form's frame with
//! (`rapidr_value::layout::form_frame`: a 1-pixel border and a 29-pixel
//! title bar) in every theme, so a form's Width / Height and ClientWidth /
//! ClientHeight are what they are on the desktop.

use std::sync::Arc;

use rapidr_ui_app::desktop::Frame;
use rapidr_ui_kernel::display::{DisplayList, Picture};
use rapidr_ui_kernel::paint::Painter;
use rapidr_ui_kernel::window_frame::{self, Button, Chrome, Metrics};
use rapidr_value::layout::{FORM_BORDER, FORM_CAPTION};
use rapidr_value::objects::font::Font;

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

/// The frame's metrics in the current theme.
fn metrics(look: &Look) -> Metrics {
    Metrics::window(rapidr_value::theme::current(), look.maximized)
}

/// The window's frame drawn, `size` its whole size (logical), for a screen
/// of `scale` device pixels per logical pixel. The inside is left as the
/// face colour (the client canvas covers it).
pub fn paint(look: &Look, size: (i64, i64), scale: f64) -> DisplayList {
    let mut list = DisplayList { size, scale, ..Default::default() };
    if !look.border {
        return list;
    }
    let mut p = Painter::new(&mut list);
    let t = p.theme();
    // (the classic look's title in Windows' caption font, MS Sans Serif
    // bold; RapidR's in the chrome font, semibold)
    let base = if t.fluent() { rapidr_value::ide_theme::chrome_font(t) } else { Font::default() };
    let icon = look.icon.as_ref().map(|icon| {
        let picture = Picture { width: icon.width as usize, height: icon.height as usize, rgba: icon.rgba.clone() };
        let revision = icon.rgba.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3));
        ("rapidr:frame-icon".to_string(), revision, Arc::new(picture))
    });
    let chrome = Chrome {
        title: look.title.clone(),
        active: look.active,
        maximized: look.maximized,
        buttons: buttons(look.frame),
        hot: None,
        pressed: None,
        icon,
        font: Font { styles: base.styles | 1, ..base },
    };
    window_frame::paint(&mut p, size, &chrome, &metrics(look));
    list
}

/// The host's styling of a window's elements for the frame's look:
/// RapidR's windows rounded as the theme's panels and lifted off the page
/// by a soft shadow (CSS's, as the desktop's window system gives its
/// windows theirs); the classic look's square and flat, as RapidQ's.
/// (`frame`'s radius, the inside's bottom corners', the shadow.)
pub fn css(look: &Look) -> (String, String, String) {
    let t = rapidr_value::theme::current();
    if !look.border || look.maximized || !t.fluent() || t.shadow_alpha == 0 {
        return ("0".into(), "0".into(), "none".into());
    }
    let r = metrics(look).radius;
    let (cr, cg, cb) = ((t.shadow_ink >> 16) & 0xFF, (t.shadow_ink >> 8) & 0xFF, t.shadow_ink & 0xFF);
    let a = f64::from(t.shadow_alpha) / 255.0;
    let (near, far) = if look.active { (a * 0.6, a) } else { (a * 0.4, a * 0.55) };
    (
        format!("{r}px"),
        format!("0 0 {inner}px {inner}px", inner = (r - 1.0).max(0.0)),
        format!("0 1px 3px rgba({cr},{cg},{cb},{near:.3}), 0 10px 32px rgba({cr},{cg},{cb},{far:.3})"),
    )
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
    match window_frame::hit(&metrics(look), size.0, &buttons(look.frame), x, y) {
        window_frame::Part::Button(Button::Close) => Part::Close,
        window_frame::Part::Button(Button::Maximize) => Part::Maximize,
        window_frame::Part::Button(Button::Minimize) => Part::Minimize,
        window_frame::Part::Title => Part::Title,
        window_frame::Part::None => Part::None,
    }
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
        assert_ne!(css(&l).2, "none");
        // the classic look: Windows' 16 × 14 buttons, the same frame
        rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
        assert_eq!(hit(&l, (320, 240), 310.0, 15.0), Part::Close);
        assert_eq!(hit(&l, (320, 240), 310.0, 4.0), Part::Title);
        assert_eq!(hit(&l, (320, 240), 290.0, 15.0), Part::Maximize);
        assert_eq!(hit(&l, (320, 240), 274.0, 15.0), Part::Minimize);
        assert_eq!(hit(&l, (320, 240), 250.0, 15.0), Part::Title);
        assert_eq!(css(&l), ("0".into(), "0".into(), "none".into()));
    }
}
