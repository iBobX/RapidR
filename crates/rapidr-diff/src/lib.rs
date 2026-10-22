//! RapidR's own diff engine (docs/versions-plan.md §2 and §4, lane G2):
//! one engine for Versions (`rapidr-vcs`), RDIFFVIEW and RAI's review.
//!
//! - [`myers`]: Myers' O(ND) algorithm, written from the 1986 paper
//!   (docs/licensing.md §8, item 6: never from another implementation).
//! - [`text`]: line diffs as runs of changed lines ([`text::changes`]) or
//!   unified hunks with context ([`text::hunks`], the same `@@` numbering
//!   as `git diff`), and the changed words of a changed line
//!   ([`text::word_marks`]).
//! - [`table`]: CSV files compared as tables: columns, rows and cells.
//! - [`form`]: forms compared as components and properties ("GreetButton:
//!   Caption "&Greet" → "&Say hello"; Left 112 → 120; added Label3").
//!
//! GUI-free, no dependencies, builds for wasm32.

pub mod form;
pub mod myers;
pub mod table;
pub mod text;

use std::collections::HashMap;
use std::hash::Hash;

/// Gives equal items the same id: `old`'s and `new`'s items as ids.
pub fn intern<T: Hash + Eq>(old: &[T], new: &[T]) -> (Vec<u32>, Vec<u32>) {
    let mut ids: HashMap<&T, u32> = HashMap::with_capacity(old.len() + new.len());
    let mut id = |t| {
        let n = ids.len() as u32;
        *ids.entry(t).or_insert(n)
    };
    let a = old.iter().map(&mut id).collect();
    let b = new.iter().map(&mut id).collect();
    (a, b)
}
