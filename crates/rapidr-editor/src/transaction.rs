//! Edits: a [`Change`] replaces one byte range; a [`ChangeSet`] is several
//! at once (one per caret, or an `ApplyEdits` list), sorted and never
//! overlapping, all in the coordinates of the text *before* the set; a
//! [`Transaction`] is a change set with the selections before and after it,
//! the unit of undo.

use std::ops::Range;

use crate::buffer::Buffer;
use crate::selection::Selections;

/// Replace `range` (bytes, in the text before the change set) with `insert`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Change {
    pub range: Range<usize>,
    pub insert: String,
}

impl Change {
    pub fn new(range: Range<usize>, insert: impl Into<String>) -> Self {
        Change { range, insert: insert.into() }
    }

    pub fn insert(at: usize, text: impl Into<String>) -> Self {
        Change { range: at..at, insert: text.into() }
    }

    pub fn delete(range: Range<usize>) -> Self {
        Change { range, insert: String::new() }
    }
}

/// Why an edit was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// Two changes overlap (their indexes in the list as given).
    Overlap(usize, usize),
    /// A range is reversed, past the text's end or not on a character
    /// boundary.
    BadRange(Range<usize>),
    /// The editor is read-only.
    ReadOnly,
    /// An `ApplyEdits` / JSON list could not be read.
    Parse(String),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditError::Overlap(a, b) => write!(f, "edits {a} and {b} overlap"),
            EditError::BadRange(r) => write!(f, "bad range {}..{}", r.start, r.end),
            EditError::ReadOnly => write!(f, "the text is read-only"),
            EditError::Parse(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for EditError {}

/// Which way a position at an insertion point goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assoc {
    /// Stays before inserted text.
    Before,
    /// Moves past inserted text.
    After,
}

/// Changes applied together: sorted by start, none overlapping (two
/// insertions at one point are allowed and keep their order).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChangeSet {
    changes: Vec<Change>,
}

impl ChangeSet {
    /// Sorts `changes` (stably) and checks them against a text of
    /// `len_bytes` bytes. Ranges must be on character boundaries; that is
    /// checked by [`ChangeSet::validate`] against a buffer.
    pub fn new(changes: Vec<Change>, len_bytes: usize) -> Result<Self, EditError> {
        let mut indexed: Vec<(usize, Change)> = changes.into_iter().enumerate().collect();
        indexed.sort_by_key(|(_, c)| (c.range.start, c.range.end));
        for (_, c) in &indexed {
            if c.range.start > c.range.end || c.range.end > len_bytes {
                return Err(EditError::BadRange(c.range.clone()));
            }
        }
        for w in indexed.windows(2) {
            let (a, b) = (&w[0].1, &w[1].1);
            // (an insertion may sit at a replacement's end or start)
            if b.range.start < a.range.end || (a.range == b.range && !a.range.is_empty()) {
                return Err(EditError::Overlap(w[0].0.min(w[1].0), w[0].0.max(w[1].0)));
            }
        }
        Ok(ChangeSet { changes: indexed.into_iter().map(|(_, c)| c).collect() })
    }

    /// Checks every range lies on character boundaries of `buffer`.
    pub fn validate(&self, buffer: &dyn Buffer) -> Result<(), EditError> {
        for c in &self.changes {
            if c.range.end > buffer.len_bytes() || !buffer.is_char_boundary(c.range.start) || !buffer.is_char_boundary(c.range.end) {
                return Err(EditError::BadRange(c.range.clone()));
            }
        }
        Ok(())
    }

    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    pub fn is_empty(&self) -> bool {
        self.changes.iter().all(|c| c.range.is_empty() && c.insert.is_empty())
    }

    /// The text's length change.
    pub fn len_delta(&self) -> isize {
        self.changes.iter().map(|c| c.insert.len() as isize - c.range.len() as isize).sum()
    }

    /// Applies the set to `buffer` (from the last change to the first, so
    /// earlier ranges stay valid), calling `each(range_before, inserted_len)`
    /// for every change in that order; returns the replaced texts in the
    /// set's order.
    pub fn apply_with(&self, buffer: &mut dyn Buffer, mut each: impl FnMut(&dyn Buffer, Range<usize>, usize, Phase)) -> Vec<String> {
        let mut removed = vec![String::new(); self.changes.len()];
        for (i, c) in self.changes.iter().enumerate().rev() {
            removed[i] = buffer.slice(c.range.clone()).into_owned();
            each(buffer, c.range.clone(), c.insert.len(), Phase::Before);
            buffer.replace(c.range.clone(), &c.insert);
            each(buffer, c.range.clone(), c.insert.len(), Phase::After);
        }
        removed
    }

    pub fn apply(&self, buffer: &mut dyn Buffer) -> Vec<String> {
        self.apply_with(buffer, |_, _, _, _| {})
    }

    /// The set that undoes this one, given the texts it replaced (from
    /// [`ChangeSet::apply`]); its ranges are in the text after this set.
    pub fn invert(&self, removed: &[String]) -> ChangeSet {
        let mut delta: isize = 0;
        let mut out = Vec::with_capacity(self.changes.len());
        for (c, old) in self.changes.iter().zip(removed) {
            let start = (c.range.start as isize + delta) as usize;
            out.push(Change { range: start..start + c.insert.len(), insert: old.clone() });
            delta += c.insert.len() as isize - c.range.len() as isize;
        }
        ChangeSet { changes: out }
    }

    /// Where `pos` (before the set) is after it.
    pub fn map_pos(&self, pos: usize, assoc: Assoc) -> usize {
        let mut delta: isize = 0;
        for c in &self.changes {
            let (s, e, n) = (c.range.start, c.range.end, c.insert.len());
            if pos < s {
                break;
            }
            if pos == s && s == e {
                // an insertion at `pos`
                match assoc {
                    Assoc::Before => break,
                    Assoc::After => {
                        delta += n as isize;
                        continue;
                    }
                }
            }
            if pos < e {
                // inside (or at the start of) a replaced range
                let base = (s as isize + delta) as usize;
                return if assoc == Assoc::After && pos > s { base + n } else { base };
            }
            delta += n as isize - (e - s) as isize;
        }
        (pos as isize + delta) as usize
    }
}

/// When [`ChangeSet::apply_with`]'s callback is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Before,
    After,
}

/// A change set and the selections around it: the unit of undo / redo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub changes: ChangeSet,
    pub selections_before: Selections,
    pub selections_after: Selections,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::RopeBuffer;

    #[test]
    fn apply_invert_map() {
        let mut b = RopeBuffer::new("hello world");
        let set = ChangeSet::new(vec![Change::new(6..11, "there"), Change::insert(0, ">> "), Change::delete(4..5)], b.len_bytes()).unwrap();
        let removed = set.apply(&mut b);
        assert_eq!(b.text(), ">> hell there");
        assert_eq!(removed, ["", "o", "world"]);
        assert_eq!(set.map_pos(0, Assoc::Before), 0);
        assert_eq!(set.map_pos(0, Assoc::After), 3);
        assert_eq!(set.map_pos(5, Assoc::Before), 7); // the space
        assert_eq!(set.map_pos(8, Assoc::Before), 8); // inside "world": its start
        assert_eq!(set.map_pos(8, Assoc::After), 13);
        assert_eq!(set.map_pos(11, Assoc::Before), 13);
        let inv = set.invert(&removed);
        inv.apply(&mut b);
        assert_eq!(b.text(), "hello world");
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn overlaps_are_refused() {
        assert_eq!(ChangeSet::new(vec![Change::new(0..3, "x"), Change::new(2..4, "y")], 9), Err(EditError::Overlap(0, 1)));
        assert_eq!(ChangeSet::new(vec![Change::new(0..3, "x"), Change::new(0..3, "y")], 9), Err(EditError::Overlap(0, 1)));
        assert!(ChangeSet::new(vec![Change::insert(3, "x"), Change::new(0..3, "y"), Change::insert(3, "z")], 9).is_ok());
        assert_eq!(ChangeSet::new(vec![Change::new(5..3, "")], 9), Err(EditError::BadRange(5..3)));
        assert_eq!(ChangeSet::new(vec![Change::new(5..10, "")], 9), Err(EditError::BadRange(5..10)));
    }

    #[test]
    fn two_insertions_at_one_point_keep_their_order() {
        let mut b = RopeBuffer::new("ab");
        let set = ChangeSet::new(vec![Change::insert(1, "1"), Change::insert(1, "2")], 2).unwrap();
        set.apply(&mut b);
        assert_eq!(b.text(), "a12b");
        assert_eq!(set.map_pos(1, Assoc::After), 3);
        assert_eq!(set.map_pos(1, Assoc::Before), 1);
    }
}
