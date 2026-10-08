//! The code editor's language features: the runtime's language service
//! (`rapidr_editor::service`; RapidR's own for BASIC) asked for diagnostics
//! when typing stops, completion as a word starts or after a trigger
//! character, signature help after `(` and `,`, hovers when the mouse
//! rests, the language's words' case as each word ends, definitions,
//! renames and formatting — or, where no service serves the language, the
//! program asked through its events (OnCompletionRequest …), which it
//! answers with ShowCompletion, ShowHover and ShowSignature.
//!
//! Also the gutter's marks of what changed since the last save.

use std::sync::Arc;
use std::time::Duration;

use rapidr_editor::service::{self, Completion, CompletionKind};
use rapidr_editor::transaction::Change as TextChange;
use rapidr_editor::{Buffer, EditKind, Selection, Selections};
use rapidr_value::objects::codeedit::{DiagSource, Diagnostic, HoverInfo, SignatureInfo};
use rapidr_value::{v_int, v_str};

use super::Ctx;
use crate::tick::{now, Instant};

/// Diagnostics this long after the last keystroke.
pub const DIAGNOSE_AFTER: Duration = Duration::from_millis(350);
/// A hover this long after the mouse stops.
pub const HOVER_AFTER: Duration = Duration::from_millis(500);
/// Files bigger than this get no language service (as large-file modes).
pub const SERVICE_LIMIT: usize = 4 << 20;

/// Whether the service answers for this editor.
pub fn served(x: &Ctx) -> bool {
    x.c.opts.language_service && x.c.doc.len_bytes() <= SERVICE_LIMIT && service::available(&x.c.doc.language().id)
}

/// Gives the service the editor's text if it changed since.
fn sync(x: &mut Ctx) -> bool {
    if !served(x) {
        return false;
    }
    let key = (x.c.generation, x.c.doc.version());
    if x.ui.service.synced != Some(key) {
        let file = x.file();
        let text = x.c.doc.text();
        service::with(|s| s.update(&file, &text));
        x.ui.service.synced = Some(key);
    }
    true
}

/// After an edit by the user (`typed`: the one character typed): the
/// diagnostics' timer, completion, signature help, keyword case.
pub fn after_edit(x: &mut Ctx, typed: Option<char>) {
    x.ui.service.diagnose_at = Some(now() + DIAGNOSE_AFTER);
    x.c.hover = None;
    let Some(ch) = typed else {
        x.c.completion = None;
        return;
    };
    let head = x.c.doc.selections().primary().head;
    let served = served(x);
    // keyword case, as the word ends
    let names = x.c.opts.identifier_case == "declaration";
    if served && (x.c.opts.keyword_case != "preserve" || names) && x.c.doc.selections().len() == 1 {
        let triggers = service::with(|s| s.case_triggers()).unwrap_or(&[]);
        // (Enter: the line left is cased whole, as VB does; else the word
        // just finished)
        if triggers.contains(&ch) && (ch == '\n' || word_wants_case(x, head - ch.len_utf8())) {
            sync(x);
            let file = x.file();
            let case = if names { format!("{}+declaration", x.c.opts.keyword_case) } else { x.c.opts.keyword_case.clone() };
            let edits = service::with(|s| s.case_edits(&file, head, ch, &case)).unwrap_or_default();
            apply_service_edits(x, edits, EditKind::Command);
        }
    }
    let lang = x.c.doc.language().clone();
    let word_char = lang.is_word_char(ch);
    // completion: a word starting, a trigger character, more of the word
    let trigger = x.c.opts.completion_trigger.contains(ch);
    match &x.c.completion {
        Some(list) if word_char || (lang.keyword_suffixes.contains(ch) && head > list.start) => {
            // (filtered as the word grows; asked again when nothing is left)
            if super::popup::filtered(x).is_empty() {
                x.c.completion = None;
            }
        }
        _ => {
            x.c.completion = None;
            let start = x.c.word_start_before(head);
            let word_len = head - start;
            if trigger || (word_char && word_len == 1 && !in_literal(x, head.saturating_sub(1))) {
                request_completion(x, false);
            }
        }
    }
    // signature help
    let sig_triggers = if served { service::with(|s| s.signature_triggers()).unwrap_or(&['(', ',']) } else { &['(', ','] };
    if sig_triggers.contains(&ch) || (ch == ')' && x.ui.service.signature_open) || x.ui.service.signature_open {
        request_signature(x);
    }
}

/// Whether the word ending at byte `end` is one the service may case: a
/// keyword, type or builtin by the tokenizer, not already in the case.
fn word_wants_case(x: &mut Ctx, end: usize) -> bool {
    let start = x.c.word_start_before(end);
    if start == end {
        return false;
    }
    let line = x.c.doc.buffer().line_of(start);
    let ls = x.c.doc.buffer().line_start(line);
    let tokens = x.c.doc.tokens(line);
    let col = start - ls;
    let word = x.c.doc.slice(start..end).into_owned();
    let cased = match x.c.opts.keyword_case.as_str() {
        "upper" => word.to_ascii_uppercase(),
        "lower" => word.to_ascii_lowercase(),
        _ => String::new(),
    };
    // (the program's names as declared: any word outside strings and
    // comments — the service knows which are names)
    if x.c.opts.identifier_case == "declaration" && !tokens.iter().any(|t| t.kind.is_literal() && (t.start as usize) <= col && col < t.end as usize) {
        return true;
    }
    if !cased.is_empty() && cased == word {
        return false;
    }
    tokens.iter().any(|t| {
        let base = t.kind.base();
        (t.start as usize) <= col && col < t.end as usize && matches!(base, rapidr_editor::TokenKind::KEYWORD | rapidr_editor::TokenKind::TYPE | rapidr_editor::TokenKind::FUNCTION | rapidr_editor::TokenKind::CONSTANT | rapidr_editor::TokenKind::DIRECTIVE)
    })
}

/// Whether byte `at` is inside a string or a comment.
fn in_literal(x: &mut Ctx, at: usize) -> bool {
    let line = x.c.doc.buffer().line_of(at);
    let col = at - x.c.doc.buffer().line_start(line);
    x.c.doc.tokens(line).iter().any(|t| t.kind.is_literal() && (t.start as usize) <= col && col < t.end as usize)
}

/// Edits from the service (bytes of this file's text) as one step.
pub fn apply_service_edits(x: &mut Ctx, edits: Vec<service::Edit>, kind: EditKind) {
    if edits.is_empty() {
        return;
    }
    let len = x.c.doc.len_bytes();
    let mut changes: Vec<TextChange> = edits.into_iter().filter(|e| e.start <= e.end && e.end <= len).map(|e| TextChange::new(e.start..e.end, e.text)).collect();
    changes.sort_by_key(|c| c.range.start);
    let Ok(set) = rapidr_editor::ChangeSet::new(changes, len) else { return };
    let after = x.c.doc.selections().map(&set);
    let _ = x.c.doc.apply(set, after, kind, 0);
    x.c.edited();
    x.c.modified = x.c.doc.is_modified() || x.c.modified;
}

/// Opens completion at the primary caret: the service's items, else the
/// program's (OnCompletionRequest).
pub fn request_completion(x: &mut Ctx, explicit: bool) {
    let head = x.c.doc.selections().primary().head;
    if sync(x) {
        let file = x.file();
        let got = service::with(|s| s.completions(&file, head));
        if let Some(got) = got {
            let start = got.start.min(head);
            let mut items = got.items;
            if explicit && items.is_empty() {
                x.c.completion = None;
                return;
            }
            // (the language's snippets too, at a word's start)
            add_snippets(x, &mut items, start, head);
            x.c.show_completion(items, true, Some(start));
            x.ui.completion_top = 0;
            if super::popup::filtered(x).is_empty() {
                x.c.completion = None;
            }
            return;
        }
    }
    let start = x.c.word_start_before(head);
    let prefix = x.c.doc.slice(start..head).into_owned();
    let (line, col) = x.c.line_col(head);
    x.fire("oncompletionrequest", vec![v_int(line as i64), v_int(col as i64), v_str(&prefix)]);
}

/// The language definition's snippets whose prefix starts what's typed.
fn add_snippets(x: &Ctx, items: &mut Vec<Completion>, start: usize, head: usize) {
    let typed = x.c.doc.slice(start..head).to_ascii_lowercase();
    if typed.is_empty() {
        return;
    }
    for sn in &x.c.doc.language().snippets {
        if sn.prefix.to_ascii_lowercase().starts_with(&typed) {
            let mut c = Completion::new(sn.prefix.clone(), CompletionKind::Snippet);
            c.detail = sn.description.clone();
            c.insert = Some(sn.body.clone());
            c.snippet = true;
            c.sort = format!("~{}", sn.prefix);
            items.push(c);
        }
    }
}

/// Signature help at the primary caret (or closes it outside a call).
pub fn request_signature(x: &mut Ctx) {
    let head = x.c.doc.selections().primary().head;
    if sync(x) {
        let file = x.file();
        let got = service::with(|s| s.signature(&file, head)).flatten();
        match got.filter(|h| !h.signatures.is_empty()) {
            Some(h) => {
                let sig = &h.signatures[h.active_signature.min(h.signatures.len() - 1)];
                x.c.signature = Some(SignatureInfo { label: sig.label.clone(), params: sig.params.clone(), active: h.active_param, doc: sig.doc.clone(), at: head });
                x.ui.service.signature_open = true;
            }
            None => {
                x.c.signature = None;
                x.ui.service.signature_open = false;
            }
        }
        return;
    }
    let (line, col) = x.c.line_col(head);
    x.ui.service.signature_open = true;
    x.fire("onsignaturerequest", vec![v_int(line as i64), v_int(col as i64)]);
}

/// A hover at byte `at`: the problems there, then the service's (or the
/// program's: OnHoverRequest).
pub fn hover(x: &mut Ctx, at: usize) {
    let mut text = String::new();
    let mut range = x.c.doc.word_at(at).unwrap_or(at..at);
    for d in x.c.diagnostics.iter().filter(|d| d.start <= at && at <= d.end.max(d.start + 1)) {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&format!("{}: {}", d.severity.name(), d.message));
        range = d.start..d.end;
    }
    if sync(x) {
        let file = x.file();
        if let Some(h) = service::with(|s| s.hover(&file, at)).flatten() {
            if !text.is_empty() {
                text.push_str("\n\n");
            }
            text.push_str(&h.text);
            if range.is_empty() {
                range = h.start..h.end;
            }
        }
    } else if text.is_empty() {
        x.c.hover_request = Some(at);
        let (line, col) = x.c.line_col(at);
        x.fire("onhoverrequest", vec![v_int(line as i64), v_int(col as i64)]);
        return;
    }
    x.c.hover = (!text.trim().is_empty()).then_some(HoverInfo { text, start: range.start, end: range.end });
}

/// Diagnostics and semantic colours from the service, now.
pub fn diagnose(x: &mut Ctx) {
    x.ui.service.diagnose_at = None;
    if !sync(x) {
        return;
    }
    let file = x.file();
    let diags = service::with(|s| s.diagnostics(&file)).unwrap_or_default();
    let same = |f: &str| f == file || std::path::Path::new(f).file_name() == std::path::Path::new(&file).file_name() && f.ends_with(&file);
    let mine: Vec<Diagnostic> = diags
        .into_iter()
        .filter(|d| same(&d.file))
        .map(|d| Diagnostic { start: d.start, end: d.end.max(d.start), severity: d.severity, message: d.message, code: d.code, source: DiagSource::Service })
        .collect();
    let before = x.c.line_severity_summary();
    x.c.set_service_diagnostics(mine);
    let tokens = service::with(|s| s.semantic_tokens(&file)).unwrap_or_default();
    x.c.semantic = tokens.into_iter().map(|t| rapidr_value::objects::codeedit::SemanticSpan { start: t.start, end: t.end, kind: t.kind }).collect();
    // (the caret line's problems, for screen readers: announced when they change)
    if x.c.line_severity_summary() != before {
        super::access::announce_line_problems(x);
    }
}

/// Ctrl+. (Cmd+.): the service's fixes for the problems at the caret —
/// else on its line — as a list to pick from (Enter or Tab applies one, as
/// one undo step); a screen reader hears when there are none.
pub fn quick_fix(x: &mut Ctx) {
    x.c.completion = None;
    if !sync(x) {
        return;
    }
    let file = x.file();
    let head = x.c.doc.selections().primary().head;
    let buf = x.c.doc.buffer();
    let line = buf.line_of(head);
    let (ls, le) = (buf.line_start(line), buf.line_end(line));
    let mut fixes = service::with(|s| s.code_actions(&file, head, head)).unwrap_or_default();
    if fixes.is_empty() {
        fixes = service::with(|s| s.code_actions(&file, ls, le)).unwrap_or_default();
    }
    if fixes.is_empty() {
        x.ui.announce = "No quick fixes here".into();
        return;
    }
    x.c.show_fixes(fixes);
    x.ui.completion_top = 0;
    super::access::completion_moved(x);
}

/// F12 / Ctrl+click: to the definition (here, or OnNavigate(File, Line,
/// Column) for the program to open another file).
pub fn goto_definition(x: &mut Ctx, at: usize) -> bool {
    if !sync(x) {
        return false;
    }
    let file = x.file();
    let locs = service::with(|s| s.definition(&file, at)).unwrap_or_default();
    let Some(loc) = locs.into_iter().next() else { return false };
    if loc.file == file {
        x.c.doc.set_selections(Selections::single(Selection::new(loc.start, loc.end)));
        x.c.unhide_carets();
        super::reveal_byte(x, loc.start, true);
    } else {
        let pos = service::with(|s| s.position(&loc.file, loc.start)).flatten().unwrap_or((1, 1));
        x.fire("onnavigate", vec![v_str(&loc.file), v_int(pos.0 as i64), v_int(pos.1 as i64)]);
    }
    true
}

/// Shift+F12: the references, selected here (every one in this file as a
/// caret) and told to the program (OnReferences(Json)).
pub fn references(x: &mut Ctx, at: usize) {
    if !sync(x) {
        return;
    }
    let file = x.file();
    let locs = service::with(|s| s.references(&file, at)).unwrap_or_default();
    let mut json = Vec::new();
    for l in &locs {
        let (line, col) = if l.file == file { x.c.line_col(l.start) } else { service::with(|s| s.position(&l.file, l.start)).flatten().unwrap_or((1, 1)) };
        json.push(format!("{{\"file\":{},\"line\":{line},\"column\":{col}}}", json_str(&l.file)));
    }
    let here: Vec<Selection> = locs.iter().filter(|l| l.file == file).map(|l| Selection::new(l.start, l.end)).collect();
    if !here.is_empty() {
        let primary = here.iter().position(|s| s.start() <= at && at <= s.end()).unwrap_or(0);
        x.c.doc.set_selections(Selections::new(here, primary));
    }
    x.fire("onreferences", vec![v_str(&format!("[{}]", json.join(",")))]);
}

/// A JSON string.
pub fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// F2: renames what's at `at` everywhere (this file's edits applied, other
/// files' sent as OnFileEdits(File, Json)); the service's reason when it
/// refuses.
pub fn rename(x: &mut Ctx, at: usize, new_name: &str) -> Result<(), String> {
    if !sync(x) {
        return Err("No language service for this file".into());
    }
    let file = x.file();
    let edits = service::with(|s| s.rename(&file, at, new_name)).unwrap_or_else(|| Err("The language service is busy".into()))?;
    for (f, list) in edits {
        if f == file {
            apply_service_edits(x, list, EditKind::Command);
        } else {
            let items: Vec<String> = list.iter().map(|e| format!("{{\"start\":{},\"end\":{},\"text\":{}}}", e.start, e.end, json_str(&e.text))).collect();
            x.fire("onfileedits", vec![v_str(&f), v_str(&format!("[{}]", items.join(",")))]);
        }
    }
    x.change();
    Ok(())
}

/// Shift+Alt+F: the file formatted (one undo step).
pub fn format(x: &mut Ctx) -> bool {
    if !sync(x) {
        return false;
    }
    let file = x.file();
    let indent = if x.c.doc.insert_spaces { " ".repeat(x.c.doc.tab_size as usize) } else { "\t".into() };
    let edits = service::with(|s| s.format(&file, &indent)).unwrap_or_default();
    let any = !edits.is_empty();
    apply_service_edits(x, edits, EditKind::Command);
    any
}

/// The view's deadlines that came: diagnostics, a resting mouse's hover.
pub fn tick(x: &mut Ctx, at: Instant) {
    if x.ui.service.diagnose_at.is_some_and(|t| t <= at) {
        diagnose(x);
    }
    if let Some((t, pos)) = x.ui.service.hover_at {
        if t <= at {
            x.ui.service.hover_at = None;
            hover(x, pos);
        }
    }
}

// ---- the changes since the last save (the gutter's marks) ----

/// A line's mark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Change {
    #[default]
    None,
    Added,
    Modified,
    /// Lines were removed after it.
    RemovedAfter,
}

/// Each line's change since the text was last saved (or loaded).
#[derive(Clone, Debug, Default)]
pub struct LineChanges {
    /// The saved text (the document's when it wasn't modified).
    pub saved: Option<Arc<str>>,
    /// (first line, marks) of the changed region; nothing elsewhere.
    pub first: usize,
    pub marks: Vec<Change>,
    /// The document version the marks are for.
    pub version: Option<u64>,
}

impl LineChanges {
    pub fn at(&self, line: usize) -> Change {
        line.checked_sub(self.first).and_then(|i| self.marks.get(i).copied()).unwrap_or_default()
    }

    /// Brings the marks up to date (at idle): `text` is the document's now,
    /// `modified` whether it differs from the saved one.
    pub fn update(&mut self, text: Arc<str>, version: u64, modified: bool) {
        if !modified {
            self.saved = Some(text);
            self.marks.clear();
            self.version = Some(version);
            return;
        }
        if self.version == Some(version) {
            return;
        }
        self.version = Some(version);
        let Some(saved) = self.saved.clone() else {
            self.saved = Some(text);
            return;
        };
        let old: Vec<&str> = saved.split('\n').collect();
        let new: Vec<&str> = text.split('\n').collect();
        let pre = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let max_suf = old.len().min(new.len()) - pre;
        let suf = old.iter().rev().zip(new.iter().rev()).take(max_suf).take_while(|(a, b)| a == b).count();
        let (o, n) = (&old[pre..old.len() - suf], &new[pre..new.len() - suf]);
        // (the marks start a line early: a removal marks the line before)
        self.first = pre.saturating_sub(1);
        let mut marks = vec![Change::None; n.len() + 2];
        let script = diff_lines(o, n);
        // (runs of removed / added lines: paired as modified)
        let (mut i, mut j) = (0usize, 0usize);
        let mut k = 0;
        while k < script.len() {
            match script[k] {
                Op::Same => {
                    i += 1;
                    j += 1;
                    k += 1;
                }
                _ => {
                    let (mut del, mut ins) = (0, 0);
                    while k < script.len() && script[k] != Op::Same {
                        match script[k] {
                            Op::Del => del += 1,
                            Op::Ins => ins += 1,
                            Op::Same => {}
                        }
                        k += 1;
                    }
                    for t in 0..ins {
                        let line = pre + j + t;
                        let c = if t < del { Change::Modified } else { Change::Added };
                        if let Some(m) = marks.get_mut(line - self.first) {
                            *m = c;
                        }
                    }
                    if ins == 0 {
                        // (removed after the line before)
                        let line = (pre + j).saturating_sub(1);
                        if let Some(m) = marks.get_mut(line.saturating_sub(self.first)) {
                            if *m == Change::None {
                                *m = Change::RemovedAfter;
                            }
                        }
                    }
                    i += del;
                    j += ins;
                }
            }
        }
        let _ = i;
        self.marks = marks;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Same,
    Del,
    Ins,
}

/// A line diff of `a` → `b` (Myers' O(ND), at most 4,000 differences;
/// beyond that, everything between as replaced).
fn diff_lines(a: &[&str], b: &[&str]) -> Vec<Op> {
    let (n, m) = (a.len() as isize, b.len() as isize);
    let max = (n + m).min(4000);
    let off = max as usize + 1;
    let mut v = vec![0isize; 2 * off + 1];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut found = false;
    'outer: for d in 0..=max {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let ki = (k + off as isize) as usize;
            let mut x = if k == -d || (k != d && v[ki - 1] < v[ki + 1]) { v[ki + 1] } else { v[ki - 1] + 1 };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[ki] = x;
            if x >= n && y >= m {
                found = true;
                break 'outer;
            }
            k += 2;
        }
    }
    if !found {
        let mut out = vec![Op::Del; a.len()];
        out.extend(std::iter::repeat_n(Op::Ins, b.len()));
        return out;
    }
    // (back through the trace)
    let mut out = Vec::new();
    let (mut x, mut y) = (n, m);
    for d in (0..trace.len()).rev() {
        let v = &trace[d];
        let d = d as isize;
        let k = x - y;
        let ki = |k: isize| (k + off as isize) as usize;
        let prev_k = if k == -d || (k != d && v[ki(k - 1)] < v[ki(k + 1)]) { k + 1 } else { k - 1 };
        let prev_x = if d == 0 { 0 } else { v[ki(prev_k)] };
        let prev_y = prev_x - prev_k;
        while x > prev_x.max(0) && y > prev_y.max(0) && x > 0 && y > 0 && a[(x - 1) as usize] == b[(y - 1) as usize] {
            out.push(Op::Same);
            x -= 1;
            y -= 1;
        }
        if d > 0 {
            if x == prev_x {
                out.push(Op::Ins);
            } else {
                out.push(Op::Del);
            }
            x = prev_x;
            y = prev_y;
        }
    }
    while x > 0 && y > 0 {
        out.push(Op::Same);
        x -= 1;
        y -= 1;
    }
    out.reverse();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_changes() {
        let mut c = LineChanges::default();
        c.update(Arc::from("a\nb\nc\nd\n"), 0, false);
        c.update(Arc::from("a\nB\nc\nnew\nd\n"), 1, true);
        assert_eq!((c.at(0), c.at(1), c.at(2), c.at(3), c.at(4)), (Change::None, Change::Modified, Change::None, Change::Added, Change::None));
        c.update(Arc::from("a\nb\nd\n"), 2, true);
        assert_eq!(c.at(1), Change::RemovedAfter);
        assert_eq!(diff_lines(&["x", "y"], &["x", "z", "y"]), vec![Op::Same, Op::Ins, Op::Same]);
    }
}
