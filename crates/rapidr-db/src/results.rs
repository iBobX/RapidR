//! A query's rows as the program walks them: FetchRow moves to the next
//! row (before the first one there is none), Row(i) reads its column `i`
//! (from 0), RowSeek(n) makes row `n` the next FetchRow's, FetchField and
//! FieldSeek count through the current row's fields.

/// The rows of a component's last query that returned rows, as text (a
/// NULL is ""), and where the program is in them.
#[derive(Debug, Default)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// The current row (from 0); -1 before the first FetchRow.
    row: i64,
    field: i64,
}

impl ResultSet {
    pub fn new(columns: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        ResultSet { columns, rows, row: -1, field: 0 }
    }

    /// The current row, if the cursor is on one.
    pub fn current(&self) -> Option<&Vec<String>> {
        usize::try_from(self.row).ok().and_then(|r| self.rows.get(r))
    }

    /// FetchRow: whether there was a next row.
    pub fn fetch_row(&mut self) -> bool {
        self.row += 1;
        self.field = 0;
        self.current().is_some()
    }

    /// FetchField: whether the current row had a next field.
    pub fn fetch_field(&mut self) -> bool {
        let fields = self.current().map_or(0, |r| r.len() as i64);
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
        let row = self.current();
        usize::try_from(column).ok().and_then(|c| row?.get(c)).cloned().unwrap_or_default()
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
