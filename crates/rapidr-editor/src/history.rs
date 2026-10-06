//! The undo tree. Undo and redo walk one branch, as in every editor; an edit
//! made after undoing starts a new branch and the old one is kept (reachable
//! with [`History::path_to`], for a history panel).
//!
//! A revision is a list of steps (transactions applied one after the other):
//! typing joins the current revision while it goes on — the same kind of
//! edit, the caret where the last step left it, less than
//! [`GROUP_PAUSE_MS`] since, and no new word started — so undo takes back a
//! word or a pause's worth. A command or an `ApplyEdits` is one revision.

use crate::selection::Selections;
use crate::transaction::ChangeSet;

/// Typing pauses longer than this start a new undo step.
pub const GROUP_PAUSE_MS: u64 = 400;

/// What made an edit (for grouping).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditKind {
    /// Characters typed.
    Typing,
    /// Backspace.
    DeleteBack,
    /// Delete.
    DeleteForward,
    /// Anything else: its own undo step.
    Command,
}

/// One transaction as applied, with its inverse.
#[derive(Clone, Debug)]
pub struct Step {
    pub changes: ChangeSet,
    pub inverse: ChangeSet,
}

/// A node of the tree.
#[derive(Clone, Debug)]
pub struct Revision {
    /// The revision this one was made on (the root is its own parent).
    pub parent: usize,
    /// The child redo goes to (the most recently made or visited).
    pub last_child: Option<usize>,
    pub steps: Vec<Step>,
    pub selections_before: Selections,
    pub selections_after: Selections,
    pub kind: EditKind,
    /// When the last step was made (the caller's clock, ms).
    pub time_ms: u64,
}

#[derive(Clone, Debug)]
pub struct History {
    revisions: Vec<Revision>,
    current: usize,
    /// No further step may join the current revision.
    sealed: bool,
}

impl Default for History {
    fn default() -> Self {
        History::new()
    }
}

/// What the document must do to move through the history: undo these
/// revisions (their steps' inverses, last step first), then redo those (their
/// steps in order).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Path {
    pub undo: Vec<usize>,
    pub redo: Vec<usize>,
}

impl History {
    pub fn new() -> Self {
        let root = Revision {
            parent: 0,
            last_child: None,
            steps: Vec::new(),
            selections_before: Selections::default(),
            selections_after: Selections::default(),
            kind: EditKind::Command,
            time_ms: 0,
        };
        History { revisions: vec![root], current: 0, sealed: true }
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn revision(&self, i: usize) -> &Revision {
        &self.revisions[i]
    }

    pub fn len(&self) -> usize {
        self.revisions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.revisions.len() == 1
    }

    pub fn can_undo(&self) -> bool {
        self.current != 0
    }

    pub fn can_redo(&self) -> bool {
        self.revisions[self.current].last_child.is_some()
    }

    /// The next edit starts a new undo step (the caret moved, a command ran).
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Whether a step of `kind` made at `now_ms` from `before` would join the
    /// current revision (`new_word`: it starts a word or a line).
    pub fn would_join(&self, kind: EditKind, now_ms: u64, before: &Selections, new_word: bool) -> bool {
        let cur = &self.revisions[self.current];
        !self.sealed
            && self.current != 0
            && kind != EditKind::Command
            && cur.kind == kind
            && cur.last_child.is_none()
            && now_ms.saturating_sub(cur.time_ms) < GROUP_PAUSE_MS
            && cur.selections_after == *before
            && !new_word
    }

    /// Records a step (already applied to the text).
    #[allow(clippy::too_many_arguments)]
    pub fn commit(&mut self, step: Step, before: Selections, after: Selections, kind: EditKind, now_ms: u64, new_word: bool) {
        if self.would_join(kind, now_ms, &before, new_word) {
            let cur = &mut self.revisions[self.current];
            cur.steps.push(step);
            cur.selections_after = after;
            cur.time_ms = now_ms;
        } else {
            let id = self.revisions.len();
            self.revisions.push(Revision {
                parent: self.current,
                last_child: None,
                steps: vec![step],
                selections_before: before,
                selections_after: after,
                kind,
                time_ms: now_ms,
            });
            self.revisions[self.current].last_child = Some(id);
            self.current = id;
        }
        self.sealed = kind == EditKind::Command;
    }

    /// Moves to the parent (the caller undoes `revision(old current)`).
    pub fn step_back(&mut self) -> Option<usize> {
        if self.current == 0 {
            return None;
        }
        let rev = self.current;
        let parent = self.revisions[rev].parent;
        self.revisions[parent].last_child = Some(rev);
        self.current = parent;
        self.sealed = true;
        Some(rev)
    }

    /// Moves to the child redo goes to (the caller redoes it).
    pub fn step_forward(&mut self) -> Option<usize> {
        let child = self.revisions[self.current].last_child?;
        self.current = child;
        self.sealed = true;
        Some(child)
    }

    /// The revisions to undo and redo to get from the current revision to
    /// `target` (any revision, on any branch).
    pub fn path_to(&self, target: usize) -> Path {
        let ancestors = |mut r: usize| {
            let mut v = vec![r];
            while r != 0 {
                r = self.revisions[r].parent;
                v.push(r);
            }
            v
        };
        let from = ancestors(self.current);
        let to = ancestors(target);
        let common = *from.iter().find(|r| to.contains(r)).unwrap_or(&0);
        let undo = from.iter().take_while(|&&r| r != common).copied().collect();
        let mut redo: Vec<usize> = to.iter().take_while(|&&r| r != common).copied().collect();
        redo.reverse();
        Path { undo, redo }
    }

    /// A revision's steps, lent out (put them back with
    /// [`History::put_steps`]).
    pub(crate) fn take_steps(&mut self, rev: usize) -> Vec<Step> {
        std::mem::take(&mut self.revisions[rev].steps)
    }

    pub(crate) fn put_steps(&mut self, rev: usize, steps: Vec<Step>) {
        self.revisions[rev].steps = steps;
    }

    /// Makes `target` current after the caller has walked [`History::path_to`].
    pub fn set_current(&mut self, target: usize) {
        // redo follows the walked branch from now on
        let mut r = target;
        while r != 0 {
            let p = self.revisions[r].parent;
            self.revisions[p].last_child = Some(r);
            r = p;
        }
        self.current = target;
        self.sealed = true;
    }
}
