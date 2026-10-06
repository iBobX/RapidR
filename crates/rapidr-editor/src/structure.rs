//! What the tokens tell about structure: fold ranges (by the language's
//! markers, by multi-line bracket pairs, or by indentation; embedded
//! regions fold too) and bracket matching. Brackets and markers inside
//! strings and comments don't count.
//!
//! The language service's outline (stage I3) adds folds for BASIC's real
//! structure later; these work for every language today.

use std::ops::Range;

use crate::buffer::Buffer;
use crate::document::{display_width, leading_ws, Document};
use crate::lang::{Language, Token};

/// How a fold was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FoldKind {
    /// A start / end marker pair (`SUB` … `END SUB`).
    Marker,
    /// A bracket pair over several lines.
    Bracket,
    /// Lines indented deeper than the first.
    Indent,
    /// An embedded language's region (`RUSTSTART` … `RUSTEND`, `<script>`).
    Embedded,
}

/// Lines `start_line + 1 ..= end_line` can be hidden under `start_line`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FoldRange {
    pub start_line: usize,
    /// For markers, the end marker's line (folded too); for brackets, the
    /// line with the closing bracket; for indentation, the last indented
    /// line.
    pub end_line: usize,
    pub kind: FoldKind,
}

/// Scans at most this many lines for a matching bracket.
pub const BRACKET_SCAN_LINES: usize = 20_000;

fn not_in_literal(tokens: &[Token], col: usize) -> bool {
    !tokens.iter().any(|t| t.kind.is_literal() && (t.start as usize) <= col && col < t.end as usize)
}

impl Document {
    /// Every fold range, sorted by start line (tokenizing the whole text
    /// first if it isn't yet).
    pub fn fold_ranges(&mut self) -> Vec<FoldRange> {
        let lang = self.language().clone();
        let n = self.line_count();
        let tab = self.tab_size;
        let (buf, hl) = self.highlighter_mut();
        hl.ensure(buf, n - 1);
        let mut out = Vec::new();

        // embedded regions: where a line enters and where it leaves one
        // (`inside[l]`: line l starts inside one; `inside[n]`: the text ends
        // inside one)
        let last_end = hl.end_state(buf, n - 1);
        let inside: Vec<bool> = (0..=n).map(|l| Language::in_embedded(hl.stack(if l < n { hl.start_state(l) } else { last_end }))).collect();
        let mut region_start: Option<usize> = None;
        for line in 0..n {
            if !inside[line] && inside[line + 1] {
                region_start = Some(line);
            } else if inside[line] && !inside[line + 1] {
                if let Some(s) = region_start.take() {
                    if line > s {
                        out.push(FoldRange { start_line: s, end_line: line, kind: FoldKind::Embedded });
                    }
                }
            }
        }

        let outside = |line: usize| !inside[line];
        let mut tokens = Vec::new();

        // markers
        if let Some(set) = &lang.fold_marker_set {
            let k = lang.fold_markers.len();
            let mut stack: Vec<(usize, usize)> = Vec::new();
            for line in 0..n {
                let text = buf.line_text(line);
                let hits = set.matches(&text);
                if !hits.matched_any() || !outside(line) {
                    continue;
                }
                // (a marker at the first word of a line that starts outside
                // any string or comment needs no tokens: the usual case)
                let first_word = leading_ws(&text).len();
                let plain_start = hl.start_state(line) == crate::highlight::ROOT;
                let mut tokenized = false;
                for i in hits.iter() {
                    let (marker, is_end) = if i < k { (i, false) } else { (i - k, true) };
                    let re = if is_end { &lang.fold_markers[marker].1 } else { &lang.fold_markers[marker].0 };
                    let Some(m) = re.find(&text) else { continue };
                    let at = m.start() + (m.as_str().len() - m.as_str().trim_start().len());
                    if !(plain_start && at == first_word) {
                        if !tokenized {
                            hl.tokens_into(buf, line, &mut tokens);
                            tokenized = true;
                        }
                        if !not_in_literal(&tokens, at) {
                            continue;
                        }
                    }
                    if is_end {
                        if let Some(pos) = stack.iter().rposition(|&(mk, _)| mk == marker) {
                            let (_, start) = stack[pos];
                            stack.truncate(pos);
                            if line > start {
                                out.push(FoldRange { start_line: start, end_line: line, kind: FoldKind::Marker });
                            }
                        }
                    } else {
                        stack.push((marker, line));
                    }
                }
            }
        }

        // multi-line bracket pairs
        if lang.fold_brackets && !lang.brackets.is_empty() {
            let mut stack: Vec<(char, usize)> = Vec::new();
            for line in 0..n {
                if !outside(line) {
                    continue;
                }
                let text = buf.line_text(line);
                if !text.contains(|c: char| lang.brackets.iter().any(|&(o, cl)| o == c || cl == c)) {
                    continue;
                }
                hl.tokens_into(buf, line, &mut tokens);
                for (col, c) in text.char_indices() {
                    if let Some(&(o, _)) = lang.brackets.iter().find(|&&(o, _)| o == c) {
                        if not_in_literal(&tokens, col) {
                            stack.push((o, line));
                        }
                    } else if let Some(&(o, _)) = lang.brackets.iter().find(|&&(_, cl)| cl == c) {
                        if !not_in_literal(&tokens, col) {
                            continue;
                        }
                        if let Some(pos) = stack.iter().rposition(|&(so, _)| so == o) {
                            let (_, start) = stack[pos];
                            stack.truncate(pos);
                            // (`{` then `}` alone on the next line: nothing to hide)
                            if line > start + 1 || (line > start && !text.trim_start().starts_with(c)) {
                                out.push(FoldRange { start_line: start, end_line: line, kind: FoldKind::Bracket });
                            }
                        }
                    }
                }
            }
        }

        // indentation
        if lang.fold_offside {
            out.extend(indent_folds(buf, tab));
        }

        out.sort_by_key(|f| (f.start_line, std::cmp::Reverse(f.end_line)));
        out.dedup_by_key(|f| f.start_line);
        out
    }

    /// The bracket at `byte` (or just before it) and its match: (that
    /// bracket's offset, the match's offset). Brackets in strings and
    /// comments match only each other.
    pub fn matching_bracket(&mut self, byte: usize) -> Option<(usize, usize)> {
        let lang = self.language().clone();
        if lang.brackets.is_empty() {
            return None;
        }
        let candidates = [Some(byte), byte.checked_sub(1)];
        for at in candidates.into_iter().flatten() {
            let Some(c) = self.buffer().char_after(at) else { continue };
            if !self.buffer().is_char_boundary(at) {
                continue;
            }
            let Some(&(open, close)) = lang.brackets.iter().find(|&&(o, cl)| o == c || cl == c) else { continue };
            if let Some(m) = self.scan_bracket(at, open, close, c == open) {
                return Some((at, m));
            }
        }
        None
    }

    fn scan_bracket(&mut self, at: usize, open: char, close: char, forward: bool) -> Option<usize> {
        let n = self.line_count();
        let line = self.buffer().line_of(at);
        let literal_at = |tokens: &[Token], col: usize| !not_in_literal(tokens, col);
        let start_tokens = self.tokens(line);
        let col0 = at - self.buffer().line_start(line);
        let in_literal = literal_at(&start_tokens, col0);
        let mut depth = 0i64;
        let mut l = line;
        let mut scanned = 0;
        loop {
            let tokens = if l == line { start_tokens.clone() } else { self.tokens(l) };
            let text = self.line(l).into_owned();
            let ls = self.buffer().line_start(l);
            let chars: Vec<(usize, char)> = text.char_indices().collect();
            let iter: Box<dyn Iterator<Item = &(usize, char)>> = if forward { Box::new(chars.iter()) } else { Box::new(chars.iter().rev()) };
            for &(col, c) in iter {
                if l == line && ((forward && col < col0) || (!forward && col > col0)) {
                    continue;
                }
                if c != open && c != close {
                    continue;
                }
                if literal_at(&tokens, col) != in_literal {
                    continue;
                }
                let toward = if forward { c == open } else { c == close };
                if toward {
                    depth += 1;
                } else {
                    depth -= 1;
                    if depth == 0 {
                        return Some(ls + col);
                    }
                }
            }
            scanned += 1;
            if scanned > BRACKET_SCAN_LINES {
                return None;
            }
            if forward {
                l += 1;
                if l >= n {
                    return None;
                }
            } else {
                if l == 0 {
                    return None;
                }
                l -= 1;
            }
        }
    }
}

/// Folds by indentation: a line followed by deeper-indented lines (blank
/// lines join the block they're in).
pub fn indent_folds(buf: &dyn Buffer, tab: u32) -> Vec<FoldRange> {
    let n = buf.len_lines();
    let indents: Vec<Option<u32>> = (0..n)
        .map(|l| {
            let t = buf.line_text(l);
            if t.trim().is_empty() {
                None
            } else {
                Some(display_width(leading_ws(&t), tab))
            }
        })
        .collect();
    let mut out = Vec::new();
    // (a stack of open blocks: (indent of the header, header line))
    let mut stack: Vec<(u32, usize)> = Vec::new();
    let mut last_non_blank: Option<usize> = None;
    for (l, ind) in indents.iter().enumerate() {
        let Some(ind) = *ind else { continue };
        while let Some(&(h_ind, h_line)) = stack.last() {
            if ind > h_ind {
                break;
            }
            stack.pop();
            if let Some(last) = last_non_blank {
                if last > h_line {
                    out.push(FoldRange { start_line: h_line, end_line: last, kind: FoldKind::Indent });
                }
            }
        }
        stack.push((ind, l));
        last_non_blank = Some(l);
    }
    while let Some((_, h_line)) = stack.pop() {
        if let Some(last) = last_non_blank {
            if last > h_line {
                out.push(FoldRange { start_line: h_line, end_line: last, kind: FoldKind::Indent });
            }
        }
    }
    out
}

/// The lines `fold` hides.
pub fn hidden_lines(fold: &FoldRange) -> Range<usize> {
    fold.start_line + 1..fold.end_line + 1
}
