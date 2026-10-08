//! A property's kind of value as the inspector edits it, and its values'
//! three spellings: as a row shows it ("akLeft, akTop", "clRed",
//! "MS Sans Serif, 8 pt"), as the program writes it (`akLeft + akTop`,
//! `True`, `&H0000FF` — what OnPropertyChange says and a designer writes
//! into the CREATE block) and as a runtime keeps it (a constant's number,
//! -1 / 0 for True / False).

use crate::Value;

/// How a property is edited (from its registry type and editor).
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Int,
    Float,
    Text,
    Bool,
    /// One of these constants.
    Enum(Vec<String>),
    /// Any of these flags.
    Set(Vec<String>),
    Color,
    /// Name, size, colour and styles, each a part.
    Font,
    /// A file's name, typed or chosen with "…" (OnEditorRequest);
    /// `picture`: an image.
    File { picture: bool },
    /// A list of lines, one row each.
    Strings,
    /// A list view's columns, a line each: `Caption|Width`.
    Columns,
    /// Another component's name, of one of these types (any when empty).
    Component(Vec<String>),
    /// Text with an editor of the program's (`multiline`, `sql`,
    /// `expression`): "…" asks for it.
    Editor(String),
}

impl Kind {
    /// The kind of registry property `p`.
    pub fn of(p: &rapidr_lang::Property) -> Option<Kind> {
        use rapidr_lang::Type as T;
        let list = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        Some(match (p.ty, p.editor) {
            (_, Some("strings")) => Kind::Strings,
            (_, Some("columns")) => Kind::Columns,
            (_, Some("file")) => Kind::File { picture: false },
            (_, Some("picture")) => Kind::File { picture: true },
            (T::String | T::Any, Some(e)) => Kind::Editor(e.to_string()),
            (T::Int, _) => Kind::Int,
            (T::Float, _) => Kind::Float,
            (T::String | T::Any, _) => Kind::Text,
            (T::Bool, _) => Kind::Bool,
            (T::Color, _) => Kind::Color,
            (T::Enum, _) if !p.values.is_empty() => Kind::Enum(list(p.values)),
            (T::Enum, _) => Kind::Text,
            (T::Set, _) => Kind::Set(list(p.values)),
            (T::Font, _) => Kind::Font,
            (T::Component, _) => Kind::Component(list(p.kinds)),
            (T::Picture, _) => Kind::File { picture: true },
            (T::Resource | T::Item, _) => return None,
        })
    }

    /// The kind AddProperty's Type names: "int", "float", "string", "bool",
    /// "color", "file", "picture", "font", "strings", "columns",
    /// "component", constants separated by "|" (an enum) or "set:" and
    /// flags separated by "|".
    pub fn named(ty: &str) -> Kind {
        let t = ty.trim();
        let list = |s: &str| s.split('|').map(str::trim).filter(|v| !v.is_empty()).map(str::to_string).collect::<Vec<_>>();
        if let Some(flags) = t.strip_prefix("set:").or_else(|| t.strip_prefix("SET:")) {
            return Kind::Set(list(flags));
        }
        if t.contains('|') {
            return Kind::Enum(list(t));
        }
        match t.to_ascii_lowercase().as_str() {
            "int" | "integer" | "long" => Kind::Int,
            "float" | "double" | "single" => Kind::Float,
            "bool" | "boolean" => Kind::Bool,
            "color" | "colour" => Kind::Color,
            "file" => Kind::File { picture: false },
            "picture" | "image" => Kind::File { picture: true },
            "font" => Kind::Font,
            "strings" => Kind::Strings,
            "columns" => Kind::Columns,
            "component" => Kind::Component(Vec::new()),
            _ => Kind::Text,
        }
    }

    /// A row of this kind opens into parts (a set's flags, a font's parts,
    /// a list's lines, a colour's picker).
    pub fn has_parts(&self) -> bool {
        matches!(self, Kind::Set(_) | Kind::Font | Kind::Strings | Kind::Columns | Kind::Color)
    }

    /// Its value is picked from a dropped list.
    pub fn drops(&self) -> bool {
        matches!(self, Kind::Enum(_) | Kind::Component(_) | Kind::Bool)
    }

    /// It has a "…" button (OnEditorRequest).
    pub fn ellipsis(&self) -> bool {
        matches!(self, Kind::File { .. } | Kind::Editor(_) | Kind::Font | Kind::Strings | Kind::Columns)
    }

    /// Its value is typed in place (else only picked or toggled).
    pub fn typed(&self) -> bool {
        !matches!(self, Kind::Bool | Kind::Font | Kind::Strings | Kind::Columns)
    }

    /// The value a property of this kind has when nothing set it.
    pub fn empty(&self) -> Value {
        match self {
            Kind::Int | Kind::Bool | Kind::Color | Kind::Set(_) => Value::Integer(0),
            Kind::Float => Value::Double(0.0),
            Kind::Enum(v) => const_of(v, 0),
            _ => Value::String(String::new()),
        }
    }
}

/// Enum / set constant `i` of `values` as a runtime keeps it: the
/// constant's number when the language has it, else its index (an enum)
/// or bit (a set) — an enum of words without constants keeps the word.
fn const_of(values: &[String], i: usize) -> Value {
    match values.get(i).and_then(|n| rapidr_lang::constant(n)) {
        Some((v, _)) => Value::Integer(v),
        None if values.iter().all(|n| rapidr_lang::constant(n).is_none()) => Value::String(values.get(i).cloned().unwrap_or_default()),
        None => Value::Integer(i as i64),
    }
}

/// Flag `i` of a set's `values`: its bit.
pub fn flag_bit(values: &[String], i: usize) -> i64 {
    match values.get(i).and_then(|n| rapidr_lang::constant(n)) {
        Some((v, _)) => v,
        None => 1 << i,
    }
}

/// RapidQ's True as runtimes keep it.
fn basic(b: bool) -> Value {
    Value::Integer(if b { -1 } else { 0 })
}

/// A True / False however it's written.
pub fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "-1" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" | "" => Some(false),
        _ => None,
    }
}

/// A whole number: decimal, `&H` hex, a constant.
pub fn parse_int(s: &str) -> Option<i64> {
    let t = s.trim();
    if let Some(h) = t.strip_prefix("&H").or_else(|| t.strip_prefix("&h")) {
        return i64::from_str_radix(h, 16).ok().map(|v| if h.len() == 8 && v > i64::from(i32::MAX) { v - (1 << 32) } else { v });
    }
    if let Ok(v) = t.parse::<i64>() {
        return Some(v);
    }
    if let Ok(f) = t.parse::<f64>() {
        if f.fract() == 0.0 && f.abs() < 9.0e15 {
            return Some(f as i64);
        }
    }
    rapidr_lang::eval_constant(t).filter(|_| !t.is_empty())
}

/// A colour's name (clRed, clBtnFace …) when it is one of the language's.
pub fn color_name(v: i64) -> Option<&'static str> {
    // (the names people write first: clGray before clDkGray, clLime before
    // clGreen's twin, clFuchsia before clPurple's)
    const PREFERRED: [&str; 16] = ["clBlack", "clMaroon", "clGreen", "clOlive", "clNavy", "clPurple", "clTeal", "clGray", "clSilver", "clRed", "clLime", "clYellow", "clBlue", "clFuchsia", "clAqua", "clWhite"];
    if let Some(n) = PREFERRED.iter().find(|n| rapidr_lang::constant(n).map(|c| c.0) == Some(v)) {
        return Some(n);
    }
    SYSTEM_COLORS.iter().find(|n| rapidr_lang::constant(n).map(|c| c.0) == Some(v)).copied()
}

/// The 16 colours of the picker's first grid, in the order Windows' and
/// Delphi's palettes show them.
pub const STANDARD_COLORS: [&str; 16] = ["clBlack", "clMaroon", "clGreen", "clOlive", "clNavy", "clPurple", "clTeal", "clGray", "clSilver", "clRed", "clLime", "clYellow", "clBlue", "clFuchsia", "clAqua", "clWhite"];

/// The system's colours (a control with one follows the theme), in the
/// picker's second grid.
pub const SYSTEM_COLORS: [&str; 24] = [
    "clBtnFace",
    "clBtnText",
    "clWindow",
    "clWindowText",
    "clHighlight",
    "clHighlightText",
    "clGrayText",
    "clBtnShadow",
    "clBtnHighlight",
    "cl3DDkShadow",
    "cl3DLight",
    "clActiveCaption",
    "clCaptionText",
    "clInActiveCaption",
    "clInActiveCaptionText",
    "clMenu",
    "clMenuText",
    "clInfoText",
    "clScrollBar",
    "clBackGround",
    "clAppWorkSpace",
    "clActiveBorder",
    "clInActiveBorder",
    "clWindowFrame",
];

/// A colour constant's value.
pub fn color_value(name: &str) -> Option<i64> {
    rapidr_lang::constant(name).map(|c| c.0)
}

/// Whether `v` is one of the system's colours (they follow the theme).
pub fn is_system_color(v: i64) -> bool {
    let n = v as u32;
    n & 0xFF00_0000 == 0x8000_0000 && n & 0x00FF_FF00 == 0
}

/// `&HBBGGRR`, RapidQ's way to write a colour (six hex digits).
pub fn color_hex(v: i64) -> String {
    if is_system_color(v) {
        return format!("&H{:08X}", v as u32);
    }
    format!("&H{:06X}", v & 0xFF_FFFF)
}

/// A colour typed: a name (clRed), `&HBBGGRR`, `#RRGGBB` (the web's way:
/// turned around), a number.
pub fn parse_color(s: &str) -> Option<i64> {
    let t = s.trim();
    if let Some(v) = color_value(t).filter(|_| t.to_ascii_lowercase().starts_with("cl")) {
        return Some(v);
    }
    if let Some(h) = t.strip_prefix('#') {
        let rgb = u32::from_str_radix(h, 16).ok().filter(|_| h.len() == 6)?;
        return Some(i64::from(((rgb & 0xFF) << 16) | (rgb & 0xFF00) | (rgb >> 16)));
    }
    parse_int(t)
}

/// A runtime's value (or a designer's text) as the value of `kind`, the
/// way a runtime keeps it: `None` when it can't be one.
pub fn normalize(kind: &Kind, v: &Value) -> Option<Value> {
    let text = || v.to_string_val();
    let num = |v: &Value| match v {
        Value::Integer(i) => Some(*i),
        Value::Double(d) => Some(*d as i64),
        Value::Null => None,
        v => parse_int(&v.to_string_val()),
    };
    Some(match kind {
        Kind::Int => Value::Integer(num(v)?),
        Kind::Float => match v {
            Value::Integer(i) => Value::Double(*i as f64),
            Value::Double(d) => Value::Double(*d),
            v => Value::Double(v.to_string_val().trim().parse::<f64>().ok().or_else(|| parse_int(&v.to_string_val()).map(|i| i as f64))?),
        },
        Kind::Bool => match v {
            Value::String(s) => basic(parse_bool(s)?),
            Value::Null => return None,
            v => basic(v.to_bool()),
        },
        Kind::Enum(values) => match v {
            Value::String(s) => {
                let s = s.trim();
                match values.iter().position(|n| n.eq_ignore_ascii_case(s)) {
                    Some(i) => const_of(values, i),
                    None => Value::Integer(parse_int(s)?),
                }
            }
            v => Value::Integer(num(v)?),
        },
        Kind::Set(values) => Value::Integer(match v {
            Value::String(s) => parse_set(values, s)?,
            v => num(v)?,
        }),
        Kind::Color => Value::Integer(match v {
            Value::String(s) => parse_color(s)?,
            v => num(v)?,
        }),
        Kind::Font => return None,
        _ => match v {
            Value::Null => return None,
            _ => Value::String(text()),
        },
    })
}

/// A set written as flags: "akLeft, akTop", "akLeft + akTop",
/// "[akLeft,akTop]", a number.
pub fn parse_set(values: &[String], s: &str) -> Option<i64> {
    let t = s.trim().trim_start_matches('[').trim_end_matches(']').trim();
    if t.is_empty() {
        return Some(0);
    }
    if let Some(n) = parse_int(t).filter(|_| t.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '&' || c == '-')) {
        return Some(n);
    }
    let mut mask = 0;
    for part in t.split([',', '+', '|']).map(str::trim).filter(|p| !p.is_empty()) {
        let i = values.iter().position(|n| n.eq_ignore_ascii_case(part))?;
        mask |= flag_bit(values, i);
    }
    Some(mask)
}

/// The flags of `values` set in `mask`.
pub fn set_flags(values: &[String], mask: i64) -> Vec<&str> {
    values.iter().enumerate().filter(|(i, _)| mask & flag_bit(values, *i) != 0).map(|(_, n)| n.as_str()).collect()
}

/// A value of `kind` (as a runtime keeps it) as its row shows it.
pub fn display(kind: &Kind, v: &Value) -> String {
    let Some(v) = normalize(kind, v) else { return v.to_string_val() };
    match kind {
        Kind::Int => v.to_i64().to_string(),
        Kind::Float => float_text(v.to_f64()),
        Kind::Bool => (if v.to_i64() != 0 { "True" } else { "False" }).into(),
        Kind::Enum(values) => enum_name(values, &v).unwrap_or_else(|| v.to_string_val()),
        Kind::Set(values) => set_flags(values, v.to_i64()).join(", "),
        Kind::Color => color_name(v.to_i64()).map(str::to_string).unwrap_or_else(|| color_hex(v.to_i64())),
        Kind::Strings => count_text(&v.to_string_val(), "line"),
        Kind::Columns => count_text(&v.to_string_val(), "column"),
        _ => v.to_string_val().replace(['\r', '\n'], " "),
    }
}

/// A value of `kind` as the program writes it: a constant's name, flags
/// added (`akLeft + akTop`), `True` / `False`, `&HBBGGRR` or a colour's
/// name, a number, the text itself.
pub fn source(kind: &Kind, v: &Value) -> String {
    let Some(v) = normalize(kind, v) else { return v.to_string_val() };
    match kind {
        Kind::Set(values) => {
            let flags = set_flags(values, v.to_i64());
            if flags.is_empty() {
                "0".into()
            } else {
                flags.join(" + ")
            }
        }
        Kind::Strings | Kind::Columns => v.to_string_val(),
        _ => display(kind, &v),
    }
}

/// What the user typed into a row of `kind`, as a runtime's value; `Err`
/// says why it can't be one.
pub fn parse(kind: &Kind, text: &str) -> Result<Value, String> {
    let t = text.trim();
    let bad = |what: &str| Err(format!("\"{t}\" is not {what}"));
    match kind {
        Kind::Int => match parse_int(t) {
            Some(i) => Ok(Value::Integer(i)),
            None => bad("a whole number"),
        },
        Kind::Float => match t.parse::<f64>().ok().or_else(|| parse_int(t).map(|i| i as f64)) {
            Some(f) if f.is_finite() => Ok(Value::Double(f)),
            _ => bad("a number"),
        },
        Kind::Bool => match parse_bool(t) {
            Some(b) => Ok(basic(b)),
            None => bad("True or False"),
        },
        Kind::Enum(values) => match values.iter().position(|n| n.eq_ignore_ascii_case(t)) {
            Some(i) => Ok(const_of(values, i)),
            None => match parse_int(t).filter(|_| !t.is_empty()) {
                Some(n) if values.iter().enumerate().any(|(i, _)| const_of(values, i) == Value::Integer(n)) => Ok(Value::Integer(n)),
                _ => Err(format!("\"{t}\" is not one of {}", values.join(", "))),
            },
        },
        Kind::Set(values) => match parse_set(values, t) {
            Some(m) => Ok(Value::Integer(m)),
            None => Err(format!("\"{t}\" is not a set of {}", values.join(", "))),
        },
        Kind::Color => match parse_color(t) {
            Some(c) => Ok(Value::Integer(c)),
            None => bad("a colour (clRed, &HBBGGRR, #RRGGBB)"),
        },
        _ => Ok(Value::String(text.to_string())),
    }
}

/// An enum's constant name for a value.
fn enum_name(values: &[String], v: &Value) -> Option<String> {
    (0..values.len()).find(|&i| const_of(values, i) == *v).map(|i| values[i].clone())
}

/// A number as short as it reads (no trailing zeros).
pub fn float_text(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        return format!("{}", f as i64);
    }
    let s = format!("{f}");
    s
}

/// "3 lines", "1 line", "(none)".
fn count_text(text: &str, what: &str) -> String {
    let n = lines(text).len();
    match n {
        0 => "(none)".into(),
        1 => format!("1 {what}"),
        n => format!("{n} {what}s"),
    }
}

/// A list's lines (CR LF, LF or CR between them; none when empty).
pub fn lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.replace("\r\n", "\n").replace('\r', "\n").split('\n').map(str::to_string).collect()
}

/// Whether two values of `kind` are the same value.
pub fn same(kind: &Kind, a: &Value, b: &Value) -> bool {
    match (normalize(kind, a), normalize(kind, b)) {
        (Some(Value::Double(x)), Some(Value::Double(y))) => (x - y).abs() < 1e-9,
        (Some(x), Some(y)) => x == y,
        _ => a.to_string_val() == b.to_string_val(),
    }
}

/// A font as its row shows it: "Arial, 10 pt, Bold Italic".
pub fn font_text(name: &str, size: i64, bold: bool, italic: bool, underline: bool, strike: bool) -> String {
    let mut s = format!("{name}, {size} pt");
    let styles: Vec<&str> = [(bold, "Bold"), (italic, "Italic"), (underline, "Underline"), (strike, "StrikeOut")].into_iter().filter(|(on, _)| *on).map(|(_, n)| n).collect();
    if !styles.is_empty() {
        s.push_str(", ");
        s.push_str(&styles.join(" "));
    }
    s
}

/// A font's parts as rows: (its name after `Font.`, its kind).
pub const FONT_PARTS: [(&str, FontPart); 7] = [
    ("Name", FontPart::Name),
    ("Size", FontPart::Size),
    ("Color", FontPart::Color),
    ("Bold", FontPart::Bold),
    ("Italic", FontPart::Italic),
    ("Underline", FontPart::Underline),
    ("StrikeOut", FontPart::StrikeOut),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontPart {
    Name,
    Size,
    Color,
    Bold,
    Italic,
    Underline,
    StrikeOut,
}

impl FontPart {
    pub fn kind(self) -> Kind {
        match self {
            FontPart::Name => Kind::Text,
            FontPart::Size => Kind::Int,
            FontPart::Color => Kind::Color,
            _ => Kind::Bool,
        }
    }

    /// The runtimes' flat property (`fontname` …) for it.
    pub fn flat(self) -> &'static str {
        match self {
            FontPart::Name => "fontname",
            FontPart::Size => "fontsize",
            FontPart::Color => "fontcolor",
            FontPart::Bold => "fontbold",
            FontPart::Italic => "fontitalic",
            FontPart::Underline => "fontunderline",
            FontPart::StrikeOut => "fontstrikeout",
        }
    }

    /// Its default (RapidQ's MS Sans Serif 8, black, plain).
    pub fn default(self) -> Value {
        match self {
            FontPart::Name => Value::String(crate::objects::DEFAULT_FONT_NAME.into()),
            FontPart::Size => Value::Integer(crate::objects::DEFAULT_FONT_SIZE),
            FontPart::Color => Value::Integer(crate::component_defaults::CL_WINDOW_TEXT),
            _ => Value::Integer(0),
        }
    }
}
