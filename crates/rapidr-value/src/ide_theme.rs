//! The themes extended for an IDE (RapidR Studio, docs/ide-plan.md I1):
//! the chrome the theme's system colours don't name — a tool bar, a status
//! bar that says what the program under development is doing, a start
//! page — and the code editor's colours, per theme. The docking chrome is
//! the dock manager's own (`crate::dock::look`).
//!
//! Every token is also readable by name from a program
//! (`Application.ThemeColor("statusbar.running")`, [`color_by_name`]), so
//! any RapidR program can draw chrome that follows the theme, as Studio's
//! shell does.
//!
//! - **classic** (Delphi 7 / Visual Basic 6): grey tool and status bars,
//!   VB6's code colours (the colours RCODEEDITOR always drew: classic
//!   programs look the same).
//! - **rapidr light** (Visual Studio 2022 / Xcode, light): flat bars on
//!   the window's face, a status bar in RapidR's blue (the theme's accent)
//!   that turns green while the program runs and orange while it is
//!   paused in the debugger.
//! - **rapidr dark**: the same, dark (the editor in Visual Studio Code's
//!   Dark+ colours).
//! - **rapidr high contrast**: black, white text and borders, the state said in
//!   words (never by colour alone), the editor in High Contrast Black's
//!   colours.

use crate::objects::font::Font;
use crate::theme::{Look, Theme};

/// The font of the chrome a theme draws that a program gives no font to
/// (menus, MDI windows' titles, tooltips, the dialogs the kernel draws —
/// message and input boxes, the colour and font dialogs): Windows'
/// (RapidQ's MS Sans Serif 8) in the classic look, Inter 10 pt (13 pixels)
/// in RapidR's (modern, dark, high contrast). A program's own components
/// keep their Font — and RapidQ's metrics — in every look.
pub fn chrome_font(t: &Theme) -> Font {
    if t.look == Look::Classic {
        Font::default()
    } else {
        Font { name: "Inter".into(), size: 10, color: 0, styles: 0 }
    }
}

/// A code editor's colours (0xRRGGBB) and styles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditorColors {
    pub background: u32,
    pub text: u32,
    pub gutter: u32,
    pub line_number: u32,
    pub current_line: u32,
    pub selection: u32,
    pub keyword: u32,
    pub string: u32,
    pub comment: u32,
    pub number: u32,
    pub directive: u32,
    /// Keywords bold (VB6's look).
    pub bold_keywords: bool,
    /// Comments italic.
    pub italic_comments: bool,
}

/// An IDE's chrome in a theme (0xRRGGBB).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IdeColors {
    /// The tool bar under the menu bar, its bottom line, the lines between
    /// its groups.
    pub toolbar: u32,
    pub toolbar_border: u32,
    pub separator: u32,
    /// The status bar at rest, and its text.
    pub status: u32,
    pub status_text: u32,
    /// The status bar while the program runs, is paused (debugging), or
    /// failed to build; the text on those.
    pub status_running: u32,
    pub status_debugging: u32,
    pub status_error: u32,
    pub status_state_text: u32,
    /// A start page: its ground, text, dimmed text, headings, links, and
    /// the cards on it with their borders.
    pub page: u32,
    pub page_text: u32,
    pub page_dim: u32,
    pub page_heading: u32,
    pub page_link: u32,
    pub card: u32,
    pub card_border: u32,
    pub editor: EditorColors,
}

/// The classic editor: RCODEEDITOR's colours since it came (VB6's).
const CLASSIC_EDITOR: EditorColors = EditorColors {
    background: 0xFFFFFF,
    text: 0x000000,
    gutter: 0xF0F0F0,
    line_number: 0x808080,
    current_line: 0xFFFFFF,
    selection: 0x0078D7,
    keyword: 0x0000B4,
    string: 0xA31515,
    comment: 0x008000,
    number: 0x800000,
    directive: 0x800080,
    bold_keywords: true,
    italic_comments: true,
};

/// Visual Studio 2022's light editor.
const MODERN_EDITOR: EditorColors = EditorColors {
    background: 0xFFFFFF,
    text: 0x1B1B1B,
    gutter: 0xFFFFFF,
    line_number: 0x8A8A8A,
    current_line: 0xF3F7FC,
    selection: 0xADD6FF,
    keyword: 0x0000FF,
    string: 0xA31515,
    comment: 0x008000,
    number: 0x098658,
    directive: 0xAF00DB,
    bold_keywords: false,
    italic_comments: false,
};

/// Visual Studio Code's Dark+.
const DARK_EDITOR: EditorColors = EditorColors {
    background: 0x1E1E1E,
    text: 0xD4D4D4,
    gutter: 0x1E1E1E,
    line_number: 0x858585,
    current_line: 0x282828,
    selection: 0x264F78,
    keyword: 0x569CD6,
    string: 0xCE9178,
    comment: 0x6A9955,
    number: 0xB5CEA8,
    directive: 0xC586C0,
    bold_keywords: false,
    italic_comments: false,
};

/// High Contrast Black's editor.
const CONTRAST_EDITOR: EditorColors = EditorColors {
    background: 0x000000,
    text: 0xFFFFFF,
    gutter: 0x000000,
    line_number: 0xFFFFFF,
    current_line: 0x000000,
    selection: 0x1AEBFF,
    keyword: 0x569CD6,
    string: 0xCE9178,
    comment: 0x7CA668,
    number: 0xB5CEA8,
    directive: 0xC586C0,
    bold_keywords: true,
    italic_comments: true,
};

/// The editor's colours in `t`.
pub fn editor(t: &Theme) -> EditorColors {
    if t.look == Look::Classic {
        CLASSIC_EDITOR
    } else if t.contrast {
        CONTRAST_EDITOR
    } else if t.dark {
        DARK_EDITOR
    } else {
        MODERN_EDITOR
    }
}

/// The IDE's chrome in `t`.
pub fn ide(t: &Theme) -> IdeColors {
    let editor = editor(t);
    if t.look == Look::Classic {
        // (Delphi 7: everything on the face, the state in words)
        return IdeColors {
            toolbar: t.face,
            toolbar_border: t.shadow,
            separator: t.shadow,
            status: t.face,
            status_text: t.text,
            status_running: t.face,
            status_debugging: t.face,
            status_error: t.face,
            status_state_text: t.text,
            page: t.window,
            page_text: t.text,
            page_dim: t.gray_text,
            page_heading: 0x0A246A,
            page_link: 0x0000EE,
            card: t.face,
            card_border: t.shadow,
            editor,
        };
    }
    if t.contrast {
        return IdeColors {
            toolbar: 0x000000,
            toolbar_border: 0xFFFFFF,
            separator: 0xFFFFFF,
            status: 0x000000,
            status_text: 0xFFFFFF,
            status_running: 0x000000,
            status_debugging: 0x000000,
            status_error: 0x000000,
            status_state_text: 0xFFFFFF,
            page: 0x000000,
            page_text: 0xFFFFFF,
            page_dim: 0xFFFFFF,
            page_heading: 0xFFFFFF,
            page_link: 0xFFFF00,
            card: 0x000000,
            card_border: 0xFFFFFF,
            editor,
        };
    }
    if t.dark {
        return IdeColors {
            toolbar: t.face,
            toolbar_border: 0x2B2B2B,
            separator: 0x3D3D3D,
            // (RapidR's blue itself: white reads on it)
            status: crate::theme::ACCENT,
            status_text: 0xFFFFFF,
            status_running: 0x0E7A0D,
            status_debugging: 0xB4500E,
            status_error: 0xC42B1C,
            status_state_text: 0xFFFFFF,
            page: 0x1E1E1E,
            page_text: 0xE6E6E6,
            page_dim: 0x9D9D9D,
            page_heading: 0xFFFFFF,
            page_link: t.accent,
            card: 0x2B2B2B,
            card_border: 0x3D3D3D,
            editor,
        };
    }
    IdeColors {
        toolbar: t.face,
        toolbar_border: 0xE5E5E5,
        separator: 0xD1D1D1,
        status: t.accent,
        status_text: 0xFFFFFF,
        status_running: 0x0F7B0F,
        status_debugging: 0xC4500E,
        status_error: 0xC42B1C,
        status_state_text: 0xFFFFFF,
        page: 0xFFFFFF,
        page_text: 0x1B1B1B,
        page_dim: 0x616161,
        page_heading: 0x1B1B1B,
        page_link: t.hot_text,
        card: 0xF9F9F9,
        card_border: 0xE5E5E5,
        editor,
    }
}

impl IdeColors {
    /// Every token by name, as `Application.ThemeColor` reads them.
    pub fn colors(&self) -> Vec<(&'static str, u32)> {
        let e = &self.editor;
        vec![
            ("toolbar", self.toolbar),
            ("toolbar.border", self.toolbar_border),
            ("separator", self.separator),
            ("statusbar", self.status),
            ("statusbar.text", self.status_text),
            ("statusbar.running", self.status_running),
            ("statusbar.debugging", self.status_debugging),
            ("statusbar.error", self.status_error),
            ("statusbar.statetext", self.status_state_text),
            ("page", self.page),
            ("page.text", self.page_text),
            ("page.dim", self.page_dim),
            ("page.heading", self.page_heading),
            ("page.link", self.page_link),
            ("card", self.card),
            ("card.border", self.card_border),
            ("editor.background", e.background),
            ("editor.text", e.text),
            ("editor.gutter", e.gutter),
            ("editor.linenumber", e.line_number),
            ("editor.currentline", e.current_line),
            ("editor.selection", e.selection),
            ("editor.keyword", e.keyword),
            ("editor.string", e.string),
            ("editor.comment", e.comment),
            ("editor.number", e.number),
            ("editor.directive", e.directive),
        ]
    }
}

/// A theme colour by name (0xRRGGBB): the theme's own tokens (`face`,
/// `accent`, `window` …, `Theme::colors`) and the IDE's ([`IdeColors::colors`]);
/// case, spaces, `-` and `_` ignored.
pub fn color_by_name(t: &Theme, name: &str) -> Option<u32> {
    let key = |s: &str| -> String { s.chars().filter(|c| !matches!(c, ' ' | '-' | '_' | '.')).flat_map(char::to_lowercase).collect() };
    let want = key(name);
    t.colors()
        .into_iter()
        .chain(ide(t).colors())
        .find(|(n, _)| key(n) == want)
        .map(|(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{contrast, ALL, CLASSIC};

    #[test]
    fn the_classic_editor_is_rcodeeditors_old_one() {
        let e = editor(&CLASSIC);
        assert_eq!((e.keyword, e.string, e.comment, e.number, e.text), (0x0000B4, 0xA31515, 0x008000, 0x800000, 0x000000));
        assert!(e.bold_keywords && e.italic_comments);
    }

    #[test]
    fn text_reads_on_every_ground() {
        for t in ALL {
            let c = ide(t);
            let e = c.editor;
            for (what, ink) in [("text", e.text), ("keyword", e.keyword), ("string", e.string), ("comment", e.comment), ("number", e.number), ("directive", e.directive)] {
                assert!(contrast(ink, e.background) >= 4.5, "{}: editor {what} {:06X} on {:06X}", t.name, ink, e.background);
            }
            for (what, ink, ground) in [
                ("status", c.status_text, c.status),
                ("running", c.status_state_text, c.status_running),
                ("debugging", c.status_state_text, c.status_debugging),
                ("error", c.status_state_text, c.status_error),
                ("page", c.page_text, c.page),
                ("link", c.page_link, c.page),
                ("heading", c.page_heading, c.page),
            ] {
                assert!(contrast(ink, ground) >= 4.5, "{}: {what} {:06X} on {:06X}", t.name, ink, ground);
            }
        }
    }

    #[test]
    fn colours_by_name() {
        assert_eq!(color_by_name(&CLASSIC, "Face"), Some(CLASSIC.face));
        assert_eq!(color_by_name(&crate::theme::RAPIDR_DARK, "statusbar.running"), Some(0x0E7A0D));
        assert_eq!(color_by_name(&crate::theme::RAPIDR, "Editor Keyword"), Some(0x0000FF));
        assert_eq!(color_by_name(&CLASSIC, "nope"), None);
    }
}
