//! One value drawn as a row (docs/preview4-plan.md S10): what the
//! debugger's Variables and Watch, its data tips, and the Properties panel
//! and the code's helpers (docs/code-helpers-plan.md E0 / E1) show for a
//! value — its kind's icon, its name, ` = `, then the value styled by kind:
//!
//! ```text
//!   [00] Score = 42
//!   [“ ] Title = ""                       (an empty text, quiet)
//!   [▢ ] FontColor = ■ clWindowText       (a swatch in the theme's own colour)
//!   [▮▯] ColName = Array(0 TO 64) · 65 items
//!   [lv] Stats = RListView                (the component's own icon)
//!   [00] Lives = 2 Δ                      (changed since the last stop)
//! ```
//!
//! - **A value and its kind in, a drawn and accessible row out.** Build a
//!   [`ValueRow`] once, when the value arrives ([`ValueRow::new`] from a
//!   [`ValueInfo`]: the debugger's kind and text, what the registry knows);
//!   paint it every frame ([`ValueRow::paint`]). The row keeps its text,
//!   its parts, its icon (found once) and its accessible name, so a frame
//!   costs a few display-list ops per visible row and nothing else: no
//!   lookups by name, no formatting, the icon's picture made once per size
//!   and theme and put in the list once however many rows show it.
//! - **Colours are the theme's** (`rapidr_value::ide_theme::ValueColors`,
//!   `debug.*`: the code editor's number, string, keyword, constant and type
//!   colours, each 4.5:1 on the window, 7:1 in high contrast). A selected
//!   row is drawn in its selection's text colour; a disabled one in the
//!   theme's grey.
//! - **Colours as colours.** A colour value (the registry's `color` type:
//!   [`Hint::Color`]) shows a swatch and its constant's name when it has one
//!   (`clRed`), else RapidQ's `&HBBGGRR`. A system colour (`clWindowText`,
//!   -2147483640) is drawn in what it is in the theme the program runs in.
//! - **Screen readers** read each row as a sentence
//!   ([`ValueRow::accessible_name`]): "FontColor, colour clWindowText",
//!   "Title, empty text", "ColName, array 0 to 64, 65 items", "Lives,
//!   whole number 2, changed". The icon never stands in for a word.

use std::cell::RefCell;
use std::ops::Range;

use rapidr_value::ide_theme::{self, ValueColors};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;
use rapidr_value::panels::inspector::values::{color_hex, color_name, is_system_color};
use rapidr_value::theme::Theme;

use crate::paint::Painter;

/// What a value is: its icon, how its text is styled, how it's read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    /// INTEGER, LONG, BYTE, WORD, DWORD, SHORT.
    Integer,
    /// SINGLE, DOUBLE.
    Decimal,
    /// STRING.
    Text,
    /// True / False.
    Boolean,
    /// An array (`Array(0 TO 64)`; its elements are its children).
    Array,
    /// A TYPE's value (its fields are its children).
    Record,
    /// A component (a handler's `Sender`, a form): its type's own icon.
    Component,
    /// A constant's name (`alClient`): the registry knows the property's
    /// constants ([`Hint::Constant`]).
    Constant,
    /// A colour (the registry's `color` type, [`Hint::Color`]).
    Color,
    /// A number that stands for something the system keeps (a window, a
    /// file, a picture: [`Hint::Handle`]).
    Handle,
    /// Nothing yet (an uninitialised variant).
    Empty,
    /// An RDataFrame's row.
    Row,
    /// The names an `$INCLUDE`d file made (RAPIDQ.INC's constants), as one
    /// closed item.
    Include,
    /// A form's components, as one item.
    Components,
    /// "… 40 more": the next page of a long array.
    More,
    // ---- sections (headings, no value) ----
    Locals,
    Globals,
    Watch,
    /// The Call Stack's heading.
    CallStack,
    /// A call on the stack (Call Stack's rows).
    Frame,
    /// The call the program is stopped in.
    CurrentFrame,
}

impl ValueKind {
    /// Its icon's key in the icon set's `[debug-kinds]` (rapidr_icons::debug_kind).
    pub fn icon_key(self) -> &'static str {
        match self {
            ValueKind::Integer => "integer",
            ValueKind::Decimal => "decimal",
            ValueKind::Text => "text",
            ValueKind::Boolean => "boolean",
            ValueKind::Array => "array",
            ValueKind::Record => "record",
            ValueKind::Component => "component",
            ValueKind::Constant => "constant",
            ValueKind::Color => "color",
            ValueKind::Handle => "handle",
            ValueKind::Empty => "empty",
            ValueKind::Row => "row",
            ValueKind::Include => "include",
            ValueKind::Components => "components",
            ValueKind::More => "more",
            ValueKind::Locals => "locals",
            ValueKind::Globals => "globals",
            ValueKind::Watch => "watch",
            ValueKind::CallStack => "call-stack",
            ValueKind::Frame => "frame",
            ValueKind::CurrentFrame => "current-frame",
        }
    }

    /// Its name, read before the value ("whole number 2").
    pub fn words(self) -> &'static str {
        match self {
            ValueKind::Integer => "whole number",
            ValueKind::Decimal => "decimal number",
            ValueKind::Text => "text",
            ValueKind::Boolean => "true or false",
            ValueKind::Array => "array",
            ValueKind::Record => "record",
            ValueKind::Component => "component",
            ValueKind::Constant => "constant",
            ValueKind::Color => "colour",
            ValueKind::Handle => "handle",
            ValueKind::Empty => "empty",
            ValueKind::Row => "row",
            ValueKind::Include => "included names",
            ValueKind::Components => "components",
            ValueKind::More => "more",
            ValueKind::Locals => "locals",
            ValueKind::Globals => "globals",
            ValueKind::Watch => "watch",
            ValueKind::CallStack => "call stack",
            ValueKind::Frame => "call",
            ValueKind::CurrentFrame => "current call",
        }
    }

    /// A section's heading or a call on the stack (no ` = `, no value).
    pub fn is_section(self) -> bool {
        matches!(self, ValueKind::Locals | ValueKind::Globals | ValueKind::Watch | ValueKind::CallStack | ValueKind::Frame | ValueKind::CurrentFrame)
    }

    /// A heading, drawn bold (Locals, Globals, Watch, Call Stack; a call on
    /// the stack is a plain row).
    fn heading(self) -> bool {
        matches!(self, ValueKind::Locals | ValueKind::Globals | ValueKind::Watch | ValueKind::CallStack)
    }

    /// The kind of a value the debugger describes as `kind`
    /// (rapidr_session's `Variable::kind`: `Integer`, `Double`, `String`,
    /// `Boolean`, `Array`, `Empty`, `Include`, `Components`, `Row`, a
    /// data frame's column types `i64` / `f64` / `str` / `bool`, a
    /// component's type, else a TYPE's name), with what the registry knows
    /// (`hint`) and its text.
    pub fn classify(kind: &str, text: &str, hint: Hint<'_>) -> ValueKind {
        let k = kind.trim();
        let numeric = matches!(classify_plain(k), Some(ValueKind::Integer | ValueKind::Decimal));
        match hint {
            Hint::Color if numeric && parse_whole(text).is_some() => return ValueKind::Color,
            Hint::Handle if numeric => return ValueKind::Handle,
            Hint::Constant(_) => return ValueKind::Constant,
            _ => {}
        }
        if let Some(plain) = classify_plain(k) {
            return plain;
        }
        let t = text.trim();
        if t == "(null)" {
            return ValueKind::Empty;
        }
        // (the debugger shows a component as `"Button1" (RButton)`; a
        // TYPE's value as its type's name)
        let shown_as_component = t.starts_with('"') && t.contains("\" (");
        if shown_as_component || (rapidr_icons::component(k).is_some() && !t.eq_ignore_ascii_case(k)) {
            return ValueKind::Component;
        }
        ValueKind::Record
    }
}

fn classify_plain(k: &str) -> Option<ValueKind> {
    let lower = k.to_ascii_lowercase();
    Some(match lower.as_str() {
        "integer" | "long" | "byte" | "word" | "dword" | "short" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "int" => ValueKind::Integer,
        "double" | "single" | "f32" | "f64" | "float" | "decimal" => ValueKind::Decimal,
        "string" | "str" | "text" => ValueKind::Text,
        "boolean" | "bool" => ValueKind::Boolean,
        "array" => ValueKind::Array,
        "empty" | "null" | "" => ValueKind::Empty,
        "include" => ValueKind::Include,
        "components" => ValueKind::Components,
        "row" => ValueKind::Row,
        "more" => ValueKind::More,
        _ => return None,
    })
}

/// What the language registry knows about a value beyond its kind (a
/// property's type, an enum's constants): how to show it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hint<'a> {
    #[default]
    None,
    /// A colour (`type = "color"`: RapidQ's BGR number, a system colour).
    Color,
    /// A handle (a window's, a file's).
    Handle,
    /// One of the property's constants, by name (`alClient`).
    Constant(&'a str),
}

/// A value, as the debugger (or the Properties panel) has it.
#[derive(Clone, Copy, Debug, Default)]
pub struct ValueInfo<'a> {
    /// Its name (`Score`, `a(2)`, `Form1.Caption`); empty: none (a row of
    /// a table, an expression's result alone).
    pub name: &'a str,
    /// The debugger's kind (rapidr_session's `Variable::kind`).
    pub kind: &'a str,
    /// Its text as the debugger shows it: a text quoted (`"Hi"`), an array
    /// `Array(0 TO 9)`, a component `"Button1" (RButton)`.
    pub text: &'a str,
    /// How many children (an array's elements, a TYPE's fields).
    pub count: u32,
    /// It changed since the program last stopped.
    pub changed: bool,
    /// What a FUNCTION returned (its result's icon; styled as its value).
    pub result: bool,
    pub hint: Hint<'a>,
}

/// What a part of the row is: its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Name,
    Punctuation,
    Number,
    String,
    Keyword,
    Constant,
    Type,
    Dim,
    /// A section's title (bold).
    Heading,
}

impl Part {
    fn color(self, c: &ValueColors) -> u32 {
        match self {
            Part::Name | Part::Heading => c.name,
            Part::Punctuation => c.punctuation,
            Part::Number => c.number,
            Part::String => c.string,
            Part::Keyword => c.keyword,
            Part::Constant => c.constant,
            Part::Type => c.type_name,
            Part::Dim => c.dim,
        }
    }
}

/// A colour shown as a swatch: RapidQ's BGR, or a system colour's index
/// (`&H80000000 + n`), drawn in what it is in the program's theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Swatch {
    Bgr(u32),
    System(u32),
}

impl Swatch {
    /// A RapidQ colour value's swatch (`None`: not a colour: negative and
    /// not a system colour, or past &HFFFFFF).
    pub fn of(v: i64) -> Option<Swatch> {
        if is_system_color(v) {
            return Some(Swatch::System((v as u32) & 0xFF));
        }
        (0..=0xFF_FFFF).contains(&v).then_some(Swatch::Bgr(v as u32))
    }

    /// Its colour (0xRRGGBB) in `theme`.
    pub fn rgb(self, theme: &Theme) -> u32 {
        match self {
            Swatch::Bgr(bgr) => ((bgr & 0xFF) << 16) | (bgr & 0xFF00) | ((bgr >> 16) & 0xFF),
            Swatch::System(i) => theme.system_color(i),
        }
    }
}

/// How a row is drawn this frame.
#[derive(Clone, Copy, Debug)]
pub struct RowStyle<'a> {
    pub font: &'a Font,
    /// The selection's text colour when the row is selected (what the
    /// list's selection fill returns: `components::list::selected_row`):
    /// every part is drawn in it.
    pub selected_ink: Option<u32>,
    pub enabled: bool,
    /// The theme the program's system colours resolve in (the program's
    /// own, when it differs from Studio's); `None`: the painter's.
    pub program_theme: Option<&'static Theme>,
}

impl<'a> RowStyle<'a> {
    pub fn new(font: &'a Font) -> Self {
        RowStyle { font, selected_ink: None, enabled: true, program_theme: None }
    }
}

/// The gap between the icon and the text, and before the changed mark.
const GAP: i64 = 4;

/// A value ready to draw: built once, painted every frame
/// ([module docs](self)).
#[derive(Debug)]
pub struct ValueRow {
    kind: ValueKind,
    icon: Option<&'static rapidr_icons::Icon>,
    /// Everything the row says, its parts' ranges into it.
    text: String,
    parts: Vec<(Range<usize>, Part)>,
    /// Where the swatch goes: before part `swatch_at`.
    swatch: Option<(Swatch, usize)>,
    /// The value's first part (after the name and ` = `).
    value_at: usize,
    changed: bool,
    access: String,
    /// The parts' widths in the font they were last measured in.
    widths: RefCell<Option<(FontKey, Vec<i64>)>>,
}

type FontKey = (u64, i64, u8);

fn font_key(f: &Font) -> FontKey {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    f.name.hash(&mut h);
    (h.finish(), f.size, f.styles)
}

/// A whole number written as the debugger writes it (`-2147483640`, `&HFF`).
fn parse_whole(s: &str) -> Option<i64> {
    let t = s.trim();
    if let Some(h) = t.strip_prefix("&H").or_else(|| t.strip_prefix("&h")) {
        return i64::from_str_radix(h, 16).ok();
    }
    t.parse::<i64>().ok().or_else(|| t.parse::<f64>().ok().filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15).map(|f| f as i64))
}

/// "1 item", "65 items".
fn count(n: u32, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

struct Builder {
    text: String,
    parts: Vec<(Range<usize>, Part)>,
}

impl Builder {
    fn push(&mut self, s: &str, part: Part) {
        if s.is_empty() {
            return;
        }
        let at = self.text.len();
        self.text.push_str(s);
        // (two parts of one colour side by side are one run)
        match self.parts.last_mut() {
            Some((r, p)) if *p == part && r.end == at => r.end = self.text.len(),
            _ => self.parts.push((at..self.text.len(), part)),
        }
    }
}

impl ValueRow {
    /// A value's row ([`ValueInfo`]).
    pub fn new(v: &ValueInfo<'_>) -> ValueRow {
        let kind = ValueKind::classify(v.kind, v.text, v.hint);
        let mut b = Builder { text: String::with_capacity(v.name.len() + v.text.len() + 16), parts: Vec::with_capacity(4) };
        let mut swatch = None;
        let text = v.text.trim();
        let name = v.name.trim();
        let pairs = !name.is_empty() && kind != ValueKind::More;
        // (an $INCLUDE's names, a form's components: a name and a count,
        // `$INCLUDE RAPIDQ.INC (412 names)`, as the debugger always wrote them)
        let grouped = matches!(kind, ValueKind::Include | ValueKind::Components);
        if pairs {
            b.push(name, Part::Name);
            b.push(if grouped { " " } else { " = " }, Part::Punctuation);
        }
        let value_at = b.parts.len();
        // what the value is read as, after the name
        let said: String = match kind {
            ValueKind::Integer | ValueKind::Decimal | ValueKind::Handle => {
                b.push(text, Part::Number);
                format!("{} {text}", kind.words())
            }
            ValueKind::Text => {
                let inner = text.strip_prefix('"').and_then(|t| t.strip_suffix('"')).unwrap_or(text);
                if inner.is_empty() {
                    b.push("\"\"", Part::Dim);
                    "empty text".to_string()
                } else {
                    b.push(text, Part::String);
                    format!("text {inner}")
                }
            }
            ValueKind::Boolean => {
                b.push(text, Part::Keyword);
                text.to_string()
            }
            ValueKind::Array => {
                b.push(text, Part::Type);
                if v.count > 0 {
                    b.push(" · ", Part::Punctuation);
                    b.push(&count(v.count, "item", "items"), Part::Dim);
                }
                let bounds = text.strip_prefix("Array(").and_then(|t| t.strip_suffix(')')).unwrap_or(text).replace(" TO ", " to ");
                format!("array {bounds}, {}", count(v.count, "item", "items"))
            }
            ValueKind::Record => {
                b.push(text, Part::Type);
                if v.count > 0 {
                    b.push(" · ", Part::Punctuation);
                    b.push(&count(v.count, "field", "fields"), Part::Dim);
                }
                format!("record {text}, {}", count(v.count, "field", "fields"))
            }
            ValueKind::Component => {
                // `"Button1" (RButton)`, `"df" (RDataFrame: 3 rows × 2 columns)`
                let (id, detail) = match text.strip_prefix('"').and_then(|t| t.split_once("\" (")) {
                    Some((id, rest)) => (id, rest.strip_suffix(')').unwrap_or(rest)),
                    None => ("", text),
                };
                let cut = detail.find([':', '(']).unwrap_or(detail.len());
                let (ty, more) = detail.split_at(cut);
                let ty = if ty.trim().is_empty() { v.kind.trim() } else { ty.trim_end() };
                let same = id.is_empty() || id.eq_ignore_ascii_case(name);
                if !same {
                    b.push(id, Part::Constant);
                    b.push(" · ", Part::Punctuation);
                }
                b.push(ty, Part::Type);
                b.push(more, Part::Dim);
                if same { format!("{ty}{more}") } else { format!("{ty} {id}{more}") }
            }
            ValueKind::Constant => {
                let Hint::Constant(c) = v.hint else { unreachable!("a constant comes from its hint") };
                b.push(c, Part::Constant);
                if !text.is_empty() && !text.eq_ignore_ascii_case(c) {
                    b.push(" (", Part::Dim);
                    b.push(text, Part::Dim);
                    b.push(")", Part::Dim);
                }
                format!("constant {c}")
            }
            ValueKind::Color => {
                let n = parse_whole(text).unwrap_or(0);
                match Swatch::of(n) {
                    Some(s) => {
                        swatch = Some((s, b.parts.len()));
                        match color_name(n) {
                            Some(c) => {
                                b.push(c, Part::Constant);
                                format!("colour {c}")
                            }
                            None => {
                                b.push(&color_hex(n), Part::Number);
                                match s {
                                    Swatch::Bgr(bgr) => format!("colour red {}, green {}, blue {}", bgr & 0xFF, (bgr >> 8) & 0xFF, (bgr >> 16) & 0xFF),
                                    Swatch::System(i) => format!("colour, the system's colour {i}"),
                                }
                            }
                        }
                    }
                    None => {
                        b.push(text, Part::Number);
                        format!("whole number {text}")
                    }
                }
            }
            ValueKind::Empty => {
                b.push("Empty", Part::Dim);
                "empty".to_string()
            }
            ValueKind::Row => {
                b.push(text, Part::Dim);
                format!("row, {text}")
            }
            ValueKind::Include | ValueKind::Components => {
                b.push(text, Part::Dim);
                text.trim_start_matches('(').trim_end_matches(')').to_string()
            }
            ValueKind::More => {
                b.push(if text.is_empty() { name } else { text }, Part::Dim);
                String::new()
            }
            ValueKind::Locals | ValueKind::Globals | ValueKind::Watch | ValueKind::CallStack | ValueKind::Frame | ValueKind::CurrentFrame => {
                // (a section: built by ValueRow::section)
                b.push(text, Part::Dim);
                text.to_string()
            }
        };
        let mut access = String::with_capacity(name.len() + said.len() + 20);
        match kind {
            ValueKind::More => access.push_str(&b.text),
            _ if pairs => {
                access.push_str(name);
                if v.result {
                    access.push_str(", result");
                }
                access.push_str(", ");
                access.push_str(&said);
            }
            _ => access.push_str(&said),
        }
        if v.changed {
            access.push_str(", changed");
        }
        let icon = if v.result {
            rapidr_icons::get("debug/result")
        } else if kind == ValueKind::Component {
            rapidr_icons::component(v.kind).or_else(|| rapidr_icons::debug_kind("component"))
        } else {
            rapidr_icons::debug_kind(kind.icon_key())
        };
        ValueRow { kind, icon, text: b.text, parts: b.parts, swatch, value_at, changed: v.changed, access, widths: RefCell::new(None) }
    }

    /// A section's heading (Locals, Globals, Watch, a call on the stack):
    /// its icon and `title`, bold, read as the title.
    pub fn section(kind: ValueKind, title: &str) -> ValueRow {
        let mut b = Builder { text: String::new(), parts: Vec::with_capacity(1) };
        b.push(title, Part::Heading);
        ValueRow {
            kind,
            icon: rapidr_icons::debug_kind(kind.icon_key()),
            access: title.to_string(),
            text: b.text,
            parts: b.parts,
            swatch: None,
            value_at: 0,
            changed: false,
            widths: RefCell::new(None),
        }
    }

    pub fn kind(&self) -> ValueKind {
        self.kind
    }

    /// Its icon (the kind's; a component's type's own; a result's).
    pub fn icon(&self) -> Option<&'static rapidr_icons::Icon> {
        self.icon
    }

    /// What it says, as one line (`FontColor = clWindowText`): a tooltip's,
    /// Copy's.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Its parts: text and what each is.
    pub fn parts(&self) -> impl Iterator<Item = (&str, Part)> {
        self.parts.iter().map(|(r, p)| (&self.text[r.clone()], *p))
    }

    /// What a screen reader reads: "FontColor, colour clWindowText".
    pub fn accessible_name(&self) -> &str {
        &self.access
    }

    /// Its kind in words (the accessible description).
    pub fn description(&self) -> &'static str {
        self.kind.words()
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn swatch(&self) -> Option<Swatch> {
        self.swatch.map(|(s, _)| s)
    }

    /// The parts' widths in `font` (measured once per font).
    fn widths(&self, font: &Font) -> std::cell::Ref<'_, Vec<i64>> {
        let key = font_key(font);
        let stale = self.widths.borrow().as_ref().is_none_or(|(k, _)| *k != key);
        if stale {
            let bold = self.kind.heading().then(|| Font { styles: font.styles | 1, ..font.clone() });
            let f = bold.as_ref().unwrap_or(font);
            let w = self.parts.iter().map(|(r, _)| text_size(&self.text[r.clone()], f).0).collect();
            *self.widths.borrow_mut() = Some((key, w));
        }
        std::cell::Ref::map(self.widths.borrow(), |w| &w.as_ref().expect("measured").1)
    }

    /// How wide it is drawn in `font` with an icon `icon` px square (a
    /// list's horizontal scroll, a column's width).
    pub fn width(&self, font: &Font, icon: i64) -> i64 {
        let side = self.swatch.map_or(0, |_| swatch_side(icon) + GAP);
        let mark = if self.changed { GAP + mark_side(icon) } else { 0 };
        let lead = if self.icon.is_some() { icon + GAP } else { 0 };
        lead + self.widths(font).iter().sum::<i64>() + side + mark
    }

    /// Draws the row in `rect` (logical pixels; its height the row's): the
    /// icon `rect.3` high at most 16, the parts, the swatch; a changed value
    /// highlighted (`debug.changedback` behind it, framed in high contrast)
    /// and marked after it. Returns the width drawn.
    pub fn paint(&self, p: &mut Painter, rect: Rect, style: &RowStyle<'_>) -> i64 {
        let (x0, y, w, h) = rect;
        let t = p.theme();
        let colors = ide_theme::values(t);
        let ink = |part: Part| -> u32 {
            if !style.enabled {
                t.gray_text
            } else if let Some(s) = style.selected_ink {
                s
            } else {
                part.color(&colors)
            }
        };
        let icon = (h - 2).clamp(8, 16);
        let mut x = x0;
        if let Some(i) = self.icon {
            p.icon_of(i, (x, y + (h - icon) / 2, icon, icon), None, !style.enabled);
            x += icon + GAP;
        }
        let widths = self.widths(style.font);
        let bold = self.kind.heading().then(|| Font { styles: style.font.styles | 1, ..style.font.clone() });
        let font = bold.as_ref().unwrap_or(style.font);
        let right = x0 + w;
        // (changed: the value highlighted, under its swatch, text and mark)
        if self.changed && style.enabled && style.selected_ink.is_none() {
            let from = x + widths[..self.value_at.min(widths.len())].iter().sum::<i64>();
            let swatch = self.swatch.map_or(0, |_| swatch_side(icon) + GAP);
            let to = (from + swatch + widths[self.value_at.min(widths.len())..].iter().sum::<i64>() + GAP + mark_side(icon)).min(right);
            let r = (from - 3, y + 1, to - from + 6, h - 2);
            if t.contrast {
                p.round(r, 3.0, None, Some(colors.changed), 1.0);
            } else if t.fluent() {
                p.round(r, 3.0, Some(colors.changed_back), None, 1.0);
            } else {
                p.fill(r, colors.changed_back);
            }
        }
        for (k, ((range, part), tw)) in self.parts.iter().zip(widths.iter()).enumerate() {
            if self.swatch.is_some_and(|(_, at)| at == k) {
                let side = swatch_side(icon);
                let theme = style.program_theme.unwrap_or(t);
                let fill = self.swatch.map(|(s, _)| s.rgb(theme)).unwrap_or(0);
                swatch(p, (x, y + (h - side) / 2, side, side), fill, style.enabled);
                x += side + GAP;
            }
            if x >= right {
                break;
            }
            p.text((x, y, *tw + 2, h), &self.text[range.clone()], font, ink(*part), Place::Left);
            x += tw;
        }
        if self.changed && x < right {
            if let Some(mark) = changed_mark() {
                let side = mark_side(icon);
                let color = if style.selected_ink.is_some() || !style.enabled { ink(Part::Name) } else { colors.changed };
                p.icon_of(mark, (x + GAP, y + (h - side) / 2, side, side), Some(color), false);
                x += GAP + side;
            }
        }
        x - x0
    }
}

/// The swatch's side for an icon `icon` px square.
fn swatch_side(icon: i64) -> i64 {
    (icon - 4).max(6)
}

/// The changed mark's side.
fn mark_side(icon: i64) -> i64 {
    (icon * 3 / 4).max(8)
}

fn changed_mark() -> Option<&'static rapidr_icons::Icon> {
    static MARK: std::sync::OnceLock<Option<&'static rapidr_icons::Icon>> = std::sync::OnceLock::new();
    *MARK.get_or_init(|| rapidr_icons::debug_kind("changed"))
}

/// A colour's swatch: the colour in a frame that shows on any ground
/// (3:1, the theme's strong border), rounded in RapidR's look (the code
/// editor's colour marks draw theirs with it too).
pub fn swatch(p: &mut Painter, rect: Rect, fill: u32, enabled: bool) {
    let t = p.theme();
    let edge = if t.fluent() { t.border_strong } else { t.text };
    let edge = if enabled { edge } else { t.gray_text };
    if t.fluent() {
        p.round(rect, 2.0, Some(fill), Some(edge), 1.0);
    } else {
        p.fill(rect, fill);
        p.frame(rect, edge);
    }
}

#[cfg(test)]
mod tests;
