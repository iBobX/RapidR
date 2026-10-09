//! Incremental colouring: the tokenizer's end state for every line, kept
//! across edits.
//!
//! Only end states are stored (4 bytes a line); a line's tokens are made
//! again on demand from the state the line before left (the view caches
//! the visible ones). Lines are tokenized lazily, up to where they are
//! needed ([`Highlighter::ensure`]) or in idle slices ([`Highlighter::advance`]).
//! After an edit, lines are tokenized again from the first changed line
//! until — past every edited line — a line ends in the state it ended in
//! before: the rest of the file is then known to be unchanged.
//!
//! States are stacks interned to `u32` numbers ([`StateId`]), carried from
//! line to line by the view that draws the code (`rapidr-ui-kernel`'s
//! `components::codeeditor`).

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use crate::buffer::Buffer;
use crate::lang::{Language, Token};

/// An interned tokenizer state (a stack). 0 is the root.
pub type StateId = u32;

/// The root state: where a file's first line starts.
pub const ROOT: StateId = 0;

#[derive(Clone, Debug)]
pub struct Highlighter {
    lang: Arc<Language>,
    /// The end state of lines `0..ends.len()`; right for `0..valid`, and
    /// past `valid` right unless an edit since says otherwise.
    ends: Vec<StateId>,
    valid: usize,
    /// Lines below this were edited: re-tokenizing can't stop before it.
    converge_from: usize,
    stacks: Vec<Box<[u16]>>,
    index: HashMap<Box<[u16]>, StateId>,
    /// Lines tokenized again since [`Highlighter::take_restyled`].
    restyled: Option<Range<usize>>,
    scratch: Vec<Token>,
}

impl Highlighter {
    pub fn new(lang: Arc<Language>) -> Self {
        let empty: Box<[u16]> = Box::new([]);
        let mut index = HashMap::new();
        index.insert(empty.clone(), ROOT);
        Highlighter { lang, ends: Vec::new(), valid: 0, converge_from: 0, stacks: vec![empty], index, restyled: None, scratch: Vec::new() }
    }

    pub fn language(&self) -> &Arc<Language> {
        &self.lang
    }

    /// Lines whose end state is known.
    pub fn valid_lines(&self) -> usize {
        self.valid
    }

    /// Whether every line of `buffer` is tokenized.
    pub fn is_complete(&self, buffer: &dyn Buffer) -> bool {
        self.valid >= buffer.len_lines()
    }

    fn intern(&mut self, stack: &[u16]) -> StateId {
        if stack.is_empty() {
            return ROOT;
        }
        if let Some(&id) = self.index.get(stack) {
            return id;
        }
        let id = self.stacks.len() as StateId;
        let b: Box<[u16]> = stack.into();
        self.stacks.push(b.clone());
        self.index.insert(b, id);
        id
    }

    /// The stack a state stands for.
    pub fn stack(&self, id: StateId) -> &[u16] {
        self.stacks.get(id as usize).map_or(&[], |s| s)
    }

    /// The state line `line` starts in (lines up to it must be tokenized:
    /// see [`Highlighter::ensure`]).
    pub fn start_state(&self, line: usize) -> StateId {
        if line == 0 {
            ROOT
        } else {
            debug_assert!(line <= self.valid, "line {line} not tokenized yet");
            self.ends.get(line - 1).copied().unwrap_or(ROOT)
        }
    }

    /// The state line `line` ends in (tokenized up to it first).
    pub fn end_state(&mut self, buffer: &dyn Buffer, line: usize) -> StateId {
        self.ensure(buffer, line);
        self.ends.get(line).copied().unwrap_or(ROOT)
    }

    /// Tokenizes lines until line `line`'s end state is known.
    pub fn ensure(&mut self, buffer: &dyn Buffer, line: usize) {
        let n = buffer.len_lines();
        if self.ends.len() > n {
            self.ends.truncate(n);
            self.valid = self.valid.min(n);
        }
        let upto = line.min(n - 1);
        let mut stack: Vec<u16> = Vec::new();
        let mut scratch = std::mem::take(&mut self.scratch);
        while self.valid <= upto {
            let i = self.valid;
            stack.clear();
            stack.extend_from_slice(self.stack(self.start_state(i)));
            self.lang.tokenize_line(&buffer.line_text(i), &mut stack, &mut scratch);
            let end = self.intern(&stack);
            self.restyled = Some(match self.restyled.take() {
                Some(r) => r.start.min(i)..r.end.max(i + 1),
                None => i..i + 1,
            });
            if i < self.ends.len() {
                let old = self.ends[i];
                self.ends[i] = end;
                self.valid = i + 1;
                if old == end && i >= self.converge_from {
                    // the rest was tokenized from this same state
                    self.valid = self.ends.len();
                    self.converge_from = 0;
                }
            } else {
                self.ends.push(end);
                self.valid = i + 1;
            }
        }
        if self.valid >= self.ends.len() {
            self.converge_from = 0;
        } else {
            // stopped short: the line at `valid` was tokenized from what the
            // line before ended in then, which may have just changed, so a
            // later pass can't stop before it (a line's own old end state is
            // still a fair comparison: its text is the same)
            self.converge_from = self.converge_from.max(self.valid);
        }
        self.scratch = scratch;
    }

    /// Tokenizes up to `budget` more lines (for idle time); true when every
    /// line is done.
    pub fn advance(&mut self, buffer: &dyn Buffer, budget: usize) -> bool {
        if budget > 0 && self.valid < buffer.len_lines() {
            self.ensure(buffer, self.valid + budget - 1);
        }
        self.is_complete(buffer)
    }

    /// Line `line`'s tokens.
    pub fn tokens(&mut self, buffer: &dyn Buffer, line: usize) -> Vec<Token> {
        let mut out = Vec::new();
        self.tokens_into(buffer, line, &mut out);
        out
    }

    /// Line `line`'s tokens, into `out` (cleared first).
    pub fn tokens_into(&mut self, buffer: &dyn Buffer, line: usize, out: &mut Vec<Token>) {
        if line >= buffer.len_lines() {
            out.clear();
            return;
        }
        if line > 0 {
            self.ensure(buffer, line - 1);
        }
        let mut stack = self.stack(self.start_state(line)).to_vec();
        self.lang.tokenize_line(&buffer.line_text(line), &mut stack, out);
    }

    /// Lines `start_line .. start_line + old_count` were replaced by
    /// `new_count` lines (both at least 1).
    pub fn edited(&mut self, start_line: usize, old_count: usize, new_count: usize) {
        let (old_count, new_count) = (old_count.max(1), new_count.max(1));
        if start_line >= self.ends.len() {
            return;
        }
        let old_last = start_line + old_count - 1;
        // the last edited line ends where the old last line did, as far as
        // anyone knows: the usual edit (inside one line) stops right there
        let keep_last = self.ends.get(old_last).copied();
        let removed_end = (start_line + old_count).min(self.ends.len());
        match keep_last {
            Some(last) => {
                let mut fill = vec![ROOT; new_count];
                fill[new_count - 1] = last;
                self.ends.splice(start_line..removed_end, fill);
            }
            None => self.ends.truncate(start_line),
        }
        self.valid = self.valid.min(start_line);
        // where re-tokenizing may stop: past this edit and every earlier one
        let shifted = if self.converge_from > old_last {
            self.converge_from + new_count - old_count
        } else {
            self.converge_from.min(start_line + new_count - 1)
        };
        self.converge_from = shifted.max(start_line + new_count - 1);
        if self.converge_from >= self.ends.len() {
            self.converge_from = self.ends.len();
        }
        if let Some(r) = self.restyled.take() {
            // (shift what the view hasn't taken yet)
            let map = |l: usize| if l > old_last { l + new_count - old_count } else { l.min(start_line + new_count) };
            self.restyled = Some(map(r.start)..map(r.end).max(map(r.start)));
        }
    }

    /// The text changed beyond recognition (or the language changed):
    /// forget everything.
    pub fn reset(&mut self, lang: Option<Arc<Language>>) {
        if let Some(l) = lang {
            *self = Highlighter::new(l);
        } else {
            self.ends.clear();
            self.valid = 0;
            self.converge_from = 0;
        }
        self.restyled = None;
    }

    /// The lines tokenized again since the last call (their colours may have
    /// changed although their text didn't: a `/*` opened above, say).
    pub fn take_restyled(&mut self) -> Option<Range<usize>> {
        self.restyled.take()
    }

    /// A line's tokens from a state, and the state it leaves (states as
    /// this highlighter interns them): how the code editor's view
    /// (`rapidr-ui-kernel`'s `components::codeeditor`) colours a line that
    /// isn't in the document, such as a hover's code.
    pub fn line_spans(&mut self, line: &str, state: StateId) -> (Vec<Token>, StateId) {
        let mut stack = self.stack(state).to_vec();
        let mut out = Vec::new();
        self.lang.tokenize_line(line, &mut stack, &mut out);
        let end = self.intern(&stack);
        (out, end)
    }

    /// Heap bytes the states take (for the memory budget).
    pub fn approx_heap_bytes(&self) -> usize {
        self.ends.capacity() * 4 + self.stacks.iter().map(|s| s.len() * 2 + 16).sum::<usize>() * 2
    }
}
