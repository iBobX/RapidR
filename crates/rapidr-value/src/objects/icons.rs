//! RapidR's own icons in programs (an addition: RapidQ has none of this):
//!
//! - `Bitmap.LoadIcon(Name$ [, Size [, Theme$]])` — QBITMAP: the named icon
//!   (crates/rapidr-icons: `"run"`, `"actions/save"`, a component type such
//!   as `"QBUTTON"`, a command id such as `"file.open"`), `Size` logical
//!   pixels square (16 if left out), in the current theme's colours (or
//!   Theme$'s: classic, modern, dark, highcontrast).
//! - `ImageList.AddIcon(Name$ [, Theme$])` — QIMAGELIST: the icon at the
//!   list's Width × Height, added at the end.
//!
//! The bitmap keeps the icon as an SVG, so the screen shows it drawn again
//! at its scale (bitmap.rs: an SVG's high-DPI pixels), and the drawing
//! picked is the one hinted for the device size at that moment
//! (`rapidr_icons::variant_for`): crisp at 1×, 1.5× and 2×. An unknown name
//! is an error the program can see, as a missing file is.

#[cfg(feature = "icons")]
use super::bitmap::{display_scale, Bitmap};
use super::{with, Object};
use crate::Value;

/// `Bitmap.LoadIcon` and `ImageList.AddIcon` in a build without the icon
/// set (rapidr-value's feature `icons` off).
#[cfg(not(feature = "icons"))]
pub fn call(id: &str, method: &str, _args: &[Value]) -> Option<Result<Value, String>> {
    let ours = match method {
        "loadicon" => with(id, |o| matches!(o, Object::Bitmap(_)))?,
        "addicon" => with(id, |o| matches!(o, Object::ImageList(_)))?,
        _ => false,
    };
    ours.then(|| Err("this build of RapidR has no icons (rapidr-value's feature `icons`)".to_string()))
}

/// `Bitmap.LoadIcon` and `ImageList.AddIcon` (`None`: not one of these).
#[cfg(feature = "icons")]
pub fn call(id: &str, method: &str, args: &[Value]) -> Option<Result<Value, String>> {
    let text = |i: usize| args.get(i).map(Value::to_string_val);
    match method {
        "loadicon" => {
            if !with(id, |o| matches!(o, Object::Bitmap(_)))? {
                return None;
            }
            let size = args.get(1).map_or(16, Value::to_i64);
            Some(icon_svg(&text(0).unwrap_or_default(), size, text(2).as_deref()).and_then(|svg| {
                with(id, |o| match o {
                    Object::Bitmap(b) => b.load_bmp_bytes(svg.as_bytes()),
                    _ => Ok(()),
                })
                .unwrap_or(Ok(()))
                .map(|_| Value::Null)
            }))
        }
        "addicon" => {
            let size = with(id, |o| match o {
                Object::ImageList(l) => Some(l.width.min(l.height)),
                _ => None,
            })??;
            Some(icon_bitmap(&text(0).unwrap_or_default(), size, text(1).as_deref()).map(|b| {
                with(id, |o| {
                    if let Object::ImageList(l) = o {
                        let at = l.images.len();
                        l.insert_icon(at, &b);
                    }
                });
                Value::Null
            }))
        }
        _ => None,
    }
}

/// Icon `name` as an SVG `size` × `size` big, for the display scale, in
/// `theme`'s colours (the current theme's when `None`).
#[cfg(feature = "icons")]
pub fn icon_svg(name: &str, size: i64, theme: Option<&str>) -> Result<String, String> {
    let icon = rapidr_icons::get(name).ok_or_else(|| format!("no icon named {name}"))?;
    let size = size.clamp(1, 1024) as u32;
    // (a theme's name — the current one's, or one named — as the icons'
    // palettes are named: classic, modern, dark, highcontrast)
    let named = match theme.filter(|t| !t.trim().is_empty()) {
        Some(n) => match crate::theme::choose(n) {
            crate::theme::Choice::Theme(t) => t,
            _ => crate::theme::current(),
        },
        None => crate::theme::current(),
    };
    let theme = named.icon_palette();
    let device = size * display_scale().max(1) as u32;
    let (variant, _) = rapidr_icons::variant_for(device);
    let svg = rapidr_icons::themed_svg(icon, variant, &rapidr_icons::Style::new(&theme));
    // (the hinted drawing, shown `size` logical pixels big)
    Ok(svg.replacen(&format!("width=\"{variant}\" height=\"{variant}\""), &format!("width=\"{size}\" height=\"{size}\""), 1))
}

/// A bitmap holding icon `name` (`icon_svg`).
#[cfg(feature = "icons")]
pub fn icon_bitmap(name: &str, size: i64, theme: Option<&str>) -> Result<Bitmap, String> {
    let mut b = Bitmap::default();
    b.load_bmp_bytes(icon_svg(name, size, theme)?.as_bytes())?;
    Ok(b)
}

#[cfg(all(test, feature = "icons"))]
mod tests {
    use super::*;

    #[test]
    fn icons_as_bitmaps() {
        let b = icon_bitmap("run", 16, Some("modern")).unwrap();
        assert_eq!((b.img.width, b.img.height), (16, 16));
        assert!(b.is_svg());
        // (the run triangle is teal; its centre is opaque)
        let alpha = b.alpha.as_ref().unwrap();
        assert!(alpha[8 * 16 + 7] > 200, "{alpha:?}");
        let big = icon_bitmap("QBUTTON", 32, None).unwrap();
        assert_eq!(big.img.width, 32);
        assert!(icon_bitmap("file.open", 24, Some("dark")).is_ok());
        assert!(icon_bitmap("no-such-icon", 16, None).unwrap_err().contains("no-such-icon"));
    }
}
