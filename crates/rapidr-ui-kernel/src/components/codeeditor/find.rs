//! The box over the text's top right: find (Ctrl+F: incremental, case,
//! whole word, regex, in the selection, "3 of 17", F3 / Shift+F3), replace
//! (Ctrl+H: one, all), go to line (Ctrl+G) and rename (F2, through the
//! language service). Its fields are the kernel's own small line editors.

use std::ops::Range;

use rapidr_editor::{SearchQuery, Searcher, Selection, Selections};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Place, Rect};
use rapidr_value::objects::text::text_size;

use super::{lang, reveal_byte, reveal_caret, Ctx};
use crate::components::KeyIn;
use crate::input::Clipboard;
use crate::paint::Painter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Find,
    Replace,
    Goto,
    Rename,
}

/// A one-line field: its text, the caret (bytes), everything selected (a
/// field just opened: typing replaces it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Field {
    pub text: String,
    pub caret: usize,
    pub all: bool,
}

impl Field {
    fn with(text: &str) -> Field {
        Field { text: text.to_string(), caret: text.len(), all: true }
    }

    /// A key on the field: whether it changed the text.
    fn key(&mut self, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let m = k.mods;
        let cmd = m.command || m.ctrl;
        let before = self.text.clone();
        match k.vk {
            8 => {
                if self.all {
                    self.text.clear();
                    self.caret = 0;
                } else if self.caret > 0 {
                    let p = self.text[..self.caret].chars().next_back().map_or(0, char::len_utf8);
                    self.text.replace_range(self.caret - p..self.caret, "");
                    self.caret -= p;
                }
            }
            46 => {
                if self.all {
                    self.text.clear();
                    self.caret = 0;
                } else if self.caret < self.text.len() {
                    let n = self.text[self.caret..].chars().next().map_or(0, char::len_utf8);
                    self.text.replace_range(self.caret..self.caret + n, "");
                }
            }
            37 => self.caret = self.text[..self.caret].char_indices().next_back().map_or(0, |(i, _)| i),
            39 => self.caret = self.text[self.caret..].chars().next().map_or(self.caret, |c| self.caret + c.len_utf8()),
            36 => self.caret = 0,
            35 => self.caret = self.text.len(),
            65 if cmd => {
                self.all = true;
                return false;
            }
            67 if cmd => {
                clip.set_text(&self.text);
                return false;
            }
            86 if cmd => {
                if let Some(t) = clip.get_text() {
                    let t = t.lines().next().unwrap_or("").to_string();
                    self.insert(&t);
                }
            }
            _ if !cmd && !k.text.is_empty() && !k.text.chars().any(char::is_control) => self.insert(k.text),
            _ => return false,
        }
        if !matches!(k.vk, 65) {
            self.all = false;
        }
        self.text != before
    }

    fn insert(&mut self, t: &str) {
        if self.all {
            self.text.clear();
            self.caret = 0;
            self.all = false;
        }
        self.text.insert_str(self.caret, t);
        self.caret += t.len();
    }
}

/// The box's state.
#[derive(Clone, Debug)]
pub struct FindBox {
    pub mode: Mode,
    pub query: Field,
    pub replace: Field,
    /// Which field the keys go to (0 the first, 1 the replacement).
    pub field: usize,
    pub focused: bool,
    pub case: bool,
    pub word: bool,
    pub regex: bool,
    pub in_selection: bool,
    /// Where the caret was when the box opened (incremental search starts
    /// there; rename's word).
    pub origin: usize,
    /// The matches for (query, options, document version).
    cache: Option<(String, u64, Vec<Range<usize>>)>,
    /// A message under the fields (a rename refused, a bad pattern).
    pub message: String,
    /// The selection the search is in.
    pub scope: Option<Range<usize>>,
}

impl FindBox {
    fn options(&self) -> String {
        let mut o = Vec::new();
        if self.case {
            o.push("case");
        }
        if self.word {
            o.push("word");
        }
        if self.regex {
            o.push("regex");
        }
        o.join(",")
    }

    fn query(&self) -> Option<SearchQuery> {
        (!self.query.text.is_empty() && matches!(self.mode, Mode::Find | Mode::Replace)).then(|| SearchQuery::with_options(self.query.text.clone(), &self.options()))
    }
}

/// UI font of the box.
fn ui_font() -> Font {
    Font { name: "Segoe UI".into(), size: -12, ..Font::default() }
}

/// Opens the box (the selected text, a word at the caret, the line number,
/// the name to rename as its first field).
pub fn open(x: &mut Ctx, mode: Mode) {
    let p = x.c.doc.selections().primary();
    let picked = if !p.is_empty() && !x.c.doc.slice(p.range()).contains('\n') {
        x.c.doc.slice(p.range()).into_owned()
    } else {
        x.c.doc.word_at(p.head).map(|r| x.c.doc.slice(r).into_owned()).unwrap_or_default()
    };
    let keep = x.ui.find.clone();
    let mut f = FindBox {
        mode,
        query: Field::default(),
        replace: keep.as_ref().map_or_else(Field::default, |k| Field::with(&k.replace.text)),
        field: 0,
        focused: true,
        case: keep.as_ref().is_some_and(|k| k.case),
        word: keep.as_ref().is_some_and(|k| k.word),
        regex: keep.as_ref().is_some_and(|k| k.regex),
        in_selection: false,
        origin: p.start(),
        cache: None,
        message: String::new(),
        scope: None,
    };
    f.query = match mode {
        Mode::Goto => Field::with(""),
        Mode::Rename => {
            if lang::served(x) {
                Field::with(&picked)
            } else {
                f.message = "No language service for this file".into();
                Field::with(&picked)
            }
        }
        _ if picked.is_empty() => keep.map_or_else(Field::default, |k| Field::with(&k.query.text)),
        _ => Field::with(&picked),
    };
    if mode == Mode::Replace && !f.query.text.is_empty() {
        f.field = 1;
    }
    x.ui.find = Some(f);
    x.c.hide_popups();
}

/// The matches in the whole text (cached per query and version).
pub fn all_matches(x: &mut Ctx) -> Vec<Range<usize>> {
    let version = x.c.doc.version();
    let Some(f) = &mut x.ui.find else { return Vec::new() };
    let Some(q) = f.query() else { return Vec::new() };
    let key = format!("{}\u{1}{}", f.query.text, f.options());
    if let Some((k, v, m)) = &f.cache {
        if *k == key && *v == version {
            return m.clone();
        }
    }
    let found = match Searcher::new(&q) {
        Ok(s) => {
            let text = x.c.doc.text();
            let within = f.scope.clone().unwrap_or(0..text.len());
            f.message.clear();
            let mut all = s.find_all(&text, within);
            all.truncate(20_000);
            all
        }
        Err(e) => {
            f.message = e.0;
            Vec::new()
        }
    };
    f.cache = Some((key, version, found.clone()));
    found
}

/// The matches inside `visible`, each with whether it's the current one
/// (the primary selection).
pub fn matches_in(x: &mut Ctx, visible: Range<usize>) -> Vec<(Range<usize>, bool)> {
    if x.ui.find.is_none() {
        return Vec::new();
    }
    let p = x.c.doc.selections().primary().range();
    all_matches(x).into_iter().filter(|r| r.end >= visible.start && r.start <= visible.end).map(|r| (r.clone(), r == p)).collect()
}

/// F3 / Shift+F3 / Enter: the next (previous) match selected.
pub fn step(x: &mut Ctx, forward: bool) {
    let all = all_matches(x);
    if all.is_empty() {
        // (no box: the last search, or the word at the caret)
        if x.ui.find.is_none() {
            let p = x.c.doc.selections().primary();
            let word = if p.is_empty() { x.c.doc.word_at(p.head).map(|r| x.c.doc.slice(r).into_owned()) } else { Some(x.c.doc.slice(p.range()).into_owned()) };
            let (text, opts) = x.c.last_find.clone().unwrap_or((word.unwrap_or_default(), String::new()));
            x.c.find(&text, &opts, forward);
            reveal_caret(x, true);
        }
        return;
    }
    let p = x.c.doc.selections().primary();
    let next = if forward { all.iter().find(|r| r.start > p.start() || (r.start == p.start() && p.is_empty())).or(all.first()) } else { all.iter().rev().find(|r| r.start < p.start()).or(all.last()) };
    if let Some(r) = next.cloned() {
        x.c.doc.set_selections(Selections::single(Selection::new(r.start, r.end)));
        x.c.unhide_carets();
        reveal_byte(x, r.start, true);
    }
    if let Some(f) = &x.ui.find {
        x.c.last_find = Some((f.query.text.clone(), f.options()));
    }
}

/// Selects the first match from where the search started (as the query is
/// typed).
fn incremental(x: &mut Ctx) {
    let Some(origin) = x.ui.find.as_ref().map(|f| f.origin) else { return };
    let all = all_matches(x);
    if let Some(r) = all.iter().find(|r| r.start >= origin).or(all.first()).cloned() {
        x.c.doc.set_selections(Selections::single(Selection::new(r.start, r.end)));
        x.c.unhide_carets();
        reveal_byte(x, r.start, true);
    }
}

/// The box's keys.
pub fn key(x: &mut Ctx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
    let Some(mut f) = x.ui.find.take() else { return false };
    let m = k.mods;
    let mut close = false;
    let mut changed = false;
    match (k.vk, m.alt, m.command || m.ctrl) {
        (27, _, _) => close = true,
        (9, false, false) if f.mode == Mode::Replace => f.field = 1 - f.field,
        (67, true, _) => f.case = !f.case,
        (87, true, _) => f.word = !f.word,
        (82, true, _) => f.regex = !f.regex,
        (76, true, _) => {
            f.in_selection = !f.in_selection;
            f.scope = if f.in_selection {
                let p = x.c.doc.selections().primary();
                (!p.is_empty()).then(|| p.range())
            } else {
                None
            };
        }
        (13, _, cmd) => {
            x.ui.find = Some(f.clone());
            match f.mode {
                Mode::Find => step(x, !m.shift),
                Mode::Replace if f.field == 1 || cmd => {
                    let q = f.query();
                    if let Some(q) = q {
                        let t = super::input::now_ms();
                        if cmd && m.alt {
                            let n = x.c.doc.replace_all(&q, &f.replace.text, f.in_selection, t).unwrap_or(0);
                            f.message = format!("Replaced {n}");
                        } else {
                            let _ = x.c.doc.replace_next(&q, &f.replace.text, t);
                        }
                        super::input::edited(x, None);
                        x.ui.find = Some(f.clone());
                        step(x, true);
                    }
                }
                Mode::Replace => step(x, !m.shift),
                Mode::Goto => {
                    let mut parts = f.query.text.split([':', ',']);
                    let line = parts.next().and_then(|s| s.trim().parse::<i64>().ok());
                    let col = parts.next().and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(1);
                    if let Some(line) = line {
                        let at = x.c.at_line_col(line, col);
                        x.c.doc.set_selections(Selections::caret(at));
                        x.c.unhide_carets();
                        reveal_byte(x, at, true);
                        close = true;
                    }
                }
                Mode::Rename => {
                    let new = f.query.text.trim().to_string();
                    match lang::rename(x, f.origin, &new) {
                        Ok(()) => close = true,
                        Err(e) => f.message = e,
                    }
                }
            }
            if let Some(now) = x.ui.find.take() {
                f.cache = now.cache;
            }
        }
        _ => {
            let field = if f.field == 1 { &mut f.replace } else { &mut f.query };
            changed = field.key(k, clip);
        }
    }
    if close {
        x.ui.find = None;
        if f.mode == Mode::Find || f.mode == Mode::Replace {
            x.c.last_find = Some((f.query.text.clone(), f.options()));
        }
        return true;
    }
    let incremental_now = changed && f.field == 0 && matches!(f.mode, Mode::Find | Mode::Replace);
    f.cache = if changed { None } else { f.cache };
    x.ui.find = Some(f);
    if incremental_now {
        incremental(x);
    }
    true
}

/// Where the box and its parts are (component pixels).
struct Parts {
    rect: Rect,
    query: Rect,
    replace: Option<Rect>,
    toggles: [Rect; 3],
    count: Rect,
    prev: Rect,
    next: Rect,
    close: Rect,
    replace_one: Option<Rect>,
    replace_all: Option<Rect>,
}

fn parts(x: &Ctx, f: &FindBox) -> Parts {
    let g = x.ui.geo;
    let w = (g.text.2 - 24).clamp(160, 460);
    let rows = if f.mode == Mode::Replace { 2 } else { 1 };
    let h = 8 + rows * 28 + if f.message.is_empty() { 0 } else { 20 };
    let rx = g.text.0 + g.text.2 - w - 14;
    let rect = (rx, g.text.1 + 4, w, h);
    let (bx, by) = (rx + 8, rect.1 + 6);
    let simple = matches!(f.mode, Mode::Goto | Mode::Rename);
    let tail = if simple { 30 } else { 196 };
    let query = (bx, by, (w - 16 - tail).max(60), 24);
    let t0 = query.0 + query.2 + 4;
    let toggles = [(t0, by + 2, 24, 20), (t0 + 26, by + 2, 24, 20), (t0 + 52, by + 2, 24, 20)];
    let count = (t0 + 80, by, 56, 24);
    let prev = (t0 + 138, by + 2, 20, 20);
    let next = (t0 + 158, by + 2, 20, 20);
    let close = (rx + w - 26, by + 2, 20, 20);
    let (replace, replace_one, replace_all) = if f.mode == Mode::Replace {
        let ry = by + 28;
        (Some((bx, ry, query.2, 24)), Some((t0, ry + 2, 52, 20)), Some((t0 + 56, ry + 2, 40, 20)))
    } else {
        (None, None, None)
    };
    Parts { rect, query, replace, toggles, count, prev, next, close, replace_one, replace_all }
}

fn hit(r: Rect, x: f64, y: f64) -> bool {
    x >= r.0 as f64 && y >= r.1 as f64 && x < (r.0 + r.2) as f64 && y < (r.1 + r.3) as f64
}

/// A press on the box: whether it was on it.
pub fn mouse_down(x: &mut Ctx, mx: f64, my: f64) -> bool {
    let Some(mut f) = x.ui.find.clone() else { return false };
    let p = parts(x, &f);
    if !hit(p.rect, mx, my) {
        return false;
    }
    f.focused = true;
    if hit(p.close, mx, my) {
        x.ui.find = None;
        return true;
    }
    for (i, t) in p.toggles.iter().enumerate() {
        if hit(*t, mx, my) && matches!(f.mode, Mode::Find | Mode::Replace) {
            match i {
                0 => f.case = !f.case,
                1 => f.word = !f.word,
                _ => f.regex = !f.regex,
            }
            f.cache = None;
        }
    }
    if p.replace.is_some_and(|r| hit(r, mx, my)) {
        f.field = 1;
    } else if hit(p.query, mx, my) {
        f.field = 0;
    }
    let (go_prev, go_next) = (hit(p.prev, mx, my), hit(p.next, mx, my));
    let one = p.replace_one.is_some_and(|r| hit(r, mx, my));
    let all = p.replace_all.is_some_and(|r| hit(r, mx, my));
    x.ui.find = Some(f.clone());
    if matches!(f.mode, Mode::Find | Mode::Replace) && (go_prev || go_next) {
        step(x, go_next);
    }
    if one || all {
        if let Some(q) = f.query() {
            let t = super::input::now_ms();
            if all {
                let _ = x.c.doc.replace_all(&q, &f.replace.text, f.in_selection, t);
            } else {
                let _ = x.c.doc.replace_next(&q, &f.replace.text, t);
            }
            super::input::edited(x, None);
            if let Some(b) = &mut x.ui.find {
                b.focused = true;
            }
        }
    }
    true
}

/// Draws the box.
pub fn paint(x: &mut Ctx, p: &mut Painter) {
    let Some(f) = x.ui.find.clone() else { return };
    let sc = x.scheme;
    let parts = parts(x, &f);
    let font = ui_font();
    let total = all_matches(x).len();
    let current = {
        let pr = x.c.doc.selections().primary().range();
        all_matches(x).iter().position(|r| *r == pr)
    };
    let (rx, ry, rw, rh) = parts.rect;
    // (a shadow, the box)
    p.op(Op::Round { rect: (rx + 1, ry + 2, rw, rh), radius: 4.0, fill: Some(super::paint::blend(0x000000, sc.background, 0.18)), stroke: None, width: 1.0 });
    p.op(Op::Round { rect: parts.rect, radius: 4.0, fill: Some(sc.popup), stroke: Some(sc.popup_border), width: 1.0 });
    let field = |p: &mut Painter, r: Rect, fl: &Field, active: bool, hint: &str| {
        p.op(Op::Round { rect: r, radius: 3.0, fill: Some(sc.background), stroke: Some(if active && f.focused { sc.caret } else { sc.popup_border }), width: 1.0 });
        let th = text_size("Ag", &font).1;
        let ty = r.1 + (r.3 - th) / 2;
        p.clipped((r.0 + 2, r.1 + 1, r.2 - 4, r.3 - 2), |p| {
            if fl.text.is_empty() {
                p.text((r.0 + 6, ty, r.2 - 8, th), hint, &font, sc.popup_detail, Place::TopLeft);
            } else {
                if fl.all && active && f.focused {
                    let w = text_size(&fl.text, &font).0;
                    p.fill((r.0 + 5, ty, w + 2, th), sc.selection);
                }
                p.text((r.0 + 6, ty, r.2 - 8, th), &fl.text, &font, sc.popup_text, Place::TopLeft);
            }
            if active && f.focused && !fl.all {
                let cx = r.0 + 6 + text_size(&fl.text[..fl.caret.min(fl.text.len())], &font).0;
                p.fill((cx, ty, 1, th), sc.caret);
            }
        });
    };
    let hint = match f.mode {
        Mode::Find | Mode::Replace => "Find".to_string(),
        Mode::Goto => format!("Go to line (1–{}), :column", x.c.doc.line_count()),
        Mode::Rename => "New name".to_string(),
    };
    field(p, parts.query, &f.query, f.field == 0, &hint);
    if let Some(r) = parts.replace {
        field(p, r, &f.replace, f.field == 1, "Replace");
    }
    let th = text_size("Ag", &font).1;
    if matches!(f.mode, Mode::Find | Mode::Replace) {
        for (i, (r, label, on)) in [(parts.toggles[0], "Aa", f.case), (parts.toggles[1], "ab", f.word), (parts.toggles[2], ".*", f.regex)].into_iter().enumerate() {
            if on {
                p.op(Op::Round { rect: r, radius: 3.0, fill: Some(sc.popup_selected), stroke: Some(sc.caret), width: 1.0 });
            }
            let color = if on { sc.popup_selected_text } else { sc.popup_text };
            p.text((r.0, r.1 + (r.3 - th) / 2, r.2, th), label, &font, color, Place::TopCenter);
            if i == 1 {
                // (whole word: underlined)
                let w = text_size("ab", &font).0;
                p.fill((r.0 + (r.2 - w) / 2, r.1 + (r.3 + th) / 2, w, 1), color);
            }
        }
        let count = if f.query.text.is_empty() {
            String::new()
        } else if total == 0 {
            "No results".into()
        } else {
            match current {
                Some(i) => format!("{} of {total}", i + 1),
                None => format!("? of {total}"),
            }
        };
        let cc = if total == 0 && !f.query.text.is_empty() { sc.error } else { sc.popup_detail };
        p.text((parts.count.0, parts.count.1 + (24 - th) / 2, parts.count.2, th), &count, &font, cc, Place::TopLeft);
        p.icon("glyphs/arrow-up", parts.prev, Some(sc.popup_text), total == 0);
        p.icon("glyphs/arrow-down", parts.next, Some(sc.popup_text), total == 0);
        if let (Some(a), Some(b)) = (parts.replace_one, parts.replace_all) {
            for (r, label) in [(a, "Replace"), (b, "All")] {
                p.op(Op::Round { rect: r, radius: 3.0, fill: Some(sc.fold_box), stroke: None, width: 1.0 });
                p.text((r.0, r.1 + (r.3 - th) / 2, r.2, th), label, &font, sc.popup_text, Place::TopCenter);
            }
        }
    }
    p.icon("actions/close", parts.close, Some(sc.popup_text), false);
    if !f.message.is_empty() {
        p.text((rx + 10, ry + rh - 20, rw - 20, th), &f.message, &font, sc.error, Place::TopLeft);
    }
}
