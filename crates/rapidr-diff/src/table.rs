//! CSV files compared as tables (docs/versions-plan.md §2: "a CSV opens as
//! a table diff"): which columns and rows were added or removed, and which
//! cells of a row changed.
//!
//! - **Reading** ([`parse_csv`]): fields separated by commas (or by
//!   semicolons or tabs, when the first line has more of those), records
//!   by line breaks; a field in double quotes may hold separators and line
//!   breaks, `""` being a quote.
//! - **Columns** are matched by name (the first row, when the file has a
//!   header) or by position (no header: "Column 1", "Column 2", …).
//! - **Rows**: when the first shared column's values are filled in and
//!   unique on both sides, it is the **key**: rows are matched by it, so a
//!   row whose values changed is one changed row. Otherwise rows are
//!   matched by their whole content, and a removed row next to an added
//!   one that shares at least half its cells is shown as one changed row.
//!   The order is the new file's; a removed row comes where it was.

use std::collections::HashSet;

use crate::text::Change;

/// Whether a column, a row or a cell is the same, new, gone or different.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Same,
    Added,
    Removed,
    Changed,
}

/// One column of the compared table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub state: State,
    /// Its index in the old and in the new file.
    pub old: Option<usize>,
    pub new: Option<usize>,
}

/// One cell: its old and new value (`None` where its row or column isn't
/// on that side).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub old: Option<String>,
    pub new: Option<String>,
}

impl Cell {
    /// Both sides have it and it differs.
    pub fn changed(&self) -> bool {
        matches!((&self.old, &self.new), (Some(a), Some(b)) if a != b)
    }
}

/// One row of the compared table: its state, its index among the data
/// rows of each file (from 0, the header not counted), and one cell per
/// [`TableDiff::columns`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub state: State,
    pub old: Option<usize>,
    pub new: Option<usize>,
    pub cells: Vec<Cell>,
}

/// Two CSV files compared.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableDiff {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    /// The column rows were matched by (`None`: by their content).
    pub key: Option<String>,
}

impl TableDiff {
    /// Rows added, removed and changed.
    pub fn counts(&self) -> (usize, usize, usize) {
        let n = |s| self.rows.iter().filter(|r| r.state == s).count();
        (n(State::Added), n(State::Removed), n(State::Changed))
    }

    /// Whether anything differs.
    pub fn is_same(&self) -> bool {
        self.columns.iter().all(|c| c.state == State::Same) && self.rows.iter().all(|r| r.state == State::Same)
    }

    /// What changed, in plain words, one line each: the columns, then each
    /// row that isn't the same ("Row 3 (Id 17): Score 10 → 12").
    pub fn summary(&self) -> Vec<String> {
        let mut out = Vec::new();
        let names = |s: State| self.columns.iter().filter(|c| c.state == s).map(|c| c.name.as_str()).collect::<Vec<_>>().join(", ");
        let (added, removed) = (names(State::Added), names(State::Removed));
        if !added.is_empty() {
            out.push(format!("Added column{}: {added}", if added.contains(", ") { "s" } else { "" }));
        }
        if !removed.is_empty() {
            out.push(format!("Removed column{}: {removed}", if removed.contains(", ") { "s" } else { "" }));
        }
        let key_col = self.key.as_ref().and_then(|k| self.columns.iter().position(|c| &c.name == k));
        for r in &self.rows {
            let label = |n: Option<usize>, side_new: bool| {
                let mut s = format!("row {}", n.map_or(0, |n| n + 1));
                if let Some(c) = key_col {
                    let cell = &r.cells[c];
                    if let Some(v) = if side_new { &cell.new } else { &cell.old } {
                        s.push_str(&format!(" ({} {v})", self.columns[c].name));
                    }
                }
                s
            };
            match r.state {
                State::Same => {}
                State::Added => out.push(format!("Added {}", label(r.new, true))),
                State::Removed => out.push(format!("Removed {}", label(r.old, false))),
                State::Changed => {
                    let what: Vec<String> = r
                        .cells
                        .iter()
                        .zip(&self.columns)
                        .filter(|(c, _)| c.changed())
                        .map(|(c, col)| format!("{} {} \u{2192} {}", col.name, shown(c.old.as_deref()), shown(c.new.as_deref())))
                        .collect();
                    let mut l = label(r.new, true);
                    if let Some(f) = l.get_mut(0..1) {
                        f.make_ascii_uppercase();
                    }
                    out.push(format!("{l}: {}", what.join("; ")));
                }
            }
        }
        out
    }
}

/// A value as the summary shows it: as is, `""` when empty.
fn shown(v: Option<&str>) -> String {
    match v {
        Some("") | None => "\"\"".into(),
        Some(s) => s.to_string(),
    }
}

/// The separator a CSV text uses: of comma, semicolon and tab, the one its
/// first line has most of (outside quotes); a comma when none.
pub fn separator(text: &str) -> char {
    let mut counts = [0usize; 3];
    let mut quoted = false;
    for c in text.chars() {
        match c {
            '"' => quoted = !quoted,
            '\n' if !quoted => break,
            ',' if !quoted => counts[0] += 1,
            ';' if !quoted => counts[1] += 1,
            '\t' if !quoted => counts[2] += 1,
            _ => {}
        }
    }
    let best = (0..3).max_by_key(|&i| (counts[i], usize::from(i == 0))).unwrap_or(0);
    if counts[best] == 0 {
        ','
    } else {
        [',', ';', '\t'][best]
    }
}

/// The records of a CSV text, each a list of fields (see the module's
/// notes). A last empty line is no record.
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let sep = separator(text);
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    // (whether the current record has anything: a field or a separator)
    let mut started = false;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => {
                quoted = true;
                started = true;
            }
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' | '\r' => {
                if started || !field.is_empty() {
                    row.push(std::mem::take(&mut field));
                }
                rows.push(std::mem::take(&mut row));
                started = false;
            }
            c if c == sep => {
                row.push(std::mem::take(&mut field));
                started = true;
            }
            c => {
                field.push(c);
                started = true;
            }
        }
    }
    if started || !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// Compares two CSV texts as tables; `header`: their first row names the
/// columns.
pub fn diff(old: &str, new: &str, header: bool) -> TableDiff {
    diff_records(&parse_csv(old), &parse_csv(new), header)
}

/// Compares two tables' records (see [`diff`]).
pub fn diff_records(old: &[Vec<String>], new: &[Vec<String>], header: bool) -> TableDiff {
    let width = |t: &[Vec<String>]| t.iter().map(Vec::len).max().unwrap_or(0);
    let names = |t: &[Vec<String>]| -> Vec<String> {
        if header {
            let mut h: Vec<String> = t.first().cloned().unwrap_or_default();
            for i in h.len()..width(t) {
                h.push(format!("Column {}", i + 1));
            }
            h
        } else {
            (1..=width(t)).map(|i| format!("Column {i}")).collect()
        }
    };
    let skip = usize::from(header);
    let (old_rows, new_rows) = (old.get(skip..).unwrap_or_default(), new.get(skip..).unwrap_or_default());
    let (old_names, new_names) = (names(old), names(new));

    // the columns, matched by name
    let mut columns = Vec::new();
    let (mut i, mut j) = (0, 0);
    for ch in crate::text::changes(&old_names, &new_names).into_iter().chain([Change { old: old_names.len()..old_names.len(), new: new_names.len()..new_names.len() }]) {
        while i < ch.old.start {
            columns.push(Column { name: new_names[j].clone(), state: State::Same, old: Some(i), new: Some(j) });
            i += 1;
            j += 1;
        }
        // (a removed column where it was, then the new ones)
        columns.extend(ch.old.clone().map(|i| Column { name: old_names[i].clone(), state: State::Removed, old: Some(i), new: None }));
        columns.extend(ch.new.clone().map(|j| Column { name: new_names[j].clone(), state: State::Added, old: None, new: Some(j) }));
        i = ch.old.end;
        j = ch.new.end;
    }
    let shared: Vec<(usize, usize)> = columns.iter().filter_map(|c| Some((c.old?, c.new?))).collect();
    let cell = |row: &[String], i: Option<usize>| -> Option<String> { i.map(|i| row.get(i).cloned().unwrap_or_default()) };
    let make = |state: State, o: Option<usize>, n: Option<usize>| -> Row {
        let cells = columns
            .iter()
            .map(|c| Cell {
                old: o.and_then(|o| cell(&old_rows[o], c.old)),
                new: n.and_then(|n| cell(&new_rows[n], c.new)),
            })
            .collect();
        Row { state, old: o, new: n, cells }
    };
    let same_cells = |o: usize, n: usize| shared.iter().filter(|&&(a, b)| cell(&old_rows[o], Some(a)) == cell(&new_rows[n], Some(b))).count();

    // the key: the first shared column, when filled in and unique
    let unique = |rows: &[Vec<String>], c: usize| {
        let mut seen = HashSet::new();
        rows.iter().all(|r| r.get(c).is_some_and(|v| !v.trim().is_empty() && seen.insert(v.as_str())))
    };
    let key = shared.first().filter(|&&(a, b)| header && !old_rows.is_empty() && !new_rows.is_empty() && unique(old_rows, a) && unique(new_rows, b)).copied();
    let ident = |rows: &[Vec<String>], side: usize| -> Vec<String> {
        rows.iter()
            .map(|r| match key {
                Some(k) => r.get(if side == 0 { k.0 } else { k.1 }).cloned().unwrap_or_default(),
                // (the shared columns' values, one string)
                None => shared.iter().map(|&(a, b)| r.get(if side == 0 { a } else { b }).map_or("", String::as_str)).collect::<Vec<_>>().join("\u{1F}"),
            })
            .collect()
    };
    let (ia, ib) = (ident(old_rows, 0), ident(new_rows, 1));
    let mut rows = Vec::with_capacity(new_rows.len());
    let (mut i, mut j) = (0, 0);
    let end = Change { old: old_rows.len()..old_rows.len(), new: new_rows.len()..new_rows.len() };
    for ch in crate::text::changes(&ia, &ib).into_iter().chain([end]) {
        while i < ch.old.start {
            // (matched by key: changed when a shared cell differs)
            let state = if same_cells(i, j) == shared.len() { State::Same } else { State::Changed };
            rows.push(make(state, Some(i), Some(j)));
            i += 1;
            j += 1;
        }
        let (mut o, mut n) = (ch.old.start, ch.new.start);
        if key.is_none() {
            // a removed row and an added one alike: one changed row
            while o < ch.old.end && n < ch.new.end && !shared.is_empty() && same_cells(o, n) * 2 >= shared.len() {
                rows.push(make(State::Changed, Some(o), Some(n)));
                o += 1;
                n += 1;
            }
        }
        rows.extend((o..ch.old.end).map(|o| make(State::Removed, Some(o), None)));
        rows.extend((n..ch.new.end).map(|n| make(State::Added, None, Some(n))));
        i = ch.old.end;
        j = ch.new.end;
    }
    TableDiff { key: key.map(|(_, b)| new_names[b].clone()), columns, rows }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_csv() {
        assert_eq!(parse_csv("a,b\n1,2\n"), [vec!["a", "b"], vec!["1", "2"]]);
        assert_eq!(parse_csv("a,b\r\n1,\"x, \"\"y\"\"\nz\"\r\n"), [vec!["a", "b"], vec!["1", "x, \"y\"\nz"]]);
        assert_eq!(parse_csv("a;b;c\n1;2;3"), [vec!["a", "b", "c"], vec!["1", "2", "3"]]);
        assert_eq!(parse_csv("a\tb\n1\t2"), [vec!["a", "b"], vec!["1", "2"]]);
        assert_eq!(parse_csv(""), Vec::<Vec<String>>::new());
        assert_eq!(parse_csv("x,\n"), [vec!["x", ""]]);
        assert_eq!(parse_csv("\"\"\n"), [vec![""]]);
    }

    #[test]
    fn rows_by_key() {
        let old = "Id,Name,Score\n1,Ana,10\n2,Ben,7\n3,Cy,5\n";
        let new = "Id,Name,Score\n1,Ana,12\n3,Cy,5\n4,Dee,9\n";
        let d = diff(old, new, true);
        assert_eq!(d.key.as_deref(), Some("Id"));
        let states: Vec<State> = d.rows.iter().map(|r| r.state).collect();
        assert_eq!(states, [State::Changed, State::Removed, State::Same, State::Added]);
        assert!(d.rows[0].cells[2].changed() && !d.rows[0].cells[1].changed());
        assert_eq!(d.counts(), (1, 1, 1));
        assert_eq!(d.summary(), ["Row 1 (Id 1): Score 10 \u{2192} 12", "Removed row 2 (Id 2)", "Added row 3 (Id 4)"]);
    }

    #[test]
    fn columns_by_name() {
        let old = "Name,Fax,Score\nAna,555,10\nBen,556,7\n";
        let new = "Name,Score,Email\nAna,10,a@x.org\nBen,8,b@x.org\n";
        let d = diff(old, new, true);
        let cols: Vec<(&str, State)> = d.columns.iter().map(|c| (c.name.as_str(), c.state)).collect();
        assert_eq!(cols, [("Name", State::Same), ("Fax", State::Removed), ("Score", State::Same), ("Email", State::Added)]);
        // (Name is the key: Ben's score changed; new cells aren't "changed")
        assert_eq!(d.rows.iter().map(|r| r.state).collect::<Vec<_>>(), [State::Same, State::Changed]);
        assert_eq!(d.rows[1].cells[3], Cell { old: None, new: Some("b@x.org".into()) });
        assert_eq!(d.summary(), ["Added column: Email", "Removed column: Fax", "Row 2 (Name Ben): Score 7 \u{2192} 8"]);
    }

    #[test]
    fn rows_by_content_without_a_key() {
        // (no unique first column: matched by content, alike rows paired)
        let old = "City,Temp\nOslo,3\nOslo,4\nRome,18\n";
        let new = "City,Temp\nOslo,3\nOslo,5\nRome,18\nLima,20\n";
        let d = diff(old, new, true);
        assert_eq!(d.key, None);
        assert_eq!(d.rows.iter().map(|r| r.state).collect::<Vec<_>>(), [State::Same, State::Changed, State::Same, State::Added]);
        assert_eq!(d.summary(), ["Row 2: Temp 4 \u{2192} 5", "Added row 4"]);
        // no header: columns by position
        let d = diff("1,2\n3,4\n", "1,2\n3,5\n", false);
        assert_eq!(d.columns[1].name, "Column 2");
        assert_eq!(d.rows[1].state, State::Changed);
        assert!(diff("a,b\n1,2\n", "a,b\n1,2\n", true).is_same());
    }
}
