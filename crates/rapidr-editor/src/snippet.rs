//! Snippets (a language definition's `[[snippets]]`): a body with tab stops
//! — `$1`, `${1:default}`, `$0` (the final caret) — expanded at a caret: its
//! tabs become the document's indent unit, its lines continue the caret
//! line's indentation and use the file's line ending.

use std::ops::Range;

use crate::document::{leading_ws, Document};
use crate::history::EditKind;
use crate::lang::Snippet;
use crate::selection::{Selection, Selections};
use crate::transaction::{Change, EditError};
use crate::Buffer;

/// A snippet body ready to insert: its text and its tab stops (byte ranges
/// in the text, in visiting order: 1, 2, … then 0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expansion {
    pub text: String,
    pub stops: Vec<Range<usize>>,
}

/// Expands `body` with `indent` after each line break, `unit` for each tab
/// and `le` as the line break.
pub fn expand(body: &str, indent: &str, unit: &str, le: &str) -> Expansion {
    let mut text = String::new();
    let mut stops: Vec<(u32, Range<usize>)> = Vec::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if matches!(chars.peek(), Some('$' | '}' | '\\')) => text.push(chars.next().unwrap_or('\\')),
            '\n' => {
                text.push_str(le);
                text.push_str(indent);
            }
            '\t' => text.push_str(unit),
            '$' if chars.peek().is_some_and(char::is_ascii_digit) => {
                let mut n = String::new();
                while let Some(d) = chars.peek().filter(|d| d.is_ascii_digit()) {
                    n.push(*d);
                    chars.next();
                }
                let at = text.len();
                stops.push((n.parse().unwrap_or(0), at..at));
            }
            '$' if chars.peek() == Some(&'{') => {
                chars.next();
                let mut n = String::new();
                while let Some(d) = chars.peek().filter(|d| d.is_ascii_digit()) {
                    n.push(*d);
                    chars.next();
                }
                let start = text.len();
                if chars.peek() == Some(&':') {
                    chars.next();
                    // the default text, to the matching `}`
                    let mut depth = 0;
                    for d in chars.by_ref() {
                        match d {
                            '{' => depth += 1,
                            '}' if depth == 0 => break,
                            '}' => depth -= 1,
                            _ => {}
                        }
                        if d == '\n' {
                            text.push_str(le);
                            text.push_str(indent);
                        } else {
                            text.push(d);
                        }
                    }
                } else {
                    for d in chars.by_ref() {
                        if d == '}' {
                            break;
                        }
                    }
                }
                stops.push((n.parse().unwrap_or(0), start..text.len()));
            }
            c => text.push(c),
        }
    }
    // 1, 2, … then 0 (the end when there is no $0); a number used twice
    // (`NEXT ${1:i}`) keeps its first place
    stops.sort_by_key(|(n, _)| if *n == 0 { u32::MAX } else { *n });
    let mut seen = Vec::new();
    stops.retain(|(n, _)| {
        let first = !seen.contains(n);
        seen.push(*n);
        first
    });
    if !seen.contains(&0) {
        stops.push((0, text.len()..text.len()));
    }
    Expansion { text, stops: stops.into_iter().map(|(_, r)| r).collect() }
}

impl Document {
    /// Inserts `snippet` at every selection (replacing the selected text, and
    /// `prefix_len` bytes typed before each caret — the word that triggered
    /// it), selecting its first tab stop at each. One undo step. Returns the
    /// expansion (for the view's tab-stop session).
    pub fn insert_snippet(&mut self, snippet: &Snippet, prefix_len: usize, now_ms: u64) -> Result<Expansion, EditError> {
        let unit = self.indent_unit();
        let le = self.line_ending().as_str();
        let mut first = None;
        let sels: Vec<Selection> = self.selections().ranges().to_vec();
        let mut changes = Vec::new();
        let mut afters = Vec::new();
        let mut delta: isize = 0;
        for s in sels {
            let line = self.buffer().line_of(s.start());
            let indent = leading_ws(&self.line(line)).to_string();
            let e = expand(&snippet.body, &indent, &unit, le);
            let start = self.buffer().clamp(s.start().saturating_sub(prefix_len).max(self.buffer().line_start(line)));
            let stop = e.stops[0].clone();
            let base = (start as isize + delta) as usize;
            afters.push(Selection::new(base + stop.start, base + stop.end));
            delta += e.text.len() as isize - (s.end() - start) as isize;
            changes.push(Change::new(start..s.end(), e.text.clone()));
            first.get_or_insert(e);
        }
        let primary = self.selections().primary_index();
        let set = crate::transaction::ChangeSet::new(changes, self.len_bytes())?;
        self.apply(set, Selections::new(afters, primary), EditKind::Command, now_ms)?;
        Ok(first.unwrap_or_else(|| expand("", "", "", "")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::Languages;

    #[test]
    fn expansions() {
        let e = expand("SUB ${1:Name}(${2})\n\t$0\nEND SUB", "  ", "    ", "\r\n");
        assert_eq!(e.text, "SUB Name()\r\n      \r\n  END SUB");
        assert_eq!(e.stops, [4..8, 9..9, 18..18]);
        let e = expand("FOR ${1:i} = 1 TO 2\nNEXT ${1:i}", "", "\t", "\n");
        assert_eq!(e.stops, [4..5, e.text.len()..e.text.len()]);
        assert_eq!(expand("a \\$1 b", "", "", "\n").text, "a $1 b");
    }

    #[test]
    fn insert_at_carets() {
        let lang = Languages::builtin().get("rapidr-basic").unwrap();
        let sub = lang.snippets.iter().find(|s| s.prefix == "sub").unwrap().clone();
        let mut d = Document::new("  sub", lang);
        d.set_selections(Selections::caret(5));
        d.insert_snippet(&sub, 3, 0).unwrap();
        assert_eq!(&*d.text(), "  SUB Name()\n      \n  END SUB");
        assert_eq!(d.selections().primary(), Selection::new(6, 10));
        d.undo();
        assert_eq!(&*d.text(), "  sub");
    }
}
