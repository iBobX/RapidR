//! A QLABEL's `AutoSize` (Delphi's TCustomLabel.AdjustBounds, which RapidQ's
//! QLABEL is), the same on every runtime. Pinned with RC.EXE (console
//! programs creating labels on a form and printing their sizes; the
//! numbers are MS Sans Serif 8's, which `objects::text` measures exactly):
//!
//! * AutoSize is True until the program sets it (`AutoSize` reads 1).
//! * A new label is 65 × 17 and stays so until something below happens —
//!   `Caption = ""` on a new label changes nothing (the caption was "").
//! * The label takes its text's size (Windows' `DrawText` with DT_CALCRECT
//!   and DT_EXPANDTABS) when, with AutoSize on:
//!   - its Caption changes (`"Password:"` → 49 × 13 — 50 in RapidR, whose
//!     MS Sans Serif draws r a pixel wider: fonts/README.md; setting the caption it
//!     already has does nothing: an explicit `Width = 20` stays);
//!   - the font it's drawn in changes — any of its properties, its colour
//!     too: its own, its parent's (Delphi's ParentFont: `Form.Font.Color =
//!     &HFF0000` resizes the form's labels and those in its panels), or a
//!     new Parent with another font (a DIM'd label given a bold form's
//!     Parent; RapidR doesn't follow a panel moved to another parent into
//!     its labels). Setting a font property to what it is (`Font.Size = 8`,
//!     `Font.Name = "MS Sans Serif"`) changes nothing; a label that chose
//!     its own font doesn't follow its parent's in RapidQ (RapidR resizes
//!     it for what it draws: its unset font properties are its parent's);
//!   - its WordWrap changes;
//!   - AutoSize goes from 0 to 1 (setting 1 when it's 1 does nothing).
//! * Width / Height set by the program stick until the next of those
//!   (`Caption = …: Width = 64` in a CREATE block reads 64; `Width = 64:
//!   Caption = …` reads the text's width).
//! * The text's size: each line (CR LF / CR / LF) measured, `&` a mnemonic
//!   (not counted; `&&` is one `&`), tabs to the next multiple of 8 average
//!   character widths; Width the widest line, Height the lines × the font's
//!   line height. An empty caption (or a lone `&`) measures one space
//!   (3 × 13: Delphi's DoDrawText adds a space). With WordWrap the lines
//!   break at spaces to fit the current Width, and Width becomes the widest
//!   line — a word wider than the Width keeps its width (`"Centred text
//!   again"` at Width 30: 37 × 39).
//! * Alignment taRightJustify (1) keeps the right edge (Left moves);
//!   taCenter (2) and taLeftJustify keep Left.

use crate::layout::Rect;
use crate::objects::font::Font;
use crate::objects::text::text_size;
use crate::Value;

/// A label's font properties, flat (as `objects::font_from_props` reads
/// them): setting one resizes an AutoSize label.
pub const FONT_PROPERTIES: [&str; 7] = ["fontname", "fontsize", "fontbold", "fontitalic", "fontunderline", "fontstrikeout", "fontcolor"];

/// The flat font property `prop` (lowercase; `font.size` or `fontsize`)
/// is, if it's one of a label's.
pub fn font_property(prop: &str) -> Option<&'static str> {
    let flat = prop.strip_prefix("font.").map(|s| format!("font{s}"));
    let p = flat.as_deref().unwrap_or(prop);
    FONT_PROPERTIES.iter().find(|f| **f == p).copied()
}

/// What setting a property can do to labels' sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Watch {
    /// A label's own Caption, AutoSize or WordWrap: compare its value
    /// before and after ([`resizes`]).
    Own,
    /// A font property or the Parent, of any component: the labels it is
    /// (a label) or holds resize when their font changed (compare each
    /// one's font before and after).
    Fonts,
}

/// Whether setting `prop` (lowercase) of a component of type `type_name`
/// can resize labels, and how a runtime checks it.
pub fn watched(type_name: &str, prop: &str) -> Option<Watch> {
    let label = type_name.eq_ignore_ascii_case("RLABEL");
    // (a label's new Parent; a container's isn't followed into its labels —
    // every component created sets one, and none has labels yet)
    if font_property(prop).is_some() || (prop == "parent" && label) {
        return Some(Watch::Fonts);
    }
    (label && matches!(prop, "caption" | "autosize" | "wordwrap")).then_some(Watch::Own)
}

/// Whether setting a label's `prop` (lowercase: caption, autosize,
/// wordwrap) from `before` to `after` makes it take its text's size (if
/// its AutoSize is on: [`label_bounds`]).
pub fn resizes(prop: &str, before: &Value, after: &Value) -> bool {
    match prop {
        "caption" => before.to_string_val() != after.to_string_val(),
        // (only from off to on: AutoSize's default is on)
        "autosize" => !flag(before, true) && flag(after, true),
        "wordwrap" => flag(before, false) != flag(after, false),
        _ => false,
    }
}

/// A Boolean property's value, `default` when unset.
fn flag(v: &Value, default: bool) -> bool {
    match v {
        Value::Null => default,
        v => v.to_bool(),
    }
}

/// A label's AutoSize (True unless set).
pub fn autosize_on(v: &Value) -> bool {
    flag(v, true)
}

/// The text a label shows for `caption`: its mnemonic `&`s dropped (`&&`
/// is one `&`), as DrawText's prefix processing.
fn shown(caption: &str) -> String {
    let mut out = String::new();
    let mut chars = caption.chars();
    while let Some(c) = chars.next() {
        if c == '&' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Width of one line of `font`'s text, tabs expanded to the next multiple
/// of 8 average character widths (DT_EXPANDTABS).
fn line_width(line: &str, font: &Font) -> i64 {
    if !line.contains('\t') {
        return text_size(line, font).0;
    }
    let tab = 8 * average_char_width(font);
    let mut x = 0;
    for (k, part) in line.split('\t').enumerate() {
        if k > 0 && tab > 0 {
            x = (x / tab + 1) * tab;
        }
        x += text_size(part, font).0;
    }
    x
}

/// GDI's tmAveCharWidth (5 for MS Sans Serif 8).
fn average_char_width(font: &Font) -> i64 {
    crate::objects::text::average_char_width(font)
}

/// `line`'s pieces once wrapped at spaces to fit `width` (DT_WORDBREAK), as
/// DrawText breaks them: as many characters as fit; when the next one is a
/// space the line ends before it, otherwise at the last space that fits,
/// that space kept (it counts in the width: RC.EXE's 184 for a 181-pixel
/// line at Width 200); a word wider than `width` is a line of its own, not
/// cut. Spaces starting the next line are skipped.
fn wrap_line(line: &str, font: &Font, width: i64) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut start = 0;
    loop {
        let rest: String = chars[start..].iter().collect();
        if line_width(&rest, font) <= width || start >= chars.len() {
            out.push(rest);
            return out;
        }
        // (how many characters fit)
        let mut n = 0;
        while start + n < chars.len() && line_width(&chars[start..start + n + 1].iter().collect::<String>(), font) <= width {
            n += 1;
        }
        let end = if chars[start + n] == ' ' {
            start + n
        } else if let Some(sp) = chars[start..start + n].iter().rposition(|&c| c == ' ') {
            start + sp + 1
        } else {
            // (a word wider than the width: up to its end)
            let mut e = start + n;
            while e < chars.len() && chars[e] != ' ' {
                e += 1;
            }
            e
        };
        out.push(chars[start..end].iter().collect());
        start = end;
        while start < chars.len() && chars[start] == ' ' {
            start += 1;
        }
        if start >= chars.len() {
            return out;
        }
    }
}

/// The lines a label shows for `caption` in `font` (`&` marks dropped):
/// broken at CR LF / CR / LF, and wrapped to `wrap` pixels with WordWrap —
/// what `text_extent` measures, for drawing.
pub fn lines(caption: &str, font: &Font, wrap: Option<i64>) -> Vec<String> {
    let text = shown(caption);
    let mut lines = Vec::new();
    for para in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        match wrap {
            Some(w) => lines.extend(wrap_line(para, font, w)),
            None => lines.push(para.to_string()),
        }
    }
    lines
}

/// The size `caption` takes in `font` (DrawText's DT_CALCRECT): lines at
/// CR LF / CR / LF, wrapped to `wrap` pixels when WordWrap is on. Measured
/// as RapidQ measures in every theme (`theme::rapidq_metrics`): a label's
/// size is RC.EXE's whatever face the theme draws its text in.
pub fn text_extent(caption: &str, font: &Font, wrap: Option<i64>) -> (i64, i64) {
    crate::theme::rapidq_metrics(|| rapidq_extent(caption, font, wrap))
}

fn rapidq_extent(caption: &str, font: &Font, wrap: Option<i64>) -> (i64, i64) {
    let mut text = shown(caption);
    // (Delphi's DoDrawText measures an empty caption, or a lone `&`, as a
    // space)
    if caption.is_empty() || caption == "&" {
        text = " ".into();
    }
    let line_height = text_size(" ", font).1;
    let mut lines: Vec<String> = Vec::new();
    for para in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        match wrap {
            Some(w) => lines.extend(wrap_line(para, font, w)),
            None => lines.push(para.to_string()),
        }
    }
    let width = lines.iter().map(|l| line_width(l, font)).max().unwrap_or(0);
    (width, line_height * lines.len() as i64)
}

/// Where an AutoSize label at `rect` goes for `caption` in `font`, with
/// its WordWrap and Alignment (taRightJustify keeps its right edge).
pub fn bounds(caption: &str, font: &Font, word_wrap: bool, alignment: i64, rect: Rect) -> Rect {
    let (w, h) = text_extent(caption, font, word_wrap.then_some(rect.width));
    let left = if alignment == 1 { rect.left + rect.width - w } else { rect.left };
    Rect::new(left, rect.top, w, h)
}

/// The new place of AutoSize label `id`, from its properties (`props`, as
/// the runtimes read them: its font through its parents) — `None` when its
/// AutoSize is off.
pub fn label_bounds(id: &str, props: &dyn Fn(&str, &str) -> Value) -> Option<Rect> {
    if !autosize_on(&props(id, "autosize")) {
        return None;
    }
    let font = crate::objects::font_from_props(id, props);
    let n = |p: &str| props(id, p).to_i64();
    let rect = Rect::new(n("left"), n("top"), n("width"), n("height"));
    Some(bounds(&props(id, "caption").to_string_val(), &font, flag(&props(id, "wordwrap"), false), n("alignment"), rect))
}

/// The font a label is drawn in, as the runtimes resolve it: its font
/// properties through its parents, and its Font.Color (`color`, the
/// runtime's: a parent's when it never set one).
pub type FontKey = (Font, i64);

/// What a runtime keeps before it stores a property that can resize
/// labels ([`watched`]).
pub enum Before {
    Own(String, Value),
    Fonts(Vec<(String, FontKey)>),
}

/// Before `rp_comp_set` stores `prop` (lowercase) of `name` (of type
/// `type_name`): what [`changed_labels`] compares with afterwards, or
/// `None` when no label can change. `stored` reads a stored property,
/// `children` lists a component's children, `key` a label's font.
pub fn before_set(name: &str, type_name: &str, prop: &str, stored: &dyn Fn(&str, &str) -> Value, children: &dyn Fn(&str) -> Vec<(String, String)>, key: &dyn Fn(&str) -> FontKey) -> Option<Before> {
    match watched(type_name, prop)? {
        Watch::Own => Some(Before::Own(prop.to_string(), stored(name, prop))),
        Watch::Fonts => {
            let labels = labels_under(name, type_name, children);
            (!labels.is_empty()).then(|| Before::Fonts(labels.into_iter().map(|l| (key(&l), l)).map(|(k, l)| (l, k)).collect()))
        }
    }
}

/// The labels to size again once the property is stored: `name` when its
/// Caption / AutoSize / WordWrap changed as [`resizes`] says, the labels
/// whose font changed.
pub fn changed_labels(name: &str, before: Before, stored: &dyn Fn(&str, &str) -> Value, key: &dyn Fn(&str) -> FontKey) -> Vec<String> {
    match before {
        Before::Own(prop, was) => {
            if resizes(&prop, &was, &stored(name, &prop)) {
                vec![name.to_string()]
            } else {
                Vec::new()
            }
        }
        Before::Fonts(list) => list.into_iter().filter(|(l, k)| key(l) != *k).map(|(l, _)| l).collect(),
    }
}

/// The labels a font or Parent change of `id` (of type `type_name`) may
/// resize: `id` itself when it's a label, and the labels inside it, at any
/// depth (`children` lists a component's children as (name, type)) — a
/// font passes down to them (Delphi's ParentFont).
pub fn labels_under(id: &str, type_name: &str, children: &dyn Fn(&str) -> Vec<(String, String)>) -> Vec<String> {
    // (a label holds no components)
    if type_name.eq_ignore_ascii_case("RLABEL") {
        return vec![id.to_string()];
    }
    let mut out = Vec::new();
    let mut todo = vec![(id.to_string(), 0)];
    while let Some((at, depth)) = todo.pop() {
        if depth > 32 {
            continue;
        }
        for (child, kind) in children(&at) {
            if child.eq_ignore_ascii_case(&at) {
                continue;
            }
            if kind.eq_ignore_ascii_case("RLABEL") {
                out.push(child.clone());
            }
            todo.push((child, depth + 1));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms8() -> Font {
        // (RapidQ's own metrics: the classic look's face)
        crate::theme::set(&crate::theme::CLASSIC);
        Font { name: "MS Sans Serif".into(), size: 8, color: 0, styles: 0 }
    }

    /// RC.EXE's numbers (scratch probes: a_label.bas, f_text.bas), and
    /// RapidR's: RapidR Sans draws r, x, y, j, C and the brackets a pixel
    /// wider than MS Sans Serif's bitmap (readable anti-aliased letters need
    /// the space: fonts/README.md), the other characters as wide as RapidQ.
    /// (text, RapidQ's size, RapidR's size)
    const LABELS: &[(&str, (i64, i64), (i64, i64))] = &[
        ("Password:", (49, 13), (50, 13)),
        ("Pass&word:", (49, 13), (50, 13)),
        ("Right", (25, 13), (25, 13)),
        ("Pass", (23, 13), (23, 13)),
        ("The quick brown fox jumps over the lazy dog. 0123456789", (277, 13), (282, 13)),
        ("Two\r\nlines here", (45, 26), (46, 26)),
        ("", (3, 13), (3, 13)),
        ("&", (3, 13), (3, 13)),
        ("&&", (6, 13), (7, 13)),
        (" ", (3, 13), (3, 13)),
        ("A\tB", (47, 13), (47, 13)),
        ("Right aligned longer", (94, 13), (95, 13)),
        ("Right aligned longer text", (114, 13), (116, 13)),
        ("Lay", (17, 13), (18, 13)),
        ("Centred text", (57, 13), (60, 13)),
        ("Centred text again", (86, 13), (89, 13)),
    ];

    #[test]
    fn texts_measure_as_rapidq_labels() {
        let f = ms8();
        for &(text, rapidq, rapidr) in LABELS {
            assert_eq!(text_extent(text, &f, None), rapidr, "{text:?} (RapidQ: {rapidq:?})");
            // (a pixel or so wider than RapidQ's, never narrower or taller)
            assert!(rapidr.0 >= rapidq.0 && rapidr.0 - rapidq.0 <= (rapidq.0 / 20).max(1) + 1 && rapidr.1 == rapidq.1, "{text:?}");
        }
    }

    #[test]
    fn word_wrap_fits_the_width_and_takes_the_widest_line() {
        let f = ms8();
        // (RapidQ: 184 × 26, both)
        assert_eq!(text_extent("Word wrap: the quick brown fox jumps over the lazy dog.", &f, Some(200)), (189, 26));
        assert_eq!(text_extent("Word wrap: the quick brown fox jumps over the lazy dog. More words.", &f, Some(189)), (189, 26));
        assert_eq!(text_extent("Centred text", &f, Some(60)), (60, 13));
        // (a word wider than the width isn't cut: the label grows to it;
        // RapidQ: 37 × 39)
        assert_eq!(text_extent("Centred text again", &f, Some(30)), (39, 39));
    }

    #[test]
    fn right_alignment_keeps_the_right_edge() {
        let f = ms8();
        // (RapidQ: 171, 12, 94, 13 and 246, 12, 114, 13 — its texts a
        // pixel or two narrower)
        assert_eq!(bounds("Right aligned longer", &f, false, 1, Rect::new(240, 12, 25, 13)), Rect::new(170, 12, 95, 13));
        assert_eq!(bounds("Right aligned longer text", &f, false, 1, Rect::new(240, 12, 120, 13)), Rect::new(244, 12, 116, 13));
        // (centred: Left stays)
        assert_eq!(bounds("Centred text", &f, false, 2, Rect::new(300, 100, 3, 13)), Rect::new(300, 100, 60, 13));
    }

    #[test]
    fn bigger_fonts_bigger_labels() {
        // (RC.EXE's widths; its heights there — 20 at 12 points, 24 at 14 —
        // and the long line's 407 at 12 points are MS Sans Serif's own
        // bitmap sizes, objects::text's to match: 19, 23 and 403 now)
        let f12 = Font { size: 12, ..ms8() };
        assert_eq!(text_extent("", &f12, None).0, 4);
        let f14 = Font { size: 14, ..ms8() };
        assert_eq!(text_extent("Pass", &f14, None).0, 40);
        assert_eq!(text_extent("Pass", &f14, None).1, text_size("Pass", &f14).1);
    }

    #[test]
    fn what_resizes() {
        use crate::{v_int, v_str};
        assert!(resizes("caption", &v_str("a"), &v_str("b")));
        assert!(!resizes("caption", &v_str("a"), &v_str("a")));
        assert!(resizes("autosize", &v_int(0), &v_int(1)));
        assert!(!resizes("autosize", &Value::Null, &v_int(1)));
        assert!(!resizes("autosize", &v_int(1), &v_int(0)));
        assert!(resizes("wordwrap", &Value::Null, &v_int(1)));
        assert!(!resizes("wordwrap", &v_int(0), &v_int(0)));
        assert_eq!(watched("RLABEL", "width"), None);
        assert_eq!(watched("RLABEL", "caption"), Some(Watch::Own));
        assert_eq!(watched("RBUTTON", "caption"), None);
        assert_eq!(watched("RFORM", "font.name"), Some(Watch::Fonts));
        assert_eq!(watched("RLABEL", "fontcolor"), Some(Watch::Fonts));
        assert_eq!(watched("RLABEL", "parent"), Some(Watch::Fonts));
        assert_eq!(watched("RPANEL", "parent"), None);
    }

    #[test]
    fn a_font_reaches_the_labels_inside() {
        let kids = |id: &str| match id {
            "f" => vec![("l1".to_string(), "RLABEL".to_string()), ("p".to_string(), "RPANEL".to_string()), ("b".to_string(), "RBUTTON".to_string())],
            "p" => vec![("l2".to_string(), "RLABEL".to_string())],
            _ => vec![],
        };
        let mut got = labels_under("f", "RFORM", &kids);
        got.sort();
        assert_eq!(got, vec!["l1", "l2"]);
        assert_eq!(labels_under("l1", "RLABEL", &kids), vec!["l1"]);
    }
}
