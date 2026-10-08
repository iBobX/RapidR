//! QFONTDIALOG (RapidQ manual, Appendix A: Delphi's TFontDialog over
//! Windows' ChooseFont) — the same on every runtime: its properties and
//! methods, the request a dialog shows and the answer it stores, its
//! layout. The kernel draws it from this and the web builds its page
//! dialog from it.
//!
//! - `Name`, `Size`, `Color` and the styles AddStyles / DelStyles set
//!   (fsBold 0, fsItalic 1, fsUnderline 2, fsStrikeOut 3) are the font
//!   chosen; RapidR's flat FontName / FontSize / FontColor / FontBold …
//!   are the same values (one is the other).
//! - `GetFont(F)` makes QFONT `F`'s font the dialog's; `SetFont(F)` gives
//!   `F` the dialog's (the manual's example: GetFont before Execute, SetFont
//!   after).
//! - `MinFontSize` / `MaxFontSize` limit the sizes with fdLimitSize.
//! - `AddOptions` / `DelOptions` (fdAnsiOnly 0 … fdApplyButton 15, RAPIDQ.INC):
//!   fdEffects (on by default, as TFontDialog's) shows Strikeout, Underline
//!   and Color; fdApplyButton an Apply button, which fires OnApply with the
//!   choice stored; fdShowHelp a (disabled) Help button; fdNoFaceSel /
//!   fdNoStyleSel / fdNoSizeSel select nothing at first; fdFixedPitchOnly
//!   lists fixed-pitch faces only. The others change nothing here (every
//!   face RapidR draws is scalable TrueType).
//! - `FontCount` and `FontName(i)`: the faces a dialog offers every
//!   program alike ([`FONT_NAMES`]; a desktop dialog adds the system's).
//! - `Execute` returns 1 for OK, else 0.

use crate::objects::font::Font;
use crate::{v_int, v_str, Value};

/// The faces RapidQ programs name, drawn with RapidR's built-in Liberation
/// fonts (`objects::text::family_name`): what every runtime's dialog lists
/// (the desktop's adds the system's own), FontCount and FontName(i).
pub const FONT_NAMES: [&str; 8] = ["Arial", "Courier New", "Georgia", "MS Sans Serif", "Tahoma", "Times New Roman", "Trebuchet MS", "Verdana"];
/// The sizes ChooseFont lists (points).
pub const SIZES: [i64; 16] = [8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72];
/// The styles (the index: italic 1 + bold 2).
pub const STYLES: [&str; 4] = ["Regular", "Italic", "Bold", "Bold Italic"];
/// ChooseFont's colours (the 16 VGA ones; &HBBGGRR).
pub const COLORS: [(&str, i64); 16] = [
    ("Black", 0x000000), ("Maroon", 0x000080), ("Green", 0x008000), ("Olive", 0x008080),
    ("Navy", 0x800000), ("Purple", 0x800080), ("Teal", 0x808000), ("Gray", 0x808080),
    ("Silver", 0xC0C0C0), ("Red", 0x0000FF), ("Lime", 0x00FF00), ("Yellow", 0x00FFFF),
    ("Blue", 0xFF0000), ("Fuchsia", 0xFF00FF), ("Aqua", 0xFFFF00), ("White", 0xFFFFFF),
];

// RAPIDQ.INC's QFONTDIALOG options (AddOptions / DelOptions).
pub const FD_EFFECTS: i64 = 2;
pub const FD_FIXED_PITCH_ONLY: i64 = 3;
pub const FD_NO_FACE_SEL: i64 = 5;
pub const FD_NO_SIZE_SEL: i64 = 8;
pub const FD_NO_STYLE_SEL: i64 = 9;
pub const FD_SHOW_HELP: i64 = 11;
pub const FD_LIMIT_SIZE: i64 = 13;
pub const FD_APPLY_BUTTON: i64 = 15;
/// TFontDialog's Options at first: [fdEffects] (as bits).
pub const DEFAULT_OPTIONS: i64 = 1 << FD_EFFECTS;

/// The flat style properties, by style number (fsBold …).
const STYLE_PROPS: [&str; 4] = ["fontbold", "fontitalic", "fontunderline", "fontstrikeout"];

/// What a font dialog shows: the font chosen so far and the options.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub font: Font,
    pub min: i64,
    pub max: i64,
    /// The options' bits (bit n: option n on).
    pub options: i64,
}

impl Request {
    pub fn has(&self, option: i64) -> bool {
        (0..63).contains(&option) && self.options & (1 << option) != 0
    }

    /// The sizes listed: within Min / MaxFontSize with fdLimitSize.
    pub fn sizes(&self) -> Vec<i64> {
        let limit = self.has(FD_LIMIT_SIZE);
        SIZES.iter().copied().filter(|s| !limit || ((self.min <= 0 || *s >= self.min) && (self.max <= 0 || *s <= self.max))).collect()
    }

    /// The faces listed from `all` (fdFixedPitchOnly: the fixed-pitch
    /// ones), sorted, with the chosen one added when missing.
    pub fn names(&self, all: &[String]) -> Vec<String> {
        let mut names: Vec<String> = all.iter().filter(|n| !self.has(FD_FIXED_PITCH_ONLY) || is_fixed_pitch(n)).cloned().collect();
        if !self.font.name.is_empty() && !names.iter().any(|n| n.eq_ignore_ascii_case(&self.font.name)) {
            names.push(self.font.name.clone());
        }
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        names
    }

    /// The colour list's entries: the 16, and "Custom" for another colour.
    pub fn colors(&self) -> Vec<(String, i64)> {
        let mut out: Vec<(String, i64)> = COLORS.iter().map(|(n, c)| (n.to_string(), *c)).collect();
        if color_index(self.font.color).is_none() {
            out.push(("Custom".into(), self.font.color));
        }
        out
    }
}

/// Whether a face has fixed-pitch characters (as RapidR draws it: the
/// built-in Mono stands for these names).
pub fn is_fixed_pitch(name: &str) -> bool {
    crate::objects::text::family_name(name) == "Liberation Mono"
}

/// The colour list's index of &HBBGGRR `c` (`None`: not one of the 16).
pub fn color_index(c: i64) -> Option<usize> {
    COLORS.iter().position(|(_, v)| *v == c & 0xFF_FFFF)
}

/// The request a dialog's properties make (`get`: a property, `Null` when
/// it has none).
pub fn request(get: &dyn Fn(&str) -> Value) -> Request {
    let either = |a: &str, b: &str| match get(a) {
        Value::Null => get(b),
        v => v,
    };
    let name = either("name", "fontname").to_string_val();
    let size = either("size", "fontsize").to_i64();
    let defaults = Font::default();
    let mut styles = 0u8;
    for (i, p) in STYLE_PROPS.iter().enumerate() {
        if get(p).to_bool() {
            styles |= 1 << i;
        }
    }
    let options = match get("options") {
        Value::Null => DEFAULT_OPTIONS,
        v => v.to_i64(),
    };
    Request {
        font: Font { name: if name.trim().is_empty() { defaults.name } else { name }, size: if size > 0 { size } else { defaults.size }, color: crate::objects::color_bgr(either("color", "fontcolor").to_i64()) as i64, styles },
        min: get("minfontsize").to_i64(),
        max: get("maxfontsize").to_i64(),
        options,
    }
}

/// The properties a dialog's font is kept in (Name / Size / Color and the
/// flat FontName …, the styles), as QFONTDIALOG answers them.
pub fn properties(font: &Font) -> Vec<(&'static str, Value)> {
    let flag = |i: usize| v_int(if font.styles & 1 << i != 0 { -1 } else { 0 });
    vec![
        ("name", v_str(&font.name)),
        ("size", v_int(font.size)),
        ("color", v_int(font.color)),
        ("fontname", v_str(&font.name)),
        ("fontsize", v_int(font.size)),
        ("fontcolor", v_int(font.color)),
        (STYLE_PROPS[0], flag(0)),
        (STYLE_PROPS[1], flag(1)),
        (STYLE_PROPS[2], flag(2)),
        (STYLE_PROPS[3], flag(3)),
    ]
}

/// Its properties at first: TFontDialog's (the default QFONT, Options
/// [fdEffects], no size limits) and FontCount.
pub fn defaults() -> Vec<(&'static str, Value)> {
    let mut out = properties(&Font::default());
    out.extend([("minfontsize", v_int(0)), ("maxfontsize", v_int(0)), ("options", v_int(DEFAULT_OPTIONS)), ("fontcount", v_int(FONT_NAMES.len() as i64))]);
    out
}

/// The property a Name / Size / Color is also kept as (and back): one
/// value.
pub fn alias(prop: &str) -> Option<&'static str> {
    Some(match prop {
        "name" => "fontname",
        "size" => "fontsize",
        "color" => "fontcolor",
        "fontname" => "name",
        "fontsize" => "size",
        "fontcolor" => "color",
        _ => return None,
    })
}

/// A QFONTDIALOG method on its properties (`get` / `set`): AddStyles,
/// DelStyles, AddOptions, DelOptions, GetFont, SetFont, FontName(i).
/// `None`: not one of them.
pub fn call(method: &str, args: &[Value], get: &dyn Fn(&str) -> Value, set: &mut dyn FnMut(&str, Value)) -> Option<Value> {
    match method {
        "addstyles" | "delstyles" => {
            for a in args {
                if let Some(p) = usize::try_from(a.to_i64()).ok().and_then(|i| STYLE_PROPS.get(i)) {
                    set(p, v_int(if method == "addstyles" { -1 } else { 0 }));
                }
            }
        }
        "addoptions" | "deloptions" => {
            let mut bits = request(get).options;
            for a in args.iter().map(Value::to_i64).filter(|n| (0..63).contains(n)) {
                if method == "addoptions" {
                    bits |= 1 << a;
                } else {
                    bits &= !(1 << a);
                }
            }
            set("options", v_int(bits));
        }
        // (the QFONT is passed by its object id)
        // (anything but a QFONT: nothing to take)
        "getfont" => {
            let id = args.first().map(Value::to_string_val).unwrap_or_default();
            if let Some(props) = crate::objects::font_properties(&id) {
                let font = crate::objects::font_from_props(&id, &|_, p| props.iter().find(|(k, _)| *k == p).map_or(Value::Null, |(_, v)| v.clone()));
                for (p, v) in properties(&font) {
                    set(p, v);
                }
            }
        }
        "setfont" => {
            let id = args.first()?.to_string_val();
            let font = request(get).font;
            let flag = |i: usize| v_int(if font.styles & 1 << i != 0 { -1 } else { 0 });
            for (p, v) in [("name", v_str(&font.name)), ("size", v_int(font.size)), ("color", v_int(font.color)), ("bold", flag(0)), ("italic", flag(1)), ("underline", flag(2)), ("strikeout", flag(3))] {
                crate::objects::set(&id, p, &v);
            }
        }
        "fontname" if !args.is_empty() => {
            let i = args[0].to_i64();
            return Some(v_str(usize::try_from(i).ok().and_then(|i| FONT_NAMES.get(i)).copied().unwrap_or("")));
        }
        _ => return None,
    }
    Some(Value::Null)
}

/// Where the dialog's parts go (logical pixels), after Windows' ChooseFont.
pub mod layout {
    pub type Rect = (i64, i64, i64, i64);
    pub const FONT_LABEL: Rect = (6, 6, 100, 14);
    pub const STYLE_LABEL: Rect = (162, 6, 100, 14);
    pub const SIZE_LABEL: Rect = (268, 6, 50, 14);
    pub const FONT_LIST: Rect = (6, 22, 150, 112);
    pub const STYLE_LIST: Rect = (162, 22, 100, 112);
    pub const SIZE_LIST: Rect = (268, 22, 50, 112);
    pub const EFFECTS: Rect = (6, 140, 150, 96);
    pub const STRIKEOUT: Rect = (14, 158, 130, 17);
    pub const UNDERLINE: Rect = (14, 178, 130, 17);
    pub const COLOR_LABEL: Rect = (14, 198, 60, 14);
    pub const COLOR_LIST: Rect = (14, 212, 134, 21);
    /// The Sample group and its text, with and without the effects.
    pub fn sample(effects: bool) -> (Rect, Rect) {
        let h = if effects { 96 } else { 62 };
        ((162, 140, 156, h), (170, 156, 140, h - 22))
    }
    pub const OK: Rect = (324, 22, 66, 23);
    pub const CANCEL: Rect = (324, 50, 66, 23);
    pub const APPLY: Rect = (324, 78, 66, 23);
    pub const HELP: Rect = (324, 106, 66, 23);
    /// The window's inside.
    pub fn size(effects: bool) -> (i64, i64) {
        (396, if effects { 244 } else { 210 })
    }
}

/// The sample's text.
pub const SAMPLE: &str = "AaBbYyZz";

/// `RAPIDR_TEST_FONT_DIALOG`'s answer: `Name,Size,styles,color` — the
/// styles letters b, i, u, s; the colour as `color_dialog::parse_color`
/// reads it — for OK, empty for Cancel.
pub fn parse_answer(s: &str) -> Option<Font> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut parts = s.split(',').map(str::trim);
    let name = parts.next().unwrap_or("").to_string();
    let size = parts.next().and_then(|v| v.parse().ok()).unwrap_or(10);
    let styles = parts.next().unwrap_or("").chars().fold(0u8, |acc, c| {
        acc | match c.to_ascii_lowercase() {
            'b' => 1,
            'i' => 2,
            'u' => 4,
            's' => 8,
            _ => 0,
        }
    });
    let color = parts.next().and_then(crate::color_dialog::parse_color).unwrap_or(0);
    Some(Font { name, size, color, styles })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn store(props: Vec<(&'static str, Value)>) -> HashMap<String, Value> {
        props.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn defaults_and_request() {
        let p = store(defaults());
        let get = |k: &str| p.get(k).cloned().unwrap_or(Value::Null);
        let r = request(&get);
        assert_eq!(r.font, Font::default());
        assert!(r.has(FD_EFFECTS) && !r.has(FD_APPLY_BUTTON));
        assert_eq!(r.sizes(), SIZES.to_vec());
        assert_eq!(get("fontcount").to_i64(), 8);
        // (Name or the flat FontName; a size limit with fdLimitSize only)
        let p = store(vec![("fontname", v_str("Courier New")), ("size", v_int(14)), ("minfontsize", v_int(10)), ("maxfontsize", v_int(20)), ("options", v_int(1 << FD_LIMIT_SIZE))]);
        let r = request(&|k| p.get(k).cloned().unwrap_or(Value::Null));
        assert_eq!((r.font.name.as_str(), r.font.size), ("Courier New", 14));
        assert_eq!(r.sizes(), vec![10, 11, 12, 14, 16, 18, 20]);
        assert!(!r.has(FD_EFFECTS));
    }

    #[test]
    fn methods() {
        let mut p = store(defaults());
        let mut run = |p: &mut HashMap<String, Value>, m: &str, args: &[Value]| {
            let snapshot = p.clone();
            let mut set = |k: &str, v: Value| {
                p.insert(k.to_string(), v);
            };
            call(m, args, &|k| snapshot.get(k).cloned().unwrap_or(Value::Null), &mut set)
        };
        run(&mut p, "addstyles", &[v_int(0), v_int(2)]);
        assert_eq!((p["fontbold"].to_i64(), p["fontunderline"].to_i64()), (-1, -1));
        run(&mut p, "delstyles", &[v_int(0)]);
        assert_eq!(p["fontbold"].to_i64(), 0);
        run(&mut p, "addoptions", &[v_int(FD_APPLY_BUTTON), v_int(FD_LIMIT_SIZE)]);
        run(&mut p, "deloptions", &[v_int(FD_EFFECTS)]);
        assert_eq!(p["options"].to_i64(), 1 << FD_APPLY_BUTTON | 1 << FD_LIMIT_SIZE);
        assert_eq!(run(&mut p, "fontname", &[v_int(1)]).unwrap().to_string_val(), "Courier New");
        assert_eq!(run(&mut p, "fontname", &[v_int(99)]).unwrap().to_string_val(), "");
        assert!(run(&mut p, "execute", &[]).is_none());
        // GetFont / SetFont with a QFONT object
        crate::objects::create("fd_test_font", "RFONT");
        crate::objects::set("fd_test_font", "name", &v_str("Times New Roman"));
        crate::objects::set("fd_test_font", "size", &v_int(16));
        crate::objects::set("fd_test_font", "italic", &v_int(-1));
        run(&mut p, "getfont", &[v_str("fd_test_font")]);
        assert_eq!((p["name"].to_string_val().as_str(), p["fontsize"].to_i64(), p["fontitalic"].to_i64()), ("Times New Roman", 16, -1));
        p.insert("name".into(), v_str("Verdana"));
        p.insert("fontstrikeout".into(), v_int(-1));
        run(&mut p, "setfont", &[v_str("fd_test_font")]);
        let font = crate::objects::font_properties("fd_test_font").unwrap();
        let f = |k: &str| font.iter().find(|(p, _)| *p == k).unwrap().1.clone();
        // (GetFont took the QFONT's styles: italic, no longer underlined)
        assert_eq!((f("fontname").to_string_val().as_str(), f("fontstrikeout").to_i64(), f("fontitalic").to_i64(), f("fontunderline").to_i64()), ("Verdana", 1, 1, 0));
        crate::objects::remove("fd_test_font");
    }

    #[test]
    fn lists_and_answers() {
        let mut r = request(&|_| Value::Null);
        r.font.name = "Comic Sans MS".into();
        let all: Vec<String> = FONT_NAMES.iter().map(|s| s.to_string()).collect();
        let names = r.names(&all);
        assert_eq!(names.len(), 9);
        assert_eq!(names[1], "Comic Sans MS");
        r.options |= 1 << FD_FIXED_PITCH_ONLY;
        r.font.name = "Courier New".into();
        assert_eq!(r.names(&all), vec!["Courier New".to_string()]);
        assert_eq!(color_index(0x0000FF), Some(9));
        r.font.color = 0x123456;
        assert_eq!(r.colors().last().map(|(n, c)| (n.as_str(), *c)), Some(("Custom", 0x123456)));
        assert_eq!(parse_answer("Courier New, 14, bu, 255"), Some(Font { name: "Courier New".into(), size: 14, color: 255, styles: 0b101 }));
        assert_eq!(parse_answer(" "), None);
        assert_eq!(alias("name"), Some("fontname"));
        assert_eq!(alias("fontcolor"), Some("color"));
    }
}
