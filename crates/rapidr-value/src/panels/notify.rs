//! RNOTIFY (lane NOTIFY; the rules: docs/design-guidelines.md,
//! "Notifications"): small messages that pop and inform — toasts —, and the
//! history behind them. The model, the same on every runtime: the queue of
//! toasts on screen, how long each stays, how they merge and stack, the
//! history (what was said, with its action still working or told why not),
//! and where everything is drawn ([`Notify::layout`], [`Notify::history_layout`]:
//! pure functions of the sizes, so the UI kernel paints, hit-tests and
//! describes from the same rectangles).
//!
//! The component is a transparent overlay: its own rectangle is the part of
//! the form it may draw in (`Align = alClient` covers the whole form), and
//! it takes the mouse only where a toast, or the open history, is
//! (`claims` in the kernel's kind). Its toasts sit at its [`Position`]
//! corner of its *area*: the whole rectangle, or the one the program gave
//! ([`Notify::area`]: RapidR Studio's editor area, less the editor's scroll
//! bar).

use super::runtime::{basic_bool, truth, Runtime};
use crate::objects::font::Font;
use crate::objects::ops::Rect;
use crate::objects::text::text_size;
use crate::Value;

// ---------------------------------------------------------------- sizes --

/// A toast's width (440 with two actions).
pub const CARD_W: i64 = 360;
pub const CARD_W_TWO: i64 = 440;
/// A toast of one line, of two (the compact control's 24 and 8 above and
/// below; two lines of 18).
pub const ONE_H: i64 = 40;
pub const TWO_H: i64 = 52;
/// Between stacked toasts.
pub const GAP: i64 = 8;
/// The close button's target (WCAG 2.5.8) and the cross inside it.
pub const CLOSE: i64 = 24;
/// What sits left of the text: the mark (4), 12, the icon (16), 8.
pub const TEXT_LEFT: i64 = 40;
/// What sits right of the text with no action: 8, the close button, 8.
pub const TEXT_RIGHT: i64 = 8 + CLOSE + 8;
/// A line of text.
pub const LINE: i64 = 18;
/// A toast's mark, at its left edge.
pub const MARK_W: i64 = 4;
/// The slide-in and fade-out, in milliseconds, and how far it slides.
pub const ENTER_MS: u64 = 160;
pub const LEAVE_MS: u64 = 120;
pub const SLIDE: i64 = 12;
/// After the mouse or the keyboard leaves a toast, it stays this long at
/// least (the guide: 2 s).
pub const HELD_MIN_MS: i64 = 2000;
/// The history card's width, its header, and the rows it shows before it
/// scrolls.
pub const HIST_W: i64 = 360;
pub const HIST_HEAD: i64 = 40;
pub const HIST_ROWS: i64 = 8;

/// How long a toast stays (the guide's formula): 3 s and 80 ms a character
/// (a slow reader's pace, about 150 words a minute), 2 s more with an
/// action; at least 5 s, at most 20 s; a warning one and a half times as
/// long; an error until it is closed. `timeout`: -1 the formula, 0 until
/// closed, else those milliseconds; `scale`: 100 as is, 200 twice as long,
/// 0 until closed. `None`: until closed.
pub fn stay_ms(text: &str, has_action: bool, kind: Kind, timeout: i64, scale: i64) -> Option<i64> {
    if scale <= 0 || timeout == 0 {
        return None;
    }
    let base = if timeout > 0 {
        timeout
    } else {
        if kind == Kind::Error {
            return None;
        }
        let ms = (3000 + 80 * text.chars().count() as i64 + if has_action { 2000 } else { 0 }).clamp(5000, 20000);
        if kind == Kind::Warning {
            ms * 3 / 2
        } else {
            ms
        }
    };
    Some(base * scale / 100)
}

// ---------------------------------------------------------------- kinds --

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl Kind {
    /// "info", "success", "warning" ("warn"), "error" ("danger"), any case;
    /// anything else is information.
    pub fn parse(s: &str) -> Kind {
        match s.trim().to_ascii_lowercase().as_str() {
            "success" | "ok" | "done" => Kind::Success,
            "warning" | "warn" => Kind::Warning,
            "error" | "danger" | "failed" => Kind::Error,
            _ => Kind::Info,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Info => "info",
            Kind::Success => "success",
            Kind::Warning => "warning",
            Kind::Error => "error",
        }
    }

    /// RapidR's icon for it.
    pub fn icon(self) -> &'static str {
        self.name()
    }

    /// Its place among the status colours (`ide_theme::statuses`).
    pub fn status(self) -> usize {
        match self {
            Kind::Info => 0,
            Kind::Success => 1,
            Kind::Warning => 2,
            Kind::Error => 3,
        }
    }
}

/// The corner of its area the toasts sit in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
}

impl Position {
    pub fn parse(s: &str) -> Option<Position> {
        let t: String = s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_ascii_lowercase();
        Some(match t.as_str() {
            "bottomright" | "br" | "0" => Position::BottomRight,
            "bottomleft" | "bl" | "1" => Position::BottomLeft,
            "topright" | "tr" | "2" => Position::TopRight,
            "topleft" | "tl" | "3" => Position::TopLeft,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Position::BottomRight => "BottomRight",
            Position::BottomLeft => "BottomLeft",
            Position::TopRight => "TopRight",
            Position::TopLeft => "TopLeft",
        }
    }

    /// The constant's value (npBottomRight 0 ...).
    pub fn index(self) -> i64 {
        match self {
            Position::BottomRight => 0,
            Position::BottomLeft => 1,
            Position::TopRight => 2,
            Position::TopLeft => 3,
        }
    }

    pub fn bottom(self) -> bool {
        matches!(self, Position::BottomRight | Position::BottomLeft)
    }

    pub fn right(self) -> bool {
        matches!(self, Position::BottomRight | Position::TopRight)
    }
}

// ------------------------------------------------------------ the data --

/// A toast on screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: i64,
    pub key: String,
    pub kind: Kind,
    pub text: String,
    /// One or two action captions, a `&` before the letter that presses it
    /// (not drawn).
    pub actions: Vec<String>,
    /// -1 the formula, 0 until closed, else milliseconds.
    pub timeout: i64,
    /// Whether its time is running (set when it is first drawn).
    pub armed: bool,
    pub timed: bool,
    /// Milliseconds it still stays.
    pub left_ms: i64,
    /// The kernel's clock when it was first drawn / last advanced.
    pub born: Option<u64>,
    pub last: u64,
    /// When it started going (the fade).
    pub leaving: Option<u64>,
    pub reason: String,
}

/// A message in the history.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: i64,
    pub key: String,
    pub kind: Kind,
    pub text: String,
    pub actions: Vec<String>,
    /// The clock when it was said ("14:32").
    pub clock: String,
    pub seen: bool,
    /// Per action: why it can't be done any more (`None`: it can).
    pub retired: Vec<Option<String>>,
}

impl Entry {
    /// Whether action `i` can still be done.
    pub fn works(&self, i: usize) -> bool {
        i < self.actions.len() && self.retired.get(i).is_none_or(Option::is_none)
    }
}

/// A part of a toast or of the history under the mouse or with the focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Card(i64),
    Action(i64, usize),
    Close(i64),
    /// The history: a row, a row's action, Clear.
    Row(i64),
    RowAction(i64, usize),
    Clear,
}

impl Hit {
    /// The toast it is on, if it is on one.
    pub fn toast(self) -> Option<i64> {
        match self {
            Hit::Card(id) | Hit::Action(id, _) | Hit::Close(id) => Some(id),
            _ => None,
        }
    }
}

/// One RNotify's state.
#[derive(Clone, Debug)]
pub struct Notify {
    pub next_id: i64,
    pub toasts: Vec<Toast>,
    pub history: Vec<Entry>,
    pub position: Position,
    pub max_visible: usize,
    /// -1: fits its length (the formula); 0: until closed.
    pub timeout: i64,
    /// Percent of the formula; 0: until closed.
    pub time_scale: i64,
    pub history_size: usize,
    pub margin: i64,
    /// Where the toasts sit, in the component's own pixels (`None`: all of
    /// it).
    pub area: Option<Rect>,
    /// Where the history card's bottom right hangs (the bell's), in the
    /// component's pixels (`None`: the area's bottom right).
    pub anchor: Option<(i64, i64)>,
    pub history_open: bool,
    /// The history's scroll, in pixels.
    pub history_top: i64,
    pub hover: Option<Hit>,
    pub focus: Option<Hit>,
    pub pressed: Option<Hit>,
    /// What was last drawn (the kernel's hit tests and the screen reader's
    /// bounds): rectangles in the component's pixels.
    pub shown: Vec<CardGeom>,
    pub shown_history: Option<HistoryGeom>,
    /// The unseen count at the last OnUnseenChange.
    pub told_unseen: usize,
}

impl Default for Notify {
    fn default() -> Self {
        Notify {
            next_id: 1,
            toasts: Vec::new(),
            history: Vec::new(),
            position: Position::BottomRight,
            max_visible: 3,
            timeout: -1,
            time_scale: 100,
            history_size: 50,
            margin: 12,
            area: None,
            anchor: None,
            history_open: false,
            history_top: 0,
            hover: None,
            focus: None,
            pressed: None,
            shown: Vec::new(),
            shown_history: None,
            told_unseen: 0,
        }
    }
}

crate::panel_models!(Notify);

/// What went when a toast went.
#[derive(Clone, Debug, PartialEq)]
pub struct Closed {
    pub id: i64,
    pub key: String,
    pub reason: String,
}

// ------------------------------------------------------------- the model --

impl Notify {
    pub fn unseen(&self) -> usize {
        self.history.iter().filter(|e| !e.seen).count()
    }

    pub fn toast(&self, id: i64) -> Option<&Toast> {
        self.toasts.iter().find(|t| t.id == id)
    }

    fn toast_mut(&mut self, id: i64) -> Option<&mut Toast> {
        self.toasts.iter_mut().find(|t| t.id == id)
    }

    /// The toasts not on their way out, oldest first: what "visible" counts.
    pub fn visible(&self) -> impl Iterator<Item = &Toast> {
        self.toasts.iter().filter(|t| t.leaving.is_none())
    }

    pub fn entry(&self, id: i64) -> Option<&Entry> {
        self.history.iter().find(|e| e.id == id)
    }

    /// Says a message: a new toast, or — when `key` is the key of one on
    /// screen — that toast with the new words (its time starting again), so
    /// a burst of the same thing is one toast ("5 files" becomes "7 files").
    /// The history takes it too (the last entry with the same key is
    /// replaced). Returns its id and the toasts pushed out by it (more than
    /// [`Notify::max_visible`]: the oldest go at once; they stay in the
    /// history).
    pub fn say(&mut self, text: &str, kind: Kind, actions: Vec<String>, timeout: Option<i64>, key: &str, clock: &str) -> (i64, Vec<Closed>) {
        let text = &clean(text);
        let timeout = timeout.unwrap_or(self.timeout);
        let mut pushed = Vec::new();
        let merged = (!key.is_empty()).then(|| self.toasts.iter().position(|t| t.key == key && t.leaving.is_none())).flatten();
        let id = match merged {
            Some(i) => {
                let t = &mut self.toasts[i];
                t.kind = kind;
                t.text = text.to_string();
                t.actions = actions.clone();
                t.timeout = timeout;
                t.armed = false;
                t.id
            }
            None => {
                let id = self.next_id;
                self.next_id += 1;
                self.toasts.push(Toast { id, key: key.to_string(), kind, text: text.to_string(), actions: actions.clone(), timeout, armed: false, timed: false, left_ms: 0, born: None, last: 0, leaving: None, reason: String::new() });
                id
            }
        };
        // (the history: the same key again replaces its newest entry)
        let again = (!key.is_empty()).then(|| self.history.iter().position(|e| e.key == key)).flatten();
        let n = actions.len();
        let entry = Entry { id, key: key.to_string(), kind, text: text.to_string(), actions, clock: clock.to_string(), seen: false, retired: vec![None; n] };
        match again {
            Some(i) => {
                self.history.remove(i);
                self.history.insert(0, entry);
            }
            None => self.history.insert(0, entry),
        }
        self.history.truncate(self.history_size.max(1));
        // (too many on screen: the oldest go at once)
        while self.visible().count() > self.max_visible.max(1) {
            let Some(old) = self.toasts.iter().position(|t| t.leaving.is_none()) else { break };
            let t = self.toasts.remove(old);
            pushed.push(Closed { id: t.id, key: t.key, reason: "pushed".into() });
        }
        (id, pushed)
    }

    /// Takes a toast away at once (the program's Close); whether it was there.
    pub fn dismiss(&mut self, id: i64) -> Option<Closed> {
        let i = self.toasts.iter().position(|t| t.id == id)?;
        let t = self.toasts.remove(i);
        self.hover = self.hover.filter(|h| h.toast() != Some(id));
        self.focus = self.focus.filter(|h| h.toast() != Some(id));
        Some(Closed { id, key: t.key, reason: "program".into() })
    }

    /// Starts a toast going (the user closed it, an action pressed it, its
    /// time ran out): it fades, then [`Notify::advance`] reports it closed.
    pub fn leave(&mut self, id: i64, reason: &str, now: u64) {
        let reduced = crate::theme::reduced_motion();
        if let Some(t) = self.toast_mut(id) {
            if t.leaving.is_none() {
                t.leaving = Some(if reduced { now.saturating_sub(LEAVE_MS) } else { now });
                t.reason = reason.to_string();
            }
        }
    }

    /// Marks every action of the notifications with `key` as no longer
    /// possible, saying why; a toast on screen loses its actions. How many
    /// entries it touched.
    pub fn retire(&mut self, key: &str, reason: &str) -> usize {
        if key.is_empty() {
            return 0;
        }
        let mut n = 0;
        for e in self.history.iter_mut().filter(|e| e.key == key) {
            for r in e.retired.iter_mut() {
                if r.is_none() {
                    *r = Some(reason.to_string());
                }
            }
            n += 1;
        }
        for t in self.toasts.iter_mut().filter(|t| t.key == key) {
            t.actions.clear();
        }
        n
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.history_top = 0;
    }

    pub fn mark_seen(&mut self) {
        for e in &mut self.history {
            e.seen = true;
        }
    }

    /// Whether the mouse or the keyboard is on toast `id` (its time stands
    /// still).
    pub fn held(&self, id: i64) -> bool {
        self.hover.and_then(Hit::toast) == Some(id) || self.focus.and_then(Hit::toast) == Some(id)
    }

    /// The mouse or the keyboard left toast `id`: it stays 2 s at least.
    pub fn released(&mut self, id: i64) {
        if let Some(t) = self.toast_mut(id) {
            if t.timed {
                t.left_ms = t.left_ms.max(HELD_MIN_MS);
            }
        }
    }

    /// Time passes (the kernel's clock, in milliseconds): toasts drawn for
    /// the first time start their time, the ones in time lose what passed
    /// (unless held), the ones that ran out start going, the ones that have
    /// gone are taken away and returned.
    pub fn advance(&mut self, now: u64) -> Vec<Closed> {
        let held: Vec<i64> = self.toasts.iter().map(|t| t.id).filter(|&id| self.held(id)).collect();
        let reduced = crate::theme::reduced_motion();
        let scale = self.time_scale;
        let mut gone = Vec::new();
        for t in &mut self.toasts {
            if t.born.is_none() {
                t.born = Some(now);
                t.last = now;
            }
            if !t.armed {
                t.armed = true;
                match stay_ms(&t.text, !t.actions.is_empty(), t.kind, t.timeout, scale) {
                    Some(ms) => {
                        t.timed = true;
                        t.left_ms = ms;
                    }
                    None => t.timed = false,
                }
            }
            let dt = now.saturating_sub(t.last) as i64;
            t.last = now;
            if t.leaving.is_some() {
                continue;
            }
            if t.timed && !held.contains(&t.id) {
                t.left_ms -= dt;
                if t.left_ms <= 0 {
                    t.leaving = Some(if reduced { now.saturating_sub(LEAVE_MS) } else { now });
                    t.reason = "timeout".into();
                }
            }
        }
        let mut i = 0;
        while i < self.toasts.len() {
            let t = &self.toasts[i];
            if t.leaving.is_some_and(|at| now.saturating_sub(at) >= LEAVE_MS) {
                let t = self.toasts.remove(i);
                self.hover = self.hover.filter(|h| h.toast() != Some(t.id));
                self.focus = self.focus.filter(|h| h.toast() != Some(t.id));
                gone.push(Closed { id: t.id, key: t.key, reason: t.reason });
            } else {
                i += 1;
            }
        }
        gone
    }

    /// When the kernel must look again (`now`, ms): soon while something
    /// slides or fades, else when the first running time ends. `None`: never
    /// (nothing in time, or everything held).
    pub fn next_wake(&self, now: u64) -> Option<u64> {
        let mut next: Option<u64> = None;
        let mut soon = |ms: u64| next = Some(next.map_or(now + ms, |n| n.min(now + ms)));
        for t in &self.toasts {
            let animating = t.leaving.is_some() || t.born.is_none_or(|b| now.saturating_sub(b) < ENTER_MS);
            if animating {
                soon(16);
            } else if t.timed && !self.held(t.id) {
                soon(t.left_ms.max(1) as u64);
            }
        }
        next
    }

    /// How far a toast has come in (0..=1000, a thousandth): its slide and
    /// fade; 1000 with reduced motion, and when it has no birth yet.
    pub fn entered(&self, t: &Toast, now: u64) -> i64 {
        if crate::theme::reduced_motion() {
            return 1000;
        }
        match t.born {
            Some(b) => (now.saturating_sub(b) * 1000 / ENTER_MS).min(1000) as i64,
            None => 0,
        }
    }

    /// How far a toast has gone (0..=1000).
    pub fn left(&self, t: &Toast, now: u64) -> i64 {
        match t.leaving {
            Some(at) => (now.saturating_sub(at) * 1000 / LEAVE_MS).min(1000) as i64,
            None => 0,
        }
    }
}

// --------------------------------------------------------------- layout --

/// One toast's rectangles, in the component's pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct CardGeom {
    pub id: i64,
    pub rect: Rect,
    pub icon: Rect,
    pub text: Rect,
    /// The text's lines as drawn (cut in the middle when it needs more than
    /// two).
    pub lines: Vec<String>,
    pub actions: Vec<(String, Rect)>,
    pub close: Rect,
}

/// The lines of a card whose text is `width` wide: words wrapped, at most
/// two lines; what doesn't fit is cut with "…" in its middle (its start and
/// its end stay: a file's extension).
pub fn wrap_two(text: &str, font: &Font, width: i64) -> Vec<String> {
    let width = width.max(40);
    let fits = |s: &str| text_size(s, font).0 <= width;
    if fits(text) {
        return vec![text.to_string()];
    }
    let words = word_spans(text);
    let (mut first, mut used) = (String::new(), 0);
    for (i, (_, word)) in words.iter().enumerate() {
        let t = if first.is_empty() { word.to_string() } else { format!("{first} {word}") };
        if !fits(&t) {
            break;
        }
        first = t;
        used = i + 1;
    }
    if first.is_empty() {
        // (a first word longer than the line: as much of it as fits, the rest
        // cut in its middle on the second line)
        let chars: Vec<char> = text.chars().collect();
        let mut k = chars.len();
        while k > 1 && !fits(&chars[..k].iter().collect::<String>()) {
            k -= 1;
        }
        let head: String = chars[..k].iter().collect();
        let rest: String = chars[k..].iter().collect::<String>().trim().to_string();
        return if rest.is_empty() { vec![head] } else { vec![head, middle(&rest, font, width)] };
    }
    let rest = words.get(used).map(|(at, _)| text[*at..].trim().to_string()).unwrap_or_default();
    if rest.is_empty() {
        return vec![first];
    }
    vec![first, middle(&rest, font, width)]
}

fn word_spans(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if let Some(s) = start.take() {
                out.push((s, &text[s..i]));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push((s, &text[s..]));
    }
    out
}

/// `text` cut with "…" in its middle to fit `width`: its start and its end
/// (a file's extension) stay.
pub fn middle(text: &str, font: &Font, width: i64) -> String {
    if text_size(text, font).0 <= width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    for keep in (2..n).rev() {
        let head = keep - keep / 2;
        let tail = keep / 2;
        let s: String = chars[..head].iter().collect::<String>().trim_end().to_string() + "…" + &chars[n - tail..].iter().collect::<String>();
        if text_size(&s, font).0 <= width {
            return s;
        }
    }
    "…".to_string()
}

impl Notify {
    /// Where the toasts go in a component `w` × `h`: the area less the
    /// margin, the newest in the corner, the others beyond it. `font` is the
    /// text's, `link` the actions'. Toasts leaving keep their place.
    pub fn layout(&self, w: i64, h: i64, font: &Font, link: &Font) -> Vec<CardGeom> {
        let (ax, ay, aw, ah) = self.area.unwrap_or((0, 0, w, h));
        let m = self.margin.max(0);
        let two_actions = |t: &Toast| t.actions.len() >= 2;
        let mut out: Vec<CardGeom> = Vec::new();
        // (newest first: nearest the corner)
        let mut edge = if self.position.bottom() { ay + ah - m } else { ay + m };
        for t in self.toasts.iter().rev() {
            let want = if two_actions(t) { CARD_W_TWO } else { CARD_W };
            let cw = want.min((aw - 2 * m).max(160));
            let action_w: Vec<i64> = t.actions.iter().map(|a| text_size(&plain(a), link).0).collect();
            let actions_total: i64 = if action_w.is_empty() { 0 } else { action_w.iter().sum::<i64>() + 16 * (action_w.len() as i64 - 1) + 12 };
            let text_w = cw - TEXT_LEFT - TEXT_RIGHT - actions_total;
            let lines = wrap_two(&t.text, font, text_w);
            let ch = if lines.len() > 1 { TWO_H } else { ONE_H };
            let x = if self.position.right() { ax + aw - m - cw } else { ax + m };
            let y = if self.position.bottom() {
                edge -= ch;
                let y = edge;
                edge -= GAP;
                y
            } else {
                let y = edge;
                edge += ch + GAP;
                y
            };
            let lh = LINE.min(ch / lines.len().max(1) as i64);
            let text = (x + TEXT_LEFT, y + (ch - lh * lines.len() as i64) / 2, text_w.max(0), lh * lines.len() as i64);
            let icon_y = if lines.len() > 1 { y + 10 } else { y + (ch - 16) / 2 };
            // (the actions, right to left from before the close button)
            let mut ax2 = x + cw - TEXT_RIGHT;
            let mut acts: Vec<(String, Rect)> = Vec::new();
            for (a, aw1) in t.actions.iter().zip(&action_w).rev() {
                ax2 -= aw1;
                acts.push((a.clone(), (ax2, y + (ch - 24) / 2, *aw1, 24)));
                ax2 -= 16;
            }
            acts.reverse();
            out.push(CardGeom { id: t.id, rect: (x, y, cw, ch), icon: (x + 16, icon_y, 16, 16), text, lines, actions: acts, close: (x + cw - 8 - CLOSE, y + (ch - CLOSE) / 2, CLOSE, CLOSE) });
        }
        out.reverse();
        out
    }
}

/// What a message may say: a credential the program holds is masked (as the
/// Output pane's text is: [`crate::secrets::mask`]), a control character —
/// a line break too — becomes a space, and the characters that reorder text
/// (bidirectional overrides and isolates) are dropped, so a message can
/// never spell a secret out or look like another one.
pub fn clean(text: &str) -> String {
    crate::secrets::mask(text)
        .chars()
        .filter(|c| !matches!(*c as u32, 0x202A..=0x202E | 0x2066..=0x2069))
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// A caption without its `&`.
pub fn plain(caption: &str) -> String {
    let mut out = String::new();
    let mut it = caption.chars().peekable();
    while let Some(c) = it.next() {
        if c == '&' {
            if it.peek() == Some(&'&') {
                out.push('&');
                it.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The letter a caption's `&` marks, lower case.
pub fn mnemonic(caption: &str) -> Option<char> {
    let mut it = caption.chars().peekable();
    while let Some(c) = it.next() {
        if c == '&' {
            match it.peek() {
                Some('&') => {
                    it.next();
                }
                Some(n) => return n.to_lowercase().next(),
                None => {}
            }
        }
    }
    None
}

/// One row of the history card.
#[derive(Clone, Debug, PartialEq)]
pub struct RowGeom {
    pub id: i64,
    pub rect: Rect,
    pub icon: Rect,
    pub time: Rect,
    pub text: Rect,
    pub lines: Vec<String>,
    pub actions: Vec<(String, Rect)>,
}

/// The history card as laid out.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryGeom {
    pub rect: Rect,
    pub head: Rect,
    pub clear: Rect,
    /// The rows' area (clipped), and every row in it (some above or below
    /// what shows).
    pub body: Rect,
    pub rows: Vec<RowGeom>,
    /// All the rows' height (what the scroll runs over).
    pub content: i64,
}

impl Notify {
    /// The history card in a component `w` x `h`: above its anchor (the
    /// bell's corner; else the area's bottom right), `HIST_W` wide, at most
    /// eight rows' height before it scrolls. Rows are evenly spaced: 8, the
    /// time (a line), the text (its lines, whole), 8.
    pub fn history_layout(&self, w: i64, h: i64, font: &Font, link: &Font) -> HistoryGeom {
        let (ax, ay, aw, ah) = self.area.unwrap_or((0, 0, w, h));
        let m = self.margin.max(0);
        let (right, bottom) = self.anchor.unwrap_or((ax + aw - m, ay + ah - m));
        let cw = HIST_W.min((w - 2 * m).max(200));
        let text_x = 44;
        // (rows from the body's top, unscrolled)
        let mut rel: Vec<(i64, i64, Vec<String>, Vec<(String, i64)>)> = Vec::new();
        let mut y = 0;
        for e in &self.history {
            let lines = wrap_all(&e.text, font, cw - text_x - 16);
            let rh = 8 + 16 + lines.len() as i64 * LINE + 8;
            let acts: Vec<(String, i64)> = e.actions.iter().map(|a| (a.clone(), text_size(&plain(a), link).0)).collect();
            rel.push((y, rh, lines, acts));
            y += rh;
        }
        let content = y;
        let cap = (HIST_ROWS * 64).min((h - 2 * m - HIST_HEAD).max(96));
        let body_h = content.max(48).min(cap);
        let ch = HIST_HEAD + body_h;
        let x = (right - cw).max(0);
        let cy = (bottom - ch).max(0);
        let top = self.history_top.clamp(0, (content - body_h).max(0));
        let base = cy + HIST_HEAD - top;
        let rows = self
            .history
            .iter()
            .zip(rel)
            .map(|(e, (ry, rh, lines, acts))| {
                let mut actions = Vec::new();
                let mut ax2 = x + cw - 16;
                for (a, aw1) in acts.iter().rev() {
                    ax2 -= aw1;
                    actions.push((a.clone(), (ax2, base + ry + 6, *aw1, 20)));
                    ax2 -= 16;
                }
                actions.reverse();
                let n = lines.len() as i64;
                RowGeom { id: e.id, rect: (x, base + ry, cw, rh), icon: (x + 16, base + ry + 8, 16, 16), time: (x + text_x, base + ry + 8, 60, 16), text: (x + text_x, base + ry + 24, cw - text_x - 16, n * LINE), lines, actions }
            })
            .collect();
        let clear_w = text_size(&crate::i18n::tr_ui("Clear"), link).0;
        HistoryGeom { rect: (x, cy, cw, ch), head: (x, cy, cw, HIST_HEAD), clear: (x + cw - 16 - clear_w, cy + (HIST_HEAD - 20) / 2, clear_w, 20), body: (x, cy + HIST_HEAD, cw, body_h), rows, content }
    }
}

/// `text` wrapped on every line it needs (the history shows it whole).
pub fn wrap_all(text: &str, font: &Font, width: i64) -> Vec<String> {
    let width = width.max(40);
    let mut lines = Vec::new();
    let mut cur = String::new();
    for (_, word) in word_spans(text) {
        let t = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if text_size(&t, font).0 <= width || cur.is_empty() {
            cur = t;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push(cur);
    }
    lines
}

// ------------------------------------------------------- the user's acts --

/// What the user did, from the kernel to the runtime glue ([`rt_user`]).
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// An action pressed (a toast's or a history row's: `entry` is the
    /// history entry, which the toast has too).
    Action { id: i64, key: String, index: usize },
    /// A toast went: its time ran out, the user closed it (reason says).
    Closed(Vec<Closed>),
    /// The history was opened or closed, cleared; the unseen count may have
    /// changed.
    History,
    /// Only the look changed (what is under the mouse): drawn again.
    Redraw,
}

// ---------------------------------------------------------- the runtime --

fn s(args: &[Value], i: usize) -> String {
    args.get(i).map(Value::to_string_val).unwrap_or_default()
}

fn int(args: &[Value], i: usize, default: i64) -> i64 {
    match args.get(i) {
        None | Some(Value::Null) => default,
        Some(v) => v.to_i64(),
    }
}

/// The toast's actions from Show's argument: one or two captions separated
/// by `|`.
pub fn parse_actions(text: &str) -> Vec<String> {
    text.split('|').map(str::trim).filter(|a| !a.is_empty()).take(2).map(str::to_string).collect()
}

/// A notification's wording as a screen reader hears it: its text, then its
/// actions.
pub fn spoken(text: &str, actions: &[String]) -> String {
    if actions.is_empty() {
        text.to_string()
    } else {
        format!("{} ({})", text.trim_end_matches('.'), actions.iter().map(|a| plain(a)).collect::<Vec<_>>().join(", "))
    }
}

/// Tells the program the unseen count changed (OnUnseenChange).
fn told<R: Runtime>(rt: R, name: &str) {
    let now = with(name, Notify::unseen).unwrap_or(0);
    let before = with_mut(name, |m| std::mem::replace(&mut m.told_unseen, now));
    if before != now {
        rt.fire(name, "OnUnseenChange", &[Value::Integer(now as i64)]);
    }
}

fn closed<R: Runtime>(rt: R, name: &str, list: &[Closed]) {
    for c in list {
        rt.fire(name, "OnClosed", &[Value::Integer(c.id), Value::String(c.reason.clone().into()), Value::String(c.key.clone().into())]);
    }
}

/// Its methods; `None`: not one of its own.
pub fn rt_method<R: Runtime>(rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let nth = |i: i64| usize::try_from(i).ok();
    match method {
        // Show(Text, [Kind], [Action], [Timeout], [Key]) AS INTEGER
        "show" => {
            let text = clean(&s(args, 0));
            if text.trim().is_empty() {
                return Some(Value::Integer(0));
            }
            let kind = Kind::parse(&s(args, 1));
            let actions = parse_actions(&s(args, 2));
            let timeout = args.get(3).filter(|v| !matches!(v, Value::Null)).map(Value::to_i64);
            let key = s(args, 4);
            let clock = rt.clock();
            let (id, pushed) = with_mut(name, |m| m.say(&text, kind, actions.clone(), timeout, &key, &clock));
            closed(rt, name, &pushed);
            // (said once, politely; an error at once)
            rt.announce(&spoken(&text, &actions), kind == Kind::Error);
            told(rt, name);
            Some(Value::Integer(id))
        }
        // Update(Id, Text)
        "update" => {
            let id = int(args, 0, 0);
            let text = clean(&s(args, 1));
            let did = with_mut(name, |m| {
                let Some(t) = m.toast_mut(id) else { return false };
                t.text = text.clone();
                t.armed = false;
                if let Some(e) = m.history.iter_mut().find(|e| e.id == id) {
                    e.text = text;
                }
                true
            });
            Some(basic_bool(did))
        }
        "close" => {
            let id = int(args, 0, 0);
            if let Some(c) = with_mut(name, |m| m.dismiss(id)) {
                closed(rt, name, &[c]);
            }
            Some(Value::Null)
        }
        "closeall" => {
            let all: Vec<Closed> = with_mut(name, |m| {
                let ids: Vec<i64> = m.toasts.iter().map(|t| t.id).collect();
                ids.into_iter().filter_map(|id| m.dismiss(id)).collect()
            });
            closed(rt, name, &all);
            Some(Value::Null)
        }
        // Press(Id, [Index]): the action, as a click would
        "press" => {
            let (id, index) = (int(args, 0, 0), int(args, 1, 1));
            let found = with(name, |m| {
                m.toast(id).map(|t| (t.key.clone(), t.actions.len())).or_else(|| m.entry(id).filter(|e| e.works((index - 1).max(0) as usize)).map(|e| (e.key.clone(), e.actions.len())))
            })
            .flatten();
            let Some((key, n)) = found else { return Some(basic_bool(false)) };
            let Some(i) = nth(index - 1).filter(|&i| i < n) else { return Some(basic_bool(false)) };
            rt_user(rt, name, User::Action { id, key, index: i });
            Some(basic_bool(true))
        }
        // Retire(Key, [Reason])
        "retire" => {
            let reason = if args.len() > 1 { s(args, 1) } else { crate::i18n::tr_ui("This can no longer be done.") };
            Some(Value::Integer(with_mut(name, |m| m.retire(&s(args, 0), &reason)) as i64))
        }
        // SetArea(Left, Top, Width, Height): where the toasts sit
        "setarea" => {
            let (x, y, w, h) = (int(args, 0, 0), int(args, 1, 0), int(args, 2, 0), int(args, 3, 0));
            with_mut(name, |m| m.area = (w > 0 && h > 0).then_some((x, y, w, h)));
            Some(Value::Null)
        }
        "showhistory" => {
            let anchor = (args.len() >= 2).then(|| (int(args, 0, 0), int(args, 1, 0)));
            // (the toasts on screen are in the history: they go, so the card
            // never covers one)
            let gone: Vec<Closed> = with_mut(name, |m| {
                if anchor.is_some() {
                    m.anchor = anchor;
                }
                m.history_open = true;
                m.history_top = 0;
                m.mark_seen();
                let ids: Vec<i64> = m.toasts.iter().map(|t| t.id).collect();
                ids.into_iter().filter_map(|id| m.dismiss(id)).collect()
            });
            closed(rt, name, &gone);
            told(rt, name);
            rt.restructure();
            rt.focus(name);
            Some(Value::Null)
        }
        "hidehistory" => {
            with_mut(name, |m| m.history_open = false);
            Some(Value::Null)
        }
        // SetFocus: the keyboard on the newest toast's first action (its card
        // when it has none) — Enter presses it, Tab goes on, Escape closes the
        // toast; while the history is open, on the history. The focus itself
        // is the runtime's own SetFocus (None: not handled here).
        "setfocus" => {
            with_mut(name, |m| {
                if !m.history_open {
                    let newest = m.toasts.iter().rev().find(|t| t.leaving.is_none()).map(|t| (t.id, !t.actions.is_empty()));
                    m.focus = newest.map(|(id, has)| if has { Hit::Action(id, 0) } else { Hit::Card(id) });
                }
            });
            None
        }
        "clearhistory" => {
            with_mut(name, Notify::clear_history);
            told(rt, name);
            Some(Value::Null)
        }
        "markseen" => {
            with_mut(name, Notify::mark_seen);
            told(rt, name);
            Some(Value::Null)
        }
        // what is on screen, for people, RAI and AI apps
        "text" | "kind" | "key" | "actiontext" | "id" => {
            let i = nth(int(args, 0, -1));
            let n = int(args, 1, 1);
            Some(Value::String(
                with(name, |m| {
                    let t = m.visible().nth(i?)?;
                    Some(match method {
                        "text" => t.text.clone(),
                        "kind" => t.kind.name().to_string(),
                        "key" => t.key.clone(),
                        "id" => t.id.to_string(),
                        _ => t.actions.get(usize::try_from(n - 1).ok()?).map(|a| plain(a)).unwrap_or_default(),
                    })
                })
                .flatten()
                .unwrap_or_default()
                .into(),
            ))
        }
        "historytext" | "historykind" | "historytime" | "historykey" | "historyaction" | "historyid" => {
            let i = nth(int(args, 0, -1));
            let n = int(args, 1, 1);
            Some(Value::String(
                with(name, |m| {
                    let e = m.history.get(i?)?;
                    Some(match method {
                        "historytext" => e.text.clone(),
                        "historykind" => e.kind.name().to_string(),
                        "historytime" => e.clock.clone(),
                        "historykey" => e.key.clone(),
                        "historyid" => e.id.to_string(),
                        _ => {
                            let k = usize::try_from(n - 1).ok()?;
                            let a = e.actions.get(k)?;
                            match e.retired.get(k).cloned().flatten() {
                                Some(why) => format!("{} ({why})", plain(a)),
                                None => plain(a),
                            }
                        }
                    })
                })
                .flatten()
                .unwrap_or_default()
                .into(),
            ))
        }
        _ => None,
    }
}

/// Its properties its model answers.
pub fn rt_get<R: Runtime>(_rt: R, name: &str, prop: &str) -> Option<Value> {
    let d = Notify::default();
    Some(match prop {
        "position" => Value::Integer(with(name, |m| m.position.index()).unwrap_or(d.position.index())),
        "maxvisible" => Value::Integer(with(name, |m| m.max_visible).unwrap_or(d.max_visible) as i64),
        "timeout" => Value::Integer(with(name, |m| m.timeout).unwrap_or(d.timeout)),
        "timescale" => Value::Integer(with(name, |m| m.time_scale).unwrap_or(d.time_scale)),
        "historysize" => Value::Integer(with(name, |m| m.history_size).unwrap_or(d.history_size) as i64),
        "margin" => Value::Integer(with(name, |m| m.margin).unwrap_or(d.margin)),
        "count" => Value::Integer(with(name, |m| m.visible().count()).unwrap_or(0) as i64),
        "historycount" => Value::Integer(with(name, |m| m.history.len()).unwrap_or(0) as i64),
        "unseencount" => Value::Integer(with(name, Notify::unseen).unwrap_or(0) as i64),
        "historyopen" => basic_bool(with(name, |m| m.history_open).unwrap_or(false)),
        _ => return None,
    })
}

/// Its properties its model keeps: whether `prop` was one.
pub fn rt_set<R: Runtime>(_rt: R, name: &str, prop: &str, v: &Value) -> bool {
    match prop {
        "position" => {
            if let Some(p) = Position::parse(&v.to_string_val()).or_else(|| Position::parse(&v.to_i64().to_string())) {
                with_mut(name, |m| m.position = p);
            }
        }
        "maxvisible" => with_mut(name, |m| m.max_visible = v.to_i64().clamp(1, 8) as usize),
        "timeout" => with_mut(name, |m| m.timeout = v.to_i64().max(-1)),
        "timescale" => with_mut(name, |m| m.time_scale = v.to_i64().max(0)),
        "historysize" => with_mut(name, |m| {
            m.history_size = v.to_i64().clamp(1, 500) as usize;
            let n = m.history_size;
            m.history.truncate(n);
        }),
        "margin" => with_mut(name, |m| m.margin = v.to_i64().clamp(0, 200)),
        "historyopen" => {
            let on = truth(v);
            with_mut(name, |m| m.history_open = on);
        }
        _ => return false,
    }
    true
}

/// What the user did.
pub fn rt_user<R: Runtime>(rt: R, name: &str, action: User) {
    match action {
        User::Action { id, key, index } => {
            // (a toast closes when its action is pressed; an action that can't
            // work any more does nothing)
            let works = with(name, |m| m.toast(id).map(|t| index < t.actions.len()).or_else(|| m.entry(id).map(|e| e.works(index)))).flatten().unwrap_or(false);
            if !works {
                return;
            }
            let was_toast = with(name, |m| m.toast(id).is_some()).unwrap_or(false);
            rt.fire(name, "OnAction", &[Value::Integer(id), Value::Integer(index as i64 + 1), Value::String(key.into())]);
            if was_toast {
                if let Some(c) = with_mut(name, |m| m.dismiss(id)) {
                    closed(rt, name, &[Closed { reason: "action".into(), ..c }]);
                }
            }
        }
        User::Closed(list) => closed(rt, name, &list),
        User::History => told(rt, name),
        User::Redraw => {}
    }
}

