//! RDATAFRAME: a table in the manner of pandas — named columns, rows of
//! cells. Methods change the frame in place (`df.filter …`, `df.sort …`) or
//! answer a value (`df.cell(0, 1)`, `df.columns`).
//!
//! A cell is text as it was read (`"30"`, `"bob"`) or null (an empty CSV
//! field, JSON's `null`); a column's type is what its cells are — `i64` if
//! every one is a whole number, `f64` if a number, `bool` if `true` /
//! `false`, `str` otherwise — and decides how it sorts, compares and is
//! summed. Numbers the frame computes (`describe`, `groupby`) are written as
//! RapidR writes numbers ([`super::num_text`]).
//!
//! **The engine** (docs/data-science-plan.md §2; decision D7: our own, the
//! same in native builds, the interpreter and the web). Columns are stored
//! typed — `i64`, `f64`, `bool` in plain vectors, text as one buffer with
//! offsets — with a validity bitmap beside them when a cell is null, every
//! buffer shared (`Arc`) so a copied frame costs nothing until it changes
//! (`frame/column.rs`). A column is typed only when that is lossless (each cell's
//! text is its value's own text: `"30"`, `"2.5"`, `"true"`); `"007"`,
//! `"1.50"` or `"True"` keep their text, so what a program reads never
//! changes with the storage. The kernels (`frame/kernels.rs`: CSV reading and
//! writing, filter, sort, group-by, join, describe, gathering rows) work on
//! typed slices and split big work across the machine's cores
//! (`std::thread::scope`, `frame/par.rs`; under 50,000 rows, and on the web, on the
//! calling thread). A million-row CSV (37 MB) loads in about 12 ms on an M4
//! Max and is filtered, sorted, grouped or joined in 3–10 ms; ten million
//! rows load, filter, sort and group in about 0.3 s (`cargo run --release
//! -p rapidr-value --example frame_bench`; tools/bench/ compares it with
//! pandas and Polars). tests/frame_oracle.rs holds it to the answers of the
//! engine before it, byte for byte.
//!
//! Printed (`ToString`, `Print`), a frame is a plain-text table, the same
//! on every runtime ([`Frame::to_text`]):
//!
//! ```text
//! name  age  city
//! ----  ---  ----
//! bob    30  NYC
//! amy    25  LA
//! [2 rows x 3 columns]
//! ```

mod column;
mod csv;
mod json;
mod kernels;
mod par;

use std::borrow::Cow;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;

pub use column::Column;
#[doc(hidden)]
pub use par::set_thresholds;
pub use csv::{parse_csv, parse_csv_opts, to_csv, to_csv_opts};
pub use json::{parse_json_frame, to_json};

use super::{arg_i, arg_s, arg_s_or, key, num_text, parse_num, random_index, Host};
use crate::{v_dbl, v_int, v_null, v_str, Value};
use csv::{csv_records, field_value};

/// A column's type, from its cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DType {
    Int,
    Float,
    Bool,
    Str,
}

impl DType {
    pub fn name(self) -> &'static str {
        match self {
            DType::Int => "i64",
            DType::Float => "f64",
            DType::Bool => "bool",
            DType::Str => "str",
        }
    }

    pub fn numeric(self) -> bool {
        matches!(self, DType::Int | DType::Float)
    }
}

/// A table: column names and their columns, every column `height` long.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub columns: Vec<String>,
    cols: Vec<Column>,
    height: usize,
}

/// Most rows a printed frame shows (the first and last half of them).
const PRINT_ROWS: usize = 20;
/// Widest cell a printed frame shows in full.
const PRINT_CELL: usize = 40;

impl Frame {
    pub fn width(&self) -> usize {
        self.columns.len()
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Column `name`'s index: the exact name first, then any case.
    pub fn col(&self, name: &str) -> Option<usize> {
        let name = name.trim();
        self.columns.iter().position(|c| c == name).or_else(|| self.columns.iter().position(|c| c.eq_ignore_ascii_case(name)))
    }

    /// A column by its index (a number) or its name.
    fn col_of(&self, v: Option<&Value>) -> Option<usize> {
        match v {
            Some(Value::Integer(i)) => usize::try_from(*i).ok().filter(|i| *i < self.width()),
            Some(Value::Double(d)) => (*d >= 0.0).then_some(*d as usize).filter(|i| *i < self.width()),
            Some(other) => {
                let s = other.to_string_val();
                match s.trim().parse::<usize>() {
                    Ok(i) if self.col(&s).is_none() => (i < self.width()).then_some(i),
                    _ => self.col(&s),
                }
            }
            None => None,
        }
    }

    /// Cell (`row`, `col`)'s text (None: null, or no such cell). Owned
    /// when the engine keeps the column typed (a number written as RapidR
    /// writes it), borrowed when it keeps the text as read.
    pub fn cell(&self, row: usize, col: usize) -> Option<Cow<'_, str>> {
        self.cols.get(col).and_then(|c| c.get(row))
    }

    /// Column `c`'s type.
    pub fn dtype(&self, c: usize) -> DType {
        self.cols[c].dtype()
    }

    /// Row `r`'s cells.
    pub fn row(&self, r: usize) -> Vec<Option<Cow<'_, str>>> {
        self.cols.iter().map(|c| c.get(r)).collect()
    }

    /// A new column of nulls (a name already taken gets `_2`, `_3` …).
    fn add_column(&mut self, name: &str) -> usize {
        let mut n = name.trim().to_string();
        if n.is_empty() {
            n = format!("column_{}", self.width() + 1);
        }
        if self.columns.contains(&n) {
            let mut k = 2;
            while self.columns.contains(&format!("{n}_{k}")) {
                k += 1;
            }
            n = format!("{n}_{k}");
        }
        self.columns.push(n);
        self.cols.push(Column::nulls(self.height));
        self.width() - 1
    }

    /// At least `cols` columns (`column_4` …) and `rows` rows.
    fn grow(&mut self, rows: usize, cols: usize) {
        while self.width() < cols {
            let n = self.width() + 1;
            self.add_column(&format!("column_{n}"));
        }
        if rows > self.height {
            self.height = rows;
            for c in &mut self.cols {
                c.grow(rows);
            }
        }
    }

    /// A row at the end (cells past the columns add columns).
    fn push_row(&mut self, cells: &[Option<String>]) {
        self.grow(self.height, cells.len());
        for (c, col) in self.cols.iter_mut().enumerate() {
            col.push(cells.get(c).and_then(|v| v.as_deref()));
        }
        self.height += 1;
    }

    /// Only the rows at `rows`, in that order (a run of rows is cut out,
    /// any other order gathered — in parallel for many rows).
    fn keep(&mut self, rows: &[usize]) {
        let run = rows.windows(2).all(|w| w[1] == w[0] + 1);
        for c in &mut self.cols {
            *c = match rows.first() {
                Some(&s) if run && s + rows.len() <= c.len() => c.slice(s..s + rows.len()),
                _ => c.gather(rows),
            };
        }
        self.height = rows.len();
    }

    /// The first `n` rows.
    fn truncate(&mut self, n: usize) {
        if n < self.height {
            for c in &mut self.cols {
                c.truncate(n);
            }
            self.height = n;
        }
    }

    /// The frame as printed: a header, a rule, the rows (the first and
    /// last ten of a longer frame), the size.
    pub fn to_text(&self) -> String {
        let (h, w) = (self.height(), self.width());
        let size = format!("[{h} {} x {w} {}]", if h == 1 { "row" } else { "rows" }, if w == 1 { "column" } else { "columns" });
        if w == 0 {
            return size;
        }
        let shown: Vec<Option<usize>> = if h > PRINT_ROWS {
            (0..PRINT_ROWS / 2).map(Some).chain(std::iter::once(None)).chain((h - PRINT_ROWS / 2..h).map(Some)).collect()
        } else {
            (0..h).map(Some).collect()
        };
        let clip = |s: &str| -> String {
            if s.chars().count() > PRINT_CELL {
                s.chars().take(PRINT_CELL - 3).collect::<String>() + "..."
            } else {
                s.to_string()
            }
        };
        let mut cols: Vec<(Vec<String>, bool)> = Vec::new();
        for c in 0..w {
            let cells: Vec<String> = shown
                .iter()
                .map(|r| match r {
                    Some(r) => clip(self.cell(*r, c).as_deref().unwrap_or("null")),
                    None => "...".to_string(),
                })
                .collect();
            cols.push((cells, self.dtype(c).numeric()));
        }
        let widths: Vec<usize> = (0..w)
            .map(|c| cols[c].0.iter().map(|s| s.chars().count()).chain(std::iter::once(clip(&self.columns[c]).chars().count())).max().unwrap_or(0))
            .collect();
        let pad = |s: &str, width: usize, right: bool| -> String {
            let fill = " ".repeat(width.saturating_sub(s.chars().count()));
            if right {
                fill + s
            } else {
                s.to_string() + &fill
            }
        };
        let line = |cells: Vec<String>| cells.join("  ").trim_end().to_string();
        let mut out = vec![
            line((0..w).map(|c| pad(&clip(&self.columns[c]), widths[c], cols[c].1)).collect()),
            line(widths.iter().map(|n| "-".repeat(*n)).collect()),
        ];
        for i in 0..shown.len() {
            out.push(line((0..w).map(|c| pad(&cols[c].0[i], widths[c], cols[c].1)).collect()));
        }
        out.push(size);
        out.join("\n")
    }

    /// The frame as a grid's rows: the header, then the cells (null as "").
    pub fn grid_rows(&self) -> Vec<Vec<String>> {
        std::iter::once(self.columns.clone())
            .chain((0..self.height).map(|r| self.cols.iter().map(|c| c.get(r).map_or_else(String::new, Cow::into_owned)).collect()))
            .collect()
    }

    /// Cell (`row`, `col`) becomes `cell` (None: null) — what a data-aware
    /// control writes back. A row past the end grows the frame; no such
    /// column: nothing.
    pub fn set_cell(&mut self, row: usize, col: usize, cell: Option<&str>) {
        if col < self.width() {
            self.grow(row + 1, 0);
            self.cols[col].set(row, cell);
        }
    }

    /// The order the rows sort in by column `col` (numbers as numbers,
    /// nulls last), stably — the rows themselves stay where they are: a
    /// grid sorts its view with it. (No such column: the rows' own order.)
    pub fn sorted_order(&self, col: usize, descending: bool) -> Vec<usize> {
        if col < self.width() {
            sorted_rows(self, &[(col, descending)])
        } else {
            (0..self.height).collect()
        }
    }

    /// The rows sorted by column `col` (numbers as numbers, nulls last),
    /// stably — a grid's heading clicked.
    pub fn sort_by_column(&mut self, col: usize, descending: bool) {
        if col < self.width() {
            let order = sorted_rows(self, &[(col, descending)]);
            self.keep(&order);
        }
    }

    /// A frame with these columns and no rows (a name already taken gets
    /// `_2`, `_3` … — `SELECT a.id, b.id` — an empty one `column_N`).
    pub fn with_columns<S: AsRef<str>>(names: &[S]) -> Frame {
        let mut f = Frame::default();
        for n in names {
            f.add_column(n.as_ref());
        }
        f
    }

    /// `other`'s rows after these, column by column (its columns past
    /// these are dropped; a column it lacks gets nulls). A background
    /// query's next chunk of rows (rapidr-db): each column is copied once,
    /// whatever the chunks' types ([`Column::append`] keeps one storage).
    pub fn append(&mut self, other: &Frame) {
        let n = other.height;
        if n == 0 {
            return;
        }
        for (c, col) in self.cols.iter_mut().enumerate() {
            match other.cols.get(c) {
                Some(theirs) => col.append(theirs),
                None => col.grow(col.len() + n),
            }
        }
        self.height += n;
    }

    /// The first `n` rows, as a frame of their own.
    pub fn head(&self, n: usize) -> Frame {
        let mut f = self.clone();
        f.truncate(n);
        f
    }

    /// Column `c` becomes `values` (padded with nulls, or the frame grown,
    /// to one cell a row).
    fn set_column(&mut self, c: usize, values: Vec<Option<String>>) {
        if values.len() > self.height {
            self.grow(values.len(), 0);
        }
        let mut col = Column::from_cells(values);
        col.grow(self.height);
        self.cols[c] = col;
    }

    /// Columns kept as text stored typed where that's lossless.
    fn retype(&mut self) {
        for c in &mut self.cols {
            if let Some(t) = c.retyped() {
                *c = t;
            }
        }
    }
}

/// Rows gathered cell by cell into a frame of their own, each column
/// stored as lossless as can be once it's finished (`i64`, `f64` … or
/// text) — a background query's chunk, built off the program's thread
/// (rapidr-db) and then [`Frame::append`]ed.
pub struct FrameBuilder {
    columns: Vec<String>,
    cols: Vec<column::Builder>,
    rows: usize,
}

impl FrameBuilder {
    /// No rows yet, these columns (named as [`Frame::with_columns`] names
    /// them).
    pub fn new<S: AsRef<str>>(names: &[S], rows_hint: usize) -> FrameBuilder {
        let columns = Frame::with_columns(names).columns;
        let cols = columns.iter().map(|_| column::Builder::new(rows_hint)).collect();
        FrameBuilder { columns, cols, rows: 0 }
    }

    /// Cell `col` of the row being added (None: null). Every column gets
    /// one before [`FrameBuilder::end_row`].
    #[inline]
    pub fn push(&mut self, col: usize, cell: Option<&str>) {
        if let Some(b) = self.cols.get_mut(col) {
            b.push(cell);
        }
    }

    /// The row's cells are in.
    #[inline]
    pub fn end_row(&mut self) {
        self.rows += 1;
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn width(&self) -> usize {
        self.columns.len()
    }

    /// The frame (and a new builder for the next rows, the same columns).
    pub fn finish(&mut self, rows_hint: usize) -> Frame {
        let cols = std::mem::replace(&mut self.cols, self.columns.iter().map(|_| column::Builder::new(rows_hint)).collect());
        let height = std::mem::take(&mut self.rows);
        Frame { columns: self.columns.clone(), cols: cols.into_iter().map(column::Builder::finish).collect(), height }
    }
}

fn field_cell((text, quoted): (String, bool)) -> Option<String> {
    field_value(&text, quoted).map(str::to_string)
}

// ---------------------------------------------------------------------------
// The frames
// ---------------------------------------------------------------------------

thread_local! {
    static FRAMES: RefCell<HashMap<String, Frame>> = RefCell::new(HashMap::new());
    /// Each frame's version: the counter's value when it last changed
    /// ([`version`]).
    static VERSIONS: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
    /// Changes so far, every frame's (versions never repeat, even for a
    /// frame removed and made again).
    static CHANGES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// Why each frame's last file read or write failed (its `Error`; none
    /// when it worked).
    static IO_ERRORS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// Frame `name`'s last file read or write: `Err(why)` kept as its `Error`
/// (a file named in plain words), `Ok` clears it.
fn io_result(name: &str, file: &str, r: &Result<(), String>) {
    IO_ERRORS.with(|m| {
        let mut m = m.borrow_mut();
        match r {
            Ok(()) => {
                m.remove(&key(name));
            }
            Err(e) => {
                m.insert(key(name), plain_io_error(file, e));
            }
        }
    });
}

/// A file error in plain words: "can't write staff_edited.csv: the folder
/// can't be written (read-only)".
fn plain_io_error(file: &str, e: &str) -> String {
    let low = e.to_ascii_lowercase();
    let why = if low.contains("permission denied") || low.contains("os error 13") || low.contains("access is denied") || low.contains("os error 5)") {
        "this folder can't be written (it is read-only, or not yours)".to_string()
    } else if low.contains("read-only file system") || low.contains("os error 30") {
        "the disk is read-only".to_string()
    } else if low.contains("no space") || low.contains("os error 28") {
        "the disk is full".to_string()
    } else if low.contains("no such file") || low.contains("os error 2)") || low.contains("not found") {
        "there is no such file or folder".to_string()
    } else {
        e.to_string()
    };
    if file.is_empty() || why.contains(file) {
        why
    } else {
        format!("{file}: {why}")
    }
}

/// Frame `name`'s `Error`: why its last file read or write failed ("" when
/// it worked).
pub fn io_error(name: &str) -> String {
    IO_ERRORS.with(|m| m.borrow().get(&key(name)).cloned().unwrap_or_default())
}

/// Frame `name` changed: a new version (and the data links are told:
/// [`super::link`]).
fn changed(name: &str) {
    let v = CHANGES.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    VERSIONS.with(|m| m.borrow_mut().insert(key(name), v));
    super::link::frame_changed(name);
}

/// Frame `name` (empty if it has none yet).
pub fn frame(name: &str) -> Frame {
    read(name, Frame::clone)
}

/// Frame `name` becomes `f`.
pub fn set_frame(name: &str, f: Frame) {
    FRAMES.with(|m| {
        m.borrow_mut().insert(key(name), f);
    });
    changed(name);
}

/// Whether `name` has a frame (an RDATAFRAME, an RCSV's table …).
pub fn has_frame(name: &str) -> bool {
    FRAMES.with(|m| m.borrow().contains_key(&key(name)))
}

/// Frame `name` forgotten.
pub fn remove_frame(name: &str) {
    if FRAMES.with(|m| m.borrow_mut().remove(&key(name))).is_some() {
        changed(name);
    }
}

/// Frame `name`'s version: it changes whenever the frame does (0: never
/// set). What a grid or a bound control showed is current while it's the
/// same.
pub fn version(name: &str) -> u64 {
    VERSIONS.with(|m| m.borrow().get(&key(name)).copied().unwrap_or(0))
}

/// `f` of frame `name` (an empty frame if it has none), without a copy.
pub fn with_frame<R>(name: &str, f: impl FnOnce(&Frame) -> R) -> R {
    read(name, f)
}

/// `f` of frame `name`, changed in place (a new version).
pub fn with_frame_mut<R>(name: &str, f: impl FnOnce(&mut Frame) -> R) -> R {
    modify(name, f)
}

fn read<R>(name: &str, f: impl FnOnce(&Frame) -> R) -> R {
    FRAMES.with(|m| match m.borrow().get(&key(name)) {
        Some(frame) => f(frame),
        None => f(&Frame::default()),
    })
}

fn modify<R>(name: &str, f: impl FnOnce(&mut Frame) -> R) -> R {
    let r = FRAMES.with(|m| f(m.borrow_mut().entry(key(name)).or_default()));
    changed(name);
    r
}

/// A file's text: `path` as given, or (on the desktop) next to the program
/// or under RAPIDR_HOME, as the examples' data files are found.
pub(crate) fn read_text(path: &str) -> Result<String, String> {
    #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
    let mut tries = vec![path.to_string()];
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.to_path_buf())) {
            tries.push(dir.join(path).to_string_lossy().into_owned());
        }
        if let Ok(home) = std::env::var("RAPIDR_HOME") {
            let home = std::path::Path::new(&home);
            tries.push(home.join("examples").join(path).to_string_lossy().into_owned());
            tries.push(home.join(path).to_string_lossy().into_owned());
        }
    }
    let mut first_error = None;
    for t in &tries {
        match crate::objects::read_file(t) {
            Ok(bytes) => return Ok(String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())),
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    Err(first_error.unwrap_or_else(|| format!("can't read {path}")))
}

/// What `LoadFromCSV` / `LoadFromJSON` read: the file `source` names, or
/// `source` itself when it is the data (it has a line break, or isn't a
/// file and looks like data).
fn load_source<'a>(source: &'a str, looks_like_data: impl Fn(&str) -> bool) -> Result<Cow<'a, str>, String> {
    if source.contains('\n') {
        return Ok(Cow::Borrowed(source));
    }
    match read_text(source) {
        Ok(text) => Ok(Cow::Owned(text)),
        Err(_) if looks_like_data(source) => Ok(Cow::Borrowed(source)),
        Err(e) => Err(e),
    }
}

/// Argument `i` as text, borrowed when it is text (a big CSV passed as a
/// string isn't copied).
fn arg_text(args: &[Value], i: usize) -> Cow<'_, str> {
    match args.get(i) {
        Some(Value::String(s)) => Cow::Borrowed(s.as_str()),
        _ => Cow::Owned(arg_s(args, i)),
    }
}

pub(crate) fn write_text(path: &str, text: &str) -> Result<(), String> {
    crate::objects::write_file(path, text.as_bytes())
}

/// The order `f`'s rows sort in by `keys` (column, descending), stably;
/// nulls last.
fn sorted_rows(f: &Frame, keys: &[(usize, bool)]) -> Vec<usize> {
    kernels::sort_rows(f, keys, None)
}

/// A comparison `filter` makes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    Contains,
    Starts,
    Ends,
}

fn op_of(op: &str) -> Option<Op> {
    Some(match op.trim().to_ascii_lowercase().as_str() {
        "==" | "=" => Op::Eq,
        "!=" | "<>" => Op::Ne,
        ">" => Op::Gt,
        "<" => Op::Lt,
        ">=" => Op::Ge,
        "<=" => Op::Le,
        "contains" => Op::Contains,
        "startswith" => Op::Starts,
        "endswith" => Op::Ends,
        _ => return None,
    })
}

/// `"age > 30"` → ("age", ">", "30"); quotes around the value go.
fn split_query(q: &str) -> Option<(String, String, String)> {
    let lower = q.to_ascii_lowercase();
    if let Some(at) = [" contains ", " startswith ", " endswith "].iter().find_map(|w| lower.find(w).map(|i| (i, w.len()))) {
        let (i, n) = at;
        return Some((q[..i].to_string(), q[i..i + n].trim().to_string(), q[i + n..].to_string()));
    }
    for op in [">=", "<=", "==", "!=", "<>", ">", "<", "="] {
        if let Some(i) = q.find(op) {
            return Some((q[..i].to_string(), op.to_string(), q[i + op.len()..].to_string()));
        }
    }
    None
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    for q in ['"', '\'', '`'] {
        if t.len() >= 2 && t.starts_with(q) && t.ends_with(q) {
            return t[1..t.len() - 1].to_string();
        }
    }
    t.to_string()
}

pub(crate) fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

/// The population standard deviation (RNUM's `std`).
pub(crate) fn std_dev(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let m = mean(v);
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt()
}

/// The `p` quantile (0 … 1) of sorted `s`, interpolated between neighbours
/// as pandas'.
fn quantile_sorted(s: &[f64], p: f64) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let pos = p * (s.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    s[lo] + (s[hi] - s[lo]) * (pos - lo as f64)
}

fn sort_floats(v: &mut [f64]) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
}

/// The `p` quantile (0 … 1), interpolated between neighbours as pandas'.
pub(crate) fn percentile(v: &[f64], p: f64) -> f64 {
    let mut s = v.to_vec();
    sort_floats(&mut s);
    quantile_sorted(&s, p)
}

/// `apply`'s operations on one cell.
fn apply_op(cell: Option<&str>, op: &str, decimals: i32) -> Option<Option<String>> {
    let Some(s) = cell else { return Some(None) };
    let num = |f: fn(f64) -> f64| Some(parse_num(s).map(|x| num_text(f(x))).filter(|t| t != "NAN"));
    match op {
        "upper" => Some(Some(s.to_uppercase())),
        "lower" => Some(Some(s.to_lowercase())),
        "trim" | "strip" => Some(Some(s.trim().to_string())),
        "abs" => num(f64::abs),
        "sqrt" => num(f64::sqrt),
        "log" | "ln" => num(f64::ln),
        "exp" => num(f64::exp),
        "round" => {
            let factor = 10f64.powi(decimals);
            Some(parse_num(s).map(|x| num_text((x * factor).round() / factor)))
        }
        _ => None,
    }
}

/// Calls RDATAFRAME `name`'s `method` (lowercase).
pub fn dataframe_method(name: &str, method: &str, args: &[Value], host: &dyn Host) -> Value {
    let warn = |what: &str| host.warn(&format!("[ERROR] RDataFrame.{method}: {what}"));
    let missing = |col: &str| host.warn(&format!("[ERROR] RDataFrame.{method}: no column \"{col}\""));
    match method {
        // --- Creation ---
        "create" | "new" | "init" => {
            set_frame(name, Frame::default());
            v_null()
        }
        "addrow" | "add_row" | "appendrow" | "push_row" => {
            // (one cell an argument, or the row as one CSV line)
            let mut cells: Vec<Option<String>> = args.iter().map(|v| field_cell((v.to_string_val(), false))).collect();
            if args.len() == 1 {
                let text = args[0].to_string_val();
                if text.contains(',') && read(name, |f| f.width() != 1) {
                    cells = csv_records(&text).into_iter().next().unwrap_or_default().into_iter().map(field_cell).collect();
                }
            }
            modify(name, |f| f.push_row(&cells));
            v_null()
        }

        // --- I/O ---
        // (each one's failure kept as the frame's Error too: a program shows
        // why — "can't write staff_edited.csv: …" — instead of nothing)
        "loadfromcsv" | "readcsv" | "read_csv" => {
            let source = arg_text(args, 0);
            let r = load_source(&source, |s| s.contains(',')).map(|text| set_frame(name, parse_csv(&text)));
            io_result(name, if source.contains(',') { "" } else { &source }, &r);
            if let Err(e) = r {
                warn(&e);
            }
            v_null()
        }
        "savetocsv" | "to_csv" | "writecsv" => {
            let path = arg_s_or(args, 0, "output.csv");
            let r = write_text(&path, &read(name, to_csv));
            io_result(name, &path, &r);
            if let Err(e) = r {
                warn(&e);
            }
            v_null()
        }
        "loadfromjson" | "read_json" => {
            let source = arg_text(args, 0);
            let r = match load_source(&source, |s| s.trim_start().starts_with(['[', '{'])).map(|t| parse_json_frame(&t)) {
                Ok(Some(f)) => {
                    set_frame(name, f);
                    Ok(())
                }
                Ok(None) => Err("not JSON records".to_string()),
                Err(e) => Err(e),
            };
            io_result(name, if source.trim_start().starts_with(['[', '{']) { "" } else { &source }, &r);
            if let Err(e) = r {
                warn(&e);
            }
            v_null()
        }
        "savetojson" | "to_json" => {
            let path = arg_s_or(args, 0, "output.json");
            let r = write_text(&path, &read(name, to_json));
            io_result(name, &path, &r);
            if let Err(e) = r {
                warn(&e);
            }
            v_null()
        }

        // --- Selection / Indexing ---
        "head" => {
            let n = arg_i(args, 0, 5).max(0) as usize;
            modify(name, |f| f.truncate(n));
            v_null()
        }
        "tail" => {
            let n = arg_i(args, 0, 5).max(0) as usize;
            modify(name, |f| {
                let skip = f.height.saturating_sub(n);
                if skip > 0 {
                    let rows: Vec<usize> = (skip..f.height).collect();
                    f.keep(&rows);
                }
            });
            v_null()
        }
        "cell" => {
            // cell(row, column): the column by its index or its name
            let row = arg_i(args, 0, 0);
            read(name, |f| {
                let col = f.col_of(args.get(1).or(Some(&Value::Integer(0))));
                v_str(&match (usize::try_from(row).ok(), col) {
                    (Some(r), Some(c)) => f.cell(r, c).unwrap_or_default(),
                    _ => Cow::Borrowed(""),
                })
            })
        }
        "cellbyname" | "at" => {
            let row = arg_i(args, 0, 0);
            let col = arg_s(args, 1);
            read(name, |f| {
                v_str(&match (usize::try_from(row).ok(), f.col(&col)) {
                    (Some(r), Some(c)) => f.cell(r, c).unwrap_or_default(),
                    _ => Cow::Borrowed(""),
                })
            })
        }
        "setcell" => {
            // setcell(row, column, value): past the end the frame grows
            let row = arg_i(args, 0, 0);
            let value = field_cell((arg_s(args, 2), false));
            let Ok(r) = usize::try_from(row) else { return v_null() };
            modify(name, |f| {
                let c = match args.get(1) {
                    Some(Value::String(s)) if s.trim().parse::<usize>().is_err() => Some(f.col(s).unwrap_or_else(|| f.add_column(s))),
                    other => {
                        let c = other.map(|v| arg_i(std::slice::from_ref(v), 0, 0)).unwrap_or(0);
                        usize::try_from(c).ok().filter(|c| *c < 10_000)
                    }
                };
                if let Some(c) = c {
                    if r < 10_000_000 {
                        f.grow(r + 1, c + 1);
                        f.cols[c].set(r, value.as_deref());
                    }
                }
            });
            v_null()
        }
        "iloc" => {
            let start = arg_i(args, 0, 0).max(0) as usize;
            modify(name, |f| {
                let end = (arg_i(args, 1, f.height() as i64).max(0) as usize).min(f.height());
                let start = start.min(end);
                let rows: Vec<usize> = (start..end).collect();
                f.keep(&rows);
            });
            v_null()
        }
        "select" => {
            // select "a,b": those columns, in that order
            let wanted: Vec<String> = arg_s(args, 0).split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            let picked: Result<Vec<usize>, String> = read(name, |f| wanted.iter().map(|w| f.col(w).ok_or_else(|| w.clone())).collect());
            match picked {
                Ok(idx) => modify(name, |f| {
                    f.columns = idx.iter().map(|&i| f.columns[i].clone()).collect();
                    f.cols = idx.iter().map(|&i| f.cols[i].clone()).collect();
                }),
                Err(col) => missing(&col),
            }
            v_null()
        }

        // --- Sorting ---
        "sort" | "sort_values" => {
            // sort "a,b", ascending (1, the default) or descending (0)
            let ascending = arg_i(args, 1, 1) != 0;
            let cols: Vec<String> = arg_s(args, 0).split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            let keys: Result<Vec<(usize, bool)>, String> = read(name, |f| cols.iter().map(|c| f.col(c).map(|i| (i, !ascending)).ok_or_else(|| c.clone())).collect());
            match keys {
                Ok(keys) => modify(name, |f| {
                    let order = sorted_rows(f, &keys);
                    f.keep(&order);
                }),
                Err(col) => missing(&col),
            }
            v_null()
        }

        // --- Filtering ---
        "filter" | "query" => {
            // filter column, op, value — or query "column op value"
            let (col, op, value) = if method == "query" || args.len() == 1 {
                match split_query(&arg_s(args, 0)) {
                    Some((c, o, v)) => (unquote(&c), o, unquote(&v)),
                    None => {
                        warn("expected \"column op value\"");
                        return v_null();
                    }
                }
            } else {
                (arg_s(args, 0), arg_s(args, 1), arg_s(args, 2))
            };
            let Some(c) = read(name, |f| f.col(&col)) else {
                missing(&col);
                return v_null();
            };
            let Some(op) = op_of(&op) else {
                warn(&format!("unknown operator \"{op}\""));
                return v_null();
            };
            let value_num = parse_num(&value);
            modify(name, |f| {
                let rows = kernels::filter_rows(&f.cols[c], op, &value, value_num);
                if rows.len() != f.height {
                    f.keep(&rows);
                }
            });
            v_null()
        }

        // --- Grouping / Aggregation ---
        "groupby" | "group_by" => {
            // groupby column, aggregate (mean, sum, count, min, max, first,
            // last, median, std) of every other column; a group a row
            let col = arg_s(args, 0);
            let agg = arg_s_or(args, 1, "mean").trim().to_ascii_lowercase();
            let Some(g) = read(name, |f| f.col(&col)) else {
                missing(&col);
                return v_null();
            };
            modify(name, |f| *f = kernels::groupby(f, g, &agg));
            v_null()
        }

        // --- Column operations ---
        "drop" | "drop_column" => {
            let col = arg_s(args, 0);
            match read(name, |f| f.col(&col)) {
                Some(c) => modify(name, |f| {
                    f.columns.remove(c);
                    f.cols.remove(c);
                }),
                None => missing(&col),
            }
            v_null()
        }
        "rename" | "rename_column" => {
            let old = arg_s(args, 0);
            let new = arg_s(args, 1).trim().to_string();
            match read(name, |f| f.col(&old)) {
                Some(c) => modify(name, |f| f.columns[c] = new),
                None => missing(&old),
            }
            v_null()
        }
        "addcolumn" | "add_column" | "set_column" => {
            // addcolumn name, "v1,v2,…" (a column of that name is replaced);
            // a frame with no rows gets one a value
            let col = arg_s(args, 0);
            let mut values: Vec<Option<String>> = match args.get(1) {
                Some(v) => {
                    let text = v.to_string_val();
                    if text.is_empty() {
                        Vec::new()
                    } else {
                        csv_records(&text).into_iter().next().unwrap_or_default().into_iter().map(field_cell).collect()
                    }
                }
                None => Vec::new(),
            };
            modify(name, |f| {
                if f.height() == 0 {
                    f.grow(values.len(), 0);
                }
                let c = f.col(&col).filter(|_| !col.trim().is_empty()).unwrap_or_else(|| f.add_column(&col));
                values.truncate(f.height);
                f.set_column(c, values);
            });
            v_null()
        }

        // --- Missing data ---
        "fillna" | "fill_null" => {
            let fill = arg_s_or(args, 0, "0");
            modify(name, |f| {
                for c in &mut f.cols {
                    if c.valid_count() < c.len() {
                        *c = c.map(|v| Some(v.map_or_else(|| fill.clone(), str::to_string)));
                    }
                }
            });
            v_null()
        }
        "dropna" | "drop_nulls" => {
            modify(name, |f| {
                let rows: Vec<usize> = (0..f.height).filter(|&r| f.cols.iter().all(|c| c.is_valid(r))).collect();
                if rows.len() != f.height {
                    f.keep(&rows);
                }
            });
            v_null()
        }

        // --- Statistics ---
        "describe" => {
            // the frame becomes the summary of its numeric columns
            modify(name, |f| *f = kernels::describe(f));
            v_null()
        }
        "value_counts" => {
            // the frame becomes each value of the column and its count, most first
            let col = arg_s(args, 0);
            let Some(c) = read(name, |f| f.col(&col)) else {
                missing(&col);
                return v_null();
            };
            modify(name, |f| *f = kernels::value_counts(f, c));
            v_null()
        }
        "nunique" => {
            let col = arg_s(args, 0);
            read(name, |f| match f.col(&col) {
                Some(c) => v_int(kernels::nunique(&f.cols[c]) as i64),
                None => v_int(0),
            })
        }
        "corr" | "correlation" => {
            // Pearson's correlation of two columns (rows where both are numbers)
            let (a, b) = (arg_s(args, 0), arg_s(args, 1));
            read(name, |f| {
                let (Some(ca), Some(cb)) = (f.col(&a), f.col(&b)) else { return v_dbl(0.0) };
                let (na, nb) = (f.cols[ca].nums(), f.cols[cb].nums());
                let pairs: Vec<(f64, f64)> = (0..f.height).filter(|&i| !na[i].is_nan() && !nb[i].is_nan()).map(|i| (na[i], nb[i])).collect();
                let n = pairs.len() as f64;
                if n < 2.0 {
                    return v_dbl(0.0);
                }
                let ma = pairs.iter().map(|p| p.0).sum::<f64>() / n;
                let mb = pairs.iter().map(|p| p.1).sum::<f64>() / n;
                let cov: f64 = pairs.iter().map(|(x, y)| (x - ma) * (y - mb)).sum();
                let sa = pairs.iter().map(|(x, _)| (x - ma).powi(2)).sum::<f64>().sqrt();
                let sb = pairs.iter().map(|(_, y)| (y - mb).powi(2)).sum::<f64>().sqrt();
                v_dbl(if sa > 0.0 && sb > 0.0 { cov / (sa * sb) } else { 0.0 })
            })
        }

        // --- Sampling ---
        "sample" => {
            // n rows at random (no row twice)
            let n = arg_i(args, 0, 5).max(0) as usize;
            modify(name, |f| {
                let mut rows: Vec<usize> = (0..f.height).collect();
                let n = n.min(rows.len());
                for i in 0..n {
                    let j = i + random_index(rows.len() - i);
                    rows.swap(i, j);
                }
                rows.truncate(n);
                f.keep(&rows);
            });
            v_null()
        }
        "nlargest" | "nsmallest" => {
            let col = arg_s(args, 0);
            let n = arg_i(args, 1, 5).max(0) as usize;
            match read(name, |f| f.col(&col)) {
                Some(c) => modify(name, |f| {
                    let mut order = sorted_rows(f, &[(c, method == "nlargest")]);
                    order.truncate(n);
                    f.keep(&order);
                }),
                None => missing(&col),
            }
            v_null()
        }

        // --- Shape / info ---
        "info" => {
            let text = read(name, |f| {
                let mut lines = vec![format!("RDataFrame: {} rows x {} columns", f.height(), f.width())];
                for c in 0..f.width() {
                    let present = f.cols[c].valid_count();
                    lines.push(format!("  {}: {} ({} non-null)", f.columns[c], f.dtype(c).name(), present));
                }
                lines.join("\n")
            });
            host.print(&text);
            v_str(&text)
        }
        "dtypes" => v_str(&read(name, |f| (0..f.width()).map(|c| format!("{}: {}", f.columns[c], f.dtype(c).name())).collect::<Vec<_>>().join(","))),
        "shape" => read(name, |f| v_str(&format!("({}, {})", f.height(), f.width()))),

        // --- Merge / Join ---
        "merge" | "join" => {
            // merge other, on, how (inner, left, right, outer, cross)
            let other = frame(&arg_s(args, 0));
            let on = arg_s(args, 1);
            let how = arg_s_or(args, 2, "inner").trim().to_ascii_lowercase();
            let joined = read(name, |this| kernels::join(this, &other, &on, &how));
            match joined {
                Ok(f) => set_frame(name, f),
                Err(e) => warn(&e),
            }
            v_null()
        }
        "concat" | "append" => {
            // the other frame's rows after these (columns matched by name)
            let other = frame(&arg_s(args, 0));
            modify(name, |f| {
                let idx: Vec<usize> = other.columns.iter().map(|c| f.columns.iter().position(|x| x == c).unwrap_or_else(|| f.add_column(c))).collect();
                let h = f.height + other.height;
                for (i, &c) in idx.iter().enumerate() {
                    let src = &other.cols[i];
                    f.cols[c].append(&src.slice(0..other.height.min(src.len())));
                    let len = f.cols[c].len();
                    f.cols[c].grow(len + other.height.saturating_sub(src.len()));
                }
                for c in &mut f.cols {
                    c.grow(h);
                }
                f.height = h;
            });
            v_null()
        }

        // --- Transform ---
        "transpose" | "t" => {
            // the columns become rows: a "column" column with the names, then
            // one column a former row ("0", "1" …)
            modify(name, |f| {
                if f.width() == 0 {
                    return;
                }
                let mut out = Frame::default();
                let names = out.add_column("column");
                out.set_column(names, f.columns.iter().map(|c| Some(c.clone())).collect());
                for r in 0..f.height() {
                    let c = out.add_column(&r.to_string());
                    let cells: Vec<Option<String>> = f.cols.iter().map(|col| col.get(r).map(Cow::into_owned)).collect();
                    out.set_column(c, cells);
                }
                *f = out;
            });
            v_null()
        }
        "apply" => {
            // apply column, op: upper, lower, trim, abs, round [decimals, 2],
            // sqrt, log, exp
            let col = arg_s(args, 0);
            let op = arg_s(args, 1).trim().to_ascii_lowercase();
            let decimals = arg_i(args, 2, 2).clamp(-15, 15) as i32;
            let Some(c) = read(name, |f| f.col(&col)) else {
                missing(&col);
                return v_null();
            };
            if apply_op(Some("1"), &op, decimals).is_none() {
                host.warn(&format!("[WARN] RDataFrame.apply: unknown op '{op}'"));
                return v_null();
            }
            modify(name, |f| f.cols[c] = f.cols[c].map(|v| apply_op(v, &op, decimals).flatten()));
            v_null()
        }
        "replace" => {
            // replace column, old, new: cells that are `old` become `new`
            let col = arg_s(args, 0);
            let (old, new) = (arg_s(args, 1), arg_s(args, 2));
            let Some(c) = read(name, |f| f.col(&col)) else {
                missing(&col);
                return v_null();
            };
            modify(name, |f| f.cols[c] = f.cols[c].map(|v| if v == Some(old.as_str()) { field_cell((new.clone(), true)) } else { v.map(str::to_string) }));
            v_null()
        }

        // --- Display / Info ---
        "clear" => {
            set_frame(name, Frame::default());
            v_null()
        }
        "columns" => v_str(&read(name, |f| f.columns.join(","))),
        "rows" | "rowcount" | "len" => v_int(read(name, Frame::height) as i64),
        "tostring" => v_str(&read(name, Frame::to_text)),
        "show" | "print" => {
            let text = read(name, Frame::to_text);
            host.print(&text);
            v_str(&text)
        }

        // --- Populate RStringGrid ---
        "togrid" | "to_grid" | "display" => {
            // the grid gets the header row and the cells
            let grid = arg_s(args, 0);
            if !grid.trim().is_empty() {
                host.to_grid(grid.trim(), &read(name, Frame::grid_rows));
            }
            v_null()
        }

        _ => {
            // (a property read the way methods are)
            let v = dataframe_get_prop(name, method);
            if matches!(v, Value::Null) {
                host.warn(&format!("[WARN] RDataFrame.{method}() not implemented"));
            }
            v
        }
    }
}

/// Reads RDATAFRAME `name`'s property `prop` (lowercase); Null if it has
/// none.
pub fn dataframe_get_prop(name: &str, prop: &str) -> Value {
    if prop == "error" {
        return v_str(&io_error(name));
    }
    read(name, |f| match prop {
        "rowcount" | "height" | "nrows" | "rows" => v_int(f.height() as i64),
        "colcount" | "width" | "ncols" => v_int(f.width() as i64),
        "columns" => v_str(&f.columns.join(",")),
        "shape" => v_str(&format!("({}, {})", f.height(), f.width())),
        "empty" => v_int(if f.height() == 0 { 1 } else { 0 }),
        "dtypes" => v_str(&(0..f.width()).map(|c| format!("{}: {}", f.columns[c], f.dtype(c).name())).collect::<Vec<_>>().join(",")),
        _ => v_null(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::test_host::TestHost;
    use super::*;

    const PEOPLE: &str = "name,age,city\nbob,30,NYC\namy,25,LA\n\"Lee, Jo\",41,NYC\ncat,,LA\n";

    /// Chunks of rows appended (a background query's) read exactly as the
    /// same cells gathered at once: each cell's text and each column's
    /// type, whatever the chunks' storages (nulls, whole numbers then
    /// fractions, numbers then text, Booleans).
    #[test]
    fn appended_chunks_read_as_one_frame() {
        let chunks: Vec<Vec<[Option<&str>; 4]>> = vec![
            vec![[Some("1"), None, Some("true"), None], [Some("2"), None, Some("false"), Some("x")]],
            vec![[Some("3"), Some("2.5"), Some("true"), Some("4")]],
            vec![[None, None, None, None]],
            vec![[Some("4.5"), Some("7"), Some("maybe"), Some("5")], [Some("5"), Some("1e3"), Some("true"), None]],
        ];
        let names = ["a", "b", "c", "d"];
        let mut whole = Frame::with_columns(&names);
        let mut all: Vec<[Option<&str>; 4]> = Vec::new();
        for rows in &chunks {
            let mut b = FrameBuilder::new(&names, rows.len());
            for row in rows {
                for (c, cell) in row.iter().enumerate() {
                    b.push(c, *cell);
                }
                b.end_row();
            }
            whole.append(&b.finish(0));
            all.extend(rows.iter().copied());
        }
        let mut once = Frame::with_columns(&names);
        for row in &all {
            once.push_row(&row.iter().map(|c| c.map(str::to_string)).collect::<Vec<_>>());
        }
        once.retype();
        assert_eq!(whole.height(), all.len());
        for c in 0..4 {
            assert_eq!(whole.dtype(c), once.dtype(c), "column {c}");
            for r in 0..all.len() {
                assert_eq!(whole.cell(r, c), once.cell(r, c), "({r}, {c})");
                assert_eq!(whole.cell(r, c).as_deref(), all[r][c].map(|s| if s == "1e3" { "1e3" } else { s }), "({r}, {c})");
            }
        }
        // (a duplicate name, an empty one)
        assert_eq!(Frame::with_columns(&["id", "id", ""]).columns, ["id", "id_2", "column_3"]);
    }

    fn call(h: &TestHost, name: &str, m: &str, args: &[Value]) -> Value {
        dataframe_method(name, m, args, h)
    }

    fn load(name: &str) -> TestHost {
        let h = TestHost::default();
        set_frame(name, parse_csv(PEOPLE));
        h
    }

    #[test]
    fn csv_reads_quotes_and_nulls() {
        let f = parse_csv(PEOPLE);
        assert_eq!(f.columns, ["name", "age", "city"]);
        assert_eq!(f.height(), 4);
        assert_eq!(f.cell(2, 0).as_deref(), Some("Lee, Jo"));
        assert_eq!(f.cell(3, 1).as_deref(), None);
        assert_eq!(f.dtype(1), DType::Int);
        assert_eq!(f.dtype(0), DType::Str);
        assert_eq!(parse_csv(&to_csv(&f)), f);
    }

    #[test]
    fn cells_are_plain_text() {
        let h = load("p1");
        assert_eq!(call(&h, "p1", "cell", &[v_int(0), v_int(0)]).to_string_val(), "bob");
        assert_eq!(call(&h, "p1", "cellbyname", &[v_int(1), v_str("CITY")]).to_string_val(), "LA");
        assert_eq!(call(&h, "p1", "cell", &[v_int(3), v_int(1)]).to_string_val(), "");
    }

    #[test]
    fn prints_once_as_a_table() {
        let h = load("p2");
        let s = call(&h, "p2", "tostring", &[]).to_string_val();
        assert!(h.printed.borrow().is_empty());
        assert_eq!(
            s,
            "name      age  city\n-------  ----  ----\nbob        30  NYC\namy        25  LA\nLee, Jo    41  NYC\ncat      null  LA\n[4 rows x 3 columns]"
        );
        call(&h, "p2", "print", &[]);
        assert_eq!(h.printed.borrow().len(), 1);
    }

    #[test]
    fn filter_sort_group() {
        let h = load("p3");
        call(&h, "p3", "filter", &[v_str("age"), v_str(">"), v_int(26)]);
        assert_eq!(dataframe_get_prop("p3", "rowcount").to_i64(), 2);
        let h = load("p3");
        call(&h, "p3", "sort", &[v_str("age"), v_int(0)]);
        assert_eq!(frame("p3").cell(0, 1).as_deref(), Some("41"));
        assert_eq!(frame("p3").cell(3, 1).as_deref(), None);
        let h = load("p3");
        call(&h, "p3", "groupby", &[v_str("city"), v_str("mean")]);
        let f = frame("p3");
        assert_eq!(f.columns, ["city", "name", "age"]);
        assert_eq!(f.cell(0, 0).as_deref(), Some("LA"));
        assert_eq!(f.cell(0, 2).as_deref(), Some("25"));
        assert_eq!(f.cell(1, 2).as_deref(), Some("35.5"));
        assert_eq!(f.cell(0, 1).as_deref(), None);
        let h = load("p3");
        call(&h, "p3", "query", &[v_str("city == 'NYC'")]);
        assert_eq!(dataframe_get_prop("p3", "rowcount").to_i64(), 2);
    }

    #[test]
    fn json_round_trip() {
        let f = parse_csv(PEOPLE);
        let j = to_json(&f);
        assert!(j.starts_with("[\n{\"name\":\"bob\",\"age\":30,\"city\":\"NYC\"},"));
        assert_eq!(parse_json_frame(&j).unwrap(), f);
        let lines = "{\"a\":1,\"b\":\"x\"}\n{\"a\":2,\"c\":true}\n";
        let g = parse_json_frame(lines).unwrap();
        assert_eq!(g.columns, ["a", "b", "c"]);
        assert_eq!(g.cell(1, 2).as_deref(), Some("true"));
    }

    #[test]
    fn build_by_hand_and_merge() {
        let h = TestHost::default();
        call(&h, "l", "create", &[]);
        call(&h, "l", "addcolumn", &[v_str("id"), v_str("1,2,3")]);
        call(&h, "l", "addcolumn", &[v_str("x"), v_str("a,b,c")]);
        call(&h, "r", "create", &[]);
        call(&h, "r", "addrow", &[v_str("id"), v_str("y")]);
        call(&h, "r", "rename", &[v_str("column_1"), v_str("id")]);
        call(&h, "r", "rename", &[v_str("column_2"), v_str("y")]);
        call(&h, "r", "clear", &[]);
        call(&h, "r", "setcell", &[v_int(0), v_str("id"), v_str("2")]);
        call(&h, "r", "setcell", &[v_int(0), v_str("y"), v_str("B")]);
        call(&h, "r", "setcell", &[v_int(1), v_int(0), v_str("4")]);
        call(&h, "r", "setcell", &[v_int(1), v_int(1), v_str("D")]);
        call(&h, "l", "merge", &[v_str("r"), v_str("id"), v_str("outer")]);
        let f = frame("l");
        assert_eq!(f.columns, ["id", "x", "y"]);
        assert_eq!(f.height(), 4);
        assert_eq!(f.cell(1, 2).as_deref(), Some("B"));
        assert_eq!(f.cell(3, 0).as_deref(), Some("4"));
    }

    #[test]
    fn describe_summarises_numbers() {
        let h = load("p4");
        call(&h, "p4", "describe", &[]);
        let f = frame("p4");
        assert_eq!(f.columns, ["statistic", "age"]);
        assert_eq!(f.cell(0, 1).as_deref(), Some("3"));
        assert_eq!(f.cell(1, 1).as_deref(), Some("32"));
        assert_eq!(f.cell(5, 1).as_deref(), Some("30"));
    }

    #[test]
    fn delimiters_and_headerless() {
        let f = parse_csv_opts("name;age\n\"a;b\";3\nc;\n", b';', true);
        assert_eq!(f.columns, ["name", "age"]);
        assert_eq!(f.cell(0, 0).as_deref(), Some("a;b"));
        assert_eq!(f.cell(1, 1), None);
        assert_eq!(parse_csv_opts(&to_csv_opts(&f, b';', true), b';', true), f);
        let t = parse_csv_opts("1\t2\n3\t\n", b'\t', false);
        assert_eq!(t.columns, ["column_1", "column_2"]);
        assert_eq!(t.height(), 2);
        assert_eq!(t.cell(1, 0).as_deref(), Some("3"));
        assert_eq!(t.cell(1, 1), None);
        assert_eq!(to_csv_opts(&t, b'\t', false), "1\t2\n3\t\n");
    }

    #[test]
    fn versions_move_with_every_change() {
        let h = TestHost::default();
        let v0 = version("vv");
        call(&h, "vv", "loadfromcsv", &[v_str(PEOPLE)]);
        let v1 = version("vv");
        assert!(v1 > v0);
        assert_eq!(with_frame("vv", Frame::height), 4);
        call(&h, "vv", "filter", &[v_str("age"), v_str(">"), v_int(26)]);
        assert!(version("vv") > v1);
        with_frame_mut("vv", |f| f.set_cell(0, 0, Some("zed")));
        assert_eq!(frame("vv").cell(0, 0).as_deref(), Some("zed"));
        let mut s = frame("vv");
        s.sort_by_column(1, true);
        assert_eq!(s.cell(0, 1).as_deref(), Some("41"));
        assert_eq!(s.head(1).height(), 1);
        assert!(has_frame("vv"));
        remove_frame("vv");
        assert!(!has_frame("vv"));
    }
}
