//! RNOTIFYBAR (lane NOTIFY; docs/design-guidelines.md, "Notifications",
//! "The decision strip"): a question the person must answer before work
//! goes on, as one slim line across the top of the thing it is about — the
//! kind's mark, an icon, the question in regular weight, compact quiet
//! buttons at the right end (the main one soft-filled, the others plain;
//! each an icon of RapidR's with its words, icon only when the room is
//! tight), a "More" button (⋯) for the rest and a close X. The question is
//! cut with "…" before anything wraps; too narrow even so, the buttons go
//! under it. The model, the same on every runtime: the words, the buttons,
//! the choice, and the strip's rectangles ([`Bar::layout`]: pure functions
//! of the width, so the kernel paints, hit-tests and describes from the
//! same places, and the dock can ask how tall the strip wants to be).

use super::notify::{mnemonic, plain, Kind};
use super::runtime::{basic_bool, Runtime};
use crate::objects::font::Font;
use crate::objects::ops::Rect;
use crate::objects::text::text_size;
use crate::Value;

/// Where the words start, and the gaps (the guide: 4-px mark, 12, icon 16, 8).
pub const TEXT_X: i64 = 40;
/// The space right of the last control.
pub const EDGE: i64 = 8;
/// Between the strip's controls.
pub const BUTTON_GAP: i64 = 4;
pub const MARK: i64 = 4;
/// A question is cut to this many characters before the buttons go under it.
pub const KEEP_CHARS: usize = 24;
/// A button's icon, its gap to the words, and its padding.
pub const ICON: i64 = 16;
pub const ICON_GAP: i64 = 6;
pub const PAD: i64 = 8;

/// One strip's state.
#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub text: String,
    pub kind: Kind,
    /// The buttons, each `icon|Caption` (an icon of RapidR's, optional) with
    /// a `&` before the letter that presses it.
    pub buttons: Vec<String>,
    /// A "More" button (⋯) after them: OnMore says where its menu goes.
    pub more: bool,
    /// A close X at the right end (OnChoose 0, "Dismiss").
    pub closable: bool,
    /// The main button (1-based; 0 none): soft-filled, the others plain.
    pub primary: usize,
    /// The button chosen last (1-based; 0 none yet).
    pub choice: i64,
    /// Under the mouse, pressed, with the keyboard: a part's number
    /// ([`Bar::parts`]).
    pub hover: Option<usize>,
    pub pressed: Option<usize>,
    pub focus: usize,
}

impl Default for Bar {
    fn default() -> Self {
        Bar { text: String::new(), kind: Kind::Warning, buttons: Vec::new(), more: false, closable: false, primary: 1, choice: 0, hover: None, pressed: None, focus: 0 }
    }
}

crate::panel_models!(Bar);

/// What a strip's part is: a button (0-based), More, the close X.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Button(usize),
    More,
    Close,
}

impl Bar {
    /// Every part the keyboard and the mouse can reach, left to right.
    pub fn parts(&self) -> Vec<Part> {
        let mut out: Vec<Part> = (0..self.buttons.len()).map(Part::Button).collect();
        if self.more {
            out.push(Part::More);
        }
        if self.closable {
            out.push(Part::Close);
        }
        out
    }
}

/// A button's icon name and its caption, from `icon|Caption`.
pub fn split_spec(raw: &str) -> (&str, &str) {
    match raw.split_once('|') {
        Some((icon, caption)) if !icon.trim().is_empty() && !icon.contains(' ') => (icon.trim(), caption.trim()),
        _ => ("", raw.trim()),
    }
}

/// A button's caption (with its `&`).
pub fn caption_of(raw: &str) -> &str {
    split_spec(raw).1
}

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
    /// Which of them show their icon only (their words are in the tooltip
    /// and the accessible name).
    pub icon_only: Vec<bool>,
    pub more: Option<Rect>,
    pub close: Option<Rect>,
    /// The hairline under the strip.
    pub line: Rect,
}

impl BarGeom {
    /// The rectangle of every part ([`Bar::parts`]).
    pub fn rects(&self) -> Vec<Rect> {
        let mut out = self.buttons.clone();
        out.extend(self.more);
        out.extend(self.close);
        out
    }
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

/// The font of the buttons: one step below the question's.
pub fn small(font: &Font) -> Font {
    Font { size: (font.size - 1).max(7), ..font.clone() }
}

/// The width of a button with its words (and its icon, when it has one).
pub fn button_w(raw: &str, font: &Font) -> i64 {
    let (icon, caption) = split_spec(raw);
    let lead = if icon.is_empty() { PAD } else { PAD + ICON + ICON_GAP };
    (lead + text_size(&plain(caption), font).0 + PAD + 2).max(compact_h())
}

impl Bar {
    /// The strip in `w` pixels: `font` is the question's, `small` the
    /// buttons'.
    pub fn layout(&self, w: i64, font: &Font, small: &Font) -> BarGeom {
        let ch = compact_h();
        let w = w.max(120);
        let n = self.buttons.len();
        let primary = self.primary.checked_sub(1).filter(|&p| p < n);
        let has_icon = |i: usize| !split_spec(&self.buttons[i]).0.is_empty();
        let full: Vec<i64> = self.buttons.iter().map(|b| button_w(b, small)).collect();
        // (the room's first answer: the words kept, the others icon only)
        let compact: Vec<bool> = (0..n).map(|i| Some(i) != primary && has_icon(i)).collect();
        let sum = |only: &dyn Fn(usize) -> bool| -> i64 {
            let btn: i64 = (0..n).map(|i| if only(i) { ch } else { full[i] }).sum::<i64>();
            let parts = n as i64 + i64::from(self.more) + i64::from(self.closable);
            btn + ch * (i64::from(self.more) + i64::from(self.closable)) + BUTTON_GAP * (parts - 1).max(0) + if self.closable && (n > 0 || self.more) { BUTTON_GAP } else { 0 }
        };
        let whole = text_size(&self.text, font).0;
        let keep = text_size(&(self.text.chars().take(KEEP_CHARS).collect::<String>() + "\u{2026}"), font).0.min(whole);
        let nothing = n == 0 && !self.more && !self.closable;
        let (mut one_line, mut only) = (true, vec![false; n]);
        let mut acts = 0;
        if !nothing {
            // (what the room gives up first, step by step: with a main action
            // the others lose their words before the question is cut; a strip
            // of equal choices, where the words are the decision, cuts the
            // question first. Each step is (buttons icon only?, question whole?);
            // the last that fits wins, and none: two lines)
            let steps: [(bool, bool); 3] = if primary.is_some() { [(false, true), (true, true), (true, false)] } else { [(false, true), (false, false), (true, false)] };
            let chosen = steps.iter().find(|(icons, whole_q)| {
                let bw = if *icons { sum(&|i| compact[i]) } else { sum(&|_| false) };
                w - TEXT_X - 12 - bw - EDGE >= if *whole_q { whole } else { keep }
            });
            match chosen {
                Some((true, _)) => {
                    only = compact.clone();
                    acts = sum(&|i| compact[i]);
                }
                Some((false, _)) => acts = sum(&|_| false),
                None => one_line = false,
            }
        }
        let mut buttons = Vec::new();
        let (height, text, shown): (i64, Rect, String);
        let (mut x, mut y): (i64, i64);
        let mut close = None;
        let mut more = None;
        if one_line {
            height = ch + 12;
            y = (height - ch) / 2;
            let room = if nothing { w - TEXT_X - EDGE } else { w - TEXT_X - 12 - acts - EDGE };
            text = (TEXT_X, (height - 18) / 2, room.max(0), 18);
            shown = middle_end(&self.text, font, room);
            x = w - EDGE - acts;
            for i in 0..n {
                let bw = if only[i] { ch } else { full[i] };
                buttons.push((x, y, bw, ch));
                x += bw + BUTTON_GAP;
            }
            if self.more {
                more = Some((x, y, ch, ch));
            }
            if self.closable {
                close = Some((w - EDGE - ch, y, ch, ch));
            }
        } else {
            // (two lines: the words across, the buttons under them from where the
            // words start, a row more each time the next would pass the right
            // edge; the close X stays at the top right)
            let tail = if self.closable { ch + BUTTON_GAP + EDGE } else { EDGE };
            let tw = w - TEXT_X - tail;
            // (the other buttons icon only here too: three rows of words are heavier than the question)
            only = compact.clone();
            let parts: Vec<i64> = (0..n).map(|i| if only[i] { ch } else { full[i] }).chain(self.more.then_some(ch)).collect();
            let (mut rows, mut px) = (1, TEXT_X);
            for pw in &parts {
                if px > TEXT_X && px + pw > w - EDGE {
                    rows += 1;
                    px = TEXT_X;
                }
                px += pw + BUTTON_GAP;
            }
            height = 10 + 18 + 8 + rows * ch + (rows - 1) * 8 + 10;
            text = (TEXT_X, 10, tw.max(0), 18);
            shown = middle_end(&self.text, font, tw);
            x = TEXT_X;
            y = 10 + 18 + 8;
            for (i, pw) in parts.iter().enumerate() {
                if x > TEXT_X && x + pw > w - EDGE {
                    x = TEXT_X;
                    y += ch + 8;
                }
                if i < n {
                    buttons.push((x, y, *pw, ch));
                } else {
                    more = Some((x, y, *pw, ch));
                }
                x += pw + BUTTON_GAP;
            }
            if self.closable {
                close = Some((w - EDGE - ch, 6, ch, ch));
            }
        }
        let icon_y = if one_line { (height - 16) / 2 } else { 11 };
        BarGeom { height, two_lines: !one_line, mark: (0, 0, MARK, height), icon: (16, icon_y, 16, 16), text, shown, buttons, icon_only: only, more, close, line: (0, height - 1, w, 1) }
    }

    /// How tall the strip wants to be in `w` pixels.
    pub fn height_for(&self, w: i64, font: &Font, small: &Font) -> i64 {
        self.layout(w, font, small).height
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
    "…".to_string()
}

/// What a screen reader hears when the strip appears, and as its name: the
/// question, then the choices.
pub fn spoken(bar: &Bar) -> String {
    if bar.buttons.is_empty() {
        bar.text.clone()
    } else {
        format!("{} {}", bar.text, bar.buttons.iter().map(|b| plain(caption_of(b))).collect::<Vec<_>>().join(", "))
    }
}

/// The button whose letter is `c` (a `&` marks it).
pub fn by_letter(buttons: &[String], c: char) -> Option<usize> {
    let c = c.to_lowercase().next()?;
    buttons.iter().position(|b| mnemonic(caption_of(b)) == Some(c))
}

// ---------------------------------------------------------------- user --

#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// A button pressed (0-based): the strip goes, then OnChoose.
    Choose(usize),
    /// More pressed: where its menu goes (the button's right edge and bottom,
    /// the form's pixels): OnMore.
    More(i64, i64),
    /// The close X: the strip goes, then OnChoose 0.
    Close,
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
    let small = small(&font);
    (font, small)
}

fn show<R: Runtime>(rt: R, name: &str) {
    rt.set(name, "visible", basic_bool(true));
    rt.invalidate();
    let (font, small) = fonts(rt, name);
    let w = rt.get(name, "width").to_i64();
    let h = with(name, |b| b.height_for(w, &font, &small));
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
    let (font, small) = fonts(rt, name);
    let w = rt.get(name, "width").to_i64();
    if let Some(h) = with(name, |b| b.height_for(w, &font, &small)) {
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
        // Press(Index): button Index (from 1; 0 the close X), as a click would
        "press" => {
            let i = args.first().map(Value::to_i64).unwrap_or(0);
            let (n, closable) = with(name, |b| (b.buttons.len() as i64, b.closable)).unwrap_or((0, false));
            if !rt.get(name, "visible").to_bool() {
                return Some(basic_bool(false));
            }
            if i == 0 && closable {
                rt_user(rt, name, User::Close);
                return Some(basic_bool(true));
            }
            if i < 1 || i > n {
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
        "more" => basic_bool(with(name, |b| b.more).unwrap_or(false)),
        "closable" => basic_bool(with(name, |b| b.closable).unwrap_or(false)),
        "primary" => Value::Integer(with(name, |b| b.primary).unwrap_or(1) as i64),
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
        "more" => with_mut(name, |b| b.more = crate::panels::runtime::truth(v)),
        "closable" => with_mut(name, |b| b.closable = crate::panels::runtime::truth(v)),
        "primary" => with_mut(name, |b| b.primary = usize::try_from(v.to_i64().max(0)).unwrap_or(0)),
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
            rt.fire(name, "OnChoose", &[Value::Integer(i as i64 + 1), Value::String(plain(caption_of(&caption)).into())]);
        }
        User::More(x, y) => rt.fire(name, "OnMore", &[Value::Integer(x), Value::Integer(y)]),
        User::Close => {
            with_mut(name, |b| {
                b.choice = 0;
                b.hover = None;
                b.pressed = None;
            });
            rt.set(name, "visible", basic_bool(false));
            rt.fire(name, "OnChoose", &[Value::Integer(0), Value::String(crate::i18n::tr_ui("Dismiss").into())]);
        }
        User::Escape => rt.fire(name, "OnEscape", &[]),
        User::Redraw => {}
    }
}
