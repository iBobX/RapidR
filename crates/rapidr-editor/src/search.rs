//! Find and replace: literal or regex (the `regex` crate), case, whole
//! word, inside the selections, incremental (from an origin, as the pattern
//! is typed); replacements with groups (`$1`, `${name}`) in regex mode.
//!
//! Searches run over the whole text (cached by the document until the next
//! edit), so patterns may span lines; `^` and `$` are line starts and ends
//! (CR LF too).

use std::ops::Range;

use regex::{Regex, RegexBuilder};

use crate::document::Document;
use crate::history::EditKind;
use crate::selection::{Selection, Selections};
use crate::transaction::{Change, ChangeSet, EditError};

/// What to look for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SearchQuery {
    pub pattern: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
}

impl SearchQuery {
    pub fn literal(pattern: impl Into<String>) -> Self {
        SearchQuery { pattern: pattern.into(), ..Default::default() }
    }

    pub fn regex(pattern: impl Into<String>) -> Self {
        SearchQuery { pattern: pattern.into(), regex: true, ..Default::default() }
    }

    /// Reads RCodeEditor's option string: any of `"case"`, `"word"`,
    /// `"regex"` (comma / space separated, any case).
    pub fn with_options(pattern: impl Into<String>, options: &str) -> Self {
        let has = |o: &str| options.split([',', ' ', ';']).any(|w| w.trim().eq_ignore_ascii_case(o));
        SearchQuery { pattern: pattern.into(), case_sensitive: has("case"), whole_word: has("word"), regex: has("regex") }
    }
}

/// A pattern that could not be compiled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchError(pub String);

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SearchError {}

/// A compiled query.
#[derive(Clone, Debug)]
pub struct Searcher {
    query: SearchQuery,
    re: Regex,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl Searcher {
    pub fn new(query: &SearchQuery) -> Result<Searcher, SearchError> {
        if query.pattern.is_empty() {
            return Err(SearchError("nothing to find".into()));
        }
        let pattern = if query.regex { query.pattern.clone() } else { regex::escape(&query.pattern) };
        let re = RegexBuilder::new(&pattern)
            .case_insensitive(!query.case_sensitive)
            .multi_line(true)
            .crlf(true)
            .size_limit(1 << 24)
            .build()
            .map_err(|e| SearchError(e.to_string()))?;
        Ok(Searcher { query: query.clone(), re })
    }

    pub fn query(&self) -> &SearchQuery {
        &self.query
    }

    fn whole_word_ok(&self, text: &str, m: &Range<usize>) -> bool {
        if !self.query.whole_word {
            return true;
        }
        let before = text[..m.start].chars().next_back();
        let after = text[m.end..].chars().next();
        let first = text[m.start..m.end].chars().next();
        let last = text[m.start..m.end].chars().next_back();
        // a boundary on each side that has a word character inside
        !(first.is_some_and(is_word) && before.is_some_and(is_word)) && !(last.is_some_and(is_word) && after.is_some_and(is_word))
    }

    /// Every match inside `within` (non-empty ones only), in order.
    pub fn find_all(&self, text: &str, within: Range<usize>) -> Vec<Range<usize>> {
        let mut out = Vec::new();
        let mut at = within.start;
        while at <= within.end {
            let Some(m) = self.re.find_at(&text[..within.end], at) else { break };
            let r = m.range();
            if r.is_empty() {
                at = r.end + text[r.end..].chars().next().map_or(1, char::len_utf8);
                continue;
            }
            if self.whole_word_ok(text, &r) {
                at = r.end;
                out.push(r);
            } else {
                at = r.start + text[r.start..].chars().next().map_or(1, char::len_utf8);
            }
        }
        out
    }

    /// The first match starting at or after `from`.
    pub fn find_from(&self, text: &str, from: usize) -> Option<Range<usize>> {
        let mut at = from;
        while at <= text.len() {
            let m = self.re.find_at(text, at)?;
            let r = m.range();
            if !r.is_empty() && self.whole_word_ok(text, &r) {
                return Some(r);
            }
            at = r.start + text[r.start..].chars().next().map_or(1, char::len_utf8);
        }
        None
    }

    /// The last match ending at or before `before`.
    pub fn find_before(&self, text: &str, before: usize) -> Option<Range<usize>> {
        // (regexes search forward: the last of the matches before)
        self.find_all(text, 0..before).pop()
    }

    /// What match `m` is replaced with: `replacement` with `$1` / `${name}`
    /// expanded in regex mode, as written otherwise.
    pub fn replacement(&self, text: &str, m: &Range<usize>, replacement: &str) -> String {
        if !self.query.regex {
            return replacement.to_string();
        }
        match self.re.captures_at(&text[..m.end], m.start) {
            Some(caps) if caps.get(0).is_some_and(|c| c.range() == *m) => {
                let mut out = String::new();
                caps.expand(replacement, &mut out);
                out
            }
            _ => replacement.to_string(),
        }
    }
}

impl Document {
    /// Every match, in the whole text or (`in_selection`) inside the
    /// non-empty selections.
    pub fn find_all(&self, query: &SearchQuery, in_selection: bool) -> Result<Vec<Range<usize>>, SearchError> {
        let s = Searcher::new(query)?;
        let text = self.text();
        if !in_selection {
            return Ok(s.find_all(&text, 0..text.len()));
        }
        Ok(self.selections().iter().filter(|r| !r.is_empty()).flat_map(|r| s.find_all(&text, r.range())).collect())
    }

    /// The next (or previous) match from `from`, wrapping around.
    pub fn find_next(&self, query: &SearchQuery, from: usize, forward: bool) -> Result<Option<Range<usize>>, SearchError> {
        let s = Searcher::new(query)?;
        let text = self.text();
        Ok(if forward { s.find_from(&text, from).or_else(|| s.find_from(&text, 0)) } else { s.find_before(&text, from).or_else(|| s.find_before(&text, text.len())) })
    }

    /// Find / FindNext: selects the next match after the primary selection
    /// (or before it, backwards); false when there is none.
    pub fn select_next_match(&mut self, query: &SearchQuery, forward: bool) -> Result<bool, SearchError> {
        let p = self.selections().primary();
        let from = if forward { p.end() } else { p.start() };
        match self.find_next(query, from, forward)? {
            Some(r) => {
                self.set_selections(Selections::single(Selection::new(r.start, r.end)));
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Incremental search: the first match at or after `origin` (where the
    /// search started), wrapping; selected when found.
    pub fn incremental_find(&mut self, query: &SearchQuery, origin: usize) -> Result<Option<Range<usize>>, SearchError> {
        let found = self.find_next(query, origin, true)?;
        if let Some(r) = &found {
            self.set_selections(Selections::single(Selection::new(r.start, r.end)));
        }
        Ok(found)
    }

    /// Replace: when the primary selection is a match, replaces it and
    /// selects the next one; otherwise just finds the next. True when
    /// something was replaced.
    pub fn replace_next(&mut self, query: &SearchQuery, replacement: &str, now_ms: u64) -> Result<bool, ReplaceError> {
        let s = Searcher::new(query)?;
        let text = self.text();
        let p = self.selections().primary();
        let current = s.find_from(&text, p.start()).filter(|r| *r == p.range());
        let Some(m) = current else {
            self.select_next_match(query, true)?;
            return Ok(false);
        };
        let with = s.replacement(&text, &m, replacement);
        self.replace_range(m.clone(), &with, now_ms)?;
        let after = m.start + with.len();
        let text = self.text();
        if let Some(r) = s.find_from(&text, after).or_else(|| s.find_from(&text, 0)) {
            self.set_selections(Selections::single(Selection::new(r.start, r.end)));
        }
        Ok(true)
    }

    /// ReplaceAll: every match (in the whole text or inside the selections)
    /// as one undo step; the number replaced.
    pub fn replace_all(&mut self, query: &SearchQuery, replacement: &str, in_selection: bool, now_ms: u64) -> Result<usize, ReplaceError> {
        let s = Searcher::new(query)?;
        let matches = self.find_all(query, in_selection)?;
        if matches.is_empty() {
            return Ok(0);
        }
        let text = self.text();
        let changes: Vec<Change> = matches.iter().map(|m| Change::new(m.clone(), s.replacement(&text, m, replacement))).collect();
        let n = changes.len();
        let set = ChangeSet::new(changes, self.len_bytes())?;
        let after = self.selections().map(&set);
        self.apply(set, after, EditKind::Command, now_ms)?;
        Ok(n)
    }
}

/// Why a replace failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplaceError {
    Search(SearchError),
    Edit(EditError),
}

impl From<SearchError> for ReplaceError {
    fn from(e: SearchError) -> Self {
        ReplaceError::Search(e)
    }
}

impl From<EditError> for ReplaceError {
    fn from(e: EditError) -> Self {
        ReplaceError::Edit(e)
    }
}

impl std::fmt::Display for ReplaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplaceError::Search(e) => e.fmt(f),
            ReplaceError::Edit(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ReplaceError {}
