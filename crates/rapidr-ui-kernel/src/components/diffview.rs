//! RDIFFVIEW (I2): two texts' differences, hunk by hunk, each accepted or
//! rejected — the shared model (`rapidr_value::objects::diffview`: the
//! diff, the hunks' states, the rows, the scroll bar, hit tests) drawn and
//! driven by the kernel, the same on the desktop and the web.
//!
//! - **The look**: split mode shows the left text and the right text side
//!   by side with their line numbers, removed lines tinted on the left,
//!   added ones on the right, blank filler rows keeping the two sides of
//!   a hunk level, the changed characters of a paired line tinted darker;
//!   inline mode shows one column, a hunk's removed lines then its added
//!   ones (an accepted hunk only its added ones: the result). Each hunk
//!   starts with a header band (`@@ -40,6 +40,7 @@`, its state, Accept
//!   and Reject buttons at its right); what a decision set aside is
//!   dimmed (an accepted hunk's removed lines, a rejected hunk's lines).
//!   Text is JetBrains Mono at 13 pixels in the language's colours from
//!   the theme's code scheme (`rapidr_value::code_scheme`), each token an
//!   `Op::Text` at its column (the font is monospaced). Only the rows in
//!   view are drawn.
//! - **The mouse**: a click on Accept or Reject (pressed and let go on
//!   it) decides the hunk and fires OnHunkChange(Index, Accepted); a click
//!   on a hunk makes it the current one; the wheel and the scroll bar
//!   scroll both sides together.
//! - **The keys**: Up / Down a line, Page Up / Page Down a page, Home /
//!   End the top and the bottom; F7 or Alt+Down the next hunk, Shift+F7 or
//!   Alt+Up the previous one; Enter or Ctrl+Y accepts the current hunk,
//!   Backspace or Ctrl+N rejects it (OnHunkChange) — then the next hunk is
//!   the current one.
//! - **A screen reader** sees a group whose children are the hunks
//!   ("Hunk 3 of 7: lines 40–45 changed, accepted"), each with an Accept
//!   and a Reject button; clicking those decides the hunk as the mouse
//!   does (OnHunkChange too).

use rapidr_editor::Token;
use rapidr_value::code_scheme::{self, Scheme};
use rapidr_value::input::Button;
use rapidr_value::objects::a11y::{node_id, part_id, AccessNode, Action, Role, PART_ITEM};
use rapidr_value::objects::diffview::{code_advance, code_font, expand, DiffView, Hit, Marks, Mode, Row, RowKind, Side, HEADER_H, ROW_H};
use rapidr_value::objects::ops::Place;
use rapidr_value::objects::with_diff;
use rapidr_value::theme::Theme;
use rapidr_value::v_int;

use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::a11y::AccessValue;
use crate::input::{Clipboard, KernelEvent, Mods};
use crate::paint::Painter;

pub struct DiffViewBox;

/// `a` mixed toward `b` by `t` (0: `a`, 1: `b`), 0xRRGGBB.
fn blend(a: u32, b: u32, t: f64) -> u32 {
    let ch = |s: u32| -> f64 { f64::from((a >> s) & 0xFF) * (1.0 - t) + f64::from((b >> s) & 0xFF) * t };
    ((ch(16).round() as u32) << 16) | ((ch(8).round() as u32) << 8) | ch(0).round() as u32
}

/// Black or white, whichever reads better on `bg`.
fn ink_on(bg: u32) -> u32 {
    let lin = |s: u32| {
        let c = f64::from((bg >> s) & 0xFF) / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    let l = 0.2126 * lin(16) + 0.7152 * lin(8) + 0.0722 * lin(0);
    if (l + 0.05) / 0.05 >= 1.05 / (l + 0.05) { 0x000000 } else { 0xFFFFFF }
}

/// Its border's width: a classic sunken edge (2), a fluent frame (1).
fn border(t: &Theme) -> i64 {
    if t.fluent() { 1 } else { 2 }
}

/// The colours a frame of the view is drawn in.
struct Look<'a> {
    s: &'a Scheme,
    t: &'a Theme,
    adv: f64,
    /// The line-number gutter's width.
    nw: i64,
    focused: bool,
}

/// How a line is tinted.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tint {
    None,
    Removed,
    Added,
}

impl Look<'_> {
    fn filler(&self) -> u32 {
        blend(self.s.background, self.s.text, if self.s.dark { 0.06 } else { 0.035 })
    }

    fn rule(&self) -> u32 {
        blend(self.s.background, self.s.text, if self.s.dark { 0.22 } else { 0.16 })
    }

    fn line_bg(&self, tint: Tint, dim: bool) -> u32 {
        let c = match tint {
            Tint::None => return self.s.background,
            Tint::Removed => self.s.diff_removed,
            Tint::Added => self.s.diff_added,
        };
        if dim { blend(c, self.s.background, 0.6) } else { c }
    }

    fn mark_bg(&self, tint: Tint, dim: bool) -> u32 {
        let c = if tint == Tint::Removed { self.s.diff_removed_text } else { self.s.diff_added_text };
        if dim { blend(c, self.s.background, 0.6) } else { c }
    }

    fn gutter_bg(&self, tint: Tint, dim: bool) -> u32 {
        match tint {
            Tint::None => self.s.gutter,
            _ => blend(self.line_bg(tint, dim), self.mark_bg(tint, dim), 0.35),
        }
    }

    fn accent(&self, tint: Tint) -> u32 {
        if tint == Tint::Removed { self.s.mark_removed } else { self.s.mark_added }
    }

    fn header_bg(&self, current: bool) -> u32 {
        blend(self.s.background, self.s.mark_changed, match (current, self.s.dark) {
            (true, true) => 0.30,
            (true, false) => 0.16,
            (false, true) => 0.16,
            (false, false) => 0.07,
        })
    }

    /// Text dimmed (what a decision set aside).
    /// (high contrast dims less: its text stays readable)
    fn dim(&self, c: u32) -> u32 {
        blend(c, self.s.background, if self.s.current_line_frame { 0.3 } else { 0.55 })
    }
}

/// Line `line` of `side` drawn at (x, y): its tokens in the scheme's
/// colours from column 0, no further right than `room` pixels.
#[allow(clippy::too_many_arguments)]
fn draw_code(p: &mut Painter, look: &Look, side: &mut Side, line: usize, x: i64, y: i64, room: i64, dim: bool) {
    let Some(text) = side.lines.get(line) else { return };
    if text.is_empty() || room <= 0 {
        return;
    }
    let text = text.clone();
    let tokens = side.tokens(line);
    let (shown, cols) = expand(&text);
    let chars: Vec<char> = shown.chars().collect();
    // (the display column of each byte of the line)
    let mut col_of = vec![0usize; text.len() + 1];
    for (i, (b, _)) in text.char_indices().enumerate() {
        col_of[b] = cols[i];
    }
    col_of[text.len()] = *cols.last().unwrap_or(&0);
    for b in (0..text.len()).rev() {
        if !text.is_char_boundary(b) {
            col_of[b] = col_of[b + 1];
        }
    }
    let max_col = ((room as f64) / look.adv).ceil() as usize + 1;
    // (the runs: the tokens, and the text between them plain)
    let mut runs: Vec<(usize, usize, Option<Token>)> = Vec::with_capacity(tokens.len() * 2 + 1);
    let mut at = 0usize;
    for t in &tokens {
        let (s, e) = (t.start as usize, t.end as usize);
        if s > at {
            runs.push((at, s, None));
        }
        if e > s.max(at) {
            runs.push((s.max(at), e, Some(*t)));
        }
        at = at.max(e);
    }
    if at < text.len() {
        runs.push((at, text.len(), None));
    }
    for (s, e, tok) in runs {
        let (c0, c1) = (col_of[s.min(text.len())], col_of[e.min(text.len())].min(max_col).min(chars.len()));
        if c0 >= c1 {
            continue;
        }
        let piece: String = chars[c0..c1].iter().collect();
        if piece.trim().is_empty() {
            continue;
        }
        let style = match tok {
            Some(t) => look.s.token(t.kind.fallbacks()),
            None => look.s.token(["text"]),
        };
        let color = if dim { look.dim(style.color) } else { style.color };
        let styles = u8::from(style.bold) | u8::from(style.italic) << 1;
        let px = x + (c0 as f64 * look.adv).round() as i64;
        let w = ((c1 - c0) as f64 * look.adv).ceil() as i64 + 2;
        p.text((px, y, w, ROW_H), &piece, &code_font(styles), color, Place::Left);
    }
}

/// A line number right-aligned in a gutter `w` wide at (x, y).
fn draw_number(p: &mut Painter, look: &Look, n: usize, x: i64, y: i64, w: i64, color: u32) {
    let s = (n + 1).to_string();
    let tw = (s.len() as f64 * look.adv).round() as i64;
    p.text((x + w - 7 - tw, y, tw + 2, ROW_H), &s, &code_font(0), color, Place::Left);
}

/// The changed characters' boxes.
fn draw_marks(p: &mut Painter, look: &Look, marks: &Marks, x: i64, y: i64, room: i64, color: u32) {
    for m in marks {
        let x0 = x + (m.start as f64 * look.adv).round() as i64;
        let x1 = (x + (m.end as f64 * look.adv).round() as i64).min(x + room);
        if x1 > x0 {
            p.round((x0, y + 1, x1 - x0, ROW_H - 2), 2.0, Some(color), None, 0.0);
        }
    }
}

/// One side's line in a split pane (or a filler when `line` is `None`).
#[allow(clippy::too_many_arguments)]
fn pane_line(p: &mut Painter, look: &Look, side: &mut Side, line: Option<usize>, x: i64, y: i64, w: i64, tint: Tint, dim: bool, marks: &Marks) {
    let Some(line) = line else {
        p.fill((x, y, w, ROW_H), look.filler());
        return;
    };
    p.fill((x, y, w, ROW_H), look.line_bg(tint, dim));
    p.fill((x, y, look.nw, ROW_H), look.gutter_bg(tint, dim));
    if tint != Tint::None {
        p.fill((x, y, 3, ROW_H), if dim { look.dim(look.accent(tint)) } else { look.accent(tint) });
    }
    let num = if tint == Tint::None { look.s.line_number } else { look.s.line_number_active };
    draw_number(p, look, line, x, y, look.nw, if dim { look.dim(num) } else { num });
    let cx = x + look.nw + 6;
    let room = w - look.nw - 6;
    p.clipped((cx, y, room.max(0), ROW_H), |p| {
        if tint != Tint::None {
            draw_marks(p, look, marks, cx, y, room, look.mark_bg(tint, dim));
        }
        draw_code(p, look, side, line, cx, y, room, dim);
    });
}

/// A hunk's header band across the view: its range, its state, Accept and
/// Reject.
fn draw_header(p: &mut Painter, look: &Look, d: &DiffView, h: usize, y: i64, cw: i64) {
    let current = d.current == h as i64;
    p.fill((0, y, cw, HEADER_H), look.header_bg(current));
    let edge = blend(look.s.background, look.s.mark_changed, if look.s.dark { 0.45 } else { 0.3 });
    p.fill((0, y, cw, 1), edge);
    p.fill((0, y + HEADER_H - 1, cw, 1), edge);
    if current && look.focused {
        p.fill((0, y, 3, HEADER_H), look.s.mark_changed);
    }
    let label = d.hunk_label(h);
    let lw = (label.chars().count() as f64 * look.adv).ceil() as i64 + 2;
    p.text((12, y, lw, HEADER_H), &label, &code_font(0), if current { look.s.text } else { look.s.line_number }, Place::Left);
    let (accept, reject) = d.buttons(y);
    let state = d.state(h);
    // (its state, before the buttons)
    let font = &d.ui_font;
    let said = match state {
        1 => Some(("Accepted", look.s.mark_added)),
        -1 => Some(("Rejected", look.s.mark_removed)),
        _ => None,
    };
    if let Some((word, color)) = said {
        let ww = rapidr_value::objects::text::text_size(word, font).0 + 4;
        if accept.0 - 12 - ww > 12 + lw {
            p.text((accept.0 - 12 - ww, y, ww, HEADER_H), word, font, color, Place::Left);
        }
    }
    for (rect, on, accept_it) in [(accept, state == 1, true), (reject, state == -1, false)] {
        let base = if accept_it { look.s.mark_added } else { look.s.mark_removed };
        let hit = if accept_it { Hit::Accept(h) } else { Hit::Reject(h) };
        let (hover, pressed) = (d.hover == hit, d.pressed == hit && d.hover == hit);
        let radius = if look.t.fluent() { 4.0 } else { 0.0 };
        let (fill, stroke, ink, icon) = if on {
            let f = if pressed { blend(base, look.s.text, 0.2) } else { base };
            (f, f, ink_on(f), ink_on(f))
        } else {
            let k = if pressed { 0.38 } else if hover { 0.24 } else { 0.12 };
            (blend(look.s.background, base, k), blend(look.s.background, base, 0.65), look.s.text, base)
        };
        p.round(rect, radius, Some(fill), Some(stroke), 1.0);
        let (x, by, w, bh) = rect;
        p.icon(if accept_it { "glyphs/check" } else { "glyphs/close-small" }, (x + 6, by + (bh - 14) / 2, 14, 14), Some(icon), false);
        p.text((x + 22, by, w - 24, bh), if accept_it { "Accept" } else { "Reject" }, font, ink, Place::Left);
    }
}

/// The view drawn at its origin (inside the border).
fn paint_view(p: &mut Painter, d: &mut DiffView, look: &Look) {
    let (cw, ch) = d.client();
    let top = d.top();
    p.fill((0, 0, cw, ch), look.s.background);
    let rows: Vec<Row> = d.rows[d.visible_rows()].to_vec();
    let split = d.mode == Mode::Split;
    let pane = (cw - 1) / 2;
    for row in rows {
        let y = row.y - top;
        let h = row.hunk.unwrap_or(0);
        let state = d.state(h);
        match row.kind {
            RowKind::Header => draw_header(p, look, d, h, y, cw),
            RowKind::Same if split => {
                let marks = Marks::new();
                pane_line(p, look, &mut d.left, row.left, 0, y, pane, Tint::None, false, &marks);
                pane_line(p, look, &mut d.right, row.right, pane + 1, y, cw - pane - 1, Tint::None, false, &marks);
                p.fill((pane, y, 1, ROW_H), look.rule());
            }
            RowKind::Pair => {
                let (ml, mr) = match d.pair_of(&row) {
                    Some(k) => d.marks(h, k),
                    None => Default::default(),
                };
                pane_line(p, look, &mut d.left, row.left, 0, y, pane, Tint::Removed, state != 0, &ml);
                pane_line(p, look, &mut d.right, row.right, pane + 1, y, cw - pane - 1, Tint::Added, state == -1, &mr);
                p.fill((pane, y, 1, ROW_H), look.rule());
            }
            RowKind::Same | RowKind::Removed | RowKind::Added => {
                let (tint, dim) = match row.kind {
                    RowKind::Removed => (Tint::Removed, state == -1),
                    RowKind::Added => (Tint::Added, state == -1),
                    _ => (Tint::None, false),
                };
                let marks = match (row.kind, d.pair_of(&row)) {
                    (RowKind::Removed, Some(k)) => d.marks(h, k).0,
                    (RowKind::Added, Some(k)) => d.marks(h, k).1,
                    _ => Marks::new(),
                };
                inline_line(p, look, d, &row, y, cw, tint, dim, &marks);
            }
        }
    }
    // (below the last row: the panes' divider goes on down)
    if let Some(last) = d.rows.last() {
        let end = (last.y + last.height() - top).max(0);
        if end < ch {
            p.fill((0, end, cw, ch - end), look.s.background);
            if split {
                p.fill((pane, end, 1, ch - end), look.rule());
            }
        }
    }
}

/// An inline row: the old and the new line numbers, the sign, the line.
#[allow(clippy::too_many_arguments)]
fn inline_line(p: &mut Painter, look: &Look, d: &mut DiffView, row: &Row, y: i64, cw: i64, tint: Tint, dim: bool, marks: &Marks) {
    let nw = look.nw;
    p.fill((0, y, cw, ROW_H), look.line_bg(tint, dim));
    p.fill((0, y, 2 * nw, ROW_H), look.gutter_bg(tint, dim));
    let num = if tint == Tint::None { look.s.line_number } else { look.s.line_number_active };
    let num = if dim { look.dim(num) } else { num };
    if let Some(l) = row.left {
        draw_number(p, look, l, 0, y, nw, num);
    }
    if let Some(r) = row.right {
        draw_number(p, look, r, nw, y, nw, num);
    }
    if tint != Tint::None {
        let accent = if dim { look.dim(look.accent(tint)) } else { look.accent(tint) };
        p.fill((0, y, 3, ROW_H), accent);
        let sign = if tint == Tint::Removed { "\u{2212}" } else { "+" };
        p.text((2 * nw + 2, y, 14, ROW_H), sign, &code_font(0), accent, Place::Center);
    }
    let cx = 2 * nw + 18;
    let room = cw - cx;
    let (side, line) = match (row.kind, row.left, row.right) {
        (RowKind::Added, _, Some(r)) => (&mut d.right, r),
        (_, Some(l), _) => (&mut d.left, l),
        _ => return,
    };
    p.clipped((cx, y, room.max(0), ROW_H), |p| {
        if tint != Tint::None {
            draw_marks(p, look, marks, cx, y, room, look.mark_bg(tint, dim));
        }
        draw_code(p, look, side, line, cx, y, room, dim);
    });
}

/// Where the hunks are, on the scroll bar's track: a mark per hunk (its
/// removed half left, its added half right), not over the thumb.
fn overview(p: &mut Painter, d: &DiffView, look: &Look) {
    let (w, h) = d.view;
    let Some((bx, by, _, bh)) = d.bars.bar(true, w, h) else { return };
    let bar = rapidr_value::scrollbars::BAR;
    let track = bh - 2 * bar;
    let total = (d.height + ROW_H / 2).max(1);
    if track <= 0 {
        return;
    }
    let thumb = d.bars.thumb(true, bh, w, h).map(|(t, s)| (by + t, by + t + s));
    let at = |y: i64| by + bar + y * track / total;
    for (k, &r) in d.header_rows.iter().enumerate() {
        let start = d.rows[r].y + HEADER_H;
        let end = d.header_rows.get(k + 1).map_or(d.rows.len(), |&n| n);
        let end = d.rows[..end].iter().rev().find(|row| row.hunk == Some(k)).map_or(start, |row| row.y + row.height());
        let (y0, y1) = (at(start), at(end).max(at(start) + 2));
        let hk = &d.hunks[k];
        let dim = |c: u32, gone: bool| if gone { look.dim(c) } else { c };
        let state = d.state(k);
        let mut halves = Vec::new();
        if !hk.left.is_empty() {
            halves.push((bx + 3, 5, dim(look.s.mark_removed, state != 0)));
        }
        if !hk.right.is_empty() {
            halves.push((bx + 9, 5, dim(look.s.mark_added, state == -1)));
        }
        for (x, mw, color) in halves {
            // (the thumb stays on top)
            let mut parts = vec![(y0, y1)];
            if let Some((t0, t1)) = thumb {
                parts = parts.into_iter().flat_map(|(a, b)| [(a, b.min(t0)), (a.max(t1), b)]).filter(|(a, b)| b > a).collect();
            }
            for (a, b) in parts {
                p.fill((x, a, mw, b - a), color);
            }
        }
    }
}

/// OnHunkChange(Index, Accepted).
fn heard(cx: &mut Cx, h: usize, accepted: bool) {
    cx.events.push(KernelEvent::Fire { id: cx.id.to_string(), event: "onhunkchange".into(), args: vec![v_int(h as i64), v_int(i64::from(accepted))] });
}

/// Decides the current hunk from the keyboard (or a screen reader's
/// button): OnHunkChange if its state changed, then the next hunk is the
/// current one.
fn decide(cx: &mut Cx, h: usize, accept: bool, advance: bool) {
    let changed = with_diff(cx.id, |d| {
        let changed = d.set_state(h, if accept { 1 } else { -1 });
        d.current = h as i64;
        if advance && h + 1 < d.hunks.len() {
            d.go_to(h + 1);
        }
        changed
    });
    if changed == Some(true) {
        heard(cx, h, accept);
    }
}

impl DiffViewBox {
    /// The view laid out at the component's size (inside its border).
    fn laid_out(cx: &Cx) -> i64 {
        let b = border(rapidr_value::theme::current());
        let (w, h) = (cx.width() - 2 * b, cx.height() - 2 * b);
        let font = cx.font.clone();
        with_diff(cx.id, |d| d.layout(w.max(0), h.max(0), &font));
        b
    }
}

impl ComponentKind for DiffViewBox {
    fn name(&self) -> &'static str {
        "RDIFFVIEW"
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let t = p.theme();
        let b = Self::laid_out(cx);
        let (w, h) = (cx.width(), cx.height());
        let s = code_scheme::for_theme(t);
        if t.fluent() {
            p.frame((0, 0, w, h), if cx.state.focused { t.accent } else { t.border });
        } else {
            p.sunken_edge((0, 0, w, h));
        }
        let focused = cx.state.focused;
        let id = cx.id.to_string();
        p.at((b, b), |p| {
            with_diff(&id, |d| {
                let look = Look { s, t, adv: code_advance(), nw: d.number_width(), focused };
                let (iw, ih) = d.view;
                let (cw, ch) = d.client();
                p.clipped((0, 0, cw, ch), |p| paint_view(p, d, &look));
                p.ops(crate::paint::bar_ops(&d.bars, iw, ih));
                overview(p, d, &look);
            });
        });
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let b = Self::laid_out(cx);
        let out = MouseOut { press: false, focus: Some(true) };
        let (x, y) = (m.x.floor() as i64 - b, m.y.floor() as i64 - b);
        // (the scroll bar first)
        let on_bars = with_diff(cx.id, |d| {
            let (w, h) = d.view;
            let took = match m.kind {
                MouseKind::Down if m.button == Button::Left => d.bars.mouse_down(x, y, w, h).is_some(),
                MouseKind::Move if d.bars.pressed.is_some() => {
                    d.bars.mouse_drag(x, y, w, h);
                    true
                }
                MouseKind::Up if d.bars.pressed.is_some() => {
                    d.bars.mouse_up(w, h);
                    true
                }
                _ => false,
            };
            if took {
                d.held = (m.kind != MouseKind::Up).then_some((x, y));
                d.revision += 1;
            }
            took
        });
        if on_bars == Some(true) {
            match m.kind {
                MouseKind::Down => cx.ui.wake = Some(crate::tick::now() + crate::tick::REPEAT_DELAY),
                MouseKind::Up => cx.ui.wake = None,
                _ => {}
            }
            return out;
        }
        let decided = with_diff(cx.id, |d| {
            let hit = if m.kind == MouseKind::Leave { Hit::Nothing } else { d.hit(x, y) };
            let button = |h: Hit| matches!(h, Hit::Accept(_) | Hit::Reject(_));
            let hover = if button(hit) { hit } else { Hit::Nothing };
            if d.hover != hover {
                d.hover = hover;
                d.revision += 1;
            }
            match m.kind {
                MouseKind::Down if m.button == Button::Left => {
                    if let Hit::Accept(h) | Hit::Reject(h) | Hit::Hunk(h) = hit {
                        d.current = h as i64;
                    }
                    d.pressed = if button(hit) { hit } else { Hit::Nothing };
                    d.revision += 1;
                    None
                }
                MouseKind::Up => {
                    let pressed = std::mem::replace(&mut d.pressed, Hit::Nothing);
                    d.revision += 1;
                    match (pressed == hit, hit) {
                        (true, Hit::Accept(h)) => d.set_state(h, 1).then_some((h, true)),
                        (true, Hit::Reject(h)) => d.set_state(h, -1).then_some((h, false)),
                        _ => None,
                    }
                }
                _ => None,
            }
        })
        .flatten();
        if let Some((h, accepted)) = decided {
            heard(cx, h, accepted);
        }
        out
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, _clip: &mut dyn Clipboard) -> bool {
        Self::laid_out(cx);
        let ctrl = k.mods.ctrl || k.mods.command;
        let Mods { shift, alt, .. } = k.mods;
        let current = with_diff(cx.id, |d| usize::try_from(d.current).ok()).flatten();
        match k.vk {
            // F7 / Alt+Down, Shift+F7 / Alt+Up: the next / previous hunk
            118 if !ctrl && !alt => {
                with_diff(cx.id, |d| if shift { d.previous_hunk() } else { d.next_hunk() });
            }
            38 | 40 if alt && !ctrl => {
                with_diff(cx.id, |d| if k.vk == 38 { d.previous_hunk() } else { d.next_hunk() });
            }
            38 | 40 if !ctrl => {
                with_diff(cx.id, |d| d.scroll_by(if k.vk == 38 { -ROW_H } else { ROW_H }));
            }
            33 | 34 => {
                with_diff(cx.id, |d| {
                    let page = (d.client().1 - ROW_H).max(ROW_H);
                    d.scroll_by(if k.vk == 33 { -page } else { page });
                });
            }
            36 | 35 => {
                with_diff(cx.id, |d| d.scroll_to(if k.vk == 36 { 0 } else { d.height }));
            }
            // Enter / Ctrl+Y accept, Backspace / Ctrl+N reject
            13 if !ctrl && !alt => {
                if let Some(h) = current {
                    decide(cx, h, true, true);
                }
            }
            89 if ctrl && !alt => {
                if let Some(h) = current {
                    decide(cx, h, true, true);
                }
            }
            8 if !ctrl && !alt => {
                if let Some(h) = current {
                    decide(cx, h, false, true);
                }
            }
            78 if ctrl && !alt => {
                if let Some(h) = current {
                    decide(cx, h, false, true);
                }
            }
            _ => return false,
        }
        true
    }

    fn wheel(&self, cx: &mut Cx, _dx: f64, dy: f64, _mods: Mods) -> bool {
        Self::laid_out(cx);
        let notches = super::list::whole_notches(cx.id, dy);
        with_diff(cx.id, |d| {
            let (w, h) = d.view;
            if !d.bars.vert.shown {
                return false;
            }
            if notches != 0 {
                d.bars.wheel(notches, false, w, h);
                d.revision += 1;
            }
            true
        })
        .unwrap_or(false)
    }

    fn tick(&self, cx: &mut Cx) {
        let held = with_diff(cx.id, |d| {
            d.repeat();
            d.held.is_some()
        });
        if held == Some(true) {
            cx.ui.wake = Some(crate::tick::now() + crate::tick::REPEAT);
        }
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let b = Self::laid_out(cx);
        let mut n = AccessNode::new(node_id(cx.id), Role::Group);
        n.name = "Differences".into();
        n.bounds = cx.rect;
        let (x0, y0) = (cx.rect.0 + b, cx.rect.1 + b);
        let id = cx.id.to_string();
        with_diff(&id, |d| {
            let count = d.hunks.len();
            let (acc, rej) = (d.states.iter().filter(|&&s| s == 1).count(), d.states.iter().filter(|&&s| s == -1).count());
            n.value = Some(format!("{count} hunk{}: {acc} accepted, {rej} rejected, {} undecided", if count == 1 { "" } else { "s" }, count - acc - rej));
            let (cw, ch) = d.client();
            for h in 0..count {
                let mut g = AccessNode::new(part_id(&id, PART_ITEM, 3 * h), Role::Group);
                g.name = d.hunk_summary(h);
                // (ARIA has no "selected" for a group: the current one says so)
                if d.current == h as i64 {
                    g.description = "current hunk".into();
                }
                g.actions = vec![Action::Click, Action::ScrollIntoView];
                let y = d.rows[d.header_rows[h]].y - d.top();
                let shown = y + HEADER_H > 0 && y < ch;
                if shown {
                    g.bounds = (x0, y0 + y.max(0), cw, HEADER_H.min(ch - y.max(0)));
                }
                let (accept, reject) = d.buttons(y);
                for (k, (word, rect, on)) in [("Accept", accept, d.state(h) == 1), ("Reject", reject, d.state(h) == -1)].into_iter().enumerate() {
                    let mut bn = AccessNode::new(part_id(&id, PART_ITEM, 3 * h + 1 + k), Role::Button);
                    bn.name = word.into();
                    bn.states.checked = Some(on);
                    bn.actions = vec![Action::Click];
                    if shown {
                        bn.bounds = (x0 + rect.0, y0 + rect.1, rect.2, rect.3);
                    }
                    g.children.push(bn);
                }
                n.children.push(g);
            }
        });
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        Self::laid_out(cx);
        let Some(part) = part else { return false };
        let (h, which) = (part / 3, part % 3);
        if with_diff(cx.id, |d| h < d.hunks.len()) != Some(true) {
            return false;
        }
        match (action, which) {
            (Action::Click | Action::ScrollIntoView, 0) => {
                with_diff(cx.id, |d| d.go_to(h));
                true
            }
            (Action::Click, 1 | 2) => {
                decide(cx, h, which == 1, false);
                true
            }
            _ => false,
        }
    }
}

/// Hunk `h`'s Accept (or Reject) button's centre in component `id`
/// (logical pixels) as last laid out; `None` when that hunk's header isn't
/// in view.
pub fn button_point(id: &str, h: usize, accept: bool) -> Option<(i64, i64)> {
    let b = border(rapidr_value::theme::current());
    with_diff(id, |d| {
        let r = *d.header_rows.get(h)?;
        let y = d.rows[r].y - d.top();
        let (_, ch) = d.client();
        if y < 0 || y + HEADER_H > ch {
            return None;
        }
        let (a, rj) = d.buttons(y);
        let (x, y, w, bh) = if accept { a } else { rj };
        Some((b + x + w / 2, b + y + bh / 2))
    })
    .flatten()
}

#[cfg(test)]
mod tests {
    use rapidr_value::objects::ops::Op;
    use rapidr_value::{v_int, v_str, Value};

    use crate::display::Item;
    use crate::{FormUi, KernelEvent, MemClipboard, MemStore, Mods, TextSystem};

    const LEFT: &str = "SUB Hello\n  PRINT \"Hello\"\nEND SUB\n\nSUB Bye\n  PRINT 1\nEND SUB";
    const RIGHT: &str = "SUB Hello\n  PRINT \"Hello, World\"\nEND SUB\n\n' added\nSUB Bye\nEND SUB";

    fn diff_form() -> (MemStore, FormUi, TextSystem) {
        let mut s = MemStore::new();
        s.add("df", "RFORM", None).set("df", "clientwidth", v_int(640)).set("df", "clientheight", v_int(400));
        s.add("dv", "RDIFFVIEW", Some("df")).set("dv", "left", v_int(10)).set("dv", "top", v_int(10)).set("dv", "width", v_int(600)).set("dv", "height", v_int(300));
        s.set("dv", "lefttext", v_str(LEFT));
        s.set("dv", "righttext", v_str(RIGHT));
        let f = FormUi::build(&s, "df", false);
        (s, f, TextSystem::new())
    }

    fn fired(events: Vec<KernelEvent>) -> Vec<(String, Vec<i64>)> {
        events
            .into_iter()
            .filter_map(|e| match e {
                KernelEvent::Fire { id, event, args } if id == "dv" => Some((event, args.iter().map(Value::to_i64).collect())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn it_draws_both_sides_and_the_buttons() {
        let (s, mut f, mut ts) = diff_form();
        let list = f.paint(&s, &mut ts, 1.0);
        let texts: Vec<String> = list.items.iter().filter_map(|i| match i {
            Item::Op { op: Op::Text { text, .. }, .. } => Some(text.clone()),
            _ => None,
        }).collect();
        for want in ["SUB", "Hello", "\"Hello, World\"", "' added", "Accept", "Reject", "@@ -2,1 +2,1 @@"] {
            assert!(texts.iter().any(|t| t == want), "{want}: {texts:?}");
        }
        // (the changed characters, darker: a rounded box)
        assert!(list.items.iter().any(|i| matches!(i, Item::Op { op: Op::Round { radius, .. }, .. } if *radius == 2.0)));
    }

    #[test]
    fn clicks_and_keys_decide_hunks() {
        let (s, mut f, mut ts) = diff_form();
        drop(f.paint(&s, &mut ts, 1.0));
        // Accept on the first hunk: press and release on it
        let (x, y) = super::button_point("dv", 0, true).unwrap();
        let (ax, ay) = (10.5 + x as f64, 10.5 + y as f64);
        f.mouse_down(&s, &mut ts, ax, ay, rapidr_value::input::Button::Left, Mods::NONE);
        f.mouse_up(&s, &mut ts, ax, ay, rapidr_value::input::Button::Left, Mods::NONE);
        assert_eq!(fired(f.take_events()), [("onhunkchange".to_string(), vec![0, 1])]);
        assert_eq!(crate::Store::get(&s, "dv", "currenthunk").to_i64(), 0);
        // F7 to the second, Backspace rejects it (and the third is current)
        let mut clip = MemClipboard::default();
        f.key_down(&s, &mut ts, 118, "", Mods::NONE, &mut clip);
        f.key_down(&s, &mut ts, 8, "", Mods::NONE, &mut clip);
        assert_eq!(fired(f.take_events()), [("onhunkchange".to_string(), vec![1, 0])]);
        assert_eq!(crate::Store::get(&s, "dv", "currenthunk").to_i64(), 2);
        // Enter accepts the third
        f.key_down(&s, &mut ts, 13, "\r", Mods::NONE, &mut clip);
        assert_eq!(fired(f.take_events()), [("onhunkchange".to_string(), vec![2, 1])]);
        assert_eq!(crate::Store::get(&s, "dv", "resulttext").to_string_val(), "SUB Hello\n  PRINT \"Hello, World\"\nEND SUB\n\nSUB Bye\nEND SUB");
        // (a screen reader's Reject on hunk 0)
        let tree = f.access_tree(&s, &mut ts);
        let json = tree.to_json();
        assert!(json.contains("Hunk 1 of 3: line 2 changed, accepted"), "{json}");
        let reject = rapidr_value::objects::a11y::part_id("dv", rapidr_value::objects::a11y::PART_ITEM, 2);
        assert!(f.access_action(&s, &mut ts, reject, rapidr_value::objects::a11y::Action::Click, None));
        assert_eq!(fired(f.take_events()), [("onhunkchange".to_string(), vec![0, 0])]);
    }

    #[test]
    fn inline_mode_and_scrolling() {
        let (mut s, mut f, mut ts) = diff_form();
        let left: String = (0..300).map(|i| format!("PRINT {i}\n")).collect();
        let right = left.replace("PRINT 250\n", "PRINT 250 ' changed\n");
        s.set("dv", "lefttext", v_str(&left));
        s.set("dv", "righttext", v_str(&right));
        s.set("dv", "mode", v_str("inline"));
        let list = f.paint(&s, &mut ts, 1.0);
        let shown = |list: &crate::display::DisplayList, t: &str| list.items.iter().any(|i| matches!(i, Item::Op { op: Op::Text { text, .. }, .. } if text == t));
        assert!(shown(&list, "1") && !shown(&list, "251"), "{}", list.dump());
        // F7: the hunk scrolled into view (one hunk: it stays the current)
        f.focus_id(&s, "dv");
        let mut clip = MemClipboard::default();
        f.key_down(&s, &mut ts, 118, "", Mods::NONE, &mut clip);
        let list = f.paint(&s, &mut ts, 1.0);
        assert!(shown(&list, "251") && shown(&list, "' changed"), "{}", list.dump());
        // the wheel back to the top
        for _ in 0..120 {
            f.mouse_wheel(&s, &mut ts, (300.0, 100.0), (0.0, -1.0), Mods::NONE);
        }
        let list = f.paint(&s, &mut ts, 1.0);
        assert!(shown(&list, "1"));
    }
}
