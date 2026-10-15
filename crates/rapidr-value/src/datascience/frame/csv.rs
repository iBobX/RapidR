//! CSV in and out (RFC 4180 as RapidR has always read it): one scanner for
//! every reader; big texts cut into pieces at record boundaries and read in
//! parallel (docs/data-science-plan.md §2.4), each piece's columns typed on
//! its own thread, the pieces joined; frames written in parallel row ranges.

use std::sync::Mutex;

use super::column::{type_cells, Bitmap, Column, Data, Texts, Typed};
use super::{par, Frame};

/// Cells that are null when read: an empty field, and the usual words for
/// a missing value.
pub(crate) fn null_word(s: &str) -> bool {
    matches!(s, "" | "NA" | "N/A" | "null" | "NULL" | "NaN")
}

/// `s` trimmed as `str::trim` trims (quickly when it starts and ends with
/// plain ASCII that isn't a space).
#[inline]
fn trim(s: &str) -> &str {
    let b = s.as_bytes();
    match (b.first(), b.last()) {
        (Some(&f), Some(&l)) if f > b' ' && f < 0x80 && l > b' ' && l < 0x80 => s,
        _ => s.trim(),
    }
}

/// A read field's cell: a quoted field as written, an unquoted one trimmed
/// (and null if it's empty or a missing-value word).
#[inline]
pub(crate) fn field_value(text: &str, quoted: bool) -> Option<&str> {
    if quoted {
        return Some(text);
    }
    let t = trim(text);
    (!null_word(t)).then_some(t)
}

/// What a scan hands its fields and records to.
pub(crate) trait Sink {
    /// One field's text (unquoted: as it stands, spaces and all) and
    /// whether it was quoted.
    fn field(&mut self, text: &str, quoted: bool);
    /// The record ended; false: stop after it.
    fn end(&mut self) -> bool;
}

/// Whether the line from `i` is blank (only white space, as `str::trim`
/// sees it) — and where it ends.
#[inline]
fn blank_line(text: &str, b: &[u8], i: usize) -> Option<usize> {
    let c = b[i];
    if c < 0x80 && !(c == b' ' || (b'\t'..=b'\r').contains(&c)) {
        return None;
    }
    let end = b[i..].iter().position(|&c| c == b'\n' || c == b'\r').map_or(b.len(), |p| i + p);
    text[i..end].trim().is_empty().then_some(end)
}

/// Where the next `delim`, line feed or carriage return is from `i` (or
/// the end).
#[inline]
fn field_end(b: &[u8], i: usize, delim: u8) -> usize {
    b[i..].iter().position(|&c| c == delim || c == b'\n' || c == b'\r').map_or(b.len(), |p| i + p)
}

/// Reads CSV records from `i` (where a record starts): quoted fields with
/// `""` for a quote and line breaks inside quotes, fields separated by
/// `delim`, blank lines skipped. Stops at the first record that starts at
/// or past `stop`, or after a record `sink.end()` says to stop at; answers
/// where it stopped (a record's start, or the end of the text).
pub(crate) fn scan(text: &str, mut i: usize, stop: usize, delim: u8, buf: &mut String, sink: &mut impl Sink) -> usize {
    let b = text.as_bytes();
    while i < b.len() && i < stop {
        // (a blank line: nothing)
        if let Some(end) = blank_line(text, b, i) {
            i = end;
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
            while j < b.len() && (b[j] == b' ' || b[j] == b'\t') && b[j] != delim {
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
                let rest_end = field_end(b, k, delim);
                buf.push_str(&text[k..rest_end]);
                sink.field(buf, true);
                i = rest_end;
            } else {
                let e = field_end(b, start, delim);
                sink.field(&text[start..e], false);
                i = e;
            }
            if i < b.len() && b[i] == delim {
                i += 1;
                if i == b.len() {
                    sink.field("", false);
                }
                continue;
            }
            break;
        }
        let go_on = sink.end();
        if i < b.len() && b[i] == b'\r' {
            i += 1;
        }
        if i < b.len() && b[i] == b'\n' {
            i += 1;
        }
        if !go_on {
            break;
        }
    }
    i
}

/// CSV records (`,` between fields), each field with whether it was quoted.
pub(crate) fn csv_records(text: &str) -> Vec<Vec<(String, bool)>> {
    struct Records(Vec<Vec<(String, bool)>>, Vec<(String, bool)>);
    impl Sink for Records {
        fn field(&mut self, text: &str, quoted: bool) {
            self.1.push((text.to_string(), quoted));
        }
        fn end(&mut self) -> bool {
            self.0.push(std::mem::take(&mut self.1));
            true
        }
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut r = Records(Vec::new(), Vec::new());
    scan(text, 0, usize::MAX, b',', &mut String::new(), &mut r);
    r.0
}

/// A frame from CSV text whose first record is the header.
pub fn parse_csv(text: &str) -> Frame {
    parse_csv_opts(text, b',', true)
}

/// One piece's records, column by column (field `c` of each record in
/// column `c`; a record without it: null there).
struct Piece {
    start: usize,
    stop: usize,
    rows: usize,
    cols: Vec<Part>,
}

/// One column of a piece, typed.
struct Part {
    typed: Typed,
    /// The text (kept only when it isn't typed).
    texts: Option<Texts>,
    valid: Bitmap,
}

struct PieceSink {
    cols: Vec<(Texts, Bitmap)>,
    c: usize,
    rows: usize,
    guess: usize,
}

impl Sink for PieceSink {
    #[inline]
    fn field(&mut self, text: &str, quoted: bool) {
        if self.c >= self.cols.len() {
            // (a column first seen here: null in the records before)
            let mut t = Texts::with_capacity(self.guess, self.guess * 8);
            for _ in 0..self.rows {
                t.push_empty();
            }
            let mut v = Bitmap::with_capacity(self.guess);
            v.resize(self.rows, false);
            self.cols.push((t, v));
        }
        let (t, v) = &mut self.cols[self.c];
        match field_value(text, quoted) {
            Some(s) => {
                t.push(s);
                v.push(true);
            }
            None => {
                t.push_empty();
                v.push(false);
            }
        }
        self.c += 1;
    }

    #[inline]
    fn end(&mut self) -> bool {
        for (t, v) in &mut self.cols[self.c..] {
            t.push_empty();
            v.push(false);
        }
        self.rows += 1;
        self.c = 0;
        true
    }
}

/// The records from `start` (a record's start) to the first record at or
/// past `stop`, their columns typed.
fn read_piece(text: &str, start: usize, stop: usize, delim: u8) -> Piece {
    let guess = (stop.min(text.len()).saturating_sub(start)) / 24 + 16;
    let mut sink = PieceSink { cols: Vec::new(), c: 0, rows: 0, guess };
    let end = scan(text, start, stop, delim, &mut String::new(), &mut sink);
    let cols = sink
        .cols
        .into_iter()
        .map(|(t, v)| {
            let typed = type_cells(&t, (!v.all()).then_some(&v));
            let texts = matches!(typed, Typed::Text).then_some(t);
            Part { typed, texts, valid: v }
        })
        .collect();
    Piece { start, stop: end, rows: sink.rows, cols }
}

/// Where a record surely starts after `p` if `p` isn't inside quotes:
/// just past the next line break.
fn next_line(b: &[u8], p: usize) -> usize {
    match b[p..].iter().position(|&c| c == b'\n' || c == b'\r') {
        Some(k) => {
            let at = p + k;
            if b[at] == b'\r' && b.get(at + 1) == Some(&b'\n') {
                at + 2
            } else {
                at + 1
            }
        }
        None => b.len(),
    }
}

/// The records from `from` read in pieces: cut at line breaks, read in
/// parallel, then checked in order — a piece whose start the piece before
/// didn't end on (a line break inside quotes) is read again from where that
/// one ended.
fn read_pieces(text: &str, from: usize, delim: u8) -> Vec<Piece> {
    let n = text.len().saturating_sub(from);
    let min = par::min_bytes();
    let k = if n < min { 1 } else { par::threads().min(n / (min / 2)).max(1) };
    let mut starts = vec![from];
    for j in 1..k {
        let s = next_line(text.as_bytes(), from + n * j / k);
        if s > *starts.last().unwrap_or(&0) && s < text.len() {
            starts.push(s);
        }
    }
    let stops: Vec<usize> = starts.iter().skip(1).copied().chain(std::iter::once(usize::MAX)).collect();
    let mut pieces = par::tasks(starts.len(), |j| read_piece(text, starts[j], stops[j], delim));
    let mut expected = from;
    for (j, p) in pieces.iter_mut().enumerate() {
        if p.start != expected {
            *p = read_piece(text, expected, stops[j], delim);
        }
        expected = p.stop;
    }
    pieces
}

/// A frame from CSV text whose fields are separated by `delim`; the first
/// record is the header when `has_header`, else the columns are
/// `column_1`, `column_2` … and every record is a row. Records with more
/// fields than the header add columns (`column_4` …, null above); records
/// with fewer are null at the end.
pub fn parse_csv_opts(text: &str, delim: u8, has_header: bool) -> Frame {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut f = Frame::default();
    let mut from = 0;
    if has_header {
        struct Header(Vec<String>);
        impl Sink for Header {
            fn field(&mut self, text: &str, _: bool) {
                self.0.push(text.to_string());
            }
            fn end(&mut self) -> bool {
                false
            }
        }
        let mut h = Header(Vec::new());
        from = scan(text, 0, usize::MAX, delim, &mut String::new(), &mut h);
        for name in &h.0 {
            f.add_column(name);
        }
    }
    let pieces = read_pieces(text, from, delim);
    let height: usize = pieces.iter().map(|p| p.rows).sum();
    let width = pieces.iter().map(|p| p.cols.len()).max().unwrap_or(0).max(f.width());
    while f.width() < width {
        let n = f.width() + 1;
        f.add_column(&format!("column_{n}"));
    }
    // (each column's parts, piece by piece: a piece without the column is
    // null there)
    let rows: Vec<usize> = pieces.iter().map(|p| p.rows).collect();
    let mut by_col: Vec<Vec<Option<Part>>> = (0..width).map(|_| Vec::with_capacity(pieces.len())).collect();
    for p in pieces {
        let mut cols = p.cols.into_iter();
        for parts in by_col.iter_mut() {
            parts.push(cols.next());
        }
    }
    let jobs: Vec<Mutex<Vec<Option<Part>>>> = by_col.into_iter().map(Mutex::new).collect();
    f.cols = par::tasks(width, |c| {
        let parts = std::mem::take(&mut *jobs[c].lock().unwrap_or_else(|e| e.into_inner()));
        join_parts(parts, &rows, height)
    });
    f.height = height;
    f
}

/// One column from its pieces' parts (None: `rows[k]` nulls): typed when
/// every part is the same type (whole numbers and fractions: floats when
/// that's lossless), else text.
fn join_parts(parts: Vec<Option<Part>>, rows: &[usize], height: usize) -> Column {
    let (mut ints, mut floats, mut bools, mut texts, mut small) = (false, false, false, false, true);
    for p in parts.iter().flatten() {
        match &p.typed {
            Typed::Null => {}
            Typed::Int(_, s) => {
                ints = true;
                small &= *s;
            }
            Typed::Float(_) => floats = true,
            Typed::Bool(_) => bools = true,
            Typed::Text => texts = true,
        }
    }
    let text = texts || (bools && (ints || floats)) || (floats && ints && !small);
    let mut valid = Bitmap::with_capacity(height);
    for (p, &n) in parts.iter().zip(rows) {
        match p {
            Some(p) => valid.extend(&p.valid),
            None => valid.resize(valid.len() + n, false),
        }
    }
    if !(ints || floats || bools || texts) {
        return Column::nulls(height);
    }
    let data = if text {
        let mut t = Texts::with_capacity(height, 0);
        let mut s = String::new();
        for (p, &n) in parts.iter().zip(rows) {
            match p {
                Some(Part { texts: Some(x), .. }) => t.extend(x),
                Some(p) => {
                    // (a typed part: its cells' text again, exactly)
                    let col = part_column(p);
                    for i in 0..n {
                        s.clear();
                        if col.write(i, &mut s) {
                            t.push(&s);
                        } else {
                            t.push_empty();
                        }
                    }
                }
                None => (0..n).for_each(|_| t.push_empty()),
            }
        }
        Data::Text(t.into())
    } else if floats {
        let mut v = Vec::with_capacity(height);
        for (p, &n) in parts.iter().zip(rows) {
            match p.as_ref().map(|p| &p.typed) {
                Some(Typed::Float(x)) => v.extend_from_slice(x),
                Some(Typed::Int(x, _)) => v.extend(x.iter().map(|&x| x as f64)),
                _ => v.resize(v.len() + n, 0.0),
            }
        }
        Data::Float(v.into())
    } else if ints {
        let mut v = Vec::with_capacity(height);
        for (p, &n) in parts.iter().zip(rows) {
            match p.as_ref().map(|p| &p.typed) {
                Some(Typed::Int(x, _)) => v.extend_from_slice(x),
                _ => v.resize(v.len() + n, 0),
            }
        }
        Data::Int(v.into())
    } else {
        let mut b = Bitmap::with_capacity(height);
        for (p, &n) in parts.iter().zip(rows) {
            match p.as_ref().map(|p| &p.typed) {
                Some(Typed::Bool(x)) => b.extend(x),
                _ => b.resize(b.len() + n, false),
            }
        }
        Data::Bool(b.into())
    };
    Column::new(data, Some(valid))
}

/// A typed part as a column (to write its cells' text).
fn part_column(p: &Part) -> Column {
    let data = match &p.typed {
        Typed::Int(x, _) => Data::Int(x.clone().into()),
        Typed::Float(x) => Data::Float(x.clone().into()),
        Typed::Bool(x) => Data::Bool(x.clone().into()),
        Typed::Null | Typed::Text => Data::Text(Texts::empties(p.valid.len()).into()),
    };
    Column::new(data, Some(p.valid.clone()))
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// Whether a field must be quoted: empty, white space around it, or the
/// delimiter, a quote or a line break in it.
#[inline]
fn needs_quotes(s: &str, d: char) -> bool {
    if s.is_empty() || trim(s).len() != s.len() {
        return true;
    }
    if d.is_ascii() {
        let d = d as u8;
        s.bytes().any(|c| c == d || c == b'"' || c == b'\n' || c == b'\r')
    } else {
        s.contains([d, '"', '\n', '\r'])
    }
}

/// `s` as a field (quoted when it must be, a quote doubled).
fn push_field(out: &mut String, s: &str, d: char) {
    if needs_quotes(s, d) {
        out.push('"');
        for (k, part) in s.split('"').enumerate() {
            if k > 0 {
                out.push_str("\"\"");
            }
            out.push_str(part);
        }
        out.push('"');
    } else {
        out.push_str(s);
    }
}

/// The frame as CSV (a header line, null cells empty).
pub fn to_csv(f: &Frame) -> String {
    to_csv_opts(f, b',', true)
}

/// The frame as CSV with fields separated by `delim`, the header line
/// first when `header`.
pub fn to_csv_opts(f: &Frame, delim: u8, header: bool) -> String {
    let d = delim as char;
    let mut out = String::new();
    if header {
        for (i, c) in f.columns.iter().enumerate() {
            if i > 0 {
                out.push(d);
            }
            push_field(&mut out, c, d);
        }
        out.push('\n');
    }
    let parts = par::ranges(f.height, par::parts(f.height), 1);
    let chunks = par::map(&parts, |r| {
        let mut s = String::with_capacity(r.len() * (f.width() * 8 + 1));
        let mut cell = String::new();
        for row in r {
            for (i, c) in f.cols.iter().enumerate() {
                if i > 0 {
                    s.push(d);
                }
                match &c.data {
                    Data::Text(t) if c.is_valid(row) => push_field(&mut s, t.get(row), d),
                    _ => {
                        cell.clear();
                        if c.write(row, &mut cell) {
                            push_field(&mut s, &cell, d);
                        }
                    }
                }
            }
            s.push('\n');
        }
        s
    });
    out.reserve(chunks.iter().map(String::len).sum());
    for c in &chunks {
        out.push_str(c);
    }
    out
}
