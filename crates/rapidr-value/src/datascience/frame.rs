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
//! **Columnar** (I7 / L-FRAME): each column keeps its cells' text in one
//! buffer (offsets and a null mask beside it, no allocation a cell), and
//! works out once — when first asked — its type and its cells as numbers;
//! filters, sorts, groups and joins then compare numbers and string slices
//! and move rows by index. A million-row CSV loads, filters, sorts, groups
//! and joins in tens to hundreds of milliseconds
//! (`cargo run --release -p rapidr-value --example frame_bench`).
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

use std::cell::{OnceCell, RefCell};
use std::cmp::Ordering;
use std::collections::HashMap;

use super::{arg_i, arg_s, arg_s_or, key, num_text, parse_num, random_index, Host};
use crate::{v_dbl, v_int, v_null, v_str, Value};

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

/// What a column is, worked out once: its type, and each cell as a number
/// (NaN where it's null or not a number — `parse_num` never gives NaN).
#[derive(Clone, Debug)]
struct Analysis {
    dtype: DType,
    nums: Vec<f64>,
}

/// One column: its cells' text end to end, where each ends, which are null.
#[derive(Clone, Debug, Default)]
pub struct Column {
    text: String,
    ends: Vec<usize>,
    nulls: Vec<bool>,
    analysis: OnceCell<Analysis>,
}

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && (0..self.len()).all(|i| self.get(i) == other.get(i))
    }
}

impl Column {
    fn with_capacity(rows: usize) -> Column {
        Column { text: String::new(), ends: Vec::with_capacity(rows), nulls: Vec::with_capacity(rows), analysis: OnceCell::new() }
    }

    /// `n` null cells.
    fn nulls(n: usize) -> Column {
        Column { text: String::new(), ends: vec![0; n], nulls: vec![true; n], analysis: OnceCell::new() }
    }

    pub fn len(&self) -> usize {
        self.nulls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nulls.is_empty()
    }

    /// Cell `i` (None: null, or past the end).
    pub fn get(&self, i: usize) -> Option<&str> {
        if *self.nulls.get(i)? {
            return None;
        }
        let start = if i == 0 { 0 } else { self.ends[i - 1] };
        Some(&self.text[start..self.ends[i]])
    }

    fn push(&mut self, cell: Option<&str>) {
        if let Some(s) = cell {
            self.text.push_str(s);
        }
        self.ends.push(self.text.len());
        self.nulls.push(cell.is_none());
        self.analysis = OnceCell::new();
    }

    fn iter(&self) -> impl Iterator<Item = Option<&str>> + '_ {
        (0..self.len()).map(move |i| self.get(i))
    }

    /// The cells at `rows`, in that order.
    fn take(&self, rows: &[usize]) -> Column {
        let mut out = Column::with_capacity(rows.len());
        for &r in rows {
            out.push(self.get(r));
        }
        out
    }

    /// The column with each cell `f` of the old one.
    fn map(&self, mut f: impl FnMut(Option<&str>) -> Option<String>) -> Column {
        let mut out = Column::with_capacity(self.len());
        for i in 0..self.len() {
            let v = f(self.get(i));
            out.push(v.as_deref());
        }
        out
    }

    /// Cell `row` becomes `cell` (in place: the text after it moves).
    fn set(&mut self, row: usize, cell: Option<&str>) {
        if row >= self.len() {
            self.grow(row);
            self.push(cell);
            return;
        }
        let start = if row == 0 { 0 } else { self.ends[row - 1] };
        let end = self.ends[row];
        let new = cell.unwrap_or("");
        self.text.replace_range(start..end, new);
        let delta = new.len() as isize - (end - start) as isize;
        if delta != 0 {
            for e in &mut self.ends[row..] {
                *e = (*e as isize + delta) as usize;
            }
        }
        self.nulls[row] = cell.is_none();
        self.analysis = OnceCell::new();
    }

    /// At least `n` cells (nulls added).
    fn grow(&mut self, n: usize) {
        if self.len() < n {
            let end = self.text.len();
            self.ends.resize(n, end);
            self.nulls.resize(n, true);
            self.analysis = OnceCell::new();
        }
    }

    fn truncate(&mut self, n: usize) {
        if n < self.len() {
            self.ends.truncate(n);
            self.nulls.truncate(n);
            self.text.truncate(self.ends.last().copied().unwrap_or(0));
            self.analysis = OnceCell::new();
        }
    }

    fn analysis(&self) -> &Analysis {
        self.analysis.get_or_init(|| {
            let mut t = None::<DType>;
            let mut is_str = false;
            let mut nums = Vec::with_capacity(self.len());
            for cell in self.iter() {
                let Some(cell) = cell else {
                    nums.push(f64::NAN);
                    continue;
                };
                let n = parse_num(cell);
                nums.push(n.unwrap_or(f64::NAN));
                if is_str {
                    continue;
                }
                let this = if n.is_some() && is_int(cell) {
                    DType::Int
                } else if n.is_some() {
                    DType::Float
                } else if cell.eq_ignore_ascii_case("true") || cell.eq_ignore_ascii_case("false") {
                    DType::Bool
                } else {
                    is_str = true;
                    continue;
                };
                t = Some(match (t, this) {
                    (None, x) => x,
                    (Some(a), b) if a == b => a,
                    (Some(DType::Int), DType::Float) | (Some(DType::Float), DType::Int) => DType::Float,
                    _ => {
                        is_str = true;
                        continue;
                    }
                });
            }
            Analysis { dtype: if is_str { DType::Str } else { t.unwrap_or(DType::Str) }, nums }
        })
    }

    pub fn dtype(&self) -> DType {
        self.analysis().dtype
    }

    /// Each cell as a number (NaN: null or not a number).
    fn nums(&self) -> &[f64] {
        &self.analysis().nums
    }
}

/// A whole number as i64 reads it (what makes a column `i64`).
fn is_int(s: &str) -> bool {
    s.trim().parse::<i64>().is_ok()
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

    pub fn cell(&self, row: usize, col: usize) -> Option<&str> {
        self.cols.get(col).and_then(|c| c.get(row))
    }

    /// Column `c`'s type.
    pub fn dtype(&self, c: usize) -> DType {
        self.cols[c].dtype()
    }

    /// Column `c`'s numbers (nulls and text skipped).
    fn numbers(&self, c: usize) -> Vec<f64> {
        self.cols[c].nums().iter().copied().filter(|x| !x.is_nan()).collect()
    }

    /// Row `r`'s cells.
    pub fn row(&self, r: usize) -> Vec<Option<&str>> {
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
            col.push(cells.get(c).cloned().flatten().as_deref());
        }
        self.height += 1;
    }

    /// Only the rows at `rows`, in that order.
    fn keep(&mut self, rows: &[usize]) {
        for c in &mut self.cols {
            *c = c.take(rows);
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
                    Some(r) => clip(self.cell(*r, c).unwrap_or("null")),
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
            .chain((0..self.height).map(|r| self.cols.iter().map(|c| c.get(r).unwrap_or("").to_string()).collect()))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// CSV and JSON
// ---------------------------------------------------------------------------

/// Cells that are null when read: an empty field, and the usual words for
/// a missing value.
fn null_word(s: &str) -> bool {
    matches!(s, "" | "NA" | "N/A" | "null" | "NULL" | "NaN")
}

/// A read field's cell: a quoted field as written, an unquoted one trimmed
/// (and null if it's empty or a missing-value word).
fn field_value(text: &str, quoted: bool) -> Option<&str> {
    if quoted {
        return Some(text);
    }
    let t = text.trim();
    (!null_word(t)).then_some(t)
}

fn field_cell((text, quoted): (String, bool)) -> Option<String> {
    field_value(&text, quoted).map(str::to_string)
}

/// Reads CSV (RFC 4180: quoted fields with `""` for a quote, line breaks
/// inside quotes), calling `field(text, quoted)` for each field and
/// `end()` after each record; blank lines are skipped.
fn csv_scan(text: &str, mut field: impl FnMut(&str, bool), mut end: impl FnMut()) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let b = text.as_bytes();
    let mut i = 0;
    let mut buf = String::new();
    while i < b.len() {
        // (a blank line: nothing)
        let line_end = b[i..].iter().position(|&c| c == b'\n' || c == b'\r').map_or(b.len(), |p| i + p);
        if text[i..line_end].trim().is_empty() {
            i = line_end;
            if i < b.len() && b[i] == b'\r' {
                i += 1;
            }
            if i < b.len() && b[i] == b'\n' {
                i += 1;
            }
            continue;
        }
        // one record
        loop {
            // (leading spaces before a quote belong to nothing)
            let start = i;
            let mut j = i;
            while j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
                j += 1;
            }
            if j < b.len() && b[j] == b'"' {
                buf.clear();
                let mut k = j + 1;
                loop {
                    let Some(p) = b[k..].iter().position(|&c| c == b'"') else {
                        buf.push_str(&text[k..]);
                        k = b.len();
                        break;
                    };
                    buf.push_str(&text[k..k + p]);
                    k += p + 1;
                    if k < b.len() && b[k] == b'"' {
                        buf.push('"');
                        k += 1;
                    } else {
                        break;
                    }
                }
                // (anything after the closing quote up to the comma: kept)
                let rest_end = b[k..].iter().position(|&c| c == b',' || c == b'\n' || c == b'\r').map_or(b.len(), |p| k + p);
                buf.push_str(&text[k..rest_end]);
                field(&buf, true);
                i = rest_end;
            } else {
                let e = b[start..].iter().position(|&c| c == b',' || c == b'\n' || c == b'\r').map_or(b.len(), |p| start + p);
                field(&text[start..e], false);
                i = e;
            }
            if i < b.len() && b[i] == b',' {
                i += 1;
                if i == b.len() {
                    field("", false);
                }
                continue;
            }
            break;
        }
        end();
        if i < b.len() && b[i] == b'\r' {
            i += 1;
        }
        if i < b.len() && b[i] == b'\n' {
            i += 1;
        }
    }
}

/// CSV records, each field with whether it was quoted.
fn csv_records(text: &str) -> Vec<Vec<(String, bool)>> {
    let records = RefCell::new(Vec::new());
    let record = RefCell::new(Vec::new());
    csv_scan(text, |f, q| record.borrow_mut().push((f.to_string(), q)), || records.borrow_mut().push(std::mem::take(&mut *record.borrow_mut())));
    records.into_inner()
}

/// A frame from CSV text whose first record is the header.
pub fn parse_csv(text: &str) -> Frame {
    let mut f = Frame::default();
    let mut header = true;
    let mut c = 0usize;
    let rows_guess = text.len() / 32 + 1;
    let f_cell = RefCell::new(&mut f);
    let col = RefCell::new(&mut c);
    let head = RefCell::new(&mut header);
    csv_scan(
        text,
        |field, quoted| {
            let mut f = f_cell.borrow_mut();
            let mut c = col.borrow_mut();
            if **head.borrow() {
                f.add_column(field);
                if **c == 0 {
                    for col in &mut f.cols {
                        col.ends.reserve(rows_guess);
                        col.nulls.reserve(rows_guess);
                    }
                }
            } else {
                if **c >= f.width() {
                    let h = f.height;
                    let n = f.width() + 1;
                    f.add_column(&format!("column_{n}"));
                    let last = f.cols.len() - 1;
                    f.cols[last].grow(h);
                }
                f.cols[**c].push(field_value(field, quoted));
            }
            **c += 1;
        },
        || {
            let mut f = f_cell.borrow_mut();
            let mut c = col.borrow_mut();
            if **head.borrow() {
                **head.borrow_mut() = false;
            } else {
                let h = f.height + 1;
                for col in f.cols.iter_mut().skip(**c) {
                    col.push(None);
                }
                f.height = h;
            }
            **c = 0;
        },
    );
    f
}

fn csv_field(s: &str) -> std::borrow::Cow<'_, str> {
    if s.is_empty() || s.contains([',', '"', '\n', '\r']) || s.trim() != s {
        format!("\"{}\"", s.replace('"', "\"\"")).into()
    } else {
        s.into()
    }
}

/// The frame as CSV (a header line, null cells empty).
pub fn to_csv(f: &Frame) -> String {
    let mut out = String::with_capacity(f.cols.iter().map(|c| c.text.len() + c.len()).sum::<usize>() + 64);
    out += &f.columns.iter().map(|c| csv_field(c)).collect::<Vec<_>>().join(",");
    out.push('\n');
    for r in 0..f.height {
        for (i, c) in f.cols.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            if let Some(s) = c.get(r) {
                out += &csv_field(s);
            }
        }
        out.push('\n');
    }
    out
}

/// A JSON value, numbers kept as written.
#[derive(Clone, Debug)]
enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

struct JsonParser<'a> {
    s: &'a str,
    i: usize,
}

impl JsonParser<'_> {
    fn ws(&mut self) {
        while self.s[self.i..].starts_with([' ', '\t', '\n', '\r']) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: char) -> bool {
        self.ws();
        if self.s[self.i..].starts_with(c) {
            self.i += c.len_utf8();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        if depth > 64 {
            return None;
        }
        self.ws();
        let rest = &self.s[self.i..];
        if rest.starts_with('{') {
            self.i += 1;
            let mut fields = Vec::new();
            if self.eat('}') {
                return Some(Json::Obj(fields));
            }
            loop {
                self.ws();
                let Some(Json::Str(k)) = self.string() else { return None };
                if !self.eat(':') {
                    return None;
                }
                fields.push((k, self.value(depth + 1)?));
                if self.eat(',') {
                    continue;
                }
                return self.eat('}').then_some(Json::Obj(fields));
            }
        }
        if rest.starts_with('[') {
            self.i += 1;
            let mut items = Vec::new();
            if self.eat(']') {
                return Some(Json::Arr(items));
            }
            loop {
                items.push(self.value(depth + 1)?);
                if self.eat(',') {
                    continue;
                }
                return self.eat(']').then_some(Json::Arr(items));
            }
        }
        if rest.starts_with('"') {
            return self.string();
        }
        for (word, v) in [("true", Json::Bool(true)), ("false", Json::Bool(false)), ("null", Json::Null)] {
            if rest.starts_with(word) {
                self.i += word.len();
                return Some(v);
            }
        }
        let n = rest.find(|c: char| !(c.is_ascii_digit() || "+-.eE".contains(c))).unwrap_or(rest.len());
        if n == 0 {
            return None;
        }
        self.i += n;
        Some(Json::Num(rest[..n].to_string()))
    }

    fn string(&mut self) -> Option<Json> {
        if !self.s[self.i..].starts_with('"') {
            return None;
        }
        self.i += 1;
        let mut out = String::new();
        let mut chars = self.s[self.i..].char_indices();
        while let Some((at, c)) = chars.next() {
            match c {
                '"' => {
                    self.i += at + 1;
                    return Some(Json::Str(out));
                }
                '\\' => {
                    let (_, e) = chars.next()?;
                    match e {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'u' => {
                            let hex: String = (0..4).filter_map(|_| chars.next().map(|(_, c)| c)).collect();
                            let mut cp = u32::from_str_radix(&hex, 16).ok()?;
                            if (0xD800..0xDC00).contains(&cp) {
                                // (a surrogate pair)
                                let rest = chars.as_str();
                                if let Some(lo) = rest.strip_prefix("\\u").and_then(|r| r.get(..4)).and_then(|h| u32::from_str_radix(h, 16).ok()) {
                                    if (0xDC00..0xE000).contains(&lo) {
                                        cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                                        for _ in 0..6 {
                                            chars.next();
                                        }
                                    }
                                }
                            }
                            out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                        }
                        other => out.push(other),
                    }
                }
                _ => out.push(c),
            }
        }
        None
    }
}

fn parse_json(text: &str) -> Option<Json> {
    let mut p = JsonParser { s: text, i: 0 };
    let v = p.value(0)?;
    p.ws();
    (p.i == text.len()).then_some(v)
}

fn json_text(v: &Json) -> String {
    match v {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Num(n) => n.clone(),
        Json::Str(s) => json_string(s),
        Json::Arr(items) => format!("[{}]", items.iter().map(json_text).collect::<Vec<_>>().join(",")),
        Json::Obj(fields) => format!("{{{}}}", fields.iter().map(|(k, v)| format!("{}:{}", json_string(k), json_text(v))).collect::<Vec<_>>().join(",")),
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_cell(v: Json) -> Option<String> {
    match v {
        Json::Null => None,
        Json::Bool(b) => Some(b.to_string()),
        Json::Num(n) => Some(n),
        Json::Str(s) => Some(s),
        other => Some(json_text(&other)),
    }
}

impl Frame {
    /// Column `c` becomes `values` (padded with nulls, or the frame grown,
    /// to one cell a row).
    fn set_column(&mut self, c: usize, values: Vec<Option<String>>) {
        if values.len() > self.height {
            self.grow(values.len(), 0);
        }
        let mut col = Column::with_capacity(self.height);
        for v in &values {
            col.push(v.as_deref());
        }
        col.grow(self.height);
        self.cols[c] = col;
    }
}

/// A frame from JSON: an array of records (`[{"a": 1}, …]`), JSON Lines
/// (one record a line), or columns (`{"a": [1, 2], …}`).
pub fn parse_json_frame(text: &str) -> Option<Frame> {
    let records: Vec<Json> = match parse_json(text.trim_start_matches('\u{feff}')) {
        Some(Json::Arr(items)) => items,
        Some(Json::Obj(fields)) if fields.iter().all(|(_, v)| matches!(v, Json::Arr(_) | Json::Obj(_))) && !fields.is_empty() => {
            let mut f = Frame::default();
            for (name, col) in fields {
                let cells: Vec<Option<String>> = match col {
                    Json::Arr(items) => items.into_iter().map(json_cell).collect(),
                    Json::Obj(items) => items.into_iter().map(|(_, v)| json_cell(v)).collect(),
                    _ => Vec::new(),
                };
                let c = f.add_column(&name);
                f.set_column(c, cells);
            }
            return Some(f);
        }
        Some(obj @ Json::Obj(_)) => vec![obj],
        Some(_) => return None,
        None => {
            let mut items = Vec::new();
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                items.push(parse_json(line.trim())?);
            }
            items
        }
    };
    let mut f = Frame::default();
    for record in records {
        let mut row: Vec<Option<String>> = vec![None; f.width()];
        match record {
            Json::Obj(fields) => {
                for (k, v) in fields {
                    let c = match f.columns.iter().position(|c| *c == k) {
                        Some(c) => c,
                        None => {
                            let c = f.add_column(&k);
                            row.push(None);
                            c
                        }
                    };
                    row[c] = json_cell(v);
                }
            }
            Json::Arr(items) => {
                for (c, v) in items.into_iter().enumerate() {
                    if c >= f.width() {
                        f.grow(f.height, c + 1);
                        row.resize(f.width(), None);
                    }
                    row[c] = json_cell(v);
                }
            }
            other => {
                f.grow(f.height, 1);
                row.resize(f.width(), None);
                row[0] = json_cell(other);
            }
        }
        f.push_row(&row);
    }
    Some(f)
}

/// The frame as JSON: an array of records, a line each, numbers and
/// booleans as such in columns of those types.
pub fn to_json(f: &Frame) -> String {
    let types: Vec<DType> = (0..f.width()).map(|c| f.dtype(c)).collect();
    let names: Vec<String> = f.columns.iter().map(|c| json_string(c)).collect();
    let rows: Vec<String> = (0..f.height)
        .map(|r| {
            let fields: Vec<String> = (0..f.width())
                .map(|c| {
                    let v = match (f.cell(r, c), types[c]) {
                        (None, _) => "null".to_string(),
                        (Some(s), DType::Int) => s.trim().parse::<i64>().map_or("null".into(), |n| n.to_string()),
                        (Some(s), DType::Float) => parse_num(s).map_or("null".into(), |x| format!("{x}")),
                        (Some(s), DType::Bool) => s.to_ascii_lowercase(),
                        (Some(s), DType::Str) => json_string(s),
                    };
                    format!("{}:{}", names[c], v)
                })
                .collect();
            format!("{{{}}}", fields.join(","))
        })
        .collect();
    if rows.is_empty() {
        "[]\n".to_string()
    } else {
        format!("[\n{}\n]\n", rows.join(",\n"))
    }
}

// ---------------------------------------------------------------------------
// The frames
// ---------------------------------------------------------------------------

thread_local! {
    static FRAMES: RefCell<HashMap<String, Frame>> = RefCell::new(HashMap::new());
}

/// Frame `name` (empty if it has none yet).
pub fn frame(name: &str) -> Frame {
    read(name, Frame::clone)
}

pub fn set_frame(name: &str, f: Frame) {
    FRAMES.with(|m| {
        m.borrow_mut().insert(key(name), f);
    });
}

fn read<R>(name: &str, f: impl FnOnce(&Frame) -> R) -> R {
    FRAMES.with(|m| match m.borrow().get(&key(name)) {
        Some(frame) => f(frame),
        None => f(&Frame::default()),
    })
}

fn modify<R>(name: &str, f: impl FnOnce(&mut Frame) -> R) -> R {
    FRAMES.with(|m| f(m.borrow_mut().entry(key(name)).or_default()))
}

/// A file's text: `path` as given, or (on the desktop) next to the program
/// or under RAPIDR_HOME, as the examples' data files are found.
fn read_text(path: &str) -> Result<String, String> {
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
fn load_source(source: &str, looks_like_data: impl Fn(&str) -> bool) -> Result<String, String> {
    if source.contains('\n') {
        return Ok(source.to_string());
    }
    match read_text(source) {
        Ok(text) => Ok(text),
        Err(_) if looks_like_data(source) => Ok(source.to_string()),
        Err(e) => Err(e),
    }
}

fn write_text(path: &str, text: &str) -> Result<(), String> {
    crate::objects::write_file(path, text.as_bytes())
}

/// Row `i`'s and row `j`'s cells of column `c` compared (`cmp_cells`, with
/// the column's numbers worked out once).
fn cmp_rows(c: &Column, numeric: bool, i: usize, j: usize) -> Ordering {
    match (c.get(i), c.get(j)) {
        (None, None) => Ordering::Equal,
        (None, _) => Ordering::Greater,
        (_, None) => Ordering::Less,
        (Some(x), Some(y)) => {
            if numeric {
                let (p, q) = (c.nums()[i], c.nums()[j]);
                if !p.is_nan() && !q.is_nan() {
                    return p.partial_cmp(&q).unwrap_or(Ordering::Equal);
                }
            }
            x.cmp(y)
        }
    }
}

/// The order `f`'s rows sort in by `keys` (column, descending), stably;
/// nulls last.
fn sorted_rows(f: &Frame, keys: &[(usize, bool)]) -> Vec<usize> {
    let numeric: Vec<bool> = keys.iter().map(|(c, _)| f.dtype(*c).numeric()).collect();
    let mut idx: Vec<usize> = (0..f.height).collect();
    // (one numeric key: sort by the numbers themselves)
    if let [(c, desc)] = keys {
        if numeric[0] {
            let nums = f.cols[*c].nums();
            let rank = |i: usize| if nums[i].is_nan() { (1u8, 0.0) } else { (0u8, if *desc { -nums[i] } else { nums[i] }) };
            idx.sort_by(|&i, &j| {
                let (a, b) = (rank(i), rank(j));
                a.0.cmp(&b.0).then_with(|| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
            });
            return idx;
        }
    }
    idx.sort_by(|&i, &j| {
        for (k, (c, desc)) in keys.iter().enumerate() {
            let col = &f.cols[*c];
            let o = match (col.get(i), col.get(j)) {
                (None, _) | (_, None) => cmp_rows(col, numeric[k], i, j),
                _ if *desc => cmp_rows(col, numeric[k], j, i),
                _ => cmp_rows(col, numeric[k], i, j),
            };
            if o != Ordering::Equal {
                return o;
            }
        }
        Ordering::Equal
    });
    idx
}

/// A comparison `filter` makes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
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

/// Whether `cell` (its number `n`: NaN if none) passes `cell OP value`:
/// numbers compare as numbers, text as text; a null cell never passes.
fn passes(cell: Option<&str>, n: f64, op: Op, value: &str, value_num: Option<f64>) -> bool {
    let Some(cell) = cell else { return false };
    let ord = || match value_num {
        Some(b) if !n.is_nan() => n.partial_cmp(&b).unwrap_or(Ordering::Equal),
        _ => cell.cmp(value),
    };
    match op {
        Op::Eq => ord() == Ordering::Equal,
        Op::Ne => ord() != Ordering::Equal,
        Op::Gt => ord() == Ordering::Greater,
        Op::Lt => ord() == Ordering::Less,
        Op::Ge => ord() != Ordering::Less,
        Op::Le => ord() != Ordering::Greater,
        Op::Contains => cell.contains(value),
        Op::Starts => cell.starts_with(value),
        Op::Ends => cell.ends_with(value),
    }
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

/// One group's (`members`, rows of `col`) value under aggregate `agg`.
fn aggregate(col: &Column, members: &[usize], agg: &str) -> Option<String> {
    let numeric = col.dtype().numeric();
    let nums = || -> Vec<f64> { members.iter().map(|&i| col.nums()[i]).filter(|x| !x.is_nan()).collect() };
    match agg {
        "count" => Some(members.iter().filter(|&&i| col.get(i).is_some()).count().to_string()),
        "first" => members.first().and_then(|&i| col.get(i)).map(str::to_string),
        "last" => members.last().and_then(|&i| col.get(i)).map(str::to_string),
        "sum" if numeric => Some(num_text(nums().iter().sum())),
        "min" | "max" if numeric => {
            let pick = if agg == "min" { f64::min } else { f64::max };
            nums().into_iter().reduce(pick).map(num_text)
        }
        "min" => members.iter().filter_map(|&i| col.get(i)).min().map(str::to_string),
        "max" => members.iter().filter_map(|&i| col.get(i)).max().map(str::to_string),
        "median" if numeric => {
            let v = nums();
            (!v.is_empty()).then(|| num_text(percentile(&v, 0.5)))
        }
        "std" if numeric => {
            let v = nums();
            (!v.is_empty()).then(|| num_text(std_dev(&v)))
        }
        "mean" if numeric => {
            let v = nums();
            (!v.is_empty()).then(|| num_text(v.iter().sum::<f64>() / v.len() as f64))
        }
        _ => None,
    }
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

/// The population standard deviation (RNUM's `std`).
fn std_dev(v: &[f64]) -> f64 {
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
fn percentile(v: &[f64], p: f64) -> f64 {
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

/// A join key: a number (equal numbers match: "1" and "1.0"), else text.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum JoinKey<'a> {
    Num(u64),
    Text(&'a str),
}

/// Column `k`'s rows by key (nulls left out).
fn join_index(f: &Frame, k: usize) -> HashMap<JoinKey<'_>, Vec<usize>> {
    let col = &f.cols[k];
    let nums = col.nums();
    let mut m: HashMap<JoinKey<'_>, Vec<usize>> = HashMap::new();
    for i in 0..f.height {
        if let Some(cell) = col.get(i) {
            m.entry(join_key(cell, nums[i])).or_default().push(i);
        }
    }
    m
}

/// Row `i`'s key in column `k` (None: null).
fn join_key_of(f: &Frame, k: usize, i: usize) -> Option<JoinKey<'_>> {
    f.cols[k].get(i).map(|cell| join_key(cell, f.cols[k].nums()[i]))
}

fn join_key(cell: &str, n: f64) -> JoinKey<'_> {
    if n.is_nan() {
        JoinKey::Text(cell)
    } else {
        // (0.0 and -0.0 are one number)
        JoinKey::Num(if n == 0.0 { 0 } else { n.to_bits() })
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
        "loadfromcsv" | "readcsv" | "read_csv" => {
            let source = arg_s(args, 0);
            match load_source(&source, |s| s.contains(',')) {
                Ok(text) => set_frame(name, parse_csv(&text)),
                Err(e) => warn(&e),
            }
            v_null()
        }
        "savetocsv" | "to_csv" | "writecsv" => {
            let path = arg_s_or(args, 0, "output.csv");
            if let Err(e) = write_text(&path, &read(name, to_csv)) {
                warn(&e);
            }
            v_null()
        }
        "loadfromjson" | "read_json" => {
            let source = arg_s(args, 0);
            match load_source(&source, |s| s.trim_start().starts_with(['[', '{'])).map(|t| parse_json_frame(&t)) {
                Ok(Some(f)) => set_frame(name, f),
                Ok(None) => warn("not JSON records"),
                Err(e) => warn(&e),
            }
            v_null()
        }
        "savetojson" | "to_json" => {
            let path = arg_s_or(args, 0, "output.json");
            if let Err(e) = write_text(&path, &read(name, to_json)) {
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
                v_str(match (usize::try_from(row).ok(), col) {
                    (Some(r), Some(c)) => f.cell(r, c).unwrap_or(""),
                    _ => "",
                })
            })
        }
        "cellbyname" | "at" => {
            let row = arg_i(args, 0, 0);
            let col = arg_s(args, 1);
            read(name, |f| {
                v_str(match (usize::try_from(row).ok(), f.col(&col)) {
                    (Some(r), Some(c)) => f.cell(r, c).unwrap_or(""),
                    _ => "",
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
                let col = &f.cols[c];
                let nums = col.nums();
                let rows: Vec<usize> = (0..f.height).filter(|&i| passes(col.get(i), nums[i], op, &value, value_num)).collect();
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
            modify(name, |f| {
                let gcol = &f.cols[g];
                let numeric = gcol.dtype().numeric();
                // (each group's rows, in order; groups in their keys' order)
                let mut index: HashMap<Option<&str>, usize> = HashMap::new();
                let mut groups: Vec<(Option<&str>, Vec<usize>)> = Vec::new();
                for i in 0..f.height {
                    let k = gcol.get(i);
                    let gi = *index.entry(k).or_insert_with(|| {
                        groups.push((k, Vec::new()));
                        groups.len() - 1
                    });
                    groups[gi].1.push(i);
                }
                groups.sort_by(|a, b| cmp_rows(gcol, numeric, a.1[0], b.1[0]));
                // (the group column first, then the others in their order)
                let order: Vec<usize> = std::iter::once(g).chain((0..f.width()).filter(|&c| c != g)).collect();
                let mut out = Frame { columns: order.iter().map(|&c| f.columns[c].clone()).collect(), cols: Vec::new(), height: groups.len() };
                for &c in &order {
                    let mut col = Column::with_capacity(groups.len());
                    for (k, members) in &groups {
                        if c == g {
                            col.push(*k);
                        } else {
                            col.push(aggregate(&f.cols[c], members, &agg).as_deref());
                        }
                    }
                    out.cols.push(col);
                }
                *f = out;
            });
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
                    if c.nulls.iter().any(|n| *n) {
                        *c = c.map(|v| Some(v.map_or_else(|| fill.clone(), str::to_string)));
                    }
                }
            });
            v_null()
        }
        "dropna" | "drop_nulls" => {
            modify(name, |f| {
                let rows: Vec<usize> = (0..f.height).filter(|&r| f.cols.iter().all(|c| !c.nulls[r])).collect();
                if rows.len() != f.height {
                    f.keep(&rows);
                }
            });
            v_null()
        }

        // --- Statistics ---
        "describe" => {
            // the frame becomes the summary of its numeric columns
            modify(name, |f| {
                let stats = ["count", "mean", "std", "min", "25%", "50%", "75%", "max"];
                let numeric: Vec<usize> = (0..f.width()).filter(|&c| f.dtype(c).numeric()).collect();
                let mut out = Frame::default();
                let s = out.add_column("statistic");
                out.set_column(s, stats.iter().map(|s| Some(s.to_string())).collect());
                for &c in &numeric {
                    let mut v = f.numbers(c);
                    let (m, sd) = (mean(&v), std_dev(&v));
                    sort_floats(&mut v);
                    let col = out.add_column(&f.columns[c]);
                    let values = [
                        v.len() as f64,
                        m,
                        sd,
                        v.first().copied().unwrap_or(f64::INFINITY),
                        quantile_sorted(&v, 0.25),
                        quantile_sorted(&v, 0.5),
                        quantile_sorted(&v, 0.75),
                        v.last().copied().unwrap_or(f64::NEG_INFINITY),
                    ];
                    out.set_column(col, values.iter().enumerate().map(|(r, x)| (r == 0 || !v.is_empty()).then(|| num_text(*x))).collect());
                }
                *f = out;
            });
            v_null()
        }
        "value_counts" => {
            // the frame becomes each value of the column and its count, most first
            let col = arg_s(args, 0);
            let Some(c) = read(name, |f| f.col(&col)) else {
                missing(&col);
                return v_null();
            };
            modify(name, |f| {
                let column = &f.cols[c];
                let mut order: Vec<&str> = Vec::new();
                let mut counts: HashMap<&str, usize> = HashMap::new();
                for v in column.iter().flatten() {
                    *counts.entry(v).or_insert_with(|| {
                        order.push(v);
                        0
                    }) += 1;
                }
                order.sort_by(|a, b| counts[b].cmp(&counts[a]));
                let mut out = Frame::default();
                let a = out.add_column(&f.columns[c]);
                let b = out.add_column("count");
                let n: Vec<Option<String>> = order.iter().map(|v| Some(counts[v].to_string())).collect();
                out.set_column(a, order.iter().map(|v| Some(v.to_string())).collect());
                out.set_column(b, n);
                *f = out;
            });
            v_null()
        }
        "nunique" => {
            let col = arg_s(args, 0);
            read(name, |f| match f.col(&col) {
                Some(c) => {
                    let set: std::collections::HashSet<&str> = f.cols[c].iter().flatten().collect();
                    v_int(set.len() as i64)
                }
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
                    let present = f.cols[c].nulls.iter().filter(|n| !**n).count();
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
            let joined = read(name, |this| join(this, &other, &on, &how));
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
                    for r in 0..other.height {
                        f.cols[c].push(src.get(r));
                    }
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
                    let cells: Vec<Option<String>> = f.cols.iter().map(|col| col.get(r).map(str::to_string)).collect();
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

/// `merge`: `a` joined with `b` on column `on` (`how`: inner, left, right,
/// outer, cross). The key column once; `b`'s other columns after `a`'s, a
/// name both have taking `_right`. Keys match as text, or as numbers ("1"
/// and "1.0"); a null key matches nothing.
fn join(a: &Frame, b: &Frame, on: &str, how: &str) -> Result<Frame, String> {
    let cross = how == "cross";
    let (ka, kb) = if cross {
        (usize::MAX, usize::MAX)
    } else {
        match (a.col(on), b.col(on)) {
            (Some(x), Some(y)) => (x, y),
            _ => return Err(format!("no column \"{on}\" in both frames")),
        }
    };
    // Which rows pair up: (a's row, b's row), either missing for an
    // unmatched row of the other.
    let mut pairs: Vec<(Option<usize>, Option<usize>)> = Vec::new();
    if cross {
        for i in 0..a.height {
            for j in 0..b.height {
                pairs.push((Some(i), Some(j)));
            }
        }
    } else {
        let (index, key_of) = (join_index, join_key_of);
        if how == "right" {
            let ia = index(a, ka);
            for j in 0..b.height {
                match key_of(b, kb, j).and_then(|k| ia.get(&k)) {
                    Some(rows) => pairs.extend(rows.iter().map(|&i| (Some(i), Some(j)))),
                    None => pairs.push((None, Some(j))),
                }
            }
        } else {
            let ib = index(b, kb);
            let mut b_used = vec![false; b.height];
            for i in 0..a.height {
                match key_of(a, ka, i).and_then(|k| ib.get(&k)) {
                    Some(rows) => {
                        for &j in rows {
                            b_used[j] = true;
                            pairs.push((Some(i), Some(j)));
                        }
                    }
                    None if matches!(how, "left" | "outer" | "full") => pairs.push((Some(i), None)),
                    None => {}
                }
            }
            if matches!(how, "outer" | "full") {
                pairs.extend((0..b.height).filter(|j| !b_used[*j]).map(|j| (None, Some(j))));
            }
        }
    }
    let mut out = Frame { columns: a.columns.clone(), cols: Vec::new(), height: pairs.len() };
    for (c, col) in a.cols.iter().enumerate() {
        let mut o = Column::with_capacity(pairs.len());
        for (i, j) in &pairs {
            o.push(match i {
                Some(i) => col.get(*i),
                // (an unmatched row of b: its key in the key column)
                None if c == ka => j.and_then(|j| b.cols[kb].get(j)),
                None => None,
            });
        }
        out.cols.push(o);
    }
    for (c, col) in b.cols.iter().enumerate().filter(|(c, _)| *c != kb) {
        let mut n = b.columns[c].clone();
        if out.columns.contains(&n) {
            n += "_right";
        }
        out.columns.push(n);
        let mut o = Column::with_capacity(pairs.len());
        for (_, j) in &pairs {
            o.push(j.and_then(|j| col.get(j)));
        }
        out.cols.push(o);
    }
    Ok(out)
}

/// Reads RDATAFRAME `name`'s property `prop` (lowercase); Null if it has
/// none.
pub fn dataframe_get_prop(name: &str, prop: &str) -> Value {
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
        assert_eq!(f.cell(2, 0), Some("Lee, Jo"));
        assert_eq!(f.cell(3, 1), None);
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
        assert_eq!(frame("p3").cell(0, 1), Some("41"));
        assert_eq!(frame("p3").cell(3, 1), None);
        let h = load("p3");
        call(&h, "p3", "groupby", &[v_str("city"), v_str("mean")]);
        let f = frame("p3");
        assert_eq!(f.columns, ["city", "name", "age"]);
        assert_eq!(f.cell(0, 0), Some("LA"));
        assert_eq!(f.cell(0, 2), Some("25"));
        assert_eq!(f.cell(1, 2), Some("35.5"));
        assert_eq!(f.cell(0, 1), None);
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
        assert_eq!(g.cell(1, 2), Some("true"));
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
        assert_eq!(f.cell(1, 2), Some("B"));
        assert_eq!(f.cell(3, 0), Some("4"));
    }

    #[test]
    fn describe_summarises_numbers() {
        let h = load("p4");
        call(&h, "p4", "describe", &[]);
        let f = frame("p4");
        assert_eq!(f.columns, ["statistic", "age"]);
        assert_eq!(f.cell(0, 1), Some("3"));
        assert_eq!(f.cell(1, 1), Some("32"));
        assert_eq!(f.cell(5, 1), Some("30"));
    }
}
