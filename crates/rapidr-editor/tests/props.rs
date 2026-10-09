//! Property tests (docs/ide-plan.md §4, I2's acceptance):
//! - random transactions, then undo / redo, restore the exact text and
//!   selections;
//! - an edit at several carets equals the same edit at each caret alone;
//! - the incremental tokenizer equals a full re-tokenize.

use std::collections::HashMap;
use std::sync::Arc;

use proptest::prelude::*;
use rapidr_editor::{Buffer, Change, Direction, Document, Language, Languages, SearchQuery, Selection, Selections};

/// Pieces random texts are made of: line breaks of every kind, multi-byte
/// characters, brackets, quotes and every language's multi-line openers and
/// closers.
const FRAGMENTS: &[&str] = &[
    "a", "b", "x1", " ", "  ", "\t", "\n", "\r\n", "\r", "é", "😀", "(", ")", "{", "}", "[", "]", "\"", "'", "`", "/*", "*/", "//", "--", "#", "<!--", "-->", "<script>", "</script>",
    "<style>", "</style>", "```", "'''", "\"\"\"", "${", "SUB ", "END SUB", "IF x THEN", "END IF", "RUSTSTART", "RUSTEND", "$ESCAPECHARS ON", "$ESCAPECHARS OFF", "\\", "\\n", "r#\"", "\"#", "123",
    "PRINT", "let ", "fn ", ",", ";", ":", "=", "<", ">", "&amp;", "REM ",
];

fn text_strategy(max: usize) -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(FRAGMENTS), 0..max).prop_map(|v| v.concat())
}

fn lang(id: &str) -> Arc<Language> {
    Languages::builtin().get(id).unwrap()
}

const LANGS: &[&str] = &["rapidr-basic", "rust", "javascript", "html", "css", "markdown", "toml", "json", "sql", "csv", "plaintext"];

#[derive(Clone, Debug)]
enum Op {
    Type(String),
    Newline,
    Backspace,
    Delete,
    DeleteWord,
    Paste(String),
    Tab,
    Outdent,
    Comment,
    Carets(Vec<u16>),
    Select(u16, u16),
    Edits(Vec<u16>, Vec<String>),
    ReplaceAll(String, String),
    Undo,
    Redo,
    Goto(u16),
    MoveWord,
    LineDown,
    Wait(u16),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    let piece = prop::sample::select(FRAGMENTS).prop_map(str::to_string);
    prop_oneof![
        6 => piece.clone().prop_map(Op::Type),
        1 => Just(Op::Newline),
        2 => Just(Op::Backspace),
        1 => Just(Op::Delete),
        1 => Just(Op::DeleteWord),
        1 => text_strategy(4).prop_map(Op::Paste),
        1 => Just(Op::Tab),
        1 => Just(Op::Outdent),
        1 => Just(Op::Comment),
        2 => prop::collection::vec(any::<u16>(), 1..4).prop_map(Op::Carets),
        1 => (any::<u16>(), any::<u16>()).prop_map(|(a, b)| Op::Select(a, b)),
        2 => (prop::collection::vec(any::<u16>(), 0..6), prop::collection::vec(text_strategy(3), 3)).prop_map(|(p, t)| Op::Edits(p, t)),
        1 => (piece.clone(), text_strategy(2)).prop_map(|(a, b)| Op::ReplaceAll(a, b)),
        3 => Just(Op::Undo),
        2 => Just(Op::Redo),
        1 => any::<u16>().prop_map(Op::Goto),
        1 => Just(Op::MoveWord),
        1 => Just(Op::LineDown),
        2 => (0u16..1000).prop_map(Op::Wait),
    ]
}

fn pos(doc: &Document, raw: u16) -> usize {
    doc.buffer().clamp(raw as usize % (doc.len_bytes() + 1))
}

/// Runs `op`; whether it was an edit attempt (for bookkeeping).
fn run(doc: &mut Document, op: &Op, now: &mut u64) {
    *now += 1;
    let t = *now;
    let _ = match op {
        Op::Type(s) => doc.type_text(s, t),
        Op::Newline => doc.newline(t),
        Op::Backspace => doc.backspace(t),
        Op::Delete => doc.delete_forward(t),
        Op::DeleteWord => doc.delete_word(Direction::Backward, t),
        Op::Paste(s) => doc.paste(s, t),
        Op::Tab => doc.tab(t),
        Op::Outdent => doc.outdent_lines(t),
        Op::Comment => doc.toggle_line_comment(t),
        Op::Carets(ps) => {
            let sels = ps.iter().map(|&p| Selection::caret(pos(doc, p))).collect();
            doc.set_selections(Selections::new(sels, 0));
            Ok(())
        }
        Op::Select(a, b) => {
            let s = Selection::new(pos(doc, *a), pos(doc, *b));
            doc.set_selections(Selections::single(s));
            Ok(())
        }
        Op::Edits(ps, texts) => {
            let mut ps: Vec<usize> = ps.iter().map(|&p| pos(doc, p)).collect();
            ps.sort();
            let edits = ps.chunks(2).enumerate().map(|(i, c)| Change::new(c[0]..*c.get(1).unwrap_or(&c[0]), texts[i % texts.len()].clone())).collect();
            doc.apply_edits(edits, t)
        }
        Op::ReplaceAll(a, b) => {
            let _ = doc.replace_all(&SearchQuery::literal(a.clone()), b, false, t);
            Ok(())
        }
        Op::Undo => {
            doc.undo();
            Ok(())
        }
        Op::Redo => {
            doc.redo();
            Ok(())
        }
        Op::Goto(r) => {
            let n = doc.history().len();
            doc.goto_revision(*r as usize % n);
            Ok(())
        }
        Op::MoveWord => {
            doc.move_word(Direction::Forward, false);
            Ok(())
        }
        Op::LineDown => {
            doc.move_line(Direction::Forward, true);
            Ok(())
        }
        Op::Wait(ms) => {
            *now += *ms as u64;
            Ok(())
        }
    };
}

/// The colouring of every line, by a fresh document (a full tokenize).
fn full_colours(text: &str, lang: Arc<Language>) -> Vec<Vec<rapidr_editor::Token>> {
    let mut fresh = Document::new(text, lang);
    (0..fresh.line_count()).map(|l| fresh.tokens(l)).collect()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// Random edits and moves; then undoing everything gives back each
    /// revision's text, and the selections from before its first step;
    /// redoing everything gives back the last text; jumping anywhere in the
    /// tree gives that revision's text.
    #[test]
    fn undo_redo_restore_text_and_selections(start in text_strategy(30), ops in prop::collection::vec(op_strategy(), 1..60), lang_i in 0..LANGS.len()) {
        let mut doc = Document::new(&start, lang(LANGS[lang_i]));
        let mut now = 0u64;
        // each revision's text, and the selections before it was made
        let mut text_at: HashMap<usize, String> = HashMap::from([(0, start.clone())]);
        let mut sels_before: HashMap<usize, Selections> = HashMap::new();
        for op in &ops {
            let before_sels = doc.selections().clone();
            let before_len = doc.history().len();
            run(&mut doc, op, &mut now);
            let cur = doc.history().current();
            if doc.history().len() > before_len {
                sels_before.insert(cur, before_sels);
            }
            // (a revision's text is the same whenever it is current)
            let text = doc.text().to_string();
            if let Some(t) = text_at.get(&cur) {
                if !matches!(op, Op::Type(_) | Op::Newline | Op::Backspace | Op::Delete | Op::Tab) {
                    prop_assert_eq!(t, &text, "revision {} came back different after {:?}", cur, op);
                }
            }
            text_at.insert(cur, text);
            prop_assert_eq!(doc.buffer().len_bytes(), doc.text().len());
        }
        let last_text = doc.text().to_string();
        let last_rev = doc.history().current();
        let at_tip = !doc.can_redo();
        let mut undone = Vec::new();
        while doc.can_undo() {
            let rev = doc.history().current();
            prop_assert!(doc.undo());
            undone.push(rev);
            let now_rev = doc.history().current();
            prop_assert_eq!(&*doc.text(), text_at[&now_rev].as_str(), "after undoing {}", rev);
            prop_assert_eq!(doc.selections(), &sels_before[&rev], "selections after undoing {}", rev);
        }
        prop_assert_eq!(&*doc.text(), start.as_str());
        prop_assert!(!doc.is_modified());
        while doc.redo() {}
        if at_tip {
            prop_assert_eq!(doc.history().current(), last_rev);
            prop_assert_eq!(&*doc.text(), last_text.as_str());
        } else {
            // (the run ended with undos: redo goes to the branch's end)
            prop_assert_eq!(&*doc.text(), text_at[&doc.history().current()].as_str());
        }
        // any revision of the tree
        for target in 0..doc.history().len() {
            if let Some(t) = text_at.get(&target) {
                doc.goto_revision(target);
                prop_assert_eq!(&*doc.text(), t.as_str(), "goto {}", target);
            }
        }
    }

    /// Typing, Backspace and Delete at several carets equal the same key at
    /// each caret alone (from the last caret to the first).
    #[test]
    fn multi_cursor_equals_sequential(start in text_strategy(30), raw in prop::collection::vec(any::<u16>(), 1..6), typed in prop::sample::select(&["x", "é", "😀", "ab", " ", "\t"][..]), key in 0..3usize) {
        let mut a = Document::new(&start, lang("plaintext"));
        a.auto_close = false;
        a.auto_indent = false;
        let mut carets: Vec<usize> = raw.iter().map(|&r| pos(&a, r)).collect();
        carets.sort();
        carets.dedup();
        // (carets a step apart would merge after Backspace / Delete)
        if key > 0 {
            let mut kept: Vec<usize> = Vec::new();
            for c in carets {
                let far = kept.last().is_none_or(|&k| {
                    let b = a.buffer();
                    b.next_boundary(b.next_boundary(k)) < c && b.prev_boundary(c) > k
                });
                if far {
                    kept.push(c);
                }
            }
            carets = kept;
        }
        let mut b = a.clone();
        a.set_selections(Selections::new(carets.iter().map(|&c| Selection::caret(c)).collect(), 0));
        let one = |d: &mut Document, t: u64| match key {
            0 => d.type_text(typed, t),
            1 => d.backspace(t),
            _ => d.delete_forward(t),
        };
        one(&mut a, 0).unwrap();
        for (i, &c) in carets.iter().enumerate().rev() {
            b.set_selections(Selections::caret(c));
            one(&mut b, 10_000 * (i as u64 + 1)).unwrap();
        }
        prop_assert_eq!(a.text(), b.text());
        // and the carets landed where the single edits leave theirs
        if key == 0 {
            let expect: Vec<usize> = carets.iter().enumerate().map(|(i, &c)| c + (i + 1) * typed.len()).collect();
            let got: Vec<usize> = a.selections().iter().map(|s| s.head).collect();
            prop_assert_eq!(got, expect);
        }
        // one undo step takes all of it back
        if *a.text() != *start {
            prop_assert!(a.undo());
            prop_assert_eq!(&*a.text(), start.as_str());
        }
    }

    /// Typing over several selections equals typing over each alone.
    #[test]
    fn multi_selection_typing_equals_sequential(start in text_strategy(30), raw in prop::collection::vec(any::<u16>(), 2..8), typed in prop::sample::select(&["x", "é(", "", "a b"][..])) {
        let mut a = Document::new(&start, lang("plaintext"));
        a.auto_close = false;
        let mut ps: Vec<usize> = raw.iter().map(|&r| pos(&a, r)).collect();
        ps.sort();
        ps.dedup();
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        for c in ps.chunks(2) {
            if let [s, e] = *c {
                if ranges.last().is_none_or(|&(_, pe)| pe < s) {
                    ranges.push((s, e));
                }
            }
        }
        prop_assume!(!ranges.is_empty());
        let mut b = a.clone();
        a.set_selections(Selections::new(ranges.iter().map(|&(s, e)| Selection::new(s, e)).collect(), 0));
        let _ = if typed.is_empty() { a.backspace(0) } else { a.type_text(typed, 0) };
        for (i, &(s, e)) in ranges.iter().enumerate().rev() {
            b.set_selections(Selections::single(Selection::new(s, e)));
            let t = 10_000 * (i as u64 + 1);
            let _ = if typed.is_empty() { b.backspace(t) } else { b.type_text(typed, t) };
        }
        prop_assert_eq!(a.text(), b.text());
    }

    /// After random edits, with lines coloured lazily in between (as a view
    /// scrolling would), every line's tokens equal a fresh full tokenize.
    #[test]
    fn incremental_tokenizer_equals_full(start in text_strategy(60), edits in prop::collection::vec((any::<u16>(), any::<u16>(), text_strategy(4), any::<u16>(), 0..4usize), 1..25), lang_i in 0..LANGS.len()) {
        let l = lang(LANGS[lang_i]);
        let mut doc = Document::new(&start, l.clone());
        doc.auto_close = false;
        doc.auto_indent = false;
        for (i, (a, b, text, peek, how)) in edits.iter().enumerate() {
            let (a, b) = (pos(&doc, *a), pos(&doc, *b));
            let (s, e) = (a.min(b), a.max(b));
            match how {
                0 => { let _ = doc.apply_edits(vec![Change::new(s..e, text.clone())], i as u64); }
                1 => { doc.set_selections(Selections::new(vec![Selection::caret(s), Selection::caret(e)], 0)); let _ = doc.type_text(text, i as u64); }
                2 => { doc.set_selections(Selections::single(Selection::new(s, e))); let _ = doc.backspace(i as u64); }
                _ => { doc.undo(); }
            }
            // colour some line now (or a slice in idle time)
            let n = doc.line_count();
            if peek % 3 == 0 {
                let _ = doc.tokens(*peek as usize % n);
            } else if peek % 3 == 1 {
                doc.highlight_idle(*peek as usize % 7);
            }
        }
        let text = doc.text().to_string();
        let full = full_colours(&text, l);
        let incremental: Vec<_> = (0..doc.line_count()).map(|line| doc.tokens(line)).collect();
        prop_assert_eq!(incremental.len(), full.len());
        for (line, (inc, fresh)) in incremental.iter().zip(&full).enumerate() {
            prop_assert_eq!(inc, fresh, "line {} of {:?}", line, text);
        }
        // the stored end states too (as stacks: numbering differs)
        let mut fresh = Document::new(&text, doc.language().clone());
        let _ = fresh.fold_ranges();
        let _ = doc.fold_ranges();
        for line in 0..doc.line_count() {
            let (ha, hb) = (doc.highlighter(), fresh.highlighter());
            prop_assert_eq!(ha.stack(ha.start_state(line)), hb.stack(hb.start_state(line)), "start state of line {}", line);
        }
    }
}
