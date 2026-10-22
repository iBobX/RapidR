//! RDIFFVIEW's table view (Kind = "table", docs/versions-plan.md §2: "a
//! CSV opens as a table diff"): two CSV texts compared as tables
//! (`rapidr_diff::table`) and shown as one table — the new file's rows in
//! order, a removed row where it was, an added row tinted, a changed row
//! as its old values over its new ones with the changed cells marked, an
//! added or removed column tinted from its header down.

use rapidr_diff::table::{self, State, TableDiff};

/// The header row's height; rows are [`super::ROW_H`].
pub const HEAD_H: i64 = 26;
/// A cell's padding left and right, and the bounds of a column's width.
pub const CELL_PAD: i64 = 8;
const MIN_COL: i64 = 48;
const MAX_COL: i64 = 280;

/// Which values a row of the view shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// An unchanged row (either side's values: they are the same).
    Same,
    /// A removed row, or a changed row's old values.
    Old,
    /// An added row, or a changed row's new values.
    New,
}

/// The two tables compared, laid out.
#[derive(Clone, Debug, Default)]
pub struct TableCompare {
    pub diff: TableDiff,
    /// Each column's width (logical pixels).
    pub widths: Vec<i64>,
    /// The row numbers' and signs' gutter, left of the cells.
    pub gutter: i64,
    /// The view's rows: the diff's row and which of its values.
    pub rows: Vec<(usize, Side)>,
}

impl TableCompare {
    /// Compares two CSV texts (`header`: their first row names the
    /// columns); `adv`: the code font's advance (cells are in it).
    pub fn new(old: &str, new: &str, header: bool, adv: f64) -> TableCompare {
        let diff = table::diff(old, new, header);
        let mut rows = Vec::with_capacity(diff.rows.len());
        for (i, r) in diff.rows.iter().enumerate() {
            match r.state {
                State::Same => rows.push((i, Side::Same)),
                State::Removed => rows.push((i, Side::Old)),
                State::Added => rows.push((i, Side::New)),
                State::Changed => {
                    rows.push((i, Side::Old));
                    rows.push((i, Side::New));
                }
            }
        }
        let chars = |s: &str| s.chars().count() as i64;
        let widths = diff
            .columns
            .iter()
            .enumerate()
            .map(|(c, col)| {
                // (the name with its sign, and every value shown)
                let longest = diff.rows.iter().flat_map(|r| [r.cells[c].old.as_deref(), r.cells[c].new.as_deref()]).flatten().map(chars).chain([chars(&col.name) + 2]).max().unwrap_or(1);
                ((longest as f64 * adv).ceil() as i64 + 2 * CELL_PAD).clamp(MIN_COL, MAX_COL)
            })
            .collect();
        let numbers = diff.rows.len().max(1).to_string().len().max(2) as i64;
        let gutter = (numbers as f64 * adv).ceil() as i64 + 30;
        TableCompare { diff, widths, gutter, rows }
    }

    /// The table's full width and height (content).
    pub fn size(&self) -> (i64, i64) {
        (self.gutter + self.widths.iter().sum::<i64>(), HEAD_H + self.rows.len() as i64 * super::ROW_H)
    }

    /// The value view row `r` shows in column `c`.
    pub fn value(&self, r: usize, c: usize) -> Option<&str> {
        let (i, side) = *self.rows.get(r)?;
        let cell = self.diff.rows.get(i)?.cells.get(c)?;
        match side {
            Side::Old => cell.old.as_deref(),
            Side::New => cell.new.as_deref(),
            Side::Same => cell.new.as_deref().or(cell.old.as_deref()),
        }
    }

    /// The row number shown for view row `r` (from 1: the new file's, an
    /// old row's in the old file).
    pub fn number(&self, r: usize) -> Option<usize> {
        let (i, side) = *self.rows.get(r)?;
        let row = self.diff.rows.get(i)?;
        match side {
            Side::Old => row.old.map(|n| n + 1),
            _ => row.new.map(|n| n + 1),
        }
    }

    /// How many changes: rows added, removed or changed, columns added or
    /// removed.
    pub fn change_count(&self) -> usize {
        let (a, r, c) = self.diff.counts();
        a + r + c + self.diff.columns.iter().filter(|c| c.state != State::Same).count()
    }
}
