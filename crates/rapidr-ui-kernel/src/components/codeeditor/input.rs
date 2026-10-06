//! The code editor's input: keys (VS Code's map, Cmd for Ctrl on macOS),
//! the mouse (carets, Alt+click for another caret, Alt+Shift+drag for a
//! column, double / triple clicks, the gutter, the minimap, the scroll
//! bars, Ctrl / Cmd+click to a definition), the wheel (Ctrl: zoom), input
//! methods, and the view's ticks (a drag past the edge scrolling on, idle
//! colouring, folds, the language service's timers).

use std::cell::Cell;

use rapidr_editor::{Buffer, Direction, EditKind, Selection, Selections};
use rapidr_value::input::Button;
use rapidr_value::objects::ops::Rect;
use rapidr_value::{v_int, v_str};

use super::paint::{layout_rows, Shown};
use super::view::{display_cols, index_at, is_wide};
use super::{find, lang, popup, reveal_caret, update_bars, with_view, Ctx, Drag, Unit};
use crate::components::{Cx, Ime, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::input::{Clipboard, Mods};
use crate::tick::{now, Instant};

thread_local! {
    static EPOCH: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Milliseconds on the kernel's clock (undo grouping).
pub fn now_ms() -> u64 {
    let t = now();
    let start = EPOCH.with(|e| {
        let s = e.get().unwrap_or(t);
        e.set(Some(s));
        s
    });
    t.saturating_duration_since(start).as_millis() as u64
}

/// Sets the node's wake to the view's next deadline.
pub(super) fn schedule(cx: &mut Cx) {
    if let Some(at) = cx.ui.code.as_ref().and_then(|c| c.next_wake()) {
        cx.ui.wake = Some(cx.ui.wake.map_or(at, |w| w.min(at)));
    }
}

/// What every user edit is followed by: the model's marks, Modified,
/// OnChange, the caret in view, the language service.
pub fn edited(x: &mut Ctx, typed: Option<char>) {
    x.c.edited();
    x.c.modified = x.c.doc.is_modified();
    x.c.unhide_carets();
    x.change();
    x.ui.idle_at = Some(now() + std::time::Duration::from_millis(300));
    lang::after_edit(x, typed);
    reveal_caret(x, false);
    snippet_follow(x);
}

/// Tells the program the caret moved or the selections changed (the
/// user's doing), and screen readers the line / column.
pub fn report_moves(x: &mut Ctx) {
    let head = x.c.doc.selections().primary().head;
    let (line, col) = x.c.line_col(head);
    if (line, col) != x.ui.last_caret {
        let line_changed = line != x.ui.last_caret.0;
        x.ui.last_caret = (line, col);
        x.fire("oncaretmove", vec![v_int(line as i64), v_int(col as i64)]);
        super::access::caret_moved(x, line_changed);
    }
    let sels: Vec<(usize, usize)> = x.c.doc.selections().iter().map(|s| (s.anchor, s.head)).collect();
    if sels != x.ui.last_selections {
        x.ui.last_selections = sels;
        x.fire("onselectionchange", vec![]);
    }
}

/// The byte under component point (`mx`, `my`) (logical), the rows laid
/// out for the window at `abs` (clamped to the rows in view).
fn byte_at(x: &mut Ctx, abs: (i64, i64), mx: f64, my: f64) -> usize {
    layout_rows(x, abs);
    let lh = x.ui.lh();
    let g = x.ui.geo;
    let row = ((my - g.text.1 as f64 + x.ui.scroll.1) / lh).floor().max(0.0) as usize;
    let sh: Option<Shown> = x.ui.shown.iter().find(|s| s.index == row).cloned().or_else(|| {
        if row < x.ui.shown.first().map_or(0, |s| s.index) { x.ui.shown.first().cloned() } else { x.ui.shown.last().cloned() }
    });
    let Some(sh) = sh else { return x.c.doc.len_bytes() };
    let dev_x = (abs.0 as f64 + mx) * x.scale;
    let Some(layout) = x.ui.cache.layout(sh.slot) else { return sh.line_start };
    let d = index_at(layout, dev_x - sh.origin.0);
    let mut at = sh.line_start + sh.text.src(d);
    // (past a wrapped row's end: its last character, not the next row's)
    let line_len = x.c.doc.line(sh.row.line).len();
    if sh.row.range.end < line_len && at >= sh.line_start + sh.row.range.end {
        at = x.c.doc.buffer().prev_boundary(sh.line_start + sh.row.range.end);
    }
    at
}

/// A display column within row `piece` of `line` → its byte (clamped).
fn byte_at_col(x: &Ctx, line: usize, piece: &std::ops::Range<usize>, last_piece: bool, goal: usize) -> usize {
    let text = x.c.doc.line(line);
    let tab = x.c.doc.tab_size as usize;
    let base = display_cols(&text[..piece.start], tab);
    let mut col = base;
    let start = x.c.doc.buffer().line_start(line);
    for (i, ch) in text[piece.clone()].char_indices() {
        let w = match ch {
            '\t' => tab - col % tab,
            c if is_wide(c) => 2,
            _ => 1,
        };
        let c0 = col - base;
        if c0 >= goal {
            return start + piece.start + i;
        }
        if c0 + w > goal {
            // (inside a tab or a wide character: nearer its start or end)
            return start + piece.start + i + if goal - c0 >= w.div_ceil(2) { ch.len_utf8() } else { 0 };
        }
        col += w;
    }
    let end = start + piece.end;
    if last_piece {
        end
    } else {
        x.c.doc.buffer().prev_boundary(end)
    }
}

/// Up / Down by `delta` rows (folds skipped, wrapped rows counted), each
/// caret keeping its column.
pub fn move_vertical(x: &mut Ctx, delta: isize, extend: bool) {
    let total = x.ui.rows.total();
    let sels: Vec<Selection> = x.c.doc.selections().ranges().to_vec();
    let primary = x.c.doc.selections().primary_index();
    let tab = x.c.doc.tab_size as usize;
    let mut out = Vec::with_capacity(sels.len());
    for s in sels {
        // (a selection collapses to its edge first, as editors do)
        let from = if !extend && !s.is_empty() { if delta < 0 { s.start() } else { s.end() } } else { s.head };
        let (row, piece) = x.ui.rows.row_at(x.c, from);
        let line_text = x.c.doc.line(piece.line);
        let ls = x.c.doc.buffer().line_start(piece.line);
        let col_now = display_cols(&line_text[..from - ls], tab) - display_cols(&line_text[..piece.range.start], tab);
        drop(line_text);
        let goal = s.goal.map_or(col_now, |g| g as usize);
        let target = row as isize + delta;
        let head = if target < 0 {
            0
        } else if target as usize >= total {
            x.c.doc.len_bytes()
        } else {
            let r = x.ui.rows.rows(x.c, target as usize, 1);
            match r.first() {
                Some(r) => {
                    let n = x.c.doc.line(r.line).len();
                    byte_at_col(x, r.line, &r.range, r.range.end >= n, goal)
                }
                None => from,
            }
        };
        let anchor = if extend { s.anchor } else { head };
        out.push(Selection { anchor, head, goal: Some(goal as u32) });
    }
    x.c.doc.set_selections(Selections::new(out, primary));
}

/// Moves the touched lines up or down one (Alt+Up / Down), or copies them
/// (with Shift), one undo step.
fn move_lines(x: &mut Ctx, up: bool, copy: bool) {
    let buf = x.c.doc.buffer();
    let p = x.c.doc.selections().primary();
    let first = buf.line_of(p.start());
    let mut last = buf.line_of(p.end());
    if last > first && p.end() == buf.line_start(last) {
        last -= 1;
    }
    let n = buf.len_lines();
    if !copy && ((up && first == 0) || (!up && last + 1 >= n)) {
        return;
    }
    let start = buf.line_start(first);
    let end = buf.line_end(last);
    let block = buf.slice(start..end).into_owned();
    let (change, shift): (rapidr_editor::transaction::Change, isize) = if copy {
        if up {
            (rapidr_editor::transaction::Change::insert(start, format!("{block}\n")), 0)
        } else {
            (rapidr_editor::transaction::Change::insert(end, format!("\n{block}")), (block.len() + 1) as isize)
        }
    } else if up {
        let prev_start = buf.line_start(first - 1);
        let prev = buf.slice(prev_start..start - 1).into_owned();
        (rapidr_editor::transaction::Change::new(prev_start..end, format!("{block}\n{prev}")), -((prev.len() + 1) as isize))
    } else {
        let next_end = buf.line_end(last + 1);
        let next = buf.slice(end + 1..next_end).into_owned();
        (rapidr_editor::transaction::Change::new(start..next_end, format!("{next}\n{block}")), (next.len() + 1) as isize)
    };
    let len = x.c.doc.len_bytes();
    let Ok(set) = rapidr_editor::ChangeSet::new(vec![change], len) else { return };
    let mv = |b: usize| (b as isize + shift).max(0) as usize;
    let after = Selections::single(Selection::new(mv(p.anchor), mv(p.head)));
    if x.c.doc.apply(set, after, EditKind::Command, now_ms()).is_ok() {
        edited(x, None);
    }
}

/// Inserts a line below (or above) the caret's and moves there
/// (Ctrl+Enter / Ctrl+Shift+Enter).
fn open_line(x: &mut Ctx, above: bool) {
    let buf = x.c.doc.buffer();
    let line = buf.line_of(x.c.doc.selections().primary().head);
    let at = if above { buf.line_start(line) } else { buf.line_end(line) };
    x.c.doc.set_selections(Selections::caret(at));
    if above {
        let indent = x.c.doc.indent_of(line);
        let len = x.c.doc.len_bytes();
        let Ok(set) = rapidr_editor::ChangeSet::new(vec![rapidr_editor::transaction::Change::insert(at, format!("{indent}\n"))], len) else { return };
        let _ = x.c.doc.apply(set, Selections::caret(at + indent.len()), EditKind::Command, now_ms());
    } else {
        let _ = x.c.doc.newline(now_ms());
    }
    edited(x, None);
}

/// Accepts the completion list's selected item: its text (a snippet
/// expanded) over the word typed so far.
pub fn accept_completion(x: &mut Ctx) -> bool {
    let items = popup::filtered(x);
    let Some(list) = x.c.completion.clone() else { return false };
    let Some(&i) = items.get(list.selected.min(items.len().saturating_sub(1))) else {
        x.c.completion = None;
        return false;
    };
    let item = list.items[i].clone();
    x.c.completion = None;
    let head = x.c.doc.selections().primary().head;
    let start = list.start.min(head);
    // (the rest of the word after the caret goes too)
    let lang = x.c.doc.language().clone();
    let mut end = head;
    while let Some(c) = x.c.doc.buffer().char_after(end) {
        if !lang.is_word_char(c) {
            break;
        }
        end += c.len_utf8();
    }
    let text = item.insert.clone().unwrap_or_else(|| item.label.clone());
    x.c.doc.set_selections(Selections::single(Selection::new(start, end)));
    if item.snippet {
        let line = x.c.doc.buffer().line_of(start);
        let indent = x.c.doc.indent_of(line);
        let unit = x.c.doc.indent_unit();
        let exp = rapidr_editor::snippet::expand(&text, &indent, &unit, "\n");
        let len = x.c.doc.len_bytes();
        let Ok(set) = rapidr_editor::ChangeSet::new(vec![rapidr_editor::transaction::Change::new(start..end, exp.text.clone())], len) else { return false };
        let first = exp.stops.first().map(|r| (start + r.start, start + r.end)).unwrap_or((start + exp.text.len(), start + exp.text.len()));
        let _ = x.c.doc.apply(set, Selections::single(Selection::new(first.0, first.1)), EditKind::Command, now_ms());
        let stops: Vec<(usize, usize)> = exp.stops.iter().map(|r| (start + r.start, start + r.end)).collect();
        x.c.anchors = stops.iter().flat_map(|&(a, b)| [(a, true), (b, false)]).collect();
        x.ui.snippet = (stops.len() > 1).then(|| super::SnippetSession { stops, current: 0 });
        edited(x, None);
    } else {
        let len = x.c.doc.len_bytes();
        let Ok(set) = rapidr_editor::ChangeSet::new(vec![rapidr_editor::transaction::Change::new(start..end, text.clone())], len) else { return false };
        let _ = x.c.doc.apply(set, Selections::caret(start + text.len()), EditKind::Command, now_ms());
        edited(x, None);
        x.c.completion = None;
    }
    true
}

/// A snippet's stops follow the text (their anchors); the session ends
/// when the caret leaves them.
fn snippet_follow(x: &mut Ctx) {
    let Some(sn) = &mut x.ui.snippet else { return };
    let anchors = &x.c.anchors;
    if anchors.len() != sn.stops.len() * 2 {
        x.ui.snippet = None;
        return;
    }
    for (i, st) in sn.stops.iter_mut().enumerate() {
        *st = (anchors[2 * i].0, anchors[2 * i + 1].0.max(anchors[2 * i].0));
    }
    let head = x.c.doc.selections().primary().head;
    let inside = sn.stops.iter().any(|&(a, b)| a <= head && head <= b);
    if !inside {
        x.ui.snippet = None;
        x.c.anchors.clear();
    }
}

/// Tab / Shift+Tab in a snippet: to its next / previous stop (`false`:
/// past the last one, the session ends).
fn snippet_jump(x: &mut Ctx, back: bool) -> bool {
    let Some(sn) = &mut x.ui.snippet else { return false };
    let next = if back { sn.current.checked_sub(1) } else { Some(sn.current + 1) };
    match next.filter(|&n| n < sn.stops.len()) {
        Some(n) => {
            sn.current = n;
            let (a, b) = sn.stops[n];
            x.c.doc.set_selections(Selections::single(Selection::new(a, b)));
            if n + 1 == sn.stops.len() {
                x.ui.snippet = None;
                x.c.anchors.clear();
            }
            true
        }
        None => {
            x.ui.snippet = None;
            x.c.anchors.clear();
            false
        }
    }
}

/// A key: whether the editor took it.
pub fn key(cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
    let handled = with_view(cx, |x| key_in(x, k, clip)).unwrap_or(false);
    schedule(cx);
    handled
}

fn key_in(x: &mut Ctx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
    // (the find box has the keyboard while it's focused)
    if x.ui.find.as_ref().is_some_and(|f| f.focused) {
        let r = find::key(x, k, clip);
        report_moves(x);
        return r;
    }
    let m = k.mods;
    let cmd = m.command;
    let ro = x.c.doc.read_only;
    let t = now_ms();
    // the chord after Ctrl+K
    if std::mem::take(&mut x.ui.chord) {
        let done = match k.vk {
            48 => {
                x.c.fold_all();
                true
            }
            74 => {
                x.c.unfold_all();
                true
            }
            73 => {
                let head = x.c.doc.selections().primary().head;
                lang::hover(x, head);
                true
            }
            67 if !ro => {
                let _ = x.c.doc.toggle_line_comment(t);
                edited(x, None);
                true
            }
            _ => false,
        };
        if done {
            return true;
        }
    }
    // the completion list's keys
    if x.c.completion.is_some() {
        let n = popup::filtered(x).len();
        let sel = x.c.completion.as_ref().map_or(0, |c| c.selected);
        let page = popup::VISIBLE_ITEMS;
        let to = match (k.vk, m.shift, cmd) {
            (38, false, false) => Some(if sel == 0 { n.saturating_sub(1) } else { sel - 1 }),
            (40, false, false) => Some(if sel + 1 >= n { 0 } else { sel + 1 }),
            (33, false, false) => Some(sel.saturating_sub(page)),
            (34, false, false) => Some((sel + page).min(n.saturating_sub(1))),
            _ => None,
        };
        if let Some(to) = to {
            if let Some(c) = &mut x.c.completion {
                c.selected = to;
            }
            popup::keep_selected_visible(x);
            super::access::completion_moved(x);
            return true;
        }
        if (k.vk == 13 && !m.shift) || (k.vk == 9 && !m.shift) {
            return accept_completion(x);
        }
        if k.vk == 27 {
            x.c.completion = None;
            return true;
        }
    }
    if k.vk == 27 {
        if x.c.hover.take().is_some() {
            return true;
        }
        if x.c.signature.take().is_some() {
            x.ui.service.signature_open = false;
            return true;
        }
        if x.ui.find.is_some() {
            x.ui.find = None;
            return true;
        }
        if x.ui.snippet.take().is_some() {
            x.c.anchors.clear();
            return true;
        }
        if x.c.doc.selections().len() > 1 {
            x.c.doc.clear_cursors();
            report_moves(x);
            return true;
        }
        return false;
    }
    let shift = m.shift;
    let ctrl_like = cmd || m.ctrl;
    let mut moved = true;
    let mut edit: Option<Option<char>> = None;
    match (k.vk, ctrl_like, m.alt) {
        // ---- the clipboard, undo, selection ----
        (65, true, false) if !m.alt => {
            x.c.doc.select_all();
        }
        (67, true, false) | (45, true, false) if !shift => {
            clip.set_text(&x.c.doc.copy_text());
            moved = false;
        }
        (88, true, false) if !ro => {
            clip.set_text(&x.c.doc.copy_text());
            cut(x, t);
            edit = Some(None);
        }
        (86, true, false) if !ro && !shift => {
            if let Some(text) = clip.get_text() {
                let _ = x.c.doc.paste(&text.replace("\r\n", "\n"), t);
                edit = Some(None);
            }
        }
        (45, false, false) if shift && !ro => {
            if let Some(text) = clip.get_text() {
                let _ = x.c.doc.paste(&text.replace("\r\n", "\n"), t);
                edit = Some(None);
            }
        }
        (46, false, false) if shift && !ro => {
            clip.set_text(&x.c.doc.copy_text());
            cut(x, t);
            edit = Some(None);
        }
        (90, true, false) if !ro => {
            let did = if shift { x.c.doc.redo() } else { x.c.doc.undo() };
            if did {
                edit = Some(None);
            }
        }
        (89, true, false) if !ro => {
            if x.c.doc.redo() {
                edit = Some(None);
            }
        }
        (68, true, false) => {
            if shift {
                x.c.doc.select_all_occurrences();
            } else {
                x.c.doc.select_next_occurrence();
            }
        }
        (76, true, false) if shift => {
            x.c.doc.select_all_occurrences();
        }
        (76, true, false) => {
            x.c.doc.select_lines();
        }
        (75, true, false) if shift && !ro => {
            let _ = x.c.doc.delete_lines(t);
            edit = Some(None);
        }
        (75, true, false) => {
            x.ui.chord = true;
            moved = false;
        }
        (191, true, false) if !ro => {
            let _ = x.c.doc.toggle_line_comment(t);
            edit = Some(None);
        }
        (221, true, false) | (221, true, true) if shift || m.alt => {
            let line = x.c.doc.buffer().line_of(x.c.doc.selections().primary().head);
            x.c.unfold(line);
            moved = false;
        }
        (219, true, false) | (219, true, true) if shift || m.alt => {
            let line = x.c.doc.buffer().line_of(x.c.doc.selections().primary().head);
            x.c.fold(line);
            moved = false;
        }
        (221, true, false) if !ro => {
            let _ = x.c.doc.indent_lines(t);
            edit = Some(None);
        }
        (219, true, false) if !ro => {
            let _ = x.c.doc.outdent_lines(t);
            edit = Some(None);
        }
        (220, true, false) if shift => {
            let head = x.c.doc.selections().primary().head;
            if let Some((_, other)) = x.c.doc.matching_bracket(head) {
                x.c.doc.set_selections(Selections::caret(other));
            }
        }
        (13, true, false) if !ro => {
            open_line(x, shift);
            return true;
        }
        (83, true, false) => {
            x.fire("onsave", vec![]);
            return true;
        }
        (70, true, false) => {
            find::open(x, if shift { find::Mode::Find } else { find::Mode::Find });
            return true;
        }
        (72, true, false) | (70, true, true) => {
            find::open(x, find::Mode::Replace);
            return true;
        }
        (71, true, false) if m.ctrl => {
            find::open(x, find::Mode::Goto);
            return true;
        }
        (71, true, false) => {
            find::step(x, !shift);
        }
        (32, true, false) if shift => {
            lang::request_signature(x);
            return true;
        }
        (32, true, false) => {
            lang::request_completion(x, true);
            return true;
        }
        (187, true, false) | (107, true, false) => {
            x.c.opts.font_size = (x.c.opts.font_size + 1).min(72);
            return true;
        }
        (189, true, false) | (109, true, false) => {
            x.c.opts.font_size = (x.c.opts.font_size - 1).max(6);
            return true;
        }
        (48, true, false) => {
            x.c.opts.font_size = 10;
            return true;
        }
        // ---- function keys ----
        (114, _, _) => find::step(x, !shift),
        (113, false, false) if !ro => {
            find::open(x, find::Mode::Rename);
            return true;
        }
        (123, false, false) => {
            let head = x.c.doc.selections().primary().head;
            if shift {
                lang::references(x, head);
            } else {
                lang::goto_definition(x, head);
            }
        }
        (70, false, true) if shift && !ro => {
            if lang::format(x) {
                edit = Some(None);
            }
        }
        (90, false, true) => {
            x.c.opts.word_wrap = !x.c.opts.word_wrap;
            return true;
        }
        // ---- lines ----
        (38, false, true) | (40, false, true) if !ro && !cmd => {
            move_lines(x, k.vk == 38, shift);
            report_moves(x);
            return true;
        }
        (38, true, true) | (40, true, true) => {
            x.c.doc.add_cursor_vertical(if k.vk == 38 { Direction::Backward } else { Direction::Forward });
        }
        // ---- moves ----
        (37, _, _) | (39, _, _) => {
            let dir = if k.vk == 37 { Direction::Backward } else { Direction::Forward };
            if m.word {
                x.c.doc.move_word(dir, shift);
            } else if cmd {
                // (Cmd+Left / Right on macOS: the line's start / end)
                if k.vk == 37 {
                    x.c.doc.move_line_start(shift);
                } else {
                    x.c.doc.move_line_end(shift);
                }
            } else {
                // (a selection collapses to its side)
                let sels = x.c.doc.selections();
                if !shift && sels.iter().any(|s| !s.is_empty()) {
                    let to = sels.transform(|s| Selection::caret(if k.vk == 37 { s.start() } else { s.end() }));
                    x.c.doc.set_selections(to);
                } else {
                    x.c.doc.move_char(dir, shift);
                }
            }
        }
        (38, _, false) | (40, _, false) => {
            let down = k.vk == 40;
            if cmd && !m.ctrl {
                // (Cmd+Up / Down on macOS: the text's start / end)
                x.c.doc.move_doc(if down { Direction::Forward } else { Direction::Backward }, shift);
            } else if m.ctrl {
                // (Ctrl+Up / Down: scroll a line, the caret stays)
                x.ui.scroll.1 += if down { x.ui.lh() } else { -x.ui.lh() };
                update_bars(x);
                return true;
            } else {
                move_vertical(x, if down { 1 } else { -1 }, shift);
            }
        }
        (33, _, _) | (34, _, _) => {
            let rows = x.ui.full_rows().saturating_sub(1).max(1) as isize;
            let d = if k.vk == 33 { -rows } else { rows };
            move_vertical(x, d, shift);
            x.ui.scroll.1 += d as f64 * x.ui.lh();
            update_bars(x);
        }
        (36, _, _) | (35, _, _) => {
            let home = k.vk == 36;
            if ctrl_like {
                x.c.doc.move_doc(if home { Direction::Backward } else { Direction::Forward }, shift);
            } else if home {
                x.c.doc.move_line_start(shift);
            } else {
                x.c.doc.move_line_end(shift);
            }
        }
        // ---- editing ----
        (8, _, _) if !ro => {
            if m.word || cmd {
                let _ = x.c.doc.delete_word(Direction::Backward, t);
            } else {
                let _ = x.c.doc.backspace(t);
            }
            edit = Some(Some('\u{8}'));
        }
        (46, false, false) if !ro => {
            let _ = x.c.doc.delete_forward(t);
            edit = Some(None);
        }
        (46, true, _) if !ro => {
            let _ = x.c.doc.delete_word(Direction::Forward, t);
            edit = Some(None);
        }
        (9, false, false) if !ro => {
            if snippet_jump(x, shift) {
                report_moves(x);
                return true;
            }
            if shift {
                let _ = x.c.doc.outdent_lines(t);
            } else {
                let _ = x.c.doc.tab(t);
            }
            edit = Some(None);
        }
        (13, false, false) if !ro => {
            let _ = x.c.doc.newline(t);
            edit = Some(Some('\n'));
        }
        _ => {
            moved = false;
            let typed = k.text;
            let printable = !typed.is_empty() && !typed.chars().any(|c| c.is_control());
            // (a shortcut types nothing; AltGr / Option letters do)
            if printable && !cmd && !(m.ctrl && !m.alt) && !ro {
                let _ = x.c.doc.type_text(typed, t);
                let mut cs = typed.chars();
                let one = cs.next().filter(|_| cs.next().is_none());
                edit = Some(one);
            } else {
                return false;
            }
        }
    }
    match edit {
        Some(typed) => {
            let typed = typed.filter(|c| *c != '\u{8}');
            edited(x, typed);
            if k.vk == 8 && x.c.completion.is_some() && popup::filtered(x).is_empty() {
                x.c.completion = None;
            }
        }
        None if moved => {
            x.c.completion = None;
            x.c.hover = None;
            if x.ui.service.signature_open {
                lang::request_signature(x);
            }
            reveal_caret(x, false);
        }
        None => {}
    }
    report_moves(x);
    true
}

/// Cut: the selections (or, with none, the caret lines) removed.
fn cut(x: &mut Ctx, t: u64) {
    if x.c.doc.selections().iter().all(|s| s.is_empty()) {
        let _ = x.c.doc.delete_lines(t);
    } else {
        let _ = x.c.doc.paste("", t);
    }
}

/// The mouse.
pub fn mouse(cx: &mut Cx, m: &MouseIn) -> MouseOut {
    let abs = (cx.rect.0, cx.rect.1);
    let out = with_view(cx, |x| mouse_in(x, m, abs)).unwrap_or_default();
    schedule(cx);
    out
}

fn inside(r: Rect, x: f64, y: f64) -> bool {
    x >= r.0 as f64 && y >= r.1 as f64 && x < (r.0 + r.2) as f64 && y < (r.1 + r.3) as f64
}

fn mouse_in(x: &mut Ctx, m: &MouseIn, abs: (i64, i64)) -> MouseOut {
    let g = x.ui.geo;
    let (mx, my) = (m.x, m.y);
    // the find box
    if m.kind == MouseKind::Down && find::mouse_down(x, mx, my) {
        return MouseOut::default();
    }
    // the scroll bars
    let (bx, by, bw, bh) = g.bars;
    let (lx, ly) = ((mx - bx as f64).floor() as i64, (my - by as f64).floor() as i64);
    if m.kind == MouseKind::Down && m.button == Button::Left && x.ui.bars.on_bars(lx, ly, bw, bh) {
        x.ui.bars.mouse_down(lx, ly, bw, bh);
        x.ui.drag = Some(Drag::Bars);
        from_bars(x);
        x.ui.idle_at = Some(now() + crate::tick::REPEAT_DELAY);
        return MouseOut { press: false, focus: Some(false) };
    }
    if let Some(Drag::Bars) = x.ui.drag {
        match m.kind {
            MouseKind::Move => {
                x.ui.bars.mouse_drag(lx, ly, bw, bh);
            }
            MouseKind::Up => {
                x.ui.bars.mouse_up(bw, bh);
                x.ui.drag = None;
            }
            _ => {}
        }
        from_bars(x);
        return MouseOut { press: false, focus: Some(false) };
    }
    match m.kind {
        MouseKind::Down => {
            x.c.hover = None;
            x.ui.service.hover_at = None;
            if let Some(f) = &mut x.ui.find {
                f.focused = false;
            }
            // the minimap
            if let Some(mm) = g.minimap {
                if inside(mm, mx, my) {
                    let first = super::paint::minimap_first(x, mm);
                    let row = first + ((my - mm.1 as f64) / 2.0).max(0.0) as usize;
                    let page = x.ui.full_rows();
                    x.ui.scroll.1 = (row.saturating_sub(page / 2)) as f64 * x.ui.lh();
                    update_bars(x);
                    x.ui.drag = Some(Drag::Minimap { grab: (page / 2) as f64 });
                    return MouseOut { press: false, focus: Some(true) };
                }
            }
            // the gutter
            if inside(g.gutter, mx, my) {
                let row = ((my - g.text.1 as f64 + x.ui.scroll.1) / x.ui.lh()).floor().max(0.0) as usize;
                let (line, _) = x.ui.rows.line_of_row(row);
                let area = if mx >= g.fold_x as f64 && x.c.opts.show_folding {
                    if x.c.folded.contains(&line) {
                        x.c.unfold(line);
                    } else {
                        x.c.fold(line);
                    }
                    "fold"
                } else if mx < (g.glyph_x + 18) as f64 {
                    "marker"
                } else {
                    let buf = x.c.doc.buffer();
                    let (s, e) = (buf.line_start(line), if line + 1 < buf.len_lines() { buf.line_start(line + 1) } else { buf.len_bytes() });
                    let anchor = if m.mods.shift { x.c.doc.selections().primary().anchor } else { s };
                    x.c.doc.set_selections(Selections::single(Selection::new(anchor, e)));
                    x.ui.drag = Some(Drag::Lines { anchor: line, at: (mx, my) });
                    "number"
                };
                x.fire("ongutterclick", vec![v_int(line as i64 + 1), v_str(area)]);
                report_moves(x);
                return MouseOut::default();
            }
            if !inside(g.text, mx, my) {
                return MouseOut::default();
            }
            let at = byte_at(x, abs, mx, my);
            // (Ctrl / Cmd+click: to the definition)
            if m.mods.command && !m.mods.alt && m.clicks == 1 {
                x.c.doc.set_selections(Selections::caret(at));
                lang::goto_definition(x, at);
                report_moves(x);
                return MouseOut::default();
            }
            let unit = match m.clicks {
                0 | 1 => Unit::Char,
                2 => Unit::Word,
                _ => Unit::Line,
            };
            let column = m.mods.alt && m.mods.shift;
            let add = m.mods.alt && !m.mods.shift;
            let range = unit_range(x, at, unit);
            if column {
                let anchor = x.c.doc.selections().primary().anchor;
                x.ui.drag = Some(Drag::Text { anchor: (anchor, anchor), unit, add: false, column: true, at: (mx, my) });
                column_select(x, anchor, at);
            } else if add {
                let sels = x.c.doc.selections().add(Selection::new(range.0, range.1));
                x.c.doc.set_selections(sels);
                x.ui.drag = Some(Drag::Text { anchor: range, unit, add: true, column: false, at: (mx, my) });
            } else if m.mods.shift && unit == Unit::Char {
                let anchor = x.c.doc.selections().primary().anchor;
                x.c.doc.set_selections(Selections::single(Selection::new(anchor, at)));
                x.ui.drag = Some(Drag::Text { anchor: (anchor, anchor), unit, add: false, column: false, at: (mx, my) });
            } else {
                x.c.doc.set_selections(Selections::single(Selection::new(range.0, range.1)));
                x.ui.drag = Some(Drag::Text { anchor: range, unit, add: false, column: false, at: (mx, my) });
            }
            x.c.completion = None;
            if x.ui.service.signature_open {
                x.c.signature = None;
                x.ui.service.signature_open = false;
            }
            report_moves(x);
            MouseOut::default()
        }
        MouseKind::Move if m.captured => {
            match x.ui.drag.clone() {
                Some(Drag::Text { anchor, unit, add, column, .. }) => {
                    if let Some(Drag::Text { at, .. }) = &mut x.ui.drag {
                        *at = (mx, my);
                    }
                    let (cx_, cy_) = clamp_to(g.text, mx, my);
                    let at = byte_at(x, abs, cx_, cy_);
                    if column {
                        column_select(x, anchor.0, at);
                    } else {
                        drag_select(x, anchor, at, unit, add);
                    }
                    if !inside(g.text, mx, my) {
                        x.ui.idle_at = Some(now() + crate::tick::REPEAT);
                    }
                    report_moves(x);
                }
                Some(Drag::Lines { anchor, .. }) => {
                    let row = ((my - g.text.1 as f64 + x.ui.scroll.1) / x.ui.lh()).floor().max(0.0) as usize;
                    let (line, _) = x.ui.rows.line_of_row(row);
                    let buf = x.c.doc.buffer();
                    let next = |l: usize| if l + 1 < buf.len_lines() { buf.line_start(l + 1) } else { buf.len_bytes() };
                    let (a, h) = if line >= anchor { (buf.line_start(anchor), next(line)) } else { (next(anchor), buf.line_start(line)) };
                    x.c.doc.set_selections(Selections::single(Selection::new(a, h)));
                    report_moves(x);
                }
                Some(Drag::Minimap { grab }) => {
                    if let Some(mm) = g.minimap {
                        let first = super::paint::minimap_first(x, mm);
                        let row = first as f64 + ((my - mm.1 as f64) / 2.0).max(0.0) - grab;
                        x.ui.scroll.1 = row.max(0.0) * x.ui.lh();
                        update_bars(x);
                    }
                }
                _ => {}
            }
            MouseOut::default()
        }
        MouseKind::Move => {
            // (resting over the text: a hover in a moment; the gutter: its arrows)
            let in_gutter = inside(g.gutter, mx, my);
            x.ui.gutter_hover = in_gutter;
            if inside(g.text, mx, my) && popup::over_popup(x, abs, mx, my).is_none() {
                let at = byte_at(x, abs, mx, my);
                let keep = x.c.hover.as_ref().is_some_and(|h| h.start <= at && at <= h.end);
                if !keep {
                    x.c.hover = None;
                    x.ui.service.hover_at = Some((now() + lang::HOVER_AFTER, at));
                }
            } else if !in_gutter {
                x.ui.service.hover_at = None;
            }
            MouseOut::default()
        }
        MouseKind::Up => {
            x.ui.drag = None;
            MouseOut::default()
        }
        MouseKind::Leave => {
            x.ui.gutter_hover = false;
            x.ui.service.hover_at = None;
            MouseOut::default()
        }
    }
}

fn clamp_to(r: Rect, x: f64, y: f64) -> (f64, f64) {
    (x.clamp(r.0 as f64, (r.0 + r.2) as f64 - 1.0), y.clamp(r.1 as f64, (r.1 + r.3) as f64 - 1.0))
}

/// The word or line around byte `at` (the char unit: `at` itself).
fn unit_range(x: &Ctx, at: usize, unit: Unit) -> (usize, usize) {
    match unit {
        Unit::Char => (at, at),
        Unit::Word => x.c.doc.word_at(at).map_or_else(|| {
            let next = x.c.doc.buffer().next_boundary(at);
            (at, next)
        }, |r| (r.start, r.end)),
        Unit::Line => {
            let buf = x.c.doc.buffer();
            let line = buf.line_of(at);
            (buf.line_start(line), if line + 1 < buf.len_lines() { buf.line_start(line + 1) } else { buf.len_bytes() })
        }
    }
}

/// A drag selecting from `anchor` (the unit pressed on) to `at`.
fn drag_select(x: &mut Ctx, anchor: (usize, usize), at: usize, unit: Unit, add: bool) {
    let (a, b) = unit_range(x, at, unit);
    let sel = if at < anchor.0 { Selection::new(anchor.1, a) } else { Selection::new(anchor.0, b.max(at)) };
    if add {
        let mut v: Vec<Selection> = x.c.doc.selections().ranges().to_vec();
        let p = x.c.doc.selections().primary_index();
        v[p] = sel;
        x.c.doc.set_selections(Selections::new(v, p));
    } else {
        x.c.doc.set_selections(Selections::single(sel));
    }
}

/// Alt+Shift+drag: a caret (selection) on every line from `anchor`'s to
/// `at`'s, between their columns.
fn column_select(x: &mut Ctx, anchor: usize, at: usize) {
    let buf = x.c.doc.buffer();
    let tab = x.c.doc.tab_size as usize;
    let (la, lb) = (buf.line_of(anchor), buf.line_of(at));
    let col = |b: usize| {
        let l = buf.line_of(b);
        display_cols(&buf.line_text(l)[..b - buf.line_start(l)], tab)
    };
    let (ca, cb) = (col(anchor), col(at));
    let mut sels = Vec::new();
    let lines: Vec<usize> = if la <= lb { (la..=lb).collect() } else { (lb..=la).rev().collect() };
    for l in lines {
        let n = x.c.doc.line(l).len();
        let a = byte_at_col(x, l, &(0..n), true, ca);
        let h = byte_at_col(x, l, &(0..n), true, cb);
        sels.push(Selection::new(a, h));
    }
    let primary = sels.len().saturating_sub(1);
    x.c.doc.set_selections(Selections::new(sels, primary));
}

/// The bars moved: the view follows.
fn from_bars(x: &mut Ctx) {
    x.ui.scroll = (x.ui.bars.horz.position as f64, x.ui.bars.vert.position as f64);
    update_bars(x);
}

/// The wheel: rows (three a notch), across with Shift, the font's size
/// with Ctrl / Cmd.
pub fn wheel(cx: &mut Cx, dx: f64, dy: f64, mods: Mods) -> bool {
    with_view(cx, |x| {
        if mods.command || mods.ctrl {
            let d = if dy < 0.0 { 1 } else { -1 };
            x.c.opts.font_size = (x.c.opts.font_size + d).clamp(6, 72);
            return true;
        }
        x.c.hover = None;
        let (dx, dy) = if mods.shift && dx == 0.0 { (dy, 0.0) } else { (dx, dy) };
        let lh = x.ui.lh();
        let before = x.ui.scroll;
        x.ui.scroll.0 += dx * 3.0 * x.ui.metrics.ch * 2.0;
        x.ui.scroll.1 += dy * 3.0 * lh;
        update_bars(x);
        x.ui.scroll != before || x.ui.rows.total() > x.ui.full_rows()
    })
    .unwrap_or(false)
}

/// An input method's composition (dead keys, CJK) at the primary caret.
pub fn ime(cx: &mut Cx, ime: &Ime) -> bool {
    let r = with_view(cx, |x| {
        if x.c.doc.read_only {
            return false;
        }
        match ime {
            Ime::Preedit(text, cursor) => {
                x.ui.preedit = (!text.is_empty()).then(|| (text.clone(), *cursor));
            }
            Ime::Commit(text) => {
                x.ui.preedit = None;
                if !text.is_empty() {
                    let _ = x.c.doc.type_text(text, now_ms());
                    let mut cs = text.chars();
                    let one = cs.next().filter(|_| cs.next().is_none());
                    edited(x, one);
                    report_moves(x);
                }
            }
        }
        true
    })
    .unwrap_or(false);
    schedule(cx);
    r
}

/// Where the input method's window goes: the primary caret (absolute
/// logical pixels).
pub fn ime_area(cx: &mut Cx) -> Option<Rect> {
    let abs = (cx.rect.0, cx.rect.1);
    with_view(cx, |x| {
        layout_rows(x, abs);
        let head = x.c.doc.selections().primary().head;
        let s = x.scale;
        let sh = super::paint::shown_at(x.ui, x.c, head)?.clone();
        let dx = super::paint::caret_x(x.ui, &sh, head) / s;
        Some((dx.floor() as i64, abs.1 + sh.top.floor() as i64, 2, x.ui.metrics.line))
    })
    .flatten()
}

/// The program's requests (TriggerCompletion, FormatDocument, OpenFind …).
fn requests(x: &mut Ctx) {
    use rapidr_value::objects::codeedit::Request;
    for r in std::mem::take(&mut x.c.requests) {
        let head = x.c.doc.selections().primary().head;
        match r {
            Request::Completion => lang::request_completion(x, true),
            Request::Signature => lang::request_signature(x),
            Request::Hover(at) => lang::hover(x, at),
            Request::Format => {
                if lang::format(x) {
                    edited(x, None);
                }
            }
            Request::Definition => {
                lang::goto_definition(x, head);
            }
            Request::References => lang::references(x, head),
            Request::Rename(name) => {
                if let Err(e) = lang::rename(x, head, &name) {
                    x.ui.announce = e;
                }
            }
            Request::Find(mode) => find::open(
                x,
                match mode.as_str() {
                    "replace" => find::Mode::Replace,
                    "goto" => find::Mode::Goto,
                    "rename" => find::Mode::Rename,
                    _ => find::Mode::Find,
                },
            ),
        }
        report_moves(x);
    }
}

/// The view's deadlines: a drag past the edge scrolls on; idle colouring,
/// folds and change marks; the language service's timers.
pub fn tick(cx: &mut Cx) {
    let abs = (cx.rect.0, cx.rect.1);
    with_view(cx, |x| {
        let at = now();
        requests(x);
        if let Some(Drag::Text { anchor, unit, add, column, at: (mx, my) }) = x.ui.drag.clone() {
            let g = x.ui.geo;
            if !inside(g.text, mx, my) {
                let lh = x.ui.lh();
                if my < g.text.1 as f64 {
                    x.ui.scroll.1 -= lh;
                } else if my >= (g.text.1 + g.text.3) as f64 {
                    x.ui.scroll.1 += lh;
                }
                if mx < g.text.0 as f64 {
                    x.ui.scroll.0 -= x.ui.metrics.ch * 2.0;
                } else if mx >= (g.text.0 + g.text.2) as f64 {
                    x.ui.scroll.0 += x.ui.metrics.ch * 2.0;
                }
                update_bars(x);
                let (cx_, cy_) = clamp_to(g.text, mx, my);
                let b = byte_at(x, abs, cx_, cy_);
                if column {
                    column_select(x, anchor.0, b);
                } else {
                    drag_select(x, anchor, b, unit, add);
                }
                report_moves(x);
                x.ui.idle_at = Some(at + crate::tick::REPEAT);
                return;
            }
        }
        if let Some(Drag::Bars) = x.ui.drag {
            let g = x.ui.geo;
            let _ = g;
            x.ui.idle_at = Some(at + crate::tick::REPEAT);
            return;
        }
        if x.ui.idle_at.is_some_and(|t| t <= at) {
            x.ui.idle_at = None;
            // (colouring a slice of the text at a time)
            let done = x.c.doc.highlight_idle(20_000);
            if !done {
                x.ui.idle_at = Some(at + std::time::Duration::from_millis(16));
            } else {
                // (folds once everything is coloured; the change marks)
                let _ = x.c.fold_ranges();
                let text = x.c.doc.text();
                let (v, m) = (x.c.doc.version(), x.c.doc.is_modified());
                x.ui.changes.update(text, v, m);
            }
        }
        lang::tick(x, at);
    });
    schedule(cx);
}
