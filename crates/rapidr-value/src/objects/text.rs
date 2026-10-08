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
//! bold and italic are the face's own designs (Liberation's Bold, Italic and
//! Bold Italic, RapidR Sans Bold — `fonts/README.md`), so bold text is as
//! wide as Arial Bold's, Times New Roman Bold's and Courier New Bold's with
//! no help; only a character the styled face lacks (Greek, Cyrillic, Hebrew
//! … — the styled faces are cut to the Latin scripts) is drawn from the
//! Regular face, made bold by drawing it twice a pixel apart or slanted;
//! underline and strike-out are lines. Like Windows' TextOut, (x, y) is the
//! top left of the text's cell and a background colour fills the cell.

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

static SANS: &[u8] = include_bytes!("../../fonts/LiberationSans-Regular.ttf");
static RSANS: &[u8] = include_bytes!("../../fonts/RapidRSans-Regular.ttf");
static SERIF: &[u8] = include_bytes!("../../fonts/LiberationSerif-Regular.ttf");
static MONO: &[u8] = include_bytes!("../../fonts/LiberationMono-Regular.ttf");
// (statics, not consts: a const's bytes are copied into every place it is
// used, a static's are in the program once)
// Their designed Bold, Italic and Bold Italic: Liberation 2.1.5's, cut to
// the Latin scripts and renamed "RapidR Text …" as the licence asks of a
// Modified Version (the kernel registers them as members of the Liberation
// families); RapidR Sans Bold: RapidR Sans' bold, with MS Sans Serif Bold's
// widths. `tools/fonts/make_liberation_styles.py`, `make_rapidr_sans.py`.
static RSANS_BOLD: &[u8] = include_bytes!("../../fonts/RapidRSans-Bold.ttf");
static SANS_BOLD: &[u8] = include_bytes!("../../fonts/RapidRTextSans-Bold.ttf");
static SANS_ITALIC: &[u8] = include_bytes!("../../fonts/RapidRTextSans-Italic.ttf");
static SANS_BOLD_ITALIC: &[u8] = include_bytes!("../../fonts/RapidRTextSans-BoldItalic.ttf");
static SERIF_BOLD: &[u8] = include_bytes!("../../fonts/RapidRTextSerif-Bold.ttf");
static SERIF_ITALIC: &[u8] = include_bytes!("../../fonts/RapidRTextSerif-Italic.ttf");
static SERIF_BOLD_ITALIC: &[u8] = include_bytes!("../../fonts/RapidRTextSerif-BoldItalic.ttf");
static MONO_BOLD: &[u8] = include_bytes!("../../fonts/RapidRTextMono-Bold.ttf");
static MONO_ITALIC: &[u8] = include_bytes!("../../fonts/RapidRTextMono-Italic.ttf");
static MONO_BOLD_ITALIC: &[u8] = include_bytes!("../../fonts/RapidRTextMono-BoldItalic.ttf");
// RapidR's own UI and code faces (docs/ide-plan.md D8; fonts/README.md):
// Inter for an IDE's chrome, JetBrains Mono for code — named by programs
// that want them ("Inter", "JetBrains Mono"); RapidQ's names never map here.
static INTER: &[u8] = include_bytes!("../../fonts/Inter-Regular.ttf");
static INTER_SEMIBOLD: &[u8] = include_bytes!("../../fonts/Inter-SemiBold.ttf");
static JBMONO: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Regular.ttf");
static JBMONO_BOLD: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Bold.ttf");

/// Longest text drawn in one call (so a huge string can't stall drawing).
const MAX_CHARS: usize = 10_000;

/// The built-in faces' files, and the family each belongs to when it isn't
/// the one the file names (`None`): what the UI kernel registers with its
/// text shaper, so its captions are drawn from the very fonts `TextWidth`
/// measures. Liberation's Regular faces are the unmodified originals; their
/// Bold, Italic and Bold Italic are renamed files ("RapidR Text Sans" …,
/// the licence's Reserved Font Name rule) that join the Liberation families
/// here, so a request for bold "Liberation Sans" finds the bold face. The
/// kernel looks a character the bold face lacks up in the family's Regular.
pub static BUILTIN_FACES: [(&[u8], Option<&str>); 18] = [
    (SANS, None),
    (SERIF, None),
    (MONO, None),
    (RSANS, None),
    (RSANS_BOLD, None),
    (SANS_BOLD, Some("Liberation Sans")),
    (SANS_ITALIC, Some("Liberation Sans")),
    (SANS_BOLD_ITALIC, Some("Liberation Sans")),
    (SERIF_BOLD, Some("Liberation Serif")),
    (SERIF_ITALIC, Some("Liberation Serif")),
    (SERIF_BOLD_ITALIC, Some("Liberation Serif")),
    (MONO_BOLD, Some("Liberation Mono")),
    (MONO_ITALIC, Some("Liberation Mono")),
    (MONO_BOLD_ITALIC, Some("Liberation Mono")),
    (INTER, None),
    (INTER_SEMIBOLD, None),
    (JBMONO, None),
    (JBMONO_BOLD, None),
];

/// The built-in face standing for a QFONT's name, by its family name:
/// MS Sans Serif (RapidQ's default; Microsoft Sans Serif, MS Shell Dlg,
/// "Sans Serif"): "RapidR Sans"; Courier / mono: "Liberation Mono"; Times /
/// serif / Roman: "Liberation Serif"; anything else: "Liberation Sans".
pub fn family_name(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    let n = n.trim();
    if n == "inter" || n.starts_with("inter ") {
        "Inter"
    } else if n.starts_with("jetbrains mono") || n == "jetbrainsmono" {
        "JetBrains Mono"
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

/// A built-in face for a font's name and styles: its file, and which of the
/// styles asked for it has designed itself (`bold`, `italic`); the others
/// the renderer makes from it (drawn twice a pixel apart, slanted).
struct Face {
    data: &'static [u8],
    bold: bool,
    italic: bool,
}

fn face(name: &str, styles: u8) -> Face {
    let (bold, italic) = (styles & 1 != 0, styles & 2 != 0);
    match family_name(name) {
        // (Inter and JetBrains Mono have a bold and no italic; RapidR Sans
        // has a bold, MS Sans Serif's italic being the regular slanted)
        "Inter" => Face { data: if bold { INTER_SEMIBOLD } else { INTER }, bold, italic: false },
        "JetBrains Mono" => Face { data: if bold { JBMONO_BOLD } else { JBMONO }, bold, italic: false },
        "RapidR Sans" => Face { data: if bold { RSANS_BOLD } else { RSANS }, bold, italic: false },
        "Liberation Mono" => liberation(bold, italic, [MONO, MONO_BOLD, MONO_ITALIC, MONO_BOLD_ITALIC]),
        "Liberation Serif" => liberation(bold, italic, [SERIF, SERIF_BOLD, SERIF_ITALIC, SERIF_BOLD_ITALIC]),
        _ => liberation(bold, italic, [SANS, SANS_BOLD, SANS_ITALIC, SANS_BOLD_ITALIC]),
    }
}

/// The Liberation face for the styles: `[regular, bold, italic, bold italic]`.
fn liberation(bold: bool, italic: bool, faces: [&'static [u8]; 4]) -> Face {
    Face { data: faces[usize::from(bold) + 2 * usize::from(italic)], bold, italic }
}

/// The font's size in pixels (Font::pixel_size: whole pixels, as GDI's).
fn pixel_size(font: &Font) -> f32 {
    font.pixel_size() as f32
}

/// A face and its scale for a font.
struct Scaled {
    face: ttf_parser::Face<'static>,
    /// The Regular face, for a character the styled face lacks (the bold,
    /// italic and bold italic faces are cut to the Latin scripts); none when
    /// the face is the Regular itself.
    regular: Option<ttf_parser::Face<'static>>,
    /// Bold / italic asked for and not designed in the face: made from its
    /// letters (drawn twice, slanted).
    synth_bold: bool,
    synth_italic: bool,
    /// Bold / italic asked for (what a Regular face's character gets).
    bold: bool,
    italic: bool,
    scale: f32,
    ascent: f32,
    height: f32,
}

fn scaled(font: &Font) -> Option<Scaled> {
    scaled_by(font, 1.0)
}

/// The font drawn `by` times larger (on a high-DPI screen's pixels).
fn scaled_by(font: &Font, by: f32) -> Option<Scaled> {
    let want = face(&font.name, font.styles);
    let regular = face(&font.name, 0).data;
    let parsed = ttf_parser::Face::parse(want.data, 0).ok()?;
    let backup = if std::ptr::eq(want.data, regular) { None } else { ttf_parser::Face::parse(regular, 0).ok() };
    let px = pixel_size(font) * by;
    let scale = px / parsed.units_per_em() as f32;
    let ascent = parsed.ascender() as f32 * scale;
    let height = (parsed.ascender() as f32 - parsed.descender() as f32) * scale;
    let (bold, italic) = (font.styles & 1 != 0, font.styles & 2 != 0);
    Some(Scaled { face: parsed, regular: backup, synth_bold: bold && !want.bold, synth_italic: italic && !want.italic, bold, italic, scale, ascent, height })
}

/// A character's glyph: the face it comes from, the glyph, and whether it
/// is made bold / slanted by drawing (the styled face's own glyphs are not).
struct Glyph<'a> {
    face: &'a ttf_parser::Face<'static>,
    id: ttf_parser::GlyphId,
    scale: f32,
    bold: bool,
    slant: bool,
}

impl Scaled {
    /// `c`'s glyph: the styled face's, else the Regular face's (made bold or
    /// slanted), else a `?` the same way.
    fn glyph(&self, c: char) -> Option<Glyph<'_>> {
        for c in [c, '?'] {
            if let Some(id) = self.face.glyph_index(c) {
                return Some(Glyph { face: &self.face, id, scale: self.scale, bold: self.synth_bold, slant: self.synth_italic });
            }
            if let Some(reg) = &self.regular {
                if let Some(id) = reg.glyph_index(c) {
                    // (the Regular face's em may differ from the styled one's)
                    let scale = self.scale * f32::from(self.face.units_per_em()) / f32::from(reg.units_per_em());
                    return Some(Glyph { face: reg, id, scale, bold: self.bold, slant: self.italic });
                }
            }
        }
        None
    }

    fn advance(&self, c: char) -> f32 {
        self.glyph(c).and_then(|g| g.face.glyph_hor_advance(g.id).map(|a| f32::from(a) * g.scale)).unwrap_or(0.0)
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

/// RapidR Sans' em in pixels at 8 pt on a 96-dpi screen (MS Sans Serif's).
const RSANS_EM_PX: f32 = 11.0;

/// What a bold character takes beyond the face's own advance, in pixels
/// (letter spacing, negative at the larger sizes).
///
/// Only for MS Sans Serif (RapidR Sans): its bold is the bitmap font's
/// emboldening, one pixel wider a character at every size (RC.EXE:
/// `TextWidth` of each character, bold against regular, at 8 to 14 pt). RapidR
/// Sans Bold carries that pixel as a 11th of its em (it is exactly 1 pixel at
/// 8 pt); at the other sizes this makes up the difference to a whole pixel,
/// so a bold text is the regular's width plus one pixel a character, as in
/// RapidQ. The other faces' bold is their own design, as wide as the real
/// Arial Bold, Times New Roman Bold and Courier New Bold: nothing added.
pub fn bold_spacing(font: &Font) -> f32 {
    if font.styles & 1 != 0 && family_name(&font.name) == "RapidR Sans" {
        1.0 - pixel_size(font) / RSANS_EM_PX
    } else {
        0.0
    }
}

/// GDI's tmAveCharWidth as Windows gives it for a font without a width
/// table — the width of `x` — where tab stops fall (DT_EXPANDTABS, a list's
/// TabWidth). MS Sans Serif's x is 5 pixels at 8 pt: RapidR Sans draws its
/// x a pixel wider (fonts/README.md) but keeps RapidQ's tab stops.
pub fn average_char_width(font: &Font) -> i64 {
    if family_name(&font.name) == "RapidR Sans" {
        return ((5.0 * pixel_size(font) / 11.0).round() as i64).max(1);
    }
    text_size("x", font).0.max(1)
}

/// `TextWidth` / `TextHeight` of `text` in `font`, in pixels: the sum of
/// the characters' advances in the face drawn (a bold face's are its own:
/// Arial Bold's, Times New Roman Bold's, MS Sans Serif Bold's).
pub fn text_size(text: &str, font: &Font) -> (i64, i64) {
    let Some(s) = scaled(font) else { return (0, 0) };
    let spacing = bold_spacing(font);
    let w: f32 = text.chars().take(MAX_CHARS).map(|c| s.advance(c) + spacing).sum();
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
fn glyphs(target: &mut impl Target, s: &Scaled, x: f32, y: f32, by: f32, text: &str, spacing: f32, color: u32) {
    let baseline = y + s.ascent;
    let mut pen = x;
    for c in text.chars().take(MAX_CHARS) {
        let Some(g) = s.glyph(c) else { continue };
        // (bold the face has no design for: drawn again a pixel — `by`
        // device pixels — to the right; italic: slanted)
        let passes = if g.bold { 1 + by.round().max(1.0) as usize } else { 1 };
        let slant = if g.slant { 0.2 } else { 0.0 };
        for dx in 0..passes {
            let mut e = Edges { edges: Vec::new(), at: (0.0, 0.0), start: (0.0, 0.0), scale: g.scale, origin: (pen + dx as f32, baseline), slant };
            if g.face.outline_glyph(g.id, &mut e).is_some() {
                fill(target, &e.edges, color);
            }
        }
        pen += g.face.glyph_hor_advance(g.id).map_or(0.0, |a| f32::from(a) * g.scale) + spacing * by;
    }
}

/// A target drawn on only inside a rectangle [l, r) × [t, b) (TextRect).
struct Clipped<'a, T: Target> {
    target: &'a mut T,
    rect: (i64, i64, i64, i64),
}

impl<T: Target> Target for Clipped<'_, T> {
    fn size(&self) -> (i64, i64) {
        self.target.size()
    }
    fn get(&self, x: i64, y: i64) -> Option<u32> {
        self.target.get(x, y)
    }
    fn set(&mut self, x: i64, y: i64, c: u32) {
        let (l, t, r, b) = self.rect;
        if x >= l && x < r && y >= t && y < b {
            self.target.set(x, y, c);
        }
    }
}

/// No clipping: the whole of any bitmap.
const UNCLIPPED: (i64, i64, i64, i64) = (i64::MIN / 4, i64::MIN / 4, i64::MAX / 4, i64::MAX / 4);

/// `TextOut(x, y, text, colour, background)` on `bmp` in `font`
/// (background `None`: transparent).
pub fn text_out(bmp: &mut Bitmap, x: i64, y: i64, text: &str, font: &Font, color: u32, background: Option<u32>) {
    if let Some(bg) = background {
        let (tw, th) = text_size(text, font);
        bmp.fill_rect(x, y, x + tw, y + th, bg);
    }
    clipped_text(bmp, UNCLIPPED, x, y, text, font, color);
}

/// `TextRect(Rect, x, y, text, colour, background)`: the text as TextOut
/// draws it at (x, y), but only inside `rect` (Left, Top, Right, Bottom;
/// the right and bottom edges excluded). A background colour fills the
/// whole rectangle first, as Windows' ExtTextOut does for Delphi's
/// TextRect with a solid brush; `None` (-1) leaves what's there.
#[allow(clippy::too_many_arguments)]
pub fn text_rect(bmp: &mut Bitmap, rect: (i64, i64, i64, i64), x: i64, y: i64, text: &str, font: &Font, color: u32, background: Option<u32>) {
    let (l, t, r, b) = (rect.0.min(rect.2), rect.1.min(rect.3), rect.0.max(rect.2), rect.1.max(rect.3));
    if let Some(bg) = background {
        bmp.fill_rect(l, t, r, b, bg);
    }
    clipped_text(bmp, (l, t, r, b), x, y, text, font, color);
}

/// The text's glyphs (and underline / strike-out) at (x, y), only inside
/// `clip` — on the pixels and on what a high-DPI screen shows.
fn clipped_text(bmp: &mut Bitmap, clip: (i64, i64, i64, i64), x: i64, y: i64, text: &str, font: &Font, color: u32) {
    let Some(s) = scaled(font) else { return };
    let (tw, _) = text_size(text, font);
    let baseline = y as f32 + s.ascent;
    glyphs(&mut Clipped { target: &mut *bmp, rect: clip }, &s, x as f32, y as f32, 1.0, text, bold_spacing(font), color);
    // What a high-DPI screen shows: the same text at its scale (the glyphs
    // finer, where they start and advance the same).
    if let Some(hi) = bmp.display_mut() {
        let by = hi.scale as f32;
        let k = hi.scale as i64;
        let fine_clip = if clip == UNCLIPPED { UNCLIPPED } else { (clip.0 * k, clip.1 * k, clip.2 * k, clip.3 * k) };
        if let Some(fine) = scaled_by(font, by) {
            glyphs(&mut Clipped { target: hi, rect: fine_clip }, &fine, x as f32 * by, y as f32 * by, by, text, bold_spacing(font), color);
        }
    }
    let line = |bmp: &mut Bitmap, at: f32| {
        let yy = at.round() as i64;
        let thick = (pixel_size(font) / 16.0).round().max(1.0) as i64;
        let (l, t, r, b) = (x.max(clip.0), yy.max(clip.1), (x + tw).min(clip.2), (yy + thick).min(clip.3));
        if l < r && t < b {
            bmp.fill_rect(l, t, r, b, color);
        }
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

    fn styled(name: &str, size: i64, styles: u8) -> Font {
        Font { name: name.into(), size, styles, color: 0 }
    }

    /// RC.EXE's `TextWidth` (tools/rc_probe.sh, scratch/boldprobe-style probe
    /// on a QBITMAP, Windows 11), bold and italic: the designed faces' own
    /// widths, no spacing added.
    #[test]
    fn bold_and_italic_widths_are_rc_exes() {
        for (name, size, styles, text, rc) in [
            // Arial Bold (Liberation Sans Bold): 37 and 39 where the regular
            // letters drawn heavier with a bit of spacing made 38 and 40
            ("Arial", 9, 1, "Pantry", 37),
            ("Arial", 12, 1, "Hello", 39),
            ("Arial", 12, 3, "Hello", 39),
            ("Arial", 12, 1, "Pantry", 50),
            ("Arial", 12, 1, "The quick brown fox jumps over the lazy dog", 342),
            ("Arial", 10, 1, "Right-click", 66),
            ("Arial", 14, 1, "Right-click", 97),
            ("Arial", 18, 1, "Right-click", 123),
            ("Arial", 12, 2, "Notepad - untitled", 126),
            ("Arial", 10, 3, "Notepad - untitled", 110),
            // Times New Roman Bold and Italic
            ("Times New Roman", 12, 1, "Hello", 36),
            ("Times New Roman", 14, 1, "Right-click", 90),
            ("Times New Roman", 12, 2, "Pantry", 44),
            ("Times New Roman", 12, 3, "Times", 40),
            // MS Sans Serif (RapidR Sans) Bold, a pixel wider a character
            ("MS Sans Serif", 8, 1, "Pantry", 36),
            ("MS Sans Serif", 8, 1, "Hello", 29),
            ("MS Sans Serif", 8, 1, "Right-click", 61),
            ("MS Sans Serif", 8, 1, "Notepad - untitled", 102),
            ("MS Sans Serif", 8, 1, "The quick brown fox jumps over the lazy dog", 254),
            ("MS Sans Serif", 8, 1, "WAVE Typography 0123456789", 180),
            ("MS Sans Serif", 8, 1, "mmmmiiiiWWWW", 96),
            ("MS Sans Serif", 8, 3, "Notepad - untitled", 102),
        ] {
            assert_eq!(text_size(text, &styled(name, size, styles)).0, rc, "{name} {size} pt style {styles}: {text:?}");
        }
    }

    /// Bold and italic are the faces' own designs: Liberation's for Arial,
    /// Times New Roman and Courier New, RapidR Sans Bold for MS Sans Serif
    /// (its italic being the regular slanted, Inter's and JetBrains Mono's
    /// too); only what a face doesn't design is made by the renderer.
    #[test]
    fn styles_come_from_designed_faces() {
        for (name, bold, italic) in [("Arial", true, true), ("Times New Roman", true, true), ("Courier New", true, true), ("MS Sans Serif", true, false), ("Inter", true, false), ("JetBrains Mono", true, false)] {
            let s = scaled(&styled(name, 12, 3)).unwrap();
            assert_eq!((!s.synth_bold, !s.synth_italic), (bold, italic), "{name}");
        }
        // the Bold face is a different design, not the Regular's: its "i"
        // is wider (Arial Bold's 0.278 em against 0.222)
        let regular = scaled(&styled("Arial", 100, 0)).unwrap();
        let bold = scaled(&styled("Arial", 100, 1)).unwrap();
        assert!(bold.advance('i') > regular.advance('i') + 3.0, "{} vs {}", bold.advance('i'), regular.advance('i'));
        // a regular face has no regular fallback of its own
        assert!(regular.regular.is_none() && bold.regular.is_some());
    }

    /// A character the styled faces (cut to the Latin scripts) lack is the
    /// Regular face's, made bold by the renderer (drawn twice a pixel
    /// apart) or slanted; its width is the Regular face's.
    #[test]
    fn characters_missing_from_a_styled_face_come_from_the_regular_one() {
        for c in ['\u{3b1}', '\u{416}', '\u{5d0}'] {
            let bold = scaled(&styled("Arial", 12, 1)).unwrap();
            let regular = scaled(&styled("Arial", 12, 0)).unwrap();
            let g = bold.glyph(c).unwrap();
            assert!(g.bold && !g.slant, "{c}");
            assert!(bold.face.glyph_index(c).is_none() && g.face.glyph_index(c).is_some(), "{c}: only the Regular face has it");
            assert_eq!(bold.advance(c), regular.advance(c), "{c}");
            let italic = scaled(&styled("Arial", 12, 2)).unwrap();
            let g = italic.glyph(c).unwrap();
            assert!(!g.bold && g.slant, "{c}");
        }
        // and it is drawn: Greek bold has ink, wider than the Greek regular's
        let ink = |styles| {
            let mut b = Bitmap::default();
            b.resize(40, 24);
            text_out(&mut b, 2, 2, "\u{3b1}\u{3b2}", &styled("Arial", 12, styles), 0, None);
            b.img.pixels.iter().map(|&p| 255 - (p & 0xFF) as u64).sum::<u64>()
        };
        assert!(ink(1) > ink(0), "{} vs {}", ink(1), ink(0));
        // Latin bold: ink too, and more of it than the regular's
        let latin = |styles| {
            let mut b = Bitmap::default();
            b.resize(60, 24);
            text_out(&mut b, 2, 2, "Hello", &styled("Arial", 12, styles), 0, None);
            b.img.pixels.iter().map(|&p| 255 - (p & 0xFF) as u64).sum::<u64>()
        };
        assert!(latin(1) > latin(0) * 11 / 10, "{} vs {}", latin(1), latin(0));
    }

    /// The OFL's Reserved Font Name rule: every built-in file with one of
    /// Liberation's names (Liberation, Arimo, Tinos, Cousine) in its font name
    /// is one of the three unmodified Regular files; the nine styled faces
    /// (subsets) and RapidR Sans are renamed and say what licence they have.
    #[test]
    fn modified_faces_are_renamed_and_licensed() {
        const RESERVED: [&str; 4] = ["liberation", "arimo", "tinos", "cousine"];
        let name = |face: &ttf_parser::Face, id: u16| face.names().into_iter().find(|n| n.name_id == id && n.is_unicode()).and_then(|n| n.to_string()).unwrap_or_default();
        let (mut reserved, mut renamed) = (0, 0);
        for &(data, _) in BUILTIN_FACES.iter() {
            let face = ttf_parser::Face::parse(data, 0).unwrap();
            let family = name(&face, 1);
            let postscript = name(&face, 6);
            let full = name(&face, 4);
            let has = |s: &str| RESERVED.iter().any(|r| s.to_lowercase().contains(r));
            if has(&family) || has(&full) || has(&postscript) {
                reserved += 1;
                // (unmodified: still the whole 2.1.5 font, hinting and all)
                assert!([SANS, SERIF, MONO].contains(&data), "{family}: a Reserved Font Name on a modified face");
            } else if family.starts_with("RapidR") {
                renamed += 1;
                assert!(name(&face, 13).contains("SIL Open Font License"), "{family}: its licence");
                assert!(name(&face, 0).contains("Red Hat") && name(&face, 0).contains("Google"), "{family}: Liberation's copyright lines");
            }
        }
        assert_eq!((reserved, renamed), (3, 11));
    }
}
