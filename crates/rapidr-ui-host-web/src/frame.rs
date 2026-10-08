//! A window's frame on the page (docs/web-host-plan.md §3.4): the
//! desktop's windows get theirs from the window system; the web host's
//! windows are canvases on the page, so their title bar, border and title
//! bar buttons are the kernel's ([`rapidr_ui_kernel::frame`], the one the
//! form designer draws too, drawn by `rapidr_ui_kernel::window_frame` as a
//! QFORMMDI child's), in the kernel's theme. In RapidR's look the window is
//! also rounded and lifted off the page by a shadow ([`css`]).

use rapidr_ui_app::desktop::Frame;
use rapidr_ui_kernel::display::{DisplayList, Picture};
use rapidr_ui_kernel::frame as kframe;
pub use rapidr_ui_kernel::frame::{inset, outer, Part};

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

impl Look {
    /// The kernel's look.
    fn kernel(&self) -> kframe::Look {
        let icon = self.icon.as_ref().map(|i| (std::sync::Arc::new(Picture { width: i.width as usize, height: i.height as usize, rgba: i.rgba.clone() }), kframe::icon_revision(&i.rgba)));
        kframe::Look { title: self.title.clone(), active: self.active, border: self.border, frame: self.frame, maximized: self.maximized, icon }
    }
}

/// The window's frame drawn, `size` its whole size (logical), for a screen
/// of `scale` device pixels per logical pixel.
pub fn paint(look: &Look, size: (i64, i64), scale: f64) -> DisplayList {
    kframe::paint(&look.kernel(), size, scale)
}

/// The part of `look`'s frame (whole size `size`) at (x, y), logical.
pub fn hit(look: &Look, size: (i64, i64), x: f64, y: f64) -> Part {
    kframe::hit(&look.kernel(), size, x, y)
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
    let r = t.panel_radius();
    let (cr, cg, cb) = ((t.shadow_ink >> 16) & 0xFF, (t.shadow_ink >> 8) & 0xFF, t.shadow_ink & 0xFF);
    let a = f64::from(t.shadow_alpha) / 255.0;
    let (near, far) = if look.active { (a * 0.6, a) } else { (a * 0.4, a * 0.55) };
    (
        format!("{r}px"),
        format!("0 0 {inner}px {inner}px", inner = (r - 1.0).max(0.0)),
        format!("0 1px 3px rgba({cr},{cg},{cb},{near:.3}), 0 10px 32px rgba({cr},{cg},{cb},{far:.3})"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_and_lifted_in_rapidrs_look_flat_in_the_classic_one() {
        let l = Look { title: "Form1".into(), active: true, border: true, frame: Frame::default(), maximized: false, theme: 0, icon: None };
        rapidr_value::theme::set(&rapidr_value::theme::RAPIDR);
        assert_ne!(css(&l).2, "none");
        assert_eq!(css(&Look { maximized: true, ..l.clone() }).2, "none");
        rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
        assert_eq!(css(&l), ("0".into(), "0".into(), "none".into()));
    }
}
