//! What RapidR Studio's panels share when they draw: one look across the
//! inspector, the toolbox, the project tree, the console, the toolbar and
//! the palette — Xcode / Xojo-class in the modern and dark themes, Delphi's
//! in the classic one, black, white and the accent in high contrast —
//! every colour taken from the theme ([`Look`]: the theme's tokens, and the
//! icon palette's hues for errors, warnings and badges), never chosen here.
//!
//! The pieces: the panel's ground, a search box, a tab strip, a category
//! heading, a row's hover / selection, a badge, text cut with "…" to fit,
//! text with the search's matched letters marked, icons.

use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::theme::{Look as Style, Theme};

use crate::paint::Painter;

/// A panel's colours in a theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub classic: bool,
    pub contrast: bool,
    pub dark: bool,
    /// The list's ground (rows).
    pub body: u32,
    /// Strips around the list: tabs, the search strip, a toolbar.
    pub chrome: u32,
    pub text: u32,
    /// A value at its default, a secondary text (a shortcut, a path).
    pub dim: u32,
    /// The search box's placeholder.
    pub faint: u32,
    pub disabled: u32,
    /// Lines between rows and columns.
    pub line: u32,
    /// Borders: the search box's, a panel's edge.
    pub border: u32,
    /// A row under the mouse.
    pub hover: u32,
    /// The selected row (the list has the focus) and its text.
    pub selected: u32,
    pub selected_text: u32,
    /// The selected row when the list hasn't the focus.
    pub inactive: u32,
    pub inactive_text: u32,
    pub accent: u32,
    pub accent_text: u32,
    /// A category's heading and its text.
    pub section: u32,
    pub section_text: u32,
    /// A text box's fill, its border, its border with the focus.
    pub field: u32,
    pub field_border: u32,
    pub field_focus: u32,
    /// The "R" badge of RapidR's extensions.
    pub badge: u32,
    pub badge_text: u32,
    /// A link (`file:line`).
    pub link: u32,
    /// The search's matched letters.
    pub mark: u32,
    /// Behind a found text (the console's search).
    pub found: u32,
    pub error: u32,
    pub warning: u32,
    pub info: u32,
    pub ok: u32,
    /// The keyboard focus' ring or dotted rectangle.
    pub focus: u32,
    /// A raised card's shadow (the command palette's, a drag's ghost):
    /// mixed toward what it falls on.
    pub shadow: u32,
}

/// `a` toward `b` by `t` (0..=1).
pub fn mix(a: u32, b: u32, t: f64) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (((a >> s) & 0xFF) as f64, ((b >> s) & 0xFF) as f64);
        ((x + (y - x) * t).round() as u32 & 0xFF) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// A hue of the icon palette (RapidR's brand colours, per theme:
/// rapidr_icons' tokens — `red`, `amber`, `blue`, `violet` …).
fn hue(t: &Theme, token: &str) -> u32 {
    rapidr_icons::palette(t.name).token(token).unwrap_or(t.text)
}

/// The panels' colours in theme `t`.
pub fn look(t: &Theme) -> Look {
    let contrast = t.name == "highcontrast";
    let classic = t.look == Style::Classic;
    let (error, warning, info, ok) = (hue(t, "red"), hue(t, "amber"), hue(t, "blue"), hue(t, "teal"));
    if contrast {
        return Look {
            classic,
            contrast,
            dark: true,
            body: t.window,
            chrome: t.face,
            text: t.text,
            dim: t.text,
            faint: t.gray_text,
            disabled: t.gray_text,
            line: t.grid_lines,
            border: t.border,
            hover: t.window,
            selected: t.highlight,
            selected_text: t.highlight_text,
            inactive: t.unfocused,
            inactive_text: t.text,
            accent: t.accent,
            accent_text: t.accent_text,
            section: t.face,
            section_text: t.text,
            field: t.window,
            field_border: t.border,
            field_focus: t.focus,
            badge: t.accent,
            badge_text: t.accent_text,
            link: t.hot_text,
            mark: t.hot_text,
            found: t.highlight,
            error,
            warning,
            info,
            ok,
            focus: t.focus,
            shadow: t.window,
        };
    }
    if classic {
        return Look {
            classic,
            contrast,
            dark: false,
            body: t.window,
            chrome: t.face,
            text: t.text,
            dim: t.gray_text,
            faint: t.gray_text,
            disabled: t.gray_text,
            line: t.grid_lines,
            border: t.shadow,
            hover: t.view_hot,
            selected: t.highlight,
            selected_text: t.highlight_text,
            inactive: t.unfocused_strong,
            inactive_text: t.text,
            accent: t.highlight,
            accent_text: t.highlight_text,
            section: t.face,
            section_text: t.text,
            field: t.window,
            field_border: t.shadow,
            field_focus: t.highlight,
            badge: hue(t, "violet"),
            badge_text: t.highlight_text,
            link: t.hot_text,
            mark: t.hot_text,
            found: mix(hue(t, "amber"), t.window, 0.55),
            error,
            warning,
            info,
            ok,
            focus: t.focus,
            shadow: t.dark_shadow,
        };
    }
    Look {
        classic,
        contrast,
        dark: t.dark,
        body: t.window,
        chrome: t.face,
        text: t.text,
        dim: t.gray_text,
        faint: mix(t.gray_text, t.window, 0.25),
        disabled: t.gray_text,
        line: t.grid_lines,
        border: t.border,
        hover: t.view_hot,
        selected: t.selected,
        selected_text: t.selected_text,
        inactive: t.unfocused,
        inactive_text: t.text,
        accent: t.accent,
        accent_text: t.accent_text,
        section: mix(t.face, t.window, 0.35),
        section_text: t.text,
        field: t.control,
        field_border: t.border,
        field_focus: t.accent,
        badge: hue(t, "violet"),
        badge_text: t.window,
        link: t.hot_text,
        mark: t.hot_text,
        found: mix(hue(t, "amber"), t.window, if t.dark { 0.55 } else { 0.6 }),
        error,
        warning,
        info,
        ok,
        focus: t.focus,
        shadow: t.dark_shadow,
    }
}

/// The panel's ground: the list's colour, and a classic theme's sunken
/// edge around it (a fluent theme's thin border).
pub fn ground(p: &mut Painter, w: i64, h: i64, l: &Look) {
    p.fill((0, 0, w, h), l.body);
    if l.classic {
        p.sunken_edge((0, 0, w, h));
    }
}

/// Where the inside of a panel starts (inside a classic theme's edge).
pub fn inset(l: &Look) -> i64 {
    if l.classic {
        2
    } else {
        0
    }
}

/// A horizontal line across `w` at `y` (a strip's bottom).
pub fn hline(p: &mut Painter, x: i64, y: i64, w: i64, color: u32) {
    p.fill((x, y, w, 1), color);
}

/// A row's background for its state; the colour its text is drawn in.
pub fn row(p: &mut Painter, rect: Rect, l: &Look, selected: bool, focused: bool, hover: bool) -> u32 {
    let (x, y, w, h) = rect;
    if selected {
        let (fill, ink) = if focused { (l.selected, l.selected_text) } else { (l.inactive, l.inactive_text) };
        if l.classic {
            p.fill(rect, fill);
        } else {
            p.round((x + 2, y + 1, w - 4, h - 2), 4.0, Some(fill), None, 1.0);
            if focused && !l.contrast {
                let mark = (h - 10).clamp(3, 14);
                p.round((x + 2, y + (h - mark) / 2, 3, mark), 1.5, Some(l.accent), None, 1.0);
            }
        }
        return ink;
    }
    if hover {
        if l.contrast {
            p.ring((x + 1, y + 1, w - 2, h - 2), 3.0, l.link, 1.0);
        } else if l.classic {
            p.fill(rect, l.hover);
        } else {
            p.round((x + 2, y + 1, w - 4, h - 2), 4.0, Some(l.hover), None, 1.0);
        }
    }
    l.text
}

/// The keyboard focus on a row (inside `rect`).
pub fn row_focus(p: &mut Painter, rect: Rect, l: &Look) {
    let (x, y, w, h) = rect;
    if l.classic {
        p.focus((x, y, w, h));
    } else {
        p.ring((x + 2, y + 1, w - 4, h - 2), 4.0, l.focus, if l.contrast { 2.0 } else { 1.0 });
    }
}

/// A category's heading across `rect`: its open / closed chevron (or the
/// classic ±), its title in bold, a count after it.
pub fn section(p: &mut Painter, rect: Rect, l: &Look, title: &str, open: bool, count: Option<usize>, font: &Font) {
    let (x, y, w, h) = rect;
    p.fill(rect, l.section);
    if l.classic {
        p.fill((x, y + h - 1, w, 1), l.line);
    }
    let cx = x + 10;
    let mid = y + h / 2;
    if l.classic {
        super::super::tree::expander(p, cx, mid, 9, open);
    } else {
        p.chevron(cx as f64 + 0.5, mid as f64 + 0.5, 7.0, open, l.dim);
    }
    let bold = Font { styles: font.styles | 1, ..font.clone() };
    let tx = x + 22;
    let (tw, _) = text_size(title, &bold);
    p.text((tx, y, w - (tx - x), h), title, &bold, l.section_text, Place::Left);
    if let Some(n) = count {
        let s = n.to_string();
        p.text((tx + tw + 6, y, w, h), &s, font, l.dim, Place::Left);
    }
}

/// A small pill with `text` (the "R" of RapidR's extensions) whose left
/// is at `x`, centred on `mid`: its width.
pub fn badge(p: &mut Painter, x: i64, mid: i64, text: &str, l: &Look, font: &Font) -> i64 {
    let small = Font { size: (font.size - 2).max(6), styles: 1, ..font.clone() };
    let (tw, th) = text_size(text, &small);
    let (w, h) = (tw + 8, th + 2);
    let r = (x, mid - h / 2, w, h);
    if l.contrast {
        p.round(r, h as f64 / 2.0, None, Some(l.badge), 1.0);
        p.text(r, text, &small, l.badge, Place::Center);
    } else {
        p.round(r, h as f64 / 2.0, Some(l.badge), None, 1.0);
        p.text(r, text, &small, l.badge_text, Place::Center);
    }
    w
}

/// `text` cut with "…" to fit `width` pixels in `font`.
pub fn elide(text: &str, font: &Font, width: i64) -> String {
    if width <= 0 {
        return String::new();
    }
    if text_size(text, font).0 <= width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let s: String = chars[..mid].iter().collect::<String>() + "…";
        if text_size(&s, font).0 <= width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>() + "…"
}

/// `text` drawn left in `rect` (centred down), cut to fit, its characters
/// at `marks` (the search's matches) in bold `mark` colour.
pub fn marked_text(p: &mut Painter, rect: Rect, text: &str, marks: &[usize], font: &Font, color: u32, mark: u32) {
    let (x, y, w, h) = rect;
    let shown = elide(text, font, w);
    if marks.is_empty() {
        p.text(rect, &shown, font, color, Place::Left);
        return;
    }
    let bold = Font { styles: font.styles | 1, ..font.clone() };
    let mut at = x;
    // (runs of marked and unmarked characters, each measured as drawn)
    let chars: Vec<char> = shown.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let on = marks.contains(&i) && chars[i] != '…';
        let mut j = i + 1;
        while j < chars.len() && (marks.contains(&j) && chars[j] != '…') == on {
            j += 1;
        }
        let run: String = chars[i..j].iter().collect();
        let f = if on { &bold } else { font };
        let (rw, _) = text_size(&run, f);
        p.text((at, y, (x + w - at).max(0), h), &run, f, if on { mark } else { color }, Place::Left);
        at += rw;
        i = j;
    }
}

/// Icon `name` (rapidr_icons: an action, a component type, a file …) in a
/// `size`-pixel square whose top left is (x, y); `color` its ink when it
/// is drawn in one colour.
pub fn icon(p: &mut Painter, name: &str, x: i64, y: i64, size: i64, color: Option<u32>, disabled: bool) -> bool {
    p.icon(name, (x, y, size, size), color, disabled)
}

/// A search box in `rect`: a magnifier, then `text` (or the grey
/// `placeholder` when empty); its border in the accent when `focused`.
/// The text's own area (for an in-place editor over it).
pub fn search_box(p: &mut Painter, rect: Rect, l: &Look, text: &str, placeholder: &str, focused: bool, font: &Font) -> Rect {
    let (x, y, w, h) = rect;
    if l.classic {
        p.fill(rect, l.field);
        p.sunken_edge(rect);
    } else {
        p.round(rect, 4.0, Some(l.field), Some(if focused { l.field_focus } else { l.field_border }), if focused && !l.contrast { 1.5 } else { 1.0 });
    }
    let s = (h - 8).clamp(10, 16);
    icon(p, "search", x + 6, y + (h - s) / 2, s, Some(l.dim), false);
    let area = (x + 10 + s, y + 1, (w - 14 - s).max(0), (h - 2).max(0));
    if text.is_empty() {
        p.text(area, placeholder, font, l.faint, Place::Left);
    } else {
        p.text(area, &elide(text, font, area.2), font, l.text, Place::Left);
    }
    area
}

/// A strip of tabs across `rect`: `titles`, `active` underlined in the
/// accent (a fluent theme) or raised (classic). Each tab's rectangle.
pub fn tabs(p: &mut Painter, rect: Rect, l: &Look, titles: &[&str], active: usize, hover: Option<usize>, font: &Font) -> Vec<Rect> {
    let (x, y, w, h) = rect;
    p.fill(rect, l.chrome);
    hline(p, x, y + h - 1, w, l.line);
    let mut out = Vec::new();
    let mut at = x + 4;
    for (i, t) in titles.iter().enumerate() {
        let (tw, _) = text_size(t, font);
        let tab = (at, y, tw + 20, h);
        let on = i == active;
        if l.classic {
            if on {
                p.fill((tab.0, y + 2, tab.2, h - 2), l.body);
                p.raised_edge((tab.0, y + 2, tab.2, h));
            }
            p.text(tab, t, font, l.text, Place::Center);
        } else {
            if hover == Some(i) && !on {
                p.round((tab.0 + 2, y + 4, tab.2 - 4, h - 8), 4.0, Some(l.hover), None, 1.0);
            }
            let bold = Font { styles: font.styles | u8::from(on), ..font.clone() };
            p.text(tab, t, &bold, if on { l.text } else { l.dim }, Place::Center);
            if on {
                p.round((tab.0 + 6, y + h - 3, tab.2 - 12, 3), 1.5, Some(l.accent), None, 1.0);
            }
        }
        out.push(tab);
        at += tab.2 + 2;
    }
    out
}

/// Whether (x, y) is inside `r`.
pub fn inside(r: Rect, x: i64, y: i64) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every theme's look gives text that reads on its rows and headings
    /// (contrast at least 4.5 : 1 for text, 3 : 1 for dimmed values and
    /// errors).
    #[test]
    fn every_theme_reads() {
        for t in rapidr_value::theme::ALL {
            let l = look(t);
            let c = rapidr_icons::contrast;
            assert!(c(l.text, l.body) >= 4.5, "{}: text on body", t.name);
            assert!(c(l.section_text, l.section) >= 4.5, "{}: heading", t.name);
            // (classic: Windows' own highlight, 4.499 : 1 with white)
            assert!(c(l.selected_text, l.selected) >= 4.45, "{}: selection", t.name);
            assert!(c(l.dim, l.body) >= 3.0, "{}: dimmed values", t.name);
            assert!(c(l.error, l.body) >= 3.0, "{}: errors", t.name);
        }
    }

    #[test]
    fn text_is_cut_to_fit() {
        let f = Font { name: "Arial".into(), size: 10, color: 0, styles: 0 };
        assert_eq!(elide("Caption", &f, 1000), "Caption");
        let cut = elide("A rather long property value", &f, 60);
        assert!(cut.ends_with('…') && text_size(&cut, &f).0 <= 60, "{cut}");
    }
}
