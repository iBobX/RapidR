//! A window's frame on the page (docs/web-host-plan.md §3.4): the
//! desktop's windows get theirs from the window system; the web host's
//! windows are canvases on the page, so their title bar, border and title
//! bar buttons are the kernel's ([`rapidr_ui_kernel::frame`], the one the
//! form designer draws too), in the kernel's theme.

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
