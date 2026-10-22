//! Text diffs: the runs of changed lines, unified hunks with context, and
//! the changed words of a changed line.
//!
//! - **Lines** are compared whole ([`split_lines`] keeps each line's own
//!   line break, so "a" at the end of a file and "a\n" differ, as Git sees
//!   it). Equal lines get one id ([`crate::intern`]) and [`crate::myers`]
//!   finds the fewest lines removed and added.
//! - **Sliding**: a run of only added (or only removed) lines can often
//!   sit higher or lower with the same result (a new `SUB … END SUB` after
//!   a blank line, or the blank line after it). Each such run is moved as
//!   far down as it goes, which keeps a block and its trailing blank line
//!   together in code: what a person would call "the added lines".
//! - **Hunks** number lines from 1 like a unified diff:
//!   `@@ -old_start,old_len +new_start,new_len @@`; a side with no lines
//!   starts at the line before.
//! - **Words**: within a removed line and the added line paired with it,
//!   the changed words (runs of letters, digits and `_`), spaces and
//!   single symbols, as byte ranges. Lines with too little in common get
//!   none: the whole line's tint says enough.

use std::hash::Hash;
use std::ops::Range;

use crate::myers::{self, Changes};

/// A run of changed lines between unchanged ones: `old` removed, `new`
/// added (0-based line indexes). One of them may be empty: a pure
/// insertion or deletion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub old: Range<usize>,
    pub new: Range<usize>,
}

/// The lines of `text`, each with its own line break ("\n", "\r\n"); the
/// last one without when the text doesn't end in one.
pub fn split_lines(text: &[u8]) -> Vec<&[u8]> {
    text.split_inclusive(|b| *b == b'\n').collect()
}

/// The runs of changed items between `old` and `new`, in order.
pub fn changes<T: Hash + Eq>(old: &[T], new: &[T]) -> Vec<Change> {
    let (a, b) = crate::intern(old, new);
    let mut c = myers::diff(&a, &b);
    slide(&a, &b, &mut c);
    runs(&c)
}

/// The runs of `c`'s changes: changed items on either side between two
/// pairs of kept ones.
pub fn runs(c: &Changes) -> Vec<Change> {
    let (n, m) = (c.old.len(), c.new.len());
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < n || j < m {
        if i < n && j < m && !c.old[i] && !c.new[j] {
            i += 1;
            j += 1;
            continue;
        }
        let (i0, j0) = (i, j);
        loop {
            if i < n && c.old[i] {
                i += 1;
            } else if j < m && c.new[j] {
                j += 1;
            } else {
                break;
            }
        }
        if (i, j) == (i0, j0) {
            // (kept items always pair up; never loop forever)
            break;
        }
        out.push(Change { old: i0..i, new: j0..j });
    }
    out
}

/// Moves every run of only added or only removed items as far down as it
/// goes (see the module's notes).
fn slide(a: &[u32], b: &[u32], c: &mut Changes) {
    for run in runs(c) {
        if run.old.is_empty() {
            let (mut s, mut e) = (run.new.start, run.new.end);
            while e < b.len() && !c.new[e] && b[s] == b[e] {
                c.new[s] = false;
                c.new[e] = true;
                s += 1;
                e += 1;
            }
        } else if run.new.is_empty() {
            let (mut s, mut e) = (run.old.start, run.old.end);
            while e < a.len() && !c.old[e] && a[s] == a[e] {
                c.old[s] = false;
                c.old[e] = true;
                s += 1;
                e += 1;
            }
        }
    }
}

/// One line of a hunk, by its index in the old or the new text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HunkLine {
    Same { old: usize, new: usize },
    Removed(usize),
    Added(usize),
}

/// Changes with up to `context` unchanged lines around them, numbered
/// from 1 as a unified diff numbers them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: usize,
    pub old_len: usize,
    pub new_start: usize,
    pub new_len: usize,
    pub lines: Vec<HunkLine>,
}

impl Hunk {
    /// `@@ -1,8 +1,9 @@`, as `git diff` writes it.
    pub fn header(&self) -> String {
        format!("@@ -{},{} +{},{} @@", self.old_start, self.old_len, self.new_start, self.new_len)
    }
}

/// The hunks of `changes` (two texts of `old_lines` and `new_lines`
/// lines): changes closer than `2 × context` unchanged lines share one.
pub fn hunks(changes: &[Change], old_lines: usize, new_lines: usize, context: usize) -> Vec<Hunk> {
    let mut groups: Vec<&[Change]> = Vec::new();
    let mut start = 0;
    for k in 1..=changes.len() {
        if k == changes.len() || changes[k].old.start > changes[k - 1].old.end + 2 * context {
            if k > start {
                groups.push(&changes[start..k]);
            }
            start = k;
        }
    }
    let mut out = Vec::with_capacity(groups.len());
    for g in groups {
        let (first, last) = (&g[0], &g[g.len() - 1]);
        let lead = first.old.start.min(first.new.start).min(context);
        let (old_from, new_from) = (first.old.start - lead, first.new.start - lead);
        let trail = (old_lines - last.old.end).min(new_lines - last.new.end).min(context);
        let (old_to, new_to) = (last.old.end + trail, last.new.end + trail);
        let mut lines = Vec::new();
        let (mut i, mut j) = (old_from, new_from);
        for ch in g {
            while i < ch.old.start {
                lines.push(HunkLine::Same { old: i, new: j });
                i += 1;
                j += 1;
            }
            lines.extend(ch.old.clone().map(HunkLine::Removed));
            lines.extend(ch.new.clone().map(HunkLine::Added));
            i = ch.old.end;
            j = ch.new.end;
        }
        while i < old_to {
            lines.push(HunkLine::Same { old: i, new: j });
            i += 1;
            j += 1;
        }
        let (old_len, new_len) = (old_to - old_from, new_to - new_from);
        out.push(Hunk {
            old_start: if old_len == 0 { old_from } else { old_from + 1 },
            old_len,
            new_start: if new_len == 0 { new_from } else { new_from + 1 },
            new_len,
            lines,
        });
    }
    out
}

/// Changed parts of a line, as byte ranges.
pub type Marks = Vec<Range<usize>>;

/// Lines longer than this (in bytes) get no word marks.
const MAX_WORD_LINE: usize = 4000;

/// A token's class: runs of word characters or of spaces are one token
/// each; any other character is a token of its own.
fn class(c: char) -> u8 {
    if c.is_alphanumeric() || c == '_' {
        1
    } else if c.is_whitespace() {
        2
    } else {
        0
    }
}

/// `line`'s tokens as byte ranges.
pub fn tokens(line: &str) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut prev = None;
    for (i, c) in line.char_indices() {
        let k = class(c);
        match out.last_mut() {
            Some(last) if k != 0 && prev == Some(k) => last.end = i + c.len_utf8(),
            _ => out.push(i..i + c.len_utf8()),
        }
        prev = Some(k);
    }
    out
}

/// The changed words of `old` and `new` (a removed line and the added line
/// paired with it), as byte ranges of each; ranges only spaces apart are
/// one. `None` when the lines have too little in common for marks to help
/// (less than two fifths of their non-space text).
pub fn word_marks(old: &str, new: &str) -> Option<(Marks, Marks)> {
    if old == new {
        return Some((Vec::new(), Vec::new()));
    }
    if old.len() > MAX_WORD_LINE || new.len() > MAX_WORD_LINE {
        return None;
    }
    let (ta, tb) = (tokens(old), tokens(new));
    let sa: Vec<&str> = ta.iter().map(|r| &old[r.clone()]).collect();
    let sb: Vec<&str> = tb.iter().map(|r| &new[r.clone()]).collect();
    let (a, b) = crate::intern(&sa, &sb);
    let c = myers::diff(&a, &b);
    // alike enough: two fifths of the non-space text, or half the words
    // and symbols, in common
    let solid = |s: &str| s.chars().filter(|c| !c.is_whitespace()).count();
    let kept = || sa.iter().zip(&c.old).filter(|(s, ch)| !**ch && solid(s) > 0).map(|(s, _)| *s);
    let (common_chars, common_tokens) = (kept().map(solid).sum::<usize>(), kept().count());
    let words = |t: &[&str]| t.iter().filter(|s| solid(s) > 0).count();
    let (longest_chars, most_tokens) = (solid(old).max(solid(new)), words(&sa).max(words(&sb)));
    if common_chars * 5 < longest_chars * 2 && common_tokens * 2 < most_tokens {
        return None;
    }
    Some((marked(old, &ta, &c.old), marked(new, &tb, &c.new)))
}

/// The byte ranges of the changed tokens, joined across unchanged spaces.
fn marked(line: &str, tokens: &[Range<usize>], changed: &[bool]) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    for (r, ch) in tokens.iter().zip(changed) {
        if !*ch {
            continue;
        }
        match out.last_mut() {
            Some(last) if line[last.end..r.start].chars().all(char::is_whitespace) => last.end = r.end,
            _ => out.push(r.clone()),
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    fn lines(t: &str) -> Vec<&str> {
        t.split('\n').collect()
    }

    #[test]
    fn runs_of_changes() {
        assert!(changes::<&str>(&[], &[]).is_empty());
        assert_eq!(changes(&lines(""), &lines("a\nb")), [Change { old: 0..1, new: 0..2 }]);
        let a = lines("1\n2\n3\n4\n5\n6\n7");
        let b = lines("1\nTWO\n3\n3.5\n4\n6\n7");
        assert_eq!(changes(&a, &b), [Change { old: 1..2, new: 1..2 }, Change { old: 3..3, new: 3..4 }, Change { old: 4..5, new: 5..5 }]);
    }

    /// A new SUB after a blank line: the added lines are the SUB and its
    /// trailing blank line, not a blank line and the SUB.
    #[test]
    fn insertions_slide_down() {
        let old = lines("SUB A\nEND SUB\n\nSUB B\nEND SUB");
        let new = lines("SUB A\nEND SUB\n\nSUB New\nEND SUB\n\nSUB B\nEND SUB");
        assert_eq!(changes(&old, &new), [Change { old: 3..3, new: 3..6 }]);
        let removed = changes(&new, &old);
        assert_eq!(removed, [Change { old: 3..6, new: 3..3 }]);
    }

    #[test]
    fn unified_numbering() {
        let old = split_lines(b"a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\n");
        let new = split_lines(b"a\nB\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nM\nn\nadded\n");
        let h = hunks(&changes(&old, &new), old.len(), new.len(), 3);
        let headers: Vec<String> = h.iter().map(Hunk::header).collect();
        assert_eq!(headers, ["@@ -1,5 +1,5 @@", "@@ -10,5 +10,6 @@"]);
        assert_eq!(h[1].lines.last(), Some(&HunkLine::Added(14)));
        // an empty file, then two lines
        let h = hunks(&changes(&split_lines(b""), &split_lines(b"x\ny\n")), 0, 2, 3);
        assert_eq!(h[0].header(), "@@ -0,0 +1,2 @@");
        // (close changes share a hunk)
        let old = split_lines(b"1\n2\n3\n4\n5\n6\n7\n8\n");
        let new = split_lines(b"1\nX\n3\n4\n5\n6\nY\n8\n");
        let h = hunks(&changes(&old, &new), 8, 8, 3);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].header(), "@@ -1,8 +1,8 @@");
        let h = hunks(&changes(&old, &new), 8, 8, 1);
        assert_eq!(h.iter().map(Hunk::header).collect::<Vec<_>>(), ["@@ -1,3 +1,3 @@", "@@ -6,3 +6,3 @@"]);
    }

    #[test]
    fn word_level_marks() {
        let (a, b) = word_marks("  PRINT \"Hello\", x", "  PRINT \"Hello, World\", x").unwrap();
        assert!(a.is_empty(), "{a:?}");
        assert_eq!(b, [14..21]); // ", World"
        let (a, b) = word_marks("Caption = \"OK\"", "Caption = \"Save\"").unwrap();
        assert_eq!((a, b), (vec![11..13], vec![11..15]));
        // words, not letters: "count" → "counter" marks the whole word
        let (a, b) = word_marks("x = count + 1", "x = counter + 1").unwrap();
        assert_eq!((a, b), (vec![4..9], vec![4..11]));
        // two changed words a space apart are one mark
        let (a, b) = word_marks("PRINT a b", "PRINT x y").unwrap();
        assert_eq!((a, b), (vec![6..9], vec![6..9]));
        let (a, b) = word_marks("Left = 10: Top = 20", "Left = 15 Top = 20").unwrap();
        assert_eq!((a, b), (vec![7..10], vec![7..9]));
        // nothing alike: no marks
        assert_eq!(word_marks("abc def", "uvw xyz"), None);
        assert_eq!(word_marks("same", "same"), Some((vec![], vec![])));
        // (non-ASCII: byte ranges on character boundaries)
        let (a, b) = word_marks("título = \"año\"", "título = \"años\"").unwrap();
        assert_eq!(&"título = \"año\""[a[0].clone()], "año");
        assert_eq!(&"título = \"años\""[b[0].clone()], "años");
    }
}
