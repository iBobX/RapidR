//! A query's rows as the program walks them: FetchRow moves to the next
//! row (before the first one there is none), Row(i) reads its column `i`
//! (from 0), RowSeek(n) makes row `n` the next FetchRow's, FetchField and
//! FieldSeek count through the current row's fields.
//!
//! A cell keeps the bytes the server sent where they aren't its text (a
//! BLOB's binary data): RMYSQL's FetchLengths, RowBlob and SaveBlob read
//! them.
//!
//! A background query's rows (QueryAsync, [`crate::background`]) aren't
//! copied here: they live in the frame under the component's name (what an
//! RDBGrid bound to it draws), and the same methods walk them there — the
//! rows that have arrived so far.

use std::borrow::Cow;

use rapidr_value::datascience::frame;

/// The rows of a component's last query that returned rows, as text (a
/// NULL is ""), and where the program is in them.
#[derive(Debug, Default)]
pub struct ResultSet {
    pub columns: Vec<String>,
    /// Each column's declared type, as the database says it (SQLite's
    /// declared type, "" for an expression; MySQL's column type: INT,
    /// VARCHAR, DECIMAL …): ColType(i).
    pub types: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// The frame that holds the rows instead (a background query's: the
    /// component's key), `columns` and `rows` then empty.
    pub frame: Option<String>,
    /// Per row, per cell: the bytes the server sent, where they aren't the
    /// text's UTF-8 (None, or no entry: they are). Empty for most results.
    pub raw: Vec<Vec<Option<Vec<u8>>>>,
    /// The current row (from 0); -1 before the first FetchRow.
    row: i64,
    field: i64,
}

impl ResultSet {
    pub fn new(columns: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        ResultSet { columns, types: Vec::new(), rows, raw: Vec::new(), frame: None, row: -1, field: 0 }
    }

    /// The same, the columns' declared types given.
    pub fn typed(mut self, types: Vec<String>) -> Self {
        self.types = types;
        self
    }

    /// The same, with each cell's bytes (as [`ResultSet::raw`]).
    pub fn with_raw(columns: Vec<String>, rows: Vec<Vec<String>>, raw: Vec<Vec<Option<Vec<u8>>>>) -> Self {
        ResultSet { raw, ..ResultSet::new(columns, rows) }
    }

    /// Rows that live in frame `key` (a background query's), as they
    /// arrive.
    pub fn framed(key: &str) -> Self {
        ResultSet { frame: Some(key.to_string()), row: -1, ..ResultSet::default() }
    }

    /// How many rows there are (so far, for a background query's).
    pub fn len(&self) -> usize {
        match &self.frame {
            Some(f) => frame::with_frame(f, frame::Frame::height),
            None => self.rows.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The columns' names, as the database gave them (ColName(i)).
    pub fn column_names(&self) -> Vec<String> {
        match &self.frame {
            Some(f) => crate::background::meta(f).map(|(names, _)| names).unwrap_or_else(|| frame::with_frame(f, |f| f.columns.clone())),
            None => self.columns.clone(),
        }
    }

    /// The columns' declared types (ColType(i); "" where there is none).
    pub fn column_types(&self) -> Vec<String> {
        match &self.frame {
            Some(f) => crate::background::meta(f).map(|(_, types)| types).unwrap_or_default(),
            None => self.types.clone(),
        }
    }

    /// Row `r`'s cells as Row(i) gives them (a NULL as "").
    pub fn row_cells(&self, r: usize) -> Option<Vec<String>> {
        match &self.frame {
            Some(f) => frame::with_frame(f, |f| (r < f.height()).then(|| (0..f.width()).map(|c| f.cell(r, c).map(Cow::into_owned).unwrap_or_default()).collect())),
            None => self.rows.get(r).cloned(),
        }
    }

    /// Cell (`r`, `c`) becomes `value` (a bound widget's edit).
    pub fn set_cell(&mut self, r: usize, c: usize, value: &str) {
        match &self.frame {
            Some(f) => frame::with_frame_mut(f, |f| {
                if r < f.height() {
                    f.set_cell(r, c, Some(value));
                }
            }),
            None => {
                if let Some(cell) = self.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                    *cell = value.to_string();
                }
            }
        }
    }

    /// The current row's column `column` as the bytes the server sent
    /// (None: no current row, or no such column).
    pub fn cell_bytes(&self, column: i64) -> Option<Cow<'_, [u8]>> {
        let r = usize::try_from(self.row).ok()?;
        let c = usize::try_from(column).ok()?;
        if let Some(f) = &self.frame {
            return frame::with_frame(f, |f| (r < f.height() && c < f.width()).then(|| Cow::Owned(f.cell(r, c).map(|s| s.into_owned().into_bytes()).unwrap_or_default())));
        }
        let text = self.rows.get(r)?.get(c)?;
        match self.raw.get(r).and_then(|row| row.get(c)) {
            Some(Some(bytes)) => Some(Cow::Borrowed(bytes)),
            _ => Some(Cow::Borrowed(text.as_bytes())),
        }
    }

    /// The current row's cells' lengths in bytes (None before the first
    /// row and past the last).
    pub fn lengths(&self) -> Option<Vec<i64>> {
        let fields = self.fields()?;
        Some((0..fields as i64).map(|c| self.cell_bytes(c).map_or(0, |b| b.len() as i64)).collect())
    }

    /// The current row, if the cursor is on one.
    pub fn current(&self) -> Option<Vec<String>> {
        usize::try_from(self.row).ok().and_then(|r| self.row_cells(r))
    }

    /// How many fields the current row has (None: no current row).
    fn fields(&self) -> Option<usize> {
        let r = usize::try_from(self.row).ok()?;
        match &self.frame {
            Some(f) => frame::with_frame(f, |f| (r < f.height()).then(|| f.width())),
            None => self.rows.get(r).map(Vec::len),
        }
    }

    /// FetchRow: whether there was a next row.
    pub fn fetch_row(&mut self) -> bool {
        self.row += 1;
        self.field = 0;
        self.fields().is_some()
    }

    /// FetchField: whether the current row had a next field.
    pub fn fetch_field(&mut self) -> bool {
        let fields = self.fields().map_or(0, |n| n as i64);
        if (0..fields).contains(&self.field) {
            self.field += 1;
            true
        } else {
            false
        }
    }

    pub fn field_seek(&mut self, field: i64) {
        self.field = field;
    }

    /// RowSeek(n): the next FetchRow fetches row `n` (from 0).
    pub fn row_seek(&mut self, row: i64) {
        self.row = row.saturating_sub(1);
    }

    /// Row(i): column `i` of the current row ("" if there's none).
    pub fn cell(&self, column: i64) -> String {
        let (Ok(r), Ok(c)) = (usize::try_from(self.row), usize::try_from(column)) else { return String::new() };
        match &self.frame {
            Some(f) => frame::with_frame(f, |f| if r < f.height() { f.cell(r, c).map(Cow::into_owned).unwrap_or_default() } else { String::new() }),
            None => self.rows.get(r).and_then(|row| row.get(c)).cloned().unwrap_or_default(),
        }
    }

    /// The row the web's bound widgets show: the current one, or the first
    /// before FetchRow.
    pub fn bound_row(&self) -> usize {
        self.row.max(0) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_rows() -> ResultSet {
        let row = |a: &str, b: &str| vec![a.to_string(), b.to_string()];
        ResultSet::new(vec!["a".into(), "b".into()], vec![row("1", "x"), row("2", "y")])
    }

    #[test]
    fn fetch_row_walks_the_rows() {
        let mut r = two_rows();
        assert_eq!(r.cell(0), "");
        assert!(r.fetch_row());
        assert_eq!((r.cell(0), r.cell(1), r.cell(2), r.cell(-1)), ("1".into(), "x".into(), "".into(), "".into()));
        assert!(r.fetch_row());
        assert_eq!(r.cell(1), "y");
        assert!(!r.fetch_row());
        assert_eq!(r.cell(0), "");
    }

    #[test]
    fn row_seek_names_the_next_row() {
        let mut r = two_rows();
        r.row_seek(1);
        assert!(r.fetch_row());
        assert_eq!(r.cell(0), "2");
        r.row_seek(0);
        assert!(r.fetch_row());
        assert_eq!(r.cell(0), "1");
    }

    #[test]
    fn a_cells_bytes_are_the_servers() {
        let row = |a: &str, b: &str| vec![a.to_string(), b.to_string()];
        let raw = vec![vec![None, Some(vec![0, 0xFF, b'a'])]];
        let mut r = ResultSet::with_raw(vec!["n".into(), "b".into()], vec![row("12", ""), row("é", "x")], raw);
        assert_eq!((r.cell_bytes(0), r.lengths()), (None, None));
        r.fetch_row();
        assert_eq!(r.cell_bytes(0).as_deref(), Some(&b"12"[..]));
        assert_eq!(r.cell_bytes(1).as_deref(), Some(&[0, 0xFF, b'a'][..]));
        assert_eq!((r.cell_bytes(2), r.cell_bytes(-1)), (None, None));
        assert_eq!(r.lengths(), Some(vec![2, 3]));
        r.fetch_row();
        // (the second row has no raw entry: its text's UTF-8)
        assert_eq!(r.lengths(), Some(vec![2, 1]));
        r.fetch_row();
        assert_eq!(r.lengths(), None);
    }

    #[test]
    fn fetch_field_counts_the_fields() {
        let mut r = two_rows();
        assert!(!r.fetch_field());
        r.fetch_row();
        assert!(r.fetch_field());
        assert!(r.fetch_field());
        assert!(!r.fetch_field());
        r.field_seek(1);
        assert!(r.fetch_field());
    }
}
