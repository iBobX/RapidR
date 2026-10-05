//! The kernel's text editor: QEDIT's line, QMEMO's and QRICHEDIT's
//! paragraphs (plan §4) — the text, the caret and the selection, the
//! input method's composition, and the moves and edits Windows' edit
//! controls make, over **one parley layout per paragraph**:
//!
//! - an edit relays out only the paragraphs it touched, and the program's
//!   new text keeps the layouts of the paragraphs it left alone (the common
//!   start and end: `AddStrings` in a loop lays out only what it added);
//! - drawing asks for the paragraphs a view shows ([`TextEditor::visible`]),
//!   so a long text costs only what's on screen to draw;
//! - WordWrap breaks lines at the view's width; without it a paragraph is
//!   one line and the view scrolls across.
//!
//! Positions are [`Pos`]: a paragraph and a byte offset in its *shown*
//! text, which is the text itself — or, with a PasswordChar, that
//! character once per character (so the layout never holds the secret,
//! and moves step over characters). The program's positions (SelStart,
//! SelLength) count characters, a paragraph break as one, as RapidQ's
//! controls do: [`TextEditor::selection_chars`].
//!
//! Everything is in device pixels at the editor's scale, relative to its
//! text's top left (a paragraph's layout at [`TextEditor::para_origin`]).
//! No program code, no model: the components (`components::edit`) keep
//! the shared `TextEdit` model and this in step.
//!
//! **Styled runs** (Stage 10): a paragraph's [`Span`]s — a byte range and a
//! [`RunStyle`] (colour, bold, italic, underline, strike-out, a font) laid
//! over the editor's look by parley's ranged styles. A code editor's come
//! from its syntax ([`Look::syntax`], `rapidr_value::objects::code`), made
//! again only for the paragraphs laid out again (an edit's), each from the
//! state the paragraph before left — so a syntax with constructs across
//! lines colours the paragraphs after an edit again only while their
//! starting state changes. Without a syntax a paragraph keeps the spans it
//! was given ([`TextEditor::set_spans`]: QRICHEDIT's runs, later).

use std::borrow::Cow;
use std::ops::Range;

use parley::{Affinity, Alignment, AlignmentOptions, Cursor, FontStyle, FontWeight, Layout, Selection, StyleProperty};
use rapidr_value::objects::code::Syntax;
use rapidr_value::objects::font::Font;

use super::{byte_of, chars_to, styles, Ink, TextSystem};

/// A place in the text: paragraph `para`, byte `index` of its shown text,
/// and which side of a soft line break the caret shows on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pos {
    pub para: usize,
    pub index: usize,
    pub affinity: Affinity,
}

impl Pos {
    pub fn new(para: usize, index: usize) -> Pos {
        Pos { para, index, affinity: Affinity::Downstream }
    }

    fn key(&self) -> (usize, usize) {
        (self.para, self.index)
    }
}

/// How lines sit in the view (QEDIT's / QRICHEDIT's Alignment:
/// taLeftJustify 0, taRightJustify 1, taCenter 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Left,
    Right,
    Center,
}

impl Align {
    pub fn from_prop(v: i64) -> Align {
        match v {
            1 => Align::Right,
            2 => Align::Center,
            _ => Align::Left,
        }
    }

    fn factor(self) -> f64 {
        match self {
            Align::Left => 0.0,
            Align::Right => 1.0,
            Align::Center => 0.5,
        }
    }
}

/// How the text is shown: its font and colour (0xRRGGBB), a PasswordChar,
/// Alignment, WordWrap, and the syntax it's coloured by (a code editor's).
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub font: Font,
    pub color: u32,
    pub mask: Option<char>,
    pub align: Align,
    pub wrap: bool,
    pub syntax: Syntax,
}

impl Default for Look {
    fn default() -> Self {
        Look { font: Font::default(), color: 0, mask: None, align: Align::Left, wrap: false, syntax: Syntax::None }
    }
}

/// How a run of text differs from the editor's look (`None`: as the look).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunStyle {
    /// 0xRRGGBB.
    pub color: Option<u32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strike: Option<bool>,
    /// Another font: its RapidQ name and size in logical pixels.
    pub font: Option<(String, f32)>,
}

impl RunStyle {
    /// A syntax token's colour, bold and italic (`objects::code`).
    pub fn of_token(t: rapidr_value::objects::code::Token) -> RunStyle {
        let st = t.style();
        RunStyle { color: Some(st.color), bold: Some(st.bold), italic: Some(st.italic), ..RunStyle::default() }
    }

    /// The parley styles that make it.
    fn props(&self) -> Vec<StyleProperty<'static, Ink>> {
        let mut out = Vec::new();
        if let Some((name, px)) = &self.font {
            let face = Font { name: name.clone(), size: -(px.round() as i64).max(1), ..Font::default() };
            out.extend(styles(&face, 0).into_iter().filter(|p| matches!(p, StyleProperty::FontFamily(_) | StyleProperty::FontSize(_))));
            out.push(StyleProperty::FontSize(*px));
        }
        if let Some(c) = self.color {
            out.push(StyleProperty::Brush(Ink(c)));
        }
        if let Some(b) = self.bold {
            out.push(StyleProperty::FontWeight(if b { FontWeight::BOLD } else { FontWeight::NORMAL }));
        }
        if let Some(i) = self.italic {
            out.push(StyleProperty::FontStyle(if i { FontStyle::Italic } else { FontStyle::Normal }));
        }
        if let Some(u) = self.underline {
            out.push(StyleProperty::Underline(u));
        }
        if let Some(st) = self.strike {
            out.push(StyleProperty::Strikethrough(st));
        }
        out
    }
}

/// Bytes `range` of a paragraph drawn in `style`.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub range: Range<usize>,
    pub style: RunStyle,
}

struct Para {
    /// The paragraph's text (no line break).
    text: String,
    layout: Option<Layout<Ink>>,
    /// Where it starts down the text, and how high it is.
    top: f64,
    height: f64,
    /// How far right its line sits (Alignment without WordWrap).
    dx: f64,
    /// Its styled runs (a syntax's, or given).
    spans: Vec<Span>,
    /// The syntax's state it was coloured from, and the one it leaves.
    state: Option<(u32, u32)>,
}

impl Para {
    fn new(text: String) -> Para {
        Para { text, layout: None, top: 0.0, height: 0.0, dx: 0.0, spans: Vec::new(), state: None }
    }
}

/// A rectangle (x0, y0, x1, y1), device pixels.
pub type DevRect = (f64, f64, f64, f64);

pub struct TextEditor {
    /// Paragraphs (QMEMO, QRICHEDIT); else one line, a pasted line break
    /// dropped.
    multi: bool,
    paras: Vec<Para>,
    anchor: Pos,
    focus: Pos,
    /// Where Up / Down aim across (the x the caret had when they started).
    h_pos: Option<f32>,
    /// An input method's composition: paragraph, byte range of its text.
    compose: Option<(usize, Range<usize>)>,
    look: Look,
    style: Vec<StyleProperty<'static, Ink>>,
    scale: f32,
    /// The view's width (logical): where WordWrap breaks, what Alignment
    /// aligns in.
    width: f64,
    /// Every paragraph laid out and placed.
    laid: bool,
    size: (f64, f64),
    /// The text system's fonts it was laid out with ([`TextSystem::generation`]).
    generation: u64,
}

impl TextEditor {
    pub fn new(multi: bool) -> TextEditor {
        let look = Look::default();
        let style = styles(&look.font, look.color);
        TextEditor {
            multi,
            paras: vec![Para::new(String::new())],
            anchor: Pos::new(0, 0),
            focus: Pos::new(0, 0),
            h_pos: None,
            compose: None,
            look,
            style,
            scale: 1.0,
            width: 0.0,
            laid: false,
            size: (0.0, 0.0),
            generation: 0,
        }
    }

    pub fn multi(&self) -> bool {
        self.multi
    }

    pub fn look(&self) -> &Look {
        &self.look
    }

    fn invalidate(&mut self) {
        for p in &mut self.paras {
            p.layout = None;
        }
        self.laid = false;
    }

    /// Its font, colour, PasswordChar, Alignment, WordWrap.
    pub fn set_look(&mut self, look: Look) {
        if look != self.look {
            if look.font != self.look.font || look.color != self.look.color {
                self.style = styles(&look.font, look.color);
            }
            self.look = look;
            self.invalidate();
        }
    }

    /// The screen's scale (device pixels per logical pixel).
    pub fn set_scale(&mut self, scale: f32) {
        if (scale - self.scale).abs() > f32::EPSILON {
            self.scale = scale;
            self.invalidate();
        }
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// The view's width (logical pixels).
    pub fn set_width(&mut self, width: f64) {
        if (width - self.width).abs() > f64::EPSILON {
            self.width = width;
            if self.look.wrap || self.look.align != Align::Left {
                self.invalidate();
            }
        }
    }

    // ------------------------------------------------------------ text --

    /// The text, paragraphs joined by '\n' (a composition included).
    pub fn text(&self) -> String {
        let mut out = String::new();
        for (i, p) in self.paras.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&p.text);
        }
        out
    }

    pub fn para_count(&self) -> usize {
        self.paras.len()
    }

    pub fn para_text(&self, p: usize) -> &str {
        self.paras.get(p).map_or("", |p| p.text.as_str())
    }

    /// Shows `text` (the caret at its start): the paragraphs it shares with
    /// the text shown, at its start and end, keep their layouts.
    pub fn set_text(&mut self, text: &str) {
        self.compose = None;
        let lines: Vec<&str> = if self.multi { text.split('\n').collect() } else { vec![text] };
        let old = &self.paras;
        let mut head = 0;
        while head < lines.len() && head < old.len() && old[head].text == lines[head] {
            head += 1;
        }
        let mut tail = 0;
        while tail < lines.len() - head && tail < old.len() - head && old[old.len() - 1 - tail].text == lines[lines.len() - 1 - tail] {
            tail += 1;
        }
        let keep_from = old.len() - tail;
        let mut paras: Vec<Para> = Vec::with_capacity(lines.len());
        let mut old: Vec<Option<Para>> = std::mem::take(&mut self.paras).into_iter().map(Some).collect();
        for slot in old.iter_mut().take(head) {
            paras.push(slot.take().expect("kept"));
        }
        for line in &lines[head..lines.len() - tail] {
            paras.push(Para::new(line.to_string()));
        }
        for slot in old.iter_mut().skip(keep_from) {
            paras.push(slot.take().expect("kept"));
        }
        self.paras = paras;
        self.anchor = Pos::new(0, 0);
        self.focus = Pos::new(0, 0);
        self.h_pos = None;
        self.laid = false;
    }

    /// The text shown for paragraph `p` (its PasswordChar's; a single
    /// line's breaks as spaces).
    fn shown(&self, p: usize) -> Cow<'_, str> {
        let t = &self.paras[p].text;
        match self.look.mask {
            Some(m) => Cow::Owned(std::iter::repeat_n(m, t.chars().count()).collect()),
            None if !self.multi && t.contains(['\n', '\r']) => Cow::Owned(t.replace(['\n', '\r'], " ")),
            None => Cow::Borrowed(t),
        }
    }

    fn mask_len(&self) -> Option<usize> {
        self.look.mask.map(char::len_utf8)
    }

    /// A shown byte offset of paragraph `p` in its text.
    fn real_of(&self, p: usize, shown: usize) -> usize {
        match self.mask_len() {
            Some(ml) => byte_of(&self.paras[p].text, shown / ml),
            None => shown.min(self.paras[p].text.len()),
        }
    }

    /// A byte offset of paragraph `p`'s text in its shown text.
    fn shown_of(&self, p: usize, real: usize) -> usize {
        match self.mask_len() {
            Some(ml) => chars_to(&self.paras[p].text, real) * ml,
            None => real,
        }
    }

    fn shown_len(&self, p: usize) -> usize {
        self.shown_of(p, self.paras[p].text.len())
    }

    /// Characters before `pos` in the whole text (a paragraph break counts
    /// one).
    pub fn char_of(&self, pos: Pos) -> usize {
        let mut n = 0;
        for p in 0..pos.para.min(self.paras.len()) {
            n += self.paras[p].text.chars().count() + 1;
        }
        let p = pos.para.min(self.paras.len() - 1);
        n + chars_to(&self.paras[p].text, self.real_of(p, pos.index))
    }

    /// The place `n` characters into the whole text.
    pub fn pos_of_char(&self, mut n: usize) -> Pos {
        for (i, p) in self.paras.iter().enumerate() {
            let len = p.text.chars().count();
            if n <= len || i + 1 == self.paras.len() {
                let real = byte_of(&p.text, n.min(len));
                return Pos::new(i, self.shown_of(i, real));
            }
            n -= len + 1;
        }
        Pos::new(0, 0)
    }

    // ------------------------------------------------------- selection --

    pub fn anchor(&self) -> Pos {
        self.anchor
    }

    pub fn focus(&self) -> Pos {
        self.focus
    }

    /// The selection's ends, in text order.
    pub fn ordered(&self) -> (Pos, Pos) {
        if self.anchor.key() <= self.focus.key() {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    pub fn has_selection(&self) -> bool {
        self.anchor.key() != self.focus.key()
    }

    /// SelStart and SelLength (characters).
    pub fn selection_chars(&self) -> (usize, usize) {
        let (a, b) = self.ordered();
        let (a, b) = (self.char_of(a), self.char_of(b));
        (a, b - a)
    }

    /// Selects `len` characters from `start` (the caret at the end).
    pub fn set_selection_chars(&mut self, start: usize, len: usize) {
        self.anchor = self.pos_of_char(start);
        self.focus = self.pos_of_char(start + len);
        self.h_pos = None;
    }

    /// The selected text (paragraphs joined by '\n').
    pub fn selected_text(&self) -> String {
        let (a, b) = self.ordered();
        let mut out = String::new();
        for p in a.para..=b.para.min(self.paras.len() - 1) {
            let from = if p == a.para { self.real_of(p, a.index) } else { 0 };
            let to = if p == b.para { self.real_of(p, b.index) } else { self.paras[p].text.len() };
            if p > a.para {
                out.push('\n');
            }
            out.push_str(&self.paras[p].text[from..to.max(from)]);
        }
        out
    }

    pub fn select(&mut self, anchor: Pos, focus: Pos) {
        self.anchor = anchor;
        self.focus = focus;
        self.h_pos = None;
    }

    pub fn select_all(&mut self) {
        let last = self.paras.len() - 1;
        self.select(Pos::new(0, 0), Pos::new(last, self.shown_len(last)));
    }

    // ---------------------------------------------------------- layout --

    /// Paragraph `p`'s styled runs (without a syntax: kept until its text
    /// changes; QRICHEDIT's runs, later).
    pub fn set_spans(&mut self, p: usize, spans: Vec<Span>) {
        if let Some(para) = self.paras.get_mut(p) {
            if para.spans != spans {
                para.spans = spans;
                para.layout = None;
                self.laid = false;
            }
        }
    }

    /// Paragraph `p`'s styled runs.
    pub fn spans(&self, p: usize) -> &[Span] {
        self.paras.get(p).map_or(&[], |p| p.spans.as_slice())
    }

    /// The syntax's runs made again for the paragraphs that need them: one
    /// laid out again (its text changed), or one whose starting state
    /// changed (a construct across lines above it opened or closed).
    fn colour(&mut self) {
        if self.look.syntax == Syntax::None {
            return;
        }
        let mut state = 0;
        for p in &mut self.paras {
            if p.layout.is_none() || p.state.is_none_or(|(from, _)| from != state) {
                let (tokens, out) = rapidr_value::objects::code::spans(self.look.syntax, &p.text, state);
                p.spans = tokens.into_iter().map(|(range, t)| Span { range, style: RunStyle::of_token(t) }).collect();
                p.state = Some((state, out));
                p.layout = None;
            }
            state = p.state.map_or(0, |(_, out)| out);
        }
    }

    /// Lays out what changed and places the paragraphs.
    pub fn lay_out(&mut self, ts: &mut TextSystem) {
        // (a font came: everything again, with the fallbacks there are now)
        if self.generation != ts.generation {
            self.generation = ts.generation;
            self.style = styles(&self.look.font, self.look.color);
            for p in &mut self.paras {
                p.layout = None;
            }
            self.laid = false;
        }
        if self.laid {
            return;
        }
        self.colour();
        let scale = f64::from(self.scale);
        let fallback = f64::from(self.look.font.pixel_size() as f32) * 1.15 * scale;
        let view = self.width * scale;
        let (mut top, mut wide) = (0.0, 0.0f64);
        for i in 0..self.paras.len() {
            if self.paras[i].layout.is_none() {
                let shown = self.shown(i).into_owned();
                let mut b = ts.layout_cx.ranged_builder(&mut ts.font_cx, &shown, self.scale, true);
                for prop in &self.style {
                    b.push_default(prop.clone());
                }
                // (its runs: over its own text, so not over a mask's)
                if self.look.mask.is_none() {
                    for span in &self.paras[i].spans {
                        let r = span.range.start.min(shown.len())..span.range.end.min(shown.len());
                        if r.is_empty() || !shown.is_char_boundary(r.start) || !shown.is_char_boundary(r.end) {
                            continue;
                        }
                        for prop in span.style.props() {
                            b.push(prop, r.clone());
                        }
                    }
                }
                let mut layout = b.build(&shown);
                if self.look.wrap {
                    layout.break_all_lines(Some(view.max(1.0) as f32));
                    let a = match self.look.align {
                        Align::Left => Alignment::Left,
                        Align::Right => Alignment::Right,
                        Align::Center => Alignment::Center,
                    };
                    layout.align(a, AlignmentOptions::default());
                } else {
                    layout.break_all_lines(None);
                    layout.align(Alignment::Left, AlignmentOptions::default());
                }
                super::note_missing(&layout, &shown);
                self.paras[i].layout = Some(layout);
            }
            let p = &mut self.paras[i];
            let l = p.layout.as_ref().expect("laid out above");
            let w = f64::from(l.full_width());
            p.dx = if self.look.wrap { 0.0 } else { ((view - w) * self.look.align.factor()).max(0.0).round() };
            p.height = match f64::from(l.height()) {
                h if h > 0.0 => h,
                _ => fallback,
            };
            p.top = top;
            top += p.height;
            wide = wide.max(w + p.dx);
        }
        self.size = (wide, top);
        self.laid = true;
    }

    fn layout(&self, p: usize) -> &Layout<Ink> {
        self.paras[p].layout.as_ref().expect("lay_out first")
    }

    /// Paragraph `p`'s layout (what the host draws).
    pub fn para_layout(&self, p: usize) -> Option<&Layout<Ink>> {
        self.paras.get(p)?.layout.as_ref()
    }

    /// Where paragraph `p`'s layout's (0, 0) is in the text.
    pub fn para_origin(&self, p: usize) -> (f64, f64) {
        self.paras.get(p).map_or((0.0, 0.0), |p| (p.dx, p.top))
    }

    /// How wide (the longest line) and high the text is laid out.
    pub fn content_size(&self) -> (f64, f64) {
        self.size
    }

    /// A line's height (the first paragraph's first line).
    pub fn line_height(&self) -> f64 {
        let fallback = f64::from(self.look.font.pixel_size() as f32) * 1.15 * f64::from(self.scale);
        self.paras.first().and_then(|p| p.layout.as_ref()).and_then(|l| l.lines().next()).map_or(fallback, |l| f64::from(l.metrics().line_height).max(1.0))
    }

    /// The paragraphs showing in `top .. bottom` (down the text).
    pub fn visible(&self, top: f64, bottom: f64) -> Range<usize> {
        let first = self.paras.partition_point(|p| p.top + p.height <= top);
        let end = self.paras.partition_point(|p| p.top < bottom);
        first..end.max(first)
    }

    fn cursor(&self, pos: Pos) -> Cursor {
        Cursor::from_byte_index(self.layout(pos.para), pos.index, pos.affinity)
    }

    fn pos(&self, para: usize, c: Cursor) -> Pos {
        Pos { para, index: c.index(), affinity: c.affinity() }
    }

    /// The caret's rectangle, `width` wide, in the text.
    pub fn caret_rect(&self, width: f32) -> DevRect {
        let p = self.focus.para;
        let b = self.cursor(self.focus).geometry(self.layout(p), width);
        let (dx, top) = self.para_origin(p);
        (b.x0 + dx, b.y0 + top, b.x1 + dx, b.y1 + top)
    }

    /// The selection's rectangles in paragraph `p` (its layout's pixels).
    pub fn selection_rects(&self, p: usize) -> Vec<DevRect> {
        let (a, b) = self.ordered();
        if !self.has_selection() || p < a.para || p > b.para {
            return Vec::new();
        }
        let from = if p == a.para { a.index } else { 0 };
        let to = if p == b.para { b.index } else { self.shown_len(p) };
        if from >= to {
            return Vec::new();
        }
        let l = self.layout(p);
        let sel = Selection::new(Cursor::from_byte_index(l, from, Affinity::Downstream), Cursor::from_byte_index(l, to, Affinity::Upstream));
        sel.geometry(l).into_iter().map(|(b, _)| (b.x0, b.y0, b.x1, b.y1)).collect()
    }

    /// The composition's underlines in paragraph `p` (its layout's pixels;
    /// `thick` device pixels).
    pub fn compose_rects(&self, p: usize, thick: f64) -> Vec<DevRect> {
        let Some((cp, range)) = self.compose.clone() else { return Vec::new() };
        if cp != p || range.is_empty() {
            return Vec::new();
        }
        let l = self.layout(p);
        let (a, b) = (self.shown_of(p, range.start), self.shown_of(p, range.end));
        let sel = Selection::new(Cursor::from_byte_index(l, a, Affinity::Downstream), Cursor::from_byte_index(l, b, Affinity::Upstream));
        sel.geometry(l).into_iter().map(|(b, _)| (b.x0, (b.y1 - thick).round(), b.x1, b.y1.round())).collect()
    }

    /// The place at (x, y) of the text.
    pub fn pos_at(&self, x: f64, y: f64) -> Pos {
        let p = self.paras.partition_point(|p| p.top + p.height <= y).min(self.paras.len() - 1);
        let (dx, top) = self.para_origin(p);
        self.pos(p, Cursor::from_point(self.layout(p), (x - dx) as f32, (y - top) as f32))
    }

    // ----------------------------------------------------------- moves --

    fn set_focus(&mut self, to: Pos, extend: bool) {
        self.focus = to;
        if !extend {
            self.anchor = to;
        }
    }

    /// Left / Right (`word`: Ctrl+): a character (word) back or on, into the
    /// paragraph before or after at its ends. Without Shift a selection
    /// collapses to its start / end.
    pub fn move_h(&mut self, forward: bool, word: bool, extend: bool) {
        self.h_pos = None;
        if !extend && !word && self.has_selection() {
            let (a, b) = self.ordered();
            self.set_focus(if forward { b } else { a }, false);
            return;
        }
        let f = self.focus;
        let l = self.layout(f.para);
        let c = self.cursor(f);
        let n = match (forward, word) {
            (true, false) => c.next_visual(l),
            (false, false) => c.previous_visual(l),
            (true, true) => c.next_visual_word(l),
            (false, true) => c.previous_visual_word(l),
        };
        let to = if n.index() != c.index() || n.affinity() != c.affinity() {
            self.pos(f.para, n)
        } else if forward && f.para + 1 < self.paras.len() {
            Pos::new(f.para + 1, 0)
        } else if !forward && f.para > 0 {
            Pos::new(f.para - 1, self.shown_len(f.para - 1))
        } else {
            f
        };
        self.set_focus(to, extend);
    }

    /// The line (index, its top and bottom) of paragraph `p` at y.
    fn line_at(&self, p: usize, y: f64) -> (usize, f64, f64) {
        let l = self.layout(p);
        let mut last = (0, 0.0, f64::from(l.height()));
        for (i, line) in l.lines().enumerate() {
            let m = line.metrics();
            last = (i, f64::from(m.block_min_coord), f64::from(m.block_max_coord));
            if y < f64::from(m.block_max_coord) {
                break;
            }
        }
        last
    }

    /// Up / Down `lines` lines (negative: up), across paragraphs, keeping
    /// to the x the caret had; stays put on the first / last line.
    pub fn move_v(&mut self, lines: isize, extend: bool) {
        let f = self.focus;
        let g = self.cursor(f).geometry(self.layout(f.para), 0.0);
        let (dx, _) = self.para_origin(f.para);
        let x = self.h_pos.map_or(g.x0 + dx, f64::from);
        let (mut p, (mut line, _, _)) = (f.para, self.line_at(f.para, (g.y0 + g.y1) / 2.0));
        let mut moved = false;
        for _ in 0..lines.unsigned_abs() {
            if lines > 0 {
                if line + 1 < self.layout(p).len() {
                    line += 1;
                } else if p + 1 < self.paras.len() {
                    p += 1;
                    line = 0;
                } else {
                    break;
                }
            } else if line > 0 {
                line -= 1;
            } else if p > 0 {
                p -= 1;
                line = self.layout(p).len().saturating_sub(1);
            } else {
                break;
            }
            moved = true;
        }
        if !moved {
            return;
        }
        let l = self.layout(p);
        let y = l.get(line).map_or(0.0, |ln| {
            let m = ln.metrics();
            (m.block_min_coord + m.block_max_coord) / 2.0
        });
        let (pdx, _) = self.para_origin(p);
        let c = Cursor::from_point(l, (x - pdx) as f32, y);
        let to = self.pos(p, c);
        self.set_focus(to, extend);
        self.h_pos = Some(x as f32);
    }

    /// Home / End: the line's start or end (`doc`: Ctrl+, the text's).
    pub fn move_edge(&mut self, end: bool, doc: bool, extend: bool) {
        self.h_pos = None;
        let to = if doc {
            let last = self.paras.len() - 1;
            if end { Pos::new(last, self.shown_len(last)) } else { Pos::new(0, 0) }
        } else {
            let f = self.focus;
            let l = self.layout(f.para);
            let s = Selection::from(self.cursor(f));
            let s = if end { s.line_end(l, false) } else { s.line_start(l, false) };
            self.pos(f.para, s.focus())
        };
        self.set_focus(to, extend);
    }

    /// The caret to (x, y) of the text (`extend`: Shift+click, a drag).
    pub fn click(&mut self, x: f64, y: f64, extend: bool) {
        let to = self.pos_at(x, y);
        self.h_pos = None;
        self.set_focus(to, extend);
    }

    /// The word at (x, y) selected (a double click).
    pub fn select_word_at(&mut self, x: f64, y: f64) {
        let at = self.pos_at(x, y);
        let (dx, top) = self.para_origin(at.para);
        let l = self.layout(at.para);
        let s = Selection::word_from_point(l, (x - dx) as f32, (y - top) as f32);
        self.select(self.pos(at.para, s.anchor()), self.pos(at.para, s.focus()));
    }

    /// The paragraph at (x, y) selected (a triple click in a memo).
    pub fn select_para_at(&mut self, x: f64, y: f64) {
        let p = self.pos_at(x, y).para;
        self.select(Pos::new(p, 0), Pos::new(p, self.shown_len(p)));
    }

    // ----------------------------------------------------------- edits --

    /// Replaces `from .. to` with `s` (paragraph breaks '\n' in a memo; a
    /// single line takes none): the caret after it.
    fn replace(&mut self, ts: &mut TextSystem, from: Pos, to: Pos, s: &str) {
        let (from, to) = if from.key() <= to.key() { (from, to) } else { (to, from) };
        let a = self.real_of(from.para, from.index);
        let b = self.real_of(to.para, to.index);
        let mut joined = String::with_capacity(a + s.len() + 16);
        joined.push_str(&self.paras[from.para].text[..a]);
        let s: Cow<str> = if self.multi { Cow::Borrowed(s) } else { Cow::Owned(s.replace(['\n', '\r'], "")) };
        joined.push_str(&s);
        let tail = self.paras[to.para].text[b..].to_string();
        let lines: Vec<String> = if self.multi { joined.split('\n').map(str::to_string).collect() } else { vec![joined] };
        let caret_para = from.para + lines.len() - 1;
        let caret_byte = lines.last().map_or(0, String::len);
        let mut new: Vec<Para> = lines.into_iter().map(Para::new).collect();
        if let Some(last) = new.last_mut() {
            last.text.push_str(&tail);
        }
        self.paras.splice(from.para..=to.para, new);
        self.laid = false;
        self.lay_out(ts);
        let at = Pos::new(caret_para, self.shown_of(caret_para, caret_byte));
        self.anchor = at;
        self.focus = at;
        self.h_pos = None;
    }

    /// The selection replaced by `s` (typing, pasting).
    pub fn replace_selection(&mut self, ts: &mut TextSystem, s: &str) {
        let (a, b) = self.ordered();
        self.replace(ts, a, b, s);
    }

    /// Backspace (`word`: Ctrl+): the selection, or the character (word)
    /// before the caret — joining the paragraph to the one before at its
    /// start. Whether the text changed.
    pub fn delete_back(&mut self, ts: &mut TextSystem, word: bool) -> bool {
        if self.has_selection() {
            self.replace_selection(ts, "");
            return true;
        }
        let f = self.focus;
        if f.index == 0 {
            if f.para == 0 {
                return false;
            }
            let prev = Pos::new(f.para - 1, self.shown_len(f.para - 1));
            self.replace(ts, prev, f, "");
            return true;
        }
        let l = self.layout(f.para);
        let c = self.cursor(f);
        let start = if word {
            c.previous_logical_word(l).index()
        } else {
            match c.logical_clusters(l)[0].as_ref() {
                Some(cl) if cl.is_emoji() => cl.text_range().start,
                _ => {
                    let shown = self.shown(f.para);
                    shown[..f.index.min(shown.len())].char_indices().next_back().map_or(0, |(i, _)| i)
                }
            }
        };
        if start >= f.index {
            return false;
        }
        self.replace(ts, Pos::new(f.para, start), f, "");
        true
    }

    /// Delete (`word`: Ctrl+): the selection, or the character (word)
    /// after the caret — joining the next paragraph at its end.
    pub fn delete_forward(&mut self, ts: &mut TextSystem, word: bool) -> bool {
        if self.has_selection() {
            self.replace_selection(ts, "");
            return true;
        }
        let f = self.focus;
        let len = self.shown_len(f.para);
        if f.index >= len {
            if f.para + 1 >= self.paras.len() {
                return false;
            }
            self.replace(ts, f, Pos::new(f.para + 1, 0), "");
            return true;
        }
        let l = self.layout(f.para);
        let c = self.cursor(f);
        let end = if word {
            c.next_logical_word(l).index()
        } else {
            c.logical_clusters(l)[1].as_ref().map_or(f.index, |cl| cl.text_range().end)
        };
        if end <= f.index {
            return false;
        }
        self.replace(ts, f, Pos::new(f.para, end.min(len)), "");
        true
    }

    // --------------------------------------------- input methods (IME) --

    pub fn composing(&self) -> bool {
        self.compose.is_some()
    }

    /// The composition is now `text` (its caret `cursor`, a byte range in
    /// it): shown at the caret, not the program's text until committed.
    pub fn set_compose(&mut self, ts: &mut TextSystem, text: &str, cursor: Option<(usize, usize)>) {
        let text = text.replace(['\n', '\r'], "");
        let (p, at) = match self.compose.take() {
            Some((p, r)) => {
                self.paras[p].text.replace_range(r.clone(), "");
                (p, r.start)
            }
            None => {
                if self.has_selection() {
                    self.replace_selection(ts, "");
                }
                (self.focus.para, self.real_of(self.focus.para, self.focus.index))
            }
        };
        self.paras[p].text.insert_str(at, &text);
        self.paras[p].layout = None;
        self.laid = false;
        self.lay_out(ts);
        self.compose = Some((p, at..at + text.len()));
        let (c0, c1) = cursor.map_or((text.len(), text.len()), |(a, b)| (a.min(text.len()), b.min(text.len())));
        self.anchor = Pos::new(p, self.shown_of(p, at + c0));
        self.focus = Pos::new(p, self.shown_of(p, at + c1));
    }

    /// The composition removed (the caret where it started).
    pub fn clear_compose(&mut self, ts: &mut TextSystem) {
        if let Some((p, r)) = self.compose.take() {
            self.paras[p].text.replace_range(r.clone(), "");
            self.paras[p].layout = None;
            self.laid = false;
            self.lay_out(ts);
            let at = Pos::new(p, self.shown_of(p, r.start));
            self.anchor = at;
            self.focus = at;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(multi: bool, text: &str) -> (TextEditor, TextSystem) {
        let mut ts = TextSystem::new();
        let mut e = TextEditor::new(multi);
        e.set_width(200.0);
        e.set_text(text);
        e.lay_out(&mut ts);
        (e, ts)
    }

    #[test]
    fn paragraphs_positions_and_selection_in_characters() {
        let (mut e, mut ts) = editor(true, "one\ntwo\nthree");
        assert_eq!(e.para_count(), 3);
        assert_eq!(e.char_of(Pos::new(1, 1)), 5);
        assert_eq!(e.pos_of_char(8), Pos::new(2, 0));
        e.set_selection_chars(2, 4);
        assert_eq!(e.selected_text(), "e\ntw");
        assert_eq!(e.selection_chars(), (2, 4));
        // typing over a selection across paragraphs joins them
        e.replace_selection(&mut ts, "X");
        assert_eq!(e.text(), "onXo\nthree");
        assert_eq!(e.selection_chars(), (3, 0));
        // a paste with breaks makes paragraphs
        e.replace_selection(&mut ts, "1\n2");
        assert_eq!(e.text(), "onX1\n2o\nthree");
        assert_eq!((e.focus().para, e.focus().index), (1, 1));
    }

    #[test]
    fn moves_cross_paragraphs_and_keep_the_column() {
        let (mut e, _ts) = editor(true, "abcdef\nab\nabcdef");
        let at = |e: &TextEditor| (e.focus().para, e.focus().index);
        e.set_selection_chars(5, 0);
        e.move_v(1, false);
        assert_eq!(at(&e), (1, 2), "the short line's end");
        e.move_v(1, false);
        assert_eq!(at(&e), (2, 5), "back in the column it started in");
        e.move_v(1, false);
        assert_eq!(e.focus().para, 2, "the last line: stays");
        e.move_edge(false, false, false);
        e.move_h(false, false, false);
        assert_eq!(at(&e), (1, 2), "Left at a paragraph's start: the one before's end");
        e.move_h(true, false, true);
        assert_eq!(e.selected_text(), "\n");
        e.move_h(false, false, false);
        assert_eq!(at(&e), (1, 2), "Left collapses a selection to its start");
        e.move_edge(true, true, false);
        assert_eq!(e.char_of(e.focus()), 16);
    }

    #[test]
    fn deleting_joins_paragraphs_and_reuses_layouts() {
        let (mut e, mut ts) = editor(true, "ab\ncd");
        e.set_selection_chars(3, 0);
        assert!(e.delete_back(&mut ts, false));
        assert_eq!(e.text(), "abcd");
        assert!(e.delete_forward(&mut ts, false));
        assert_eq!(e.text(), "abd");
        assert!(!{
            e.move_edge(true, true, false);
            e.delete_forward(&mut ts, false)
        });
        // the program's new text keeps the layouts of unchanged paragraphs
        e.set_text("x\ny\nz");
        e.lay_out(&mut ts);
        e.set_text("x\ny\nz\nw");
        assert!((0..3).all(|p| e.para_layout(p).is_some()) && e.para_layout(3).is_none(), "only the new paragraph to lay out");
        e.set_text("x\nY\nz\nw");
        assert!(e.para_layout(1).is_none() && e.para_layout(2).is_some());
        e.lay_out(&mut ts);
        assert_eq!(e.visible(0.0, 1.0), 0..1);
        let (_, h) = e.content_size();
        assert_eq!(e.visible(0.0, h + 10.0), 0..4);
    }

    #[test]
    fn a_password_shows_its_character_and_keeps_the_text() {
        let (mut e, mut ts) = editor(false, "secret");
        e.set_look(Look { mask: Some('*'), ..Look::default() });
        e.lay_out(&mut ts);
        assert_eq!(e.shown(0), "******");
        e.set_selection_chars(3, 0);
        e.move_h(true, false, true);
        assert_eq!(e.selected_text(), "r");
        e.replace_selection(&mut ts, "R");
        assert_eq!(e.text(), "secRet");
        assert!(e.delete_back(&mut ts, false));
        assert_eq!(e.text(), "secet");
    }

    #[test]
    fn composition_is_shown_then_committed_or_dropped() {
        let (mut e, mut ts) = editor(false, "ab");
        e.set_selection_chars(1, 0);
        e.set_compose(&mut ts, "k", Some((1, 1)));
        e.set_compose(&mut ts, "ka", Some((2, 2)));
        assert_eq!(e.text(), "akab");
        assert_eq!(e.compose_rects(0, 1.0).len(), 1);
        e.clear_compose(&mut ts);
        assert_eq!(e.text(), "ab");
        assert_eq!(e.selection_chars(), (1, 0));
    }

    /// What a paragraph's layout draws: (glyphs, colour), its glyph runs
    /// merged by colour.
    fn inks(e: &TextEditor, p: usize) -> Vec<(usize, u32)> {
        let mut out: Vec<(usize, u32)> = Vec::new();
        for line in e.para_layout(p).unwrap().lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::GlyphRun(g) = item {
                    let (n, ink) = (g.glyphs().count(), g.style().brush.0);
                    match out.last_mut() {
                        Some((m, c)) if *c == ink => *m += n,
                        _ => out.push((n, ink)),
                    }
                }
            }
        }
        out
    }

    #[test]
    fn a_syntax_colours_only_the_paragraphs_an_edit_touched() {
        let (mut e, mut ts) = editor(true, "DIM a\nPRINT 1 ' c\nx = 2");
        e.set_look(Look { syntax: Syntax::Basic, ..Look::default() });
        e.lay_out(&mut ts);
        assert_eq!(inks(&e, 0), [(3, 0x0000B4), (2, 0)], "DIM, then \" a\"");
        assert_eq!(inks(&e, 1).iter().map(|(_, c)| *c).collect::<Vec<_>>(), [0x0000B4, 0, 0x800000, 0, 0x008000]);
        assert!(e.spans(1)[0].style.bold == Some(true) && e.spans(1)[2].style.italic == Some(true), "keywords bold, comments italic");
        // typing in the last paragraph lays out (and colours) only it
        e.set_selection_chars(e.text().chars().count(), 0);
        e.replace_selection(&mut ts, "0");
        assert!(e.para_layout(0).is_some() && e.para_layout(1).is_some());
        assert_eq!(e.spans(2).len(), 1, "x = 20: one number");
        assert_eq!(e.spans(2)[0].range, 4..6);
        // given runs, without a syntax (QRICHEDIT's, later)
        let (mut r, mut ts) = editor(true, "plain bold");
        r.set_spans(0, vec![Span { range: 6..10, style: RunStyle { color: Some(0xFF0000), underline: Some(true), ..RunStyle::default() } }]);
        r.lay_out(&mut ts);
        assert_eq!(inks(&r, 0), [(6, 0), (4, 0xFF0000)]);
    }

    #[test]
    fn word_wrap_breaks_at_the_view_and_up_down_walk_its_lines() {
        let (mut e, mut ts) = editor(true, "alpha beta gamma delta epsilon zeta eta theta");
        e.set_look(Look { wrap: true, ..Look::default() });
        e.set_width(60.0);
        e.lay_out(&mut ts);
        let lines = e.para_layout(0).unwrap().len();
        assert!(lines > 2, "{lines} lines");
        e.move_v(1, false);
        assert_eq!(e.focus().para, 0);
        assert!(e.focus().index > 0);
        // right alignment without wrap: the line sits at the right
        e.set_look(Look { align: Align::Right, ..Look::default() });
        e.set_text("ab");
        e.lay_out(&mut ts);
        assert!(e.para_origin(0).0 > 30.0, "{:?}", e.para_origin(0));
    }
}
