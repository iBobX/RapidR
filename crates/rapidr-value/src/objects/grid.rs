//! QSTRINGGRID's data (RapidQ manual, Appendix A): a table of strings with
//! fixed (header) rows and columns, per-column widths and per-row heights,
//! the selected cell and the grid options. The runtimes draw it
//! (`with_grid`); the program changes it through the properties and methods
//! below, the same on the desktop and the web.
//!
//! RapidQ's API:
//! * `Cell(col, row)` read / `Cell(col, row) = s` (compiled as the method
//!   `cell` with the value as a last argument), also `Cells`;
//! * `ColCount`, `RowCount` (5 each), `FixedCols`, `FixedRows` (1 each),
//!   `DefaultColWidth` (64), `DefaultRowHeight` (24), `ColWidths(i)`,
//!   `RowHeights(i)`, `Col`, `Row`, `TopRow`, `LeftCol`, `Separator`,
//!   `ColumnStyle(i)`, `ColumnList(i)`, `GridWidth`, `GridHeight`;
//! * `InsertRow`, `DeleteRow`, `InsertCol`, `DeleteCol`, `SwapRows`,
//!   `SwapCols`, `AddOptions`, `DelOptions`, and `SaveToFile` /
//!   `LoadFromFile` / `SaveToStream` / `LoadFromStream` (rows of cells
//!   joined by `Separator`, through [`StringGrid::to_text`] /
//!   [`StringGrid::load_text`]).
//!
//! RapidR's own additions (used by its IDE): `AddRow a, b, …` appends a row
//! (widening the grid if needed), `GetCell(col, row)` / `SetCell(col, row,
//! s)` (the same order as `Cell`), `Clear` (no rows left), `SetRowCount n` / `SetColCount n`, `Cols` /
//! `Rows` / `ColWidth` (= ColCount / RowCount / DefaultColWidth) and
//! `SelectedRow` / `SelectedCol` (= Row / Col).
//!
//! Owner drawing (OnDrawCell): after the grid's content or layout changes,
//! the runtimes fire `OnDrawCell(Col, Row, State, Rect)` for each cell
//! ([`StringGrid::owner_draw_needed`], [`StringGrid::owner_draw_cells`]);
//! the handler's `Line`, `Rectangle`, `FillRect`, `Circle`, `Pset`,
//! `TextOut` and `Draw` on the grid are kept per cell ([`CellDraw`]) and
//! drawn over the cell until the grid changes again. Rect is in the grid's
//! own coordinates (cells from the top left, 1px grid lines), like
//! Delphi's for an unscrolled grid.
//!
//! Indexes out of range read as "" / 0 and are ignored when written, as
//! RapidQ doesn't stop the program for them. Sizes are capped so a program
//! can't make a grid of billions of cells.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use super::bitmap::Bitmap;
use crate::{v_int, v_str, Value};

/// Delphi's TGridDrawState bits, as OnDrawCell's State.
pub const GD_SELECTED: i64 = 1;
pub const GD_FOCUSED: i64 = 2;
pub const GD_FIXED: i64 = 4;

/// Most cells OnDrawCell is fired for after a change.
pub const MAX_OWNER_DRAWN: usize = 20_000;

/// Something OnDrawCell drew on a cell, relative to the cell's top left.
/// Colors are &HBBGGRR; `None` = transparent.
#[derive(Clone, Debug)]
pub enum CellDraw {
    Line(i64, i64, i64, i64, u32),
    Rect(i64, i64, i64, i64, u32),
    Fill(i64, i64, i64, i64, u32),
    Ellipse(i64, i64, i64, i64, u32, Option<u32>),
    Pixel(i64, i64, u32),
    Text(i64, i64, String, u32, Option<u32>),
    Image(i64, i64, Bitmap),
    /// `Paint(x, y, c, borderc)`: a flood fill from (x, y) with c up to the
    /// border colour, on the pixels drawn so far (only a raster can show
    /// it: the runtimes draw a cell or item that has one as pixels).
    Flood(i64, i64, u32, u32),
}

impl CellDraw {
    /// The same drawing `dx`, `dy` pixels away.
    pub fn moved(self, dx: i64, dy: i64) -> CellDraw {
        match self {
            CellDraw::Line(x1, y1, x2, y2, c) => CellDraw::Line(x1 + dx, y1 + dy, x2 + dx, y2 + dy, c),
            CellDraw::Rect(x1, y1, x2, y2, c) => CellDraw::Rect(x1 + dx, y1 + dy, x2 + dx, y2 + dy, c),
            CellDraw::Fill(x1, y1, x2, y2, c) => CellDraw::Fill(x1 + dx, y1 + dy, x2 + dx, y2 + dy, c),
            CellDraw::Ellipse(x1, y1, x2, y2, c, fill) => CellDraw::Ellipse(x1 + dx, y1 + dy, x2 + dx, y2 + dy, c, fill),
            CellDraw::Pixel(x, y, c) => CellDraw::Pixel(x + dx, y + dy, c),
            CellDraw::Text(x, y, text, c, bg) => CellDraw::Text(x + dx, y + dy, text, c, bg),
            CellDraw::Image(x, y, b) => CellDraw::Image(x + dx, y + dy, b),
            CellDraw::Flood(x, y, c, border) => CellDraw::Flood(x + dx, y + dy, c, border),
        }
    }

    /// Paints this onto `bmp` (whose top left is the cell's), text in
    /// `font`: the same pixels on every platform.
    pub fn paint(&self, bmp: &mut Bitmap, font: &super::font::Font) {
        match self {
            CellDraw::Line(x1, y1, x2, y2, c) => bmp.line(*x1, *y1, *x2, *y2, *c),
            CellDraw::Rect(x1, y1, x2, y2, c) => bmp.rectangle(*x1, *y1, *x2, *y2, *c),
            CellDraw::Fill(x1, y1, x2, y2, c) => bmp.fill_rect(*x1, *y1, *x2, *y2, *c),
            CellDraw::Ellipse(x1, y1, x2, y2, c, fill) => {
                if let Some(f) = fill {
                    bmp.ellipse(*x1, *y1, *x2, *y2, *f, true);
                }
                bmp.ellipse(*x1, *y1, *x2, *y2, *c, false);
            }
            CellDraw::Pixel(x, y, c) => bmp.pset(*x, *y, *c),
            CellDraw::Text(x, y, text, c, bg) => super::text::text_out(bmp, *x, *y, text, font, *c, *bg),
            CellDraw::Image(x, y, src) => bmp.draw(*x, *y, src),
            CellDraw::Flood(x, y, c, border) => bmp.flood_fill(*x, *y, *c, *border),
        }
    }
}

/// A drawing call on a grid or a list (RapidQ's methods: `Line`,
/// `Rectangle`, `FillRect`, `Circle`, `Pset`, `TextOut`, and `Paint(x, y,
/// c, borderc)`, its flood fill): the point it's anchored at and the op, in
/// the control's coordinates. `None`: not one of them (`Draw` needs the
/// source image: rapidr_value::objects; `Paint` with fewer arguments is a
/// repaint).
pub fn owner_draw_op(method: &str, args: &[Value]) -> Option<((i64, i64), CellDraw)> {
    let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
    let c = |i: usize| crate::objects::color_bgr(n(i));
    let optional = |i: usize| args.get(i).map(Value::to_i64).filter(|v| *v >= 0 || (*v as u32) & 0xFF00_0000 == 0x8000_0000).map(crate::objects::color_bgr);
    let at = (n(0), n(1));
    Some(match method {
        "line" => (at, CellDraw::Line(n(0), n(1), n(2), n(3), c(4))),
        "rectangle" => (at, CellDraw::Rect(n(0), n(1), n(2), n(3), c(4))),
        "fillrect" => (at, CellDraw::Fill(n(0), n(1), n(2), n(3), c(4))),
        "circle" => (at, CellDraw::Ellipse(n(0), n(1), n(2), n(3), c(4), optional(5))),
        "pset" => (at, CellDraw::Pixel(n(0), n(1), c(2))),
        // TextOut(x, y, text, color, background (-1: transparent)).
        "textout" => (at, CellDraw::Text(n(0), n(1), args.get(2).map(Value::to_string_val).unwrap_or_default(), c(3), optional(4))),
        "paint" if args.len() >= 3 => (at, CellDraw::Flood(n(0), n(1), c(2), c(3))),
        _ => return None,
    })
}

/// Most rows / columns a grid can have, and most cells in all.
pub const MAX_ROWS: usize = 1_000_000;
pub const MAX_COLS: usize = 10_000;
pub const MAX_CELLS: usize = 4_000_000;

/// `AddOptions` numbers (RAPIDQ.INC `goFixedVertLine` = 0 … `goThumbTracking` = 14).
pub const GO_FIXED_VERT_LINE: u32 = 0;
pub const GO_FIXED_HORZ_LINE: u32 = 1;
pub const GO_VERT_LINE: u32 = 2;
pub const GO_HORZ_LINE: u32 = 3;
pub const GO_RANGE_SELECT: u32 = 4;
/// goDrawFocusSelected: the focused cell highlighted like the rest of the selection.
pub const GO_DRAW_FOCUS_SELECTED: u32 = 5;
pub const GO_ROW_SIZING: u32 = 6;
pub const GO_COL_SIZING: u32 = 7;
/// The user drags a fixed column's cell to move its row, a fixed row's cell
/// to move its column ([`StringGrid::move_row`] / [`StringGrid::move_col`]).
pub const GO_ROW_MOVING: u32 = 8;
pub const GO_COL_MOVING: u32 = 9;
pub const GO_EDITING: u32 = 10;
pub const GO_TABS: u32 = 11;
pub const GO_ROW_SELECT: u32 = 12;
pub const GO_ALWAYS_SHOW_EDITOR: u32 = 13;

/// `ColumnStyle` values.
pub const GCS_LIST: i64 = 0;
pub const GCS_ELLIPSIS: i64 = 1;
pub const GCS_NONE: i64 = 2;

/// The items of a drop-down list's text (one per line, empty lines left
/// out): a ColumnList, or what OnListDropDown answered.
pub fn list_lines(text: &str) -> Vec<String> {
    text.split('\n').map(|s| s.trim_end_matches('\r').to_string()).filter(|s| !s.is_empty()).collect()
}

/// A grid's selection: (Col, Row, where a range started).
pub type Selection = (i64, i64, Option<(i64, i64)>);

#[derive(Clone, Debug)]
pub struct StringGrid {
    /// `cells[row][col]`, always `row_count() × col_count()`.
    cells: Vec<Vec<String>>,
    col_count: usize,
    pub col_widths: Vec<i64>,
    pub row_heights: Vec<i64>,
    /// FixedCols / FixedRows as set (in effect: at most all but one).
    want_fixed_cols: usize,
    want_fixed_rows: usize,
    pub default_col_width: i64,
    pub default_row_height: i64,
    /// The selected cell (the focused one of a range).
    pub col: i64,
    pub row: i64,
    /// With goRangeSelect: the cell a range selection started from; the
    /// range is from it to (col, row).
    pub anchor: Option<(i64, i64)>,
    pub top_row: i64,
    pub left_col: i64,
    pub separator: String,
    /// `AddOptions` bits (`1 << goEditing`, …).
    pub options: u32,
    pub column_styles: Vec<i64>,
    pub column_lists: Vec<String>,
    /// RapidR's `SetSuggestions` (one per line).
    pub suggestions: Vec<String>,
    /// What OnDrawCell drew, per cell (col, row).
    pub owner_drawing: HashMap<(usize, usize), Vec<CellDraw>>,
    /// The state OnDrawCell was last fired for ([`Self::owner_draw_needed`]).
    drawn_state: Option<u64>,
    /// The control's inside (width, height), as the runtime shows it, for
    /// VisibleRowCount / VisibleColCount.
    pub view: (i64, i64),
}

impl Default for StringGrid {
    fn default() -> Self {
        let mut g = StringGrid {
            cells: Vec::new(),
            col_count: 0,
            col_widths: Vec::new(),
            row_heights: Vec::new(),
            want_fixed_cols: 1,
            want_fixed_rows: 1,
            default_col_width: 64,
            default_row_height: 24,
            col: 1,
            row: 1,
            anchor: None,
            top_row: 1,
            left_col: 1,
            separator: ",".into(),
            // Delphi's defaults: lines everywhere, range selection.
            options: bit(GO_FIXED_VERT_LINE) | bit(GO_FIXED_HORZ_LINE) | bit(GO_VERT_LINE) | bit(GO_HORZ_LINE) | bit(GO_RANGE_SELECT),
            column_styles: Vec::new(),
            column_lists: Vec::new(),
            suggestions: Vec::new(),
            owner_drawing: HashMap::new(),
            drawn_state: None,
            view: (0, 0),
        };
        g.resize(5, 5);
        g
    }
}

/// Where index `i` is after the item at `from` moved to `to`.
fn moved_index(i: i64, from: usize, to: usize) -> i64 {
    let (f, t) = (from as i64, to as i64);
    if i == f {
        t
    } else if f < t && i > f && i <= t {
        i - 1
    } else if t < f && i >= t && i < f {
        i + 1
    } else {
        i
    }
}

fn bit(option: u32) -> u32 {
    1u32.checked_shl(option).unwrap_or(0)
}

fn index(v: Option<&Value>) -> Option<usize> {
    usize::try_from(v?.to_i64()).ok()
}

fn flag(on: bool) -> Value {
    v_int(if on { -1 } else { 0 })
}

impl StringGrid {
    pub fn row_count(&self) -> usize {
        self.cells.len()
    }

    pub fn col_count(&self) -> usize {
        self.col_count
    }

    /// The text of cell (`col`, `row`), "" outside the grid.
    pub fn cell(&self, col: usize, row: usize) -> &str {
        self.cells.get(row).and_then(|r| r.get(col)).map_or("", |s| s.as_str())
    }

    /// Sets cell (`col`, `row`); ignored outside the grid.
    pub fn set_cell(&mut self, col: usize, row: usize, text: String) {
        if let Some(c) = self.cells.get_mut(row).and_then(|r| r.get_mut(col)) {
            *c = text;
        }
    }

    /// Header rows / columns that don't scroll: FixedRows / FixedCols, but
    /// always leaving one row / column that isn't fixed.
    pub fn fixed_rows(&self) -> usize {
        self.want_fixed_rows.min(self.row_count().saturating_sub(1))
    }

    pub fn fixed_cols(&self) -> usize {
        self.want_fixed_cols.min(self.col_count.saturating_sub(1))
    }

    pub fn has_option(&self, option: u32) -> bool {
        self.options & bit(option) != 0
    }

    /// The user can type into the cells (`goEditing`).
    pub fn editable(&self) -> bool {
        self.has_option(GO_EDITING)
    }

    /// A gcsList column's items (its ColumnList, one per line), for the
    /// drop-down button of its selected cell; `None` for other cells.
    pub fn list_items(&self, col: usize, row: usize) -> Option<Vec<String>> {
        self.list_text(col, row).map(|t| list_lines(&t))
    }

    /// A gcsList column's ColumnList as written (items on lines): what
    /// OnListDropDown(Col, Row, S) gets as `S`; `None` for other cells.
    pub fn list_text(&self, col: usize, row: usize) -> Option<String> {
        let fixed = row < self.fixed_rows() || col < self.fixed_cols();
        if fixed || self.column_style(col) != GCS_LIST {
            return None;
        }
        Some(self.column_lists.get(col).cloned().unwrap_or_default())
    }

    pub fn column_style(&self, col: usize) -> i64 {
        self.column_styles.get(col).copied().unwrap_or(GCS_NONE)
    }

    /// Resizes to `rows × cols` (capped), keeping the cells that still fit.
    pub fn resize(&mut self, rows: usize, cols: usize) {
        let cols = cols.min(MAX_COLS);
        let rows = rows.min(MAX_ROWS).min(MAX_CELLS.checked_div(cols).unwrap_or(MAX_ROWS));
        self.col_count = cols;
        self.cells.resize_with(rows, Vec::new);
        for r in &mut self.cells {
            r.resize(cols, String::new());
        }
        self.col_widths.resize(cols, self.default_col_width);
        self.row_heights.resize(rows, self.default_row_height);
        self.fix_selection();
    }

    /// Keeps the fixed rows/columns and the selection inside the grid.
    fn fix_selection(&mut self) {
        let (rows, cols) = (self.row_count() as i64, self.col_count as i64);
        if self.row >= rows {
            self.row = rows - 1;
        }
        if self.col >= cols {
            self.col = cols - 1;
        }
        self.top_row = self.top_row.clamp(self.fixed_rows() as i64, rows.max(1) - 1).max(0);
        self.left_col = self.left_col.clamp(self.fixed_cols() as i64, cols.max(1) - 1).max(0);
    }

    /// Selects cell (`col`, `row`) if it's in the grid.
    pub fn select(&mut self, col: i64, row: i64) {
        if (0..self.col_count as i64).contains(&col) && (0..self.row_count() as i64).contains(&row) {
            self.col = col;
            self.row = row;
            self.anchor = None;
        }
    }

    /// The selection: the selected cell and where a range started.
    pub fn selection(&self) -> Selection {
        (self.col, self.row, self.anchor)
    }

    /// Puts a selection back (OnSelectCell answered `CanSelect = 0`).
    pub fn set_selection(&mut self, (col, row, anchor): Selection) {
        (self.col, self.row, self.anchor) = (col, row, anchor);
    }

    /// The user clicked (or moved with the keys) to cell (`col`, `row`), or
    /// with `extend` dragged or shift-clicked to it: selects it (fixed
    /// cells can't be; a range needs goRangeSelect). Returns the selection
    /// before if it moved: the runtime then fires OnSelectCell(Col, Row,
    /// CanSelect) and puts it back if the handler refuses.
    pub fn user_select(&mut self, col: i64, row: i64, extend: bool) -> Option<Selection> {
        if col < 0 || row < 0 || (row as usize) < self.fixed_rows() || (col as usize) < self.fixed_cols() || (extend && !self.range_select()) {
            return None;
        }
        let before = self.selection();
        if extend {
            self.extend_to(col, row);
        } else {
            self.select(col, row);
        }
        let reached = extend || (self.col, self.row) == (col, row);
        (reached && before != self.selection()).then_some(before)
    }

    /// Whether the user can select a range: goRangeSelect, and not
    /// goEditing (which turns it off, per the manual).
    pub fn range_select(&self) -> bool {
        self.has_option(GO_RANGE_SELECT) && !self.editable()
    }

    /// The user dragged (or shift-clicked) to (col, row): with range
    /// selection the range grows from where it started; otherwise the cell
    /// is selected alone.
    pub fn extend_to(&mut self, col: i64, row: i64) {
        if !self.range_select() {
            return self.select(col, row);
        }
        let anchor = self.anchor.unwrap_or((self.col, self.row));
        if (0..self.col_count as i64).contains(&col) && (0..self.row_count() as i64).contains(&row) {
            self.col = col;
            self.row = row;
            self.anchor = Some(anchor);
        }
    }

    /// Whether cell (col, row) is selected: the selected cell, its row with
    /// goRowSelect, or a cell of the range.
    pub fn is_selected(&self, col: usize, row: usize) -> bool {
        if row < self.fixed_rows() || col < self.fixed_cols() {
            return false;
        }
        let (c, r) = (col as i64, row as i64);
        if self.has_option(GO_ROW_SELECT) {
            return r == self.row;
        }
        match self.anchor {
            Some((ac, ar)) => (ac.min(self.col)..=ac.max(self.col)).contains(&c) && (ar.min(self.row)..=ar.max(self.row)).contains(&r),
            None => (c, r) == (self.col, self.row),
        }
    }

    /// Rows `row_offset…` (at most `max_rows`) as text: one line per row,
    /// the cells from `col_offset` joined by the separator.
    pub fn to_text(&self, row_offset: usize, col_offset: usize, max_rows: usize) -> String {
        let mut out = String::new();
        for r in self.cells.iter().skip(row_offset).take(max_rows) {
            let cells: Vec<&str> = r.iter().skip(col_offset).map(|s| s.as_str()).collect();
            out.push_str(&cells.join(&self.separator));
            out.push_str("\r\n");
        }
        out
    }

    /// Fills the grid from text written by [`Self::to_text`]: line `i` (at
    /// most `max_rows`) into row `row_offset + i`, from column `col_offset`,
    /// growing the grid to fit.
    pub fn load_text(&mut self, text: &str, row_offset: usize, col_offset: usize, max_rows: usize) {
        let lines: Vec<&str> = text.lines().take(max_rows).collect();
        let split = |line: &str| -> Vec<String> {
            if self.separator.is_empty() {
                vec![line.to_string()]
            } else {
                line.split(self.separator.as_str()).map(str::to_string).collect()
            }
        };
        let rows: Vec<Vec<String>> = lines.iter().map(|l| split(l)).collect();
        let need_rows = row_offset + rows.len();
        let need_cols = col_offset + rows.iter().map(Vec::len).max().unwrap_or(0);
        self.resize(need_rows.max(self.row_count()), need_cols.max(self.col_count));
        for (i, cells) in rows.into_iter().enumerate() {
            for (j, s) in cells.into_iter().enumerate() {
                self.set_cell(col_offset + j, row_offset + i, s);
            }
        }
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "colcount" | "cols" => v_int(self.col_count as i64),
            "rowcount" | "rows" => v_int(self.row_count() as i64),
            "fixedcols" => v_int(self.fixed_cols() as i64),
            "fixedrows" => v_int(self.fixed_rows() as i64),
            "defaultcolwidth" | "colwidth" => v_int(self.default_col_width),
            "defaultrowheight" => v_int(self.default_row_height),
            "col" | "selectedcol" => v_int(self.col),
            "row" | "selectedrow" => v_int(self.row),
            "toprow" => v_int(self.top_row),
            "leftcol" => v_int(self.left_col),
            "separator" => v_str(&self.separator),
            "gridwidth" => v_int(self.col_widths.iter().sum()),
            "gridheight" => v_int(self.row_heights.iter().sum()),
            "editormode" => flag(self.editable()),
            "visiblerowcount" => v_int(self.visible_count(&self.row_heights, self.fixed_rows(), self.top_row, self.view.1)),
            "visiblecolcount" => v_int(self.visible_count(&self.col_widths, self.fixed_cols(), self.left_col, self.view.0)),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        let n = val.to_i64();
        let count = usize::try_from(n).unwrap_or(0);
        match prop {
            "colcount" | "cols" => self.resize(self.row_count(), count),
            "rowcount" | "rows" => self.resize(count, self.col_count),
            // (a grid not scrolled, its current cell the first one beside the
            // fixed ones, stays so: with FixedCols = 0 column 0 shows and is
            // the current one, as Delphi's grid does)
            "fixedcols" => {
                let old = self.fixed_cols() as i64;
                self.want_fixed_cols = count;
                let new = self.fixed_cols() as i64;
                if self.left_col == old {
                    self.left_col = new;
                }
                if self.col == old {
                    self.col = new;
                }
                self.fix_selection();
            }
            "fixedrows" => {
                let old = self.fixed_rows() as i64;
                self.want_fixed_rows = count;
                let new = self.fixed_rows() as i64;
                if self.top_row == old {
                    self.top_row = new;
                }
                if self.row == old {
                    self.row = new;
                }
                self.fix_selection();
            }
            // Setting the default size sizes every column / row (Delphi).
            "defaultcolwidth" | "colwidth" => {
                self.default_col_width = n.clamp(0, 10_000);
                self.col_widths.fill(self.default_col_width);
            }
            "defaultrowheight" => {
                self.default_row_height = n.clamp(0, 10_000);
                self.row_heights.fill(self.default_row_height);
            }
            "col" | "selectedcol" => self.select(n, self.row),
            "row" | "selectedrow" => self.select(self.col, n),
            "toprow" => {
                self.top_row = n;
                self.fix_selection();
            }
            "leftcol" => {
                self.left_col = n;
                self.fix_selection();
            }
            "separator" => self.separator = val.to_string_val(),
            "editormode" => {
                if val.to_bool() {
                    self.options |= bit(GO_EDITING);
                }
            }
            _ => return false,
        }
        true
    }

    /// VisibleRowCount / VisibleColCount: the scrolling rows (columns) shown
    /// whole from `first` on, in `room` pixels less the fixed ones (each with
    /// its grid line).
    fn visible_count(&self, sizes: &[i64], fixed: usize, first: i64, room: i64) -> i64 {
        let size = |v: &i64| (*v).clamp(0, 10_000) + 1;
        let mut left = room - sizes.iter().take(fixed).map(size).sum::<i64>();
        let mut n = 0;
        for s in sizes.iter().skip(usize::try_from(first).unwrap_or(0).max(fixed)) {
            left -= size(s);
            if left < 0 {
                break;
            }
            n += 1;
        }
        n
    }

    /// Moves column `from` to `to` (goColMoving): its cells, width, style
    /// and list go with it, and the selection stays on the cell it was on.
    pub fn move_col(&mut self, from: usize, to: usize) -> bool {
        let n = self.col_count;
        if from >= n || to >= n || from == to {
            return false;
        }
        for row in &mut self.cells {
            let c = row.remove(from);
            row.insert(to, c);
        }
        let w = self.col_widths.remove(from);
        self.col_widths.insert(to, w);
        for v in [&mut self.column_styles] {
            if v.len() > from.max(to) {
                let x = v.remove(from);
                v.insert(to, x);
            }
        }
        if self.column_lists.len() > from.max(to) {
            let x = self.column_lists.remove(from);
            self.column_lists.insert(to, x);
        }
        self.col = moved_index(self.col, from, to);
        self.anchor = self.anchor.map(|(c, r)| (moved_index(c, from, to), r));
        self.owner_drawing.clear();
        self.drawn_state = None;
        true
    }

    /// Moves row `from` to `to` (goRowMoving), as [`Self::move_col`].
    pub fn move_row(&mut self, from: usize, to: usize) -> bool {
        let n = self.row_count();
        if from >= n || to >= n || from == to {
            return false;
        }
        let r = self.cells.remove(from);
        self.cells.insert(to, r);
        let h = self.row_heights.remove(from);
        self.row_heights.insert(to, h);
        self.row = moved_index(self.row, from, to);
        self.anchor = self.anchor.map(|(c, r)| (c, moved_index(r, from, to)));
        self.owner_drawing.clear();
        self.drawn_state = None;
        true
    }

    /// Cell (col, row)'s rectangle (left, top, right, bottom; right and
    /// bottom excluded) in the grid's coordinates: cells from the top left
    /// with a 1px line after each.
    pub fn cell_rect(&self, col: usize, row: usize) -> (i64, i64, i64, i64) {
        let size = |v: &i64| (*v).clamp(0, 10_000);
        let left: i64 = self.col_widths.iter().take(col).map(|w| size(w) + 1).sum();
        let top: i64 = self.row_heights.iter().take(row).map(|h| size(h) + 1).sum();
        let w = self.col_widths.get(col).map_or(0, size);
        let h = self.row_heights.get(row).map_or(0, size);
        (left, top, left + w, top + h)
    }

    /// The cell at (x, y) in the grid's coordinates.
    pub fn cell_at(&self, x: i64, y: i64) -> Option<(usize, usize)> {
        fn find(sizes: &[i64], at: i64) -> Option<usize> {
            let mut edge = 0;
            for (i, s) in sizes.iter().enumerate() {
                edge += (*s).clamp(0, 10_000) + 1;
                if at < edge {
                    return Some(i);
                }
            }
            None
        }
        if x < 0 || y < 0 {
            return None;
        }
        Some((find(&self.col_widths, x)?, find(&self.row_heights, y)?))
    }

    /// Whether the grid changed (content, sizes, selection, options) since
    /// OnDrawCell was last fired: if so its drawing is dropped and it must
    /// be fired again. Drawing on the grid isn't a change.
    pub fn owner_draw_needed(&mut self) -> bool {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (&self.cells, &self.col_widths, &self.row_heights, self.col_count).hash(&mut h);
        (self.fixed_rows(), self.fixed_cols(), self.col, self.row, self.anchor, self.options).hash(&mut h);
        let state = h.finish();
        if self.drawn_state == Some(state) {
            return false;
        }
        self.drawn_state = Some(state);
        self.owner_drawing.clear();
        true
    }

    /// Each cell's OnDrawCell arguments: (col, row, State, Rect), up to
    /// [`MAX_OWNER_DRAWN`] cells.
    pub fn owner_draw_cells(&self) -> Vec<(usize, usize, i64, (i64, i64, i64, i64))> {
        let mut out = Vec::new();
        for row in 0..self.row_count() {
            for col in 0..self.col_count() {
                if out.len() >= MAX_OWNER_DRAWN {
                    return out;
                }
                let fixed = row < self.fixed_rows() || col < self.fixed_cols();
                let focused = (col as i64, row as i64) == (self.col, self.row);
                let state = if fixed {
                    GD_FIXED
                } else {
                    (if self.is_selected(col, row) { GD_SELECTED } else { 0 }) | (if focused { GD_FOCUSED } else { 0 })
                };
                out.push((col, row, state, self.cell_rect(col, row)));
            }
        }
        out
    }

    /// Keeps a drawing whose anchor is (x, y): on the cell there, with
    /// `make` given that cell's origin (to make it relative).
    pub fn record(&mut self, x: i64, y: i64, make: impl FnOnce(i64, i64) -> CellDraw) {
        let Some((col, row)) = self.cell_at(x, y) else { return };
        let (left, top, _, _) = self.cell_rect(col, row);
        let list = self.owner_drawing.entry((col, row)).or_default();
        if list.len() < 1_000 {
            list.push(make(left, top));
        }
    }

    /// The owner-drawing methods (except `Draw`, which needs the source
    /// image: rapidr_value::objects): [`owner_draw_op`]'s.
    fn draw(&mut self, method: &str, args: &[Value]) -> bool {
        let Some(((x, y), op)) = owner_draw_op(method, args) else { return false };
        self.record(x, y, |l, t| op.moved(-l, -t));
        true
    }

    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        let text = |i: usize| args.get(i).map(|v| v.to_string_val()).unwrap_or_default();
        if self.draw(method, args) {
            return Some(Value::Null);
        }
        match method {
            // Cell(col, row) [= s]; RapidR's GetCell(col, row) / SetCell(col, row, s).
            "cell" | "cells" | "getcell" | "setcell" => {
                let (col, row) = (index(args.first()), index(args.get(1)));
                if args.len() >= 3 && method != "getcell" {
                    if let (Some(col), Some(row)) = (col, row) {
                        self.set_cell(col, row, text(2));
                    }
                } else if method != "setcell" {
                    return Some(v_str(col.zip(row).map_or("", |(c, r)| self.cell(c, r))));
                }
            }
            "colwidths" | "rowheights" => {
                let sizes = if method == "colwidths" { &mut self.col_widths } else { &mut self.row_heights };
                let i = index(args.first());
                if args.len() >= 2 {
                    if let Some(size) = i.and_then(|i| sizes.get_mut(i)) {
                        *size = arg(1).to_i64().clamp(0, 10_000);
                    }
                } else {
                    return Some(v_int(i.and_then(|i| sizes.get(i).copied()).unwrap_or(0)));
                }
            }
            "columnstyle" => {
                let i = index(args.first());
                if args.len() >= 2 {
                    if let Some(i) = i.filter(|&i| i < self.col_count) {
                        if self.column_styles.len() <= i {
                            self.column_styles.resize(i + 1, GCS_NONE);
                        }
                        self.column_styles[i] = arg(1).to_i64();
                    }
                } else {
                    return Some(v_int(i.map_or(GCS_NONE, |i| self.column_style(i))));
                }
            }
            "columnlist" => {
                let i = index(args.first());
                if args.len() >= 2 {
                    if let Some(i) = i.filter(|&i| i < self.col_count) {
                        if self.column_lists.len() <= i {
                            self.column_lists.resize(i + 1, String::new());
                        }
                        self.column_lists[i] = text(1);
                    }
                } else {
                    return Some(v_str(i.and_then(|i| self.column_lists.get(i)).map_or("", |s| s.as_str())));
                }
            }
            "addoptions" | "deloptions" => {
                for a in args {
                    let b = u32::try_from(a.to_i64()).map(bit).unwrap_or(0);
                    if method == "addoptions" {
                        self.options |= b;
                    } else {
                        self.options &= !b;
                    }
                }
            }
            "insertrow" | "insertcol" => {
                let at = index(args.first()).unwrap_or(usize::MAX);
                if method == "insertrow" {
                    if self.row_count() < MAX_ROWS && (self.row_count() + 1) * self.col_count.max(1) <= MAX_CELLS {
                        let at = at.min(self.row_count());
                        self.cells.insert(at, vec![String::new(); self.col_count]);
                        self.row_heights.insert(at, self.default_row_height);
                    }
                } else if self.col_count < MAX_COLS && self.row_count() * (self.col_count + 1) <= MAX_CELLS {
                    let at = at.min(self.col_count);
                    for r in &mut self.cells {
                        r.insert(at, String::new());
                    }
                    self.col_widths.insert(at, self.default_col_width);
                    self.col_count += 1;
                }
            }
            "deleterow" => {
                if let Some(at) = index(args.first()).filter(|&i| i < self.row_count()) {
                    self.cells.remove(at);
                    self.row_heights.remove(at);
                    self.fix_selection();
                }
            }
            "deletecol" => {
                if let Some(at) = index(args.first()).filter(|&i| i < self.col_count) {
                    for r in &mut self.cells {
                        r.remove(at);
                    }
                    self.col_widths.remove(at);
                    self.col_count -= 1;
                    self.fix_selection();
                }
            }
            // (RapidR: what goColMoving / goRowMoving do, from the program)
            "movecol" | "movecolumn" => {
                if let (Some(a), Some(b)) = (index(args.first()), index(args.get(1))) {
                    self.move_col(a, b);
                }
            }
            "moverow" => {
                if let (Some(a), Some(b)) = (index(args.first()), index(args.get(1))) {
                    self.move_row(a, b);
                }
            }
            "swaprows" => {
                if let (Some(a), Some(b)) = (index(args.first()), index(args.get(1))) {
                    if a < self.row_count() && b < self.row_count() {
                        self.cells.swap(a, b);
                    }
                }
            }
            "swapcols" => {
                if let (Some(a), Some(b)) = (index(args.first()), index(args.get(1))) {
                    if a < self.col_count && b < self.col_count {
                        for r in &mut self.cells {
                            r.swap(a, b);
                        }
                    }
                }
            }
            // RapidR: AddRow a, b, … appends a row, widening the grid.
            "addrow" => {
                let cols = self.col_count.max(args.len());
                self.resize(self.row_count() + 1, cols);
                let row = self.row_count() - 1;
                for (i, a) in args.iter().enumerate() {
                    self.set_cell(i, row, a.to_string_val());
                }
            }
            "clear" => self.resize(0, self.col_count),
            "setrowcount" => self.resize(usize::try_from(arg(0).to_i64()).unwrap_or(0), self.col_count),
            "setcolcount" => self.resize(self.row_count(), usize::try_from(arg(0).to_i64()).unwrap_or(0)),
            "setsuggestions" => self.suggestions = text(0).lines().map(str::to_string).collect(),
            // Every cell is drawn again: OnDrawCell fires again.
            "repaint" | "refresh" => self.drawn_state = None,
            _ => return None,
        }
        Some(Value::Null)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn moving_and_visible_counts() {
        let mut g = StringGrid::default();
        for c in 0..5 {
            g.set_cell(c, 1, format!("c{c}"));
        }
        g.col_widths[1] = 30;
        g.select(1, 1);
        assert!(g.move_col(1, 3));
        assert_eq!((g.cell(3, 1), g.cell(1, 1), g.col_widths[3], g.col), ("c1", "c2", 30, 3));
        g.set_cell(0, 2, "row2".into());
        assert!(g.move_row(2, 4));
        assert_eq!(g.cell(0, 4), "row2");
        assert_eq!(g.row, 1);
        assert!(!g.move_col(9, 0));
        // Rows of 24 (+1 line), a fixed one: 99 px show 2 whole, 100 px 3.
        g.view = (300, 99);
        assert_eq!(g.get("visiblerowcount").unwrap().to_i64(), 2);
        g.view = (300, 100);
        assert_eq!(g.get("visiblerowcount").unwrap().to_i64(), 3);
        g.view = (1000, 1000);
        assert_eq!(g.get("visiblecolcount").unwrap().to_i64(), 4);
    }

    #[test]
    fn no_fixed_column_shows_column_0() {
        let mut g = StringGrid::default();
        g.set("fixedcols", &v_int(0));
        assert_eq!((g.left_col, g.col, g.fixed_cols()), (0, 0, 0));
        g.set("fixedrows", &v_int(0));
        assert_eq!((g.top_row, g.row), (0, 0));
        // (scrolled, it stays where it was)
        let mut g = StringGrid::default();
        g.left_col = 3;
        g.set("fixedcols", &v_int(2));
        assert_eq!(g.left_col, 3);
    }

    #[test]
    fn range_selection() {
        let mut g = StringGrid::default();
        assert!(g.range_select());
        g.select(1, 1);
        g.extend_to(3, 2);
        assert!(g.is_selected(2, 2) && g.is_selected(1, 1) && !g.is_selected(4, 2) && !g.is_selected(0, 1));
        assert_eq!((g.col, g.row), (3, 2));
        let states: Vec<i64> = g.owner_draw_cells().iter().filter(|c| (c.0, c.1) == (2, 1) || (c.0, c.1) == (3, 2)).map(|c| c.2).collect();
        assert_eq!(states, [GD_SELECTED, GD_SELECTED | GD_FOCUSED]);
        // Clicking selects one cell again; goEditing turns ranges off.
        g.select(2, 2);
        assert!(!g.is_selected(1, 1));
        g.call("addoptions", &[v_int(GO_EDITING as i64)]);
        g.extend_to(4, 4);
        assert!(!g.is_selected(2, 2) && g.is_selected(4, 4));
    }

    #[test]
    fn owner_drawing() {
        let mut g = StringGrid::default();
        // 64 wide columns, 24 high rows, 1px lines.
        assert_eq!(g.cell_rect(2, 1), (130, 25, 194, 49));
        assert_eq!(g.cell_at(130, 25), Some((2, 1)));
        assert_eq!(g.cell_at(129, 25), Some((1, 1)));
        assert!(g.owner_draw_needed());
        assert!(!g.owner_draw_needed());
        let cells = g.owner_draw_cells();
        assert_eq!(cells.len(), 25);
        assert_eq!(cells[0].2, GD_FIXED);
        assert_eq!(cells.iter().find(|c| (c.0, c.1) == (1, 1)).unwrap().2, GD_SELECTED | GD_FOCUSED);
        // Drawing is kept relative to its cell and isn't a change.
        g.call("line", &[v_int(135), v_int(30), v_int(140), v_int(40), v_int(255)]);
        g.call("textout", &[v_int(132), v_int(27), v_str("x"), v_int(0), v_int(-1)]);
        let kept = &g.owner_drawing[&(2, 1)];
        assert!(matches!(kept[0], CellDraw::Line(5, 5, 10, 15, 255)));
        assert!(matches!(&kept[1], CellDraw::Text(2, 2, t, 0, None) if t == "x"));
        let _ = g.call("cell", &[v_int(1), v_int(1)]);
        assert!(!g.owner_draw_needed());
        // Repaint draws again, as does a change (which drops the drawing).
        g.call("repaint", &[]);
        assert!(g.owner_draw_needed());
        g.call("cell", &[v_int(1), v_int(1), v_str("new")]);
        assert!(g.owner_draw_needed());
        assert!(g.owner_drawing.is_empty());
    }

    use super::*;

    fn s(x: &str) -> Value {
        v_str(x)
    }

    #[test]
    fn rapidq_defaults_and_cells() {
        let mut g = StringGrid::default();
        assert_eq!((g.col_count(), g.row_count()), (5, 5));
        assert_eq!(g.get("fixedrows").unwrap().to_i64(), 1);
        assert_eq!(g.get("defaultcolwidth").unwrap().to_i64(), 64);
        g.call("cell", &[v_int(1), v_int(0), s("Age")]);
        assert_eq!(g.call("cell", &[v_int(1), v_int(0)]).unwrap().to_string_val(), "Age");
        g.call("setcell", &[v_int(2), v_int(3), s("x")]);
        assert_eq!(g.call("getcell", &[v_int(2), v_int(3)]).unwrap().to_string_val(), "x");
        assert_eq!(g.cell(2, 3), "x");
        g.call("colwidths", &[v_int(0), v_int(25)]);
        assert_eq!(g.call("colwidths", &[v_int(0)]).unwrap().to_i64(), 25);
        assert_eq!(g.call("colwidths", &[v_int(1)]).unwrap().to_i64(), 64);
        assert_eq!(g.get("gridwidth").unwrap().to_i64(), 25 + 4 * 64);
    }

    #[test]
    fn rows_and_columns_change() {
        let mut g = StringGrid::default();
        g.set("colcount", &v_int(3));
        g.set("rowcount", &v_int(2));
        g.call("cell", &[v_int(2), v_int(1), s("x")]);
        g.call("insertrow", &[v_int(1)]);
        assert_eq!(g.cell(2, 2), "x");
        g.call("insertcol", &[v_int(0)]);
        assert_eq!(g.cell(3, 2), "x");
        g.call("swaprows", &[v_int(0), v_int(2)]);
        assert_eq!(g.cell(3, 0), "x");
        g.call("deletecol", &[v_int(3)]);
        assert_eq!((g.col_count(), g.row_count()), (3, 3));
        g.call("deleterow", &[v_int(0)]);
        assert_eq!(g.row_count(), 2);
        g.set("rowcount", &v_int(1));
        assert_eq!(g.get("fixedrows").unwrap().to_i64(), 0);
        // …and the header row comes back with the rows.
        g.set("rowcount", &v_int(4));
        assert_eq!(g.get("fixedrows").unwrap().to_i64(), 1);
    }

    #[test]
    fn rapidr_add_row_and_clear() {
        let mut g = StringGrid::default();
        g.call("clear", &[]);
        assert_eq!(g.row_count(), 0);
        g.set("cols", &v_int(2));
        g.call("addrow", &[s("Event"), s("Handler"), s("...")]);
        g.call("addrow", &[s("OnClick"), s("Go")]);
        assert_eq!((g.col_count(), g.row_count()), (3, 2));
        assert_eq!(g.call("getcell", &[v_int(1), v_int(1)]).unwrap().to_string_val(), "Go");
        assert_eq!(g.call("getcell", &[v_int(2), v_int(0)]).unwrap().to_string_val(), "...");
        g.set("selectedrow", &v_int(1));
        assert_eq!(g.get("row").unwrap().to_i64(), 1);
    }

    #[test]
    fn text_round_trip() {
        let mut g = StringGrid::default();
        g.set("separator", &s(";"));
        g.call("cell", &[v_int(0), v_int(0), s("a")]);
        g.call("cell", &[v_int(4), v_int(1), s("b")]);
        let text = g.to_text(0, 0, 2);
        assert_eq!(text, "a;;;;\r\n;;;;b\r\n");
        let mut h = StringGrid::default();
        h.set("separator", &s(";"));
        h.set("rowcount", &v_int(1));
        h.load_text("1;2;3;4;5;6\r\nx\r\ny", 1, 0, 2);
        assert_eq!((h.col_count(), h.row_count()), (6, 3));
        assert_eq!(h.cell(5, 1), "6");
        assert_eq!(h.cell(0, 2), "x");
    }

    #[test]
    fn out_of_range_and_huge_sizes_are_harmless() {
        let mut g = StringGrid::default();
        assert_eq!(g.call("cell", &[v_int(-1), v_int(99)]).unwrap().to_string_val(), "");
        g.call("cell", &[v_int(99), v_int(99), s("x")]);
        g.call("colwidths", &[v_int(-3), v_int(10)]);
        g.set("rowcount", &v_int(i64::MAX));
        g.set("colcount", &v_int(i64::MAX));
        assert!(g.row_count() * g.col_count() <= MAX_CELLS);
        g.set("rowcount", &v_int(-5));
        assert_eq!(g.row_count(), 0);
        g.call("addoptions", &[v_int(99), v_int(GO_EDITING as i64)]);
        assert!(g.editable());
    }
}
