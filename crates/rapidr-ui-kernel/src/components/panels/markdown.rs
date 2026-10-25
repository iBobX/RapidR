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
//! Its links are reachable by Tab: with the view focused, Tab steps to its
//! first link, then each next one (a ring round the link, the view scrolling
//! to it), and out of the view after the last; Shift+Tab the other way
//! round; Enter or Space follows the link the keyboard has, as a click does.

use std::rc::Rc;

use parley::{Affinity, Alignment, AlignmentOptions, Cursor, FontStyle, FontWeight, Layout, LineHeight, Selection, StyleProperty};
use rapidr_value::objects::a11y::{part_id, AccessNode, Action, Role, PART_CELL, PART_ITEM, PART_ROW};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Rect};
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
use crate::tree::FormUi;

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
    /// (CodeLanguage) The editor's language for the inline `code`.
    code_lang: Option<&'static str>,
    /// The ink of added and removed counts (`<ins>`, `<del>`): the editor's
    /// diff colours of the theme, made readable on the view's ground.
    added: u32,
    removed: u32,
}

/// `fg` made readable on `bg` (WCAG `need`:1): darker on a light ground,
/// lighter on a dark one, as little as it takes.
fn ink_for(fg: u32, bg: u32, need: f64) -> u32 {
    use rapidr_value::theme::{contrast, luminance};
    if contrast(fg, bg) >= need {
        return fg;
    }
    let target = if luminance(bg) > 0.4 { 0x000000 } else { 0xFFFFFF };
    for k in 1..=20 {
        let t = f64::from(k) * 0.05;
        let mixed = (0..3).fold(0u32, |acc, i| {
            let sh = 16 - 8 * i;
            let (a, b) = (f64::from((fg >> sh) & 0xFF), f64::from((target >> sh) & 0xFF));
            acc | (((a + (b - a) * t).round() as u32) << sh)
        });
        if contrast(mixed, bg) >= need {
            return mixed;
        }
    }
    target
}

fn faces(cx: &Cx, theme: &'static Theme) -> Faces {
    let program_font = !store::string(cx.store, cx.id, "fontname").trim().is_empty();
    let body = if program_font { Font { color: 0, styles: 0, ..cx.font.clone() } } else { Font { name: String::new(), size: -BODY_PX, color: 0, styles: 0 } };
    let px = body.pixel_size().max(6);
    let mono = Font { name: "JetBrains Mono".into(), size: -((px as f64 * 0.92).round() as i64).max(6), color: 0, styles: 0 };
    let code_lang = model::with(cx.id, |m| m.code_language.clone()).and_then(|l| code_language(&l));
    let l = look(theme);
    let scheme = rapidr_value::code_scheme::for_theme(theme);
    let need = if theme.contrast { 7.0 } else { 4.5 };
    Faces { body, mono, l, theme, code_lang, added: ink_for(scheme.mark_added, l.body, need), removed: ink_for(scheme.mark_removed, l.body, need) }
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

/// The code editor's language for a code block's (its fence's word, in
/// lower case): BASIC, SQL (`sql`; `sqlite`, `mysql` with their dialect's
/// words) and the editor's other languages, coloured as the code editor
/// colours them; `None` for any other (plain).
pub fn code_language(lang: &str) -> Option<&'static str> {
    Some(match lang {
        "basic" | "bas" | "rapidq" | "rapidr" | "rr" | "rq" | "rqw" | "rqb" | "inc" | "qbasic" | "freebasic" | "vb" | "vbnet" | "vb6" => "rapidr-basic",
        "sql" => "sql",
        "sqlite" | "sqlite3" => "sql-sqlite",
        "mysql" | "mariadb" => "sql-mysql",
        "json" | "jsonl" => "json",
        "toml" => "toml",
        "csv" => "csv",
        "css" => "css",
        "javascript" | "js" | "mjs" => "javascript",
        "rust" | "rs" => "rust",
        "markdown" | "md" => "markdown",
        "html" | "htm" | "xhtml" => "html",
        _ => return None,
    })
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
    // (a diff's counts: the diff colours, the code font — monospaced, so the
    // digits line up —, the signs say it without the colour)
    if st.diff != 0 {
        out.push(StyleProperty::Brush(Ink(if st.diff == 1 { f.added } else { f.removed })));
        let mono = Font { size: -((base.pixel_size() as f64 * 0.92).round() as i64).max(6), ..f.mono.clone() };
        out.extend(styles(&mono, 0).into_iter().filter(|p| matches!(p, StyleProperty::FontFamily(_) | StyleProperty::FontSize(_) | StyleProperty::FontFeatures(_))));
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
        Kind::Code(lang) if code_language(lang).is_some() => {
            // (BASIC's, SQL's … colours as the code editor gives them: the
            // language's definition and the theme's scheme — S-EDITOR's one
            // highlighter, as the editor's popups use it)
            let languages = rapidr_editor::lang::Languages::builtin();
            let mut hl = rapidr_editor::Highlighter::new(code_language(lang).and_then(|id| languages.get(id)).unwrap_or_else(|| languages.plain_text()));
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
                    // (inline code of a CodeLanguage: coloured as the code
                    // editor colours it — the same highlighter and scheme as
                    // a code block's)
                    if let (true, Some(id)) = (st.code, f.code_lang) {
                        let languages = rapidr_editor::lang::Languages::builtin();
                        let mut hl = rapidr_editor::Highlighter::new(languages.get(id).unwrap_or_else(|| languages.plain_text()));
                        let scheme = rapidr_value::code_scheme::for_theme(f.theme);
                        let mut state = rapidr_editor::ROOT;
                        let mut at = r.start;
                        for line in text[r.clone()].split('\n') {
                            let (tokens, next) = hl.line_spans(line, state);
                            state = next;
                            for t in tokens {
                                let ts = scheme.token(t.kind.fallbacks());
                                let range = at + t.start as usize..at + t.end as usize;
                                b.push(StyleProperty::Brush(Ink(ts.color)), range.clone());
                                if ts.bold {
                                    b.push(StyleProperty::FontWeight(FontWeight::BOLD), range.clone());
                                }
                                if ts.italic {
                                    b.push(StyleProperty::FontStyle(FontStyle::Italic), range);
                                }
                            }
                            at += line.len() + 1;
                        }
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

    /// The scale it was shaped at.
    fn scale(&self) -> f64 {
        f64::from_bits(self.shaped_for.1)
    }

    /// The word of code under (x, y) (logical, from the text's top left),
    /// in inline `code` or a code block of RapidR's language: the word, and
    /// whether a dot comes before it (a member). `None` for anything else.
    fn code_word(&self, x: f64, y: f64, scale: f64) -> Option<(String, bool)> {
        let ((k, at), on) = self.hit(x, y, scale)?;
        if !on {
            return None;
        }
        let p = self.doc.paras.get(k)?;
        let t = p.text.as_str();
        let span = match &p.kind {
            Kind::Code(lang) => (code_language(lang) == Some("rapidr-basic")).then_some(0..t.len())?,
            Kind::Text | Kind::Cell { .. } => p.runs.iter().find(|(r, st)| st.code && r.start <= at && at <= r.end).map(|(r, _)| r.clone())?,
            _ => return None,
        };
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let at = at.clamp(span.start, span.end);
        // (the word the point is on: the character at it, else the one
        // before — the cursor sits between characters)
        let after = t[at..span.end].chars().next();
        let before = t[span.start..at].chars().next_back();
        let mut from = at;
        let mut to = at;
        if after.is_some_and(is_word) {
            to = at + after.map_or(0, char::len_utf8);
        } else if before.is_some_and(is_word) {
            from = at - before.map_or(0, char::len_utf8);
        } else {
            return None;
        }
        while let Some(c) = t[span.start..from].chars().next_back().filter(|c| is_word(*c)) {
            from -= c.len_utf8();
        }
        while let Some(c) = t[to..span.end].chars().next().filter(|c| is_word(*c)) {
            to += c.len_utf8();
        }
        // (LEFT$, $THEME: a directive's $ before, a string function's after)
        if t[span.start..from].ends_with('$') {
            from -= 1;
        }
        if t[to..span.end].starts_with('$') {
            to += 1;
        }
        let word = &t[from..to];
        if word.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return None;
        }
        // (the point must be on the word itself, not beside it)
        let (s, l) = (&self.spots[k], &self.boxes[k].layout);
        let (px, py) = ((x - s.x) * scale, (y - s.y) * scale);
        let sel = Selection::new(Cursor::from_byte_index(l, from, Affinity::Downstream), Cursor::from_byte_index(l, to, Affinity::Upstream));
        let inside = sel.geometry(l).iter().any(|(b, _)| px >= b.x0 && px < b.x1 && py >= b.y0 && py < b.y1);
        inside.then(|| (word.to_string(), t[span.start..from].ends_with('.')))
    }

    /// Link `li`'s boxes (logical, from the text's top left), one for each
    /// line it runs over.
    fn link_rects(&self, li: usize, scale: f64) -> Vec<(f64, f64, f64, f64)> {
        let Some(k) = self.doc.paras.iter().position(|p| p.runs.iter().any(|(_, st)| st.link == Some(li))) else { return Vec::new() };
        let (p, s, l) = (&self.doc.paras[k], &self.spots[k], &self.boxes[k].layout);
        let mut out = Vec::new();
        for (r, _) in p.runs.iter().filter(|(_, st)| st.link == Some(li)) {
            let sel = Selection::new(Cursor::from_byte_index(l, r.start, Affinity::Downstream), Cursor::from_byte_index(l, r.end, Affinity::Upstream));
            for (b, _) in sel.geometry(l) {
                out.push((s.x + b.x0 / scale, s.y + b.y0 / scale, (b.x1 - b.x0) / scale, (b.y1 - b.y0) / scale));
            }
        }
        out
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
    // (FollowEnd: a view at its end before the text grew stays at its end)
    // (at its end as it was at the last layout too: a view made shorter — a
    // status strip appearing under a chat — was at its end all the same)
    let last_bh = model::with_mut(cx.id, |md| std::mem::replace(&mut md.last_body_h, bh));
    let at_end = model::with_mut(cx.id, |md| md.follow_end) && {
        let (pos, _, range) = vscroll_state(cx.id);
        range <= bh || pos + bh >= range - 4 || pos + last_bh >= range - 4
    };
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
    // (the link the keyboard just moved to: shown, with a little of the
    // text around it)
    if model::with_mut(cx.id, |md| std::mem::take(&mut md.reveal_link)) {
        if let Some(li) = model::with(cx.id, |md| md.link_focus).flatten() {
            let rects = m.link_rects(li, scale);
            if let Some(top) = rects.iter().map(|r| r.1).reduce(f64::min) {
                let bottom = rects.iter().map(|r| r.1 + r.3).fold(0.0, f64::max);
                let (margin, pos) = (16.0, vscroll_state(cx.id).0 as f64);
                if top - margin < pos {
                    m.want = Some((top - margin).max(0.0) as i64);
                } else if bottom + margin > pos + bh as f64 {
                    m.want = Some((bottom + margin - bh as f64).ceil() as i64);
                }
            }
        }
    }
    let content = m.height.ceil() as i64;
    let at = m.want.take().or_else(|| at_end.then_some((content - bh).max(0)));
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
                // (wrapped to the view's width: a narrow pane shows all of it)
                common::wrapped_lines(p, (bx + 12, by + 12, (bw - 24).max(0), (bh - 24).max(0)), &empty, &f.body, l.dim, true);
            }
        }
        let sel = model::with(cx.id, |md| md.selection).flatten().map(|(a, b)| if a <= b { (a, b) } else { (b, a) }).filter(|(a, b)| a != b);
        // (the link the keyboard has: ringed)
        let ringed = if cx.state.focused { model::with(cx.id, |md| md.link_focus).flatten().map(|li| m.link_rects(li, cx.scale)).unwrap_or_default() } else { Vec::new() };
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
            for &(x, y, w, h) in &ringed {
                p.focus_ring(((bx as f64 + x - 2.0).round() as i64, (by as f64 + y - top - 1.0).round() as i64, (w + 4.0).round() as i64, (h + 2.0).round() as i64), false);
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
                model::with_mut(cx.id, |md| md.link_focus = None);
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
                } else if m.inside && m.button == rapidr_value::input::Button::Left && !model::with(cx.id, |md| md.code_language.is_empty()).unwrap_or(true) {
                    // (a word of code clicked — not the end of a selection
                    // being dragged — that the language service knows)
                    let collapsed = model::with(cx.id, |md| md.selection.is_none_or(|(a, b)| a == b)).unwrap_or(true);
                    if let (true, Some((word, member))) = (collapsed, laid.code_word(x, y, scale)) {
                        if let Some(h) = model::word_help(&word, member) {
                            super::send(cx, User::Markdown(model::User::Word { word, of: h.of }));
                        }
                    }
                }
            }
            _ => {}
        }
        MouseOut::default()
    }

    /// A hand over a link, and over a word of code the language service
    /// knows (a click opens its help).
    fn pointer(&self, cx: &mut Cx, x: i64, y: i64) -> rapidr_value::input::Cursor {
        let l = look(rapidr_value::theme::current());
        let (bx, by, _, bh) = body(cx, &l);
        let pos = vscroll_state(cx.id).0 as f64;
        let words = !model::with(cx.id, |md| md.code_language.is_empty()).unwrap_or(true);
        let scale = cx.scale;
        let Some(laid) = cx.ui.markdown.as_ref() else { return rapidr_value::input::Cursor::Default };
        let (px, py) = ((x - bx) as f64, ((y - by) as f64).clamp(0.0, (bh - 1).max(0) as f64) + pos);
        let on_link = laid.link_at(px, py, scale).is_some();
        let on_word = words && laid.code_word(px, py, scale).is_some_and(|(w, member)| model::word_help(&w, member).is_some());
        if on_link || on_word {
            rapidr_value::input::Cursor::Hand
        } else {
            rapidr_value::input::Cursor::Default
        }
    }

    /// The word of code under the mouse, in one line (a tooltip).
    fn tip_laid(&self, ui: &crate::tree::NodeUi, _store: &dyn crate::store::Store, id: &str, x: f64, y: f64) -> Option<String> {
        if model::with(id, |md| md.code_language.is_empty()).unwrap_or(true) {
            return None;
        }
        let laid = ui.markdown.as_ref()?;
        let inset = common::inset(&look(rapidr_value::theme::current())) as f64;
        let tip = laid.code_word(x - inset, y - inset + vscroll_state(id).0 as f64, laid.scale()).and_then(|(word, member)| model::word_help(&word, member)).map(|h| h.tip);
        model::with_mut(id, |m| m.last_tip = tip.clone().unwrap_or_default());
        tip
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
        // (Enter or Space on the link the keyboard has: a click on it)
        if matches!(k.vk, vk::ENTER | vk::SPACE) && !ctrl && !k.mods.alt {
            if let Some(li) = model::with(cx.id, |md| md.link_focus).flatten() {
                if let Some(url) = model::with_mut(cx.id, |md| md.doc().links.get(li).cloned()) {
                    super::send(cx, User::Markdown(model::User::Link(url)));
                    return true;
                }
            }
        }
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
        // (the link the keyboard has, while the view has the focus)
        let focused_link = if cx.state.focused { model::with(cx.id, |md| md.link_focus).flatten() } else { None };
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
                ln.states.focused = focused_link == Some(li);
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
                        c.name = m.doc.paras[k].speech();
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
                    node.name = p.speech();
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

/// Tab (Shift+Tab: `back`) with an RMARKDOWNVIEW focused: the keyboard goes
/// to the view's next stop — the view itself, then each link; the view
/// shows the link at its next paint. Whether it did; `false` when the view
/// has no more (or isn't the focused component): the focus moves on.
pub(crate) fn tab(f: &FormUi, back: bool) -> bool {
    let Some(i) = f.focus.filter(|&i| f.nodes[i].type_name == "RMARKDOWNVIEW") else { return false };
    model::with_mut(&f.nodes[i].id, |m| m.step_link_focus(back))
}

/// The focus came to node `i` by Tab (`back`: Shift+Tab): the view's link
/// the keyboard starts at (the last, coming back; else the view itself).
pub(crate) fn entered(f: &FormUi, i: usize, back: bool) {
    if f.nodes[i].type_name == "RMARKDOWNVIEW" {
        model::with_mut(&f.nodes[i].id, |m| m.enter_link_focus(back));
    }
}

/// The view lost the focus: the keyboard's link goes (it starts at the view
/// again the next time).
pub(crate) fn blurred(id: &str) {
    model::with_mut(id, |m| m.link_focus = None);
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

    /// A code block's glyph runs: (their text, colour 0xRRGGBB), runs of one
    /// colour merged.
    fn code_inks(fence: &str, code: &str) -> (Vec<(String, u32)>, u32) {
        let theme = rapidr_value::theme::current();
        let f = super::Faces {
            body: rapidr_value::objects::font::Font { name: String::new(), size: -14, color: 0, styles: 0 },
            mono: rapidr_value::objects::font::Font { name: "JetBrains Mono".into(), size: -13, color: 0, styles: 0 },
            l: super::look(theme),
            theme,
            code_lang: None,
            added: 0,
            removed: 0,
        };
        let doc = model::Doc::parse(&format!("```{fence}\n{code}\n```\n"));
        let mut ts = TextSystem::new();
        let mut layout = super::shape(&mut ts, &doc.paras[0], &f, 1.0);
        layout.break_all_lines(None);
        let text = doc.paras[0].text.clone();
        let mut out: Vec<(String, u32)> = Vec::new();
        // (each character's colour: its cluster's glyphs' style)
        for line in layout.lines() {
            for run in line.runs() {
                for cluster in run.clusters() {
                    let Some(g) = cluster.glyphs().next() else { continue };
                    let (piece, ink) = (&text[cluster.text_range()], layout.styles()[g.style_index()].brush.0);
                    match out.last_mut() {
                        Some((t, c)) if *c == ink => t.push_str(piece),
                        _ => out.push((piece.to_string(), ink)),
                    }
                }
            }
        }
        (out, f.l.text)
    }

    #[test]
    fn sql_code_blocks_have_the_code_editors_colours() {
        let colour = |runs: &[(String, u32)], word: &str| runs.iter().find(|(t, _)| t.contains(word)).map(|r| r.1).unwrap_or_else(|| panic!("{word} in {runs:?}"));
        for fence in ["sql", "SQL", "sqlite", "mysql"] {
            let (runs, text) = code_inks(&fence.to_lowercase(), "SELECT name, 'Ada' FROM people WHERE id = 42 -- the first");
            let (kw, s, n, c) = (colour(&runs, "SELECT"), colour(&runs, "'Ada'"), colour(&runs, "42"), colour(&runs, "-- the first"));
            assert_ne!(kw, text, "{fence}: a keyword isn't the text's colour: {runs:?}");
            assert!(kw != s && s != c && kw != c && n != text, "{fence}: keywords, strings, numbers, comments apart: {runs:?}");
            assert_eq!(colour(&runs, "FROM"), kw);
        }
        // (and the editor's other languages; an unknown one stays plain)
        let (json, text) = code_inks("json", "{\"a\": 1}");
        assert!(json.len() > 1 && json.iter().any(|r| r.1 != text), "{json:?}");
        let (plain, text) = code_inks("cobol", "SELECT x FROM y");
        assert!(plain.iter().all(|r| r.1 == text), "{plain:?}");
        assert_eq!(super::code_language("sqlite"), Some("sql-sqlite"));
        assert_eq!(super::code_language("rr"), Some("rapidr-basic"));
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

    /// A form: a button, the view (the text with two links), a button.
    fn with_buttons(id: &str, text: &str) -> (MemStore, FormUi, TextSystem) {
        let mut s = MemStore::new();
        s.add("mform", "RFORM", None);
        s.add("before", "RBUTTON", Some("mform")).set("before", "width", v_int(80));
        s.add(id, "RMARKDOWNVIEW", Some("mform")).set(id, "top", v_int(40)).set(id, "width", v_int(500)).set(id, "height", v_int(200));
        s.add("after", "RBUTTON", Some("mform")).set("after", "top", v_int(250)).set("after", "width", v_int(80));
        model::remove(id);
        model::with_mut(id, |m| m.set_text(text));
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, "mform", false);
        f.paint(&s, &mut ts, 1.0);
        (s, f, ts)
    }

    fn pressed(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, vk: i64, shift: bool) -> Vec<User> {
        f.key_down(s, ts, vk, "", Mods { shift, ..Mods::NONE }, &mut MemClipboard::default());
        f.take_events()
            .into_iter()
            .filter_map(|e| match e {
                KernelEvent::Container(Container::Panel { action, .. }) => Some(action),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn tab_steps_through_the_links_with_a_ring_and_enter_follows_one() {
        let (s, mut f, mut ts) = with_buttons("mdk", TEXT);
        let kept = || model::with("mdk", |m| m.link_focus).flatten();
        assert!(f.focus_id(&s, "mdk"));
        assert_eq!(kept(), None, "the view itself first");
        let ring = |f: &mut FormUi, ts: &mut TextSystem| {
            let list = f.paint(&s, ts, 1.0);
            list.items.iter().filter(|i| matches!(i, crate::display::Item::Op { op: rapidr_value::objects::ops::Op::Focus { .. } | rapidr_value::objects::ops::Op::Round { fill: None, stroke: Some(_), .. }, .. })).count()
        };
        let before = ring(&mut f, &mut ts);
        // Tab: the first link, ringed, and named for a screen reader as the focused part
        assert!(pressed(&mut f, &s, &mut ts, 9, false).is_empty());
        assert_eq!((f.focused(), kept()), (Some("mdk"), Some(0)));
        assert!(ring(&mut f, &mut ts) > before, "a ring round the link");
        let tree = f.access_tree(&s, &mut ts);
        let doc = tree.children.iter().find(|n| n.role == Role::Document).expect("the view");
        assert!(doc.states.focused, "the view has the focus");
        let links: Vec<(&str, bool)> = doc.children[1].children.iter().map(|l| (l.name.as_str(), l.states.focused)).collect();
        assert_eq!(links, vec![("the docs", true), ("more", false)]);
        // Enter, and Space: the link is followed, as a click does
        let link = User::Markdown(model::User::Link("https://example.com".into()));
        assert_eq!(pressed(&mut f, &s, &mut ts, 13, false), vec![link.clone()]);
        assert_eq!(pressed(&mut f, &s, &mut ts, 32, false), vec![link]);
        // Tab: the second link; Tab: out of the view, to the next component
        pressed(&mut f, &s, &mut ts, 9, false);
        assert_eq!(kept(), Some(1));
        assert_eq!(pressed(&mut f, &s, &mut ts, 13, false), vec![User::Markdown(model::User::Link("#more".into()))]);
        pressed(&mut f, &s, &mut ts, 9, false);
        assert_eq!((f.focused(), kept()), (Some("after"), None));
        // Shift+Tab back: into the view at its last link, the first, the view, out
        pressed(&mut f, &s, &mut ts, 9, true);
        assert_eq!((f.focused(), kept()), (Some("mdk"), Some(1)));
        pressed(&mut f, &s, &mut ts, 9, true);
        assert_eq!(kept(), Some(0));
        pressed(&mut f, &s, &mut ts, 9, true);
        assert_eq!((f.focused(), kept()), (Some("mdk"), None));
        // (Space on the view itself still scrolls: no link to follow)
        assert!(pressed(&mut f, &s, &mut ts, 32, false).is_empty());
        pressed(&mut f, &s, &mut ts, 9, true);
        assert_eq!(f.focused(), Some("before"));
        // (the mouse takes the view: the keyboard's link goes)
        assert!(f.focus_id(&s, "mdk"));
        pressed(&mut f, &s, &mut ts, 9, false);
        assert_eq!(kept(), Some(0));
        f.mouse_down(&s, &mut ts, 300.0, 150.0, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, 300.0, 150.0, Button::Left, Mods::NONE);
        assert_eq!(kept(), None);
        model::remove("mdk");
    }

    #[test]
    fn the_view_scrolls_to_the_link_the_keyboard_has() {
        // (forty paragraphs, then a link: Tab shows it; Shift+Tab back to the view and out)
        let mut text = String::new();
        for i in 0..40 {
            text.push_str(&format!("paragraph {i}\n\n"));
        }
        text.push_str("last: [the end](ai:open/end)\n");
        let (s, mut f, mut ts) = with_buttons("mds", &text);
        assert!(f.focus_id(&s, "mds"));
        let top = super::vscroll_state("mds").0;
        assert_eq!(top, 0);
        pressed(&mut f, &s, &mut ts, 9, false);
        f.paint(&s, &mut ts, 1.0);
        let (pos, _, range) = super::vscroll_state("mds");
        assert!(pos > 800, "scrolled down to the link: {pos} of {range}");
        // (the link is inside the view: what a screen reader is told of where it is)
        let tree = f.access_tree(&s, &mut ts);
        let doc = tree.children.iter().find(|n| n.role == Role::Document).expect("the view");
        let link = doc.children.last().and_then(|p| p.children.first()).expect("the link");
        assert_eq!((link.name.as_str(), link.states.focused), ("the end", true));
        assert!(link.bounds.1 >= doc.bounds.1 && link.bounds.1 + link.bounds.3 <= doc.bounds.1 + doc.bounds.3, "{:?} in {:?}", link.bounds, doc.bounds);
        // (a click away and Tab back from the next component: its last link, shown)
        assert!(f.focus_id(&s, "after"));
        model::with_mut("mds", |_| ());
        f.mouse_wheel(&s, &mut ts, (50.0, 100.0), (0.0, -50.0), Mods::NONE);
        f.paint(&s, &mut ts, 1.0);
        assert_eq!(super::vscroll_state("mds").0, 0);
        pressed(&mut f, &s, &mut ts, 9, true);
        f.paint(&s, &mut ts, 1.0);
        assert!(super::vscroll_state("mds").0 > 0, "Shift+Tab into the view shows its last link");
        model::remove("mds");
    }

    /// The window point of the bytes `from..to` of paragraph `k` (its middle).
    fn word_point(f: &FormUi, id: &str, k: usize, from: usize, to: usize) -> (f64, f64) {
        use parley::{Affinity, Cursor, Selection};
        let n = f.node(id).expect("node");
        let m = n.ui.markdown.as_ref().expect("laid out");
        let l = m.layout(k).expect("layout");
        let sel = Selection::new(Cursor::from_byte_index(l, from, Affinity::Downstream), Cursor::from_byte_index(l, to, Affinity::Upstream));
        let geo = sel.geometry(l);
        let b = geo.first().expect("a box").0;
        let inset = super::common::inset(&super::look(rapidr_value::theme::current())) as f64;
        let (s, (ax, ay, _, _)) = (m.spots[k], n.abs);
        (ax as f64 + inset + s.x + (b.x0 + b.x1) / 2.0, ay as f64 + inset + s.y + (b.y0 + b.y1) / 2.0)
    }

    fn word_events(f: &mut FormUi) -> Vec<User> {
        f.take_events()
            .into_iter()
            .filter_map(|e| match e {
                KernelEvent::Container(Container::Panel { action, .. }) => Some(action),
                _ => None,
            })
            .collect()
    }

    /// A view with a CodeLanguage (the chat of RapidR Studio's assistant):
    /// its inline `code` is coloured as the code editor colours it, and a
    /// word of inline code or of a RapidR code block answers the mouse with
    /// the language service's one line (a tooltip), a hand, and a click.
    #[test]
    fn code_words_answer_the_mouse_when_the_view_has_a_code_language() {
        use rapidr_value::panels::markdown::WordHelp;
        model::set_word_help(|w, member| match (w.to_ascii_uppercase().as_str(), member) {
            ("PRINT", false) => Some(WordHelp { tip: "PRINT: writes text".into(), of: String::new() }),
            ("ADDITEMS", true) => Some(WordHelp { tip: "AddItems: adds lines".into(), of: "RListBox".into() }),
            ("SHOWMESSAGE", false) => Some(WordHelp { tip: "SHOWMESSAGE: a message box".into(), of: String::new() }),
            _ => None,
        });
        let (s, mut f, mut ts) = setup("md3");
        let text = "Use `PRINT` or `List1.AddItems` here.\n\n```rapidr\nSHOWMESSAGE \"hi\"\n```\n";
        model::with_mut("md3", |m| m.set_text(text));
        f.paint(&s, &mut ts, 1.0);
        let at = |w: &str| text.find(w).map(|i| i - text.find("Use").unwrap()).unwrap();
        let (print, add) = (at("PRINT"), at("AddItems"));
        // (no CodeLanguage: no tooltip, no click)
        let (x, y) = word_point(&f, "md3", 0, print, print + 5);
        f.mouse_move(&s, &mut ts, x, y, Mods::NONE);
        assert_eq!(f.tip.text, "");
        f.mouse_down(&s, &mut ts, x, y, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, x, y, Button::Left, Mods::NONE);
        assert!(word_events(&mut f).is_empty());
        // (with one)
        model::with_mut("md3", |m| {
            m.code_language = "rapidr".into();
            m.rev += 1;
        });
        f.paint(&s, &mut ts, 1.0);
        let (x, y) = word_point(&f, "md3", 0, print, print + 5);
        f.mouse_move(&s, &mut ts, x, y, Mods::NONE);
        assert_eq!(f.tip.text, "PRINT: writes text");
        assert_eq!(f.pointer_at(&s, &mut ts, x, y), rapidr_value::input::Cursor::Hand);
        f.mouse_down(&s, &mut ts, x, y, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, x, y, Button::Left, Mods::NONE);
        assert_eq!(word_events(&mut f), vec![User::Markdown(model::User::Word { word: "PRINT".into(), of: String::new() })]);
        // (a member after a dot: its component type goes with the click)
        let (x, y) = word_point(&f, "md3", 0, add, add + 8);
        f.mouse_move(&s, &mut ts, x, y, Mods::NONE);
        assert_eq!(f.tip.text, "AddItems: adds lines");
        f.mouse_down(&s, &mut ts, x, y, Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, x, y, Button::Left, Mods::NONE);
        assert_eq!(word_events(&mut f), vec![User::Markdown(model::User::Word { word: "AddItems".into(), of: "RListBox".into() })]);
        // (a word of a RapidR code block)
        let (x, y) = word_point(&f, "md3", 1, 0, 11);
        f.mouse_move(&s, &mut ts, x, y, Mods::NONE);
        assert_eq!(f.tip.text, "SHOWMESSAGE: a message box");
        // (plain text beside the code: nothing)
        let (x, y) = word_point(&f, "md3", 0, 0, 3);
        f.mouse_move(&s, &mut ts, x, y, Mods::NONE);
        assert_eq!(f.tip.text, "");
        assert_eq!(f.pointer_at(&s, &mut ts, x, y), rapidr_value::input::Cursor::Default);
        // (the inline code coloured as the editor colours its words)
        let theme = rapidr_value::theme::current();
        let faces = super::Faces {
            body: rapidr_value::objects::font::Font { name: String::new(), size: -14, color: 0, styles: 0 },
            mono: rapidr_value::objects::font::Font { name: "JetBrains Mono".into(), size: -13, color: 0, styles: 0 },
            l: super::look(theme),
            theme,
            code_lang: Some("rapidr-basic"),
            added: 0,
            removed: 0,
        };
        let doc = model::Doc::parse(text);
        let mut layout = super::shape(&mut ts, &doc.paras[0], &faces, 1.0);
        layout.break_all_lines(None);
        let ink_of = |from: usize| {
            for line in layout.lines() {
                for run in line.runs() {
                    for c in run.clusters() {
                        if c.text_range().start == from {
                            return c.glyphs().next().map(|g| layout.styles()[g.style_index()].brush.0).unwrap_or(0);
                        }
                    }
                }
            }
            0
        };
        assert_ne!(ink_of(print), faces.l.text, "PRINT is a keyword: coloured");
        assert_eq!(ink_of(0), faces.l.text, "the text around it is not");
        model::remove("md3");
    }

    /// FollowEnd: a chat at its end stays there while its text grows — also
    /// when the view is made shorter first (a strip appearing under it).
    #[test]
    fn a_view_at_its_end_follows_the_text_after_it_was_made_shorter() {
        let (mut s, mut f, mut ts) = setup("md4");
        let text = |n: usize| (0..n).map(|k| format!("line {k}\n\n")).collect::<String>();
        // (a chat that starts short: at its end, as one is)
        model::with_mut("md4", |m| {
            m.follow_end = true;
            m.set_text("hi");
        });
        f.paint(&s, &mut ts, 1.0);
        model::with_mut("md4", |m| m.set_text(&text(60)));
        f.paint(&s, &mut ts, 1.0);
        f.paint(&s, &mut ts, 1.0);
        let at_end = |id: &str| {
            let (pos, _, range) = crate::components::list::vscroll_state(id);
            (range - (pos + 300 - 38)).abs() < 80
        };
        assert!(at_end("md4"), "{:?}", crate::components::list::vscroll_state("md4"));
        // (shorter by 60: the strip; then the text grows, a screenful at once)
        s.set("md4", "height", v_int(240));
        f.paint(&s, &mut ts, 1.0);
        model::with_mut("md4", |m| m.set_text(&text(90)));
        f.paint(&s, &mut ts, 1.0);
        f.paint(&s, &mut ts, 1.0);
        let (pos, _, range) = crate::components::list::vscroll_state("md4");
        assert!(pos + 240 - 28 >= range - 60, "followed to the end: pos {pos} of {range}");
        model::remove("md4");
    }

    /// The counts' ink: the diff's green and red, readable (4.5:1, 7:1 in high
    /// contrast) on the view's ground in every look, the signs kept.
    #[test]
    fn added_and_removed_counts_are_readable_in_every_look() {
        for theme in rapidr_value::theme::ALL {
            let l = super::look(theme);
            let need = if theme.contrast { 7.0 } else { 4.5 };
            let scheme = rapidr_value::code_scheme::for_theme(theme);
            for (what, fg) in [("added", scheme.mark_added), ("removed", scheme.mark_removed)] {
                let ink = super::ink_for(fg, l.body, need);
                assert!(rapidr_value::theme::contrast(ink, l.body) >= need, "{} {what}: {ink:06X} on {:06X}", theme.name, l.body);
            }
            let (a, r) = (super::ink_for(scheme.mark_added, l.body, need), super::ink_for(scheme.mark_removed, l.body, need));
            assert_ne!(a, r, "{}: green and red differ", theme.name);
        }
    }
}
