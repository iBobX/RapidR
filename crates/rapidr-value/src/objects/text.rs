//! Text on bitmaps (`Bitmap.TextOut`, `TextWidth`, `TextHeight`; a
//! QIMAGE's picture too): drawn the same on every platform, from fonts
//! built into RapidR — the Liberation fonts (SIL Open Font License 1.1,
//! `fonts/`), made with the same character widths as Arial, Times New Roman
//! and Courier New, which RapidQ programs name — read with `ttf-parser`
//! and filled here with 4 × 4 anti-aliasing.
//!
//! A QFONT's name picks the face (MS Sans Serif, RapidQ's default — and
//! Microsoft Sans Serif, MS Shell Dlg: RapidR Sans, Liberation Sans made as
//! wide as MS Sans Serif, `fonts/README.md`; Courier / mono: Liberation
//! Mono; Times / serif / Roman: Liberation Serif; anything else: Liberation
//! Sans, Arial's widths), its
//! size is points at 96 dpi (Windows' screen resolution: 12 pt = 16 px);
//! bold is drawn twice a pixel apart, italic slanted, underline and
//! strike-out as lines. Like Windows' TextOut, (x, y) is the top left of
//! the text's cell and a background colour fills the cell.

use super::bitmap::{Bitmap, HiRes};
use super::font::Font;

/// What text is drawn onto: a bitmap's pixels, or what a high-DPI screen
/// shows of them (drawn at its scale).
trait Target {
    fn size(&self) -> (i64, i64);
    fn get(&self, x: i64, y: i64) -> Option<u32>;
    fn set(&mut self, x: i64, y: i64, c: u32);
}

impl Target for Bitmap {
    fn size(&self) -> (i64, i64) {
        (self.img.width as i64, self.img.height as i64)
    }
    fn get(&self, x: i64, y: i64) -> Option<u32> {
        self.pixel(x, y)
    }
    fn set(&mut self, x: i64, y: i64, c: u32) {
        self.lo_pset(x, y, c);
    }
}

impl Target for HiRes {
    fn size(&self) -> (i64, i64) {
        (self.img.width as i64, self.img.height as i64)
    }
    fn get(&self, x: i64, y: i64) -> Option<u32> {
        self.pixel(x, y)
    }
    fn set(&mut self, x: i64, y: i64, c: u32) {
        self.put(x, y, c);
    }
}

const SANS: &[u8] = include_bytes!("../../fonts/LiberationSans-Regular.ttf");
const RSANS: &[u8] = include_bytes!("../../fonts/RapidRSans-Regular.ttf");
const SERIF: &[u8] = include_bytes!("../../fonts/LiberationSerif-Regular.ttf");
const MONO: &[u8] = include_bytes!("../../fonts/LiberationMono-Regular.ttf");
// RapidR's own UI and code faces (docs/ide-plan.md D8; fonts/README.md):
// Inter for an IDE's chrome, JetBrains Mono for code — named by programs
// that want them ("Inter", "JetBrains Mono"); RapidQ's names never map here.
const INTER: &[u8] = include_bytes!("../../fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../fonts/Inter-SemiBold.ttf");
const JBMONO: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Regular.ttf");
const JBMONO_BOLD: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Bold.ttf");
/// The code editor's comments (RCODEEDITOR's schemes draw them italic).
const JBMONO_ITALIC: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Italic.ttf");

/// The code editor's face (RCODEEDITOR, RDIFFVIEW): what [`family_name`]
/// gives for "JetBrains Mono", and the shaper's family name.
pub const CODE_FACE: &str = "JetBrains Mono";

/// Longest text drawn in one call (so a huge string can't stall drawing).
const MAX_CHARS: usize = 10_000;

/// The built-in faces' files (Liberation Sans, Serif, Mono, RapidR Sans,
/// Inter regular and semibold, JetBrains Mono regular, bold and italic):
/// what the UI kernel registers with its text shaper, so its captions are
/// drawn from the very fonts `TextWidth` measures.
pub const BUILTIN_FONTS: [&[u8]; 9] = [SANS, SERIF, MONO, RSANS, INTER, INTER_SEMIBOLD, JBMONO, JBMONO_BOLD, JBMONO_ITALIC];

/// The built-in face standing for a QFONT's name, by its family name:
/// JetBrains: RapidR's code font; MS Sans Serif (RapidQ's default;
/// Microsoft Sans Serif, MS Shell Dlg, "Sans Serif"): "RapidR Sans";
/// Courier / mono: "Liberation Mono"; Times / serif / Roman: "Liberation
/// Serif"; anything else: "Liberation Sans".
pub fn family_name(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    let n = n.trim();
    if n == "inter" || n.starts_with("inter ") {
        "Inter"
    } else if n.starts_with("jetbrains mono") || n == "jetbrainsmono" {
        CODE_FACE
    } else if n.contains("courier") || n.contains("mono") || n.contains("fixed") || n.contains("terminal") || n.contains("console") {
        "Liberation Mono"
    } else if n == "ms sans serif" || n == "microsoft sans serif" || n == "sans serif" || n.starts_with("ms shell dlg") || n == "ms sans" || n == "helv" {
        "RapidR Sans"
    } else if n.contains("sans") {
        "Liberation Sans"
    } else if n.contains("times") || n.contains("serif") || n.contains("roman") || n.contains("georgia") {
        "Liberation Serif"
    } else {
        "Liberation Sans"
    }
}

fn face_data(name: &str, bold: bool) -> &'static [u8] {
    match family_name(name) {
        "Inter" if bold => INTER_SEMIBOLD,
        "Inter" => INTER,
        "JetBrains Mono" if bold => JBMONO_BOLD,
        "JetBrains Mono" => JBMONO,
        "Liberation Mono" => MONO,
        "Liberation Serif" => SERIF,
        "RapidR Sans" => RSANS,
        _ => SANS,
    }
}

/// The font's size in pixels (Font::pixel_size: whole pixels, as GDI's).
fn pixel_size(font: &Font) -> f32 {
    font.pixel_size() as f32
}

/// A face and its scale for a font.
struct Scaled {
    face: ttf_parser::Face<'static>,
    scale: f32,
    ascent: f32,
    height: f32,
}

fn scaled(font: &Font) -> Option<Scaled> {
    scaled_by(font, 1.0)
}

/// The font drawn `by` times larger (on a high-DPI screen's pixels).
fn scaled_by(font: &Font, by: f32) -> Option<Scaled> {
    let face = ttf_parser::Face::parse(face_data(&font.name, font.styles & 1 != 0), 0).ok()?;
    let px = pixel_size(font) * by;
    let scale = px / face.units_per_em() as f32;
    let ascent = face.ascender() as f32 * scale;
    let height = (face.ascender() as f32 - face.descender() as f32) * scale;
    Some(Scaled { face, scale, ascent, height })
}

impl Scaled {
    fn advance(&self, c: char) -> f32 {
        let g = self.face.glyph_index(c).or_else(|| self.face.glyph_index('?'));
        g.and_then(|g| self.face.glyph_hor_advance(g)).unwrap_or(0) as f32 * self.scale
    }
}

/// The distance from a line's top to its text's baseline, and the line's
/// height, in `font`, in pixels (the designer's baseline guides).
pub fn line_metrics(font: &Font) -> (f32, f32) {
    scaled(font).map_or((0.0, 0.0), |s| (s.ascent, s.height))
}

/// How far below the top of a line of `font`'s text its baseline is, in
/// pixels (GDI's tmAscent: 11 for MS Sans Serif 8).
pub fn ascent(font: &Font) -> f32 {
    scaled(font).map_or(0.0, |s| s.ascent)
}

/// The space a bold character takes beyond its regular width, in pixels:
/// one for MS Sans Serif (RapidR Sans) — Windows' MS Sans Serif Bold is a
/// pixel wider a character, RapidQ's capture shows — none for the others
/// (their bold is the regular advance, one pixel more for the whole text).
pub fn bold_spacing(font: &Font) -> f32 {
    if font.styles & 1 != 0 && family_name(&font.name) == "RapidR Sans" { 1.0 } else { 0.0 }
}

/// `TextWidth` / `TextHeight` of `text` in `font`, in pixels.
pub fn text_size(text: &str, font: &Font) -> (i64, i64) {
    let Some(s) = scaled(font) else { return (0, 0) };
    let bold = font.styles & 1 != 0;
    let spacing = bold_spacing(font);
    let w: f32 = text.chars().take(MAX_CHARS).map(|c| s.advance(c) + spacing).sum::<f32>() + if bold && spacing == 0.0 { 1.0 } else { 0.0 };
    (w.round() as i64, s.height.ceil() as i64)
}

/// Collects a glyph's outline as line segments (curves flattened), in
/// pixels, y down.
struct Edges {
    edges: Vec<(f32, f32, f32, f32)>,
    at: (f32, f32),
    start: (f32, f32),
    scale: f32,
    origin: (f32, f32),
    slant: f32,
}

impl Edges {
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        let (px, py) = (self.origin.0 + x * self.scale, self.origin.1 - y * self.scale);
        // Italic: the higher, the further right.
        (px + self.slant * (self.origin.1 - py), py)
    }

    fn line(&mut self, to: (f32, f32)) {
        if to != self.at {
            self.edges.push((self.at.0, self.at.1, to.0, to.1));
        }
        self.at = to;
    }
}

impl ttf_parser::OutlineBuilder for Edges {
    fn move_to(&mut self, x: f32, y: f32) {
        self.at = self.point(x, y);
        self.start = self.at;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        self.line(p);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (p0, p1, p2) = (self.at, self.point(x1, y1), self.point(x, y));
        for i in 1..=8 {
            let t = i as f32 / 8.0;
            let u = 1.0 - t;
            self.line((u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0, u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1));
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (p0, p1, p2, p3) = (self.at, self.point(x1, y1), self.point(x2, y2), self.point(x, y));
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            self.line((a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0, a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1));
        }
    }

    fn close(&mut self) {
        let s = self.start;
        self.line(s);
    }
}

/// Subsamples per pixel on each axis.
const SUB: usize = 4;

/// Fills the outline `edges` (nonzero winding) onto `bmp` in colour `c`,
/// each pixel by how much of it is covered.
fn fill(bmp: &mut impl Target, edges: &[(f32, f32, f32, f32)], c: u32) {
    if edges.is_empty() {
        return;
    }
    let (w, h) = bmp.size();
    let min_y = edges.iter().map(|e| e.1.min(e.3)).fold(f32::MAX, f32::min).floor().max(0.0) as i64;
    let max_y = (edges.iter().map(|e| e.1.max(e.3)).fold(f32::MIN, f32::max).ceil() as i64).min(h);
    let min_x = edges.iter().map(|e| e.0.min(e.2)).fold(f32::MAX, f32::min).floor().max(0.0) as i64;
    let max_x = (edges.iter().map(|e| e.0.max(e.2)).fold(f32::MIN, f32::max).ceil() as i64).min(w);
    if min_y >= max_y || min_x >= max_x {
        return;
    }
    let cols = (max_x - min_x) as usize;
    let mut cover = vec![0u16; cols];
    let mut crossings: Vec<(f32, i32)> = Vec::new();
    for py in min_y..max_y {
        cover.iter_mut().for_each(|c| *c = 0);
        for sy in 0..SUB {
            let y = py as f32 + (sy as f32 + 0.5) / SUB as f32;
            crossings.clear();
            for &(x0, y0, x1, y1) in edges {
                if (y0 <= y) != (y1 <= y) {
                    let x = x0 + (y - y0) * (x1 - x0) / (y1 - y0);
                    crossings.push((x, if y1 > y0 { 1 } else { -1 }));
                }
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            for pair in crossings.windows(2) {
                winding += pair[0].1;
                if winding == 0 {
                    continue;
                }
                // Subsample columns whose centres are inside [a, b).
                let (a, b) = (pair[0].0, pair[1].0);
                let first = ((a - min_x as f32) * SUB as f32 - 0.5).ceil().max(0.0) as usize;
                let last = ((b - min_x as f32) * SUB as f32 - 0.5).ceil().max(0.0) as usize;
                for s in first..last.min(cols * SUB) {
                    cover[s / SUB] += 1;
                }
            }
        }
        for (i, &n) in cover.iter().enumerate() {
            if n > 0 {
                blend(bmp, min_x + i as i64, py, c, n as u32 * 255 / (SUB * SUB) as u32);
            }
        }
    }
}

/// Mixes `c` into pixel (x, y) with coverage `a` (0–255).
fn blend(bmp: &mut impl Target, x: i64, y: i64, c: u32, a: u32) {
    let Some(old) = bmp.get(x, y) else { return };
    let a = a.min(255);
    let mix = |shift: u32| {
        let (o, n) = ((old >> shift) & 0xFF, (c >> shift) & 0xFF);
        ((n * a + o * (255 - a) + 127) / 255) << shift
    };
    bmp.set(x, y, mix(0) | mix(8) | mix(16));
}

/// The glyphs of `text` from (x, y) (their cell's top left) on `target`,
/// `by` times larger than the font's size.
#[allow(clippy::too_many_arguments)]
fn glyphs(target: &mut impl Target, s: &Scaled, x: f32, y: f32, by: f32, text: &str, font: &Font, color: u32) {
    // (a face with a bold of its own — Inter, JetBrains Mono — isn't drawn twice)
    let bold = font.styles & 1 != 0 && !matches!(family_name(&font.name), "Inter" | "JetBrains Mono");
    let slant = if font.styles & 2 != 0 { 0.2 } else { 0.0 };
    let baseline = y + s.ascent;
    let mut pen = x;
    for c in text.chars().take(MAX_CHARS) {
        let Some(g) = s.face.glyph_index(c).or_else(|| s.face.glyph_index('?')) else { continue };
        // (bold: drawn again a pixel — `by` device pixels — to the right)
        let passes = if bold { 1 + by.round().max(1.0) as usize } else { 1 };
        for dx in 0..passes {
            let mut e = Edges { edges: Vec::new(), at: (0.0, 0.0), start: (0.0, 0.0), scale: s.scale, origin: (pen + dx as f32, baseline), slant };
            if s.face.outline_glyph(g, &mut e).is_some() {
                fill(target, &e.edges, color);
            }
        }
        pen += s.advance(c) + bold_spacing(font) * by;
    }
}

/// `TextOut(x, y, text, colour, background)` on `bmp` in `font`
/// (background `None`: transparent).
pub fn text_out(bmp: &mut Bitmap, x: i64, y: i64, text: &str, font: &Font, color: u32, background: Option<u32>) {
    let Some(s) = scaled(font) else { return };
    let (tw, th) = text_size(text, font);
    if let Some(bg) = background {
        bmp.fill_rect(x, y, x + tw, y + th, bg);
    }
    let baseline = y as f32 + s.ascent;
    glyphs(bmp, &s, x as f32, y as f32, 1.0, text, font, color);
    // What a high-DPI screen shows: the same text at its scale (the glyphs
    // finer, where they start and advance the same).
    if let Some(hi) = bmp.display_mut() {
        let by = hi.scale as f32;
        if let Some(fine) = scaled_by(font, by) {
            glyphs(hi, &fine, x as f32 * by, y as f32 * by, by, text, font, color);
        }
    }
    let line = |bmp: &mut Bitmap, at: f32| {
        let yy = at.round() as i64;
        let thick = (pixel_size(font) / 16.0).round().max(1.0) as i64;
        bmp.fill_rect(x, yy, x + tw, yy + thick, color);
    };
    if font.styles & 4 != 0 {
        line(bmp, baseline + 1.0);
    }
    if font.styles & 8 != 0 {
        line(bmp, baseline - s.ascent * 0.3);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sans_serif_names_are_sans() {
        assert_eq!(super::family_name("MS Sans Serif"), "RapidR Sans");
        assert_eq!(super::family_name("Microsoft Sans Serif"), "RapidR Sans");
        assert_eq!(super::family_name("Arial"), "Liberation Sans");
        assert_eq!(super::family_name("Comic Sans MS"), "Liberation Sans");
        assert_eq!(super::family_name("Times New Roman"), "Liberation Serif");
        assert_eq!(super::family_name("Courier New"), "Liberation Mono");
        assert_eq!(super::family_name("JetBrains Mono"), super::CODE_FACE);
    }

    use super::*;

    fn font(name: &str, size: i64) -> Font {
        Font { name: name.into(), size, ..Font::default() }
    }

    #[test]
    fn metrics_like_arial() {
        // Liberation Sans has Arial's widths: "Hello" at 12 pt (16 px).
        let (w, h) = text_size("Hello", &font("Arial", 12));
        assert_eq!(w, 36);
        assert!((17..=20).contains(&h), "{h}");
        // Courier New / Liberation Mono: every character 0.6 em.
        assert_eq!(text_size("iiii", &font("Courier New", 12)).0, text_size("MMMM", &font("Courier New", 12)).0);
    }

    #[test]
    fn draws_text() {
        let mut b = Bitmap::default();
        b.resize(60, 30);
        text_out(&mut b, 2, 2, "Hi", &font("Arial", 12), 0x0000FF, None);
        let red = b.img.pixels.iter().filter(|&&p| p == 0x0000FF).count();
        let touched = b.img.pixels.iter().filter(|&&p| p != 0xFFFFFF).count();
        assert!(red > 10 && touched > red, "solid and anti-aliased pixels ({red}, {touched})");
        // Nothing outside the text's cell.
        let (tw, th) = text_size("Hi", &font("Arial", 12));
        for y in 0..30 {
            for x in 0..60 {
                if x < 2 || y < 2 || x >= 2 + tw + 1 || y >= 2 + th {
                    assert_eq!(b.pixel(x, y), Some(0xFFFFFF), "({x}, {y})");
                }
            }
        }
        // A background fills the cell.
        text_out(&mut b, 30, 2, "x", &font("Arial", 12), 0, Some(0x00FF00));
        assert_eq!(b.pixel(30, 2), Some(0x00FF00));
    }
}
