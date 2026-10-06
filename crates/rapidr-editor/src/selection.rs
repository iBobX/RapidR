//! Selections: one or more carets, each with an anchor (where the selection
//! started) and a head (where the caret is). They are kept sorted, and
//! merged when they overlap.

use std::ops::Range;

use crate::transaction::{Assoc, ChangeSet};

/// One caret and its selection (empty when `anchor == head`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
    /// The display column vertical moves aim for (kept across short lines).
    pub goal: Option<u32>,
}

impl Selection {
    pub fn new(anchor: usize, head: usize) -> Self {
        Selection { anchor, head, goal: None }
    }

    pub fn caret(at: usize) -> Self {
        Selection::new(at, at)
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn range(&self) -> Range<usize> {
        self.start()..self.end()
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// The head before the anchor.
    pub fn is_backward(&self) -> bool {
        self.head < self.anchor
    }

    /// Through a change set: carets go past inserted text, a selection
    /// keeps its ends outside insertions at its edges.
    pub fn map(&self, changes: &ChangeSet) -> Selection {
        if self.is_empty() {
            let at = changes.map_pos(self.head, Assoc::After);
            return Selection::caret(at);
        }
        let (start, end) = (changes.map_pos(self.start(), Assoc::After), changes.map_pos(self.end(), Assoc::Before));
        let end = end.max(start);
        if self.is_backward() {
            Selection::new(end, start)
        } else {
            Selection::new(start, end)
        }
    }
}

/// The editor's selections: never empty, sorted by start, not overlapping;
/// one is the primary (the one scrolled to, the one IME follows).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Selections {
    ranges: Vec<Selection>,
    primary: usize,
}

impl Default for Selections {
    fn default() -> Self {
        Selections::single(Selection::caret(0))
    }
}

impl Selections {
    pub fn single(sel: Selection) -> Self {
        Selections { ranges: vec![sel], primary: 0 }
    }

    pub fn caret(at: usize) -> Self {
        Selections::single(Selection::caret(at))
    }

    /// Sorts and merges `ranges`; `primary` indexes `ranges` as given. An
    /// empty list becomes one caret at 0.
    pub fn new(ranges: Vec<Selection>, primary: usize) -> Self {
        if ranges.is_empty() {
            return Selections::default();
        }
        let primary = primary.min(ranges.len() - 1);
        let mut indexed: Vec<(bool, Selection)> = ranges.into_iter().enumerate().map(|(i, s)| (i == primary, s)).collect();
        indexed.sort_by_key(|(_, s)| (s.start(), s.end()));
        let mut out: Vec<Selection> = Vec::with_capacity(indexed.len());
        let mut new_primary = 0;
        for (is_primary, s) in indexed {
            if let Some(last) = out.last_mut() {
                let touching = s.start() == last.end() && (s.is_empty() || last.is_empty());
                if s.start() < last.end() || touching {
                    let (start, end) = (last.start(), last.end().max(s.end()));
                    let keep_goal = if is_primary { s.goal } else { last.goal };
                    *last = if last.is_backward() { Selection::new(end, start) } else { Selection::new(start, end) };
                    last.goal = keep_goal;
                    if is_primary {
                        new_primary = out.len() - 1;
                    }
                    continue;
                }
            }
            if is_primary {
                new_primary = out.len();
            }
            out.push(s);
        }
        Selections { ranges: out, primary: new_primary }
    }

    pub fn ranges(&self) -> &[Selection] {
        &self.ranges
    }

    pub fn iter(&self) -> impl Iterator<Item = &Selection> {
        self.ranges.iter()
    }

    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn primary_index(&self) -> usize {
        self.primary
    }

    pub fn primary(&self) -> Selection {
        self.ranges[self.primary]
    }

    /// Every selection through `f`, merged again.
    pub fn transform(&self, mut f: impl FnMut(Selection) -> Selection) -> Selections {
        Selections::new(self.ranges.iter().map(|s| f(*s)).collect(), self.primary)
    }

    /// Through a change set (see [`Selection::map`]).
    pub fn map(&self, changes: &ChangeSet) -> Selections {
        self.transform(|s| s.map(changes))
    }

    /// Just the primary selection.
    pub fn only_primary(&self) -> Selections {
        Selections::single(self.primary())
    }

    /// Adds `sel` and makes it the primary.
    pub fn add(&self, sel: Selection) -> Selections {
        let mut ranges = self.ranges.clone();
        ranges.push(sel);
        let n = ranges.len() - 1;
        Selections::new(ranges, n)
    }

    /// Every position clamped by `clamp` (after the text shrank, say).
    pub fn clamped(&self, clamp: impl Fn(usize) -> usize) -> Selections {
        self.transform(|s| Selection { anchor: clamp(s.anchor), head: clamp(s.head), goal: s.goal })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merging() {
        let s = Selections::new(vec![Selection::new(10, 15), Selection::caret(3), Selection::new(12, 20), Selection::caret(3)], 2);
        assert_eq!(s.ranges(), &[Selection::caret(3), Selection::new(10, 20)]);
        assert_eq!(s.primary_index(), 1);
        // a caret at a selection's end merges; two touching selections don't
        let s = Selections::new(vec![Selection::new(0, 5), Selection::caret(5), Selection::new(5, 8)], 0);
        assert_eq!(s.ranges(), &[Selection::new(0, 5), Selection::new(5, 8)]);
        // the first one's direction wins
        let s = Selections::new(vec![Selection::new(5, 0), Selection::new(3, 9)], 1);
        assert_eq!(s.ranges(), &[Selection::new(9, 0)]);
        assert_eq!(s.primary_index(), 0);
        assert_eq!(Selections::new(vec![], 0), Selections::caret(0));
    }
}
