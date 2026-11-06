//! RNOTIFYBAR (lane NOTIFY; docs/design-guidelines.md, "Notifications",
//! "The decision strip"): a question the person must answer before work
//! goes on, as one slim line across the top of the thing it is about — the
//! mark in the warning colour, an icon, the question in bold, compact
//! buttons at the right end. Too narrow for one line, the buttons go under
//! the question. The model, the same on every runtime: the words, the
//! buttons, the choice, and the strip's rectangles ([`Bar::layout`]: pure
//! functions of the width, so the kernel paints, hit-tests and describes
//! from the same places, and the dock can ask how tall the strip wants to
//! be).

use super::notify::{middle, mnemonic, plain, Kind};
use super::runtime::{basic_bool, Runtime};
use crate::objects::font::Font;
use crate::objects::ops::Rect;
use crate::objects::text::text_size;
use crate::Value;

/// Where the words start, and the gaps (the guide: 4-px mark, 12, icon 16, 8).
pub const TEXT_X: i64 = 40;
pub const EDGE: i64 = 12;
pub const BUTTON_GAP: i64 = 8;
pub const MARK: i64 = 4;
/// A compact button is at least this wide (the guide: 72).
pub const BUTTON_MIN: i64 = 72;
/// A question is cut to this many characters before the buttons go under it.
pub const KEEP_CHARS: usize = 24;

/// One strip's state.
#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub text: String,
    pub kind: Kind,
    /// Captions, `&` before the letter that presses each.
    pub buttons: Vec<String>,
    /// The button chosen last (1-based; 0 none yet).
    pub choice: i64,
    pub hover: Option<usize>,
    pub pressed: Option<usize>,
    /// The button the keyboard is on.
    pub focus: usize,
}

impl Default for Bar {
    fn default() -> Self {
        Bar { text: String::new(), kind: Kind::Warning, buttons: Vec::new(), choice: 0, hover: None, pressed: None, focus: 0 }
    }
}

crate::panel_models!(Bar);

/// The strip as laid out for a width.
#[derive(Clone, Debug, PartialEq)]
pub struct BarGeom {
    pub height: i64,
    pub two_lines: bool,
    pub mark: Rect,
    pub icon: Rect,
    pub text: Rect,
    /// What the question shows: whole, or cut with "…".
    pub shown: String,
    pub buttons: Vec<Rect>,
    /// The hairline under the strip.
    pub line: Rect,
}

/// A compact control's height in the current look (the guide: 24; 21 in the
/// classic look).
pub fn compact_h() -> i64 {
    if crate::theme::current().look == crate::theme::Look::Classic {
        21
    } else {
        24
    }
}

/// The width of a button as wide as its words (12 on each side, 72 at least).
pub fn button_w(caption: &str, font: &Font) -> i64 {
    (text_size(&plain(caption), font).0 + 24).max(BUTTON_MIN)
}

impl Bar {
    /// The strip in `w` pixels: `bold` is the question's font, `font` the
    /// buttons'.
    pub fn layout(&self, w: i64, bold: &Font, font: &Font) -> BarGeom {
        let ch = compact_h();
        let w = w.max(120);
        let widths: Vec<i64> = self.buttons.iter().map(|b| button_w(b, font)).collect();
        let bw: i64 = if widths.is_empty() { 0 } else { widths.iter().sum::<i64>() + BUTTON_GAP * (widths.len() as i64 - 1) };
        let room = w - TEXT_X - 16 - bw - EDGE;
        let keep = text_size(&(self.text.chars().take(KEEP_CHARS).collect::<String>() + "…"), bold).0;
        let two = !self.buttons.is_empty() && room < keep.min(text_size(&self.text, bold).0);
        // (two lines: the buttons under the question, from where its words start,
        // and on a row more each time the next one would pass the right edge)
        let rows = if two {
            let (mut rows, mut x) = (1, TEXT_X);
            for bw1 in &widths {
                if x > TEXT_X && x + bw1 > w - EDGE {
                    rows += 1;
                    x = TEXT_X;
                }
                x += bw1 + BUTTON_GAP;
            }
            rows
        } else {
            1
        };
        let (height, text, shown, mut x, y) = if two {
            let h = 10 + 18 + 8 + rows * ch + (rows - 1) * 8 + 10;
            let tw = w - TEXT_X - EDGE;
            (h, (TEXT_X, 10, tw, 18), middle_end(&self.text, bold, tw), TEXT_X, 10 + 18 + 8)
        } else {
            let h = ch + 12;
            let tw = if self.buttons.is_empty() { w - TEXT_X - EDGE } else { room };
            (h, (TEXT_X, (h - 18) / 2, tw.max(0), 18), middle_end(&self.text, bold, tw), w - EDGE - bw, (h - ch) / 2)
        };
        let mut buttons = Vec::new();
        let mut y = y;
        for bw1 in &widths {
            if two && x > TEXT_X && x + bw1 > w - EDGE {
                x = TEXT_X;
                y += ch + 8;
            }
            buttons.push((x, y, *bw1, ch));
            x += bw1 + BUTTON_GAP;
        }
        let icon_y = if two { 11 } else { (height - 16) / 2 };
        BarGeom { height, two_lines: two, mark: (0, 0, MARK, height), icon: (16, icon_y, 16, 16), text, shown, buttons, line: (0, height - 1, w, 1) }
    }

    /// How tall the strip wants to be in `w` pixels.
    pub fn height_for(&self, w: i64, bold: &Font, font: &Font) -> i64 {
        self.layout(w, bold, font).height
    }
}

/// `text` cut at its end with "…" to fit `width` (a question keeps its
/// start: that is where it says what).
pub fn middle_end(text: &str, font: &Font, width: i64) -> String {
    if text_size(text, font).0 <= width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    for k in (1..chars.len()).rev() {
        let s: String = chars[..k].iter().collect::<String>().trim_end().to_string() + "…";
        if text_size(&s, font).0 <= width {
            return s;
        }
    }
    let _ = middle;
    "…".to_string()
}

/// What a screen reader hears when the strip appears, and as its name: the
/// question, then the choices.
pub fn spoken(bar: &Bar) -> String {
    if bar.buttons.is_empty() {
        bar.text.clone()
    } else {
        format!("{} {}", bar.text, bar.buttons.iter().map(|b| plain(b)).collect::<Vec<_>>().join(", "))
    }
}

/// The button whose letter is `c` (a `&` marks it).
pub fn by_letter(buttons: &[String], c: char) -> Option<usize> {
    let c = c.to_lowercase().next()?;
    buttons.iter().position(|b| mnemonic(b) == Some(c))
}

// ---------------------------------------------------------------- user --

#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// A button pressed (0-based): the strip goes, then OnChoose.
    Choose(usize),
    /// Escape: the keyboard goes back where it was; the question stays.
    Escape,
    /// Only the look changed.
    Redraw,
}

// ------------------------------------------------------------- runtime --

fn s(args: &[Value], i: usize) -> String {
    args.get(i).map(Value::to_string_val).unwrap_or_default()
}

/// The buttons from a comma-separated list: trimmed, empty ones left out.
pub fn parse_buttons(text: &str) -> Vec<String> {
    text.split(',').map(str::trim).filter(|b| !b.is_empty()).take(6).map(str::to_string).collect()
}

fn fonts<R: Runtime>(rt: R, name: &str) -> (Font, Font) {
    let font = crate::objects::font_from_props(name, &|i, p| rt.get(i, p));
    let bold = Font { styles: font.styles | 1, ..font.clone() };
    (bold, font)
}

fn show<R: Runtime>(rt: R, name: &str) {
    rt.set(name, "visible", basic_bool(true));
    rt.invalidate();
    let (bold, font) = fonts(rt, name);
    let w = rt.get(name, "width").to_i64();
    let h = with(name, |b| b.height_for(w, &bold, &font));
    if let Some(h) = h {
        if rt.get(name, "height").to_i64() != h {
            rt.set(name, "height", Value::Integer(h));
        }
    }
}

/// The strip was given another width: its height follows (one line or two).
pub fn width_changed<R: Runtime>(rt: R, name: &str) {
    if with(name, |b| b.text.is_empty()).unwrap_or(true) {
        return;
    }
    let (bold, font) = fonts(rt, name);
    let w = rt.get(name, "width").to_i64();
    if let Some(h) = with(name, |b| b.height_for(w, &bold, &font)) {
        if rt.get(name, "height").to_i64() != h {
            rt.set(name, "height", Value::Integer(h));
        }
    }
}

/// Its methods; `None`: not one of its own.
pub fn rt_method<R: Runtime>(rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    match method {
        // Ask(Text, Buttons, [Kind]): shows the strip and says it once, at once
        "ask" => {
            let text = super::notify::clean(&s(args, 0));
            let buttons = parse_buttons(&s(args, 1));
            let kind = if args.len() > 2 && !s(args, 2).is_empty() { Kind::parse(&s(args, 2)) } else { Kind::Warning };
            with_mut(name, |b| {
                b.text = text.clone();
                b.buttons = buttons;
                b.kind = kind;
                b.choice = 0;
                b.hover = None;
                b.pressed = None;
                b.focus = 0;
            });
            show(rt, name);
            if let Some(said) = with(name, spoken) {
                rt.announce(&said, true);
            }
            Some(Value::Null)
        }
        "hide" => {
            rt.set(name, "visible", basic_bool(false));
            rt.invalidate();
            Some(Value::Null)
        }
        // Press(Index): button Index (from 1), as a click would
        "press" => {
            let i = args.first().map(Value::to_i64).unwrap_or(0);
            let n = with(name, |b| b.buttons.len()).unwrap_or(0) as i64;
            if i < 1 || i > n || !rt.get(name, "visible").to_bool() {
                return Some(basic_bool(false));
            }
            rt_user(rt, name, User::Choose(i as usize - 1));
            Some(basic_bool(true))
        }
        _ => None,
    }
}

/// Its properties its model answers.
pub fn rt_get<R: Runtime>(_rt: R, name: &str, prop: &str) -> Option<Value> {
    Some(match prop {
        "text" => Value::String(with(name, |b| b.text.clone()).unwrap_or_default().into()),
        "kind" => Value::String(with(name, |b| b.kind.name()).unwrap_or("warning").into()),
        "buttons" => Value::String(with(name, |b| b.buttons.join(",")).unwrap_or_default().into()),
        "buttoncount" => Value::Integer(with(name, |b| b.buttons.len()).unwrap_or(0) as i64),
        "choice" => Value::Integer(with(name, |b| b.choice).unwrap_or(0)),
        _ => return None,
    })
}

/// Its properties its model keeps: whether `prop` was one.
pub fn rt_set<R: Runtime>(rt: R, name: &str, prop: &str, v: &Value) -> bool {
    match prop {
        "text" => with_mut(name, |b| b.text = v.to_string_val()),
        "kind" => with_mut(name, |b| b.kind = Kind::parse(&v.to_string_val())),
        "buttons" => {
            let list = parse_buttons(&v.to_string_val());
            with_mut(name, |b| {
                b.buttons = list;
                b.focus = 0;
            });
        }
        _ => return false,
    }
    if rt.get(name, "visible").to_bool() {
        show(rt, name);
    }
    true
}

/// What the user did.
pub fn rt_user<R: Runtime>(rt: R, name: &str, action: User) {
    match action {
        User::Choose(i) => {
            let Some(caption) = with_mut(name, |b| {
                let c = b.buttons.get(i).cloned()?;
                b.choice = i as i64 + 1;
                b.hover = None;
                b.pressed = None;
                Some(c)
            }) else {
                return;
            };
            // (the strip goes first, then the program hears which)
            rt.set(name, "visible", basic_bool(false));
            rt.fire(name, "OnChoose", &[Value::Integer(i as i64 + 1), Value::String(plain(&caption).into())]);
        }
        User::Escape => rt.fire(name, "OnEscape", &[]),
        User::Redraw => {}
    }
}
