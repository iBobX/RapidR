//! Text: shaping, line layout and editing with `parley`, from the very
//! fonts RapidR measures and draws bitmap text with
//! (`rapidr_value::objects::text`): the built-in Liberation fonts, made
//! with the character widths of Arial, Times New Roman and Courier New,
//! which RapidQ programs name. Characters they lack (CJK, emoji …) fall
//! back to the system's fonts (feature `system-fonts`).
//!
//! Two decisions (`docs/desktop-host-plan.md` §4) keep a caption exactly as
//! wide as `TextWidth` says, and as the shared models (tab widths) measure:
//!
//! - **One size rule**: a QFONT's size in pixels is
//!   `Font::pixel_size()` — points at 96 dpi rounded as Windows' MulDiv
//!   rounds them (10 pt = 13 px).
//! - **No kerning, no ligatures**: GDI's TextOut applies neither, and
//!   `text_size` sums the characters' advances.
//!
//! Layouts are made at the screen's scale, so glyphs land on device pixels.

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use parley::fontique::{Blob, GenericFamily};
use parley::{FontContext, FontFamily, FontFamilyName, FontFeatures, FontStyle, FontWeight, Layout, LayoutContext, LineHeight, StyleProperty};
use rapidr_value::objects::font::Font;

pub mod editor;

pub use editor::{Align, Look, Pos, RunStyle, Span, TextEditor};

/// Text colour carried through a layout: 0xRRGGBB.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ink(pub u32);

/// GDI's TextOut: no kerning, no ligatures.
pub const FEATURES: &str = "\"kern\" off, \"liga\" off, \"clig\" off";

/// The font database and parley's scratch space, shared by every component
/// of every form (making either is costly).
pub struct TextSystem {
    pub font_cx: FontContext,
    pub layout_cx: LayoutContext<Ink>,
    /// Goes up when a font is added ([`TextSystem::add_font`]): what was
    /// laid out before is laid out again.
    pub generation: u64,
}

/// What hears of a character no loaded font has.
pub type MissingHook = Rc<dyn Fn(char)>;

thread_local! {
    /// The families a character is looked for in after a QFONT's own face
    /// and before the system's: the fallback fonts a host loads (the web's
    /// Noto set, docs/web-host-plan.md §3.7).
    static FALLBACKS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Told each character no loaded font has (the web host then fetches
    /// the fallback font that has it); none on the desktop.
    static MISSING: RefCell<Option<MissingHook>> = const { RefCell::new(None) };
}

/// The fallback families, in the order they're tried.
pub fn set_fallback_families(names: Vec<String>) {
    FALLBACKS.with(|f| *f.borrow_mut() = names);
}

/// What hears of characters no loaded font has (`None`: nothing).
pub fn set_missing_glyph_hook(hook: Option<MissingHook>) {
    MISSING.with(|m| *m.borrow_mut() = hook);
}

/// Tells the missing-glyph hook (if any) about `layout`'s characters drawn
/// as no font's (glyph 0) — `text` is what it laid out.
pub fn note_missing(layout: &Layout<Ink>, text: &str) {
    let Some(hook) = MISSING.with(|m| m.borrow().clone()) else { return };
    for line in layout.lines() {
        for item in line.items() {
            let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item else { continue };
            let run = glyph_run.run();
            for cluster in run.clusters() {
                if cluster.glyphs().any(|g| g.id == 0) {
                    if let Some(s) = text.get(cluster.text_range()) {
                        s.chars().filter(|c| !c.is_whitespace() && !c.is_control()).for_each(|c| hook(c));
                    }
                }
            }
        }
    }
}

impl Default for TextSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl TextSystem {
    pub fn new() -> Self {
        let mut font_cx = FontContext::new();
        for data in rapidr_value::objects::text::BUILTIN_FONTS {
            font_cx.collection.register_fonts(Blob::new(Arc::new(data)), None);
        }
        TextSystem { font_cx, layout_cx: LayoutContext::new(), generation: 0 }
    }

    /// Font file `data` (OpenType) added to what text is drawn with: a
    /// fallback font's chunk the host loaded.
    pub fn add_font(&mut self, data: Vec<u8>) {
        self.font_cx.collection.register_fonts(Blob::new(Arc::new(data)), None);
        self.generation += 1;
    }

    /// `text` (its lines) in `font`, laid out at `scale` device pixels per
    /// logical pixel (positions in device pixels).
    pub fn layout(&mut self, text: &str, font: &Font, color: u32, scale: f32) -> Layout<Ink> {
        let mut builder = self.layout_cx.ranged_builder(&mut self.font_cx, text, scale, true);
        for prop in styles(font, color) {
            builder.push_default(prop);
        }
        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        note_missing(&layout, text);
        layout
    }

    /// (the dialogs lane's) The font families there are — the system's and
    /// the built-in ones — for a font dialog's list (names starting with a
    /// `.`, the system's private faces, left out).
    pub fn family_names(&mut self) -> Vec<String> {
        let mut names: Vec<String> = self.font_cx.collection.family_names().filter(|n| !n.starts_with('.')).map(str::to_string).collect();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        names
    }

    /// How wide and high `text` is in `font`, in logical pixels (unrounded;
    /// `text_size` rounds the same width).
    pub fn measure(&mut self, text: &str, font: &Font) -> (f32, f32) {
        let layout = self.layout(text, font, 0, 1.0);
        (layout.full_width(), layout.height())
    }
}

/// The parley family name of the built-in face standing for a QFONT name.
pub fn family(name: &str) -> &'static str {
    rapidr_value::objects::text::family_name(name)
}

/// A QFONT's size in logical pixels (the one size rule).
pub fn font_pixels(font: &Font) -> f32 {
    font.pixel_size() as f32
}

/// The parley styles for a QFONT (fsBold = bit 0, fsItalic 1, fsUnderline
/// 2, fsStrikeOut 3) drawn in `color` (0xRRGGBB).
pub fn styles(font: &Font, color: u32) -> Vec<StyleProperty<'static, Ink>> {
    let face = family(&font.name);
    // (then the fallback fonts and the system's for what Liberation lacks:
    // symbols, CJK, emoji)
    let mut names = vec![FontFamilyName::Named(Cow::Borrowed(face))];
    // (the code font's Latin subset: then the built-in mono, so columns
    // stay even)
    if face == rapidr_value::objects::text::CODE_FACE {
        names.push(FontFamilyName::Named(Cow::Borrowed("Liberation Mono")));
    }
    names.push(FontFamilyName::Generic(generic(face)));
    FALLBACKS.with(|f| names.extend(f.borrow().iter().map(|n| FontFamilyName::Named(Cow::Owned(n.clone())))));
    names.extend([FontFamilyName::Generic(GenericFamily::SystemUi), FontFamilyName::Generic(GenericFamily::Emoji)]);
    let mut out = vec![
        StyleProperty::FontFamily(FontFamily::List(Cow::Owned(names))),
        StyleProperty::FontSize(font_pixels(font)),
        StyleProperty::LineHeight(LineHeight::MetricsRelative(1.0)),
        StyleProperty::FontFeatures(FontFeatures::Source(Cow::Borrowed(FEATURES))),
        StyleProperty::Brush(Ink(color)),
    ];
    if font.styles & 1 != 0 {
        out.push(StyleProperty::FontWeight(FontWeight::BOLD));
    }
    if font.styles & 2 != 0 {
        out.push(StyleProperty::FontStyle(FontStyle::Italic));
    }
    if font.styles & 4 != 0 {
        out.push(StyleProperty::Underline(true));
    }
    if font.styles & 8 != 0 {
        out.push(StyleProperty::Strikethrough(true));
    }
    out
}

/// The generic family a Liberation face falls back to (for characters it
/// doesn't have).
fn generic(face: &str) -> GenericFamily {
    match face {
        "Liberation Mono" | rapidr_value::objects::text::CODE_FACE => GenericFamily::Monospace,
        "Liberation Serif" => GenericFamily::Serif,
        _ => GenericFamily::SansSerif,
    }
}

/// &HBBGGRR → 0xRRGGBB.
pub fn bgr_to_rgb(bgr: i64) -> u32 {
    // (a system colour, clBtnFace …: the theme's)
    let c = rapidr_value::objects::color_bgr(bgr);
    (c & 0xFF) << 16 | (c & 0xFF00) | (c >> 16)
}

/// Byte offset of the `n`th character.
pub(crate) fn byte_of(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map_or(s.len(), |(b, _)| b)
}

/// Characters before byte offset `b`.
pub(crate) fn chars_to(s: &str, b: usize) -> usize {
    s[..b.min(s.len())].chars().count()
}
