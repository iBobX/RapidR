//! The code editor's colour schemes (RCODEEDITOR, RDIFFVIEW; docs/ide-plan.md
//! I2): one per theme — what the editor's text, gutter, selections, carets,
//! squiggles, popups and diffs are drawn in, and each token kind's colour,
//! weight and slant. A scheme names the token kinds of
//! `rapidr_editor::TokenKind`'s vocabulary (`keyword`, `string.escape` …)
//! and a kind it doesn't name falls back to its base (`type.component` →
//! `type`), then to the text's colour.
//!
//! - **classic**: RapidQ-era Windows — white, keywords dark blue and bold,
//!   strings red, comments green and italic, numbers maroon (the colours
//!   RCODEEDITOR always drew), the gutter on the button face.
//! - **modern**: a light, quiet scheme on white (no bold, comments
//!   italic), the gutter on the window's grey.
//! - **dark**: the dark theme's, on near-black.
//! - **highcontrast**: Windows' high contrast black: every colour ≥ 7:1 on
//!   black, the current line framed instead of tinted.
//!
//! Every token colour of every scheme has a WCAG contrast of at least 4.5
//! on its background (7 in high contrast): a test checks it.

use crate::theme::{self, Theme};

/// How a token kind is drawn: its colour (0xRRGGBB), bold, italic,
/// underlined (`invalid`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenStyle {
    pub color: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

const fn plain(color: u32) -> TokenStyle {
    TokenStyle { color, bold: false, italic: false, underline: false }
}
const fn bold(color: u32) -> TokenStyle {
    TokenStyle { color, bold: true, italic: false, underline: false }
}
const fn italic(color: u32) -> TokenStyle {
    TokenStyle { color, bold: false, italic: true, underline: false }
}
const fn wavy(color: u32) -> TokenStyle {
    TokenStyle { color, bold: false, italic: false, underline: true }
}

/// A theme's code colours (0xRRGGBB).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scheme {
    /// Its name (`ColorScheme` reads it): the theme's.
    pub name: &'static str,
    pub dark: bool,
    pub background: u32,
    /// Text no token colours.
    pub text: u32,
    pub gutter: u32,
    pub line_number: u32,
    /// The caret's line's number.
    pub line_number_active: u32,
    /// The caret's line (`current_line_frame`: framed instead of filled).
    pub current_line: u32,
    pub current_line_frame: bool,
    pub selection: u32,
    /// The selection while another component has the focus.
    pub selection_inactive: u32,
    /// Other places with the selected text (Ctrl+D's candidates).
    pub selection_echo: u32,
    pub caret: u32,
    /// Spaces and tabs drawn (ShowWhitespace), indent guides, rulers.
    pub whitespace: u32,
    pub indent_guide: u32,
    pub ruler: u32,
    /// The bracket matching the caret's: a fill and a frame.
    pub bracket: u32,
    pub bracket_frame: u32,
    /// Find's matches and the current one.
    pub find_match: u32,
    pub find_current: u32,
    /// Squiggles (and their gutter / scroll-bar marks).
    pub error: u32,
    pub warning: u32,
    pub info: u32,
    pub hint: u32,
    /// A folded line's "…" box.
    pub fold_box: u32,
    pub fold_arrow: u32,
    /// Breakpoints, the debugger's current line, bookmarks.
    pub breakpoint: u32,
    pub current_statement: u32,
    pub current_statement_line: u32,
    pub bookmark: u32,
    /// The minimap's view slider.
    pub minimap_slider: u32,
    /// Popups (completion, hover, signature) and the find box.
    pub popup: u32,
    pub popup_border: u32,
    pub popup_text: u32,
    pub popup_detail: u32,
    pub popup_selected: u32,
    pub popup_selected_text: u32,
    /// The typed part of a completion's label.
    pub popup_match: u32,
    /// The active parameter in a signature.
    pub popup_param: u32,
    /// Diffs: whole lines added / removed, the changed characters in
    /// them, the gutter marks of a modified editor (added, changed,
    /// removed).
    pub diff_added: u32,
    pub diff_removed: u32,
    pub diff_added_text: u32,
    pub diff_removed_text: u32,
    pub mark_added: u32,
    pub mark_changed: u32,
    pub mark_removed: u32,
    /// Token kinds → styles, most specific names first.
    pub tokens: &'static [(&'static str, TokenStyle)],
}

impl Scheme {
    /// The style of a token of kind `fallbacks` (most specific name first,
    /// as `TokenKind::fallbacks` gives them).
    pub fn token<'a>(&self, fallbacks: impl IntoIterator<Item = &'a str>) -> TokenStyle {
        for name in fallbacks {
            if let Some((_, s)) = self.tokens.iter().find(|(n, _)| *n == name) {
                return *s;
            }
        }
        plain(self.text)
    }

    /// The style of the kind called `name` (`keyword.control`), falling
    /// back along its dots.
    pub fn token_named(&self, name: &str) -> TokenStyle {
        let mut names = vec![name];
        let mut rest = name;
        while let Some(i) = rest.rfind('.') {
            rest = &name[..i];
            names.push(rest);
        }
        self.token(names)
    }
}

/// RapidQ-era Windows: the colours RCODEEDITOR drew before schemes.
pub const CLASSIC: Scheme = Scheme {
    name: "classic",
    dark: false,
    background: 0xFFFFFF,
    text: 0x000000,
    gutter: 0xF0F0F0,
    line_number: 0x6E6E6E,
    line_number_active: 0x000000,
    current_line: 0xF2F6FC,
    current_line_frame: false,
    selection: 0xADD6FF,
    selection_inactive: 0xE5EBF1,
    selection_echo: 0xD7E8FA,
    caret: 0x000000,
    whitespace: 0xBFBFBF,
    indent_guide: 0xE3E3E3,
    ruler: 0xE0E0E0,
    bracket: 0xDCEBDC,
    bracket_frame: 0x7FA37F,
    find_match: 0xFFE9A8,
    find_current: 0xF5B83D,
    error: 0xE51400,
    warning: 0xBF8803,
    info: 0x1A85FF,
    hint: 0x6E6E6E,
    fold_box: 0xE8E8E8,
    fold_arrow: 0x6E6E6E,
    breakpoint: 0xE51400,
    current_statement: 0xFFCC00,
    current_statement_line: 0xFFF5C2,
    bookmark: 0x1A85FF,
    minimap_slider: 0xC8C8C8,
    popup: 0xFFFFFF,
    popup_border: 0x808080,
    popup_text: 0x000000,
    popup_detail: 0x5A5A5A,
    popup_selected: 0x0067C0,
    popup_selected_text: 0xFFFFFF,
    popup_match: 0x0000B4,
    popup_param: 0x0000B4,
    diff_added: 0xE6F4E6,
    diff_removed: 0xFBE9E9,
    diff_added_text: 0xB8E2B8,
    diff_removed_text: 0xF4BDBD,
    mark_added: 0x2EA043,
    mark_changed: 0x1A85FF,
    mark_removed: 0xE51400,
    tokens: &[
        ("keyword", bold(0x0000B4)),
        ("type", bold(0x0000B4)),
        ("constant", plain(0x0000B4)),
        ("function", plain(0x000000)),
        ("string.escape", plain(0x7A0000)),
        ("string", plain(0xA31515)),
        ("comment", italic(0x007A00)),
        ("number", plain(0x800000)),
        ("directive", plain(0x7A3E9D)),
        ("label", plain(0x7A3E00)),
        ("invalid", wavy(0xC80000)),
    ],
};

/// The modern theme's: quiet, light, no bold.
pub const MODERN: Scheme = Scheme {
    name: "modern",
    dark: false,
    background: 0xFFFFFF,
    text: 0x1F2328,
    gutter: 0xFFFFFF,
    line_number: 0x8C959F,
    line_number_active: 0x1F2328,
    current_line: 0xF5F8FC,
    current_line_frame: false,
    selection: 0xCCE2FB,
    selection_inactive: 0xE8EDF3,
    selection_echo: 0xE3EEFB,
    caret: 0x0B57D0,
    whitespace: 0xC4CAD1,
    indent_guide: 0xEBEEF1,
    ruler: 0xEBEEF1,
    bracket: 0xE1F0E3,
    bracket_frame: 0x8FBF96,
    find_match: 0xFFF0B3,
    find_current: 0xF9C846,
    error: 0xD1242F,
    warning: 0xB07800,
    info: 0x0969DA,
    hint: 0x8C959F,
    fold_box: 0xEEF1F4,
    fold_arrow: 0x6E7781,
    breakpoint: 0xD1242F,
    current_statement: 0xF2B705,
    current_statement_line: 0xFFF6D6,
    bookmark: 0x0969DA,
    minimap_slider: 0xD6DBE1,
    popup: 0xFFFFFF,
    popup_border: 0xD0D7DE,
    popup_text: 0x1F2328,
    popup_detail: 0x656D76,
    popup_selected: 0xDDEBFB,
    popup_selected_text: 0x1F2328,
    popup_match: 0x0B57D0,
    popup_param: 0x0B57D0,
    diff_added: 0xE9F6EC,
    diff_removed: 0xFDECEC,
    diff_added_text: 0xBDE5C6,
    diff_removed_text: 0xF8C7C7,
    mark_added: 0x2DA44E,
    mark_changed: 0x0969DA,
    mark_removed: 0xCF222E,
    tokens: &[
        ("keyword.control", plain(0x8E24AA)),
        ("keyword.operator", plain(0x0B57D0)),
        ("keyword", plain(0x0B57D0)),
        ("type.component", plain(0x00796B)),
        ("type", plain(0x00796B)),
        ("function", plain(0x7A4B00)),
        ("variable.parameter", plain(0x9C4221)),
        ("variable.property", plain(0x1F5F99)),
        ("variable", plain(0x1F2328)),
        ("constant", plain(0x005CC5)),
        ("number", plain(0x0E7C3A)),
        ("string.escape", plain(0x9A3412)),
        ("string", plain(0xB3261E)),
        ("comment", italic(0x56705A)),
        ("operator", plain(0x57606A)),
        ("punctuation", plain(0x57606A)),
        ("directive", plain(0x8E24AA)),
        ("label", plain(0xA3550C)),
        ("invalid", wavy(0xC62828)),
    ],
};

/// The dark theme's.
pub const DARK: Scheme = Scheme {
    name: "dark",
    dark: true,
    background: 0x1E1E1E,
    text: 0xD4D4D4,
    gutter: 0x1E1E1E,
    line_number: 0x6E7681,
    line_number_active: 0xD4D4D4,
    current_line: 0x282828,
    current_line_frame: false,
    selection: 0x1F3F5F,
    selection_inactive: 0x3A3D41,
    selection_echo: 0x2B3A4A,
    caret: 0xAEAFAD,
    whitespace: 0x4B4B4B,
    indent_guide: 0x333333,
    ruler: 0x333333,
    bracket: 0x2D3B2D,
    bracket_frame: 0x7A9F7A,
    find_match: 0x5A4A1E,
    find_current: 0x8A6A12,
    error: 0xF14C4C,
    warning: 0xCCA700,
    info: 0x3794FF,
    hint: 0x8B949E,
    fold_box: 0x333333,
    fold_arrow: 0x9DA5B4,
    breakpoint: 0xE51400,
    current_statement: 0xFFCC00,
    current_statement_line: 0x3D3A1E,
    bookmark: 0x3794FF,
    minimap_slider: 0x4A4A4A,
    popup: 0x252526,
    popup_border: 0x454545,
    popup_text: 0xCCCCCC,
    popup_detail: 0x9DA5B4,
    popup_selected: 0x04395E,
    popup_selected_text: 0xFFFFFF,
    popup_match: 0x4FC1FF,
    popup_param: 0x4FC1FF,
    diff_added: 0x1F3324,
    diff_removed: 0x3A1F22,
    diff_added_text: 0x2C5A35,
    diff_removed_text: 0x6B2A2F,
    mark_added: 0x2EA043,
    mark_changed: 0x3794FF,
    mark_removed: 0xF85149,
    tokens: &[
        ("keyword.control", plain(0xC586C0)),
        ("keyword.operator", plain(0x569CD6)),
        ("keyword", plain(0x569CD6)),
        ("type.component", plain(0x4EC9B0)),
        ("type", plain(0x4EC9B0)),
        ("function", plain(0xDCDCAA)),
        ("variable.parameter", plain(0x9CDCFE)),
        ("variable.property", plain(0x9CDCFE)),
        ("variable", plain(0x9CDCFE)),
        ("constant", plain(0x4FC1FF)),
        ("number", plain(0xB5CEA8)),
        ("string.escape", plain(0xD7BA7D)),
        ("string", plain(0xCE9178)),
        ("comment", italic(0x7FAE6A)),
        ("operator", plain(0xD4D4D4)),
        ("punctuation", plain(0xABB2BF)),
        ("directive", plain(0xC586C0)),
        ("label", plain(0xD7BA7D)),
        ("invalid", wavy(0xFF6B6B)),
    ],
};

/// Windows' high contrast black: every colour ≥ 7:1 on black.
pub const HIGH_CONTRAST: Scheme = Scheme {
    name: "highcontrast",
    dark: true,
    background: 0x000000,
    text: 0xFFFFFF,
    gutter: 0x000000,
    line_number: 0xC0C0C0,
    line_number_active: 0xFFFF00,
    current_line: 0xFFFFFF,
    current_line_frame: true,
    selection: 0x0F4A85,
    selection_inactive: 0x303030,
    selection_echo: 0x1B2B3B,
    caret: 0xFFFFFF,
    whitespace: 0x7F7F7F,
    indent_guide: 0x5A5A5A,
    ruler: 0x5A5A5A,
    bracket: 0x000000,
    bracket_frame: 0x1AEBFF,
    find_match: 0x4D3D00,
    find_current: 0x806600,
    error: 0xFF5555,
    warning: 0xFFD700,
    info: 0x1AEBFF,
    hint: 0xC0C0C0,
    fold_box: 0x303030,
    fold_arrow: 0xFFFFFF,
    breakpoint: 0xFF3B3B,
    current_statement: 0xFFFF00,
    current_statement_line: 0x3D3A00,
    bookmark: 0x1AEBFF,
    minimap_slider: 0x7F7F7F,
    popup: 0x000000,
    popup_border: 0xFFFFFF,
    popup_text: 0xFFFFFF,
    popup_detail: 0xC0C0C0,
    popup_selected: 0x1AEBFF,
    popup_selected_text: 0x000000,
    popup_match: 0xFFFF00,
    popup_param: 0xFFFF00,
    diff_added: 0x0B3D0B,
    diff_removed: 0x4A0B0B,
    diff_added_text: 0x1E6B1E,
    diff_removed_text: 0x8A1A1A,
    mark_added: 0x3FF23F,
    mark_changed: 0x1AEBFF,
    mark_removed: 0xFF5555,
    tokens: &[
        ("keyword.control", bold(0xFF9EF0)),
        ("keyword", bold(0x6FD6FF)),
        ("type", plain(0x7CFFB2)),
        ("function", plain(0xFFFF66)),
        ("variable.parameter", plain(0xBDE6FF)),
        ("variable.property", plain(0xBDE6FF)),
        ("constant", plain(0x9FD8FF)),
        ("number", plain(0xB5F5A0)),
        ("string.escape", plain(0xFFE0A3)),
        ("string", plain(0xFFC27A)),
        ("comment", italic(0x7CE07C)),
        ("operator", plain(0xFFFFFF)),
        ("punctuation", plain(0xFFFFFF)),
        ("directive", plain(0xFF9EF0)),
        ("label", plain(0xFFD27A)),
        ("invalid", wavy(0xFF7B7B)),
    ],
};

pub const ALL: [&Scheme; 4] = [&CLASSIC, &MODERN, &DARK, &HIGH_CONTRAST];

/// The scheme for `theme`.
pub fn for_theme(theme: &Theme) -> &'static Scheme {
    // (a theme without a scheme of its own — RapidR's coming RAPIDR light,
    // dark and high contrast — takes the one of its kind: the modern
    // schemes are RapidR's own)
    by_name(theme.name).unwrap_or_else(|| {
        if theme.name.contains("contrast") || theme.name.ends_with("hc") {
            &HIGH_CONTRAST
        } else if theme.dark {
            &DARK
        } else {
            &MODERN
        }
    })
}

/// The scheme called `name` (a theme's name, any case).
pub fn by_name(name: &str) -> Option<&'static Scheme> {
    ALL.iter().copied().find(|s| s.name.eq_ignore_ascii_case(name))
}

/// The scheme an editor draws with: `chosen` (`ColorScheme`: a scheme's
/// name), else (`""`, `"auto"`) the current theme's.
pub fn resolve(chosen: &str) -> &'static Scheme {
    match chosen.trim() {
        "" => for_theme(theme::current()),
        c if c.eq_ignore_ascii_case("auto") => for_theme(theme::current()),
        c => by_name(c).unwrap_or_else(|| for_theme(theme::current())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::contrast;

    #[test]
    fn every_theme_has_its_scheme() {
        for t in theme::ALL {
            assert_eq!(for_theme(t).name, t.name);
        }
        assert_eq!(resolve("Dark").name, "dark");
        assert_eq!(MODERN.token_named("type.component").color, 0x00796B);
        assert_eq!(MODERN.token_named("keyword.tag").color, MODERN.token_named("keyword").color);
        assert_eq!(MODERN.token_named("nothing").color, MODERN.text);
    }

    #[test]
    fn every_token_is_readable() {
        let mut bad = Vec::new();
        let mut check = |what: String, fg: u32, bg: u32, min: f64| {
            let c = contrast(fg, bg);
            if c < min {
                bad.push(format!("{what}: #{fg:06x} on #{bg:06x} is {c:.2}:1 (< {min})"));
            }
        };
        for s in ALL {
            let min = if s.name == "highcontrast" { 7.0 } else { 4.5 };
            for (name, st) in s.tokens {
                check(format!("{}: {name}", s.name), st.color, s.background, min);
                check(format!("{}: {name} on the current line", s.name), st.color, s.current_line_or_bg(), min);
                // (text stays readable on the selection: it isn't recoloured)
                check(format!("{}: {name} on the selection", s.name), st.color, s.selection, 3.0);
            }
            check(format!("{}: text", s.name), s.text, s.background, min);
            check(format!("{}: line numbers", s.name), s.line_number, s.gutter, 3.0);
            check(format!("{}: popups", s.name), s.popup_text, s.popup, min);
            check(format!("{}: popup details", s.name), s.popup_detail, s.popup, 4.5);
            check(format!("{}: the selected completion", s.name), s.popup_selected_text, s.popup_selected, min);
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    impl Scheme {
        fn current_line_or_bg(&self) -> u32 {
            if self.current_line_frame { self.background } else { self.current_line }
        }
    }
}
