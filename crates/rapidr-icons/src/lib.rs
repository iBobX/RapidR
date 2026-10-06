//! RapidR's own icon set (docs/ide-plan.md, decision D8; the design system
//! is design/icons/README.md): every action, component, file type, symbol,
//! toolbox group and glyph of RapidR Studio, the manual and the website.
//!
//! - **Sources**: design/icons/src (SVG, drawn by design/icons/tools), each
//!   icon hinted at 16, 24 and 32 px. `generated.rs` holds them optimized,
//!   with the palettes and the inventory (design/icons/tools/build.py).
//! - **Colours are tokens**: an SVG spells each token as its light-theme
//!   value; [`themed_svg`] swaps in a theme's ([`Palette`]). Monochrome icons
//!   use `currentColor` only ([`Style::color`]).
//! - **Pixels**: [`render`] draws an icon `logical` pixels big at a display
//!   scale (1, 1.5, 2 …) through resvg, picking the hinted drawing that
//!   lands on the device's pixel grid ([`variant_for`]).
//! - **Names**: an icon's id is `category/name` (`actions/run`,
//!   `components/button`); [`get`] also takes a bare name, a component type
//!   (`QBUTTON`, `RBUTTON`) or a command id. Icons never replace names:
//!   whoever shows one also shows (or speaks) the thing's name.

mod generated;

pub use generated::{COMMANDS, COMPONENTS, FILES, MARKERS, PROJECT_KINDS, SYMBOLS, TOOLBOX_GROUPS};

/// The hinted sizes every icon is drawn at.
pub const SIZES: [u32; 3] = [16, 24, 32];

/// Category order a bare name is looked up in ([`get`]).
pub const CATEGORIES: [&str; 6] = ["actions", "glyphs", "components", "files", "symbols", "groups"];

/// One icon: its id (`category/name`), its title (what it means, in words:
/// a tooltip or an accessible name when nothing else names it), and its
/// three hinted drawings.
#[derive(Debug)]
pub struct IconData {
    pub id: &'static str,
    pub category: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    /// Drawn in `currentColor` only (the actions and glyphs, mostly).
    pub mono: bool,
    pub(crate) svg: [&'static str; 3],
}

pub type Icon = IconData;

impl IconData {
    /// The SVG source of the drawing hinted for `variant` px (16, 24 or 32;
    /// other sizes get the nearest), colours as tokens.
    pub fn svg(&self, variant: u32) -> &'static str {
        match variant {
            0..=19 => self.svg[0],
            20..=27 => self.svg[1],
            _ => self.svg[2],
        }
    }
}

/// A toolbox group (RToolbox): its components, under "RapidQ" or "RapidR".
#[derive(Debug)]
pub struct ToolboxGroup {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    /// The top group it sits under ("" for the top groups).
    pub parent: &'static str,
    pub members: &'static [&'static str],
}

/// Every icon, sorted by id.
pub fn all() -> &'static [Icon] {
    generated::ICONS
}

/// The icon `name` names: an id (`actions/run`), a bare name looked up in
/// [`CATEGORIES`]' order (`run`, `button`), a component type (`QBUTTON`,
/// `RBUTTON`, `QGAUGE`) or a command id (`file.save`); any case.
pub fn get(name: &str) -> Option<&'static Icon> {
    let lower = name.trim().to_ascii_lowercase();
    if lower.contains('/') {
        return by_id(&lower);
    }
    if let Some(i) = CATEGORIES.iter().find_map(|c| by_id(&format!("{c}/{lower}"))) {
        return Some(i);
    }
    component(name).or_else(|| command(name))
}

fn by_id(id: &str) -> Option<&'static Icon> {
    generated::ICONS.binary_search_by(|i| i.id.cmp(id)).ok().map(|i| &generated::ICONS[i])
}

/// RapidR's name for a component type (rapidr_ast::canonical_type_name's
/// rules): `QBUTTON` → `RBUTTON`, `QGAUGE` → `RPROGRESSBAR`, `QOUTLINE` →
/// `RTREEVIEW`, `COMPORT` → `RCOMPORT`.
pub fn canonical_component(type_name: &str) -> String {
    let upper = type_name.trim().to_ascii_uppercase();
    match upper.as_str() {
        "QGAUGE" => return "RPROGRESSBAR".into(),
        "QOUTLINE" => return "RTREEVIEW".into(),
        "COMPORT" => return "RCOMPORT".into(),
        _ => {}
    }
    match upper.strip_prefix('Q') {
        Some(rest) if COMPONENTS.iter().any(|(t, _, _)| t[1..] == *rest) => format!("R{rest}"),
        _ => upper,
    }
}

/// A component type's icon (RapidQ and RapidR names alike; the planned
/// components' too). When the language registry (rapidr-lang) lands, its
/// components' `icon` field is this table.
pub fn component(type_name: &str) -> Option<&'static Icon> {
    let t = canonical_component(type_name);
    COMPONENTS.iter().find(|(n, _, _)| *n == t).and_then(|(_, id, _)| by_id(id))
}

/// A command's icon (inventory.toml's [commands]: `file.save`, `run.start` …).
pub fn command(id: &str) -> Option<&'static Icon> {
    lookup(COMMANDS, id)
}

/// A gutter or problems-list marker's icon (`error`, `breakpoint` …).
pub fn marker(kind: &str) -> Option<&'static Icon> {
    lookup(MARKERS, kind)
}

/// A symbol kind's icon (`sub`, `function`, `variable` …).
pub fn symbol(kind: &str) -> Option<&'static Icon> {
    lookup(SYMBOLS, kind)
}

/// A project-tree node kind's icon (`module`, `form`, `include` …).
pub fn project_kind(kind: &str) -> Option<&'static Icon> {
    lookup(PROJECT_KINDS, kind)
}

/// A file's icon, by its extension (`Main.rr`, `.bas`, `csv`); any other
/// file gets `files/file`.
pub fn file(path: &str) -> &'static Icon {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let ext = name.rsplit_once('.').map_or(name, |(_, e)| e).to_ascii_lowercase();
    lookup(FILES, &ext).or_else(|| lookup(FILES, "*")).expect("files/file exists")
}

fn lookup(table: &[(&str, &'static str)], key: &str) -> Option<&'static Icon> {
    table.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).and_then(|(_, id)| by_id(id))
}

// ---- colours ----------------------------------------------------------------------

/// A theme's icon colours: a value per token, `currentColor`'s default and a
/// disabled icon's ink. The themes are rapidr_value::theme's: `classic`,
/// `modern`, `dark`, `highcontrast`.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub theme: &'static str,
    /// What `currentColor` is unless the caller says (monochrome icons' ink).
    pub fg: u32,
    /// A disabled icon's outlines and solid shapes.
    pub disabled: u32,
    /// The backgrounds this theme's icons are checked against (contrast).
    pub backgrounds: &'static [u32],
    values: &'static [u32],
}

impl Palette {
    /// A token's value (0xRRGGBB): `ink`, `paper`, `blue`, `blue-tint` …
    pub fn token(&self, name: &str) -> Option<u32> {
        generated::TOKENS.iter().position(|t| *t == name).map(|i| self.values[i])
    }

    /// Every token and its value.
    pub fn tokens(&self) -> impl Iterator<Item = (&'static str, u32)> + '_ {
        generated::TOKENS.iter().copied().zip(self.values.iter().copied())
    }
}

/// The palette of theme `name` (rapidr_value::theme's names; an unknown one
/// gets `modern`'s).
pub fn palette(name: &str) -> Palette {
    let p = generated::PALETTES.iter().find(|p| p.0.eq_ignore_ascii_case(name)).unwrap_or(&generated::PALETTES[1]);
    Palette { theme: p.0, fg: p.1, disabled: p.2, backgrounds: p.3, values: &p.4 }
}

/// Every palette, in the themes' order.
pub fn palettes() -> impl Iterator<Item = Palette> {
    generated::PALETTES.iter().map(|p| palette(p.0))
}

/// How an icon is drawn: in a theme's colours, its `currentColor` (default:
/// the palette's `fg`), enabled or not.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub palette: Palette,
    pub color: Option<u32>,
    pub disabled: bool,
}

impl Style {
    pub fn new(theme: &str) -> Self {
        Style { palette: palette(theme), color: None, disabled: false }
    }
}

fn hex(c: u32) -> String {
    format!("#{:06X}", c & 0xFF_FFFF)
}

/// `icon`'s drawing for `variant` px in `style`'s colours: an SVG that
/// stands alone (the docs export, the web).
pub fn themed_svg(icon: &Icon, variant: u32, style: &Style) -> String {
    let mut s = icon.svg(variant).to_string();
    let p = &style.palette;
    for (i, key) in generated::KEYS.iter().enumerate() {
        let token = generated::TOKENS[i];
        let value = if style.disabled {
            if token.ends_with("-tint") || token == "paper" || token == "shade" {
                p.token("paper").unwrap_or(0xFFFFFF)
            } else {
                p.disabled
            }
        } else {
            p.values[i]
        };
        // (each source spells a token as `"#RRGGBB"`, uppercase; values are
        // written lowercase so one already swapped is never swapped again)
        s = s.replace(&format!("\"{key}\""), &format!("\"{}\"", hex(value).to_ascii_lowercase()));
    }
    let fg = if style.disabled { p.disabled } else { style.color.unwrap_or(p.fg) };
    s.replacen("<svg ", &format!("<svg color=\"{}\" ", hex(fg)), 1)
}

// ---- pixels -------------------------------------------------------------------------

/// Which hinted drawing to draw `device` pixels big, and its scale: the
/// exact size, else a whole multiple of one (48 = 24 × 2), else the one
/// nearest a whole multiple.
pub fn variant_for(device: u32) -> (u32, f32) {
    if SIZES.contains(&device) {
        return (device, 1.0);
    }
    let mut best = (f32::MAX, SIZES[2]);
    for &v in SIZES.iter().rev() {
        let r = device as f32 / v as f32;
        let err = if r >= 1.0 { (r - r.round()).abs() } else { 2.0 - r };
        if err < best.0 - 1e-6 {
            best = (err, v);
        }
    }
    (best.1, device as f32 / best.1 as f32)
}

/// Pixels: `width` × `height`, straight (not premultiplied) RGBA.
#[derive(Clone, Debug, PartialEq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// `icon` drawn `logical` pixels big at display `scale` (device pixels per
/// logical pixel), in `style`: a square of `round(logical × scale)` device
/// pixels.
pub fn render(icon: &Icon, logical: u32, scale: f32, style: &Style) -> Option<Rgba> {
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let device = ((logical as f32 * scale).round() as u32).clamp(1, 1024);
    let (variant, k) = variant_for(device);
    render_svg(&themed_svg(icon, variant, style), device, k)
}

/// An icon's SVG (`themed_svg`) drawn into `device` × `device` pixels at
/// scale `k`.
pub fn render_svg(svg: &str, device: u32, k: f32) -> Option<Rgba> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(device, device)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pixmap.as_mut());
    let mut data = Vec::with_capacity((device * device * 4) as usize);
    for p in pixmap.pixels() {
        let c = p.demultiply();
        data.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    Some(Rgba { width: device, height: device, data })
}

/// WCAG 2.x contrast of two colours (0xRRGGBB).
pub fn contrast(a: u32, b: u32) -> f64 {
    fn lum(c: u32) -> f64 {
        let ch = |v: u32| {
            let v = f64::from(v & 0xFF) / 255.0;
            if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * ch(c >> 16) + 0.7152 * ch(c >> 8) + 0.0722 * ch(c)
    }
    let (la, lb) = (lum(a), lum(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod tests;
