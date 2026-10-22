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
//! - **classic** (the classic Windows look): grey tool and status bars,
//!   the classic code colours (the colours RCODEEDITOR always drew: classic
//!   programs look the same).
//! - **rapidr light**: flat bars on
//!   the window's face, a status bar in RapidR's blue (the theme's accent)
//!   that turns green while the program runs and orange while it is
//!   paused in the debugger.
//! - **rapidr dark**: the same, dark (the editor in its
//!   own dark colours).
//! - **rapidr high contrast**: black, white text and borders, the state said in
//!   words (never by colour alone), the editor in High Contrast Black's
//!   colours.

use crate::objects::font::Font;
use crate::theme::{brand, mix, Look, Theme};

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
    /// Keywords bold (the classic look).
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
    /// A debugger's value that changed since the program last stopped, on
    /// the `window` ground (Variables, Watch): the code editor's data tips'
    /// mark (`code_scheme`'s `mark_changed`), Windows' blue in the classic
    /// look.
    pub changed: u32,
    /// The debugger's values as Variables, Watch, the data tips and the
    /// Properties panel draw them ([`ValueColors`]).
    pub values: ValueColors,
    pub editor: EditorColors,
}

/// A value's parts as the debugger draws them (0xRRGGBB, on the `window`
/// ground): its name, the ` = `, its value styled by kind — the code
/// editor's own token colours (`code_scheme`), so a number in Watch is the
/// colour it is in the code — and the quiet parts (an empty text's `""`, an
/// array's count). Each reads at 4.5:1 on the window (7:1 in high contrast):
/// the theme's contrast tests check them all. A selected row draws in its
/// selection's text colour instead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueColors {
    pub name: u32,
    /// ` = `, ` · ` between a value's parts.
    pub punctuation: u32,
    pub number: u32,
    /// A text (STRING) value, quotes and all.
    pub string: u32,
    /// True and False.
    pub keyword: u32,
    /// A constant's name (`clWindowText`, `alClient`).
    pub constant: u32,
    /// A type's name (`Array(0 TO 64)`, a TYPE's, a component's).
    pub type_name: u32,
    /// The quiet parts: `""`, `· 65 items`, Empty.
    pub dim: u32,
    /// A value changed since the program last stopped (`debug.changed`):
    /// its mark, and its highlight's frame in high contrast.
    pub changed: u32,
    /// The highlight behind a changed value (`debug.changedback`): the
    /// changed colour, faint, on the window — every part still reads on it.
    /// High contrast frames the value instead (this is the window).
    pub changed_back: u32,
}

/// The debugger's value colours in `t` ([`ValueColors`]).
pub fn values(t: &Theme) -> ValueColors {
    let code = crate::code_scheme::for_theme(t);
    let tok = |name: &str| code.token_named(name).color;
    let changed = if t.look == Look::Classic {
        0x0066CC
    } else if t.contrast {
        crate::code_scheme::HIGH_CONTRAST.mark_changed
    } else {
        code.mark_changed
    };
    ValueColors {
        name: t.text,
        punctuation: t.secondary_text(),
        number: tok("number"),
        string: tok("string"),
        keyword: tok("keyword"),
        constant: tok("constant"),
        type_name: tok("type"),
        dim: t.secondary_text(),
        changed,
        changed_back: if t.contrast {
            t.window
        } else {
            mix(t.window, changed, if t.dark { 150 } else { 80 })
        },
    }
}

/// The classic editor: RCODEEDITOR's colours since it came (the classic look's).
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

/// RapidR's light editor: white, Ink text, the brand's deep blue and
/// teal for keywords and comments.
const MODERN_EDITOR: EditorColors = EditorColors {
    background: 0xFFFFFF,
    text: brand::INK,
    gutter: 0xFFFFFF,
    line_number: 0x8A93A6,
    current_line: 0xF3F6FC,
    selection: mix(0xFFFFFF, brand::BLUE, 200),
    keyword: brand::BLUE_DEEP,
    string: 0xA31515,
    comment: brand::TEAL_DEEP,
    number: 0x8A4B08,
    directive: 0x8B2FC9,
    bold_keywords: false,
    italic_comments: false,
};

/// RapidR's dark editor: the Ink ground, Blue on Dark keywords.
const DARK_EDITOR: EditorColors = EditorColors {
    background: mix(brand::INK, 0xFFFFFF, 15),
    text: brand::BOARD,
    gutter: mix(brand::INK, 0xFFFFFF, 15),
    line_number: 0x6B7590,
    current_line: mix(brand::INK, 0xFFFFFF, 50),
    selection: mix(brand::INK, brand::BLUE, 400),
    keyword: brand::BLUE_ON_DARK,
    string: 0xE8A07A,
    comment: 0x5FBF9A,
    number: brand::AMBER,
    directive: 0xC792EA,
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
        // (classic: everything on the face, the state in words)
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
            changed: 0x0066CC,
            values: values(t),
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
            changed: crate::code_scheme::HIGH_CONTRAST.mark_changed,
            values: values(t),
            editor,
        };
    }
    // RapidR's look, light and dark: the bars on the window's surface, a
    // hairline under them; the status bar quiet at rest (the surface, the
    // secondary text), in a state's colour while the program runs, is
    // paused or failed; the start page on the fields' ground, its cards
    // raised like buttons.
    let (running, debugging, error) = (brand::TEAL_DEEP, 0xB45309, 0xC42B1C);
    // (the theme's secondary text: Slate, and its dark counterpart)
    let dim = t.secondary_text();
    IdeColors {
        toolbar: t.face,
        toolbar_border: t.border,
        separator: t.border,
        status: t.face,
        status_text: dim,
        status_running: running,
        status_debugging: debugging,
        status_error: error,
        status_state_text: 0xFFFFFF,
        page: t.window,
        page_text: t.text,
        page_dim: dim,
        page_heading: t.text,
        page_link: t.hot_text,
        card: t.control,
        card_border: t.border,
        changed: crate::code_scheme::for_theme(t).mark_changed,
        values: values(t),
        editor,
    }
}

impl IdeColors {
    /// Every token by name, as `Application.ThemeColor` reads them.
    pub fn colors(&self) -> Vec<(&'static str, u32)> {
        let e = &self.editor;
        let v = &self.values;
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
            ("debug.changed", self.changed),
            ("debug.name", v.name),
            ("debug.punctuation", v.punctuation),
            ("debug.number", v.number),
            ("debug.string", v.string),
            ("debug.keyword", v.keyword),
            ("debug.constant", v.constant),
            ("debug.type", v.type_name),
            ("debug.dim", v.dim),
            ("debug.changedback", v.changed_back),
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
                ("a changed value", c.changed, t.window),
            ] {
                assert!(contrast(ink, ground) >= 4.5, "{}: {what} {:06X} on {:06X}", t.name, ink, ground);
            }
        }
    }

    /// The debugger's values (Variables, Watch, the data tips, the
    /// Properties panel: the UI kernel's value_view) read on the window
    /// they're drawn on: AA, AAA in high contrast — the contrast gate's
    /// entries for the `debug.*` tokens.
    #[test]
    fn debugger_values_read_on_the_window() {
        for t in ALL {
            let v = values(t);
            assert_eq!(v, ide(t).values);
            let need = if t.contrast { 7.0 } else { 4.5 };
            for (what, ink) in [
                ("name", v.name),
                ("punctuation", v.punctuation),
                ("number", v.number),
                ("string", v.string),
                ("keyword", v.keyword),
                ("constant", v.constant),
                ("type", v.type_name),
                ("dim", v.dim),
                ("changed", v.changed),
            ] {
                assert!(contrast(ink, t.window) >= need, "{}: debug.{what} {:06X} on {:06X}: {:.2}", t.name, ink, t.window, contrast(ink, t.window));
                // (a changed value keeps its colours on its highlight; its
                // mark, a glyph, needs 3:1)
                let on_back = if what == "changed" { 3.0 } else { need };
                assert!(contrast(ink, v.changed_back) >= on_back, "{}: debug.{what} {:06X} on the changed highlight {:06X}: {:.2}", t.name, ink, v.changed_back, contrast(ink, v.changed_back));
            }
            // (the highlight shows: not the window)
            assert!(t.contrast || v.changed_back != t.window, "{}", t.name);
            // (the kinds stay apart: a number isn't drawn as text)
            assert!(v.number != v.string && v.string != v.keyword, "{}", t.name);
        }
        assert_eq!(color_by_name(&crate::theme::RAPIDR, "debug.number"), Some(values(&crate::theme::RAPIDR).number));
    }

    #[test]
    fn colours_by_name() {
        assert_eq!(color_by_name(&CLASSIC, "Face"), Some(CLASSIC.face));
        assert_eq!(color_by_name(&crate::theme::RAPIDR_DARK, "statusbar.running"), Some(brand::TEAL_DEEP));
        assert_eq!(color_by_name(&crate::theme::RAPIDR, "Editor Keyword"), Some(brand::BLUE_DEEP));
        assert_eq!(color_by_name(&CLASSIC, "nope"), None);
    }
}
