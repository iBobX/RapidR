//! Drawing the code editor: the rows in view (each a cached parley layout
//! with the language's colours and the service's semantic colours), then
//! around and over them — the current line, rulers, indent guides, find's
//! matches, selections, the bracket pair, whitespace, squiggles, folded
//! lines' boxes, carets — the gutter, the minimap and the scroll bars.
//! Positions come from the layouts, snapped to device pixels.

use std::borrow::Cow;
use std::ops::Range;

use parley::StyleProperty;
use rapidr_editor::service::Severity;
use rapidr_editor::Buffer;
use rapidr_value::code_scheme::TokenStyle;
use rapidr_value::objects::ops::{Op, Place, Rect};

use rapidr_value::objects::codeedit::CodeEditor as Model;

use super::view::{display_cols, hash_of, x_of, Row};
use super::{with_view, CodeUi, Ctx};
use crate::components::Cx;
use crate::display::TextItem;
use crate::paint::Painter;
use crate::text::{styles, Ink};

/// A row in view this frame: where it is and how its text was laid out.
#[derive(Clone, Debug)]
pub struct Shown {
    pub index: usize,
    pub row: Row,
    /// Its layout's slot in the cache.
    pub slot: usize,
    /// The layout's (0, 0) in the window (device pixels).
    pub origin: (f64, f64),
    /// The row's top (the component's logical pixels).
    pub top: f64,
    pub text: RowText,
    /// The line's first byte in the document.
    pub line_start: usize,
}

/// A row's text as drawn: tabs as spaces to their stops, an input
/// method's composition inserted at the caret.
#[derive(Clone, Debug, Default)]
pub struct RowText {
    pub text: String,
    /// The row's bytes of its line.
    pub start: usize,
    pub end: usize,
    /// (byte of the line, extra display bytes, from that byte on (true) or
    /// after it (false)).
    shifts: Vec<(usize, usize, bool)>,
    /// The composition's display bytes.
    pub preedit: Option<Range<usize>>,
}

impl RowText {
    /// `line`'s bytes `range` (its display columns starting at
    /// `start_col`), tabs `tab` wide, a composition at byte `pre.0`.
    pub fn new(line: &str, range: Range<usize>, tab: usize, start_col: usize, pre: Option<(usize, &str)>) -> RowText {
        let piece = &line[range.clone()];
        let mut text = String::with_capacity(piece.len());
        let mut shifts = Vec::new();
        let mut col = start_col;
        let mut preedit = None;
        for (i, c) in piece.char_indices() {
            let at = range.start + i;
            if let Some((p, s)) = pre {
                if p == at && !s.is_empty() {
                    preedit = Some(text.len()..text.len() + s.len());
                    text.push_str(s);
                    shifts.push((at, s.len(), true));
                    col += display_cols(s, tab);
                }
            }
            if c == '\t' {
                let w = tab - col % tab;
                text.extend(std::iter::repeat_n(' ', w));
                shifts.push((at, w - 1, false));
                col += w;
            } else {
                text.push(c);
                col += if super::view::is_wide(c) { 2 } else { 1 };
            }
        }
        if let Some((p, s)) = pre {
            if p == range.end && !s.is_empty() && preedit.is_none() {
                preedit = Some(text.len()..text.len() + s.len());
                text.push_str(s);
                shifts.push((p, s.len(), true));
            }
        }
        RowText { text, start: range.start, end: range.end, shifts, preedit }
    }

    /// Byte `b` of the line as a display byte of the row.
    pub fn disp(&self, b: usize) -> usize {
        let b = b.clamp(self.start, self.end);
        let extra: usize = self.shifts.iter().filter(|(at, _, incl)| b > *at || (*incl && b >= *at)).map(|(_, n, _)| n).sum();
        b - self.start + extra
    }

    /// Display byte `d` back as a byte of the line (inside a tab or the
    /// composition: its start).
    pub fn src(&self, d: usize) -> usize {
        let mut best = self.start;
        let mut b = self.start;
        loop {
            if self.disp(b) > d {
                return best;
            }
            best = b;
            if b >= self.end {
                return b;
            }
            b += 1;
            while b < self.end && !self.is_boundary(b) {
                b += 1;
            }
        }
    }

    fn is_boundary(&self, b: usize) -> bool {
        // (the row's text has the line's characters, shifted)
        self.text.is_char_boundary(self.disp(b).min(self.text.len()))
    }
}

/// A rectangle in device pixels (absolute) drawn as a polygon in the
/// painter's logical pixels, on the pixel grid.
fn dev_fill(p: &mut Painter, (x0, y0, x1, y1): (f64, f64, f64, f64), color: u32) {
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let s = p.scale();
    let (ox, oy) = p.origin();
    let l = |v: f64, o: i64| v.round() / s - o as f64;
    let (a, b, c, d) = (l(x0, ox), l(y0, oy), l(x1, ox), l(y1, oy));
    p.op(Op::Polygon { points: vec![(a, b), (c, b), (c, d), (a, d)], color });
}

/// A rectangle's outline (device pixels, absolute), `w` device pixels.
fn dev_frame(p: &mut Painter, (x0, y0, x1, y1): (f64, f64, f64, f64), w: f64, color: u32) {
    dev_fill(p, (x0, y0, x1, y0 + w), color);
    dev_fill(p, (x0, y1 - w, x1, y1), color);
    dev_fill(p, (x0, y0, x0 + w, y1), color);
    dev_fill(p, (x1 - w, y0, x1, y1), color);
}

/// `fg` over `bg` at `a` (0..1).
pub fn blend(fg: u32, bg: u32, a: f64) -> u32 {
    let ch = |c: u32, s: u32| (f64::from((c >> s) & 0xFF) * a + f64::from((bg >> s) & 0xFF) * (1.0 - a)).round() as u32;
    (ch(fg, 16) << 16) | (ch(fg, 8) << 8) | ch(fg, 0)
}

/// The parley styles of a run of `st`.
fn run_props(st: TokenStyle) -> Vec<StyleProperty<'static, Ink>> {
    let mut out = vec![StyleProperty::Brush(Ink(st.color))];
    if st.bold {
        out.push(StyleProperty::FontWeight(parley::FontWeight::BOLD));
    }
    if st.italic {
        out.push(StyleProperty::FontStyle(parley::FontStyle::Italic));
    }
    out
}

/// A row's coloured runs (display bytes): the language's tokens, then the
/// language service's names over them.
fn row_runs(x: &mut Ctx, line: usize, line_start: usize, rt: &RowText, highlighted: bool) -> Vec<(Range<usize>, TokenStyle)> {
    let mut runs = Vec::new();
    if highlighted {
        for t in x.c.doc.tokens(line) {
            let (a, b) = (t.start as usize, t.end as usize);
            if b <= rt.start || a >= rt.end || t.kind == rapidr_editor::TokenKind::TEXT {
                continue;
            }
            let st = x.scheme.token(t.kind.fallbacks());
            if st.color == x.scheme.text && !st.bold && !st.italic {
                continue;
            }
            runs.push((rt.disp(a)..rt.disp(b), st));
        }
    }
    let line_end = line_start + rt.end;
    for s in &x.c.semantic {
        if s.end <= line_start + rt.start || s.start >= line_end {
            continue;
        }
        let st = x.scheme.token_named(s.kind);
        let (a, b) = (s.start.saturating_sub(line_start).max(rt.start), (s.end - line_start).min(rt.end));
        runs.push((rt.disp(a)..rt.disp(b), st));
    }
    runs
}

/// Lays out (or finds) row `row`'s text: its cache slot and its height
/// (device pixels).
fn lay_out(x: &mut Ctx, rt: &RowText, runs: &[(Range<usize>, TokenStyle)]) -> usize {
    let key = hash_of((&rt.text, runs.iter().map(|(r, s)| (r.start, r.end, s.color, s.bold, s.italic)).collect::<Vec<_>>(), &x.ui.font.name, x.ui.font.size, x.ui.cache_key.0, x.scheme.text));
    let font = x.ui.font.clone();
    let color = x.scheme.text;
    let scale = x.scale as f32;
    let ts = &mut *x.ts;
    x.ui.cache.get_or_make(key, || {
        let shown: &str = &rt.text;
        let mut b = ts.layout_cx.ranged_builder(&mut ts.font_cx, shown, scale, true);
        for prop in styles(&font, color) {
            b.push_default(prop);
        }
        for (r, st) in runs {
            let r = r.start.min(shown.len())..r.end.min(shown.len());
            if r.is_empty() || !shown.is_char_boundary(r.start) || !shown.is_char_boundary(r.end) {
                continue;
            }
            for prop in run_props(*st) {
                b.push(prop, r.clone());
            }
        }
        let mut layout = b.build(shown);
        layout.break_all_lines(None);
        layout.align(parley::Alignment::Left, parley::AlignmentOptions::default());
        crate::text::note_missing(&layout, shown);
        layout
    })
}

/// Lines a frame colours past what's coloured (a jump far down shows its
/// lines plain until idle colouring reaches them).
const COLOUR_AHEAD: usize = 4000;

/// The rows in view laid out (`ui.shown`), positions in device pixels of
/// the window whose painter `p` is.
pub fn layout_rows(x: &mut Ctx, origin: (i64, i64)) {
    let lh = x.ui.lh();
    let g = x.ui.geo;
    let s = x.scale;
    let first = (x.ui.scroll.1 / lh).floor().max(0.0) as usize;
    let rows = x.ui.rows.rows(x.c, first, x.ui.page_rows() + 1);
    x.ui.cache.begin_frame();
    let tab = x.c.doc.tab_size as usize;
    let valid = x.c.doc.highlighter().valid_lines();
    let head = x.c.doc.selections().primary().head;
    let pre = x.ui.preedit.clone();
    let mut shown = Vec::with_capacity(rows.len());
    let mut behind = false;
    for (k, row) in rows.into_iter().enumerate() {
        let index = first + k;
        let line_start = x.c.doc.buffer().line_start(row.line);
        let line: Cow<str> = x.c.doc.line(row.line);
        let start_col = display_cols(&line[..row.range.start], tab);
        let pre_here = pre.as_ref().filter(|_| {
            let (r, piece) = x.ui.rows.row_at(x.c, head);
            r == index && piece.line == row.line
        });
        let rt = RowText::new(&line, row.range.clone(), tab, start_col, pre_here.map(|(t, _)| (head - line_start, t.as_str())));
        drop(line);
        let highlighted = row.line <= valid + COLOUR_AHEAD;
        behind |= !highlighted;
        let runs = row_runs(x, row.line, line_start, &rt, highlighted);
        let slot = lay_out(x, &rt, &runs);
        let lay_h = x.ui.cache.layout(slot).map_or(lh * s, |l| f64::from(l.height()));
        let top = g.text.1 as f64 + index as f64 * lh - x.ui.scroll.1;
        let dev_top = ((origin.1 as f64 + top) * s).round();
        let oy = dev_top + ((lh * s - lay_h) / 2.0).round();
        let ox = ((origin.0 + g.text.0) as f64 * s).round() - (x.ui.scroll.0 * s).round();
        shown.push(Shown { index, row, slot, origin: (ox, oy), top, text: rt, line_start });
    }
    if behind {
        x.ui.idle_at.get_or_insert_with(crate::tick::now);
    }
    x.ui.shown = shown;
}

/// The device x (absolute) of byte `at` of the document in row `sh`.
pub fn caret_x(ui: &CodeUi, sh: &Shown, at: usize) -> f64 {
    let Some(layout) = ui.cache.layout(sh.slot) else { return sh.origin.0 };
    sh.origin.0 + x_of(layout, sh.text.disp(at.saturating_sub(sh.line_start)))
}

/// Which shown row holds byte `at` (a wrapped line: the piece it's in).
pub fn shown_at<'a>(ui: &'a CodeUi, c: &Model, at: usize) -> Option<&'a Shown> {
    let buf = c.doc.buffer();
    let at = at.min(buf.len_bytes());
    let line = buf.line_of(at);
    let col = at - buf.line_start(line);
    let pieces: Vec<&Shown> = ui.shown.iter().filter(|s| s.row.line == line).collect();
    let n = pieces.len();
    pieces.iter().enumerate().find(|(i, s)| col < s.row.range.end || (*i + 1 == n && col >= s.row.range.start)).map(|(_, s)| *s).or_else(|| pieces.first().copied())
}

pub fn paint(cx: &mut Cx, p: &mut Painter) {
    let (w, h) = (cx.width(), cx.height());
    let caret_on = cx.state.caret_on;
    with_view(cx, |x| paint_view(x, p, w, h, caret_on));
    super::input::schedule(cx);
}

fn paint_view(x: &mut Ctx, p: &mut Painter, w: i64, h: i64, caret_on: bool) {
    let sc = x.scheme;
    let t = rapidr_value::theme::current();
    // (the frame: the classic look's sunken edge, a flat line otherwise)
    p.fill((0, 0, w, h), sc.background);
    if t.fluent() {
        p.op(Op::Fill { rect: (0, 0, w, 1), color: t.border });
        p.op(Op::Fill { rect: (0, h - 1, w, 1), color: t.border });
        p.op(Op::Fill { rect: (0, 0, 1, h), color: t.border });
        p.op(Op::Fill { rect: (w - 1, 0, 1, h), color: t.border });
    } else {
        p.sunken_edge((0, 0, w, h));
    }
    let origin = p.origin();
    layout_rows(x, origin);
    let g = x.ui.geo;
    p.clipped(g.text, |p| paint_text(x, p, caret_on));
    paint_gutter(x, p);
    if let Some(m) = g.minimap {
        paint_minimap(x, p, m);
    }
    let (bx, by, bw, bh) = g.bars;
    if x.ui.bars.vert.shown || x.ui.bars.horz.shown {
        let ops = crate::paint::bar_ops(&x.ui.bars, bw, bh);
        p.at((bx, by), |p| p.clipped((0, 0, bw, bh), |p| p.ops(ops)));
        paint_overview(x, p);
    }
    super::find::paint(x, p);
}

/// The text's view: everything under, in and over the rows.
fn paint_text(x: &mut Ctx, p: &mut Painter, caret_on: bool) {
    let sc = x.scheme;
    let s = x.scale;
    let lh = x.ui.lh();
    let g = x.ui.geo;
    let (ox, oy) = p.origin();
    let dev_l = ((ox + g.text.0) as f64 * s).round();
    let dev_r = ((ox + g.text.0 + g.text.2) as f64 * s).round();
    let row_dev = |top: f64| (((oy as f64) + top) * s).round();
    let sels: Vec<rapidr_editor::Selection> = x.c.doc.selections().ranges().to_vec();
    let primary = x.c.doc.selections().primary();
    let head_line = x.c.doc.buffer().line_of(primary.head);
    let shown = x.ui.shown.clone();
    let ch_dev = x.ui.metrics.ch * s;
    let text_x0 = ((ox + g.text.0) as f64 * s).round() - (x.ui.scroll.0 * s).round();

    // the current line
    if x.c.opts.highlight_current_line && sels.iter().all(|r| r.is_empty()) {
        for sh in shown.iter().filter(|sh| sh.row.line == head_line) {
            let (y0, y1) = (row_dev(sh.top), row_dev(sh.top + lh));
            if sc.current_line_frame {
                dev_frame(p, (dev_l, y0, dev_r, y1), s.round().max(1.0), sc.current_line);
            } else {
                dev_fill(p, (dev_l, y0, dev_r, y1), sc.current_line);
            }
        }
    }
    // rulers
    for &col in &x.c.opts.rulers {
        let rx = (text_x0 + f64::from(col) * ch_dev).round();
        dev_fill(p, (rx, row_dev(g.text.1 as f64), rx + s.round().max(1.0), row_dev((g.text.1 + g.text.3) as f64)), sc.ruler);
    }
    // indent guides
    let tab = x.c.doc.tab_size as usize;
    for sh in shown.iter().filter(|sh| sh.row.wrap_index == 0) {
        let line = x.c.doc.line(sh.row.line);
        let lead = if line.trim().is_empty() { indent_of_next(x, sh.row.line) } else { display_cols(&line[..line.len() - line.trim_start().len()], tab) };
        let unit = if x.c.doc.insert_spaces { x.c.doc.tab_size as usize } else { tab };
        let mut col = unit;
        while col < lead {
            let gx = (text_x0 + col as f64 * ch_dev).round();
            dev_fill(p, (gx, row_dev(sh.top), gx + s.round().max(1.0), row_dev(sh.top + lh)), sc.indent_guide);
            col += unit;
        }
    }
    // find's matches, and the selected word's other places
    let visible = shown.first().map(|f| f.line_start + f.row.range.start).unwrap_or(0)..shown.last().map(|l| l.line_start + l.row.range.end).unwrap_or(0);
    let matches = super::find::matches_in(x, visible.clone());
    for (r, current) in matches {
        fill_range(x, p, &shown, r, if current { sc.find_current } else { sc.find_match }, lh);
    }
    for r in echo_ranges(x, visible.clone()) {
        fill_range(x, p, &shown, r, sc.selection_echo, lh);
    }
    // selections
    let sel_color = if x.focused { sc.selection } else { sc.selection_inactive };
    for r in &sels {
        if !r.is_empty() {
            fill_range(x, p, &shown, r.range(), sel_color, lh);
        }
    }
    // the bracket pair at the caret
    if x.focused && sels.len() == 1 {
        if let Some((a, b)) = x.c.doc.matching_bracket(primary.head) {
            for at in [a, b] {
                if let Some(sh) = shown_at(x.ui, x.c, at) {
                    let x0 = caret_x(x.ui, sh, at);
                    let len = x.c.doc.buffer().char_after(at).map_or(1, char::len_utf8);
                    let x1 = caret_x(x.ui, sh, at + len).max(x0 + ch_dev);
                    let (y0, y1) = (row_dev(sh.top), row_dev(sh.top + lh));
                    dev_fill(p, (x0, y0, x1, y1), sc.bracket);
                    dev_frame(p, (x0, y0, x1, y1), s.round().max(1.0), sc.bracket_frame);
                }
            }
        }
    }
    // the rows' text
    for sh in &shown {
        let mut item = TextItem { node: x.id.to_string(), para: sh.slot, origin: sh.origin, selection: Vec::new(), highlight: 0, highlight_text: 0, underlines: Vec::new(), caret: None, caret_color: sc.caret };
        if let (Some(pr), Some(layout)) = (&sh.text.preedit, x.ui.cache.layout(sh.slot)) {
            let (a, b) = (x_of(layout, pr.start), x_of(layout, pr.end));
            let lay_h = f64::from(layout.height());
            item.underlines.push((a, lay_h - s.round().max(1.0), b, lay_h));
        }
        p.editor(item);
    }
    // whitespace
    if x.c.opts.show_whitespace {
        for sh in &shown {
            let Some(layout) = x.ui.cache.layout(sh.slot) else { continue };
            let line = x.c.doc.line(sh.row.line).into_owned();
            let (y0, y1) = (row_dev(sh.top), row_dev(sh.top + lh));
            let mid = ((y0 + y1) / 2.0).round();
            for (i, c) in line[sh.row.range.clone()].char_indices() {
                let at = sh.row.range.start + i;
                let (a, b) = (sh.origin.0 + x_of(layout, sh.text.disp(at)), sh.origin.0 + x_of(layout, sh.text.disp(at + 1)));
                match c {
                    ' ' => {
                        let cxp = ((a + b) / 2.0).round();
                        let d = (s * 1.0).round().max(1.0);
                        dev_fill(p, (cxp - d / 2.0, mid - d / 2.0, cxp + d / 2.0, mid + d / 2.0), sc.whitespace);
                    }
                    '\t' => {
                        let (lx, rx) = (a + 2.0 * s, b - 2.0 * s);
                        let to_l = |v: f64, o: i64| v / s - o as f64;
                        let yl = to_l(mid, oy);
                        p.stroke(&[(to_l(lx, ox), yl), (to_l(rx, ox), yl)], sc.whitespace, 1.0);
                        p.stroke(&[(to_l(rx, ox) - 3.0, yl - 3.0), (to_l(rx, ox), yl), (to_l(rx, ox) - 3.0, yl + 3.0)], sc.whitespace, 1.0);
                    }
                    _ => {}
                }
            }
        }
    }
    // squiggles
    let diags: Vec<(usize, usize, Severity)> = x.c.diagnostics.iter().filter(|d| d.end >= visible.start && d.start <= visible.end).map(|d| (d.start, d.end, d.severity)).collect();
    for (a, b, sev) in diags {
        let color = match sev {
            Severity::Error => sc.error,
            Severity::Warning => sc.warning,
            Severity::Info => sc.info,
            Severity::Hint => sc.hint,
        };
        squiggle(x, p, &shown, a, b, color, sev == Severity::Hint, lh);
    }
    // folded lines' "…" boxes
    let folded: Vec<usize> = x.c.folded.iter().copied().collect();
    for sh in shown.iter().filter(|sh| folded.contains(&sh.row.line)) {
        let last = shown.iter().rfind(|o| o.row.line == sh.row.line).map_or(sh.index, |o| o.index);
        if sh.index != last {
            continue;
        }
        let end = sh.line_start + sh.row.range.end;
        let x0 = caret_x(x.ui, sh, end) + 6.0 * s;
        let (y0, y1) = (row_dev(sh.top) + 3.0 * s, row_dev(sh.top + lh) - 3.0 * s);
        let w = (22.0 * s).round();
        let lx = |v: f64| v / s - ox as f64;
        let ly = |v: f64| v / s - oy as f64;
        p.op(Op::Round { rect: (lx(x0).round() as i64, ly(y0).round() as i64, (w / s).round() as i64, ((y1 - y0) / s).round() as i64), radius: 3.0, fill: Some(sc.fold_box), stroke: None, width: 1.0 });
        for k in 0..3 {
            let dx = lx(x0) + 6.0 + k as f64 * 5.0;
            let dy = ly((y0 + y1) / 2.0);
            p.op(Op::Round { rect: (dx.round() as i64, (dy - 1.0).round() as i64, 2, 2), radius: 1.0, fill: Some(sc.fold_arrow), stroke: None, width: 1.0 });
        }
    }
    // the carets (and a snippet's next stops, outlined)
    if x.focused && caret_on && x.ui.find.as_ref().is_none_or(|f| !f.focused) {
        let cw = (1.5 * s).round().max(2.0);
        for r in &sels {
            let at = r.head;
            let pre_extra = if at == primary.head { x.ui.preedit.as_ref().and_then(|(t, c)| c.map(|c| (t.clone(), c.1))) } else { None };
            if let Some(sh) = shown_at(x.ui, x.c, at) {
                let mut cxp = caret_x(x.ui, sh, at);
                if let (Some((_, cur)), Some(pr), Some(layout)) = (pre_extra, &sh.text.preedit, x.ui.cache.layout(sh.slot)) {
                    cxp = sh.origin.0 + x_of(layout, pr.start + cur);
                }
                let (y0, y1) = (row_dev(sh.top), row_dev(sh.top + lh));
                dev_fill(p, (cxp.round(), y0, cxp.round() + cw, y1), sc.caret);
            }
        }
    }
    if let Some(sn) = &x.ui.snippet {
        for (i, &(a, b)) in sn.stops.iter().enumerate() {
            if i == sn.current {
                continue;
            }
            if let Some(sh) = shown_at(x.ui, x.c, a) {
                let (x0, x1) = (caret_x(x.ui, sh, a), caret_x(x.ui, sh, b.max(a)).max(caret_x(x.ui, sh, a) + 2.0 * s));
                dev_frame(p, (x0, row_dev(sh.top), x1, row_dev(sh.top + lh)), 1.0, sc.bracket_frame);
            }
        }
    }
}

/// The indentation an empty line shows guides for: the next non-empty
/// line's (a few lines on at most).
fn indent_of_next(x: &Ctx, line: usize) -> usize {
    let tab = x.c.doc.tab_size as usize;
    let n = x.c.doc.line_count();
    for l in line + 1..(line + 40).min(n) {
        let t = x.c.doc.line(l);
        if !t.trim().is_empty() {
            return display_cols(&t[..t.len() - t.trim_start().len()], tab);
        }
    }
    0
}

/// Fills bytes `r` of the document across the shown rows (a range past a
/// row's end goes on a little, as a line break's share).
fn fill_range(x: &Ctx, p: &mut Painter, shown: &[Shown], r: Range<usize>, color: u32, lh: f64) {
    let s = x.scale;
    let oy = p.origin().1 as f64;
    for sh in shown {
        let (rs, re) = (sh.line_start + sh.row.range.start, sh.line_start + sh.row.range.end);
        let line_end = sh.line_start + x.c.doc.line(sh.row.line).len();
        if r.end < rs || r.start > re || (r.start == r.end && r.start != rs) {
            continue;
        }
        if r.end == rs && r.start < rs {
            // (ends at this row's start: nothing on it)
            continue;
        }
        let a = r.start.max(rs);
        let b = r.end.min(re);
        let x0 = caret_x(x.ui, sh, a);
        let mut x1 = caret_x(x.ui, sh, b);
        if r.end > re && re == line_end {
            x1 += (x.ui.metrics.ch * 0.6 * s).round();
        }
        let y0 = ((oy + sh.top) * s).round();
        let y1 = ((oy + sh.top + lh) * s).round();
        dev_fill(p, (x0, y0, x1, y1), color);
    }
}

/// A wavy line under bytes `a..b` (dots under the start for a hint).
#[allow(clippy::too_many_arguments)]
fn squiggle(x: &Ctx, p: &mut Painter, shown: &[Shown], a: usize, b: usize, color: u32, dots: bool, lh: f64) {
    let s = x.scale;
    let (ox, oy) = (p.origin().0 as f64, p.origin().1 as f64);
    for sh in shown {
        let (rs, re) = (sh.line_start + sh.row.range.start, sh.line_start + sh.row.range.end);
        if b < rs || a > re {
            continue;
        }
        let x0 = caret_x(x.ui, sh, a.max(rs));
        let mut x1 = caret_x(x.ui, sh, b.min(re));
        if x1 - x0 < x.ui.metrics.ch * s * 0.8 {
            x1 = x0 + x.ui.metrics.ch * s;
        }
        let base = sh.top + lh - 2.5;
        let (l0, l1) = (x0 / s - ox, x1 / s - ox);
        if dots {
            for k in 0..3 {
                let dx = l0 + k as f64 * 2.0;
                p.op(Op::Round { rect: (dx.round() as i64, (base + 1.0).round() as i64, 1, 1), radius: 0.5, fill: Some(color), stroke: None, width: 1.0 });
            }
            continue;
        }
        let mut pts = Vec::new();
        let mut xx = l0;
        let mut up = true;
        while xx < l1 {
            pts.push((xx, base + if up { -1.0 } else { 1.0 }));
            xx += 2.0;
            up = !up;
        }
        pts.push((l1, base + if up { -1.0 } else { 1.0 }));
        let _ = oy;
        p.stroke(&pts, color, 1.0);
    }
}

/// The other places of the selected word in `visible` (Ctrl+D's next
/// candidates).
fn echo_ranges(x: &Ctx, visible: Range<usize>) -> Vec<Range<usize>> {
    let sels = x.c.doc.selections();
    if sels.len() != 1 {
        return Vec::new();
    }
    let p = sels.primary();
    if p.is_empty() || p.end() - p.start() > 100 {
        return Vec::new();
    }
    let word = x.c.doc.slice(p.range()).into_owned();
    if word.contains('\n') || !word.chars().all(|c| x.c.doc.language().is_word_char(c)) {
        return Vec::new();
    }
    let text = x.c.doc.slice(visible.clone());
    let lang = x.c.doc.language();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = text[from..].find(&word) {
        let a = from + i;
        let b = a + word.len();
        let before_ok = text[..a].chars().next_back().is_none_or(|c| !lang.is_word_char(c));
        let after_ok = text[b..].chars().next().is_none_or(|c| !lang.is_word_char(c));
        if before_ok && after_ok && visible.start + a != p.start() {
            out.push(visible.start + a..visible.start + b);
        }
        from = b;
    }
    out
}

/// The gutter: line numbers, markers, diagnostics' icons, the changes
/// since saving, fold arrows.
fn paint_gutter(x: &mut Ctx, p: &mut Painter) {
    let sc = x.scheme;
    let g = x.ui.geo;
    let lh = x.ui.lh();
    let t = rapidr_value::theme::current();
    p.fill(g.gutter, sc.gutter);
    if !t.fluent() {
        p.fill((g.gutter.0 + g.gutter.2 - 1, g.gutter.1, 1, g.gutter.3), sc.whitespace);
    }
    let shown = x.ui.shown.clone();
    let head_line = x.c.doc.buffer().line_of(x.c.doc.selections().primary().head);
    let font = x.ui.font.clone();
    let num_h = rapidr_value::objects::text::text_size("0", &font).1;
    let (folds, _) = x.c.cached_fold_ranges();
    let fold_starts: Vec<usize> = folds.iter().map(|f| f.start_line).collect();
    let folded = x.c.folded.clone();
    let show_arrows = x.ui.gutter_hover || x.focused;
    p.clipped(g.gutter, |p| {
        for sh in shown.iter().filter(|sh| sh.row.wrap_index == 0) {
            let line = sh.row.line;
            let top = sh.top.round() as i64;
            if x.c.opts.show_line_numbers {
                let color = if line == head_line { sc.line_number_active } else { sc.line_number };
                let ny = top + (lh as i64 - num_h) / 2;
                p.text((g.glyph_x, ny, g.numbers_right - g.glyph_x, num_h), &(line + 1).to_string(), &font, color, Place::TopRight);
            }
            // markers (the strongest first), else the line's worst problem
            let kinds: Vec<String> = x.c.markers_on(line).map(|m| m.kind.clone()).collect();
            let icon_rect = (g.glyph_x + 1, top + (lh as i64 - 16) / 2, 16, 16);
            let marker = ["current", "breakpoint", "bookmark", "error", "warning"].into_iter().find(|k| kinds.iter().any(|m| m == k)).map(str::to_string).or_else(|| kinds.first().cloned());
            match marker.as_deref() {
                Some("breakpoint") => {
                    p.op(Op::Round { rect: (icon_rect.0 + 2, icon_rect.1 + 2, 12, 12), radius: 6.0, fill: Some(sc.breakpoint), stroke: None, width: 1.0 });
                    if kinds.iter().any(|k| k == "current") {
                        arrow(p, icon_rect, sc.current_statement);
                    }
                }
                Some("current") => arrow(p, icon_rect, sc.current_statement),
                Some("bookmark") => {
                    let (bx, by) = (icon_rect.0 as f64 + 4.0, icon_rect.1 as f64 + 2.0);
                    p.op(Op::Polygon { points: vec![(bx, by), (bx + 8.0, by), (bx + 8.0, by + 12.0), (bx + 4.0, by + 9.0), (bx, by + 12.0)], color: sc.bookmark });
                }
                Some("error") => {
                    p.icon("actions/error", icon_rect, Some(sc.error), false);
                }
                Some("warning") => {
                    p.icon("actions/warning", icon_rect, Some(sc.warning), false);
                }
                Some(_) => {
                    let (cxp, cyp) = (icon_rect.0 as f64 + 8.0, icon_rect.1 as f64 + 8.0);
                    p.op(Op::Polygon { points: vec![(cxp, cyp - 5.0), (cxp + 5.0, cyp), (cxp, cyp + 5.0), (cxp - 5.0, cyp)], color: sc.info });
                }
                None => match x.c.line_severity(line) {
                    Some(Severity::Error) => {
                        p.icon("actions/error", icon_rect, Some(sc.error), false);
                    }
                    Some(Severity::Warning) => {
                        p.icon("actions/warning", icon_rect, Some(sc.warning), false);
                    }
                    _ => {}
                },
            }
            if line == head_line && kinds.iter().any(|k| k == "current") {
                // (the debugger's line: tinted across the text too — paint_text drew it under)
            }
            // changes since the save
            match x.ui.changes.at(line) {
                super::lang::Change::Added => p.fill((g.diff_x, top, 3, lh as i64), sc.mark_added),
                super::lang::Change::Modified => p.fill((g.diff_x, top, 3, lh as i64), sc.mark_changed),
                super::lang::Change::RemovedAfter => {
                    let y = (top + lh as i64) as f64;
                    p.op(Op::Polygon { points: vec![(g.diff_x as f64, y - 4.0), (g.diff_x as f64 + 5.0, y), (g.diff_x as f64, y + 4.0)], color: sc.mark_removed });
                }
                super::lang::Change::None => {}
            }
            // fold arrows
            if x.c.opts.show_folding && fold_starts.contains(&line) {
                let is_folded = folded.contains(&line);
                if is_folded || show_arrows {
                    let (cxp, cyp) = (g.fold_x as f64 + 7.5, top as f64 + lh / 2.0);
                    p.chevron(cxp, cyp, 8.0, !is_folded, if is_folded { sc.line_number_active } else { sc.fold_arrow });
                }
            }
        }
    });
}

/// The debugger's execution point: an arrow in the glyph column.
fn arrow(p: &mut Painter, r: Rect, color: u32) {
    let (x0, y0) = (r.0 as f64 + 1.0, r.1 as f64 + 3.0);
    p.op(Op::Polygon { points: vec![(x0, y0 + 3.0), (x0 + 7.0, y0 + 3.0), (x0 + 7.0, y0), (x0 + 14.0, y0 + 5.0), (x0 + 7.0, y0 + 10.0), (x0 + 7.0, y0 + 7.0), (x0, y0 + 7.0)], color });
}

/// Rows the minimap shows (two pixels each) from which row.
pub fn minimap_first(x: &Ctx, m: Rect) -> usize {
    let total = x.ui.rows.total();
    let fit = (m.3 / 2).max(1) as usize;
    if total <= fit {
        return 0;
    }
    let page = x.ui.full_rows();
    let first_view = (x.ui.scroll.1 / x.ui.lh()).floor() as usize;
    let span = total.saturating_sub(page).max(1);
    ((first_view as f64 / span as f64) * (total - fit) as f64).round() as usize
}

/// The minimap: each row's tokens as two-pixel bars, a slider over what
/// the view shows.
fn paint_minimap(x: &mut Ctx, p: &mut Painter, m: Rect) {
    let sc = x.scheme;
    p.fill(m, sc.background);
    let first = minimap_first(x, m);
    let fit = (m.3 / 2).max(1) as usize;
    let rows = x.ui.rows.rows(x.c, first, fit);
    let tab = x.c.doc.tab_size as usize;
    let valid = x.c.doc.highlighter().valid_lines();
    p.clipped(m, |p| {
        for (k, row) in rows.iter().enumerate() {
            let y = m.1 + k as i64 * 2;
            let line = x.c.doc.line(row.line).into_owned();
            let piece = &line[row.range.clone()];
            let base_col = display_cols(&line[..row.range.start], tab);
            let tokens = if row.line <= valid + COLOUR_AHEAD { x.c.doc.tokens(row.line) } else { Vec::new() };
            // (every non-blank run, coloured by its token)
            let mut col = 0usize;
            let mut run_start: Option<(usize, u32)> = None;
            let color_at = |b: usize| -> u32 {
                let at = row.range.start + b;
                tokens.iter().find(|t| (t.start as usize) <= at && at < t.end as usize).map_or(sc.text, |t| sc.token(t.kind.fallbacks()).color)
            };
            let flush = |p: &mut Painter, start: usize, end: usize, c: u32| {
                let x0 = m.0 + 2 + (start.saturating_sub(base_col)) as i64;
                let w = (end - start) as i64;
                if x0 < m.0 + m.2 {
                    p.fill((x0, y, w.min(m.0 + m.2 - x0), 1), blend(c, sc.background, 0.75));
                }
            };
            for (b, ch) in piece.char_indices() {
                let w = if ch == '\t' { tab - (base_col + col) % tab } else { 1 };
                if ch.is_whitespace() {
                    if let Some((s0, c)) = run_start.take() {
                        flush(p, base_col + s0, base_col + col, c);
                    }
                } else {
                    let c = color_at(b);
                    match run_start {
                        Some((s0, c0)) if c0 != c => {
                            flush(p, base_col + s0, base_col + col, c0);
                            run_start = Some((col, c));
                        }
                        None => run_start = Some((col, c)),
                        _ => {}
                    }
                }
                col += w;
            }
            if let Some((s0, c)) = run_start {
                flush(p, base_col + s0, base_col + col, c);
            }
        }
        // the slider
        let first_view = (x.ui.scroll.1 / x.ui.lh()).floor() as usize;
        let y = m.1 + (first_view.saturating_sub(first) * 2) as i64;
        let h = (x.ui.full_rows() * 2) as i64;
        p.op(Op::Polygon { points: vec![(m.0 as f64, y as f64), ((m.0 + m.2) as f64, y as f64), ((m.0 + m.2) as f64, (y + h) as f64), (m.0 as f64, (y + h) as f64)], color: blend(sc.minimap_slider, sc.background, 0.55) });
    });
    // (a line between it and the text)
    p.fill((m.0, m.1, 1, m.3), sc.indent_guide);
}

/// Marks on the vertical scroll bar: problems, find's matches, the carets.
fn paint_overview(x: &mut Ctx, p: &mut Painter) {
    let sc = x.scheme;
    let g = x.ui.geo;
    let Some((bx, by, bw, bh)) = x.ui.bars.bar(true, g.bars.2, g.bars.3) else { return };
    let (bx, by) = (g.bars.0 + bx, g.bars.1 + by);
    let track = (by + 17, (bh - 34).max(1));
    let total = x.ui.rows.total().max(1) as f64;
    let at_row = |row: usize| track.0 + ((row as f64 / total) * track.1 as f64) as i64;
    let mut marks: Vec<(usize, u32, i64)> = Vec::new();
    for d in x.c.diagnostics.iter().take(2000) {
        let (row, _) = x.ui.rows.row_at(x.c, d.start);
        let c = match d.severity {
            Severity::Error => sc.error,
            Severity::Warning => sc.warning,
            _ => continue,
        };
        marks.push((row, c, 1));
    }
    for r in super::find::all_matches(x).iter().take(2000) {
        let (row, _) = x.ui.rows.row_at(x.c, r.start);
        marks.push((row, sc.find_current, 0));
    }
    let head = x.c.doc.selections().primary().head;
    let (row, _) = x.ui.rows.row_at(x.c, head);
    marks.push((row, sc.caret, 2));
    for (row, c, lane) in marks {
        let y = at_row(row);
        let (x0, w) = match lane {
            0 => (bx + 3, 5),
            1 => (bx + bw - 6, 5),
            _ => (bx + 2, bw - 4),
        };
        p.fill((x0, y, w, 2), c);
    }
}
