//! RapidR's own icons drawn by the kernel (crates/rapidr-icons; the design
//! system: design/icons/README.md): [`Painter::icon`] puts an icon into
//! the display list as a picture made for the list's scale — the hinted
//! drawing for that device size, in the current theme's colours — so it
//! is crisp at 1×, 1.5× and 2× on the desktop and the web alike (both draw
//! the same lists). Pictures are kept per icon, size, scale, theme and
//! state, so a toolbar redrawn every frame renders each icon once.
//!
//! An icon never stands in for a name: a component drawing one still
//! describes itself by its caption or hint to screen readers (a11y).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use rapidr_value::objects::ops::Rect;

use crate::display::Picture;
use crate::paint::Painter;

/// Pictures kept (the oldest set is dropped past this many).
const KEEP: usize = 1024;

thread_local! {
    static PICTURES: RefCell<HashMap<String, Arc<Picture>>> = RefCell::new(HashMap::new());
}

/// Icon `name` (rapidr_icons::get: `run`, `actions/save`, `QBUTTON`,
/// `file.open` …) as a picture `logical` pixels square at `scale`, in
/// `theme`'s colours, its `currentColor` `color` (the theme's icon ink when
/// `None`), dimmed when `disabled`: the picture's key and pixels.
pub fn picture(name: &str, logical: u32, scale: f64, theme: &str, color: Option<u32>, disabled: bool) -> Option<(String, Arc<Picture>)> {
    let icon = rapidr_icons::get(name)?;
    let key = format!("icon:{}@{logical}x{scale}:{theme}:{}:{}", icon.id, color.map_or(-1, i64::from), u8::from(disabled));
    if let Some(p) = PICTURES.with(|c| c.borrow().get(&key).cloned()) {
        return Some((key, p));
    }
    let style = rapidr_icons::Style { palette: rapidr_icons::palette(theme), color, disabled };
    let px = rapidr_icons::render(icon, logical, scale as f32, &style)?;
    let pic = Arc::new(Picture { width: px.width as usize, height: px.height as usize, rgba: px.data });
    PICTURES.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= KEEP {
            c.clear();
        }
        c.insert(key.clone(), pic.clone());
    });
    Some((key, pic))
}

impl Painter<'_> {
    /// Draws icon `name` centred in `rect` (logical pixels), as big as the
    /// rectangle's shorter side, in the current theme; `color` is its
    /// `currentColor` (`None`: the theme's icon ink), `disabled` dims it.
    /// False when there is no such icon.
    pub fn icon(&mut self, name: &str, rect: Rect, color: Option<u32>, disabled: bool) -> bool {
        let (x, y, w, h) = rect;
        let side = w.min(h);
        if side <= 0 {
            return false;
        }
        let Some((key, pic)) = picture(name, side as u32, self.scale(), self.theme().name, color, disabled) else { return false };
        self.picture(&key, 0, pic, (x + (w - side) / 2, y + (h - side) / 2, side, side));
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::display::{DisplayList, Item};
    use crate::paint::Painter;
    use rapidr_value::objects::ops::Op;

    /// An icon becomes a picture made for the list's scale (the hinted
    /// drawing at its device size), centred in its rectangle.
    #[test]
    fn icons_are_pictures_at_the_device_size() {
        for (scale, device) in [(1.0, 16), (1.5, 24), (2.0, 32)] {
            let mut list = DisplayList { size: (100, 40), scale, ..Default::default() };
            let mut p = Painter::new(&mut list);
            assert!(p.icon("run", (10, 4, 24, 16), None, false));
            assert!(!p.icon("no-such-icon", (0, 0, 16, 16), None, false));
            let Item::Op { op: Op::Image { source, rect, .. }, .. } = &list.items[0] else { panic!("{:?}", list.items) };
            assert_eq!(*rect, (14, 4, 16, 16));
            let pic = &list.images[source];
            assert_eq!((pic.width, pic.height), (device, device), "at {scale}x");
            assert!(pic.rgba.chunks(4).any(|c| c[3] == 255));
        }
    }

    /// The same icon in the same state is one picture, made once.
    #[test]
    fn pictures_are_kept() {
        let a = super::picture("save", 16, 2.0, "dark", None, false).unwrap();
        let b = super::picture("actions/save", 16, 2.0, "dark", None, false).unwrap();
        assert!(std::sync::Arc::ptr_eq(&a.1, &b.1));
        let off = super::picture("save", 16, 2.0, "dark", None, true).unwrap();
        assert_ne!(a.0, off.0);
    }
}
