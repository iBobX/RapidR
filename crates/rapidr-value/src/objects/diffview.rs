//! RDIFFVIEW's model (docs/ide-plan.md I2, docs/ide-components.md §3.2):
//! two texts compared line by line, the differences as hunks the user (or
//! the program) accepts or rejects one by one, and the text that results.
//! The same on every runtime; the UI kernel's `components/diffview.rs`
//! draws it (split: the two texts side by side; inline: one column,
//! removed lines then added ones) and passes it the mouse and the keys.
//!
//! - **The diff** is a line diff by Myers' algorithm ([`myers`]): the
//!   fewest lines removed and added. A **hunk** is a run of changed lines
//!   (lines removed from the left text, lines added from the right one)
//!   between unchanged lines — no context lines, so two changes with no
//!   unchanged line between them are one hunk. Within a hunk, the k-th
//!   removed line is paired with the k-th added one and their changed
//!   characters are found by the same algorithm (a character diff, shown
//!   darker); two lines with too little in common get no character marks.
//! - **Line breaks** are '\n': CR LF and a lone CR become '\n' when a text
//!   is set (as RCODEEDITOR does). A text ending in a line break ends with
//!   an empty line, so ResultText gives the line breaks back exactly.
//! - **Hunk states**: each starts undecided (0); AcceptHunk takes the right
//!   side's lines (1), RejectHunk keeps the left side's (-1). **ResultText**
//!   is the left text with every accepted hunk's lines replaced by the
//!   right side's — an undecided hunk counts as rejected.
//! - **Indexes** (hunks, CurrentHunk) count from 0; line numbers shown
//!   count from 1. The program's own changes fire no OnHunkChange; the
//!   user's clicks and keys do (the kernel fires it).

pub mod myers;

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use rapidr_editor::{Highlighter, Language, StateId, Token, ROOT};

use super::font::Font;
use super::ops::Rect;
use crate::scrollbars::{Child, Scroller};
use crate::{v_int, v_str, Value};

/// A text line's height (JetBrains Mono at 13 pixels).
pub const ROW_H: i64 = 18;
/// A hunk's header band (its range, its state, Accept and Reject).
pub const HEADER_H: i64 = 26;
/// The code font's size in pixels.
pub const CODE_PX: i64 = 13;
/// Display columns a tab takes.
pub const TAB: usize = 4;
/// The Accept / Reject buttons' height.
pub const BUTTON_H: i64 = 20;
/// Lines longer than this get no character marks.
const MAX_INLINE_CHARS: usize = 2000;

/// The code font (built in: rapidr_value::objects::text::CODE_FACE).
pub fn code_font(styles: u8) -> Font {
    Font { name: super::text::CODE_FACE.into(), size: -CODE_PX, color: 0, styles }
}

/// The code font's advance (every glyph's: it is monospaced) in logical
/// pixels, unrounded.
pub fn code_advance() -> f64 {
    static ADVANCE: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *ADVANCE.get_or_init(|| {
        let face = ttf_parser::Face::parse(super::text::code_face_file(), 0).ok();
        face.and_then(|f| {
            let g = f.glyph_index('0')?;
            Some(f64::from(f.glyph_hor_advance(g)?) * CODE_PX as f64 / f64::from(f.units_per_em()))
        })
        .unwrap_or(7.8)
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Split,
    Inline,
}

/// A run of changed lines: `left` removed (0-based line indexes of the
/// left text), `right` added (of the right text). One of them may be
/// empty (a pure insertion or deletion).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    pub left: Range<usize>,
    pub right: Range<usize>,
}

/// What a row shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// An unchanged line (both sides' numbers).
    Same,
    /// A hunk's header band.
    Header,
    /// Split mode: a hunk's k-th left and right lines side by side (either
    /// may be missing: a filler).
    Pair,
    /// Inline mode: a removed line.
    Removed,
    /// Inline mode: an added line.
    Added,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// Its top in the content (logical pixels).
    pub y: i64,
    pub kind: RowKind,
    pub hunk: Option<usize>,
    pub left: Option<usize>,
    pub right: Option<usize>,
}

impl Row {
    pub fn height(&self) -> i64 {
        if self.kind == RowKind::Header {
            HEADER_H
        } else {
            ROW_H
        }
    }
}

/// What is at a point of the view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Accept(usize),
    Reject(usize),
    /// A hunk's header or one of its lines.
    Hunk(usize),
    /// An unchanged line.
    Line,
    /// The scroll bar.
    Bars,
    Nothing,
}

/// One text, its lines and their colours.
#[derive(Clone, Debug)]
pub struct Side {
    pub lines: Vec<String>,
    hl: Highlighter,
    /// The tokenizer's state at the start of lines `0..starts.len()`.
    starts: Vec<StateId>,
}

impl Side {
    fn new(text: &str, lang: Arc<Language>) -> Side {
        Side { lines: text.split('\n').map(str::to_string).collect(), hl: Highlighter::new(lang), starts: vec![ROOT] }
    }

    fn set_language(&mut self, lang: Arc<Language>) {
        self.hl = Highlighter::new(lang);
        self.starts = vec![ROOT];
    }

    /// Line `line`'s tokens (byte ranges of the line), its language's.
    pub fn tokens(&mut self, line: usize) -> Vec<Token> {
        if line >= self.lines.len() {
            return Vec::new();
        }
        while self.starts.len() <= line {
            let k = self.starts.len() - 1;
            let (_, end) = self.hl.line_spans(&self.lines[k], self.starts[k]);
            self.starts.push(end);
        }
        self.hl.line_spans(&self.lines[line], self.starts[line]).0
    }
}

/// Changed characters of a line, as display-column ranges.
pub type Marks = Vec<Range<usize>>;

/// A line as shown: tabs expanded, and each character's display column
/// (`cols[i]` for char i; `cols[len]` the end).
pub fn expand(line: &str) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(line.len());
    let mut cols = Vec::with_capacity(line.len() + 1);
    let mut col = 0;
    for c in line.chars() {
        cols.push(col);
        if c == '\t' {
            let n = TAB - col % TAB;
            out.extend(std::iter::repeat_n(' ', n));
            col += n;
        } else {
            out.push(c);
            col += 1;
        }
    }
    cols.push(col);
    (out, cols)
}

/// RDIFFVIEW's state.
#[derive(Clone, Debug)]
pub struct DiffView {
    pub left: Side,
    pub right: Side,
    pub lang: Arc<Language>,
    pub mode: Mode,
    pub hunks: Vec<Hunk>,
    /// Each hunk's state: 1 accepted, -1 rejected, 0 undecided.
    pub states: Vec<i8>,
    /// The hunk the keyboard is on (-1: none).
    pub current: i64,
    pub rows: Vec<Row>,
    /// Each hunk's header row.
    pub header_rows: Vec<usize>,
    /// The content's height.
    pub height: i64,
    /// The vertical scroll bar (the view's position is its Position).
    pub bars: Scroller,
    /// The view's size (inside its border) and the labels' font, as the
    /// kernel last laid it out.
    pub view: (i64, i64),
    pub ui_font: Font,
    /// The Accept and Reject buttons' widths.
    pub button_w: (i64, i64),
    /// A hunk to scroll into view at the next layout.
    reveal: Option<usize>,
    /// Character marks of a hunk's k-th line pair (left, right), made when
    /// first shown.
    marks: HashMap<(usize, usize), (Marks, Marks)>,
    /// The button under the mouse, and the one pressed.
    pub hover: Hit,
    pub pressed: Hit,
    /// Where a scroll bar is held (for its repeat).
    pub held: Option<(i64, i64)>,
    /// Goes up whenever it must be drawn again.
    pub revision: u64,
}

impl Default for DiffView {
    fn default() -> Self {
        let lang = super::codeedit::basic();
        let mut d = DiffView {
            left: Side::new("", lang.clone()),
            right: Side::new("", lang.clone()),
            lang,
            mode: Mode::Split,
            hunks: Vec::new(),
            states: Vec::new(),
            current: -1,
            rows: Vec::new(),
            header_rows: Vec::new(),
            height: 0,
            bars: Scroller::default(),
            view: (0, 0),
            ui_font: Font::default(),
            button_w: (72, 68),
            reveal: None,
            marks: HashMap::new(),
            hover: Hit::Nothing,
            pressed: Hit::Nothing,
            held: None,
            revision: 0,
        };
        d.bars.horz.visible = false;
        d.bars.vert.tracking = true;
        d.bars.vert.increment = ROW_H;
        d.recompute();
        d
    }
}

/// '\n' line breaks (CR LF and a lone CR become one).
fn normalize(s: &str) -> String {
    if s.contains('\r') {
        s.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        s.to_string()
    }
}

/// The hunks of two line lists: runs of changed lines between unchanged
/// ones.
pub fn line_hunks(a: &[String], b: &[String]) -> Vec<Hunk> {
    let mut ids: HashMap<&str, u32> = HashMap::with_capacity(a.len() + b.len());
    let mut ia = Vec::with_capacity(a.len());
    for l in a {
        let n = ids.len() as u32;
        ia.push(*ids.entry(l.as_str()).or_insert(n));
    }
    let mut ib = Vec::with_capacity(b.len());
    for l in b {
        let n = ids.len() as u32;
        ib.push(*ids.entry(l.as_str()).or_insert(n));
    }
    let c = myers::diff(&ia, &ib, ids.len());
    let mut hunks = Vec::new();
    let (n, m) = (a.len(), b.len());
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && !c.a[i] && !c.b[j] {
            i += 1;
            j += 1;
            continue;
        }
        let (i0, j0) = (i, j);
        loop {
            if i < n && c.a[i] {
                i += 1;
            } else if j < m && c.b[j] {
                j += 1;
            } else {
                break;
            }
        }
        if (i, j) == (i0, j0) {
            // (can't happen: the kept lines pair up; never loop forever)
            break;
        }
        hunks.push(Hunk { left: i0..i, right: j0..j });
    }
    hunks
}

/// The changed characters of two paired lines (display-column ranges), or
/// none when they have too little in common (the whole line's tint says
/// enough).
pub fn char_marks(a: &str, b: &str) -> (Marks, Marks) {
    let ca: Vec<char> = a.chars().collect();
    let cb: Vec<char> = b.chars().collect();
    if ca.len() > MAX_INLINE_CHARS || cb.len() > MAX_INLINE_CHARS || ca == cb {
        return (Vec::new(), Vec::new());
    }
    let mut ids: HashMap<char, u32> = HashMap::new();
    let mut id = |c: char| {
        let n = ids.len() as u32;
        *ids.entry(c).or_insert(n)
    };
    let ia: Vec<u32> = ca.iter().map(|&c| id(c)).collect();
    let ib: Vec<u32> = cb.iter().map(|&c| id(c)).collect();
    let c = myers::diff(&ia, &ib, ids.len());
    // (too little in common: no marks — a line rewritten isn't confetti)
    let kept = c.a.iter().filter(|&&x| !x).count();
    let non_space = |v: &[char]| v.iter().filter(|c| !c.is_whitespace()).count().max(1);
    if kept * 10 < ca.len().max(cb.len()) * 4 || kept * 2 < non_space(&ca).min(non_space(&cb)) {
        return (Vec::new(), Vec::new());
    }
    (ranges(a, &c.a), ranges(b, &c.b))
}

/// The display columns of the changed characters, runs a character apart
/// joined.
fn ranges(line: &str, changed: &[bool]) -> Marks {
    let (_, cols) = expand(line);
    let mut out: Marks = Vec::new();
    let mut i = 0;
    while i < changed.len() {
        if !changed[i] {
            i += 1;
            continue;
        }
        let s = i;
        while i < changed.len() && changed[i] {
            i += 1;
        }
        let r = cols[s]..cols[i];
        match out.last_mut() {
            Some(last) if r.start <= last.end + 1 => last.end = r.end,
            _ => out.push(r),
        }
    }
    out
}

impl DiffView {
    pub fn left_text(&self) -> String {
        self.left.lines.join("\n")
    }

    pub fn right_text(&self) -> String {
        self.right.lines.join("\n")
    }

    /// The left text with every accepted hunk's lines the right side's.
    pub fn result_text(&self) -> String {
        let mut out: Vec<&str> = Vec::with_capacity(self.left.lines.len());
        let mut i = 0;
        for (h, hk) in self.hunks.iter().enumerate() {
            out.extend(self.left.lines[i..hk.left.start].iter().map(String::as_str));
            if self.states[h] == 1 {
                out.extend(self.right.lines[hk.right.clone()].iter().map(String::as_str));
            } else {
                out.extend(self.left.lines[hk.left.clone()].iter().map(String::as_str));
            }
            i = hk.left.end;
        }
        out.extend(self.left.lines[i..].iter().map(String::as_str));
        out.join("\n")
    }

    pub fn set_texts(&mut self, left: Option<&str>, right: Option<&str>) {
        if let Some(l) = left {
            self.left = Side::new(&normalize(l), self.lang.clone());
        }
        if let Some(r) = right {
            self.right = Side::new(&normalize(r), self.lang.clone());
        }
        self.recompute();
    }

    /// The diff made again (every hunk undecided, the view at the top).
    fn recompute(&mut self) {
        self.hunks = line_hunks(&self.left.lines, &self.right.lines);
        self.states = vec![0; self.hunks.len()];
        self.current = if self.hunks.is_empty() { -1 } else { 0 };
        self.marks.clear();
        self.bars.vert.position = 0;
        self.reveal = None;
        self.hover = Hit::Nothing;
        self.pressed = Hit::Nothing;
        self.build_rows();
    }

    /// The rows for the mode and the hunks' states.
    pub fn build_rows(&mut self) {
        let mut rows = Vec::with_capacity(self.left.lines.len().max(self.right.lines.len()) + 2 * self.hunks.len());
        let mut headers = Vec::with_capacity(self.hunks.len());
        let mut y = 0;
        let mut push = |rows: &mut Vec<Row>, kind, hunk, left, right| {
            let r = Row { y, kind, hunk, left, right };
            y += r.height();
            rows.push(r);
        };
        let (mut i, mut j) = (0, 0);
        for (h, hk) in self.hunks.iter().enumerate() {
            while i < hk.left.start {
                push(&mut rows, RowKind::Same, None, Some(i), Some(j));
                i += 1;
                j += 1;
            }
            headers.push(rows.len());
            push(&mut rows, RowKind::Header, Some(h), None, None);
            let (nl, nr) = (hk.left.len(), hk.right.len());
            match self.mode {
                Mode::Split => {
                    for k in 0..nl.max(nr) {
                        push(&mut rows, RowKind::Pair, Some(h), (k < nl).then_some(hk.left.start + k), (k < nr).then_some(hk.right.start + k));
                    }
                }
                Mode::Inline => {
                    // (accepted: the right side's lines are the result)
                    if self.states[h] != 1 {
                        for l in hk.left.clone() {
                            push(&mut rows, RowKind::Removed, Some(h), Some(l), None);
                        }
                    }
                    for r in hk.right.clone() {
                        push(&mut rows, RowKind::Added, Some(h), None, Some(r));
                    }
                }
            }
            i = hk.left.end;
            j = hk.right.end;
        }
        while i < self.left.lines.len() {
            push(&mut rows, RowKind::Same, None, Some(i), Some(j));
            i += 1;
            j += 1;
        }
        self.height = y;
        self.rows = rows;
        self.header_rows = headers;
        self.revision += 1;
    }

    /// Character marks of hunk `h`'s pair `k` (its k-th removed and added
    /// lines).
    pub fn marks(&mut self, h: usize, k: usize) -> (Marks, Marks) {
        let Some(hk) = self.hunks.get(h) else { return Default::default() };
        if k >= hk.left.len() || k >= hk.right.len() {
            return Default::default();
        }
        let (l, r) = (hk.left.start + k, hk.right.start + k);
        let (left, right) = (&self.left.lines, &self.right.lines);
        self.marks.entry((h, k)).or_insert_with(|| char_marks(&left[l], &right[r])).clone()
    }

    /// The pair index of a row's line within its hunk.
    pub fn pair_of(&self, row: &Row) -> Option<usize> {
        let hk = self.hunks.get(row.hunk?)?;
        match (row.left, row.right) {
            (Some(l), _) if hk.left.contains(&l) => Some(l - hk.left.start),
            (_, Some(r)) if hk.right.contains(&r) => Some(r - hk.right.start),
            _ => None,
        }
    }

    pub fn state(&self, h: usize) -> i8 {
        self.states.get(h).copied().unwrap_or(0)
    }

    /// Sets hunk `h`'s state: whether it changed.
    pub fn set_state(&mut self, h: usize, state: i8) -> bool {
        match self.states.get_mut(h) {
            Some(s) if *s != state => {
                *s = state;
                if self.mode == Mode::Inline {
                    self.rebuild_keeping_top();
                }
                self.revision += 1;
                true
            }
            _ => false,
        }
    }

    /// Shows the texts split or inline; the line at the top of the view
    /// stays there.
    pub fn set_mode(&mut self, mode: Mode) {
        if self.mode != mode {
            self.mode = mode;
            self.rebuild_keeping_top();
        }
    }

    /// The rows made again (a mode or an inline hunk's state changed), the
    /// first line in view kept where it was.
    fn rebuild_keeping_top(&mut self) {
        let top = self.top();
        let anchor = self.row_at(top).map(|i| {
            let r = self.rows[i];
            let key = if r.kind == RowKind::Header { (r.hunk, None, None) } else { (None, r.left, r.right) };
            (key, top - r.y)
        });
        self.build_rows();
        if let Some(((hunk, left, right), offset)) = anchor {
            let found = self.rows.iter().find(|r| match hunk {
                Some(h) => r.kind == RowKind::Header && r.hunk == Some(h),
                None => (left.is_some() && r.left == left) || (left.is_none() && right.is_some() && r.right == right),
            });
            if let Some(r) = found {
                let y = r.y + offset;
                self.scroll_to(y);
            }
        }
    }

    pub fn set_language(&mut self, name: &str) {
        self.lang = if name.trim().is_empty() { super::codeedit::basic() } else { super::codeedit::language_named(name) };
        self.left.set_language(self.lang.clone());
        self.right.set_language(self.lang.clone());
        self.revision += 1;
    }

    /// Makes hunk `h` the current one and scrolls it into view.
    pub fn go_to(&mut self, h: usize) -> bool {
        if h >= self.hunks.len() {
            return false;
        }
        self.current = h as i64;
        self.reveal = Some(h);
        self.apply_reveal();
        self.revision += 1;
        true
    }

    pub fn next_hunk(&mut self) -> i64 {
        let n = self.hunks.len() as i64;
        if n > 0 {
            self.go_to(if self.current + 1 < n { (self.current + 1) as usize } else { 0 });
        }
        self.current
    }

    pub fn previous_hunk(&mut self) -> i64 {
        let n = self.hunks.len() as i64;
        if n > 0 {
            self.go_to(if self.current > 0 { (self.current - 1) as usize } else { (n - 1) as usize });
        }
        self.current
    }

    // ------------------------------------------------------------ view --

    /// Lays the view out `w` × `h` (inside its border), the labels in
    /// `ui_font` (the buttons' widths come from it).
    pub fn layout(&mut self, w: i64, h: i64, ui_font: &Font) {
        if self.view != (w, h) || self.ui_font != *ui_font {
            self.view = (w, h);
            self.ui_font = ui_font.clone();
            let label = |s: &str| super::text::text_size(s, ui_font).0;
            self.button_w = (label("Accept") + 34, label("Reject") + 34);
            self.revision += 1;
        }
        self.update_bars();
        self.apply_reveal();
    }

    fn update_bars(&mut self) {
        let (w, h) = self.view;
        let pos = self.bars.vert.position;
        let content = Child { left: 0, top: -pos, width: 0, height: self.height + ROW_H / 2, align: crate::layout::Align::None, visible: true };
        self.bars.update(w, h, &[content]);
    }

    fn apply_reveal(&mut self) {
        let Some(h) = self.reveal else { return };
        let (w, vh) = self.view;
        if vh <= 0 || w <= 0 {
            return;
        }
        self.reveal = None;
        let Some(&r) = self.header_rows.get(h) else { return };
        let top = self.rows[r].y;
        let bottom = self.rows.get(self.header_rows.get(h + 1).copied().unwrap_or(self.rows.len()).saturating_sub(1)).map_or(top + HEADER_H, |l| l.y + l.height());
        let (_, ch) = self.client();
        let pos = self.bars.vert.position;
        // (shown whole if it fits; else its header near the top)
        if top < pos || bottom > pos + ch {
            let to = if bottom - top <= ch && top >= pos { bottom - ch } else { top - (ch / 6).min(3 * ROW_H) };
            self.scroll_to(to);
        }
    }

    /// The rows' area (without the scroll bar).
    pub fn client(&self) -> (i64, i64) {
        self.bars.client(self.view.0, self.view.1)
    }

    pub fn top(&self) -> i64 {
        self.bars.vert.position
    }

    pub fn scroll_to(&mut self, y: i64) {
        self.bars.vert.position = y.max(0);
        self.update_bars();
        self.revision += 1;
    }

    pub fn scroll_by(&mut self, dy: i64) {
        self.scroll_to(self.top() + dy);
    }

    /// The rows showing in the view: their indexes.
    pub fn visible_rows(&self) -> Range<usize> {
        let (_, ch) = self.client();
        let top = self.top();
        let first = self.rows.partition_point(|r| r.y + r.height() <= top);
        let last = self.rows.partition_point(|r| r.y < top + ch);
        first..last.max(first)
    }

    /// The row at content height `y`.
    pub fn row_at(&self, y: i64) -> Option<usize> {
        let i = self.rows.partition_point(|r| r.y + r.height() <= y);
        (i < self.rows.len() && self.rows[i].y <= y).then_some(i)
    }

    /// A header's Accept and Reject buttons, in the view (`y`: the header
    /// row's top in the view).
    pub fn buttons(&self, y: i64) -> (Rect, Rect) {
        let (cw, _) = self.client();
        let (aw, rw) = self.button_w;
        let by = y + (HEADER_H - BUTTON_H) / 2;
        let reject = (cw - 10 - rw, by, rw, BUTTON_H);
        let accept = (reject.0 - 6 - aw, by, aw, BUTTON_H);
        (accept, reject)
    }

    /// What is at (x, y) of the view.
    pub fn hit(&self, x: i64, y: i64) -> Hit {
        let (w, h) = self.view;
        if self.bars.on_bars(x, y, w, h) {
            return Hit::Bars;
        }
        let Some(r) = self.row_at(y + self.top()) else { return Hit::Nothing };
        let row = self.rows[r];
        let Some(hk) = row.hunk else { return Hit::Line };
        if row.kind == RowKind::Header {
            let (a, rj) = self.buttons(row.y - self.top());
            let inside = |(bx, by, bw, bh): Rect| x >= bx && x < bx + bw && y >= by && y < by + bh;
            if inside(a) {
                return Hit::Accept(hk);
            }
            if inside(rj) {
                return Hit::Reject(hk);
            }
        }
        Hit::Hunk(hk)
    }

    /// The gutter's width for line numbers (with its padding).
    pub fn number_width(&self) -> i64 {
        let digits = self.left.lines.len().max(self.right.lines.len()).to_string().len().max(2);
        (digits as f64 * code_advance()).ceil() as i64 + 14
    }

    /// The scroll bar's held part repeats.
    pub fn repeat(&mut self) {
        if let Some((x, y)) = self.held {
            let (w, h) = self.view;
            self.bars.repeat(x, y, w, h);
            self.revision += 1;
        }
    }

    /// Hunk `h` for a screen reader: "Hunk 3 of 7: lines 40–45 changed,
    /// accepted".
    pub fn hunk_summary(&self, h: usize) -> String {
        let Some(hk) = self.hunks.get(h) else { return String::new() };
        let span = |r: &Range<usize>| if r.len() == 1 { format!("line {}", r.start + 1) } else { format!("lines {}\u{2013}{}", r.start + 1, r.end) };
        let what = match (hk.left.is_empty(), hk.right.is_empty()) {
            (false, false) => format!("{} changed", span(&hk.left)),
            (true, _) => {
                let n = hk.right.len();
                format!("{n} line{} added after line {}", if n == 1 { "" } else { "s" }, hk.left.start)
            }
            (_, true) => format!("{} removed", span(&hk.left)),
        };
        let state = match self.state(h) {
            1 => "accepted",
            -1 => "rejected",
            _ => "undecided",
        };
        format!("Hunk {} of {}: {what}, {state}", h + 1, self.hunks.len())
    }

    /// A header's text: `@@ -40,6 +40,7 @@` (unified diff's ranges).
    pub fn hunk_label(&self, h: usize) -> String {
        let Some(hk) = self.hunks.get(h) else { return String::new() };
        let range = |r: &Range<usize>| if r.is_empty() { format!("{},0", r.start) } else { format!("{},{}", r.start + 1, r.len()) };
        format!("@@ -{} +{} @@", range(&hk.left), range(&hk.right))
    }

    // --------------------------------------------------------- program --

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "lefttext" => v_str(&self.left_text()),
            "righttext" => v_str(&self.right_text()),
            "language" => v_str(&self.lang.id),
            "mode" => v_str(match self.mode {
                Mode::Split => "split",
                Mode::Inline => "inline",
            }),
            "hunkcount" => v_int(self.hunks.len() as i64),
            "resulttext" => v_str(&self.result_text()),
            "currenthunk" => v_int(self.current),
            "acceptedcount" => v_int(self.states.iter().filter(|&&s| s == 1).count() as i64),
            "rejectedcount" => v_int(self.states.iter().filter(|&&s| s == -1).count() as i64),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "lefttext" => self.set_texts(Some(&val.to_string_val()), None),
            "righttext" => self.set_texts(None, Some(&val.to_string_val())),
            "language" => self.set_language(&val.to_string_val()),
            "mode" => self.set_mode(if val.to_string_val().trim().eq_ignore_ascii_case("inline") { Mode::Inline } else { Mode::Split }),
            "currenthunk" => {
                let i = val.to_i64();
                if i < 0 || self.hunks.is_empty() {
                    self.current = if self.hunks.is_empty() { -1 } else { 0 };
                } else {
                    self.go_to((i as usize).min(self.hunks.len() - 1));
                }
                self.revision += 1;
            }
            _ => return false,
        }
        true
    }

    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let index = || args.first().map_or(-1, Value::to_i64);
        let hunk = || usize::try_from(index()).ok();
        Some(match method {
            "accepthunk" => {
                if let Some(h) = hunk() {
                    self.set_state(h, 1);
                }
                Value::Null
            }
            "rejecthunk" => {
                if let Some(h) = hunk() {
                    self.set_state(h, -1);
                }
                Value::Null
            }
            "acceptall" | "rejectall" => {
                let s = if method == "acceptall" { 1 } else { -1 };
                for h in 0..self.hunks.len() {
                    self.set_state(h, s);
                }
                Value::Null
            }
            "hunkstate" => v_int(hunk().map_or(0, |h| i64::from(self.state(h)))),
            "nexthunk" => v_int(self.next_hunk()),
            "previoushunk" => v_int(self.previous_hunk()),
            _ => return None,
        })
    }
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    fn s(v: Option<Value>) -> String {
        v.map(|v| v.to_string_val()).unwrap_or_default()
    }

    fn lines(t: &str) -> Vec<String> {
        t.split('\n').map(str::to_string).collect()
    }

    fn view(left: &str, right: &str) -> DiffView {
        let mut d = DiffView::default();
        d.set("lefttext", &v_str(left));
        d.set("righttext", &v_str(right));
        d
    }

    #[test]
    fn hunks_of_simple_texts() {
        // (empty sides, identical, all different)
        assert!(line_hunks(&lines(""), &lines("")).is_empty());
        assert_eq!(line_hunks(&lines(""), &lines("a\nb")), [Hunk { left: 0..1, right: 0..2 }]);
        assert_eq!(line_hunks(&lines("a\nb"), &lines("")), [Hunk { left: 0..2, right: 0..1 }]);
        assert!(line_hunks(&lines("a\nb\nc"), &lines("a\nb\nc")).is_empty());
        assert_eq!(line_hunks(&lines("a\nb"), &lines("c\nd\ne")), [Hunk { left: 0..2, right: 0..3 }]);
        // (a change, an insertion, a deletion; adjacent changes are one hunk)
        let a = lines("1\n2\n3\n4\n5\n6\n7");
        let b = lines("1\nTWO\n3\n3.5\n4\n6\n7");
        assert_eq!(line_hunks(&a, &b), [Hunk { left: 1..2, right: 1..2 }, Hunk { left: 3..3, right: 3..4 }, Hunk { left: 4..5, right: 5..5 }]);
        assert_eq!(line_hunks(&lines("a\nb\nc\nd"), &lines("a\nX\nY\nd")), [Hunk { left: 1..3, right: 1..3 }]);
    }

    #[test]
    fn line_breaks_are_normalized_and_kept() {
        let d = view("a\r\nb\r\nc\r\n", "a\nb\nc\n");
        assert_eq!(s(d.get("hunkcount")), "0");
        assert_eq!(s(d.get("lefttext")), "a\nb\nc\n");
        assert_eq!(s(d.get("resulttext")), "a\nb\nc\n");
        let d = view("x\ry", "x\ny");
        assert_eq!(s(d.get("hunkcount")), "0");
        // (a missing final line break is a change of the last line)
        let mut d = view("a\nb", "a\nb\n");
        assert_eq!(s(d.get("hunkcount")), "1");
        assert_eq!(s(d.get("resulttext")), "a\nb");
        d.call("acceptall", &[]);
        assert_eq!(s(d.get("resulttext")), "a\nb\n");
    }

    #[test]
    fn accept_reject_and_the_result() {
        let left = "SUB A\n  PRINT 1\nEND SUB\n\nSUB B\n  PRINT 2\nEND SUB";
        let right = "SUB A\n  PRINT 10\nEND SUB\n\n' new\nSUB B\nEND SUB";
        let mut d = view(left, right);
        assert_eq!(d.hunks, [Hunk { left: 1..2, right: 1..2 }, Hunk { left: 4..4, right: 4..5 }, Hunk { left: 5..6, right: 6..6 }]);
        assert_eq!(s(d.get("hunkcount")), "3");
        assert_eq!(s(d.get("currenthunk")), "0");
        // (undecided counts as rejected)
        assert_eq!(s(d.get("resulttext")), left);
        d.call("accepthunk", &[v_int(1)]);
        d.call("rejecthunk", &[v_int(2)]);
        assert_eq!(s(d.call("hunkstate", &[v_int(0)])), "0");
        assert_eq!(s(d.call("hunkstate", &[v_int(1)])), "1");
        assert_eq!(s(d.call("hunkstate", &[v_int(2)])), "-1");
        assert_eq!(s(d.call("hunkstate", &[v_int(9)])), "0");
        assert_eq!(s(d.get("resulttext")), "SUB A\n  PRINT 1\nEND SUB\n\n' new\nSUB B\n  PRINT 2\nEND SUB");
        d.call("acceptall", &[]);
        assert_eq!(s(d.get("resulttext")), right);
        d.call("rejectall", &[]);
        assert_eq!(s(d.get("resulttext")), left);
        // (out of range: nothing)
        d.call("accepthunk", &[v_int(-1)]);
        d.call("accepthunk", &[v_int(3)]);
        assert_eq!(s(d.get("resulttext")), left);
        // (navigation wraps around)
        assert_eq!(s(d.call("nexthunk", &[])), "1");
        assert_eq!(s(d.call("nexthunk", &[])), "2");
        assert_eq!(s(d.call("nexthunk", &[])), "0");
        assert_eq!(s(d.call("previoushunk", &[])), "2");
        d.set("currenthunk", &v_int(1));
        assert_eq!(s(d.get("currenthunk")), "1");
        // (a new text: every hunk undecided again)
        d.set("righttext", &v_str(left));
        assert_eq!((s(d.get("hunkcount")), s(d.get("currenthunk"))), ("0".into(), "-1".into()));
    }

    #[test]
    fn rows_split_and_inline() {
        let mut d = view("a\nb\nc\nd", "a\nB\nB2\nd");
        // split: a, header, (b|B), (c|B2), d
        let kinds: Vec<_> = d.rows.iter().map(|r| (r.kind, r.left, r.right)).collect();
        assert_eq!(
            kinds,
            [
                (RowKind::Same, Some(0), Some(0)),
                (RowKind::Header, None, None),
                (RowKind::Pair, Some(1), Some(1)),
                (RowKind::Pair, Some(2), Some(2)),
                (RowKind::Same, Some(3), Some(3))
            ]
        );
        assert_eq!(d.height, 4 * ROW_H + HEADER_H);
        d.set("mode", &v_str("Inline"));
        assert_eq!(s(d.get("mode")), "inline");
        let kinds: Vec<_> = d.rows.iter().map(|r| r.kind).collect();
        assert_eq!(kinds, [RowKind::Same, RowKind::Header, RowKind::Removed, RowKind::Removed, RowKind::Added, RowKind::Added, RowKind::Same]);
        // (accepted: only the right side's lines)
        d.call("accepthunk", &[v_int(0)]);
        let kinds: Vec<_> = d.rows.iter().map(|r| r.kind).collect();
        assert_eq!(kinds, [RowKind::Same, RowKind::Header, RowKind::Added, RowKind::Added, RowKind::Same]);
    }

    #[test]
    fn the_changed_characters() {
        let (a, b) = char_marks("  PRINT \"Hello\", x", "  PRINT \"Hello, World\", x");
        assert!(a.is_empty(), "{a:?}");
        assert_eq!(b, [14..21]);
        // (tabs: display columns)
        let (a, b) = char_marks("\tx = 1", "\tx = 2");
        assert_eq!((a, b), (vec![8..9], vec![8..9]));
        // (nothing alike: no marks)
        assert_eq!(char_marks("abcdef", "uvwxyz"), (vec![], vec![]));
        let mut d = view("x = 1\ny", "x = 2\ny");
        assert_eq!(d.marks(0, 0), (vec![4..5], vec![4..5]));
        assert_eq!(d.marks(0, 1), (vec![], vec![]));
    }

    #[test]
    fn layout_hits_and_scrolling() {
        let left: String = (0..200).map(|i| format!("line {i}\n")).collect();
        let right = left.replace("line 100\n", "line one hundred\n").replace("line 150\n", "");
        let mut d = view(&left, &right);
        assert_eq!(d.hunks.len(), 2);
        d.layout(600, 300, &Font::default());
        assert!(d.bars.vert.shown);
        assert_eq!(d.visible_rows().start, 0);
        // F7: the second hunk scrolled into view, its buttons hit
        d.next_hunk();
        let r = d.header_rows[1];
        let y = d.rows[r].y - d.top();
        assert!(y >= 0 && y + HEADER_H <= 300, "{y}");
        let (a, rj) = d.buttons(y);
        assert_eq!(d.hit(a.0 + 3, a.1 + 3), Hit::Accept(1));
        assert_eq!(d.hit(rj.0 + 3, rj.1 + 3), Hit::Reject(1));
        assert_eq!(d.hit(5, y + 2), Hit::Hunk(1));
        assert_eq!(d.hit(595, 10), Hit::Bars);
        d.scroll_by(-100_000);
        assert_eq!(d.top(), 0);
        d.scroll_by(100_000);
        assert_eq!(d.top(), d.height + ROW_H / 2 - 300);
        assert_eq!(d.hunk_summary(0), "Hunk 1 of 2: line 101 changed, undecided");
        assert_eq!(d.hunk_summary(1), "Hunk 2 of 2: line 151 removed, undecided");
        assert_eq!(d.hunk_label(1), "@@ -151,1 +150,0 @@");
    }

    #[test]
    fn syntax_colours_come_from_the_language() {
        let mut d = view("PRINT \"x\" ' hi", "");
        let t = d.left.tokens(0);
        assert!(t.len() >= 3, "{t:?}");
        d.set("language", &v_str("x.sql"));
        assert_eq!(s(d.get("language")), "sql");
        d.set("language", &v_str(""));
        assert_eq!(s(d.get("language")), "rapidr-basic");
    }

    /// Two 20,000-line texts with 100 scattered changes diff in well under
    /// 50 ms (release builds; `cargo test --release`).
    #[test]
    fn big_texts_are_fast() {
        let left: Vec<String> = (0..20_000).map(|i| format!("    x{} = x{} + {} ' line {i}", i % 97, i % 89, i % 13)).collect();
        let mut right = left.clone();
        let mut seed = 7u64;
        for _ in 0..100 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let at = (seed >> 33) as usize % right.len();
            match seed % 3 {
                0 => right[at] = format!("{} ' changed", right[at]),
                1 => right.insert(at, "    PRINT \"new\"".into()),
                _ => {
                    right.remove(at);
                }
            }
        }
        let (l, r) = (left.join("\n"), right.join("\n"));
        let t = std::time::Instant::now();
        let d = view(&l, &r);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        assert!(d.hunks.len() >= 90 && d.hunks.len() <= 100, "{}", d.hunks.len());
        let mut d = d;
        d.call("acceptall", &[]);
        assert_eq!(d.result_text(), r);
        eprintln!("diff of 2 × 20,000 lines, 100 changes: {ms:.2} ms");
        if !cfg!(debug_assertions) {
            assert!(ms < 50.0, "{ms} ms");
        }
    }
}
