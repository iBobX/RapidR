//! The code editor for screen readers: a multi-line text field whose value
//! is the lines around the caret (a 10 MB file never goes into a tree or a
//! page's element), described with the caret's line and column and the
//! caret line's problems; a polite live region (a status node) announcing
//! line / column moves and problems as they change; the completion list
//! as a list box with its active option selected. The window's lines are
//! text runs too (one per line, a long line's pieces), with their
//! characters, words and (the lines in view) where each character is
//! drawn, and the primary selection in them: screen readers read the code
//! by character, word and line and follow the caret.

use rapidr_editor::{Buffer, CharClass};
use rapidr_value::objects::a11y::{node_id, AccessNode, Action, Role, TextInfo, TextPos, TextRun};
use rapidr_value::objects::codeedit::CodeEditor as Model;

use super::paint::{RowText, Shown};
use super::view::{display_cols, x_of};
use super::{popup, with_view, Ctx};
use crate::a11y::AccessValue;
use crate::components::{shared_describe, Cx};

/// The node of the editor.
pub fn describe(cx: &mut Cx) -> AccessNode {
    let mut n = shared_describe(cx, "RCODEEDITOR");
    let (x0, y0) = (cx.rect.0, cx.rect.1);
    let extra = with_view(cx, |x| {
        let head = x.c.doc.selections().primary().head;
        let (line, col) = x.c.line_col(head);
        let (errors, warnings, here) = x.c.line_severity_summary();
        let mut d = format!("Line {line}, column {col}");
        if x.c.doc.selections().len() > 1 {
            d.push_str(&format!(", {} cursors", x.c.doc.selections().len()));
        }
        if errors + warnings > 0 {
            d.push_str(&format!("; {errors} errors, {warnings} warnings"));
        }
        for h in &here {
            d.push_str("; ");
            d.push_str(h);
        }
        let mut kids = Vec::new();
        // (the live region: what was just announced)
        let mut live = AccessNode::new(node_id(&format!("{}$live", x.id)), Role::Status);
        live.name = x.ui.announce.clone();
        live.bounds = (x0, y0, 0, 0);
        kids.push(live);
        // (the completion list, its active item selected)
        if let Some(list) = &x.c.completion {
            let items = popup::filtered(x);
            if !items.is_empty() {
                let mut lb = AccessNode::new(node_id(&format!("{}$completion", x.id)), Role::ListBox);
                lb.name = "Suggestions".into();
                lb.bounds = (x0, y0, 0, 0);
                let sel = list.selected.min(items.len() - 1);
                for (k, &i) in items.iter().enumerate().take(200) {
                    let it = &list.items[i];
                    let mut o = AccessNode::new(node_id(&format!("{}$completion${k}", x.id)), Role::ListBoxOption);
                    o.name = if it.detail.is_empty() { format!("{}, {}", it.label, it.kind.name()) } else { format!("{}, {}, {}", it.label, it.kind.name(), it.detail) };
                    o.states.selected = Some(k == sel);
                    o.states.focused = k == sel;
                    o.bounds = (x0, y0, 0, 0);
                    lb.children.push(o);
                }
                kids.push(lb);
            }
        }
        (d, kids, text_info(x, (x0, y0)))
    });
    if let Some((d, kids, text)) = extra {
        n.text = Some(Box::new(text));
        // (after its AccessibleDescription: the kernel's name rule)
        n.description = d;

        n.states.multiline = true;
        n.children.extend(kids);
    }
    if n.name.is_empty() {
        n.name = "Code editor".into();
    }
    n
}

/// The characters a run holds at most (AccessKit's word starts are
/// bytes): a longer line is several runs.
const RUN_CHARS: usize = 250;

/// The window's lines as text runs (their texts joined: the node's value,
/// `Model::text_window`'s) and the primary selection in them; `(x0, y0)`
/// the component's place in the form.
pub fn text_info(x: &Ctx, (x0, y0): (i64, i64)) -> TextInfo {
    let (first, last) = x.c.window_lines(Model::WINDOW_RADIUS);
    let buf = x.c.doc.buffer();
    let lh = x.ui.lh();
    let s = x.scale.max(0.01);
    let g = x.ui.geo;
    let tab = x.c.doc.tab_size as usize;
    let ch = x.ui.metrics.ch;
    // (the layouts' left, the component's logical pixels: paint.rs's
    // layout_rows)
    let left = g.text.0 as f64 - x.ui.scroll.0;
    let mut runs: Vec<TextRun> = Vec::new();
    // (each run's line and bytes of it, its break not counted)
    let mut spans: Vec<(usize, usize, usize)> = Vec::new();
    for line in first..=last {
        let text = buf.line_text(line);
        let brk = if line < last { buf.slice(buf.line_end(line)..buf.line_start(line + 1)).into_owned() } else { String::new() };
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        // (words as Ctrl+Left / Right stop: a run of one class, its
        // trailing spaces with it; a line's leading spaces a word)
        let class: Vec<CharClass> = chars.iter().map(|&(_, c)| x.c.doc.char_class(c)).collect();
        let word_start = |i: usize| i == 0 || (class[i] != CharClass::Space && class[i] != class[i - 1]);
        // (the line's rows as drawn this frame, when they're all in view
        // and current; else the line unlaid)
        let pieces = laid_pieces(x, line, &text, tab);
        let pieces: Vec<(std::ops::Range<usize>, Option<&Shown>)> = match pieces {
            Some(p) => p.into_iter().map(|sh| (sh.row.range.clone(), Some(sh))).collect(),
            None => vec![(0..text.len(), None)],
        };
        let hidden = x.ui.rows.hidden(line);
        let mut k = 0;
        let n_pieces = pieces.len();
        for (pi, (range, sh)) in pieces.into_iter().enumerate() {
            // (the piece's characters, at most RUN_CHARS a run)
            let cs: Vec<usize> = (0..chars.len()).filter(|&i| range.contains(&chars[i].0)).collect();
            let groups: Vec<&[usize]> = if cs.is_empty() { vec![&[][..]] } else { cs.chunks(RUN_CHARS).collect() };
            let n_groups = groups.len();
            for (gi, grp) in groups.into_iter().enumerate() {
                let a = grp.first().map_or(range.start, |&i| chars[i].0);
                let b = grp.last().map_or(range.end, |&i| chars[i].0 + chars[i].1.len_utf8());
                let line_end = pi + 1 == n_pieces && gi + 1 == n_groups;
                let mut run = TextRun {
                    id: node_id(&if k == 0 { format!("{}$line${line}", x.id) } else { format!("{}$line${line}${k}", x.id) }),
                    text: text[a..b].to_string(),
                    continues: !line_end,
                    ..TextRun::default()
                };
                run.char_lengths = grp.iter().map(|&i| chars[i].1.len_utf8() as u8).collect();
                run.word_starts = grp.iter().enumerate().filter(|&(_, &i)| word_start(i)).map(|(j, _)| j as u8).collect();
                if line_end && !brk.is_empty() {
                    run.text.push_str(&brk);
                    run.char_lengths.push(brk.len() as u8);
                }
                match sh.and_then(|sh| x.ui.cache.layout(sh.slot).map(|l| (sh, l))) {
                    Some((sh, layout)) => {
                        // (each character from its layout, device pixels to
                        // logical; a tab as wide as drawn)
                        let at = |byte: usize| x0 as f64 + left + x_of(layout, sh.text.disp(byte)) / s;
                        let bx = at(a).floor();
                        let mut pos = Vec::with_capacity(run.char_lengths.len());
                        let mut wid = Vec::with_capacity(run.char_lengths.len());
                        for &i in grp {
                            let (c0, c1) = (at(chars[i].0), at(chars[i].0 + chars[i].1.len_utf8()));
                            pos.push((c0 - bx) as f32);
                            wid.push((c1 - c0).max(0.0) as f32);
                        }
                        let end = at(b);
                        if run.char_lengths.len() > grp.len() {
                            // (the line break: where it would be drawn, no width)
                            pos.push((end - bx) as f32);
                            wid.push(0.0);
                        }
                        run.char_positions = pos;
                        run.char_widths = wid;
                        run.bounds = (bx as i64, y0 + sh.top.round() as i64, ((end.ceil() - bx) as i64).max(1), lh as i64);
                    }
                    None => {
                        // (not in view: where its row is, a column's width a
                        // character)
                        let row = x.ui.rows.row_of_line(line) as f64;
                        let top = g.text.1 as f64 + row * lh - x.ui.scroll.1;
                        let col = display_cols(&text[..a], tab) as f64;
                        let w = display_cols(&text[a..b], tab) as f64 * ch;
                        let (w, h) = if hidden { (0, 0) } else { ((w.ceil() as i64).max(1), lh as i64) };
                        run.bounds = (x0 + (left + col * ch).floor() as i64, y0 + top.round() as i64, w, h);
                    }
                }
                runs.push(run);
                spans.push((line, a, b));
                k += 1;
            }
        }
    }
    // (the primary selection, clamped to the window)
    let (wa, wb) = (buf.line_start(first), buf.line_end(last));
    let pos_of = |byte: usize| -> TextPos {
        let byte = byte.clamp(wa, wb);
        let line = buf.line_of(byte);
        let col = byte - buf.line_start(line);
        let i0 = spans.partition_point(|&(l, _, _)| l < line);
        let i1 = spans.partition_point(|&(l, _, _)| l <= line);
        // (the run holding it: at a piece's end, the next piece's start;
        // the line's end on its last, before its break)
        let run = (i0..i1).find(|&r| col < spans[r].2).unwrap_or(i1.saturating_sub(1).max(i0));
        let Some(&(_, a, _)) = spans.get(run) else { return TextPos::default() };
        let text = &runs[run].text;
        let upto = col.saturating_sub(a).min(text.len());
        let upto = (0..=upto).rev().find(|&u| text.is_char_boundary(u)).unwrap_or(0);
        TextPos { run, char_index: text[..upto].chars().count() }
    };
    let p = x.c.doc.selections().primary();
    let selection = (!runs.is_empty()).then(|| (pos_of(p.anchor), pos_of(p.head)));
    TextInfo { runs, selection }
}

/// Line `line`'s rows as drawn this frame, when every row of it is in
/// view and its layout is of the line as it is now (else `None`).
fn laid_pieces<'a>(x: &'a Ctx, line: usize, text: &str, tab: usize) -> Option<Vec<&'a Shown>> {
    let shown: Vec<&Shown> = x.ui.shown.iter().filter(|sh| sh.row.line == line).collect();
    let (first, last) = (shown.first()?, shown.last()?);
    if first.row.range.start != 0 || last.row.range.end != text.len() || shown.windows(2).any(|w| w[0].row.range.end != w[1].row.range.start) {
        return None;
    }
    for sh in &shown {
        let r = sh.row.range.clone();
        if r.end > text.len() || !text.is_char_boundary(r.start) || !text.is_char_boundary(r.end) {
            return None;
        }
        // (a composition shows inside it: its characters still where drawn)
        if sh.text.preedit.is_none() && RowText::new(text, r.clone(), tab, display_cols(&text[..r.start], tab), None).text != sh.text.text {
            return None;
        }
        x.ui.cache.layout(sh.slot)?;
    }
    Some(shown)
}

/// A screen reader's request: focus, or (SetValue) the window of lines it
/// was shown replaced by its new text, as typing would.
pub fn access(cx: &mut Cx, action: Action, _part: Option<usize>, value: Option<&AccessValue>) -> bool {
    let (Action::SetValue, Some(AccessValue::Text(s))) = (action, value) else { return action == Action::Focus };
    let s = s.clone();
    let changed = with_view(cx, |x| {
        if x.c.doc.read_only {
            return false;
        }
        let w = x.c.text_window(Model::WINDOW_RADIUS);
        if w.text == s {
            return false;
        }
        let a = x.c.byte_of(w.first);
        let b = a + w.text.len();
        let _ = x.c.doc.replace_range(a..b, &s, super::input::now_ms());
        super::input::edited(x, None);
        true
    })
    .unwrap_or(false);
    let _ = changed;
    true
}

/// The caret moved: its line and column announced (and the new line's
/// problems).
pub fn caret_moved(x: &mut Ctx, line_changed: bool) {
    let (line, col) = x.ui.last_caret;
    x.ui.announce = if line_changed {
        let (_, _, here) = x.c.line_severity_summary();
        let mut a = format!("Line {line}");
        for h in here {
            a.push_str(", ");
            a.push_str(&h);
        }
        a
    } else {
        format!("Column {col}")
    };
}

/// The completion list's active item changed: announced.
pub fn completion_moved(x: &mut Ctx) {
    let items = popup::filtered(x);
    if let Some(list) = &x.c.completion {
        if let Some(&i) = items.get(list.selected.min(items.len().saturating_sub(1))) {
            let it = &list.items[i];
            x.ui.announce = format!("{} ({}), {} of {}", it.label, it.kind.name(), list.selected + 1, items.len());
        }
    }
}

/// The problems changed: the counts and the caret line's announced.
pub fn announce_line_problems(x: &mut Ctx) {
    let (errors, warnings, here) = x.c.line_severity_summary();
    x.ui.announce = if here.is_empty() { format!("{errors} errors, {warnings} warnings") } else { here.join("; ") };
}
