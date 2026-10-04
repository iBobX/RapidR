//! Text for the UI kernel: shaping, line layout and editing with `parley`,
//! from the same fonts RapidR draws bitmap text with
//! (`rapidr_value::objects::text`): the Liberation fonts, made with the
//! character widths of Arial, Times New Roman and Courier New, which RapidQ
//! programs name. So a caption is as wide here as `TextWidth` says it is,
//! and as the shared models (tab widths) measure it. Characters those fonts
//! lack (CJK, symbols …) fall back to the system's fonts.
//!
//! A QFONT's size is points at 96 dpi, rounded to whole pixels as Windows'
//! `MulDiv(size, 96, 72)` does (10 pt = 13 px); layouts are made at the
//! screen's scale, so glyphs are placed on device pixels.

use std::borrow::Cow;

use parley::fontique::{Blob, GenericFamily};
use parley::{FontContext, FontFamily, FontFamilyName, FontStyle, FontWeight, Layout, LayoutContext, LineHeight, StyleProperty};
use rapidr_value::objects::font::Font;

/// Text colour carried through a layout: 0xRRGGBB.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ink(pub u32);

const SANS: &[u8] = include_bytes!("../../rapidr-value/fonts/LiberationSans-Regular.ttf");
const SERIF: &[u8] = include_bytes!("../../rapidr-value/fonts/LiberationSerif-Regular.ttf");
const MONO: &[u8] = include_bytes!("../../rapidr-value/fonts/LiberationMono-Regular.ttf");

/// The font database and parley's scratch space, shared by every component
/// of every form (making either is costly).
pub struct TextSystem {
    pub font_cx: FontContext,
    pub layout_cx: LayoutContext<Ink>,
}

impl TextSystem {
    pub fn new() -> Self {
        let mut font_cx = FontContext::new();
        for data in [SANS, SERIF, MONO] {
            font_cx.collection.register_fonts(Blob::new(std::sync::Arc::new(data)), None);
        }
        TextSystem { font_cx, layout_cx: LayoutContext::new() }
    }

    /// One line of `text` in `font`, laid out at `scale` device pixels per
    /// logical pixel (positions in device pixels).
    pub fn layout(&mut self, text: &str, font: &Font, color: u32, scale: f32) -> Layout<Ink> {
        let mut builder = self.layout_cx.ranged_builder(&mut self.font_cx, text, scale, true);
        for prop in styles(font, color) {
            builder.push_default(prop);
        }
        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        layout
    }
}

/// The Liberation face standing for a QFONT's name, as
/// `rapidr_value::objects::text` picks it (Courier / mono: Mono; Times /
/// serif / Roman: Serif; anything else: Sans).
pub fn family(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    if ["courier", "mono", "fixed", "terminal", "console"].iter().any(|k| n.contains(k)) {
        "Liberation Mono"
    } else if ["times", "serif", "roman", "georgia"].iter().any(|k| n.contains(k)) {
        "Liberation Serif"
    } else {
        "Liberation Sans"
    }
}

/// A QFONT's size in logical pixels: points at 96 dpi rounded as Windows
/// does; a negative size is already pixels.
pub fn font_pixels(font: &Font) -> f32 {
    if font.size < 0 {
        (-font.size).min(512) as f32
    } else {
        ((font.size.clamp(1, 384) * 96 + 36) / 72) as f32
    }
}

/// The parley styles for a QFONT (fsBold = bit 0, fsItalic 1, fsUnderline
/// 2, fsStrikeOut 3) drawn in `color`.
pub fn styles(font: &Font, color: u32) -> Vec<StyleProperty<'static, Ink>> {
    let face = family(&font.name);
    // (then the system's fonts for what Liberation lacks: symbols, emoji)
    let names = vec![
        FontFamilyName::Named(Cow::Borrowed(face)),
        FontFamilyName::Generic(generic(face)),
        FontFamilyName::Generic(GenericFamily::SystemUi),
        FontFamilyName::Generic(GenericFamily::Emoji),
    ];
    let mut out = vec![
        StyleProperty::FontFamily(FontFamily::List(Cow::Owned(names))),
        StyleProperty::FontSize(font_pixels(font)),
        StyleProperty::LineHeight(LineHeight::MetricsRelative(1.0)),
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
        "Liberation Mono" => GenericFamily::Monospace,
        "Liberation Serif" => GenericFamily::Serif,
        _ => GenericFamily::SansSerif,
    }
}
