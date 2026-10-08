//! The code editor's popups, in the window's popup layer (over every
//! component, as drop-down lists and menus): the completion list (an icon
//! per kind, the typed part marked, the detail, the selected item's docs
//! beside it), hovers (code coloured as the language, then prose) and
//! signature help (the active parameter marked).

use rapidr_editor::service::CompletionKind;
use rapidr_editor::{Highlighter, Languages};
use rapidr_value::code_scheme::Scheme;
use rapidr_value::objects::codeedit::CodeEditor as Model;
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Place, Rect};
use rapidr_value::objects::text::{text_size, CODE_FACE};
use rapidr_value::objects::{with_code, with_code_mut};

use super::paint::{caret_x, shown_at};
use super::{CodeUi, Ctx};
use crate::paint::Painter;
use crate::store::Store;
use crate::text::TextSystem;
use crate::tree::FormUi;

/// Items the completion list shows at once.
pub const VISIBLE_ITEMS: usize = 10;
const ITEM_H: i64 = 22;
const LIST_W: i64 = 380;
const DOC_W: i64 = 300;

/// The completion items that match what's typed since the list opened,
/// best first (`CodeEditor::completion_shown`: the model's ranking).
pub fn filtered(x: &Ctx) -> Vec<usize> {
    x.c.completion_shown()
}

/// Scrolls the list so the selected item shows.
pub fn keep_selected_visible(x: &mut Ctx) {
    let sel = x.c.completion.as_ref().map_or(0, |c| c.selected);
    if sel < x.ui.completion_top {
        x.ui.completion_top = sel;
    } else if sel >= x.ui.completion_top + VISIBLE_ITEMS {
        x.ui.completion_top = sel + 1 - VISIBLE_ITEMS;
    }
}

fn ui_font() -> Font {
    Font { name: "Segoe UI".into(), size: -12, ..Font::default() }
}

fn code_font(px: i64) -> Font {
    Font { name: CODE_FACE.into(), size: -px.max(8), ..Font::default() }
}

/// The icon of a completion kind.
fn kind_icon(k: CompletionKind) -> &'static str {
    match k {
        CompletionKind::Keyword => "symbols/keyword",
        CompletionKind::Builtin | CompletionKind::Function => "symbols/function",
        CompletionKind::Sub => "symbols/sub",
        CompletionKind::Method => "symbols/method",
        CompletionKind::Property => "symbols/property",
        CompletionKind::Event => "symbols/event",
        CompletionKind::Variable | CompletionKind::Text => "symbols/variable",
        CompletionKind::Parameter => "symbols/parameter",
        CompletionKind::Field => "symbols/field",
        CompletionKind::Constant => "symbols/constant",
        CompletionKind::Component => "symbols/component",
        CompletionKind::Type => "symbols/type",
        CompletionKind::Directive => "symbols/directive",
        CompletionKind::Label => "symbols/label",
        CompletionKind::Snippet => "symbols/snippet",
        CompletionKind::Fix => "actions/hint",
    }
}

/// Where a popup sits: the caret's row in the window (logical, absolute):
/// its left x at byte `at`, the row's top and bottom.
fn anchor(c: &Model, ui: &CodeUi, abs: (i64, i64), scale: f64, at: usize) -> Option<(i64, i64, i64)> {
    let sh = shown_at(ui, c, at)?;
    let dx = caret_x(ui, sh, at) / scale;
    let top = abs.1 + sh.top.round() as i64;
    Some((dx.round() as i64, top, top + ui.metrics.line))
}

/// The completion list's rectangle, and the docs panel's (absolute).
fn completion_rects(c: &Model, ui: &CodeUi, abs: (i64, i64), scale: f64, client: (i64, i64)) -> Option<(Rect, Option<Rect>, usize)> {
    let list = c.completion.as_ref()?;
    let n = c.completion_shown().len();
    if n == 0 {
        return None;
    }
    let (ax, _top, bottom) = anchor(c, ui, abs, scale, list.start)?;
    let rows = n.min(VISIBLE_ITEMS);
    let h = rows as i64 * ITEM_H + 4;
    let w = LIST_W;
    let x = (ax - 26).clamp(0, (client.0 - w).max(0));
    let y = if bottom + h <= client.1 || bottom - ui.metrics.line - h < 0 { bottom + 2 } else { bottom - ui.metrics.line - h - 2 };
    let doc = (x + w + DOC_W <= client.0).then_some((x + w + 4, y, DOC_W, h.max(80))).or_else(|| (x >= DOC_W + 4).then_some((x - DOC_W - 4, y, DOC_W, h.max(80))));
    Some(((x, y, w, h), doc, rows))
}

/// Wraps `text` at `width` pixels in `font` (lines kept).
fn wrap(text: &str, font: &Font, width: i64) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if !line.is_empty() && text_size(&candidate, font).0 > width {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        out.push(line);
    }
    out
}

/// `s` cut to `width` pixels in `font`, "…" at its end when cut (empty
/// when not even that fits).
fn ellipsized(s: &str, font: &Font, width: i64) -> String {
    if text_size(s, font).0 <= width {
        return s.to_string();
    }
    let mut chars: Vec<char> = s.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let t: String = chars.iter().collect::<String>().trim_end().to_string() + "…";
        if text_size(&t, font).0 <= width {
            return t;
        }
    }
    String::new()
}

/// Markdown made plain: emphasis marks (`**bold**`, `*italic*`) and
/// backticks dropped; a `*` between spaces or digits (`a * b`) kept.
fn plain(s: &str) -> String {
    let s = s.replace("**", "").replace('`', "").replace("\\_", "_");
    let chars: Vec<char> = s.chars().collect();
    let edge = |c: Option<&char>| c.is_none_or(|c| c.is_whitespace() || "([{\"'.,;:!?)]}".contains(*c));
    chars
        .iter()
        .enumerate()
        .filter(|&(i, &c)| {
            if c != '*' {
                return true;
            }
            let (before, after) = (i.checked_sub(1).and_then(|j| chars.get(j)), chars.get(i + 1));
            let opens = edge(before) && after.is_some_and(|a| !a.is_whitespace());
            let closes = before.is_some_and(|b| !b.is_whitespace()) && edge(after);
            !(opens || closes)
        })
        .map(|(_, &c)| c)
        .collect()
}

/// A hover's or doc's text as blocks: code (fenced) and prose.
fn blocks(text: &str) -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = Vec::new();
    let mut code = false;
    for line in text.split('\n') {
        if line.trim_start().starts_with("```") {
            code = !code;
            continue;
        }
        match out.last_mut() {
            Some((c, s)) if *c == code => {
                s.push('\n');
                s.push_str(line);
            }
            _ => out.push((code, line.to_string())),
        }
    }
    out.retain(|(_, s)| !s.trim().is_empty());
    for (_, s) in &mut out {
        *s = s.trim_matches('\n').to_string();
    }
    out
}

/// A line of code drawn in the language's colours from (x, y).
#[allow(clippy::too_many_arguments)]
fn code_line(p: &mut Painter, sc: &Scheme, hl: &mut Highlighter, state: &mut rapidr_editor::StateId, line: &str, font: &Font, x: i64, y: i64, h: i64) {
    let (tokens, next) = hl.line_spans(line, *state);
    *state = next;
    let mut at = 0usize;
    let mut cx = x;
    let mut draw = |p: &mut Painter, piece: &str, color: u32, bold: bool, italic: bool| {
        if piece.is_empty() {
            return;
        }
        let f = Font { styles: u8::from(bold) | (u8::from(italic) << 1), ..font.clone() };
        p.text((cx, y, text_size(piece, &f).0 + 2, h), piece, &f, color, Place::TopLeft);
        cx += text_size(piece, font).0;
    };
    for t in tokens {
        let (a, b) = (t.start as usize, t.end as usize);
        if a > at {
            draw(p, &line[at..a], sc.text, false, false);
        }
        let st = sc.token(t.kind.fallbacks());
        draw(p, &line[a..b], st.color, st.bold, st.italic);
        at = b;
    }
    if at < line.len() {
        draw(p, &line[at..], sc.text, false, false);
    }
}

/// The height text blocks take at `width`.
fn blocks_height(bl: &[(bool, String)], width: i64, code_px: i64) -> i64 {
    let ui = ui_font();
    let lh_ui = text_size("Ag", &ui).1 + 3;
    let lh_code = text_size("Ag", &code_font(code_px)).1 + 3;
    bl.iter().map(|(code, s)| if *code { s.lines().count() as i64 * lh_code + 6 } else { wrap(&plain(s), &ui, width).len() as i64 * lh_ui + 6 }).sum()
}

/// Draws text blocks in `r`.
fn draw_blocks(p: &mut Painter, sc: &Scheme, lang: &str, bl: &[(bool, String)], r: Rect, code_px: i64) {
    let ui = ui_font();
    let cf = code_font(code_px);
    let lh_ui = text_size("Ag", &ui).1 + 3;
    let lh_code = text_size("Ag", &cf).1 + 3;
    let language = Languages::builtin().get(lang).unwrap_or_else(|| Languages::builtin().plain_text());
    let mut y = r.1;
    p.clipped(r, |p| {
        for (i, (code, s)) in bl.iter().enumerate() {
            if *code {
                let mut hl = Highlighter::new(language.clone());
                let mut state = rapidr_editor::ROOT;
                for line in s.lines() {
                    code_line(p, sc, &mut hl, &mut state, line, &cf, r.0, y, lh_code);
                    y += lh_code;
                }
            } else {
                for line in wrap(&plain(s), &ui, r.2) {
                    p.text((r.0, y, r.2, lh_ui), &line, &ui, sc.popup_text, Place::TopLeft);
                    y += lh_ui;
                }
            }
            if i + 1 < bl.len() {
                p.fill((r.0, y + 2, r.2, 1), sc.popup_border);
            }
            y += 6;
        }
    });
}

/// A popup's box with its shadow.
fn frame(p: &mut Painter, sc: &Scheme, r: Rect) {
    p.op(Op::Round { rect: (r.0 + 1, r.1 + 2, r.2, r.3), radius: 4.0, fill: Some(super::paint::blend(0x000000, sc.background, if sc.dark { 0.45 } else { 0.16 })), stroke: None, width: 1.0 });
    p.op(Op::Round { rect: r, radius: 4.0, fill: Some(sc.popup), stroke: Some(sc.popup_border), width: 1.0 });
}

/// Every code editor's popups in form `f` (its popup layer).
pub fn paint_popups(f: &FormUi, _store: &dyn Store, _ts: &mut TextSystem, p: &mut Painter) {
    let client = (f.client.0, f.client.1 + f.menu_offset);
    for (i, n) in f.nodes.iter().enumerate() {
        let Some(ui) = n.ui.code.as_ref() else { continue };
        if !n.shown {
            continue;
        }
        let focused = f.focus == Some(i);
        let abs = (n.abs.0, n.abs.1);
        with_code(&n.id, |c| paint_one(c, ui, abs, f.scale, client, focused, p));
    }
}

fn paint_one(c: &Model, ui: &CodeUi, abs: (i64, i64), scale: f64, client: (i64, i64), focused: bool, p: &mut Painter) {
    let sc = rapidr_value::code_scheme::resolve(&c.opts.color_scheme);
    let code_px = ui.font.pixel_size().max(8);
    let lang = c.doc.language().id.clone();
    // signature help, over the caret's line
    if let (Some(sig), true) = (&c.signature, focused) {
        if let Some((ax, top, _)) = anchor(c, ui, abs, scale, sig.at) {
            let cf = code_font(code_px);
            let uf = ui_font();
            let lw = text_size(&sig.label, &cf).0 + 20;
            let doc = (!sig.doc.trim().is_empty()).then(|| wrap(&plain(&sig.doc), &uf, 460));
            let w = lw.max(doc.as_ref().map_or(0, |_| 480)).min(client.0 - 8).max(120);
            let lh_code = text_size("Ag", &cf).1 + 6;
            let lh_ui = text_size("Ag", &uf).1 + 3;
            let h = lh_code + doc.as_ref().map_or(0, |d| d.len().min(4) as i64 * lh_ui + 6) + 6;
            let y = if top - h - 2 >= 0 { top - h - 2 } else { top + ui.metrics.line + 2 };
            let x = ax.clamp(0, (client.0 - w).max(0));
            frame(p, sc, (x, y, w, h));
            // (the label, the active parameter marked)
            let active = sig.params.get(sig.active).copied();
            let mut cx = x + 10;
            let mut at = 0;
            let mut pieces: Vec<(&str, bool)> = Vec::new();
            if let Some((a, b)) = active.filter(|&(a, b)| a <= b && b <= sig.label.len()) {
                pieces.push((&sig.label[..a], false));
                pieces.push((&sig.label[a..b], true));
                at = b;
            }
            pieces.push((&sig.label[at..], false));
            for (piece, on) in pieces {
                if piece.is_empty() {
                    continue;
                }
                let f = Font { styles: u8::from(on), ..cf.clone() };
                p.text((cx, y + 4, text_size(piece, &f).0 + 2, lh_code), piece, &f, if on { sc.popup_param } else { sc.popup_text }, Place::TopLeft);
                if on {
                    p.fill((cx, y + 4 + lh_code - 5, text_size(piece, &cf).0, 1), sc.popup_param);
                }
                cx += text_size(piece, &cf).0;
            }
            if let Some(d) = doc {
                let mut dy = y + lh_code + 4;
                p.fill((x + 8, dy - 2, w - 16, 1), sc.popup_border);
                for line in d.iter().take(4) {
                    p.text((x + 10, dy + 2, w - 20, lh_ui), line, &uf, sc.popup_detail, Place::TopLeft);
                    dy += lh_ui;
                }
            }
        }
    }
    // the hover
    if let Some(h) = &c.hover {
        if let Some((ax, top, bottom)) = anchor(c, ui, abs, scale, h.start) {
            let bl = blocks(&h.text);
            let w = 480.min(client.0 - 8).max(160);
            let hh = (blocks_height(&bl, w - 20, code_px) + 10).min(320);
            let y = if top - hh - 2 >= 0 { top - hh - 2 } else { bottom + 2 };
            let x = ax.clamp(0, (client.0 - w).max(0));
            frame(p, sc, (x, y, w, hh));
            draw_blocks(p, sc, &lang, &bl, (x + 10, y + 6, w - 20, hh - 10), code_px);
        }
    }
    // the completion list
    if !focused {
        return;
    }
    let Some(list) = &c.completion else { return };
    let items = c.completion_shown();
    let Some(((lx, ly, lw, lh), doc_rect, rows)) = completion_rects(c, ui, abs, scale, client) else { return };
    frame(p, sc, (lx, ly, lw, lh));
    let uf = ui_font();
    let cf = code_font(code_px);
    let head = c.doc.selections().primary().head;
    let typed_len = c.doc.slice(list.start..head.max(list.start)).chars().count();
    let top = ui.completion_top.min(items.len().saturating_sub(rows));
    let th = text_size("Ag", &cf).1;
    p.clipped((lx + 1, ly + 1, lw - 2, lh - 2), |p| {
        for (k, &i) in items.iter().enumerate().skip(top).take(rows) {
            let it = &list.items[i];
            let y = ly + 2 + (k - top) as i64 * ITEM_H;
            let selected = k == list.selected.min(items.len() - 1);
            if selected {
                p.op(Op::Round { rect: (lx + 3, y, lw - 6, ITEM_H), radius: 3.0, fill: Some(sc.popup_selected), stroke: None, width: 1.0 });
            }
            let fg = if selected { sc.popup_selected_text } else { sc.popup_text };
            p.icon(kind_icon(it.kind), (lx + 8, y + 3, 16, 16), None, false);
            let ty = y + (ITEM_H - th) / 2;
            // (the typed part in the match colour)
            let label = &it.label;
            let cut = label.char_indices().nth(typed_len).map_or(label.len(), |(b, _)| b);
            let (a, b) = label.split_at(if label.to_lowercase().starts_with(&c.doc.slice(list.start..head.max(list.start)).to_lowercase()) { cut } else { 0 });
            let mut cx = lx + 30;
            if !a.is_empty() {
                let bold = Font { styles: 1, ..cf.clone() };
                p.text((cx, ty, text_size(a, &bold).0 + 2, th), a, &bold, if selected { fg } else { sc.popup_match }, Place::TopLeft);
                cx += text_size(a, &cf).0;
            }
            p.text((cx, ty, lw - (cx - lx) - 8, th), b, &cf, fg, Place::TopLeft);
            if !it.detail.is_empty() {
                // (the detail right-aligned, cut with "…" where it meets the label)
                let lab_end = lx + 30 + text_size(label, &cf).0 + 16;
                let room = lx + lw - 12 - lab_end;
                let detail = ellipsized(&it.detail, &uf, room);
                if !detail.is_empty() {
                    let dw = text_size(&detail, &uf).0;
                    let dx = lx + lw - 12 - dw;
                    p.text((dx, ty + 1, dw + 4, th), &detail, &uf, if selected { fg } else { sc.popup_detail }, Place::TopLeft);
                }
            }
        }
        // (a scroll indicator)
        if items.len() > rows {
            let track = lh - 6;
            let th = ((rows as f64 / items.len() as f64) * track as f64).max(16.0) as i64;
            let ty = ly + 3 + ((top as f64 / (items.len() - rows) as f64) * (track - th) as f64) as i64;
            p.op(Op::Round { rect: (lx + lw - 6, ty, 3, th), radius: 1.5, fill: Some(sc.minimap_slider), stroke: None, width: 1.0 });
        }
    });
    // the selected item's docs
    if let (Some(dr), Some(&i)) = (doc_rect, items.get(list.selected.min(items.len().saturating_sub(1)))) {
        let it = &list.items[i];
        let mut text = String::new();
        if !it.detail.is_empty() {
            text.push_str(&format!("```\n{}\n```\n", it.detail));
        }
        text.push_str(&it.doc);
        let bl = blocks(&text);
        if !bl.is_empty() {
            let h = (blocks_height(&bl, dr.2 - 20, code_px) + 10).clamp(40, 320);
            let r = (dr.0, dr.1, dr.2, h);
            frame(p, sc, r);
            draw_blocks(p, sc, &lang, &bl, (r.0 + 10, r.1 + 6, r.2 - 20, r.3 - 10), code_px);
        }
    }
}

/// Which popup of the editor is at component point (`mx`, `my`).
pub fn over_popup(x: &Ctx, abs: (i64, i64), mx: f64, my: f64) -> Option<usize> {
    let client = (i64::MAX / 4, i64::MAX / 4);
    let (r, _, _) = completion_rects(x.c, x.ui, abs, x.scale, client)?;
    let (px, py) = (abs.0 as f64 + mx, abs.1 as f64 + my);
    (px >= r.0 as f64 && py >= r.1 as f64 && px < (r.0 + r.2) as f64 && py < (r.1 + r.3) as f64).then_some(0)
}

/// A press in the form at (x, y): on a code editor's completion list, its
/// item accepted (true: the press was the list's).
pub fn popup_mouse_down(f: &mut FormUi, store: &dyn Store, ts: &mut TextSystem, x: f64, y: f64) -> bool {
    let client = (f.client.0, f.client.1 + f.menu_offset);
    let Some(i) = f.focus else { return false };
    let Some(ui) = f.nodes[i].ui.code.as_ref() else { return false };
    let abs = (f.nodes[i].abs.0, f.nodes[i].abs.1);
    let id = f.nodes[i].id.clone();
    let hit = with_code(&id, |c| {
        let (r, _, rows) = completion_rects(c, ui, abs, f.scale, client)?;
        let inside = x >= r.0 as f64 && y >= r.1 as f64 && x < (r.0 + r.2) as f64 && y < (r.1 + r.3) as f64;
        inside.then(|| {
            let k = ((y - r.1 as f64 - 2.0) / ITEM_H as f64).floor().max(0.0) as usize;
            (ui.completion_top + k.min(rows.saturating_sub(1)), r)
        })
    })
    .flatten();
    let Some((k, _)) = hit else {
        // (a press elsewhere closes the popups — the editor's own press too)
        let mut closed = false;
        for n in &f.nodes {
            if n.ui.code.is_some() {
                closed |= with_code_mut(&n.id, |c| {
                    let had = c.completion.is_some() || c.hover.is_some();
                    c.completion = None;
                    c.hover = None;
                    had
                })
                .unwrap_or(false);
            }
        }
        if closed {
            f.dirty = true;
        }
        return false;
    };
    f.dirty = true;
    with_code_mut(&id, |c| {
        if let Some(l) = &mut c.completion {
            l.selected = k;
        }
    });
    f.with_cx(store, ts, i, |_, cx| {
        super::with_view(cx, |x| {
            super::input::accept_completion(x);
            super::input::report_moves(x);
        })
    });
    true
}

/// The wheel over a completion list scrolls it.
pub fn popup_wheel(f: &mut FormUi, x: f64, y: f64, dy: f64) -> bool {
    let client = (f.client.0, f.client.1 + f.menu_offset);
    let Some(i) = f.focus else { return false };
    let scale = f.scale;
    let abs = (f.nodes[i].abs.0, f.nodes[i].abs.1);
    let id = f.nodes[i].id.clone();
    let Some(ui) = f.nodes[i].ui.code.as_mut() else { return false };
    let over = with_code(&id, |c| {
        let (r, _, rows) = completion_rects(c, ui, abs, scale, client)?;
        let n = c.completion_shown().len();
        (x >= r.0 as f64 && y >= r.1 as f64 && x < (r.0 + r.2) as f64 && y < (r.1 + r.3) as f64).then_some((n, rows))
    })
    .flatten();
    let Some((n, rows)) = over else { return false };
    let step = if dy > 0.0 { 3 } else { -3 };
    ui.completion_top = (ui.completion_top as i64 + step).clamp(0, n.saturating_sub(rows) as i64) as usize;
    f.dirty = true;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emphasis_is_dropped_and_arithmetic_kept() {
        assert_eq!(plain("*Choosing it adds `$INCLUDE`.* And **this**."), "Choosing it adds $INCLUDE. And this.");
        assert_eq!(plain("a * b, 2*3"), "a * b, 2*3");
    }

    #[test]
    fn blocks_and_wrapping() {
        let b = blocks("```basic\nSUB Go(x AS INTEGER)\n```\nRuns **it**.");
        assert_eq!(b, vec![(true, "SUB Go(x AS INTEGER)".to_string()), (false, "Runs **it**.".to_string())]);
        let f = ui_font();
        let lines = wrap("one two three four five six seven", &f, 60);
        assert!(lines.len() > 2);
        assert_eq!(rapidr_value::objects::codeedit::match_tier("ShowModal", "smd"), Some(4));
    }
}
