//! A frame's column (docs/data-science-plan.md §2.1, §2.2): typed storage —
//! `i64`, `f64`, `bool` — when that is lossless, its cells' text otherwise;
//! a validity bitmap beside it when a cell is null; every buffer shared
//! (`Arc`) so a frame copied costs nothing until one of the copies changes.
//!
//! **Lossless** means every cell reads back exactly as it was written: a
//! column is stored typed only when each non-null cell's text is the
//! canonical text of its value — `i64::to_string` for `i64`, RapidR's own
//! number text ([`crate::format::float_to_str`], [`super::super::num_text`])
//! for `f64`, lower-case `true` / `false` for `bool`. `"30"` and `"2.5"` are
//! stored as numbers; `"007"`, `"+3"`, `"1.50"`, `"1e3"` or `"True"` keep
//! their text (and their numbers are worked out once, as always). So a
//! cell's text, a column's type ([`Column::dtype`]: what its cells are, as
//! the text analysis has always said) and its numbers are the same whatever
//! the storage — only faster.

use std::borrow::Cow;
use std::fmt::Write as _;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use super::super::parse_num;
use super::{par, DType};
use crate::format::float_to_str;

/// A row number that means "no row": the cell is null (a join's unmatched
/// side).
pub(crate) const NONE: usize = usize::MAX;

// ---------------------------------------------------------------------------
// Bitmaps
// ---------------------------------------------------------------------------

/// One bit a row (bits past `len` always 0).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Bitmap {
    words: Vec<u64>,
    len: usize,
}

impl Bitmap {
    /// `len` bits, all `on` or all off.
    pub(crate) fn filled(len: usize, on: bool) -> Bitmap {
        let mut b = Bitmap { words: vec![if on { u64::MAX } else { 0 }; len.div_ceil(64)], len };
        b.clear_tail();
        b
    }

    pub(crate) fn with_capacity(len: usize) -> Bitmap {
        Bitmap { words: Vec::with_capacity(len.div_ceil(64)), len: 0 }
    }

    fn clear_tail(&mut self) {
        let r = self.len % 64;
        if r != 0 {
            if let Some(w) = self.words.last_mut() {
                *w &= (1u64 << r) - 1;
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub(crate) fn get(&self, i: usize) -> bool {
        (self.words[i >> 6] >> (i & 63)) & 1 != 0
    }

    #[inline]
    pub(crate) fn set(&mut self, i: usize, on: bool) {
        let m = 1u64 << (i & 63);
        if on {
            self.words[i >> 6] |= m;
        } else {
            self.words[i >> 6] &= !m;
        }
    }

    #[inline]
    pub(crate) fn push(&mut self, on: bool) {
        if self.len.is_multiple_of(64) {
            self.words.push(0);
        }
        if on {
            self.words[self.len >> 6] |= 1 << (self.len & 63);
        }
        self.len += 1;
    }

    pub(crate) fn count_ones(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Whether every bit is on.
    pub(crate) fn all(&self) -> bool {
        self.count_ones() == self.len
    }

    pub(crate) fn truncate(&mut self, n: usize) {
        if n < self.len {
            self.len = n;
            self.words.truncate(n.div_ceil(64));
            self.clear_tail();
        }
    }

    /// `n` bits: cut, or the new ones `on` / off.
    pub(crate) fn resize(&mut self, n: usize, on: bool) {
        if n <= self.len {
            self.truncate(n);
            return;
        }
        while self.len < n && !self.len.is_multiple_of(64) {
            self.push(on);
        }
        let full = (n - self.len) / 64;
        self.words.extend(std::iter::repeat_n(if on { u64::MAX } else { 0 }, full));
        self.len += full * 64;
        while self.len < n {
            self.push(on);
        }
    }

    /// `other`'s bits after these.
    pub(crate) fn extend(&mut self, other: &Bitmap) {
        let shift = self.len % 64;
        if shift == 0 {
            self.words.extend_from_slice(&other.words);
        } else {
            for &w in &other.words {
                if let Some(last) = self.words.last_mut() {
                    *last |= w << shift;
                }
                self.words.push(w >> (64 - shift));
            }
        }
        self.len += other.len;
        self.words.truncate(self.len.div_ceil(64));
        self.clear_tail();
    }

    /// The bits in `r`.
    pub(crate) fn slice(&self, r: Range<usize>) -> Bitmap {
        if r.start.is_multiple_of(64) {
            let mut b = Bitmap { words: self.words[r.start / 64..r.end.div_ceil(64)].to_vec(), len: r.len() };
            b.clear_tail();
            return b;
        }
        let mut b = Bitmap::with_capacity(r.len());
        for i in r {
            b.push(self.get(i));
        }
        b
    }

    /// The bits at `rows`, in that order ([`NONE`]: off).
    pub(crate) fn gather(&self, rows: &[usize]) -> Bitmap {
        let mut words = vec![0u64; rows.len().div_ceil(64)];
        let parts = par::ranges(rows.len(), par::parts(rows.len()), 64);
        par::fill(&mut words, &parts, |r| r.len().div_ceil(64), |r, out| {
            for (w, chunk) in out.iter_mut().zip(rows[r.clone()].chunks(64)) {
                let mut bits = 0u64;
                for (k, &row) in chunk.iter().enumerate() {
                    if row != NONE && self.get(row) {
                        bits |= 1 << k;
                    }
                }
                *w = bits;
            }
        });
        Bitmap { words, len: rows.len() }
    }
}

// ---------------------------------------------------------------------------
// Text storage
// ---------------------------------------------------------------------------

/// Cells' text end to end; cell `i` is `bytes[offs[i]..offs[i + 1]]` (a
/// null cell is empty).
#[derive(Clone, Debug)]
pub(crate) struct Texts {
    pub(crate) bytes: String,
    pub(crate) offs: Vec<usize>,
}

impl Default for Texts {
    fn default() -> Texts {
        Texts { bytes: String::new(), offs: vec![0] }
    }
}

impl Texts {
    pub(crate) fn with_capacity(rows: usize, bytes: usize) -> Texts {
        let mut offs = Vec::with_capacity(rows + 1);
        offs.push(0);
        Texts { bytes: String::with_capacity(bytes), offs }
    }

    /// `n` empty cells.
    pub(crate) fn empties(n: usize) -> Texts {
        Texts { bytes: String::new(), offs: vec![0; n + 1] }
    }

    pub(crate) fn len(&self) -> usize {
        self.offs.len() - 1
    }

    #[inline]
    pub(crate) fn get(&self, i: usize) -> &str {
        &self.bytes[self.offs[i]..self.offs[i + 1]]
    }

    #[inline]
    pub(crate) fn bytes_of(&self, i: usize) -> &[u8] {
        &self.bytes.as_bytes()[self.offs[i]..self.offs[i + 1]]
    }

    #[inline]
    pub(crate) fn push(&mut self, s: &str) {
        self.bytes.push_str(s);
        self.offs.push(self.bytes.len());
    }

    #[inline]
    pub(crate) fn push_empty(&mut self) {
        self.offs.push(self.bytes.len());
    }

    pub(crate) fn extend(&mut self, other: &Texts) {
        let base = self.bytes.len();
        self.bytes.push_str(&other.bytes);
        self.offs.extend(other.offs[1..].iter().map(|o| o + base));
    }

    /// Cell `i` becomes `s` (the text after it moves).
    fn set(&mut self, i: usize, s: &str) {
        let (start, end) = (self.offs[i], self.offs[i + 1]);
        self.bytes.replace_range(start..end, s);
        let delta = s.len() as isize - (end - start) as isize;
        if delta != 0 {
            for o in &mut self.offs[i + 1..] {
                *o = (*o as isize + delta) as usize;
            }
        }
    }

    fn slice(&self, r: Range<usize>) -> Texts {
        let base = self.offs[r.start];
        Texts { bytes: self.bytes[base..self.offs[r.end]].to_string(), offs: self.offs[r.start..=r.end].iter().map(|o| o - base).collect() }
    }

    /// The cells at `rows`, in that order ([`NONE`]: empty), copied in
    /// parallel for many rows.
    fn gather(&self, rows: &[usize]) -> Texts {
        let n = rows.len();
        let len_of = |i: usize| if i == NONE { 0 } else { self.offs[i + 1] - self.offs[i] };
        let parts = par::ranges(n, par::parts(n), 1);
        if parts.len() <= 1 {
            let mut out = Texts::with_capacity(n, rows.iter().map(|&i| len_of(i)).sum());
            for &i in rows {
                if i == NONE {
                    out.push_empty();
                } else {
                    out.push(self.get(i));
                }
            }
            return out;
        }
        let sizes = par::map(&parts, |r| rows[r].iter().map(|&i| len_of(i)).sum::<usize>());
        let mut bytes = vec![0u8; sizes.iter().sum()];
        let mut offs = vec![0usize; n + 1];
        // (each part: its rows, where its bytes start, its bytes, its offsets)
        let mut jobs = Vec::with_capacity(parts.len());
        let (mut brest, mut orest) = (&mut bytes[..], &mut offs[1..]);
        let mut base = 0;
        for (r, &size) in parts.iter().zip(&sizes) {
            let (b, bt) = brest.split_at_mut(size);
            let (o, ot) = orest.split_at_mut(r.len());
            jobs.push(std::sync::Mutex::new((r.clone(), base, b, o)));
            (brest, orest) = (bt, ot);
            base += size;
        }
        par::tasks(jobs.len(), |j| {
            let mut g = jobs[j].lock().unwrap_or_else(|e| e.into_inner());
            let (r, base, b, o) = &mut *g;
            let mut at = 0;
            for (k, &i) in rows[r.clone()].iter().enumerate() {
                if i != NONE {
                    let s = self.bytes_of(i);
                    b[at..at + s.len()].copy_from_slice(s);
                    at += s.len();
                }
                o[k] = *base + at;
            }
        });
        drop(jobs);
        // (whole cells of valid text, end to end: valid text)
        let bytes = String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
        Texts { bytes, offs }
    }
}

// ---------------------------------------------------------------------------
// Numbers as text
// ---------------------------------------------------------------------------

/// `v` as `i64::to_string` writes it.
pub(crate) fn push_int(out: &mut String, v: i64) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    let mut n = v.unsigned_abs();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if v < 0 {
        out.push('-');
    }
    for &d in &buf[i..] {
        out.push(d as char);
    }
}

/// A few bytes of text on the stack (`{:e}` of a float).
struct Stack {
    buf: [u8; 40],
    len: usize,
}

impl std::fmt::Write for Stack {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len + s.len();
        if end > self.buf.len() {
            return Err(std::fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// `v` as RapidR writes a number ([`float_to_str`], byte for byte): the
/// common case — up to 15 significant digits, fixed notation — written
/// straight from the shortest digits that read back as `v`; anything else
/// by `float_to_str` itself.
pub(crate) fn push_float(out: &mut String, v: f64) {
    if v == v.trunc() && v.abs() < 1e15 {
        push_int(out, v as i64);
        return;
    }
    if v.is_finite() {
        let mut s = Stack { buf: [0; 40], len: 0 };
        if write!(s, "{:e}", v.abs()).is_ok() {
            let text = &s.buf[..s.len];
            if let Some(e) = text.iter().position(|&c| c == b'e') {
                let exp = std::str::from_utf8(&text[e + 1..]).ok().and_then(|x| x.parse::<i32>().ok()).map(|x| x + 1);
                let mut digits = [0u8; 24];
                let mut nd = 0;
                for &c in &text[..e] {
                    if c.is_ascii_digit() && nd < digits.len() {
                        digits[nd] = c;
                        nd += 1;
                    }
                }
                while nd > 1 && digits[nd - 1] == b'0' {
                    nd -= 1;
                }
                if let Some(exp) = exp {
                    if nd <= 15 && (-3..=15).contains(&exp) && (exp <= 0 || (exp as usize) < nd) {
                        if v < 0.0 {
                            out.push('-');
                        }
                        if exp <= 0 {
                            out.push_str("0.");
                            for _ in 0..-exp {
                                out.push('0');
                            }
                            for &d in &digits[..nd] {
                                out.push(d as char);
                            }
                        } else {
                            let e = exp as usize;
                            for &d in &digits[..e] {
                                out.push(d as char);
                            }
                            out.push('.');
                            for &d in &digits[e..nd] {
                                out.push(d as char);
                            }
                        }
                        return;
                    }
                }
            }
        }
    }
    out.push_str(&float_to_str(v));
}

/// `s` as an `i64`, when `s` is exactly how `i64::to_string` writes it
/// (no `+`, no leading zeros, no `-0`, no spaces).
pub(crate) fn canonical_int(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let neg = b.first() == Some(&b'-');
    let digits = if neg { &b[1..] } else { b };
    if digits.is_empty() || digits.len() > 19 || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    if digits[0] == b'0' && (digits.len() > 1 || neg) {
        return None;
    }
    if digits.len() <= 18 {
        let mut n: i64 = 0;
        for &d in digits {
            n = n * 10 + (d - b'0') as i64;
        }
        return Some(if neg { -n } else { n });
    }
    s.parse::<i64>().ok()
}

/// `s` as an `f64`, when `s` is exactly how RapidR writes that number
/// ([`float_to_str`] of it gives `s` back).
pub(crate) fn canonical_float(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let body = b.strip_prefix(b"-").unwrap_or(b);
    let neg = body.len() != b.len();
    // A plain decimal: decided by its shape alone (float_to_str writes at
    // most 15 significant digits, fixed notation from 0.000d to 15 whole
    // digits, no `+`, no leading zeros, no trailing zeros after the point).
    if !body.is_empty() && body.iter().all(|&c| c.is_ascii_digit() || c == b'.') {
        let (int, frac) = match body.iter().position(|&c| c == b'.') {
            Some(p) => (&body[..p], Some(&body[p + 1..])),
            None => (body, None),
        };
        if int.is_empty() || (int.len() > 1 && int[0] == b'0') {
            return None;
        }
        match frac {
            None => {
                if int.len() > 15 || (neg && int == b"0") {
                    return None;
                }
            }
            Some(frac) => {
                if frac.is_empty() || frac.contains(&b'.') || frac.last() == Some(&b'0') {
                    return None;
                }
                let significant = if int == b"0" {
                    let zeros = frac.iter().take_while(|&&c| c == b'0').count();
                    if zeros > 3 {
                        return None;
                    }
                    frac.len() - zeros
                } else {
                    int.len() + frac.len()
                };
                if significant > 15 {
                    return None;
                }
            }
        }
        return s.parse::<f64>().ok().filter(|v| v.is_finite());
    }
    // Scientific (`1E20`, `1.5E-7`): rare — asked of float_to_str itself.
    if s.contains('E') {
        let v = parse_num(s)?;
        return (float_to_str(v) == s).then_some(v);
    }
    None
}

/// A whole number as `i64` reads it (what makes a column `i64`).
pub(crate) fn is_int(s: &str) -> bool {
    s.trim().parse::<i64>().is_ok()
}

/// Whether a stored float's text (canonical) is a whole number `i64` reads.
#[inline]
fn float_reads_as_int(v: f64) -> bool {
    v == v.trunc() && v.abs() < 1e15
}

// ---------------------------------------------------------------------------
// Columns
// ---------------------------------------------------------------------------

/// A column's cells, stored as lossless as can be.
#[derive(Clone, Debug)]
pub(crate) enum Data {
    Int(Arc<Vec<i64>>),
    Float(Arc<Vec<f64>>),
    Bool(Arc<Bitmap>),
    Text(Arc<Texts>),
}

/// One column of a frame: its cells (typed, or text), which are null, and
/// — worked out once, when first asked — its type and its cells as numbers.
#[derive(Clone, Debug)]
pub struct Column {
    pub(crate) data: Data,
    /// Which cells aren't null (None: none is).
    pub(crate) valid: Option<Arc<Bitmap>>,
    dtype: OnceLock<DType>,
    /// Each cell as a number (NaN: null or not a number).
    nums: OnceLock<Arc<Vec<f64>>>,
}

impl PartialEq for Column {
    /// Equal when every cell reads the same (whatever the storage).
    fn eq(&self, other: &Self) -> bool {
        let n = self.len();
        if n != other.len() {
            return false;
        }
        if (0..n).any(|i| self.is_valid(i) != other.is_valid(i)) {
            return false;
        }
        let valid = |i: usize| self.is_valid(i);
        match (&self.data, &other.data) {
            (Data::Int(a), Data::Int(b)) => (0..n).all(|i| !valid(i) || a[i] == b[i]),
            (Data::Float(a), Data::Float(b)) => (0..n).all(|i| !valid(i) || a[i] == b[i]),
            (Data::Bool(a), Data::Bool(b)) => (0..n).all(|i| !valid(i) || a.get(i) == b.get(i)),
            (Data::Text(a), Data::Text(b)) => (0..n).all(|i| !valid(i) || a.get(i) == b.get(i)),
            _ => (0..n).all(|i| self.get(i) == other.get(i)),
        }
    }
}

/// What a column's cells are, worked out from their text.
pub(crate) enum Typed {
    /// Every cell null.
    Null,
    /// Every cell an `i64`'s own text (and whether all are under 1e15, so
    /// also floats' own text).
    Int(Vec<i64>, bool),
    Float(Vec<f64>),
    Bool(Bitmap),
    /// Anything else: kept as text.
    Text,
}

const SMALL: u64 = 1_000_000_000_000_000;

/// How the cells of `t` (`valid`: which aren't null) can be stored without
/// losing a byte.
pub(crate) fn type_cells(t: &Texts, valid: Option<&Bitmap>) -> Typed {
    let n = t.len();
    let ok = |i: usize| valid.is_none_or(|b| b.get(i));
    let Some(first) = (0..n).find(|&i| ok(i)) else { return Typed::Null };
    let s = t.get(first);
    if canonical_int(s).is_some() {
        let mut v = vec![0i64; n];
        let mut small = true;
        for i in first..n {
            if !ok(i) {
                continue;
            }
            match canonical_int(t.get(i)) {
                Some(x) => {
                    small &= x.unsigned_abs() < SMALL;
                    v[i] = x;
                }
                None if small => {
                    // (whole numbers so far, now a fraction: floats)
                    let f: Vec<f64> = v.iter().map(|&x| x as f64).collect();
                    return floats_from(t, valid, f, i);
                }
                None => return Typed::Text,
            }
        }
        return Typed::Int(v, small);
    }
    if canonical_float(s).is_some() {
        return floats_from(t, valid, vec![0.0; n], first);
    }
    if s == "true" || s == "false" {
        let mut b = Bitmap::filled(n, false);
        for i in first..n {
            if ok(i) {
                match t.get(i) {
                    "true" => b.set(i, true),
                    "false" => {}
                    _ => return Typed::Text,
                }
            }
        }
        return Typed::Bool(b);
    }
    Typed::Text
}

fn floats_from(t: &Texts, valid: Option<&Bitmap>, mut v: Vec<f64>, from: usize) -> Typed {
    for (i, slot) in v.iter_mut().enumerate().skip(from) {
        if valid.is_some_and(|b| !b.get(i)) {
            continue;
        }
        match canonical_float(t.get(i)) {
            Some(x) => *slot = x,
            None => return Typed::Text,
        }
    }
    Typed::Float(v)
}

/// Cells collected one by one, stored at the end as lossless as can be.
pub(crate) struct Builder {
    texts: Texts,
    valid: Bitmap,
}

impl Builder {
    pub(crate) fn new(rows: usize) -> Builder {
        Builder { texts: Texts::with_capacity(rows, rows * 4), valid: Bitmap::with_capacity(rows) }
    }

    #[inline]
    pub(crate) fn push(&mut self, cell: Option<&str>) {
        match cell {
            Some(s) => self.texts.push(s),
            None => self.texts.push_empty(),
        }
        self.valid.push(cell.is_some());
    }

    pub(crate) fn finish(self) -> Column {
        Column::from_texts(self.texts, self.valid)
    }
}

impl Column {
    /// A column of `data`; `valid` dropped when no cell is null.
    pub(crate) fn new(data: Data, valid: Option<Bitmap>) -> Column {
        Column { data, valid: valid.filter(|b| !b.all()).map(Arc::new), dtype: OnceLock::new(), nums: OnceLock::new() }
    }

    /// `n` null cells.
    pub(crate) fn nulls(n: usize) -> Column {
        Column::new(Data::Text(Arc::new(Texts::empties(n))), Some(Bitmap::filled(n, false)))
    }

    /// The cells `texts` (`valid`: which aren't null), typed when that's
    /// lossless.
    pub(crate) fn from_texts(texts: Texts, valid: Bitmap) -> Column {
        let v = (!valid.all()).then_some(&valid);
        let data = match type_cells(&texts, v) {
            Typed::Int(x, _) => Data::Int(Arc::new(x)),
            Typed::Float(x) => Data::Float(Arc::new(x)),
            Typed::Bool(x) => Data::Bool(Arc::new(x)),
            Typed::Null | Typed::Text => Data::Text(Arc::new(texts)),
        };
        Column::new(data, Some(valid))
    }

    /// A column of these cells.
    pub(crate) fn from_cells<S: AsRef<str>>(cells: impl IntoIterator<Item = Option<S>>) -> Column {
        let cells = cells.into_iter();
        let mut b = Builder::new(cells.size_hint().0);
        for c in cells {
            b.push(c.as_ref().map(|s| s.as_ref()));
        }
        b.finish()
    }

    pub fn len(&self) -> usize {
        match &self.data {
            Data::Int(v) => v.len(),
            Data::Float(v) => v.len(),
            Data::Bool(b) => b.len(),
            Data::Text(t) => t.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether cell `i` isn't null.
    #[inline]
    pub(crate) fn is_valid(&self, i: usize) -> bool {
        self.valid.as_ref().is_none_or(|b| b.get(i))
    }

    /// How many cells aren't null.
    pub(crate) fn valid_count(&self) -> usize {
        self.valid.as_ref().map_or(self.len(), |b| b.count_ones())
    }

    /// Cell `i`'s text (None: null, or past the end): borrowed from text
    /// storage, written when the column is typed.
    pub fn get(&self, i: usize) -> Option<Cow<'_, str>> {
        if i >= self.len() || !self.is_valid(i) {
            return None;
        }
        Some(match &self.data {
            Data::Int(v) => {
                let mut s = String::with_capacity(20);
                push_int(&mut s, v[i]);
                Cow::Owned(s)
            }
            Data::Float(v) => {
                let mut s = String::with_capacity(24);
                push_float(&mut s, v[i]);
                Cow::Owned(s)
            }
            Data::Bool(b) => Cow::Borrowed(if b.get(i) { "true" } else { "false" }),
            Data::Text(t) => Cow::Borrowed(t.get(i)),
        })
    }

    /// Cell `i`'s text added to `out` (false: it's null).
    #[inline]
    pub(crate) fn write(&self, i: usize, out: &mut String) -> bool {
        if !self.is_valid(i) {
            return false;
        }
        match &self.data {
            Data::Int(v) => push_int(out, v[i]),
            Data::Float(v) => push_float(out, v[i]),
            Data::Bool(b) => out.push_str(if b.get(i) { "true" } else { "false" }),
            Data::Text(t) => out.push_str(t.get(i)),
        }
        true
    }

    /// Column's type: what its cells are — `i64` if every one is a whole
    /// number, `f64` if a number, `bool` if `true` / `false` (any case),
    /// `str` otherwise (and when every cell is null).
    pub fn dtype(&self) -> DType {
        *self.dtype.get_or_init(|| match &self.data {
            Data::Int(_) | Data::Bool(_) if self.valid_count() == 0 => DType::Str,
            Data::Int(_) => DType::Int,
            Data::Bool(_) => DType::Bool,
            Data::Float(v) => {
                let mut any = false;
                for (i, &x) in v.iter().enumerate() {
                    if self.is_valid(i) {
                        any = true;
                        if !float_reads_as_int(x) {
                            return DType::Float;
                        }
                    }
                }
                if any {
                    DType::Int
                } else {
                    DType::Str
                }
            }
            Data::Text(t) => text_dtype(t, self.valid.as_deref()),
        })
    }

    /// Each cell as a number (NaN: null or not a number).
    pub(crate) fn nums(&self) -> &[f64] {
        if let (Data::Float(v), None) = (&self.data, &self.valid) {
            return v;
        }
        self.nums.get_or_init(|| {
            let n = self.len();
            let mut out = vec![f64::NAN; n];
            let parts = par::ranges(n, par::parts(n), 1);
            par::fill(&mut out, &parts, |r| r.len(), |r, part| {
                for (o, i) in part.iter_mut().zip(r.clone()) {
                    if self.is_valid(i) {
                        *o = match &self.data {
                            Data::Int(v) => v[i] as f64,
                            Data::Float(v) => v[i],
                            Data::Bool(_) => f64::NAN,
                            Data::Text(t) => parse_num(t.get(i)).unwrap_or(f64::NAN),
                        };
                    }
                }
            });
            Arc::new(out)
        })
    }

    /// The type and numbers are worked out again (the cells changed).
    fn changed(&mut self) {
        self.dtype = OnceLock::new();
        self.nums = OnceLock::new();
    }

    /// The validity bitmap (made, every cell valid, if there wasn't one);
    /// `n`: the cells so far.
    fn valid_mut(&mut self, n: usize) -> &mut Bitmap {
        Arc::make_mut(self.valid.get_or_insert_with(|| Arc::new(Bitmap::filled(n, true))))
    }

    /// The column kept as text from now on (a cell that isn't its type's
    /// own text is coming).
    fn make_text(&mut self) {
        if matches!(self.data, Data::Text(_)) {
            return;
        }
        let n = self.len();
        let mut t = Texts::with_capacity(n, n * 8);
        let mut s = String::new();
        for i in 0..n {
            s.clear();
            if self.write(i, &mut s) {
                t.push(&s);
            } else {
                t.push_empty();
            }
        }
        self.data = Data::Text(Arc::new(t));
    }

    /// `cell` after the last cell.
    pub(crate) fn push(&mut self, cell: Option<&str>) {
        self.changed();
        let n = self.len();
        let Some(s) = cell else {
            match &mut self.data {
                Data::Int(v) => Arc::make_mut(v).push(0),
                Data::Float(v) => Arc::make_mut(v).push(0.0),
                Data::Bool(b) => Arc::make_mut(b).push(false),
                Data::Text(t) => Arc::make_mut(t).push_empty(),
            }
            self.valid_mut(n).push(false);
            return;
        };
        let stored = match &mut self.data {
            Data::Int(v) => canonical_int(s).map(|x| Arc::make_mut(v).push(x)).is_some(),
            Data::Float(v) => canonical_float(s).map(|x| Arc::make_mut(v).push(x)).is_some(),
            Data::Bool(b) => match s {
                "true" | "false" => {
                    Arc::make_mut(b).push(s == "true");
                    true
                }
                _ => false,
            },
            Data::Text(t) => {
                Arc::make_mut(t).push(s);
                true
            }
        };
        if !stored {
            self.make_text();
            if let Data::Text(t) = &mut self.data {
                Arc::make_mut(t).push(s);
            }
        }
        if let Some(b) = &mut self.valid {
            Arc::make_mut(b).push(true);
        }
    }

    /// Cell `row` becomes `cell` (past the end: nulls up to it first).
    pub(crate) fn set(&mut self, row: usize, cell: Option<&str>) {
        if row >= self.len() {
            self.grow(row);
            self.push(cell);
            return;
        }
        self.changed();
        let n = self.len();
        let Some(s) = cell else {
            if let Data::Text(t) = &mut self.data {
                Arc::make_mut(t).set(row, "");
            }
            self.valid_mut(n).set(row, false);
            return;
        };
        let stored = match &mut self.data {
            Data::Int(v) => canonical_int(s).map(|x| Arc::make_mut(v)[row] = x).is_some(),
            Data::Float(v) => canonical_float(s).map(|x| Arc::make_mut(v)[row] = x).is_some(),
            Data::Bool(b) => match s {
                "true" | "false" => {
                    Arc::make_mut(b).set(row, s == "true");
                    true
                }
                _ => false,
            },
            Data::Text(t) => {
                Arc::make_mut(t).set(row, s);
                true
            }
        };
        if !stored {
            self.make_text();
            if let Data::Text(t) = &mut self.data {
                Arc::make_mut(t).set(row, s);
            }
        }
        if let Some(b) = &mut self.valid {
            Arc::make_mut(b).set(row, true);
        }
    }

    /// At least `n` cells (nulls added).
    pub(crate) fn grow(&mut self, n: usize) {
        let len = self.len();
        if len >= n {
            return;
        }
        self.changed();
        match &mut self.data {
            Data::Int(v) => Arc::make_mut(v).resize(n, 0),
            Data::Float(v) => Arc::make_mut(v).resize(n, 0.0),
            Data::Bool(b) => Arc::make_mut(b).resize(n, false),
            Data::Text(t) => {
                let t = Arc::make_mut(t);
                let end = t.bytes.len();
                t.offs.resize(n + 1, end);
            }
        }
        self.valid_mut(len).resize(n, false);
    }

    /// The first `n` cells.
    pub(crate) fn truncate(&mut self, n: usize) {
        if n < self.len() {
            *self = self.slice(0..n);
        }
    }

    /// The cells in `r`, as a column of their own.
    pub(crate) fn slice(&self, r: Range<usize>) -> Column {
        if r.start == 0 && r.end == self.len() {
            return self.clone();
        }
        let data = match &self.data {
            Data::Int(v) => Data::Int(Arc::new(v[r.clone()].to_vec())),
            Data::Float(v) => Data::Float(Arc::new(v[r.clone()].to_vec())),
            Data::Bool(b) => Data::Bool(Arc::new(b.slice(r.clone()))),
            Data::Text(t) => Data::Text(Arc::new(t.slice(r.clone()))),
        };
        Column::new(data, self.valid.as_ref().map(|b| b.slice(r)))
    }

    /// The cells at `rows`, in that order ([`NONE`]: null).
    pub(crate) fn gather(&self, rows: &[usize]) -> Column {
        let any_none = rows.contains(&NONE);
        let valid = if self.valid.is_none() && !any_none {
            None
        } else {
            match &self.valid {
                Some(b) => Some(b.gather(rows)),
                None => Some(Bitmap::from_rows(rows)),
            }
        };
        let data = match &self.data {
            Data::Int(v) => Data::Int(Arc::new(gather_values(v, rows, 0))),
            Data::Float(v) => Data::Float(Arc::new(gather_values(v, rows, 0.0))),
            Data::Bool(b) => Data::Bool(Arc::new(b.gather(rows))),
            Data::Text(t) => Data::Text(Arc::new(t.gather(rows))),
        };
        Column::new(data, valid)
    }

    /// `other`'s cells after these.
    pub(crate) fn append(&mut self, other: &Column) {
        let n = self.len();
        let same = match (&mut self.data, &other.data) {
            (Data::Int(a), Data::Int(b)) => {
                Arc::make_mut(a).extend_from_slice(b);
                true
            }
            (Data::Float(a), Data::Float(b)) => {
                Arc::make_mut(a).extend_from_slice(b);
                true
            }
            (Data::Bool(a), Data::Bool(b)) => {
                Arc::make_mut(a).extend(b);
                true
            }
            (Data::Text(a), Data::Text(b)) => {
                Arc::make_mut(a).extend(b);
                true
            }
            _ => false,
        };
        if !same {
            let cells: Vec<Option<Cow<'_, str>>> = (0..n).map(|i| self.get(i)).chain((0..other.len()).map(|i| other.get(i))).collect();
            *self = Column::from_cells(cells);
            return;
        }
        self.changed();
        if self.valid.is_some() || other.valid.is_some() {
            let theirs = other.valid.as_deref().cloned().unwrap_or_else(|| Bitmap::filled(other.len(), true));
            self.valid_mut(n).extend(&theirs);
        }
    }

    /// A column kept as text, stored typed when that's lossless (None:
    /// it's typed already, or must stay text).
    pub(crate) fn retyped(&self) -> Option<Column> {
        let Data::Text(t) = &self.data else { return None };
        let valid = self.valid.as_deref().cloned().unwrap_or_else(|| Bitmap::filled(t.len(), true));
        let c = Column::from_texts((**t).clone(), valid);
        (!matches!(c.data, Data::Text(_))).then_some(c)
    }

    /// The column with each cell `f` of the old one.
    pub(crate) fn map(&self, mut f: impl FnMut(Option<&str>) -> Option<String>) -> Column {
        let n = self.len();
        let mut b = Builder::new(n);
        let mut s = String::new();
        for i in 0..n {
            s.clear();
            let v = if self.write(i, &mut s) { f(Some(&s)) } else { f(None) };
            b.push(v.as_deref());
        }
        b.finish()
    }
}

impl Bitmap {
    /// On where `rows` names a row.
    fn from_rows(rows: &[usize]) -> Bitmap {
        let mut b = Bitmap::with_capacity(rows.len());
        for &r in rows {
            b.push(r != NONE);
        }
        b
    }
}

/// `v` at `rows` ([`NONE`]: `none`), in parallel for many rows.
fn gather_values<T: Copy + Send + Sync>(v: &[T], rows: &[usize], none: T) -> Vec<T> {
    let mut out = vec![none; rows.len()];
    let parts = par::ranges(rows.len(), par::parts(rows.len()), 1);
    par::fill(&mut out, &parts, |r| r.len(), |r, part| {
        for (o, &i) in part.iter_mut().zip(&rows[r.clone()]) {
            if let Some(x) = v.get(i) {
                *o = *x;
            }
        }
    });
    out
}

/// A text column's type, as the cells say (the rules of [`Column::dtype`]).
fn text_dtype(t: &Texts, valid: Option<&Bitmap>) -> DType {
    let mut ty = None::<DType>;
    for i in 0..t.len() {
        if valid.is_some_and(|b| !b.get(i)) {
            continue;
        }
        let cell = t.get(i);
        let n = parse_num(cell);
        let this = if n.is_some() && is_int(cell) {
            DType::Int
        } else if n.is_some() {
            DType::Float
        } else if cell.eq_ignore_ascii_case("true") || cell.eq_ignore_ascii_case("false") {
            DType::Bool
        } else {
            return DType::Str;
        };
        ty = Some(match (ty, this) {
            (None, x) => x,
            (Some(a), b) if a == b => a,
            (Some(DType::Int), DType::Float) | (Some(DType::Float), DType::Int) => DType::Float,
            _ => return DType::Str,
        });
    }
    ty.unwrap_or(DType::Str)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic stream of numbers (xorshift64*).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
    }

    #[test]
    fn numbers_written_as_rapidr_writes_them() {
        let mut r = Rng(0x9E37_79B9_7F4A_7C15);
        let check = |v: f64| {
            let mut s = String::new();
            push_float(&mut s, v);
            assert_eq!(s, float_to_str(v), "{v:e}");
        };
        for v in [0.0, -0.0, 0.5, -0.5, 1e-4, 1.5e-4, 1e-5, 0.1 + 0.2, 1e15, 1e15 + 0.5, 999999999999999.9, 123.456, f64::MAX, f64::MIN_POSITIVE, 5e-324, f64::NAN, f64::INFINITY] {
            check(v);
        }
        for _ in 0..300_000 {
            let bits = r.next();
            check(f64::from_bits(bits));
            // (everyday numbers: a few digits, a point somewhere)
            let digits = (r.next() % 10_000_000_000_000_000) as f64;
            let scale = 10f64.powi((r.next() % 24) as i32 - 6);
            check(digits / scale);
            check(-((r.next() % 100_000) as f64) / 100.0);
        }
        let mut s = String::new();
        for v in [0, 1, -1, 9, 10, -10, i64::MAX, i64::MIN, 1234567890] {
            s.clear();
            push_int(&mut s, v);
            assert_eq!(s, v.to_string());
        }
    }

    #[test]
    fn canonical_text_is_what_reads_back() {
        // the shape test agrees with float_to_str itself
        let mut r = Rng(42);
        let mut texts: Vec<String> = ["0", "-0", "1", "007", "+3", ".5", "5.", "1.50", "1.5", "0.0001", "0.00001", "1e3", "1E3", "1E20", "1E-5", "1.5E-7", "-0.5", "0.0", "123456789012345", "1234567890123456", "1.23456789012345", "12345678901234.5", "0.000123456789012345", "inf", "NaN", " 1", "1 ", "--1", "1.2.3", "", "-", "."]
            .iter()
            .map(|s| s.to_string())
            .collect();
        for _ in 0..200_000 {
            let v = f64::from_bits(r.next());
            if v.is_finite() {
                texts.push(float_to_str(v));
                texts.push(format!("{v}"));
            }
            let whole = (r.next() % 1_000_000) as i64 - 500_000;
            let frac = r.next() % 1000;
            texts.push(format!("{whole}.{frac}"));
            texts.push(format!("{whole}.{frac:03}"));
            texts.push(format!("0.{:0w$}{}", 0, frac, w = (r.next() % 6) as usize));
        }
        for t in &texts {
            let slow = parse_num(t).filter(|v| float_to_str(*v) == *t);
            assert_eq!(canonical_float(t), slow, "{t:?}");
            let slow_int = t.parse::<i64>().ok().filter(|v| v.to_string() == *t);
            assert_eq!(canonical_int(t), slow_int, "{t:?}");
        }
    }

    #[test]
    fn bitmaps() {
        let mut a = Bitmap::default();
        for i in 0..150 {
            a.push(i % 3 == 0);
        }
        let mut b = Bitmap::filled(70, true);
        b.set(5, false);
        let mut c = a.clone();
        c.extend(&b);
        assert_eq!(c.len(), 220);
        for i in 0..220 {
            assert_eq!(c.get(i), if i < 150 { i % 3 == 0 } else { i != 155 }, "{i}");
        }
        assert_eq!(c.slice(3..200), Bitmap { words: c.slice(3..200).words.clone(), len: 197 });
        for i in 3..200 {
            assert_eq!(c.slice(3..200).get(i - 3), c.get(i));
        }
        let rows: Vec<usize> = (0..220).rev().chain([NONE]).collect();
        let g = c.gather(&rows);
        for (k, &r) in rows.iter().enumerate() {
            assert_eq!(g.get(k), r != NONE && c.get(r));
        }
        c.resize(300, true);
        assert!((220..300).all(|i| c.get(i)));
        c.truncate(10);
        assert_eq!(c.count_ones(), 4);
    }

    #[test]
    fn columns_keep_their_text() {
        let cells = [Some("1"), None, Some("22"), Some("-3")];
        let c = Column::from_cells(cells);
        assert!(matches!(c.data, Data::Int(_)));
        assert_eq!(c.dtype(), DType::Int);
        assert_eq!((0..4).map(|i| c.get(i)).collect::<Vec<_>>(), cells.map(|s| s.map(Cow::Borrowed)));
        let f = Column::from_cells([Some("1.5"), Some("2"), None]);
        assert!(matches!(f.data, Data::Float(_)));
        assert_eq!(f.dtype(), DType::Float);
        assert_eq!(f.gather(&[1, 2]).dtype(), DType::Int);
        let t = Column::from_cells([Some("1.50"), Some("2")]);
        assert!(matches!(t.data, Data::Text(_)));
        assert_eq!(t.dtype(), DType::Float);
        let mut m = c.clone();
        m.set(1, Some("x"));
        assert!(matches!(m.data, Data::Text(_)));
        assert_eq!(m.get(1).as_deref(), Some("x"));
        assert_eq!(m.get(2).as_deref(), Some("22"));
        assert_eq!(c.get(1), None);
        let mut p = c.clone();
        p.push(Some("5"));
        p.push(None);
        p.grow(8);
        assert_eq!(p.len(), 8);
        assert!(matches!(p.data, Data::Int(_)));
        assert_eq!(p.get(4).as_deref(), Some("5"));
        assert_eq!(p.valid_count(), 4);
        assert_eq!(Column::nulls(3).dtype(), DType::Str);
        let b = Column::from_cells([Some("true"), Some("false")]);
        assert!(matches!(b.data, Data::Bool(_)));
        assert_eq!(b, Column::from_cells([Some("true"), Some("false")]));
        assert_ne!(b, Column::from_cells([Some("True"), Some("false")]));
    }
}
