//! The code editor's view geometry: metrics from the font, the rows the
//! lines make (folds hide lines; word wrap breaks one into several), and a
//! least-recently-used cache of each shown row's parley layout.
//!
//! Every row is one line high, so a scroll position is a row number times
//! the line height: a 1,000,000-line file costs the same per frame as a
//! 100-line one. Without word wrap and folds a row *is* a line (no table);
//! with them, a table of each line's rows and their running sum, spliced
//! for the lines an edit replaced.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;

use parley::{Affinity, Cursor, Layout};
use rapidr_editor::Buffer;
use rapidr_value::objects::codeedit::CodeEditor;
use rapidr_value::objects::font::Font;

use crate::text::{Ink, TextSystem};

/// The font's measures, logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// Pixel size of the code font.
    pub px: f32,
    /// A row's height (whole pixels).
    pub line: i64,
    /// A column's width (the font's "0").
    pub ch: f64,
}

impl Metrics {
    /// The measures of `font` (its parley size `px`).
    pub fn of(ts: &mut TextSystem, font: &Font) -> Metrics {
        let px = font.pixel_size().max(4) as f32;
        let (w, _) = ts.measure("0000000000", font);
        Metrics { px, line: (f64::from(px) * 1.5).round().max(4.0) as i64, ch: f64::from(w) / 10.0 }
    }
}

/// One row: line `line`'s bytes `range` (a wrapped line's piece; a whole
/// line otherwise; `wrap_index` 0 for its first row).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub line: usize,
    pub range: Range<usize>,
    pub wrap_index: usize,
}

/// Where a line breaks when wrapped at `cols` columns (tabs `tab` wide):
/// each row's byte range of the line, at a space where it can (the space
/// ending the row before), else anywhere.
pub fn wrap_line(text: &str, cols: usize, tab: usize) -> Vec<Range<usize>> {
    if cols == 0 || text.is_empty() {
        return std::iter::once(0..text.len()).collect();
    }
    let mut out = Vec::new();
    let mut start = 0;
    let mut col = 0usize;
    let mut last_space: Option<usize> = None;
    let mut i = 0;
    let b = text.as_bytes();
    while i < text.len() {
        let c = text[i..].chars().next().unwrap_or(' ');
        let w = match c {
            '\t' => tab - col % tab,
            c if is_wide(c) => 2,
            _ => 1,
        };
        if col + w > cols && i > start {
            let at = match last_space {
                Some(s) if s > start => s,
                _ => i,
            };
            out.push(start..at);
            start = at;
            // (the columns of what moved to the new row)
            col = display_cols(&text[start..i], tab);
            last_space = None;
            continue;
        }
        col += w;
        if b[i] == b' ' || b[i] == b'\t' {
            last_space = Some(i + 1);
        }
        i += c.len_utf8();
    }
    out.push(start..text.len());
    out
}

/// A wide (two-column) character: CJK, full-width forms, most emoji.
pub fn is_wide(c: char) -> bool {
    matches!(u32::from(c), 0x1100..=0x115F | 0x2E80..=0x303E | 0x3041..=0x33FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xA000..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 | 0x1F300..=0x1F64F | 0x1F900..=0x1F9FF | 0x20000..=0x3FFFD)
}

/// Display columns of `s` from a tab stop (tabs `tab` wide, wide
/// characters 2).
pub fn display_cols(s: &str, tab: usize) -> usize {
    let mut col = 0;
    for c in s.chars() {
        col += match c {
            '\t' => tab - col % tab,
            c if is_wide(c) => 2,
            _ => 1,
        };
    }
    col
}

/// What a row table was built for: the document's generation and version,
/// the wrap columns, the tab width, the hidden lines.
type RowsKey = (u64, u64, usize, usize, Vec<Range<usize>>);

/// The rows the lines make: none stored without wrap or folds.
#[derive(Debug, Default)]
pub struct RowMap {
    /// The document's generation and version it was built for, the wrap
    /// columns (0: no wrap), the tab width, the hidden lines.
    key: Option<RowsKey>,
    /// Each line's row count (0 when hidden), with word wrap or folds.
    counts: Vec<u32>,
    /// Rows before each line (`counts`' running sum; one more entry).
    before: Vec<u64>,
    /// Hidden line ranges (folds), sorted.
    hidden: Vec<Range<usize>>,
    /// Wrap columns (0: no wrap).
    cols: usize,
    tab: usize,
    lines: usize,
}

impl RowMap {
    /// Brings the table up to date with `c` (its line edits drained).
    pub fn sync(&mut self, c: &mut CodeEditor, cols: usize) {
        let hidden = c.hidden_ranges();
        let tab = c.doc.tab_size as usize;
        let lines = c.doc.line_count();
        let edits = std::mem::take(&mut c.line_edits);
        let gen = c.generation;
        let version = c.doc.version();
        let same_shape = self.key.as_ref().is_some_and(|k| k.0 == gen && k.2 == cols && k.3 == tab && k.4 == hidden);
        if same_shape && self.key.as_ref().is_some_and(|k| k.1 == version) {
            return;
        }
        let tabled = cols > 0 || !hidden.is_empty();
        self.cols = cols;
        self.tab = tab;
        self.lines = lines;
        self.hidden = hidden.clone();
        if !tabled {
            self.counts.clear();
            self.before.clear();
        } else if same_shape && !self.counts.is_empty() && cols > 0 {
            // (word wrap: only the lines edits replaced measured again)
            for (first, old, new) in edits {
                let end = (first + old).min(self.counts.len());
                let fresh: Vec<u32> = (first..first + new).map(|l| self.count_of(c, l)).collect();
                if first <= self.counts.len() {
                    self.counts.splice(first..end, fresh);
                }
            }
            if self.counts.len() != lines {
                self.rebuild(c);
            } else {
                self.apply_hidden();
                self.sum();
            }
        } else {
            self.rebuild(c);
        }
        self.key = Some((gen, version, cols, tab, hidden));
    }

    fn count_of(&self, c: &CodeEditor, line: usize) -> u32 {
        if self.cols == 0 {
            return 1;
        }
        let text = c.doc.line(line);
        if text.len() <= self.cols && !text.contains('\t') && text.is_ascii() {
            return 1;
        }
        wrap_line(&text, self.cols, self.tab).len() as u32
    }

    fn rebuild(&mut self, c: &CodeEditor) {
        self.counts = (0..self.lines).map(|l| self.count_of(c, l)).collect();
        self.apply_hidden();
        self.sum();
    }

    fn apply_hidden(&mut self) {
        if self.cols == 0 {
            self.counts.fill(1);
        }
        for r in &self.hidden {
            for l in r.clone() {
                if let Some(n) = self.counts.get_mut(l) {
                    *n = 0;
                }
            }
        }
    }

    /// Re-applies counts after folds changed for a wrapped table (counts of
    /// unhidden lines need measuring again).
    fn sum(&mut self) {
        self.before.clear();
        self.before.reserve(self.counts.len() + 1);
        let mut acc = 0u64;
        for n in &self.counts {
            self.before.push(acc);
            acc += u64::from(*n);
        }
        self.before.push(acc);
    }

    fn tabled(&self) -> bool {
        !self.before.is_empty()
    }

    /// How many rows there are.
    pub fn total(&self) -> usize {
        if self.tabled() {
            *self.before.last().unwrap_or(&0) as usize
        } else {
            self.lines.max(1)
        }
    }

    /// The first row of `line` (a hidden line: the row of the line folding
    /// it).
    pub fn row_of_line(&self, line: usize) -> usize {
        if !self.tabled() {
            return line;
        }
        let line = line.min(self.lines.saturating_sub(1));
        let mut l = line;
        while l > 0 && self.counts.get(l) == Some(&0) {
            l -= 1;
        }
        self.before.get(l).copied().unwrap_or(0) as usize
    }

    /// The line row `row` is on, and which of its rows it is.
    pub fn line_of_row(&self, row: usize) -> (usize, usize) {
        if !self.tabled() {
            return (row.min(self.lines.saturating_sub(1)), 0);
        }
        let row = (row as u64).min(self.before.last().copied().unwrap_or(1).saturating_sub(1));
        // (the last line whose first row is at or before `row`, not hidden)
        let mut l = self.before.partition_point(|&b| b <= row).saturating_sub(1).min(self.lines.saturating_sub(1));
        while l > 0 && self.counts.get(l) == Some(&0) {
            l -= 1;
        }
        (l, (row - self.before[l]) as usize)
    }

    /// Whether 0-based `line` is hidden by a fold.
    pub fn hidden(&self, line: usize) -> bool {
        self.hidden.iter().any(|r| r.contains(&line))
    }

    /// The rows from `first` (at most `n`), each line's pieces.
    pub fn rows(&self, c: &CodeEditor, first: usize, n: usize) -> Vec<Row> {
        let mut out = Vec::with_capacity(n);
        let total = self.total();
        let mut row = first;
        while out.len() < n && row < total {
            let (line, k) = self.line_of_row(row);
            let text = c.doc.line(line);
            let pieces = if self.cols > 0 { wrap_line(&text, self.cols, self.tab) } else { std::iter::once(0..text.len()).collect() };
            for (i, r) in pieces.into_iter().enumerate().skip(k) {
                if out.len() >= n {
                    break;
                }
                out.push(Row { line, range: r, wrap_index: i });
                row += 1;
            }
            if self.tabled() {
                // (past the line's rows: the next shown line's)
                row = row.max(self.before.get(line + 1).copied().unwrap_or(u64::MAX) as usize);
            }
        }
        out
    }

    /// The row and its piece holding byte `at` of the document.
    pub fn row_at(&self, c: &CodeEditor, at: usize) -> (usize, Row) {
        let buf = c.doc.buffer();
        let at = at.min(buf.len_bytes());
        let line = buf.line_of(at);
        let col = at - buf.line_start(line);
        let first = self.row_of_line(line);
        if self.cols == 0 {
            let len = buf.line_text(line).len();
            return (first, Row { line, range: 0..len, wrap_index: 0 });
        }
        let text = buf.line_text(line);
        let pieces = wrap_line(&text, self.cols, self.tab);
        let n = pieces.len();
        for (i, r) in pieces.into_iter().enumerate() {
            // (a byte at a break shows at the start of the next row, but the
            // line's end on its last)
            if col < r.end || i + 1 == n {
                return (first + i, Row { line, range: r, wrap_index: i });
            }
        }
        (first, Row { line, range: 0..text.len(), wrap_index: 0 })
    }
}

/// A row's text drawn: its layout, keyed by what it shows.
pub struct Laid {
    pub key: u64,
    pub layout: Layout<Ink>,
    /// The frame it was last used in.
    pub used: u64,
}

/// The layouts of shown rows, by a hash of their text and style.
#[derive(Default)]
pub struct LayoutCache {
    pub slots: Vec<Laid>,
    index: HashMap<u64, usize>,
    pub frame: u64,
    /// Layouts made this frame (for the performance harness).
    pub made: usize,
}

/// Most layouts kept.
pub const CACHE_SIZE: usize = 512;

impl LayoutCache {
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        self.made = 0;
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.index.clear();
    }

    /// The slot of the layout for `key`, made by `make` when not cached
    /// (replacing the least recently used one not used this frame).
    pub fn get_or_make(&mut self, key: u64, make: impl FnOnce() -> Layout<Ink>) -> usize {
        if let Some(&i) = self.index.get(&key) {
            self.slots[i].used = self.frame;
            return i;
        }
        let layout = make();
        self.made += 1;
        let entry = Laid { key, layout, used: self.frame };
        if self.slots.len() < CACHE_SIZE {
            self.slots.push(entry);
            self.index.insert(key, self.slots.len() - 1);
            return self.slots.len() - 1;
        }
        let victim = (0..self.slots.len()).filter(|&i| self.slots[i].used != self.frame).min_by_key(|&i| self.slots[i].used);
        match victim {
            Some(i) => {
                self.index.remove(&self.slots[i].key);
                self.slots[i] = entry;
                self.index.insert(key, i);
                i
            }
            None => {
                self.slots.push(entry);
                self.index.insert(key, self.slots.len() - 1);
                self.slots.len() - 1
            }
        }
    }

    pub fn layout(&self, slot: usize) -> Option<&Layout<Ink>> {
        self.slots.get(slot).map(|l| &l.layout)
    }
}

/// A hash of anything hashable (layout keys).
pub fn hash_of(parts: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

/// The x (device pixels from the layout's left) of byte `index` of the
/// laid-out row.
pub fn x_of(layout: &Layout<Ink>, index: usize) -> f64 {
    let c = Cursor::from_byte_index(layout, index, Affinity::Downstream);
    c.geometry(layout, 0.0).x0
}

/// The byte of the row nearest to device x `x` (its layout's).
pub fn index_at(layout: &Layout<Ink>, x: f64) -> usize {
    let h = layout.height().max(1.0);
    Cursor::from_point(layout, x as f32, h / 2.0).index()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_breaks_at_spaces() {
        let t = "PRINT hello world again";
        let r = wrap_line(t, 12, 4);
        let pieces: Vec<&str> = r.iter().map(|r| &t[r.clone()]).collect();
        assert_eq!(pieces, ["PRINT hello ", "world again"]);
        let long = "abcdefghijklmnop";
        let pieces: Vec<&str> = wrap_line(long, 5, 4).iter().map(|r| &long[r.clone()]).collect();
        assert_eq!(pieces, ["abcde", "fghij", "klmno", "p"]);
        assert_eq!(display_cols("\tx", 4), 5);
        assert_eq!(display_cols("日本", 4), 4);
    }

    #[test]
    fn rows_with_folds_and_wrap() {
        let mut c = CodeEditor::new();
        c.set_text("SUB A\n  PRINT 1\n  PRINT 2\nEND SUB\nx = 1\n");
        let mut m = RowMap::default();
        m.sync(&mut c, 0);
        assert_eq!(m.total(), 6);
        assert_eq!(m.line_of_row(3), (3, 0));
        c.fold(0);
        m.sync(&mut c, 0);
        let lines: Vec<usize> = m.rows(&c, 0, 10).iter().map(|r| r.line).collect();
        assert_eq!(lines, [0, 4, 5]);
        assert_eq!(m.row_of_line(4), 1);
        assert_eq!(m.row_of_line(2), 0, "a hidden line shows as its fold's");
        c.unfold_all();
        m.sync(&mut c, 4);
        // "SUB A" 2 rows, "  PRINT 1" … each ≥ 3 rows at 4 columns
        assert!(m.total() > 6);
        let (row, piece) = m.row_at(&c, 3);
        assert_eq!((piece.line, piece.wrap_index), (0, 0));
        assert!(row == 0);
    }
}
