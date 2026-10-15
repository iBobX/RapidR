//! The frame engine against its former self: the row-of-text engine of
//! commit 920c8a85 (tests/oracle/old_frame.rs, compiled only into this
//! test) and the typed, columnar, parallel engine run the same random
//! programs — random CSV (whole numbers and fractions written every way,
//! booleans in any case, text, quotes, nulls, ragged rows, odd line ends)
//! then random member calls — and must agree on everything a program can
//! see: what each call answers, prints and warns, and the frame after it
//! (printed, every cell, every type, CSV and JSON written).
//!
//! The parallel paths are made to run on small tables too
//! (`set_thresholds`), so pieces and parts start at random places.
//!
//!   cargo test -p rapidr-value --test frame_oracle
//!   FRAME_ORACLE_SCALE=10 cargo test --release -p rapidr-value --test frame_oracle
//!
//! (`FRAME_ORACLE_SCALE`: ten times the programs and big tables.)

use std::cell::RefCell;
use std::fmt::Write as _;

pub use rapidr_value::{v_dbl, v_int, v_null, v_str, Value};

use rapidr_value::datascience::frame as new;
use rapidr_value::datascience::Host;

/// What the old engine reaches as `crate::objects`.
mod objects {
    pub use rapidr_value::objects::{read_file, write_file};
}

/// The old engine and what it reached as `super::…` (datascience/mod.rs's
/// helpers, as they were).
mod oracle {
    pub use rapidr_value::datascience::Host;
    use rapidr_value::Value;

    pub(crate) fn key(name: &str) -> String {
        name.trim().to_ascii_lowercase()
    }

    pub(crate) fn arg_s(args: &[Value], i: usize) -> String {
        args.get(i).map(Value::to_string_val).unwrap_or_default()
    }

    pub(crate) fn arg_s_or(args: &[Value], i: usize, default: &str) -> String {
        args.get(i).map(Value::to_string_val).unwrap_or_else(|| default.to_string())
    }

    pub(crate) fn arg_i(args: &[Value], i: usize, default: i64) -> i64 {
        match args.get(i) {
            Some(Value::String(s)) => s.trim().parse::<f64>().map(|f| f as i64).unwrap_or(default),
            Some(v) => v.to_i64(),
            None => default,
        }
    }

    pub fn num_text(v: f64) -> String {
        rapidr_value::format::float_to_str(v)
    }

    pub(crate) fn parse_num(s: &str) -> Option<f64> {
        let t = s.trim();
        let first = t.chars().next()?;
        if !(first.is_ascii_digit() || matches!(first, '-' | '+' | '.')) {
            return None;
        }
        t.parse::<f64>().ok().filter(|f| f.is_finite())
    }

    pub(crate) fn random_index(n: usize) -> usize {
        if n == 0 {
            0
        } else {
            rapidr_value::builtins::random_index(n).min(n - 1)
        }
    }

    pub mod link {
        pub fn frame_changed(_name: &str) {}
    }

    pub mod test_host {
        use std::cell::RefCell;

        #[derive(Default)]
        pub struct TestHost {
            pub printed: RefCell<Vec<String>>,
            pub warned: RefCell<Vec<String>>,
            pub grids: RefCell<Vec<(String, Vec<Vec<String>>)>>,
            pub shown: RefCell<Vec<String>>,
        }

        impl super::Host for TestHost {
            fn print(&self, text: &str) {
                self.printed.borrow_mut().push(text.to_string());
            }
            fn warn(&self, text: &str) {
                self.warned.borrow_mut().push(text.to_string());
            }
            fn to_grid(&self, grid: &str, rows: &[Vec<String>]) {
                self.grids.borrow_mut().push((grid.to_string(), rows.to_vec()));
            }
            fn save_plot(&self, plot: &str, file: &str, _scale: f64) {
                self.shown.borrow_mut().push(format!("save {plot} {file}"));
            }
            fn show_plot(&self, plot: &str) {
                self.shown.borrow_mut().push(format!("show {plot}"));
            }
        }
    }

    #[allow(dead_code, clippy::all)]
    #[path = "old_frame.rs"]
    pub mod frame;
}

use oracle::frame as old;

/// Everything a host is asked, in order.
#[derive(Default)]
struct Log(RefCell<Vec<String>>);

impl Host for Log {
    fn print(&self, text: &str) {
        self.0.borrow_mut().push(format!("print {text}"));
    }
    fn warn(&self, text: &str) {
        self.0.borrow_mut().push(format!("warn {text}"));
    }
    fn to_grid(&self, grid: &str, rows: &[Vec<String>]) {
        self.0.borrow_mut().push(format!("grid {grid} {rows:?}"));
    }
    fn save_plot(&self, _: &str, _: &str, _: f64) {}
    fn show_plot(&self, _: &str) {}
}

/// A deterministic stream of numbers (xorshift64*).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// A cell's text as a CSV file might have it, for a column of `kind`.
fn cell(r: &mut Rng, kind: usize) -> String {
    const NULLS: [&str; 7] = ["", "NA", "N/A", "null", "NULL", "NaN", " "];
    const ODD_INTS: [&str; 12] = ["007", "+3", "-0", "00", "9223372036854775807", "-9223372036854775808", "9223372036854775808", "1234567890123456", "999999999999999", "1000000000000000", " 12", "1_000"];
    const ODD_FLOATS: [&str; 22] = ["1.50", "1e3", "1E3", ".5", "5.", "0.00001", "0.0001", "1.0", "3.14159265358979323", "1E20", "1.5E-7", "-0.0", "1e400", "inf", "-.25", "2.50e1", "123456789012345.6", "0.1", "-1.25", "1E15", "1E-5", "100000000000000000000"];
    const BOOLS: [&str; 7] = ["true", "false", "True", "FALSE", "tRuE", "false", "true"];
    const WORDS: [&str; 16] = ["bob", "amy", "Lee, Jo", "say \"hi\"", "two\nlines", "  spaced  ", "é", "日本", "NYC", "LA", "na", "x", "Zed", "a;b", "tab\there", "\"\""];
    if r.chance(12) {
        return r.pick(&NULLS).to_string();
    }
    match kind {
        // whole numbers, now and then written oddly
        0 => {
            if r.chance(5) {
                r.pick(&ODD_INTS).to_string()
            } else {
                (r.below(2001) as i64 - 1000).to_string()
            }
        }
        // fractions and whole numbers
        1 => {
            if r.chance(8) {
                r.pick(&ODD_FLOATS).to_string()
            } else if r.chance(30) {
                (r.below(100) as i64 - 50).to_string()
            } else {
                rapidr_value::format::float_to_str((r.below(200_001) as f64 - 100_000.0) / [4.0, 100.0, 8.0, 1000.0, 3.0][r.below(5)])
            }
        }
        2 => r.pick(&BOOLS).to_string(),
        3 => r.pick(&WORDS).to_string(),
        // a mixture
        4 => match r.below(4) {
            0 => cell(r, 0),
            1 => cell(r, 1),
            2 => cell(r, 2),
            _ => cell(r, 3),
        },
        // mostly whole numbers, one odd one now and then
        5 => {
            if r.chance(3) {
                r.pick(&["2.5", "abc", "true", "1.50", "+1"]).to_string()
            } else {
                r.below(50).to_string()
            }
        }
        // keys: few values
        _ => r.pick(&["a", "b", "c", "1", "1.0", "2", "B", "true"]).to_string(),
    }
}

/// A cell of any kind.
fn any_cell(r: &mut Rng) -> String {
    let kind = r.below(7);
    cell(r, kind)
}

/// `s` as a CSV field (quoted when it must be, or at random).
fn field(r: &mut Rng, s: &str, delim: char) -> String {
    let must = s.contains([delim, '"', '\n', '\r']) || s.trim() != s;
    if must || r.chance(10) {
        let lead = if r.chance(5) { "  " } else { "" };
        let tail = if r.chance(4) { "x" } else { "" };
        format!("{lead}\"{}\"{tail}", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// A random CSV text: `rows` records of `kinds` columns, a header maybe,
/// line ends of every kind, blank lines, ragged rows, a BOM.
fn csv_text(r: &mut Rng, kinds: &[usize], rows: usize, delim: char, header: bool) -> String {
    let mut s = String::new();
    if r.chance(5) {
        s.push('\u{feff}');
    }
    let eol = *r.pick(&["\n", "\n", "\r\n", "\r"]);
    let names = ["id", "name", "age", "City", "x", "x", "", " pad ", "column_3", "score"];
    if header {
        let line: Vec<String> = kinds
            .iter()
            .map(|_| {
                let k = r.below(names.len());
                field(r, names[k], delim)
            })
            .collect();
        s += &line.join(&delim.to_string());
        s += eol;
    }
    for _ in 0..rows {
        if r.chance(3) {
            s += *r.pick(&["", "   ", "\t"]);
            s += eol;
        }
        let mut n = kinds.len();
        if r.chance(6) {
            n = r.below(kinds.len() + 3);
        }
        let line: Vec<String> = (0..n).map(|c| { let v = cell(r, kinds[c % kinds.len()]); field(r, &v, delim) }).collect();
        s += &line.join(&delim.to_string());
        if r.chance(3) {
            s.push(delim);
        }
        s += eol;
    }
    if r.chance(20) && s.ends_with(eol) {
        s.truncate(s.len() - eol.len());
    }
    if r.chance(2) {
        s += "\"unclosed, quote";
    }
    s
}

/// Everything a program can see of frame `name`.
fn view_new(name: &str, delim: u8, header: bool) -> String {
    new::with_frame(name, |f| {
        let mut s = String::new();
        let _ = writeln!(s, "{}x{} {:?}", f.height(), f.width(), f.columns);
        s += &f.to_text();
        s.push('\n');
        for c in 0..f.width() {
            let _ = write!(s, "{}:{} ", f.columns[c], f.dtype(c).name());
        }
        s.push('\n');
        for r in 0..f.height() + 1 {
            for c in 0..f.width() + 1 {
                let _ = write!(s, "{:?}|", f.cell(r, c).as_deref());
            }
            s.push('\n');
        }
        s += &new::to_csv(f);
        s += &new::to_csv_opts(f, delim, header);
        s += &new::to_json(f);
        let _ = write!(s, "{:?}", f.grid_rows());
        s
    })
}

fn view_old(name: &str, delim: u8, header: bool) -> String {
    old::with_frame(name, |f| {
        let mut s = String::new();
        let _ = writeln!(s, "{}x{} {:?}", f.height(), f.width(), f.columns);
        s += &f.to_text();
        s.push('\n');
        for c in 0..f.width() {
            let _ = write!(s, "{}:{} ", f.columns[c], f.dtype(c).name());
        }
        s.push('\n');
        for r in 0..f.height() + 1 {
            for c in 0..f.width() + 1 {
                let _ = write!(s, "{:?}|", f.cell(r, c).as_deref());
            }
            s.push('\n');
        }
        s += &old::to_csv(f);
        s += &old::to_csv_opts(f, delim, header);
        s += &old::to_json(f);
        let _ = write!(s, "{:?}", f.grid_rows());
        s
    })
}

/// A column of frame `name` by name (one in ten: one it hasn't).
fn some_column(r: &mut Rng, name: &str) -> String {
    let cols = new::with_frame(name, |f| f.columns.clone());
    if cols.is_empty() || r.chance(8) {
        return r.pick(&["nope", "AGE", "x", "column_2", ""]).to_string();
    }
    let c = r.pick(&cols).clone();
    if r.chance(10) {
        c.to_uppercase()
    } else {
        c
    }
}

/// A value to compare with: one of the column's own cells, or anything.
fn some_value(r: &mut Rng, name: &str, col: &str) -> String {
    let own = new::with_frame(name, |f| {
        let c = f.col(col)?;
        (f.height() > 0).then(|| f.cell(r.below(f.height()), c).map(|s| s.into_owned()))?
    });
    match (own, r.below(3)) {
        (Some(v), 0 | 1) => v,
        _ => any_cell(r),
    }
}

/// One random member call on frame `name` (`other`: the second frame).
fn random_call(r: &mut Rng, name: &str, other: &str) -> (String, Vec<Value>) {
    let col = some_column(r, name);
    let h = new::with_frame(name, |f| f.height()) as i64;
    let int = |r: &mut Rng| -> i64 { r.below(h as usize + 4) as i64 - 1 };
    let ops = ["==", "=", "!=", "<>", ">", "<", ">=", "<=", "contains", "startswith", "endswith", "~"];
    let m = *r.pick(&[
        "filter", "filter", "query", "sort", "sort", "sort_values", "groupby", "groupby", "merge", "concat", "describe", "value_counts", "nunique", "corr", "apply", "replace", "fillna", "dropna", "head", "tail", "iloc", "select", "setcell", "addrow", "addcolumn",
        "transpose", "drop", "rename", "nlargest", "nsmallest", "cell", "cellbyname", "info", "dtypes", "shape", "columns", "rows", "tostring", "print", "clear", "togrid", "rowcount", "empty",
    ]);
    let args: Vec<Value> = match m {
        "filter" => {
            if r.chance(20) {
                let v = some_value(r, name, &col);
                vec![v_str(&format!("{col} {} {v}", r.pick(&ops)))]
            } else {
                let v = some_value(r, name, &col);
                let value = if r.chance(20) { v_int(int(r)) } else { v_str(&v) };
                vec![v_str(&col), v_str(r.pick(&ops)), value]
            }
        }
        "query" => {
            let v = some_value(r, name, &col);
            let q = *r.pick(&["'", "\"", "", "`"]);
            vec![v_str(&format!("{col} {} {q}{v}{q}", r.pick(&ops)))]
        }
        "sort" | "sort_values" => {
            let keys = if r.chance(30) { format!("{col},{}", some_column(r, name)) } else { col };
            vec![v_str(&keys), v_int(r.below(2) as i64)]
        }
        "groupby" => {
            let agg = *r.pick(&["mean", "sum", "count", "min", "max", "first", "last", "median", "std", "avg", " MEAN "]);
            if r.chance(5) {
                vec![v_str(&col)]
            } else {
                vec![v_str(&col), v_str(agg)]
            }
        }
        "merge" => {
            let on = if r.chance(70) { some_column(r, other) } else { col };
            vec![v_str(other), v_str(&on), v_str(r.pick(&["inner", "left", "right", "outer", "full", "cross", "weird", "LEFT"]))]
        }
        "concat" => vec![v_str(other)],
        "value_counts" | "nunique" | "drop" => vec![v_str(&col)],
        "corr" => vec![v_str(&col), v_str(&some_column(r, name))],
        "apply" => {
            let op = *r.pick(&["upper", "lower", "trim", "abs", "round", "sqrt", "log", "exp", "bad"]);
            vec![v_str(&col), v_str(op), v_int(r.below(5) as i64 - 1)]
        }
        "replace" => {
            let old = some_value(r, name, &col);
            vec![v_str(&col), v_str(&old), v_str(&any_cell(r))]
        }
        "fillna" => {
            if r.chance(30) {
                vec![]
            } else {
                vec![v_str(&any_cell(r))]
            }
        }
        "head" | "tail" => vec![v_int(int(r))],
        "iloc" => vec![v_int(int(r)), v_int(int(r))],
        "select" => vec![v_str(&format!("{col},{}", some_column(r, name)))],
        "setcell" => {
            let c = if r.chance(50) { v_int(r.below(6) as i64) } else { v_str(&some_column(r, name)) };
            vec![v_int(int(r)), c, v_str(&any_cell(r))]
        }
        "addrow" => {
            if r.chance(40) {
                let cells: Vec<String> = (0..r.below(5) + 1).map(|_| any_cell(r).replace(['\n', '"', ','], "")).collect();
                vec![v_str(&cells.join(","))]
            } else {
                (0..r.below(5)).map(|_| v_str(&any_cell(r))).collect()
            }
        }
        "addcolumn" => {
            let n = r.below(h as usize + 3);
            let kind = r.below(7);
            let cells: Vec<String> = (0..n).map(|_| cell(r, kind).replace(['\n', '"', ','], "")).collect();
            vec![v_str(r.pick(&["new", "age", "x", " ", "id"])), v_str(&cells.join(","))]
        }
        "rename" => vec![v_str(&col), v_str(r.pick(&["renamed", "x", "id", " y "]))],
        "nlargest" | "nsmallest" => vec![v_str(&col), v_int(r.below(6) as i64)],
        "cell" => vec![v_int(int(r)), if r.chance(50) { v_int(r.below(6) as i64 - 1) } else { v_str(&col) }],
        "cellbyname" => vec![v_int(int(r)), v_str(&col)],
        "togrid" => vec![v_str("Grid1")],
        _ => vec![],
    };
    (m.to_string(), args)
}

/// Loads random CSV into frame `name` in both engines; the CSV's delimiter
/// and header.
fn load_random(r: &mut Rng, name: &str, max_rows: usize) -> (u8, bool) {
    let width = r.below(5) + 1;
    let kinds: Vec<usize> = (0..width).map(|_| r.below(7)).collect();
    let rows = r.below(max_rows + 1);
    let (delim, header) = if r.chance(70) { (',', true) } else { (*r.pick(&[',', ';', '\t', '|']), r.chance(50)) };
    let text = csv_text(r, &kinds, rows, delim, header);
    if delim == ',' && header && r.chance(80) {
        let (a, b) = (Log::default(), Log::default());
        new::dataframe_method(name, "loadfromcsv", &[v_str(&text)], &a);
        oracle::frame::dataframe_method(name, "loadfromcsv", &[v_str(&text)], &b);
        assert_eq!(a.0.borrow().join("\n"), b.0.borrow().join("\n"), "loading {text:?}");
    } else {
        new::set_frame(name, new::parse_csv_opts(&text, delim as u8, header));
        old::set_frame(name, old::parse_csv_opts(&text, delim as u8, header));
    }
    let (d, h) = (delim as u8, header);
    assert_eq!(view_new(name, d, h), view_old(name, d, h), "after loading {text:?} (delimiter {delim:?}, header {header})");
    (d, h)
}

/// One random program: load, then calls; both engines must agree after
/// each step.
fn run_case(seed: u64, max_rows: usize, steps: usize) {
    let mut r = Rng::new(seed);
    let (name, other) = (format!("t{seed}"), format!("o{seed}"));
    let (d, h) = load_random(&mut r, &name, max_rows);
    load_random(&mut r, &other, max_rows / 4 + 3);
    let mut log = vec![format!("seed {seed}")];
    for _ in 0..steps {
        let (m, args) = random_call(&mut r, &name, &other);
        log.push(format!("{m} {args:?}"));
        let (a, b) = (Log::default(), Log::default());
        let va = new::dataframe_method(&name, &m, &args, &a);
        let vb = oracle::frame::dataframe_method(&name, &m, &args, &b);
        let steps = log.join("\n  ");
        assert_eq!(format!("{va:?}"), format!("{vb:?}"), "answers differ:\n  {steps}");
        assert_eq!(a.0.borrow().join("\n"), b.0.borrow().join("\n"), "host calls differ:\n  {steps}");
        assert_eq!(view_new(&name, d, h), view_old(&name, d, h), "frames differ:\n  {steps}");
        for p in ["rowcount", "colcount", "columns", "shape", "empty", "dtypes", "nope"] {
            assert_eq!(format!("{:?}", new::dataframe_get_prop(&name, p)), format!("{:?}", old::dataframe_get_prop(&name, p)), "{p}:\n  {steps}");
        }
        // (the contract's members, beside the methods: sort_by_column as
        // the old engine sorts, sorted_order the order it sorts in)
        let sorted = |f_text: Vec<String>| f_text;
        let old_sorted = sorted(old::with_frame(&name, |f| {
            (0..f.width())
                .map(|c| {
                    let mut g = f.clone();
                    g.sort_by_column(c, c % 2 == 1);
                    g.to_text()
                })
                .collect()
        }));
        new::with_frame(&name, |f| {
            for (c, old_text) in old_sorted.iter().enumerate() {
                let mut g = f.clone();
                g.sort_by_column(c, c % 2 == 1);
                assert_eq!(&g.to_text(), old_text, "sort_by_column {c}:\n  {steps}");
                let order = f.sorted_order(c, c % 2 == 1);
                let by_order: Vec<_> = order.iter().map(|&i| f.row(i)).collect();
                let by_sort: Vec<_> = (0..g.height()).map(|i| g.row(i)).collect();
                assert_eq!(by_order, by_sort, "sorted_order {c}:\n  {steps}");
            }
        });
    }
    new::remove_frame(&name);
    new::remove_frame(&other);
    old::remove_frame(&name);
    old::remove_frame(&other);
}

/// How many times the default depth (FRAME_ORACLE_SCALE).
fn scale() -> u64 {
    std::env::var("FRAME_ORACLE_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1).max(1)
}

#[test]
fn random_programs_small_tables_parallel_paths() {
    // (every table split: the parallel paths, pieces at random places)
    new::set_thresholds(8, 3, 16, 4);
    let n = 2500 * scale();
    for seed in 0..n {
        run_case(seed, 40, 8);
    }
    eprintln!("{n} random programs of up to 40 rows, 8 calls each: the engines agree");
}

#[test]
fn random_programs_wider_tables() {
    new::set_thresholds(8, 3, 16, 4);
    let n = 200 * scale();
    for seed in 1_000_000..1_000_000 + n {
        run_case(seed, 400, 6);
    }
    eprintln!("{n} random programs of up to 400 rows, 6 calls each: the engines agree");
}

#[test]
fn csv_quirks_agree() {
    new::set_thresholds(8, 3, 16, 4);
    let texts = [
        "",
        "\n\n",
        "a,b\n",
        "a,b",
        "\u{feff}a,b\n1,2\n",
        "a,b\n1,2,",
        "a,b\n1,2,\n",
        "a,b\n1,2,3,4\n5\n",
        "a,a,,column_3\n1,2,3,4,5\n",
        "a,b\n  \"x\",\"y\"z\n\"multi\nline\",2\n",
        "a,b\r\n1,2\r\n\r\n3,4\r\n",
        "a,b\r1,2\r3,4",
        "a\n\"unclosed\n1\n2\n",
        "a,b\n \u{3000}\n1,2\n",
        "a,b\n\t\t\n1,2\n",
        "a,b\n\"\",NA\n N/A ,null\n",
        "a\n007\n+3\n1.50\n1e3\n",
        "a\n1\n2.5\n",
        "a\n1\n2.50\n",
        "a\ntrue\nFalse\n",
        "a\n9223372036854775807\n1.5\n",
        "a\n999999999999999\n0.5\n",
        "a\n1000000000000000\n0.5\n",
    ];
    for t in texts {
        for (d, h) in [(b',', true), (b',', false), (b';', true), (b'\t', false)] {
            let (a, b) = (new::parse_csv_opts(t, d, h), old::parse_csv_opts(t, d, h));
            new::set_frame("q", a);
            old::set_frame("q", b);
            assert_eq!(view_new("q", d, h), view_old("q", d, h), "{t:?} {d} {h}");
        }
    }
    // (written with a delimiter that numbers and true / false contain:
    // those cells must be quoted)
    let typed = "i,f,b,s\n12,-2.5,true,x\n-7,1E20,false,1.5\n,0.0001,,\n";
    for d in [b'1', b'2', b'5', b'-', b'.', b'E', b'e', b't', b'r', b'u', b'f', b'a', b'l', b's', b' ', b'x', b'|'] {
        let (a, b) = (new::parse_csv(typed), old::parse_csv(typed));
        assert_eq!(new::to_csv_opts(&a, d, true), old::to_csv_opts(&b, d, true), "{}", d as char);
        assert_eq!(new::to_csv_opts(&a, d, false), old::to_csv_opts(&b, d, false), "{}", d as char);
    }
}

/// Big tables with the real thresholds: pieces of a few megabytes, parts
/// of thousands of rows.
#[test]
fn big_tables_agree() {
    let mut r = Rng::new(77);
    let cases = 2 * scale() as usize;
    for case in 0..cases {
        let kinds: Vec<usize> = (0..5).map(|k| [0, 1, 3, 6, 5, 2, 4][(k + case) % 7]).collect();
        let text = csv_text(&mut r, &kinds, 60_000 + 10_000 * (case % 7), ',', true);
        let (a, b) = (Log::default(), Log::default());
        new::dataframe_method("big", "loadfromcsv", &[v_str(&text)], &a);
        oracle::frame::dataframe_method("big", "loadfromcsv", &[v_str(&text)], &b);
        let (fa, fb) = (new::frame("big"), old::frame("big"));
        assert_eq!(fa.height(), fb.height());
        assert_eq!(new::to_csv(&fa), old::to_csv(&fb));
        assert_eq!(fa.to_text(), fb.to_text());
        let cols = fa.columns.clone();
        let mut calls: Vec<(&str, Vec<Value>)> = Vec::new();
        for c in &cols {
            calls.push(("sort", vec![v_str(c), v_int(0)]));
            calls.push(("sort", vec![v_str(c), v_int(1)]));
            calls.push(("groupby", vec![v_str(c), v_str("mean")]));
            calls.push(("groupby", vec![v_str(c), v_str("median")]));
            calls.push(("groupby", vec![v_str(c), v_str("max")]));
            calls.push(("value_counts", vec![v_str(c)]));
            calls.push(("filter", vec![v_str(c), v_str(">"), v_str("3")]));
            calls.push(("filter", vec![v_str(c), v_str("contains"), v_str("1")]));
            calls.push(("merge", vec![v_str("big2"), v_str(c), v_str("left")]));
        }
        calls.push(("sort", vec![v_str(&format!("{},{}", cols[3], cols[0])), v_int(0)]));
        calls.push(("describe", vec![]));
        calls.push(("dropna", vec![]));
        calls.push(("fillna", vec![v_str("0")]));
        let small = csv_text(&mut r, &kinds, 300, ',', false);
        let small = format!("{}\n{small}", cols.join(","));
        new::dataframe_method("big2", "loadfromcsv", &[v_str(&small)], &a);
        oracle::frame::dataframe_method("big2", "loadfromcsv", &[v_str(&small)], &b);
        for (m, args) in calls {
            new::set_frame("w", fa.clone());
            old::set_frame("w", fb.clone());
            new::dataframe_method("w", m, &args, &a);
            oracle::frame::dataframe_method("w", m, &args, &b);
            let (x, y) = (new::frame("w"), old::frame("w"));
            assert_eq!(x.to_text(), y.to_text(), "{m} {args:?}");
            assert_eq!(new::to_csv(&x), old::to_csv(&y), "{m} {args:?}");
            assert_eq!((0..x.width()).map(|c| x.dtype(c).name()).collect::<Vec<_>>(), (0..y.width()).map(|c| y.dtype(c).name()).collect::<Vec<_>>(), "{m} {args:?}");
        }
        assert_eq!(a.0.borrow().join("\n"), b.0.borrow().join("\n"));
    }
    eprintln!("{cases} tables of 60,000+ rows, 9 calls a column: the engines agree");
}
