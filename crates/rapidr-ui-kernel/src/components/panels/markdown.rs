//! RMARKDOWNVIEW drawn and driven by the kernel; its model (the text read
//! into paragraphs, the selection) is `rapidr_value::panels::markdown`.
//! The web draws the same.
//!
//! Each paragraph is one parley layout, shaped once per text, theme, font
//! and scale (with its runs: bold, italic, struck out, inline code in
//! JetBrains Mono, links underlined in the theme's link colour, a BASIC
//! code block coloured as the code editor colours it), then flowed at the
//! view's width: headings larger (the first two levels underlined by a
//! line), list items indented with their marker (its own small layout)
//! at their left, block quotes indented with a bar, code blocks on a
//! tinted ground, tables as a grid (columns as wide as their cells want,
//! narrowed to fit; the header row tinted), rules as lines. The hosts draw
//! the layouts as they draw an editor's ([`crate::TextItem`]), so the
//! selection's highlight comes free.
//!
//! The mouse: a link pressed and let go on is OnLinkClick (a web link opens
//! in the browser, a `#heading` scrolls there: the model); elsewhere a
//! press and a drag select text (Shift extends). The wheel and the bar
//! scroll. The keys: Up / Down, Page Up / Page Down, Space, Home / End
//! scroll; Ctrl+A selects everything; Ctrl+C / Cmd+C copies the selection.

use std::rc::Rc;

use parley::{Affinity, Alignment, AlignmentOptions, Cursor, FontStyle, FontWeight, Layout, LineHeight, Selection, StyleProperty};
use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_CELL, PART_ITEM, PART_ROW};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Place, Rect};
use rapidr_value::panels::markdown::{self as model, Doc, Kind, Para, Style};
use rapidr_value::panels::rows::vk;
use rapidr_value::panels::User;
use rapidr_value::theme::Theme;

use super::common::{self, look, Look};
use crate::a11y::AccessValue;
use crate::components::list::{bar_mouse, bar_tick, vscroll_at, vscroll_state, vscroll_wheel};
use crate::components::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::display::TextItem;
use crate::input::{Clipboard, KernelEvent};
use crate::paint::Painter;
use crate::store;
use crate::text::{note_missing, styles, Ink, TextSystem};

pub struct MarkdownView;

/// The text's margins (logical pixels).
const PAD_X: f64 = 18.0;
const PAD_Y: f64 = 14.0;
/// A list level's indent, a block quote's.
const LIST_INDENT: f64 = 24.0;
const QUOTE_INDENT: f64 = 18.0;
/// A code block's padding; a table cell's across and down.
const CODE_PAD: f64 = 10.0;
const CELL_PAD_X: f64 = 10.0;
const CELL_PAD_Y: f64 = 6.0;
/// The body text's size when the program sets no font.
const BODY_PX: i64 = 13;
/// How many paragraphs around the view a screen reader is told about.
const A11Y_AROUND: usize = 300;
/// The accessibility parts' indexes: links after the paragraphs.
const A11Y_LINK: usize = 1 << 24;

/// What the view draws besides the text (logical, from the text's top
/// left).
#[derive(Clone, Debug, PartialEq)]
enum Deco {
    /// A tinted ground (a code block, inline code, a table's header).
    Ground { x: f64, y: f64, w: f64, h: f64, radius: f64, color: u32 },
    /// A line (a rule, a heading's underline, a table's grid, a quote's bar).
    Bar { x: f64, y: f64, w: f64, h: f64, color: u32 },
}

/// A paragraph's (or a marker's) layout, shaped.
struct Shaped {
    layout: Layout<Ink>,
    /// The paragraph it is (a marker's: its item's first paragraph).
    para: usize,
}

/// Where a layout sits (logical, from the text's top left), and the
/// rectangle it's clipped to (a table's cell).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Spot {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    clip: Option<(f64, f64, f64, f64)>,
}

/// An RMARKDOWNVIEW laid out: kept in its node's UI state
/// ([`crate::tree::NodeUi::markdown`]).
pub struct Laid {
    /// What it was shaped for: the model's text, the scale, the theme, the
    /// font, the text system's fonts.
    shaped_for: (u64, u64, usize, String, i64, u64),
    /// The width it was flowed at (logical).
    flowed_at: f64,
    doc: Rc<Doc>,
    /// The paragraphs' layouts (one each, in order), then the list markers'.
    boxes: Vec<Shaped>,
    spots: Vec<Spot>,
    decos: Vec<Deco>,
    /// The text's height (logical), margins included.
    height: f64,
    /// The scroll position asked for (a key, a heading to show).
    want: Option<i64>,
}

impl Laid {
    /// Layout `i` (a paragraph's, then the markers').
    pub fn layout(&self, i: usize) -> Option<&Layout<Ink>> {
        self.boxes.get(i).map(|b| &b.layout)
    }
}

/// The fonts and colours a text is shaped with.
struct Faces {
    body: Font,
    mono: Font,
    l: Look,
    theme: &'static Theme,
}

fn faces(cx: &Cx, theme: &'static Theme) -> Faces {
    let program_font = !store::string(cx.store, cx.id, "fontname").trim().is_empty();
    let body = if program_font { Font { color: 0, styles: 0, ..cx.font.clone() } } else { Font { name: String::new(), size: -BODY_PX, color: 0, styles: 0 } };
    let px = body.pixel_size().max(6);
    let mono = Font { name: "JetBrains Mono".into(), size: -((px as f64 * 0.92).round() as i64).max(6), color: 0, styles: 0 };
    Faces { body, mono, l: look(theme), theme }
}

/// A font `ratio` times the body's, bold or not.
fn scaled(f: &Font, ratio: f64, bold: bool) -> Font {
    let px = (f.pixel_size() as f64 * ratio).round() as i64;
    Font { size: -px.max(6), styles: if bold { f.styles | 1 } else { f.styles }, ..f.clone() }
}

/// A heading's size against the body's.
fn heading_ratio(level: u8) -> f64 {
    match level {
        1 => 1.9,
        2 => 1.5,
        3 => 1.25,
        4 => 1.08,
        _ => 1.0,
    }
}

/// Whether a code block's language is BASIC (coloured as the code editor
/// colours it).
fn is_basic(lang: &str) -> bool {
    matches!(lang, "basic" | "bas" | "rapidq" | "rapidr" | "rr" | "rq" | "rqw" | "rqb" | "inc" | "qbasic" | "freebasic" | "vb" | "vbnet" | "vb6")
}

/// A run's parley styles over the paragraph's.
fn run_props(st: &Style, f: &Faces, base: &Font) -> Vec<StyleProperty<'static, Ink>> {
    let mut out = Vec::new();
    if st.bold {
        let face = crate::text::family(&base.name);
        out.push(StyleProperty::FontWeight(if face == "Inter" { FontWeight::SEMI_BOLD } else { FontWeight::BOLD }));
    }
    if st.italic {
        out.push(StyleProperty::FontStyle(FontStyle::Italic));
    }
    if st.strike {
        out.push(StyleProperty::Strikethrough(true));
    }
    if st.code {
        let mono = Font { size: -((base.pixel_size() as f64 * 0.9).round() as i64).max(6), ..f.mono.clone() };
        out.extend(styles(&mono, 0).into_iter().filter(|p| matches!(p, StyleProperty::FontFamily(_) | StyleProperty::FontSize(_) | StyleProperty::FontFeatures(_))));
    }
    if st.link.is_some() {
        out.push(StyleProperty::Brush(Ink(f.l.link)));
        out.push(StyleProperty::Underline(true));
    }
    out
}

/// Paragraph `p` shaped (not yet broken into lines).
fn shape(ts: &mut TextSystem, p: &Para, f: &Faces, scale: f32) -> Layout<Ink> {
    let (font, color, line) = match &p.kind {
        Kind::Heading(lv) => (scaled(&f.body, heading_ratio(*lv), true), if *lv >= 6 { f.l.dim } else { f.l.text }, 1.3),
        Kind::Code(_) => (f.mono.clone(), f.l.text, 1.45),
        Kind::Cell { row: 0, .. } => (scaled(&f.body, 1.0, true), f.l.text, 1.45),
        _ => (f.body.clone(), if p.quote > 0 { f.l.dim } else { f.l.text }, 1.5),
    };
    let text = if p.kind == Kind::Rule { "" } else { p.text.as_str() };
    let mut b = ts.layout_cx.ranged_builder(&mut ts.font_cx, text, scale, true);
    for prop in styles(&font, color) {
        b.push_default(prop);
    }
    b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(line)));
    // (a long word or path breaks where it must, rather than run out)
    b.push_default(StyleProperty::OverflowWrap(parley::OverflowWrap::Anywhere));
    match &p.kind {
        Kind::Code(lang) if is_basic(lang) => {
            // (RapidR BASIC's colours as the code editor gives them: its
            // language definition and the theme's scheme — S-EDITOR's one
            // highlighter, as the editor's popups use it)
            let languages = rapidr_editor::lang::Languages::builtin();
            let mut hl = rapidr_editor::Highlighter::new(languages.get("rapidr-basic").unwrap_or_else(|| languages.plain_text()));
            let scheme = rapidr_value::code_scheme::for_theme(f.theme);
            let mut state = rapidr_editor::ROOT;
            let mut at = 0;
            for line in text.split('\n') {
                let (tokens, next) = hl.line_spans(line, state);
                state = next;
                for t in tokens {
                    let st = scheme.token(t.kind.fallbacks());
                    let range = at + t.start as usize..at + t.end as usize;
                    b.push(StyleProperty::Brush(Ink(st.color)), range.clone());
                    if st.bold {
                        b.push(StyleProperty::FontWeight(FontWeight::BOLD), range.clone());
                    }
                    if st.italic {
                        b.push(StyleProperty::FontStyle(FontStyle::Italic), range);
                    }
                }
                at += line.len() + 1;
            }
        }
        Kind::Code(_) => {}
        _ => {
            for (r, st) in &p.runs {
                if r.end <= text.len() && text.is_char_boundary(r.start) && text.is_char_boundary(r.end) {
                    for prop in run_props(st, f, &font) {
                        b.push(prop, r.clone());
                    }
                }
            }
        }
    }
    let layout = b.build(text);
    note_missing(&layout, text);
    layout
}

/// A list marker shaped: in the body's font, dimmed.
fn shape_marker(ts: &mut TextSystem, text: &str, f: &Faces, scale: f32) -> Layout<Ink> {
    let mut b = ts.layout_cx.ranged_builder(&mut ts.font_cx, text, scale, true);
    for prop in styles(&f.body, f.l.dim) {
        b.push_default(prop);
    }
    b.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(1.5)));
    let mut layout = b.build(text);
    layout.break_all_lines(None);
    layout.align(Alignment::Left, AlignmentOptions::default());
    layout
}

/// Breaks `l` at `width` logical pixels (`None`: one line), aligned:
/// its height and the width it takes (logical).
fn flow(l: &mut Layout<Ink>, width: Option<f64>, align: Alignment, scale: f64) -> (f64, f64) {
    l.break_all_lines(width.map(|w| (w.max(1.0) * scale) as f32));
    l.align(align, AlignmentOptions::default());
    (f64::from(l.height()) / scale, f64::from(l.full_width()) / scale)
}

impl Laid {
    /// Shapes the model's text for `cx` (when the text, the theme, the
    /// font or the scale changed).
    fn shape(cx: &mut Cx, theme: &'static Theme) {
        let f = faces(cx, theme);
        let (rev, doc) = model::with_mut(cx.id, |m| (m.rev, m.doc()));
        let key = (rev, cx.scale.to_bits(), theme as *const Theme as usize, f.body.name.clone(), f.body.size, cx.text.generation);
        if cx.ui.markdown.as_ref().is_some_and(|m| m.shaped_for == key) {
            return;
        }
        let scale = cx.scale as f32;
        let mut boxes: Vec<Shaped> = doc.paras.iter().enumerate().map(|(i, p)| Shaped { layout: shape(cx.text, p, &f, scale), para: i }).collect();
        for (i, p) in doc.paras.iter().enumerate() {
            if let Some(m) = &p.marker {
                boxes.push(Shaped { layout: shape_marker(cx.text, m, &f, scale), para: i });
            }
        }
        let want = cx.ui.markdown.as_ref().and_then(|m| m.want);
        cx.ui.markdown = Some(Box::new(Laid { shaped_for: key, flowed_at: f64::NAN, doc, boxes, spots: Vec::new(), decos: Vec::new(), height: 0.0, want }));
    }

    /// Places everything at `width` (logical: the view's).
    fn flow(&mut self, width: f64, scale: f64, f: &Faces) {
        if self.flowed_at == width {
            return;
        }
        self.flowed_at = width;
        let l = &f.l;
        let n = self.doc.paras.len();
        let mut spots = vec![Spot::default(); self.boxes.len()];
        let mut decos = Vec::new();
        let code_ground = common::mix(l.body, l.text, if l.dark { 0.10 } else { 0.05 });
        let inline_ground = common::mix(l.body, l.text, if l.dark { 0.14 } else { 0.07 });
        let body_px = f.body.pixel_size() as f64;
        let text_w = (width - 2.0 * PAD_X).max(40.0);
        let mut y = PAD_Y;
        let mut i = 0;
        // (the markers' layouts, by their paragraph)
        let markers: std::collections::HashMap<usize, usize> = (n..self.boxes.len()).map(|k| (self.boxes[k].para, k)).collect();
        while i < n {
            let p = self.doc.paras[i].clone();
            let indent = p.quote as f64 * QUOTE_INDENT + p.depth as f64 * LIST_INDENT;
            let x = PAD_X + indent;
            let w = (text_w - indent).max(40.0);
            let top = y;
            match &p.kind {
                Kind::Heading(lv) => {
                    if i > 0 {
                        y += match lv {
                            1 => 22.0,
                            2 => 18.0,
                            3 => 14.0,
                            _ => 10.0,
                        };
                    }
                    let (h, _) = flow(&mut self.boxes[i].layout, Some(w), Alignment::Left, scale);
                    spots[i] = Spot { x, y, w, h, clip: None };
                    y += h;
                    if *lv <= 2 {
                        decos.push(Deco::Bar { x, y: y + 4.0, w, h: 1.0, color: l.line });
                        y += 5.0;
                    }
                    y += 8.0;
                    i += 1;
                }
                Kind::Code(_) => {
                    let (h, _) = flow(&mut self.boxes[i].layout, Some(w - 2.0 * CODE_PAD), Alignment::Left, scale);
                    decos.push(Deco::Ground { x, y, w, h: h + 2.0 * CODE_PAD, radius: 5.0, color: code_ground });
                    spots[i] = Spot { x: x + CODE_PAD, y: y + CODE_PAD, w: w - 2.0 * CODE_PAD, h, clip: None };
                    y += h + 2.0 * CODE_PAD + 12.0;
                    i += 1;
                }
                Kind::Rule => {
                    decos.push(Deco::Bar { x, y: y + 8.0, w, h: 1.0, color: l.line });
                    spots[i] = Spot { x, y, w, h: 17.0, clip: None };
                    y += 17.0 + 6.0;
                    i += 1;
                }
                Kind::Cell { table, .. } => {
                    let t = *table;
                    let end = (i..n).find(|&k| !matches!(self.doc.paras[k].kind, Kind::Cell { table, .. } if table == t)).unwrap_or(n);
                    y = self.flow_table(i..end, x, y, w, scale, l, &mut spots, &mut decos);
                    y += 12.0;
                    i = end;
                }
                Kind::Text => {
                    let (h, _) = flow(&mut self.boxes[i].layout, Some(w), Alignment::Left, scale);
                    spots[i] = Spot { x, y, w, h, clip: None };
                    if let Some(&k) = markers.get(&i) {
                        let (mh, mw) = flow(&mut self.boxes[k].layout, None, Alignment::Left, scale);
                        spots[k] = Spot { x: x - 7.0 - mw, y, w: mw, h: mh, clip: None };
                    }
                    let next_tight = self.doc.paras.get(i + 1).is_some_and(|q| q.depth > 0 && q.tight);
                    y += h + if p.depth > 0 && p.tight && next_tight { 3.0 } else { (body_px * 0.75).round() };
                    i += 1;
                }
            }
            // (inline code's ground, under its glyphs)
            if !matches!(p.kind, Kind::Code(_) | Kind::Rule) {
                let k = if matches!(p.kind, Kind::Cell { .. }) { None } else { Some(i - 1) };
                if let Some(k) = k {
                    self.code_grounds(k, &spots[k], scale, inline_ground, &mut decos);
                }
            }
            // (a quote's bars, down its paragraphs and the space after them)
            for q in 0..p.quote {
                let bx = PAD_X + q as f64 * QUOTE_INDENT + 2.0;
                decos.push(Deco::Bar { x: bx, y: top - 2.0, w: 3.0, h: (y - top - 4.0).max(1.0), color: l.border });
            }
        }
        // (table cells' inline code, now that the cells are placed)
        for (k, spot) in spots.iter().enumerate().take(n) {
            if matches!(self.doc.paras[k].kind, Kind::Cell { .. }) {
                self.code_grounds(k, spot, scale, inline_ground, &mut decos);
            }
        }
        self.spots = spots;
        self.decos = decos;
        self.height = y + PAD_Y;
    }

    /// A table's cells (paragraphs `range`) placed at (x, y), `w` wide at
    /// most: the grid's lines and the header's ground; where it ends.
    #[allow(clippy::too_many_arguments)]
    fn flow_table(&mut self, range: std::ops::Range<usize>, x: f64, y: f64, w: f64, scale: f64, l: &Look, spots: &mut [Spot], decos: &mut Vec<Deco>) -> f64 {
        let cells: Vec<(usize, usize, usize)> = range.clone().filter_map(|k| if let Kind::Cell { row, col, .. } = self.doc.paras[k].kind { Some((k, row, col)) } else { None }).collect();
        let t = match self.doc.paras[range.start].kind {
            Kind::Cell { table, .. } => table,
            _ => return y,
        };
        let align = self.doc.tables.get(t).map(|t| t.align.clone()).unwrap_or_default();
        let cols = cells.iter().map(|c| c.2 + 1).max().unwrap_or(1).max(align.len());
        let rows = cells.iter().map(|c| c.1 + 1).max().unwrap_or(1);
        // (each column: as wide as its widest cell wants, at least its
        // longest word, both capped)
        let (mut min, mut max) = (vec![24.0f64; cols], vec![24.0f64; cols]);
        for &(k, _, c) in &cells {
            self.boxes[k].layout.break_all_lines(None);
            let cw = self.boxes[k].layout.calculate_content_widths();
            let (lo, hi) = (f64::from(cw.min) / scale + 2.0 * CELL_PAD_X, f64::from(cw.max) / scale + 2.0 * CELL_PAD_X + 1.0);
            min[c] = min[c].max(lo.min(160.0));
            max[c] = max[c].max(hi);
        }
        let (smin, smax): (f64, f64) = (min.iter().sum(), max.iter().sum());
        let widths: Vec<f64> = if smax <= w {
            max.clone()
        } else if smin >= w {
            min.iter().map(|m| m * w / smin).collect()
        } else {
            (0..cols).map(|c| min[c] + (max[c] - min[c]) * (w - smin) / (smax - smin)).collect()
        };
        let lefts: Vec<f64> = widths.iter().scan(x, |at, cw| {
            let l = *at;
            *at += cw;
            Some(l)
        }).collect();
        let total_w: f64 = widths.iter().sum();
        let mut heights = vec![0.0f64; rows];
        for &(k, r, c) in &cells {
            let a = match align.get(c) {
                Some(1) => Alignment::Center,
                Some(2) => Alignment::Right,
                _ => Alignment::Left,
            };
            let (h, _) = flow(&mut self.boxes[k].layout, Some(widths[c] - 2.0 * CELL_PAD_X), a, scale);
            heights[r] = heights[r].max(h + 2.0 * CELL_PAD_Y);
        }
        let tops: Vec<f64> = heights.iter().scan(y, |at, h| {
            let t = *at;
            *at += h;
            Some(t)
        }).collect();
        let total_h: f64 = heights.iter().sum();
        // (the header row's ground)
        if rows > 0 {
            decos.push(Deco::Ground { x, y, w: total_w, h: heights[0], radius: 0.0, color: l.section });
        }
        for &(k, r, c) in &cells {
            let (cx0, cy0) = (lefts[c], tops[r]);
            spots[k] = Spot { x: cx0 + CELL_PAD_X, y: cy0 + CELL_PAD_Y, w: widths[c] - 2.0 * CELL_PAD_X, h: heights[r] - 2.0 * CELL_PAD_Y, clip: Some((cx0 + 1.0, cy0 + 1.0, widths[c] - 1.0, heights[r] - 1.0)) };
        }
        // (the grid: the rows' and the columns' lines, the frame)
        for ly in tops.iter().copied().chain([y + total_h]) {
            decos.push(Deco::Bar { x, y: ly, w: total_w + 1.0, h: 1.0, color: l.line });
        }
        for lx in lefts.iter().copied().chain([x + total_w]) {
            decos.push(Deco::Bar { x: lx, y, w: 1.0, h: total_h, color: l.line });
        }
        y + total_h + 1.0
    }

    /// Inline code's tinted ground under paragraph `k`'s code runs.
    fn code_grounds(&self, k: usize, spot: &Spot, scale: f64, color: u32, decos: &mut Vec<Deco>) {
        let p = &self.doc.paras[k];
        for (r, st) in &p.runs {
            if !st.code {
                continue;
            }
            let l = &self.boxes[k].layout;
            let sel = Selection::new(Cursor::from_byte_index(l, r.start, Affinity::Downstream), Cursor::from_byte_index(l, r.end, Affinity::Upstream));
            for (b, _) in sel.geometry(l) {
                let (x0, x1) = (b.x0 / scale, b.x1 / scale);
                let (y0, y1) = (b.y0 / scale, b.y1 / scale);
                let inset = ((y1 - y0) * 0.12).round();
                decos.push(Deco::Ground { x: spot.x + x0 - 2.0, y: spot.y + y0 + inset, w: x1 - x0 + 4.0, h: (y1 - y0 - 2.0 * inset).max(1.0), radius: 3.0, color });
            }
        }
    }

    /// The place in the text at (x, y) (logical, from the text's top
    /// left): the paragraph and byte nearest it, and whether the point is
    /// on that paragraph's box.
    fn hit(&self, x: f64, y: f64, scale: f64) -> Option<((usize, usize), bool)> {
        let n = self.doc.paras.len();
        if n == 0 {
            return None;
        }
        // (the boxes across y; the one under x, else the nearest)
        let mut best: Option<(usize, f64)> = None;
        for k in 0..n {
            let s = &self.spots[k];
            if y < s.y && best.is_none() && (k == 0 || self.spots[k - 1].y + self.spots[k - 1].h <= y) {
                return Some(((k, 0), false));
            }
            if y >= s.y && y < s.y + s.h.max(1.0) {
                let d = if x < s.x { s.x - x } else if x > s.x + s.w { x - s.x - s.w } else { 0.0 };
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((k, d));
                }
            }
        }
        let Some((k, d)) = best else {
            // (below the text, or between paragraphs: the next one's start)
            let next = (0..n).find(|&k| self.spots[k].y > y);
            return Some(match next {
                Some(k) => ((k, 0), false),
                None => ((n - 1, self.doc.paras[n - 1].text.len()), false),
            });
        };
        let s = &self.spots[k];
        let l = &self.boxes[k].layout;
        let c = Cursor::from_point(l, ((x - s.x) * scale) as f32, ((y - s.y) * scale) as f32);
        Some(((k, c.index().min(self.doc.paras[k].text.len())), d == 0.0))
    }

    /// The link under (x, y), if the point is on its text.
    fn link_at(&self, x: f64, y: f64, scale: f64) -> Option<usize> {
        let ((k, at), on) = self.hit(x, y, scale)?;
        if !on {
            return None;
        }
        let link = self.doc.link_at(k, at).or_else(|| at.checked_sub(1).and_then(|b| self.doc.link_at(k, b)))?;
        let p = &self.doc.paras[k];
        let s = &self.spots[k];
        let l = &self.boxes[k].layout;
        let (px, py) = ((x - s.x) * scale, (y - s.y) * scale);
        p.runs.iter().filter(|(_, st)| st.link == Some(link)).any(|(r, _)| {
            let sel = Selection::new(Cursor::from_byte_index(l, r.start, Affinity::Downstream), Cursor::from_byte_index(l, r.end, Affinity::Upstream));
            sel.geometry(l).iter().any(|(b, _)| px >= b.x0 && px < b.x1 && py >= b.y0 && py < b.y1)
        })
        .then_some(link)
    }
}

/// The view's text area (its own pixels): inside a classic edge.
fn body(cx: &Cx, l: &Look) -> Rect {
    let i = common::inset(l);
    (i, i, (cx.width() - 2 * i).max(0), (cx.height() - 2 * i).max(0))
}

/// Shapes and flows the text for the view's size now; the scroll bar's
/// place: (position, its ops).
fn lay_out(cx: &mut Cx, theme: &'static Theme) -> (i64, Vec<Op>) {
    let f = faces(cx, theme);
    let (_, _, bw, bh) = body(cx, &f.l);
    Laid::shape(cx, theme);
    let scale = cx.scale;
    let Some(m) = cx.ui.markdown.as_mut() else { return (0, Vec::new()) };
    // (the full width; less the bar's when the text is taller than the view
    // — kept so while it still is)
    let narrow = (bw - rapidr_value::scrollbars::BAR) as f64;
    if !(m.flowed_at == narrow && m.height > bh as f64) {
        m.flow(bw as f64, scale, &f);
        if m.height > bh as f64 {
            m.flow(narrow, scale, &f);
        }
    }
    let scroll_to = model::with_mut(cx.id, |md| md.scroll_to.take());
    if let Some(k) = scroll_to {
        if let Some(s) = m.spots.get(k) {
            m.want = Some((s.y - 8.0).max(0.0) as i64);
        }
    }
    let at = m.want.take();
    let content = m.height.ceil() as i64;
    let step = (f.body.pixel_size() as f64 * 1.5).round() as i64;
    let (pos, _, ops) = vscroll_at(cx.id, bw, bh, content, step.max(1), at);
    (pos, ops)
}

impl ComponentKind for MarkdownView {
    fn name(&self) -> &'static str {
        "RMARKDOWNVIEW"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let theme = p.theme();
        let l = look(theme);
        let (w, h) = (cx.width(), cx.height());
        common::ground(p, w, h, &l);
        let (pos, bar) = lay_out(cx, theme);
        let (bx, by, bw, bh) = body(cx, &l);
        let Some(m) = cx.ui.markdown.as_ref() else { return };
        let scale = p.scale();
        let (ox, oy) = p.origin();
        if m.doc.paras.is_empty() {
            let empty = model::with(cx.id, |md| md.empty_text.clone()).unwrap_or_default();
            if !empty.is_empty() {
                let f = faces(cx, theme);
                p.text((bx + 12, by + 12, (bw - 24).max(0), (bh - 24).max(0)), &empty, &f.body, l.dim, Place::Center);
            }
        }
        let sel = model::with(cx.id, |md| md.selection).flatten().map(|(a, b)| if a <= b { (a, b) } else { (b, a) }).filter(|(a, b)| a != b);
        let (hl, hl_text) = if cx.state.focused { (l.selected, l.selected_text) } else { (l.inactive, l.inactive_text) };
        let top = pos as f64;
        let bottom = top + bh as f64;
        let id = cx.id.to_string();
        p.clipped((bx, by, bw, bh), |p| {
            for d in &m.decos {
                match *d {
                    Deco::Ground { x, y, w, h, radius, color } if y + h >= top && y <= bottom => {
                        let r = ((bx as f64 + x).round() as i64, (by as f64 + y - top).round() as i64, w.round() as i64, h.round() as i64);
                        if radius > 0.0 {
                            p.round(r, radius, Some(color), None, 0.0);
                        } else {
                            p.fill(r, color);
                        }
                    }
                    Deco::Bar { x, y, w, h, color } if y + h >= top && y <= bottom => {
                        p.fill(((bx as f64 + x).round() as i64, (by as f64 + y - top).round() as i64, w.round().max(1.0) as i64, h.round().max(1.0) as i64), color);
                    }
                    _ => {}
                }
            }
            for (k, s) in m.spots.iter().enumerate() {
                if s.y + s.h < top || s.y > bottom {
                    continue;
                }
                let para = m.boxes[k].para;
                let selection = match sel {
                    Some((a, b)) if k < m.doc.paras.len() && para >= a.0 && para <= b.0 => {
                        let l = &m.boxes[k].layout;
                        let len = m.doc.paras[para].text.len();
                        let from = if para == a.0 { a.1.min(len) } else { 0 };
                        let to = if para == b.0 { b.1.min(len) } else { len };
                        if from < to {
                            let sl = Selection::new(Cursor::from_byte_index(l, from, Affinity::Downstream), Cursor::from_byte_index(l, to, Affinity::Upstream));
                            sl.geometry(l).into_iter().map(|(b, _)| (b.x0, b.y0, b.x1, b.y1)).collect()
                        } else {
                            Vec::new()
                        }
                    }
                    _ => Vec::new(),
                };
                let origin = (((ox + bx) as f64 + s.x) * scale).round();
                let origin_y = (((oy + by) as f64 + s.y - top) * scale).round();
                let item = TextItem { node: id.clone(), para: k, origin: (origin, origin_y), selection, highlight: hl, highlight_text: hl_text, underlines: Vec::new(), caret: None, caret_color: l.text };
                match s.clip {
                    Some((x, y, w, h)) => {
                        let r = ((bx as f64 + x).round() as i64, (by as f64 + y - top).round() as i64, w.round() as i64, h.round() as i64);
                        p.clipped(r, |p| p.editor(item));
                    }
                    None => p.editor(item),
                }
            }
        });
        p.at((bx, by), |p| p.ops(bar));
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let theme = rapidr_value::theme::current();
        let l = look(theme);
        let (bx, by, bw, bh) = body(cx, &l);
        let inner = MouseIn { x: m.x - bx as f64, y: m.y - by as f64, ..*m };
        if bar_mouse(cx, &inner, bw, bh) {
            return MouseOut::default();
        }
        // (laid out as it's shown: input may come before the first paint)
        let (pos, _) = lay_out(cx, theme);
        let scale = cx.scale;
        let Some(laid) = cx.ui.markdown.as_ref() else { return MouseOut::default() };
        let (x, y) = (inner.x, inner.y.clamp(0.0, (bh - 1).max(0) as f64) + pos as f64);
        match m.kind {
            MouseKind::Down if m.button == rapidr_value::input::Button::Left => {
                let link = laid.link_at(x, y, scale);
                PRESSED.with(|p| p.borrow_mut().insert(cx.id.to_string(), link));
                if link.is_none() {
                    if let Some((at, _)) = laid.hit(x, y, scale) {
                        model::with_mut(cx.id, |md| {
                            md.selection = match (md.selection, m.mods.shift) {
                                (Some((a, _)), true) => Some((a, at)),
                                _ => Some((at, at)),
                            };
                        });
                    }
                }
                return MouseOut { press: false, focus: Some(true) };
            }
            MouseKind::Move if m.captured => {
                let pressed = PRESSED.with(|p| p.borrow().get(cx.id).copied()).flatten();
                if pressed.is_none() {
                    if let Some((at, _)) = laid.hit(x, y, scale) {
                        model::with_mut(cx.id, |md| {
                            if let Some((a, _)) = md.selection {
                                md.selection = Some((a, at));
                            }
                        });
                    }
                }
            }
            MouseKind::Up => {
                let pressed = PRESSED.with(|p| p.borrow_mut().remove(cx.id)).flatten();
                if let Some(link) = pressed {
                    if m.inside && laid.link_at(x, y, scale) == Some(link) {
                        let url = laid.doc.links.get(link).cloned().unwrap_or_default();
                        super::send(cx, User::Markdown(model::User::Link(url)));
                    }
                }
            }
            _ => {}
        }
        MouseOut::default()
    }

    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: crate::input::Mods) -> bool {
        let l = look(rapidr_value::theme::current());
        let (_, _, bw, bh) = body(cx, &l);
        vscroll_wheel(cx.id, dy, bw, bh)
    }

    fn tick(&self, cx: &mut Cx) {
        bar_tick(cx);
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let theme = rapidr_value::theme::current();
        let l = look(theme);
        let (_, _, _, bh) = body(cx, &l);
        let ctrl = k.mods.ctrl || k.mods.command;
        lay_out(cx, theme);
        let (pos, _, range) = vscroll_state(cx.id);
        let line = (BODY_PX as f64 * 1.5 * 2.0) as i64;
        let to = match k.vk {
            // Ctrl+A: everything; Ctrl+C / Cmd+C (Ctrl+Insert): the selection
            65 if ctrl => {
                model::with_mut(cx.id, |m| m.select_all());
                return true;
            }
            67 | 45 if ctrl => {
                let t = model::with_mut(cx.id, |m| m.selected_text());
                if !t.is_empty() {
                    clip.set_text(&t);
                }
                return true;
            }
            vk::UP => pos - line,
            vk::DOWN => pos + line,
            vk::PAGE_UP => pos - (bh - line).max(line),
            vk::PAGE_DOWN | vk::SPACE => pos + (bh - line).max(line),
            vk::HOME => 0,
            vk::END => range,
            _ => return false,
        };
        if let Some(m) = cx.ui.markdown.as_mut() {
            m.want = Some(to.clamp(0, (range - bh).max(0)));
        }
        true
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = crate::components::shared_describe(cx, self.name());
        n.role = Role::Document;
        n.value = None;
        if n.name.is_empty() {
            n.name = model::with(cx.id, |m| m.file_name.rsplit(['/', '\\']).next().unwrap_or("").to_string()).unwrap_or_default();
        }
        let theme = rapidr_value::theme::current();
        let l = look(theme);
        let (bx, by, _, bh) = body(cx, &l);
        let (pos, _) = lay_out(cx, theme);
        let Some(m) = cx.ui.markdown.as_ref() else { return n };
        let (x0, y0) = (cx.rect.0 + bx, cx.rect.1 + by - pos);
        let at = |s: &Spot| (x0 + s.x.round() as i64, y0 + s.y.round() as i64, s.w.round() as i64, s.h.round() as i64);
        let np = m.doc.paras.len();
        // (the paragraphs around the view)
        let first_seen = (0..np).find(|&k| m.spots[k].y + m.spots[k].h >= pos as f64).unwrap_or(0);
        let last_seen = (0..np).rev().find(|&k| m.spots[k].y <= (pos + bh) as f64).unwrap_or(0);
        let from = first_seen.saturating_sub(A11Y_AROUND);
        let to = (last_seen + A11Y_AROUND).min(np);
        let link_nodes = |k: usize, parent: &mut AccessNode| {
            let p = &m.doc.paras[k];
            let mut seen = Vec::new();
            for (r, st) in &p.runs {
                let Some(li) = st.link else { continue };
                if seen.contains(&li) {
                    continue;
                }
                seen.push(li);
                let text: String = p.runs.iter().filter(|(_, s)| s.link == Some(li)).map(|(r, _)| p.text.get(r.clone()).unwrap_or("")).collect();
                let mut ln = AccessNode::new(part_id(cx.id, PART_ITEM, A11Y_LINK + li), Role::Link);
                ln.name = text;
                ln.description = m.doc.links.get(li).cloned().unwrap_or_default();
                ln.actions = vec![Action::Click];
                let l = &m.boxes[k].layout;
                let s = &m.spots[k];
                let sel = Selection::new(Cursor::from_byte_index(l, r.start, Affinity::Downstream), Cursor::from_byte_index(l, r.end, Affinity::Upstream));
                if let Some((b, _)) = sel.geometry(l).first() {
                    let sc = cx.scale;
                    ln.bounds = (x0 + (s.x + b.x0 / sc).round() as i64, y0 + (s.y + b.y0 / sc).round() as i64, ((b.x1 - b.x0) / sc).round() as i64, ((b.y1 - b.y0) / sc).round() as i64);
                }
                parent.children.push(ln);
            }
        };
        let mut list: Option<AccessNode> = None;
        let mut k = from;
        while k < to {
            let p = &m.doc.paras[k];
            if p.depth == 0 {
                if let Some(ls) = list.take() {
                    n.children.push(ls);
                }
            }
            match &p.kind {
                Kind::Cell { table, .. } => {
                    let t = *table;
                    let mut tb = AccessNode::new(part_id(cx.id, PART_CELL, k), Role::Table);
                    let mut row: Option<(usize, AccessNode)> = None;
                    let start = k;
                    while k < to {
                        let Kind::Cell { table, row: r, .. } = m.doc.paras[k].kind else { break };
                        if table != t {
                            break;
                        }
                        if row.as_ref().is_none_or(|(rr, _)| *rr != r) {
                            if let Some((_, rn)) = row.take() {
                                tb.children.push(rn);
                            }
                            row = Some((r, AccessNode::new(part_id(cx.id, PART_ROW, k), Role::Row)));
                        }
                        let mut c = AccessNode::new(part_id(cx.id, PART_CELL, k + (1 << 23)), Role::Cell);
                        c.name = m.doc.paras[k].text.clone();
                        c.bounds = at(&m.spots[k]);
                        link_nodes(k, &mut c);
                        if let Some((_, rn)) = row.as_mut() {
                            rn.children.push(c);
                        }
                        k += 1;
                    }
                    if let Some((_, rn)) = row.take() {
                        tb.children.push(rn);
                    }
                    let (a, b) = (&m.spots[start], &m.spots[k - 1]);
                    tb.bounds = (x0 + a.x.round() as i64, y0 + a.y.round() as i64, (b.x + b.w - a.x).round() as i64, (b.y + b.h - a.y).round() as i64);
                    n.children.push(tb);
                    continue;
                }
                kind => {
                    let mut node = AccessNode::new(part_id(cx.id, PART_ROW, k), Role::Label);
                    node.name = p.text.clone();
                    node.bounds = at(&m.spots[k]);
                    match kind {
                        Kind::Heading(lv) => {
                            node.role = Role::Heading;
                            node.level = Some(usize::from(*lv));
                        }
                        Kind::Code(_) => node.description = "code".into(),
                        Kind::Rule => {
                            node.role = Role::Splitter;
                            node.name.clear();
                        }
                        _ => {}
                    }
                    link_nodes(k, &mut node);
                    if p.depth > 0 {
                        if p.marker.is_some() {
                            node.role = Role::ListItem;
                            node.level = Some(p.depth);
                        }
                        let ls = list.get_or_insert_with(|| AccessNode::new(part_id(cx.id, PART_ITEM, k), Role::List));
                        ls.children.push(node);
                    } else {
                        n.children.push(node);
                    }
                }
            }
            k += 1;
        }
        if let Some(ls) = list.take() {
            n.children.push(ls);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        match (part, action) {
            (Some(i), Action::Click) if i >= A11Y_LINK => {
                let url = cx.ui.markdown.as_ref().and_then(|m| m.doc.links.get(i - A11Y_LINK).cloned());
                match url {
                    Some(u) => {
                        super::send(cx, User::Markdown(model::User::Link(u)));
                        true
                    }
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// `__item_<n>`: link n clicked (the tests' step).
    fn test_action(&self, cx: &mut Cx, action: &str) -> bool {
        if let Some(n) = action.strip_prefix("__item_").and_then(|s| s.parse::<usize>().ok()) {
            let url = model::with_mut(cx.id, |m| m.doc().links.get(n).cloned());
            if let Some(u) = url {
                super::send(cx, User::Markdown(model::User::Link(u)));
                return true;
            }
            return false;
        }
        false
    }
}

thread_local! {
    /// A press on a link: its release on the same link opens it.
    static PRESSED: std::cell::RefCell<std::collections::HashMap<String, Option<usize>>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// (no in-place editor)
pub fn focus_left(_id: &str, _ed: crate::components::list::InPlace) -> Vec<KernelEvent> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use rapidr_value::input::Button;
    use rapidr_value::objects::a11y::Role;
    use rapidr_value::panels::markdown as model;
    use rapidr_value::panels::User;
    use rapidr_value::v_int;

    use crate::components::form::Container;
    use crate::input::MemClipboard;
    use crate::{FormUi, KernelEvent, MemStore, Mods, TextSystem};

    const TEXT: &str = "# Title\n\nSee [the docs](https://example.com) and [more](#more).\n\n- one\n- two\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n## More\n\nend\n";

    fn setup(id: &str) -> (MemStore, FormUi, TextSystem) {
        let mut s = MemStore::new();
        s.add("mform", "RFORM", None);
        s.add(id, "RMARKDOWNVIEW", Some("mform")).set(id, "width", v_int(500)).set(id, "height", v_int(300));
        model::remove(id);
        model::with_mut(id, |m| m.set_text(TEXT));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "mform", false);
        f.paint(&s, &mut ts, 1.0);
        (s, f, ts)
    }

    fn spot(f: &FormUi, id: &str, k: usize) -> super::Spot {
        f.node(id).and_then(|n| n.ui.markdown.as_ref()).map(|m| m.spots[k]).expect("laid out")
    }

    #[test]
    fn laid_out_drawn_and_described() {
        let (s, mut f, mut ts) = setup("md1");
        let list = f.paint(&s, &mut ts, 1.0);
        // (every paragraph and both markers drawn as text: 10 paragraphs + 2)
        assert_eq!(list.items.iter().filter(|i| matches!(i, crate::display::Item::Text(t) if t.node == "md1")).count(), 12);
        // (headings above paragraphs, list items indented)
        let (h, p, li) = (spot(&f, "md1", 0), spot(&f, "md1", 1), spot(&f, "md1", 2));
        assert!(h.y < p.y && p.y < li.y && li.x > p.x, "{h:?} {p:?} {li:?}");
        // (the table's cells side by side)
        let (a, b) = (spot(&f, "md1", 4), spot(&f, "md1", 5));
        assert!((a.y - b.y).abs() < 0.5 && b.x > a.x + a.w);
        let tree = f.access_tree(&s, &mut ts);
        let doc = &tree.children[0];
        assert_eq!(doc.role, Role::Document);
        let roles: Vec<Role> = doc.children.iter().map(|n| n.role).collect();
        assert_eq!(roles, vec![Role::Heading, Role::Label, Role::List, Role::Table, Role::Heading, Role::Label]);
        assert_eq!(doc.children[0].level, Some(1));
        assert_eq!(doc.children[1].children.iter().map(|l| (l.role, l.name.as_str())).collect::<Vec<_>>(), vec![(Role::Link, "the docs"), (Role::Link, "more")]);
        assert_eq!(doc.children[2].children[1].role, Role::ListItem);
        assert_eq!(doc.children[3].children[1].children[1].name, "2");
        model::remove("md1");
    }

    #[test]
    fn links_selection_and_copy() {
        let (s, mut f, mut ts) = setup("md2");
        // a click on "the docs": OnLinkClick with its target
        let p = spot(&f, "md2", 1);
        let x = p.x + 40.0;
        let y = p.y + p.h / 2.0;
        f.mouse_down(&s, &mut ts, x, y, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, x, y, Button::Left, Mods::NONE);
        let events: Vec<User> = f
            .take_events()
            .into_iter()
            .filter_map(|e| match e {
                KernelEvent::Container(Container::Panel { action, .. }) => Some(action),
                _ => None,
            })
            .collect();
        assert_eq!(events, vec![User::Markdown(model::User::Link("https://example.com".into()))]);
        // a drag from the title's start into "See": selected, Ctrl+C copies it
        let h = spot(&f, "md2", 0);
        f.mouse_down(&s, &mut ts, h.x + 0.5, h.y + h.h / 2.0, Button::Left, Mods::NONE);
        f.mouse_move(&s, &mut ts, p.x + 2.0, y, Mods::NONE);
        f.mouse_up(&s, &mut ts, p.x + 2.0, y, Button::Left, Mods::NONE);
        let mut clip = MemClipboard::default();
        let ctrl = Mods { ctrl: true, ..Mods::NONE };
        f.key_down(&s, &mut ts, 67, "", ctrl, &mut clip);
        assert!(clip.0.as_deref().is_some_and(|t| t.starts_with("Title\n")), "{:?}", clip.0);
        // Ctrl+A, Ctrl+C: everything, a table's row by tabs
        f.key_down(&s, &mut ts, 65, "", ctrl, &mut clip);
        f.key_down(&s, &mut ts, 67, "", ctrl, &mut clip);
        assert_eq!(clip.0.as_deref(), Some("Title\nSee the docs and more.\none\ntwo\nA\tB\n1\t2\nMore\nend"));
        model::remove("md2");
    }

    /// A long text (the RapidQ corpus's import report is ~7,700 lines) is
    /// laid out once and drawn by what's in view.
    #[test]
    fn a_long_text_lays_out_quickly() {
        let mut big = String::from("# Report\n\n| Program | Result |\n|---|---|\n");
        for i in 0..400 {
            big.push_str(&format!("| `dir/program{i}.bas` | yes: identical bytecode |\n"));
        }
        big.push('\n');
        for i in 0..6000 {
            big.push_str(&format!("- line {i}, column 3: `QBUTTON` → `RButton`\n"));
        }
        let (s, mut f, mut ts) = setup("md3");
        model::with_mut("md3", |m| m.set_text(&big));
        let t0 = std::time::Instant::now();
        let list = f.paint(&s, &mut ts, 2.0);
        let first = t0.elapsed();
        let t1 = std::time::Instant::now();
        f.paint(&s, &mut ts, 2.0);
        let again = t1.elapsed();
        eprintln!("long text: first paint {first:?}, next {again:?}");
        // (only what's in view is drawn)
        assert!(list.items.iter().filter(|i| matches!(i, crate::display::Item::Text(_))).count() < 80);
        assert!(again < first);
        model::remove("md3");
    }
}
