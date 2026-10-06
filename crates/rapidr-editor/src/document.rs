//! The document: the text, its line ending, the selections, the undo tree
//! and the incremental highlighter, kept in step by every edit.
//!
//! Every change goes through [`Document::apply`] (a change set and the
//! selections after it), so undo, colouring and selections never disagree.
//! The editing commands (typing with auto-closing pairs and auto-indent,
//! Backspace, Tab, comments, paste …) build change sets for every
//! selection at once: multi-cursor editing is the same code path as one
//! caret. Times are the caller's clock in milliseconds (`now_ms`), for
//! undo grouping.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use crate::buffer::{Buffer, LineEnding, Position, RopeBuffer};
use crate::highlight::Highlighter;
use crate::history::{EditKind, History, Path, Step};
use crate::lang::{Language, Languages, Token};
use crate::selection::{Selection, Selections};
use crate::transaction::{Change, ChangeSet, EditError, Phase};

/// A character's class, for word moves and undo grouping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharClass {
    Word,
    Space,
    LineBreak,
    Punct,
}

/// Which way a move goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Backward,
    Forward,
}

/// Where a selection goes after an edit built by [`Document::edit_each`].
enum After {
    /// Anchor and head as offsets into the selection's change's new text
    /// (from the change's start).
    Rel(usize, usize),
    /// Positions in the text before the edit, before any change of this
    /// selection.
    Abs(Selection),
}

#[derive(Debug)]
pub struct Document {
    buffer: RopeBuffer,
    line_ending: LineEnding,
    selections: Selections,
    history: History,
    highlighter: Highlighter,
    /// Columns per tab stop.
    pub tab_size: u32,
    /// Tab inserts spaces.
    pub insert_spaces: bool,
    pub read_only: bool,
    /// Typing an opening bracket or quote inserts its closing one.
    pub auto_close: bool,
    /// New lines are indented as the language says.
    pub auto_indent: bool,
    version: u64,
    saved: Option<usize>,
    text_cache: OnceLock<Arc<str>>,
    last_typed: Option<CharClass>,
}

impl Clone for Document {
    fn clone(&self) -> Self {
        Document {
            buffer: self.buffer.clone(),
            line_ending: self.line_ending,
            selections: self.selections.clone(),
            history: self.history.clone(),
            highlighter: self.highlighter.clone(),
            tab_size: self.tab_size,
            insert_spaces: self.insert_spaces,
            read_only: self.read_only,
            auto_close: self.auto_close,
            auto_indent: self.auto_indent,
            version: self.version,
            saved: self.saved,
            text_cache: OnceLock::new(),
            last_typed: self.last_typed,
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Document::new("", Languages::builtin().plain_text())
    }
}

impl Document {
    /// A document holding `text` (its line ending detected), coloured as
    /// `lang`.
    pub fn new(text: &str, lang: Arc<Language>) -> Self {
        Document {
            buffer: RopeBuffer::new(text),
            line_ending: LineEnding::detect(text),
            selections: Selections::default(),
            history: History::new(),
            highlighter: Highlighter::new(lang),
            tab_size: 4,
            insert_spaces: true,
            read_only: false,
            auto_close: true,
            auto_indent: true,
            version: 0,
            saved: Some(0),
            text_cache: OnceLock::new(),
            last_typed: None,
        }
    }

    /// A document for a file: its language from the path.
    pub fn for_path(text: &str, path: &str) -> Self {
        Document::new(text, Languages::builtin().for_path(path))
    }

    // ---- reading ----

    pub fn buffer(&self) -> &RopeBuffer {
        &self.buffer
    }

    /// The whole text (cached until the next edit).
    pub fn text(&self) -> Arc<str> {
        self.text_cache.get_or_init(|| Arc::from(self.buffer.text().as_ref())).clone()
    }

    pub fn len_bytes(&self) -> usize {
        self.buffer.len_bytes()
    }

    pub fn line_count(&self) -> usize {
        self.buffer.len_lines()
    }

    /// Line `line`'s text, without its break.
    pub fn line(&self, line: usize) -> Cow<'_, str> {
        self.buffer.line_text(line)
    }

    pub fn slice(&self, range: Range<usize>) -> Cow<'_, str> {
        self.buffer.slice(range)
    }

    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// Sets the break new lines get (the text isn't converted: see
    /// [`Document::convert_line_endings`]).
    pub fn set_line_ending(&mut self, le: LineEnding) {
        self.line_ending = le;
    }

    pub fn selections(&self) -> &Selections {
        &self.selections
    }

    pub fn language(&self) -> &Arc<Language> {
        self.highlighter.language()
    }

    pub fn highlighter(&self) -> &Highlighter {
        &self.highlighter
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    /// Bumped by every change of the text (undo and redo included).
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Whether the text differs from the last save (or load).
    pub fn is_modified(&self) -> bool {
        self.saved != Some(self.history.current())
    }

    /// The text was saved as it is now.
    pub fn mark_saved(&mut self) {
        self.saved = Some(self.history.current());
    }

    /// Heap bytes the document takes, approximately (rope, colouring states,
    /// the cached text; not the undo history).
    pub fn approx_heap_bytes(&self) -> usize {
        self.buffer.approx_heap_bytes() + self.highlighter.approx_heap_bytes() + self.text_cache.get().map_or(0, |t| t.len())
    }

    // ---- colouring ----

    /// Colours as `lang` from now on.
    pub fn set_language(&mut self, lang: Arc<Language>) {
        self.highlighter.reset(Some(lang));
    }

    /// Line `line`'s tokens (tokenizing the lines above it first, as far as
    /// they aren't yet).
    pub fn tokens(&mut self, line: usize) -> Vec<Token> {
        self.highlighter.tokens(&self.buffer, line)
    }

    /// Tokenizes up to `budget` more lines in idle time; true when all are.
    pub fn highlight_idle(&mut self, budget: usize) -> bool {
        self.highlighter.advance(&self.buffer, budget)
    }

    /// Lines whose colours may have changed beyond the edited ones.
    pub fn take_restyled(&mut self) -> Option<Range<usize>> {
        self.highlighter.take_restyled()
    }

    pub(crate) fn highlighter_mut(&mut self) -> (&RopeBuffer, &mut Highlighter) {
        (&self.buffer, &mut self.highlighter)
    }

    // ---- positions ----

    pub fn position(&self, byte: usize) -> Position {
        self.buffer.position(byte)
    }

    pub fn offset(&self, pos: Position) -> usize {
        self.buffer.offset(pos)
    }

    /// The display column of `byte` (tabs to the next stop; one column a
    /// character — the view measures real widths).
    pub fn display_column(&self, byte: usize) -> u32 {
        let line = self.buffer.line_of(byte);
        let start = self.buffer.line_start(line);
        display_width(&self.buffer.line_text(line)[..byte - start], self.tab_size)
    }

    /// The byte offset of display column `col` on `line` (clamped to it).
    pub fn at_display_column(&self, line: usize, col: u32) -> usize {
        let start = self.buffer.line_start(line);
        let text = self.buffer.line_text(line);
        let mut w = 0;
        for (i, c) in text.char_indices() {
            let next = if c == '\t' { (w / self.tab_size.max(1) + 1) * self.tab_size.max(1) } else { w + 1 };
            if next > col {
                // (inside a tab: the nearer side)
                return start + if col - w <= (next - w) / 2 { i } else { i + c.len_utf8() };
            }
            w = next;
        }
        start + text.len()
    }

    /// The class of character `c` in this document's language.
    pub fn char_class(&self, c: char) -> CharClass {
        if c == '\n' || c == '\r' {
            CharClass::LineBreak
        } else if c.is_whitespace() {
            CharClass::Space
        } else if c.is_alphanumeric() || c == '_' || self.language().is_word_char(c) {
            CharClass::Word
        } else {
            CharClass::Punct
        }
    }

    /// The word at or touching `byte` (by the language's `word` pattern).
    pub fn word_at(&self, byte: usize) -> Option<Range<usize>> {
        let line = self.buffer.line_of(byte);
        let start = self.buffer.line_start(line);
        let text = self.buffer.line_text(line);
        let col = byte - start;
        self.language().word.find_iter(&text).find(|m| m.start() <= col && col <= m.end()).map(|m| start + m.start()..start + m.end())
    }

    /// The line's leading whitespace.
    pub fn indent_of(&self, line: usize) -> String {
        leading_ws(&self.buffer.line_text(line)).to_string()
    }

    /// One level of indentation as text.
    pub fn indent_unit(&self) -> String {
        if self.insert_spaces {
            " ".repeat(self.tab_size.max(1) as usize)
        } else {
            "\t".into()
        }
    }

    // ---- the one way to change the text ----

    /// Applies `changes` (in today's coordinates), leaving `after` selected;
    /// records one undo step of `kind` (typing joins the current one while
    /// it goes on).
    pub fn apply(&mut self, changes: ChangeSet, after: Selections, kind: EditKind, now_ms: u64) -> Result<(), EditError> {
        self.apply_grouped(changes, after, kind, now_ms, kind == EditKind::Command)
    }

    fn apply_grouped(&mut self, changes: ChangeSet, after: Selections, kind: EditKind, now_ms: u64, new_word: bool) -> Result<(), EditError> {
        if self.read_only {
            return Err(EditError::ReadOnly);
        }
        changes.validate(&self.buffer)?;
        if changes.is_empty() {
            self.set_selections(after);
            return Ok(());
        }
        let before = self.selections.clone();
        let removed = self.apply_raw(&changes);
        let inverse = changes.invert(&removed);
        let after = after.clamped(|p| self.buffer.clamp(p));
        self.history.commit(Step { changes, inverse }, before, after.clone(), kind, now_ms, new_word);
        self.selections = after;
        if kind != EditKind::Typing {
            self.last_typed = None;
        }
        Ok(())
    }

    /// Changes the text and tells the highlighter which lines changed.
    fn apply_raw(&mut self, changes: &ChangeSet) -> Vec<String> {
        let hl = &mut self.highlighter;
        let mut lines = (0, 0);
        let removed = changes.apply_with(&mut self.buffer, |buf, range, inserted, phase| match phase {
            Phase::Before => lines = (buf.line_of(range.start), buf.line_of(range.end)),
            Phase::After => {
                let (first, old_last) = lines;
                let new_last = buf.line_of(range.start + inserted).max(first);
                hl.edited(first, old_last - first + 1, new_last - first + 1);
            }
        });
        self.version += 1;
        self.text_cache = OnceLock::new();
        removed
    }

    /// Selects `sels` (clamped to the text). Ends the current undo group.
    pub fn set_selections(&mut self, sels: Selections) {
        self.selections = sels.clamped(|p| self.buffer.clamp(p));
        self.history.seal();
        self.last_typed = None;
    }

    /// Replaces `range` with `text` (one undo step), the caret after it.
    pub fn replace_range(&mut self, range: Range<usize>, text: &str, now_ms: u64) -> Result<(), EditError> {
        let set = ChangeSet::new(vec![Change::new(range.clone(), text)], self.len_bytes())?;
        let after = Selections::caret(range.start + text.len());
        self.apply(set, after, EditKind::Command, now_ms)
    }

    /// Replaces the whole text (one undo step); the caret goes to the start.
    pub fn set_text(&mut self, text: &str, now_ms: u64) -> Result<(), EditError> {
        let set = ChangeSet::new(vec![Change::new(0..self.len_bytes(), text)], self.len_bytes())?;
        self.apply(set, Selections::caret(0), EditKind::Command, now_ms)
    }

    /// `ApplyEdits`: several range edits as one undo step (refused whole if
    /// two overlap); the selections follow the text.
    pub fn apply_edits(&mut self, edits: Vec<Change>, now_ms: u64) -> Result<(), EditError> {
        let set = ChangeSet::new(edits, self.len_bytes())?;
        set.validate(&self.buffer)?;
        let after = self.selections.map(&set);
        self.apply(set, after, EditKind::Command, now_ms)
    }

    /// Rewrites every line break as `le` (one undo step) and types new ones
    /// with it.
    pub fn convert_line_endings(&mut self, le: LineEnding, now_ms: u64) -> Result<(), EditError> {
        self.line_ending = le;
        let text = self.text();
        let b = text.as_bytes();
        let mut changes = Vec::new();
        let mut i = 0;
        while i < b.len() {
            let (len, is_break) = match b[i] {
                b'\r' if b.get(i + 1) == Some(&b'\n') => (2, true),
                b'\r' | b'\n' => (1, true),
                _ => (1, false),
            };
            if is_break && &text[i..i + len] != le.as_str() {
                changes.push(Change::new(i..i + len, le.as_str()));
            }
            i += len;
        }
        let set = ChangeSet::new(changes, self.len_bytes())?;
        let after = self.selections.map(&set);
        self.apply(set, after, EditKind::Command, now_ms)
    }

    /// Builds one change set from every selection: `f` gives a selection's
    /// change (if any) and where the selection goes. A change that would
    /// overlap the one before is dropped (that selection stays).
    fn edit_each(&mut self, kind: EditKind, now_ms: u64, new_word: bool, mut f: impl FnMut(&Document, Selection) -> (Option<Change>, After)) -> Result<(), EditError> {
        let mut changes: Vec<Change> = Vec::new();
        let mut afters = Vec::new();
        let mut delta: isize = 0;
        let mut last_end = 0usize;
        let sels: Vec<Selection> = self.selections.ranges().to_vec();
        let shift = |p: usize, d: isize| (p as isize + d) as usize;
        for sel in sels {
            let (change, after) = f(self, sel);
            let change = change.filter(|c| changes.is_empty() || c.range.start >= last_end);
            match (change, after) {
                (Some(c), After::Rel(a, h)) => {
                    let base = shift(c.range.start, delta);
                    afters.push(Selection::new(base + a, base + h));
                    delta += c.insert.len() as isize - c.range.len() as isize;
                    last_end = c.range.end;
                    changes.push(c);
                }
                (c, After::Abs(s)) => {
                    afters.push(Selection { anchor: shift(s.anchor, delta), head: shift(s.head, delta), goal: s.goal });
                    if let Some(c) = c {
                        delta += c.insert.len() as isize - c.range.len() as isize;
                        last_end = c.range.end;
                        changes.push(c);
                    }
                }
                (None, After::Rel(..)) => afters.push(Selection { anchor: shift(sel.anchor, delta), head: shift(sel.head, delta), goal: None }),
            }
        }
        let primary = self.selections.primary_index();
        let set = ChangeSet::new(changes, self.len_bytes())?;
        self.apply_grouped(set, Selections::new(afters, primary), kind, now_ms, new_word)
    }

    // ---- typing ----

    /// Types `text` at every selection (replacing selected text) — a key, or
    /// an IME commit. A single character gets the language's auto-closing
    /// pairs and outdenting (`END SUB` goes back a level); a line break goes
    /// to [`Document::newline`].
    pub fn type_text(&mut self, text: &str, now_ms: u64) -> Result<(), EditError> {
        if text.is_empty() {
            return Ok(());
        }
        if text == "\n" || text == "\r" || text == "\r\n" {
            return self.newline(now_ms);
        }
        let first = text.chars().next().unwrap_or(' ');
        let class = self.char_class(first);
        let new_word = class == CharClass::LineBreak || (class == CharClass::Word && self.last_typed != Some(CharClass::Word)) || text.chars().count() > 1;
        let single = if text.chars().count() == 1 { Some(first) } else { None };
        let lang = self.language().clone();
        let auto_close = self.auto_close;
        let auto_indent = self.auto_indent;
        let result = self.edit_each(EditKind::Typing, now_ms, new_word, |doc, sel| {
            if let Some(c) = single {
                if auto_close {
                    if let Some(r) = doc.auto_pair(&lang, sel, c) {
                        return r;
                    }
                }
                if auto_indent && sel.is_empty() {
                    if let Some(r) = doc.outdent_on_type(&lang, sel.head, c) {
                        return r;
                    }
                }
            }
            (Some(Change::new(sel.range(), text)), After::Rel(text.len(), text.len()))
        });
        let last = text.chars().last().map(|c| self.char_class(c));
        self.last_typed = last;
        result
    }

    /// Auto-closing pairs: wrap a selection, overtype a closing character,
    /// or insert a pair around the caret.
    fn auto_pair(&self, lang: &Language, sel: Selection, c: char) -> Option<(Option<Change>, After)> {
        let mut buf = [0u8; 4];
        let typed: &str = c.encode_utf8(&mut buf);
        let next = self.buffer.char_after(sel.head);
        // overtype the closing character that's already there
        if sel.is_empty() && next == Some(c) && lang.auto_close.iter().any(|(_, close)| close == typed) {
            let is_open_too = lang.auto_close.iter().any(|(o, cl)| o == typed && cl == typed);
            if !is_open_too || !self.in_literal(sel.head) || self.literal_ends_at(sel.head) {
                let to = sel.head + c.len_utf8();
                return Some((None, After::Abs(Selection::caret(to))));
            }
        }
        let (open, close) = lang.auto_close.iter().find(|(o, _)| o.ends_with(c))?;
        // (a multi-character opening: its rest must be typed already)
        let before = &open[..open.len() - c.len_utf8()];
        if !before.is_empty() && !self.buffer.slice(sel.start().saturating_sub(before.len())..sel.start()).ends_with(before) {
            return None;
        }
        if !sel.is_empty() && before.is_empty() {
            // surround the selection
            let inner = self.buffer.slice(sel.range()).into_owned();
            let insert = format!("{typed}{inner}{close}");
            let (a, h) = (typed.len(), typed.len() + inner.len());
            return Some((Some(Change::new(sel.range(), insert)), if sel.is_backward() { After::Rel(h, a) } else { After::Rel(a, h) }));
        }
        if !sel.is_empty() {
            return None;
        }
        // only before whitespace, a closing character or the line's end;
        // not inside a string or comment; a quote not right after a word
        let ok_next = match next {
            None | Some('\n') | Some('\r') => true,
            Some(n) => lang.auto_close_before.contains(n),
        };
        if !ok_next || self.in_literal(sel.head) {
            return None;
        }
        if open == close {
            if let Some(p) = self.buffer.char_before(sel.head) {
                if self.char_class(p) == CharClass::Word {
                    return None;
                }
            }
        }
        let insert = format!("{typed}{close}");
        Some((Some(Change::insert(sel.head, insert)), After::Rel(typed.len(), typed.len())))
    }

    /// Whether `byte` is inside a string or a comment (at a line's end: in
    /// a comment, or in a string left open).
    fn in_literal(&self, byte: usize) -> bool {
        let line = self.buffer.line_of(byte);
        let text = self.buffer.line_text(line);
        let col = (byte - self.buffer.line_start(line)) as u32;
        self.peek_tokens(line).iter().any(|t| {
            if !t.kind.is_literal() || col <= t.start || col > t.end {
                return false;
            }
            if col < t.end {
                return true;
            }
            // right after the token: still inside if it runs to the line's
            // end and doesn't close there
            let s = &text[t.start as usize..t.end as usize];
            t.end as usize == text.len() && (t.kind.is_comment() || s.chars().count() < 2 || s.chars().next() != s.chars().last())
        })
    }

    /// Whether a string token ends right after `byte` (its closing quote).
    fn literal_ends_at(&self, byte: usize) -> bool {
        let line = self.buffer.line_of(byte);
        let col = (byte - self.buffer.line_start(line)) as u32;
        self.peek_tokens(line).iter().any(|t| t.kind.is_string() && t.end == col + 1)
    }

    /// A line's tokens without tokenizing anything new: from the stored
    /// start state when it is known, else from the root (a guess).
    fn peek_tokens(&self, line: usize) -> Vec<Token> {
        let hl = &self.highlighter;
        let state = if line == 0 || line <= hl.valid_lines() { hl.start_state(line.min(hl.valid_lines())) } else { crate::highlight::ROOT };
        let mut stack = hl.stack(state).to_vec();
        let mut out = Vec::new();
        self.language().tokenize_line(&self.buffer.line_text(line), &mut stack, &mut out);
        out
    }

    /// Typing `c` at `at` makes the line one that goes back a level (`NEXT`,
    /// `END SUB`, `}`): the change that outdents it while typing.
    fn outdent_on_type(&self, lang: &Language, at: usize, c: char) -> Option<(Option<Change>, After)> {
        let decrease = lang.indent_decrease.as_ref()?;
        let line = self.buffer.line_of(at);
        let start = self.buffer.line_start(line);
        let text = self.buffer.line_text(line);
        let col = at - start;
        let mut new_text = String::with_capacity(text.len() + 4);
        new_text.push_str(&text[..col]);
        new_text.push(c);
        new_text.push_str(&text[col..]);
        if decrease.is_match(&text) || !decrease.is_match(&new_text) {
            return None;
        }
        let want = self.wanted_indent(line, &new_text)?;
        let have = leading_ws(&text);
        if display_width(&want, self.tab_size) >= display_width(have, self.tab_size) || col < have.len() {
            return None;
        }
        let insert = format!("{want}{}{c}", &text[have.len()..col]);
        let caret = insert.len();
        Some((Some(Change::new(start..at, insert)), After::Rel(caret, caret)))
    }

    /// The indentation line `line` (whose text would be `text`) should have
    /// by the language's rules, from the nearest non-blank line above.
    fn wanted_indent(&self, line: usize, text: &str) -> Option<String> {
        let lang = self.language();
        let prev = (0..line).rev().find(|&l| !self.buffer.line_text(l).trim().is_empty())?;
        let prev_text = self.buffer.line_text(prev);
        let mut level = display_width(leading_ws(&prev_text), self.tab_size);
        let unit = self.tab_size.max(1);
        if lang.indent_increase.as_ref().is_some_and(|r| r.is_match(&prev_text)) {
            level += unit;
        }
        if lang.indent_decrease.as_ref().is_some_and(|r| r.is_match(text)) {
            level = level.saturating_sub(unit);
        }
        Some(self.indent_text(level))
    }

    fn indent_text(&self, columns: u32) -> String {
        if self.insert_spaces {
            " ".repeat(columns as usize)
        } else {
            let t = self.tab_size.max(1);
            format!("{}{}", "\t".repeat((columns / t) as usize), " ".repeat((columns % t) as usize))
        }
    }

    /// Enter: a line break at every selection, indented like the line (one
    /// level more after a line that opens a block; between a bracket pair,
    /// the pair goes on its own lines).
    pub fn newline(&mut self, now_ms: u64) -> Result<(), EditError> {
        let le = self.line_ending.as_str();
        let lang = self.language().clone();
        let auto_indent = self.auto_indent;
        let unit = self.indent_unit();
        let result = self.edit_each(EditKind::Typing, now_ms, true, |doc, sel| {
            let line = doc.buffer.line_of(sel.start());
            let start = doc.buffer.line_start(line);
            let text = doc.buffer.line_text(line);
            let before = &text[..(sel.start() - start).min(text.len())];
            if !auto_indent {
                return (Some(Change::new(sel.range(), le)), After::Rel(le.len(), le.len()));
            }
            let mut indent = leading_ws(before).to_string();
            if lang.indent_increase.as_ref().is_some_and(|r| r.is_match(before)) {
                indent.push_str(&unit);
            }
            let prev = doc.buffer.char_before(sel.start());
            let next = doc.buffer.char_after(sel.end());
            if let (Some(p), Some(n)) = (prev, next) {
                if lang.brackets.iter().any(|&(o, c)| o == p && c == n) {
                    let outer = leading_ws(before);
                    let mut inner = outer.to_string();
                    inner.push_str(&unit);
                    let insert = format!("{le}{inner}{le}{outer}");
                    let caret = le.len() + inner.len();
                    return (Some(Change::new(sel.range(), insert)), After::Rel(caret, caret));
                }
            }
            // (whitespace after the caret isn't carried to the new line)
            let rest_ws = doc.buffer.line_text(doc.buffer.line_of(sel.end()));
            let end_col = sel.end() - doc.buffer.line_start(doc.buffer.line_of(sel.end()));
            let skip = rest_ws[end_col.min(rest_ws.len())..].len() - rest_ws[end_col.min(rest_ws.len())..].trim_start_matches([' ', '\t']).len();
            let insert = format!("{le}{indent}");
            let caret = insert.len();
            (Some(Change::new(sel.start()..sel.end() + skip, insert)), After::Rel(caret, caret))
        });
        self.last_typed = Some(CharClass::LineBreak);
        result
    }

    // ---- deleting ----

    /// Backspace at every selection: the selected text, else the character
    /// before (a CR LF at once; an empty auto-closed pair at once; spaces back
    /// to the previous tab stop in the indentation).
    pub fn backspace(&mut self, now_ms: u64) -> Result<(), EditError> {
        let lang = self.language().clone();
        let tab = self.tab_size.max(1);
        self.edit_each(EditKind::DeleteBack, now_ms, false, |doc, sel| {
            if !sel.is_empty() {
                return (Some(Change::delete(sel.range())), After::Rel(0, 0));
            }
            let p = sel.head;
            if p == 0 {
                return (None, After::Abs(sel));
            }
            let (prev, next) = (doc.buffer.char_before(p), doc.buffer.char_after(p));
            if let (Some(a), Some(b)) = (prev, next) {
                let (mut x, mut y) = ([0u8; 4], [0u8; 4]);
                let (a_s, b_s) = (a.encode_utf8(&mut x) as &str, b.encode_utf8(&mut y) as &str);
                if lang.auto_close.iter().any(|(o, c)| o == a_s && c == b_s) {
                    return (Some(Change::delete(p - a.len_utf8()..p + b.len_utf8())), After::Rel(0, 0));
                }
            }
            let line = doc.buffer.line_of(p);
            let start = doc.buffer.line_start(line);
            let before = doc.buffer.slice(start..p);
            if prev == Some(' ') && before.bytes().all(|b| b == b' ') {
                let col = before.len() as u32;
                let to = ((col - 1) / tab) * tab;
                return (Some(Change::delete(start + to as usize..p)), After::Rel(0, 0));
            }
            (Some(Change::delete(doc.buffer.prev_boundary(p)..p)), After::Rel(0, 0))
        })
    }

    /// Delete at every selection: the selected text, else the character after.
    pub fn delete_forward(&mut self, now_ms: u64) -> Result<(), EditError> {
        self.edit_each(EditKind::DeleteForward, now_ms, false, |doc, sel| {
            if !sel.is_empty() {
                return (Some(Change::delete(sel.range())), After::Rel(0, 0));
            }
            let to = doc.buffer.next_boundary(sel.head);
            if to == sel.head {
                return (None, After::Abs(sel));
            }
            (Some(Change::delete(sel.head..to)), After::Rel(0, 0))
        })
    }

    /// Deletes to the previous / next word boundary (or the selection).
    pub fn delete_word(&mut self, dir: Direction, now_ms: u64) -> Result<(), EditError> {
        let kind = if dir == Direction::Backward { EditKind::DeleteBack } else { EditKind::DeleteForward };
        self.edit_each(kind, now_ms, true, |doc, sel| {
            if !sel.is_empty() {
                return (Some(Change::delete(sel.range())), After::Rel(0, 0));
            }
            let to = doc.word_boundary(sel.head, dir);
            let r = if to < sel.head { to..sel.head } else { sel.head..to };
            if r.is_empty() {
                return (None, After::Abs(sel));
            }
            (Some(Change::delete(r)), After::Rel(0, 0))
        })
    }

    /// Deletes the lines the selections touch.
    pub fn delete_lines(&mut self, now_ms: u64) -> Result<(), EditError> {
        let ranges = self.touched_lines();
        let mut changes = Vec::new();
        for r in &ranges {
            let start = self.buffer.line_start(r.start);
            let end = if r.end < self.line_count() { self.buffer.line_start(r.end) } else { self.len_bytes() };
            // the last line: take the break before it instead
            let start = if r.end >= self.line_count() && r.start > 0 { self.buffer.line_end(r.start - 1) } else { start };
            changes.push(Change::delete(start..end));
        }
        let set = ChangeSet::new(changes, self.len_bytes())?;
        let after = self.selections.transform(|s| Selection::caret(set.map_pos(s.head, crate::transaction::Assoc::Before)));
        self.apply(set, after, EditKind::Command, now_ms)
    }

    // ---- indentation, comments, paste ----

    /// Tab: indents the touched lines when a selection spans lines, else
    /// inserts a tab (or spaces to the next stop) at every caret.
    pub fn tab(&mut self, now_ms: u64) -> Result<(), EditError> {
        let multi_line = self.selections.iter().any(|s| self.buffer.line_of(s.start()) != self.buffer.line_of(s.end()));
        if multi_line {
            return self.indent_lines(now_ms);
        }
        let tab = self.tab_size.max(1);
        let spaces = self.insert_spaces;
        self.edit_each(EditKind::Typing, now_ms, true, |doc, sel| {
            let insert = if spaces {
                let col = doc.display_column(sel.start());
                " ".repeat((tab - col % tab) as usize)
            } else {
                "\t".into()
            };
            let n = insert.len();
            (Some(Change::new(sel.range(), insert)), After::Rel(n, n))
        })
    }

    /// The line ranges (`start..end`, exclusive) the selections touch,
    /// merged; a selection ending at a line's start doesn't touch that line.
    pub fn touched_lines(&self) -> Vec<Range<usize>> {
        let mut out: Vec<Range<usize>> = Vec::new();
        for s in self.selections.iter() {
            let first = self.buffer.line_of(s.start());
            let mut last = self.buffer.line_of(s.end());
            if last > first && self.buffer.line_start(last) == s.end() {
                last -= 1;
            }
            match out.last_mut() {
                Some(r) if first <= r.end => r.end = r.end.max(last + 1),
                _ => out.push(first..last + 1),
            }
        }
        out
    }

    fn line_edits(&mut self, now_ms: u64, mut f: impl FnMut(&Document, usize) -> Option<Change>) -> Result<(), EditError> {
        let mut changes = Vec::new();
        for r in self.touched_lines() {
            for line in r {
                if let Some(c) = f(self, line) {
                    changes.push(c);
                }
            }
        }
        let set = ChangeSet::new(changes, self.len_bytes())?;
        let after = self.selections.map(&set);
        self.apply(set, after, EditKind::Command, now_ms)
    }

    /// Indents the touched lines one level (blank lines stay blank).
    pub fn indent_lines(&mut self, now_ms: u64) -> Result<(), EditError> {
        let unit = self.indent_unit();
        self.line_edits(now_ms, |doc, line| {
            if doc.buffer.line_text(line).trim().is_empty() {
                return None;
            }
            Some(Change::insert(doc.buffer.line_start(line), unit.clone()))
        })
    }

    /// Outdents the touched lines one level.
    pub fn outdent_lines(&mut self, now_ms: u64) -> Result<(), EditError> {
        let tab = self.tab_size.max(1) as usize;
        self.line_edits(now_ms, |doc, line| {
            let text = doc.buffer.line_text(line);
            let n = if text.starts_with('\t') { 1 } else { text.bytes().take(tab).take_while(|&b| b == b' ').count() };
            if n == 0 {
                return None;
            }
            let start = doc.buffer.line_start(line);
            Some(Change::delete(start..start + n))
        })
    }

    /// Comments the touched lines out with the language's line comment, or
    /// back in when every non-blank one is commented.
    pub fn toggle_line_comment(&mut self, now_ms: u64) -> Result<(), EditError> {
        let Some(prefix) = self.language().line_comment.first().cloned() else {
            return Ok(());
        };
        let ci = self.language().case_insensitive;
        let lines: Vec<usize> = self.touched_lines().into_iter().flatten().filter(|&l| !self.buffer.line_text(l).trim().is_empty()).collect();
        if lines.is_empty() {
            return Ok(());
        }
        let trimmed_prefix = prefix.trim_end().to_string();
        let starts_with = |text: &str| {
            let t = text.trim_start();
            if ci {
                t.len() >= trimmed_prefix.len() && t.is_char_boundary(trimmed_prefix.len()) && t[..trimmed_prefix.len()].eq_ignore_ascii_case(&trimmed_prefix)
            } else {
                t.starts_with(&trimmed_prefix)
            }
        };
        let all_commented = lines.iter().all(|&l| starts_with(&self.buffer.line_text(l)));
        let min_indent = lines.iter().map(|&l| leading_ws(&self.buffer.line_text(l)).len()).min().unwrap_or(0);
        let mut changes = Vec::new();
        for &l in &lines {
            let text = self.buffer.line_text(l);
            let start = self.buffer.line_start(l);
            if all_commented {
                let at = start + leading_ws(&text).len();
                let mut n = trimmed_prefix.len();
                if text[at - start + n..].starts_with(' ') {
                    n += 1;
                }
                changes.push(Change::delete(at..at + n));
            } else {
                let insert = if prefix.ends_with(' ') { prefix.clone() } else { format!("{prefix} ") };
                changes.push(Change::insert(start + min_indent, insert));
            }
        }
        let set = ChangeSet::new(changes, self.len_bytes())?;
        let after = self.selections.map(&set);
        self.apply(set, after, EditKind::Command, now_ms)
    }

    /// Pastes `text` at every selection; with as many selections as `text`
    /// has lines, each gets its own line.
    pub fn paste(&mut self, text: &str, now_ms: u64) -> Result<(), EditError> {
        let lines: Vec<&str> = text.split_inclusive('\n').map(|l| l.trim_end_matches(['\r', '\n'])).collect();
        let spread = self.selections.len() > 1 && lines.len() == self.selections.len();
        let mut i = 0;
        self.edit_each(EditKind::Command, now_ms, true, |_, sel| {
            let insert = if spread { lines[i].to_string() } else { text.to_string() };
            i += 1;
            let n = insert.len();
            (Some(Change::new(sel.range(), insert)), After::Rel(n, n))
        })
    }

    /// The selected texts, joined by line breaks (what Copy puts on the
    /// clipboard); empty carets copy their whole line.
    pub fn copy_text(&self) -> String {
        let le = self.line_ending.as_str();
        let parts: Vec<String> = self
            .selections
            .iter()
            .map(|s| {
                if s.is_empty() {
                    let line = self.buffer.line_of(s.head);
                    format!("{}{le}", self.buffer.line_text(line))
                } else {
                    self.buffer.slice(s.range()).into_owned()
                }
            })
            .collect();
        if self.selections.iter().all(|s| s.is_empty()) {
            parts.concat()
        } else {
            parts.join(le)
        }
    }

    // ---- undo ----

    fn run_revision(&mut self, rev: usize, undo: bool) {
        let steps = self.history.take_steps(rev);
        if undo {
            for s in steps.iter().rev() {
                self.apply_raw(&s.inverse);
            }
        } else {
            for s in &steps {
                self.apply_raw(&s.changes);
            }
        }
        self.history.put_steps(rev, steps);
    }

    /// Undoes the current undo step; false when there is none.
    pub fn undo(&mut self) -> bool {
        let Some(rev) = self.history.step_back() else {
            return false;
        };
        self.run_revision(rev, true);
        self.selections = self.history.revision(rev).selections_before.clamped(|p| self.buffer.clamp(p));
        self.last_typed = None;
        true
    }

    /// Redoes the step undone last; false when there is none.
    pub fn redo(&mut self) -> bool {
        let Some(rev) = self.history.step_forward() else {
            return false;
        };
        self.run_revision(rev, false);
        self.selections = self.history.revision(rev).selections_after.clamped(|p| self.buffer.clamp(p));
        self.last_typed = None;
        true
    }

    /// Goes to any revision of the undo tree (another branch included).
    pub fn goto_revision(&mut self, target: usize) -> bool {
        if target >= self.history.len() || target == self.history.current() {
            return false;
        }
        let Path { undo, redo } = self.history.path_to(target);
        for &r in &undo {
            self.run_revision(r, true);
        }
        for &r in &redo {
            self.run_revision(r, false);
        }
        self.history.set_current(target);
        let sels = if target == 0 { self.history.revision(*undo.last().unwrap_or(&0)).selections_before.clone() } else { self.history.revision(target).selections_after.clone() };
        self.selections = sels.clamped(|p| self.buffer.clamp(p));
        self.last_typed = None;
        true
    }

    // ---- moving the carets ----

    /// The next word boundary from `at` in `dir` (spaces skipped first; a
    /// line break is a stop of its own).
    pub fn word_boundary(&self, at: usize, dir: Direction) -> usize {
        let b = &self.buffer;
        let mut p = at;
        match dir {
            Direction::Forward => {
                let Some(c) = b.char_after(p) else { return p };
                if self.char_class(c) == CharClass::LineBreak {
                    return b.next_boundary(p);
                }
                while let Some(c) = b.char_after(p) {
                    if self.char_class(c) != CharClass::Space {
                        break;
                    }
                    p = b.next_boundary(p);
                }
                let Some(c) = b.char_after(p) else { return p };
                let class = self.char_class(c);
                if class == CharClass::LineBreak {
                    return p;
                }
                while let Some(c) = b.char_after(p) {
                    if self.char_class(c) != class {
                        break;
                    }
                    p = b.next_boundary(p);
                }
                p
            }
            Direction::Backward => {
                let Some(c) = b.char_before(p) else { return p };
                if self.char_class(c) == CharClass::LineBreak {
                    return b.prev_boundary(p);
                }
                while let Some(c) = b.char_before(p) {
                    if self.char_class(c) != CharClass::Space {
                        break;
                    }
                    p = b.prev_boundary(p);
                }
                let Some(c) = b.char_before(p) else { return p };
                let class = self.char_class(c);
                if class == CharClass::LineBreak {
                    return p;
                }
                while let Some(c) = b.char_before(p) {
                    if self.char_class(c) != class {
                        break;
                    }
                    p = b.prev_boundary(p);
                }
                p
            }
        }
    }

    fn move_with(&mut self, extend: bool, mut f: impl FnMut(&Document, Selection) -> (usize, Option<u32>)) {
        let sels = self.selections.transform(|s| {
            let (head, goal) = f(self, s);
            Selection { anchor: if extend { s.anchor } else { head }, head, goal }
        });
        self.set_selections(sels);
    }

    /// Left / Right: one character (a collapsing selection goes to its edge).
    pub fn move_char(&mut self, dir: Direction, extend: bool) {
        self.move_with(extend, |doc, s| {
            if !extend && !s.is_empty() {
                return (if dir == Direction::Backward { s.start() } else { s.end() }, None);
            }
            let p = if dir == Direction::Backward { doc.buffer.prev_boundary(s.head) } else { doc.buffer.next_boundary(s.head) };
            (p, None)
        });
    }

    /// Ctrl / Alt + Left / Right.
    pub fn move_word(&mut self, dir: Direction, extend: bool) {
        self.move_with(extend, |doc, s| (doc.word_boundary(s.head, dir), None));
    }

    /// Up / Down: the same display column (kept across short lines) on the
    /// line above / below; past the first / last line, its start / end.
    pub fn move_line(&mut self, dir: Direction, extend: bool) {
        self.move_with(extend, |doc, s| {
            let goal = s.goal.unwrap_or_else(|| doc.display_column(s.head));
            let line = doc.buffer.line_of(s.head);
            match dir {
                Direction::Backward if line == 0 => (0, Some(goal)),
                Direction::Forward if line + 1 >= doc.line_count() => (doc.len_bytes(), Some(goal)),
                Direction::Backward => (doc.at_display_column(line - 1, goal), Some(goal)),
                Direction::Forward => (doc.at_display_column(line + 1, goal), Some(goal)),
            }
        });
    }

    /// Home: the line's first non-blank character, or its start when already
    /// there.
    pub fn move_line_start(&mut self, extend: bool) {
        self.move_with(extend, |doc, s| {
            let line = doc.buffer.line_of(s.head);
            let start = doc.buffer.line_start(line);
            let first = start + leading_ws(&doc.buffer.line_text(line)).len();
            (if s.head == first { start } else { first }, None)
        });
    }

    /// End.
    pub fn move_line_end(&mut self, extend: bool) {
        self.move_with(extend, |doc, s| (doc.buffer.line_end(doc.buffer.line_of(s.head)), None));
    }

    /// Ctrl+Home / Ctrl+End (the primary caret only).
    pub fn move_doc(&mut self, dir: Direction, extend: bool) {
        let p = self.selections.primary();
        let to = if dir == Direction::Backward { 0 } else { self.len_bytes() };
        self.set_selections(Selections::single(Selection::new(if extend { p.anchor } else { to }, to)));
    }

    pub fn select_all(&mut self) {
        self.set_selections(Selections::single(Selection::new(0, self.len_bytes())));
    }

    /// Selects the touched lines whole (with their breaks).
    pub fn select_lines(&mut self) {
        let sels = self.selections.transform(|s| {
            let first = self.buffer.line_of(s.start());
            let last = self.buffer.line_of(s.end());
            let end = if last + 1 < self.line_count() { self.buffer.line_start(last + 1) } else { self.len_bytes() };
            Selection::new(self.buffer.line_start(first), end)
        });
        self.set_selections(sels);
    }

    /// Adds a caret at `byte` (it becomes the primary).
    pub fn add_cursor(&mut self, byte: usize) {
        let sels = self.selections.add(Selection::caret(byte));
        self.set_selections(sels);
    }

    /// Adds a caret on the line above / below every caret, same column.
    pub fn add_cursor_vertical(&mut self, dir: Direction) {
        let mut ranges = self.selections.ranges().to_vec();
        let mut primary = self.selections.primary_index();
        for s in self.selections.iter() {
            let line = self.buffer.line_of(s.head);
            let goal = s.goal.unwrap_or_else(|| self.display_column(s.head));
            let target = match dir {
                Direction::Backward if line > 0 => line - 1,
                Direction::Forward if line + 1 < self.line_count() => line + 1,
                _ => continue,
            };
            let p = self.at_display_column(target, goal);
            ranges.push(Selection { anchor: p, head: p, goal: Some(goal) });
            primary = ranges.len() - 1;
        }
        self.set_selections(Selections::new(ranges, primary));
    }

    /// Only the primary selection stays.
    pub fn clear_cursors(&mut self) {
        let s = self.selections.only_primary();
        self.set_selections(s);
    }

    /// Ctrl+D: selects the word at an empty primary caret, else adds the next
    /// occurrence of the primary selection's text (wrapping) as a new primary
    /// selection. False when there is no other occurrence.
    pub fn select_next_occurrence(&mut self) -> bool {
        let p = self.selections.primary();
        if p.is_empty() {
            let Some(w) = self.word_at(p.head) else { return false };
            let mut ranges = self.selections.ranges().to_vec();
            ranges[self.selections.primary_index()] = Selection::new(w.start, w.end);
            let prim = self.selections.primary_index();
            self.set_selections(Selections::new(ranges, prim));
            return true;
        }
        let needle = self.buffer.slice(p.range()).into_owned();
        let whole = self.word_at(p.start()) == Some(p.range());
        let text = self.text();
        let from = self.selections.ranges().last().map_or(p.end(), |s| s.end());
        let found = self.occurrences(&text, &needle, from, whole).next().or_else(|| self.occurrences(&text, &needle, 0, whole).next());
        match found {
            Some(at) if !self.selections.iter().any(|s| s.start() == at) => {
                let sels = self.selections.add(Selection::new(at, at + needle.len()));
                self.set_selections(sels);
                true
            }
            _ => false,
        }
    }

    /// Selects every occurrence of the primary selection's text (or word).
    pub fn select_all_occurrences(&mut self) -> usize {
        let p = self.selections.primary();
        let range = if p.is_empty() {
            match self.word_at(p.head) {
                Some(w) => w,
                None => return 0,
            }
        } else {
            p.range()
        };
        let needle = self.buffer.slice(range.clone()).into_owned();
        if needle.is_empty() {
            return 0;
        }
        let whole = self.word_at(range.start) == Some(range.clone());
        let text = self.text();
        let mut ranges = Vec::new();
        let mut primary = 0;
        for at in self.occurrences(&text, &needle, 0, whole) {
            if at == range.start {
                primary = ranges.len();
            }
            ranges.push(Selection::new(at, at + needle.len()));
        }
        let n = ranges.len();
        self.set_selections(Selections::new(ranges, primary));
        n
    }

    /// Where `needle` occurs in `text` from `from` on (`whole`: only where
    /// no word character touches it).
    fn occurrences<'a>(&'a self, text: &'a str, needle: &'a str, from: usize, whole: bool) -> impl Iterator<Item = usize> + 'a {
        let mut at = from;
        std::iter::from_fn(move || loop {
            if needle.is_empty() || at > text.len() {
                return None;
            }
            let i = at + text[at..].find(needle)?;
            at = i + needle.len();
            let touches = |c: Option<char>| c.is_some_and(|c| self.char_class(c) == CharClass::Word);
            if !whole || !(touches(text[..i].chars().next_back()) || touches(text[i + needle.len()..].chars().next())) {
                return Some(i);
            }
        })
    }
}

/// The leading spaces and tabs of `text`.
pub fn leading_ws(text: &str) -> &str {
    &text[..text.len() - text.trim_start_matches([' ', '\t']).len()]
}

/// `text`'s width in columns, tabs to the next multiple of `tab`.
pub fn display_width(text: &str, tab: u32) -> u32 {
    let tab = tab.max(1);
    text.chars().fold(0, |w, c| if c == '\t' { (w / tab + 1) * tab } else { w + 1 })
}
