//! A program's icon: RapidR's own for compiled programs (the brand's
//! "program" icon, `design/brand/icons`), or one the user gives as `.icns`,
//! `.ico`, `.png` (ideally 1024 × 1024) or `.svg` — drawn at every size each
//! system wants.

use std::path::Path;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

use crate::{icns, ico, picture};

/// Which drawing of an icon a system shows: macOS' (Apple's grid: the tile
/// inset in the canvas, with its shadow) or everyone else's (the tile fills
/// the canvas).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Mac,
    Full,
}

// The default icon's SVG masters (design/brand/icons/svg, made by
// design/brand/src/icons.py): large ones and hand-hinted small ones.
const MAC_MASTER: &str = include_str!("../../../design/brand/icons/svg/app-program-macos.svg");
const FULL_MASTER: &str = include_str!("../../../design/brand/icons/svg/app-program-full.svg");
const MAC_32: &str = include_str!("../../../design/brand/icons/svg/app-program-mac32.svg");
const HINTED: [(u32, &str); 5] = [
    (16, include_str!("../../../design/brand/icons/svg/app-program-16.svg")),
    (20, include_str!("../../../design/brand/icons/svg/app-program-20.svg")),
    (22, include_str!("../../../design/brand/icons/svg/app-program-22.svg")),
    (24, include_str!("../../../design/brand/icons/svg/app-program-24.svg")),
    (32, include_str!("../../../design/brand/icons/svg/app-program-32.svg")),
];

enum Source {
    /// RapidR's own.
    Default,
    Svg(Box<usvg::Tree>, String),
    /// The pictures of a PNG, ICO or ICNS, largest first.
    Pictures(Vec<Pixmap>),
}

/// An icon, ready to be drawn at any size.
pub struct Icon {
    source: Source,
    /// Where it came from: a path, or "RapidR's default icon".
    pub origin: String,
}

impl std::fmt::Debug for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Icon({})", self.origin)
    }
}

/// The formats an icon file can be in, for messages.
pub const FORMATS: &str = ".icns, .ico, .png (ideally 1024 × 1024) or .svg";

fn parse_svg(text: &[u8]) -> Result<usvg::Tree, String> {
    usvg::Tree::from_data(text, &usvg::Options::default()).map_err(|e| format!("not an SVG RapidR can read ({e})"))
}

fn is_svg(b: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&b[..b.len().min(1024)]).to_ascii_lowercase();
    let t = head.trim_start_matches('\u{feff}').trim_start();
    t.starts_with("<svg") || ((t.starts_with("<?xml") || t.starts_with("<!--") || t.starts_with("<!doctype")) && head.contains("<svg"))
}

impl Icon {
    /// RapidR's icon for compiled programs.
    pub fn rapidr_default() -> Icon {
        Icon { source: Source::Default, origin: "RapidR's default icon".into() }
    }

    /// An icon file: `.icns`, `.ico`, `.png` or `.svg` (told by its content).
    pub fn load(path: &Path) -> Result<Icon, String> {
        let shown = path.display();
        let bytes = std::fs::read(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => format!("{shown}: the icon file isn't there"),
            _ => format!("{shown}: can't read the icon ({e})"),
        })?;
        Icon::from_bytes(&bytes, &shown.to_string())
    }

    /// An icon from a file's bytes; `origin` names it in messages.
    pub fn from_bytes(bytes: &[u8], origin: &str) -> Result<Icon, String> {
        let bad = |why: String| format!("{origin}: {why}. An icon is a {FORMATS} file");
        let source = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Source::Pictures(vec![picture::decode_png(bytes).map_err(bad)?])
        } else if icns::is_icns(bytes) {
            Source::Pictures(icns::read(bytes).map_err(bad)?)
        } else if ico::is_ico(bytes) {
            Source::Pictures(ico::read(bytes).map_err(bad)?)
        } else if is_svg(bytes) {
            let tree = parse_svg(bytes).map_err(bad)?;
            if tree.size().width() <= 0.0 || tree.size().height() <= 0.0 {
                return Err(bad("the SVG has no size".into()));
            }
            Source::Svg(Box::new(tree), String::from_utf8_lossy(bytes).into_owned())
        } else {
            return Err(bad("not a picture RapidR can make an icon of".into()));
        };
        let source = match source {
            Source::Pictures(mut pics) => {
                pics.retain(|p| p.width() > 0 && p.height() > 0);
                pics.sort_by_key(|p| std::cmp::Reverse(p.width().max(p.height())));
                Source::Pictures(pics)
            }
            s => s,
        };
        Ok(Icon { source, origin: origin.to_string() })
    }

    /// Whether it's RapidR's own.
    pub fn is_default(&self) -> bool {
        matches!(self.source, Source::Default)
    }

    /// Advice about the icon: a small picture scaled up looks blurry.
    pub fn notes(&self) -> Vec<String> {
        match &self.source {
            Source::Pictures(p) => {
                let side = p.first().map_or(0, |p| p.width().max(p.height()));
                if side < 512 {
                    vec![format!(
                        "{}: its largest picture is {side} × {side}: larger sizes are scaled up and look blurry (a 1024 × 1024 PNG or an SVG is sharp everywhere)",
                        self.origin
                    )]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    /// The icon at `px` × `px` for `look`.
    pub fn picture(&self, px: u32, look: Look) -> Pixmap {
        match &self.source {
            Source::Default => {
                let svg = match (look, px) {
                    (Look::Mac, 32) => MAC_32,
                    _ => match HINTED.iter().find(|(s, _)| *s == px) {
                        Some((_, svg)) => svg,
                        None if look == Look::Mac => MAC_MASTER,
                        None => FULL_MASTER,
                    },
                };
                // (RapidR's own masters: they parse)
                draw_svg(&parse_svg(svg.as_bytes()).expect("the default icon's SVG"), px)
            }
            Source::Svg(tree, _) => draw_svg(tree, px),
            Source::Pictures(pics) => {
                // the smallest picture as large as `px`, else the largest
                let best = pics.iter().rev().find(|p| p.width().max(p.height()) >= px).unwrap_or(&pics[0]);
                picture::resize(&picture::squared(best), px, px)
            }
        }
    }

    /// The `.icns` (macOS).
    pub fn icns(&self) -> Vec<u8> {
        icns::write(|px| self.picture(px, Look::Mac))
    }

    /// The `.ico` (Windows).
    pub fn ico(&self) -> Vec<u8> {
        let pics: Vec<Pixmap> = ico::SIZES.iter().map(|&px| self.picture(px, Look::Full)).collect();
        ico::write(&pics)
    }

    /// A PNG at `px` (Linux).
    pub fn png(&self, px: u32) -> Vec<u8> {
        picture::encode_png(&self.picture(px, Look::Full))
    }

    /// The SVG itself, when the icon is one (Linux' scalable icon).
    pub fn svg_text(&self) -> Option<String> {
        match &self.source {
            Source::Default => Some(FULL_MASTER.to_string()),
            Source::Svg(_, text) => Some(text.clone()),
            Source::Pictures(_) => None,
        }
    }
}

/// An SVG drawn into a `px` square, its aspect kept, centred.
fn draw_svg(tree: &usvg::Tree, px: u32) -> Pixmap {
    let mut p = picture::blank(px, px);
    let size = tree.size();
    let scale = (px as f32 / size.width()).min(px as f32 / size.height());
    let (dx, dy) = ((px as f32 - size.width() * scale) / 2.0, (px as f32 - size.height() * scale) / 2.0);
    resvg::render(tree, Transform::from_row(scale, 0.0, 0.0, scale, dx, dy), &mut p.as_mut());
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opaque_share(p: &Pixmap) -> f32 {
        p.pixels().iter().filter(|c| c.alpha() > 200).count() as f32 / p.pixels().len() as f32
    }

    #[test]
    fn the_default_icon_draws_at_every_size() {
        let icon = Icon::rapidr_default();
        assert!(icon.is_default() && icon.notes().is_empty());
        for px in [16, 20, 22, 24, 32, 48, 64, 128, 256, 512, 1024] {
            let mac = icon.picture(px, Look::Mac);
            let full = icon.picture(px, Look::Full);
            assert_eq!((mac.width(), full.height()), (px, px));
            // the tile: most of the canvas elsewhere, less within Apple's margin
            assert!(opaque_share(&full) > 0.7, "{px}: {}", opaque_share(&full));
            if px >= 64 {
                assert!(opaque_share(&mac) < opaque_share(&full), "{px}");
            }
        }
        // the corners are see-through, the middle isn't
        let p = picture::to_rgba(&icon.picture(256, Look::Full));
        assert_eq!(p[3], 0);
        assert_eq!(p[(128 * 256 + 128) * 4 + 3], 255);
    }

    #[test]
    fn loads_each_format() {
        let dir = std::env::temp_dir().join(format!("rapidr-package-icon-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = picture::encode_png(&picture::from_rgba(1024, 1024, &[0, 128, 255, 255].repeat(1024 * 1024)));
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="#f00"/></svg>"##;
        let files = [
            ("a.png", png.clone()),
            ("a.svg", svg.as_bytes().to_vec()),
            ("a.ico", Icon::rapidr_default().ico()),
            ("a.icns", Icon::rapidr_default().icns()),
            ("rq.ico", crate::ico::tests::rapidq_icon()),
        ];
        for (name, bytes) in &files {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            let icon = Icon::load(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
            for px in [16, 32, 256, 1024] {
                let p = icon.picture(px, Look::Full);
                assert_eq!((p.width(), p.height()), (px, px), "{name}");
            }
            assert!(icns::read(&icon.icns()).is_ok() && ico::read(&icon.ico()).is_ok(), "{name}");
        }
        // a wide SVG is centred: see-through above and below
        let wide = Icon::load(&dir.join("a.svg")).unwrap().picture(100, Look::Full);
        let rgba = picture::to_rgba(&wide);
        assert_eq!(rgba[3], 0);
        assert_eq!(&rgba[(50 * 100 + 50) * 4..][..4], &[255, 0, 0, 255]);
        // RapidQ's 32 px icon is used, with a note that it's small
        let rq = Icon::load(&dir.join("rq.ico")).unwrap();
        assert_eq!(rq.notes().len(), 1);
        assert!(Icon::load(&dir.join("a.png")).unwrap().notes().is_empty());

        // clear errors
        let missing = Icon::load(&dir.join("nowhere.ico")).unwrap_err();
        assert!(missing.contains("isn't there"), "{missing}");
        std::fs::write(dir.join("bad.png"), b"\x89PNG\r\n\x1a\nbroken").unwrap();
        let broken = Icon::load(&dir.join("bad.png")).unwrap_err();
        assert!(broken.contains("not a PNG") && broken.contains(".icns, .ico"), "{broken}");
        std::fs::write(dir.join("x.txt"), b"hello").unwrap();
        assert!(Icon::load(&dir.join("x.txt")).unwrap_err().contains("not a picture"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
