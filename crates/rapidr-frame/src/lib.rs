//! RDATAFRAME — a table in the manner of pandas, on polars: the one
//! implementation every RapidR runtime uses (decision D7, docs/ide-plan.md).
//! The desktop runtime (native and interpreted programs) links this crate;
//! the web loads it as a wasm module of its own (`rapidr-frame-web`), only
//! when a program uses data frames.
//!
//! The interface is pure, so it crosses a module boundary as data: a
//! [`Call`] in — the frame's name, the method, its arguments, the bytes of
//! a file the method reads (the runtime reads it: [`file_arg`]), a seed
//! from the program's random numbers — and a [`Reply`] out: the value, text
//! to print, a file to write, a grid to fill, an error to report. Frames
//! live here by name (not case-sensitive), as the runtimes' other
//! components do.
//!
//! Column names are matched without regard to case (BASIC's way); methods
//! change the frame in place; what a cell reads is its text (`""` for a
//! missing value). Where the two former implementations differed, the
//! desktop's meaning was kept — `SetCell(row, column, value)` as
//! `Cell(row, column)` — and the web's extra members (`Create`, `AddRow`, a
//! CSV text for `LoadFromCsv`) joined it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;

use polars::prelude::*;
use serde::{Deserialize, Serialize};

/// An argument as BASIC passed it: its text, its number, and whether it
/// was a number (rather than a string).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Arg {
    pub text: String,
    pub num: f64,
    pub is_num: bool,
}

impl Arg {
    pub fn text(s: &str) -> Arg {
        Arg { text: s.to_string(), num: s.trim().parse().unwrap_or(0.0), is_num: false }
    }
    pub fn num(n: f64, text: String) -> Arg {
        Arg { text, num: n, is_num: true }
    }
    fn int(&self) -> i64 {
        if self.is_num {
            self.num as i64
        } else {
            self.text.trim().parse::<f64>().map(|f| f as i64).unwrap_or(0)
        }
    }
}

/// A value a method returns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Out {
    Null,
    Int(i64),
    Dbl(f64),
    Str(String),
}

/// One method call.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Call {
    pub name: String,
    /// Lower case.
    pub method: String,
    pub args: Vec<Arg>,
    /// The bytes of the file [`file_arg`] named (or why it couldn't be read).
    #[serde(skip)]
    pub file: Option<Result<Vec<u8>, String>>,
    /// From the program's random numbers (RANDOMIZE): what `Sample` draws with.
    pub seed: u64,
}

/// What a call did.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reply {
    pub value: Out,
    /// Text to PRINT (a line each).
    pub print: Option<String>,
    /// A file to write: its name (as the program gave it) and contents.
    pub write: Option<(String, Vec<u8>)>,
    /// A QSTRINGGRID to fill: its name and rows (the column names first).
    pub grid: Option<(String, Vec<Vec<String>>)>,
    /// A problem to report (the runtime's warning stream).
    pub error: Option<String>,
}

impl Reply {
    fn value(value: Out) -> Reply {
        Reply { value, print: None, write: None, grid: None, error: None }
    }
    fn null() -> Reply {
        Reply::value(Out::Null)
    }
    fn error(method: &str, e: impl std::fmt::Display) -> Reply {
        Reply { error: Some(format!("RDataFrame.{method}: {e}")), ..Reply::null() }
    }
}

thread_local! {
    static FRAMES: RefCell<HashMap<String, DataFrame>> = RefCell::new(HashMap::new());
}

fn get(name: &str) -> DataFrame {
    FRAMES.with(|m| m.borrow().get(&name.to_lowercase()).cloned().unwrap_or_default())
}

fn set(name: &str, df: DataFrame) {
    FRAMES.with(|m| {
        m.borrow_mut().insert(name.to_lowercase(), df);
    });
}

/// The file a call reads, when it reads one: the runtime passes its bytes
/// in [`Call::file`]. (`LoadFromCsv` given CSV text itself — with a line
/// break in it — reads none.)
pub fn file_arg(method: &str, args: &[Arg]) -> Option<String> {
    let path = args.first().map(|a| a.text.clone()).unwrap_or_default();
    match method {
        "loadfromcsv" | "readcsv" | "read_csv" | "loadfromjson" | "read_json" if !path.contains('\n') => Some(path),
        _ => None,
    }
}

/// The frame's column called `name`, without regard to case.
fn column_name(df: &DataFrame, name: &str) -> Option<PlSmallStr> {
    let names = df.get_column_names();
    names.iter().find(|n| n.as_str() == name).or_else(|| names.iter().find(|n| n.eq_ignore_ascii_case(name))).map(|n| (*n).clone())
}

/// A cell as text: strings as they are, numbers as polars shows them,
/// nothing for a missing value.
fn cell_text(av: AnyValue) -> String {
    match av {
        AnyValue::Null => String::new(),
        AnyValue::String(s) => s.to_string(),
        AnyValue::StringOwned(s) => s.to_string(),
        other => format!("{other}"),
    }
}

fn cell(df: &DataFrame, row: usize, col: usize) -> String {
    if col < df.width() && row < df.height() {
        df.get_columns()[col].get(row).map(cell_text).unwrap_or_default()
    } else {
        String::new()
    }
}

/// A column of texts typed as they read: all integers → Int64, all
/// numbers → Float64, else strings (empty texts are missing values).
fn typed(name: &str, values: &[String]) -> Column {
    let present: Vec<&str> = values.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let opt = |s: &String| if s.trim().is_empty() { None } else { Some(s.trim().to_string()) };
    if !present.is_empty() && present.iter().all(|s| s.parse::<i64>().is_ok()) {
        let v: Vec<Option<i64>> = values.iter().map(|s| opt(s).and_then(|t| t.parse().ok())).collect();
        return Column::new(name.into(), v);
    }
    if !present.is_empty() && present.iter().all(|s| s.parse::<f64>().is_ok()) {
        let v: Vec<Option<f64>> = values.iter().map(|s| opt(s).and_then(|t| t.parse().ok())).collect();
        return Column::new(name.into(), v);
    }
    Column::new(name.into(), values.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn read_csv(bytes: Vec<u8>) -> PolarsResult<DataFrame> {
    CsvReadOptions::default().with_has_header(true).into_reader_with_file_handle(Cursor::new(bytes)).finish()
}

fn rows_of(df: &DataFrame) -> Vec<Vec<String>> {
    let mut rows = vec![df.get_column_names().iter().map(|c| c.to_string()).collect::<Vec<_>>()];
    for r in 0..df.height() {
        rows.push((0..df.width()).map(|c| cell(df, r, c)).collect());
    }
    rows
}

/// A column's values as numbers (what isn't one is skipped).
fn floats(df: &DataFrame, col: &str) -> Option<Vec<f64>> {
    let name = column_name(df, col)?;
    let s = df.column(&name).ok()?.as_materialized_series().cast(&DataType::Float64).ok()?;
    Some(s.f64().ok()?.into_iter().flatten().collect())
}

/// Calls RDATAFRAME method `call.method`; `None` when there's no such.
pub fn call(call: Call) -> Option<Reply> {
    let Call { name, method, args, file, seed } = call;
    let m = method.as_str();
    let a = |i: usize| args.get(i).cloned().unwrap_or_default();
    let s = |i: usize| args.get(i).map(|a| a.text.clone()).unwrap_or_default();
    let n = |i: usize, d: i64| args.get(i).map(Arg::int).unwrap_or(d);
    let df = || get(&name);
    let store = |r: PolarsResult<DataFrame>| match r {
        Ok(d) => {
            set(&name, d);
            Reply::null()
        }
        Err(e) => Reply::error(m, e),
    };
    let col = |d: &DataFrame, i: usize| column_name(d, &s(i));
    Some(match m {
        // --- I/O ---
        "loadfromcsv" | "readcsv" | "read_csv" => {
            let bytes = match file {
                Some(Ok(b)) => b,
                Some(Err(e)) => return Some(Reply::error(m, e)),
                None => s(0).into_bytes(),
            };
            store(read_csv(bytes))
        }
        "loadfromjson" | "read_json" => {
            let bytes = match file {
                Some(Ok(b)) => b,
                Some(Err(e)) => return Some(Reply::error(m, e)),
                None => s(0).into_bytes(),
            };
            store(JsonReader::new(Cursor::new(bytes)).finish())
        }
        "savetocsv" | "to_csv" | "writecsv" => {
            let path = if s(0).is_empty() { "output.csv".to_string() } else { s(0) };
            let mut out = Vec::new();
            match CsvWriter::new(&mut out).finish(&mut df()) {
                Ok(()) => Reply { write: Some((path, out)), ..Reply::null() },
                Err(e) => Reply::error(m, e),
            }
        }
        "savetojson" | "to_json" => {
            let path = if s(0).is_empty() { "output.json".to_string() } else { s(0) };
            let mut out = Vec::new();
            match JsonWriter::new(&mut out).with_json_format(JsonFormat::Json).finish(&mut df()) {
                Ok(()) => Reply { write: Some((path, out)), ..Reply::null() },
                Err(e) => Reply::error(m, e),
            }
        }

        // --- Building ---
        "create" | "new" | "init" | "clear" => {
            set(&name, DataFrame::empty());
            Reply::null()
        }
        "addrow" | "add_row" | "appendrow" | "push_row" => {
            let vals: Vec<String> = args.iter().map(|a| a.text.clone()).collect();
            let d = df();
            if d.width() == 0 {
                let cols: Vec<Column> = vals.iter().enumerate().map(|(i, v)| typed(&format!("column_{}", i + 1), std::slice::from_ref(v))).collect();
                return Some(store(DataFrame::new(cols)));
            }
            // (each value in its column's type; a column that can't hold it becomes text)
            let mut base = d.clone();
            let mut row = Vec::new();
            for (i, c) in d.get_columns().iter().enumerate() {
                let v = vals.get(i).cloned().unwrap_or_default();
                let text = Series::new(c.name().clone(), [if v.trim().is_empty() { None } else { Some(v.trim().to_string()) }]);
                let fits = text.cast(c.dtype()).ok().filter(|t| t.null_count() == text.null_count());
                match fits {
                    Some(t) => row.push(t.into_column()),
                    None => {
                        let as_text = c.cast(&DataType::String).ok()?;
                        let _ = base.replace(c.name().as_str(), as_text.take_materialized_series());
                        row.push(text.into_column());
                    }
                }
            }
            match DataFrame::new(row) {
                Ok(r) => store(base.vstack(&r)),
                Err(e) => Reply::error(m, e),
            }
        }
        "addcolumn" | "add_column" | "set_column" => {
            let mut d = df();
            let vals: Vec<String> = s(1).split(',').map(|v| v.trim().to_string()).collect();
            let vals = if d.width() == 0 {
                vals
            } else {
                let mut v = vals;
                v.resize(d.height(), String::new());
                v
            };
            let column = typed(&s(0), &vals);
            match d.with_column(column) {
                Ok(_) => store(Ok(d)),
                Err(e) => Reply::error(m, e),
            }
        }

        // --- Selection / indexing ---
        "head" => store(Ok(df().head(Some(n(0, 5).max(0) as usize)))),
        "tail" => store(Ok(df().tail(Some(n(0, 5).max(0) as usize)))),
        "cell" => Reply::value(Out::Str(cell(&df(), n(0, 0).max(0) as usize, n(1, 0).max(0) as usize))),
        "cellbyname" | "at" => {
            let d = df();
            let v = col(&d, 1).and_then(|c| d.get_column_index(&c)).map(|ci| cell(&d, n(0, 0).max(0) as usize, ci)).unwrap_or_default();
            Reply::value(Out::Str(v))
        }
        "setcell" => {
            let (row, ci, val) = (n(0, 0).max(0) as usize, n(1, 0).max(0) as usize, s(2));
            let mut d = df();
            if ci < d.width() && row < d.height() {
                let c = d.get_columns()[ci].clone();
                let mut texts: Vec<String> = (0..c.len()).map(|i| c.get(i).map(cell_text).unwrap_or_default()).collect();
                texts[row] = val;
                let mut new = typed(c.name().as_str(), &texts);
                // (the column keeps its type when the value fits it)
                if let Ok(back) = new.cast(c.dtype()) {
                    if back.null_count() == new.null_count() {
                        new = back;
                    }
                }
                let _ = d.replace_column(ci, new);
                set(&name, d);
            }
            Reply::null()
        }
        "iloc" => {
            let d = df();
            let end = (if args.len() > 1 { n(1, 0) } else { d.height() as i64 }).clamp(0, d.height() as i64) as usize;
            let start = (n(0, 0).max(0) as usize).min(end);
            store(Ok(d.slice(start as i64, end - start)))
        }
        "select" => {
            let d = df();
            let names: Vec<PlSmallStr> = s(0).split(',').map(|c| column_name(&d, c.trim()).unwrap_or_else(|| c.trim().into())).collect();
            store(d.select(names))
        }
        "column" | "col" => {
            let d = df();
            let v = col(&d, 0).and_then(|c| d.column(&c).ok().cloned()).map(|c| (0..c.len()).map(|i| c.get(i).map(cell_text).unwrap_or_default()).collect::<Vec<_>>().join(",")).unwrap_or_default();
            Reply::value(Out::Str(v))
        }

        // --- Sorting ---
        "sort" | "sort_values" => {
            let d = df();
            let ascending = args.get(1).map(|a| a.int() != 0).unwrap_or(true);
            let cols: Vec<PlSmallStr> = s(0).split(',').map(|c| column_name(&d, c.trim()).unwrap_or_else(|| c.trim().into())).collect();
            store(d.sort(cols, SortMultipleOptions::default().with_order_descending(!ascending).with_maintain_order(true)))
        }
        "nlargest" | "nsmallest" => {
            let d = df();
            let Some(c) = col(&d, 0) else { return Some(Reply::error(m, format!("no column {}", s(0)))) };
            let opts = SortMultipleOptions::default().with_order_descending(m == "nlargest").with_maintain_order(true);
            match d.sort([c], opts) {
                Ok(sorted) => store(Ok(sorted.head(Some(n(1, 5).max(0) as usize)))),
                Err(e) => Reply::error(m, e),
            }
        }

        // --- Filtering ---
        "filter" | "query" => {
            let (cname, op, val) = if m == "query" {
                let q = s(0);
                let parts: Vec<&str> = q.splitn(3, ' ').collect();
                if parts.len() < 3 {
                    return Some(Reply::error(m, "expected \"column op value\""));
                }
                (parts[0].to_string(), parts[1].to_string(), Arg::text(parts[2].trim_matches('"')))
            } else {
                (s(0), s(1), a(2))
            };
            let d = df();
            let Some(c) = column_name(&d, &cname) else { return Some(Reply::error(m, format!("no column {cname}"))) };
            let op = op.trim().to_lowercase();
            if op == "contains" {
                let texts = d.column(&c).ok()?.cast(&DataType::String).ok()?;
                let mask: BooleanChunked = texts.str().ok()?.into_iter().map(|v| Some(v.is_some_and(|t| t.contains(val.text.as_str())))).collect();
                return Some(store(d.filter(&mask)));
            }
            let num = val.is_num || val.text.trim().parse::<f64>().is_ok();
            let x = if val.is_num { val.num } else { val.text.trim().parse::<f64>().unwrap_or(0.0) };
            let e = polars::lazy::dsl::col(c.clone());
            let (f, t) = (e.clone().cast(DataType::Float64), e.cast(DataType::String));
            let cond = match op.as_str() {
                "==" | "=" if num => f.eq(lit(x)),
                "==" | "=" => t.eq(lit(val.text.clone())),
                "!=" | "<>" if num => f.neq(lit(x)),
                "!=" | "<>" => t.neq(lit(val.text.clone())),
                ">" => f.gt(lit(x)),
                "<" => f.lt(lit(x)),
                ">=" => f.gt_eq(lit(x)),
                "<=" => f.lt_eq(lit(x)),
                other => return Some(Reply::error(m, format!("unknown comparison {other}"))),
            };
            store(d.lazy().filter(cond).collect())
        }

        // --- Grouping ---
        "groupby" | "group_by" => {
            let d = df();
            let Some(c) = col(&d, 0) else { return Some(Reply::error(m, format!("no column {}", s(0)))) };
            let rest = polars::lazy::dsl::all().exclude([c.as_str()]);
            let agg = match s(1).to_lowercase().as_str() {
                "sum" => rest.sum(),
                "count" => rest.count(),
                "min" => rest.min(),
                "max" => rest.max(),
                "first" => rest.first(),
                "last" => rest.last(),
                "median" => rest.median(),
                "std" => rest.std(1),
                _ => rest.mean(),
            };
            // (groups in their keys' order: the same on every runtime)
            store(d.lazy().group_by([polars::lazy::dsl::col(c.clone())]).agg([agg]).sort([c.as_str()], Default::default()).collect())
        }

        // --- Columns ---
        "drop" | "drop_column" => {
            let d = df();
            match col(&d, 0) {
                Some(c) => store(d.drop(&c)),
                None => Reply::error(m, format!("no column {}", s(0))),
            }
        }
        "rename" | "rename_column" => {
            let mut d = df();
            match col(&d, 0) {
                Some(c) => match d.rename(&c, s(1).as_str().into()) {
                    Ok(_) => store(Ok(d)),
                    Err(e) => Reply::error(m, e),
                },
                None => Reply::error(m, format!("no column {}", s(0))),
            }
        }

        // --- Missing data ---
        "fillna" | "fill_null" => {
            let fill = if args.is_empty() { Arg::num(0.0, "0".into()) } else { a(0) };
            let d = df();
            let mut cols = Vec::new();
            for c in d.get_columns() {
                let filled = if c.dtype().is_primitive_numeric() && (fill.is_num || fill.text.trim().parse::<f64>().is_ok()) {
                    let x = if fill.is_num { fill.num } else { fill.text.trim().parse().unwrap_or(0.0) };
                    let ones = Column::new(c.name().clone(), vec![x; c.len()]).cast(c.dtype());
                    ones.and_then(|o| c.as_materialized_series().zip_with(&c.is_not_null(), o.as_materialized_series())).map(Column::from)
                } else if c.dtype() == &DataType::String || c.null_count() > 0 {
                    let t = c.cast(&DataType::String);
                    t.and_then(|t| {
                        let fills = Series::new(c.name().clone(), vec![fill.text.clone(); c.len()]);
                        t.as_materialized_series().zip_with(&t.is_not_null(), &fills).map(Column::from)
                    })
                } else {
                    Ok(c.clone())
                };
                match filled {
                    Ok(f) => cols.push(f),
                    Err(e) => return Some(Reply::error(m, e)),
                }
            }
            store(DataFrame::new(cols))
        }
        "dropna" | "drop_nulls" => store(df().drop_nulls::<String>(None)),

        // --- Statistics ---
        "describe" => store(describe(&df())),
        "value_counts" => {
            let d = df();
            let Some(c) = col(&d, 0) else { return Some(Reply::error(m, format!("no column {}", s(0)))) };
            let s0 = d.column(&c).ok()?.as_materialized_series().clone();
            match s0.value_counts(false, false, "count".into(), false) {
                // (the most frequent first, ties by value: the same everywhere)
                Ok(vc) => store(vc.sort([PlSmallStr::from("count"), c.clone()], SortMultipleOptions::default().with_order_descending_multi([true, false]).with_maintain_order(true))),
                Err(e) => Reply::error(m, e),
            }
        }
        "nunique" => {
            let d = df();
            let v = col(&d, 0).and_then(|c| d.column(&c).ok().and_then(|c| c.as_materialized_series().n_unique().ok())).unwrap_or(0);
            Reply::value(Out::Int(v as i64))
        }
        "corr" | "correlation" => {
            let d = df();
            let (va, vb) = (floats(&d, &s(0)).unwrap_or_default(), floats(&d, &s(1)).unwrap_or_default());
            let k = va.len().min(vb.len()) as f64;
            let mut r = 0.0;
            if k > 1.0 {
                let (ma, mb) = (va.iter().sum::<f64>() / k, vb.iter().sum::<f64>() / k);
                let cov: f64 = va.iter().zip(&vb).map(|(x, y)| (x - ma) * (y - mb)).sum::<f64>() / k;
                let sa = (va.iter().map(|x| (x - ma).powi(2)).sum::<f64>() / k).sqrt();
                let sb = (vb.iter().map(|y| (y - mb).powi(2)).sum::<f64>() / k).sqrt();
                if sa > 0.0 && sb > 0.0 {
                    r = cov / (sa * sb);
                }
            }
            Reply::value(Out::Dbl(r))
        }
        "sample" => {
            let d = df();
            let k = (n(0, 5).max(0) as usize).min(d.height());
            store(d.sample_n_literal(k, false, true, Some(seed)))
        }

        // --- Shape / information ---
        "info" => {
            let d = df();
            let mut t = format!("DataFrame: {} rows x {} columns", d.height(), d.width());
            for c in d.get_columns() {
                t += &format!("\n  {}: {} ({})", c.name(), c.dtype(), c.len());
            }
            Reply { print: Some(t), ..Reply::null() }
        }
        "dtypes" => Reply::value(Out::Str(df().get_columns().iter().map(|c| format!("{}: {}", c.name(), c.dtype())).collect::<Vec<_>>().join(","))),
        "shape" => {
            let d = df();
            Reply::value(Out::Str(format!("({}, {})", d.height(), d.width())))
        }
        "columns" => Reply::value(Out::Str(df().get_column_names().iter().map(|c| c.as_str()).collect::<Vec<_>>().join(","))),
        "rows" | "rowcount" | "len" => Reply::value(Out::Int(df().height() as i64)),

        // --- Joins ---
        "merge" | "join" => {
            let (d, other) = (df(), get(&s(0)));
            let how = match s(2).to_lowercase().as_str() {
                "left" => JoinType::Left,
                "right" => JoinType::Right,
                "outer" | "full" => JoinType::Full,
                "cross" => JoinType::Cross,
                _ => JoinType::Inner,
            };
            let (Some(l), Some(r)) = (col(&d, 1), column_name(&other, &s(1))) else {
                return Some(Reply::error(m, format!("no column {} in both frames", s(1))));
            };
            let mut ja = JoinArgs::new(how);
            ja.maintain_order = MaintainOrderJoin::LeftRight;
            store(d.join(&other, [l.as_str()], [r.as_str()], ja, None))
        }
        "concat" | "append" => store(df().vstack(&get(&s(0)))),

        // --- Transforms ---
        "transpose" | "t" => store(df().transpose(None, None)),
        "apply" => {
            let d = df();
            let Some(c) = col(&d, 0) else { return Some(Reply::error(m, format!("no column {}", s(0)))) };
            let e = polars::lazy::dsl::col(c.clone());
            let e = match s(1).to_lowercase().as_str() {
                "upper" => e.cast(DataType::String).str().to_uppercase(),
                "lower" => e.cast(DataType::String).str().to_lowercase(),
                "abs" => e.abs(),
                "round" => e.round(n(2, 2).clamp(0, 15) as u32),
                "sqrt" => e.cast(DataType::Float64).sqrt(),
                "log" => e.cast(DataType::Float64).log(std::f64::consts::E),
                other => return Some(Reply::error(m, format!("unknown operation {other}"))),
            };
            store(d.lazy().with_columns([e.alias(c)]).collect())
        }
        "replace" => {
            let d = df();
            let Some(c) = col(&d, 0) else { return Some(Reply::error(m, format!("no column {}", s(0)))) };
            let e = polars::lazy::dsl::col(c.clone()).cast(DataType::String);
            let swapped = polars::lazy::dsl::when(e.clone().eq(lit(s(1)))).then(lit(s(2))).otherwise(e).alias(c);
            store(d.lazy().with_columns([swapped]).collect())
        }

        // --- Output ---
        "tostring" => Reply::value(Out::Str(format!("{}", df()))),
        "show" | "print" => {
            let t = format!("{}", df());
            Reply { print: Some(t.clone()), ..Reply::value(Out::Str(t)) }
        }
        "togrid" | "to_grid" | "display" => {
            let g = s(0);
            if g.is_empty() {
                Reply::null()
            } else {
                Reply { grid: Some((g, rows_of(&df()))), ..Reply::null() }
            }
        }
        // (a property read as a method: `PRINT df.Empty`)
        other => return get_prop(&name, other).map(Reply::value),
    })
}

/// pandas' describe: count, mean, std, min, 25 %, 50 %, 75 %, max of each
/// column (its values as numbers), the statistic's name first.
fn describe(d: &DataFrame) -> PolarsResult<DataFrame> {
    let mut cols: Vec<Column> = vec![Column::new("statistic".into(), ["count", "mean", "std", "min", "25%", "50%", "75%", "max"])];
    for c in d.get_columns() {
        let count = format!("{}", c.len() - c.null_count());
        let numeric = c.dtype().is_primitive_numeric();
        let values: Vec<String> = match numeric.then(|| c.cast(&DataType::Float64).ok()).flatten() {
            Some(f) => {
                let mut v: Vec<f64> = f.f64()?.into_iter().flatten().collect();
                v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let k = v.len();
                let mean = if k > 0 { v.iter().sum::<f64>() / k as f64 } else { 0.0 };
                let std = if k > 1 { (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (k - 1) as f64).sqrt() } else { 0.0 };
                // (linear interpolation between the closest ranks, as pandas)
                let q = |p: f64| -> f64 {
                    if v.is_empty() {
                        return 0.0;
                    }
                    let pos = p * (k as f64 - 1.0);
                    let (i, frac) = (pos.floor() as usize, pos - pos.floor());
                    v[i] + (v[(i + 1).min(k - 1)] - v[i]) * frac
                };
                let f4 = |x: f64| format!("{x:.4}");
                vec![count, f4(mean), f4(std), f4(q(0.0)), f4(q(0.25)), f4(q(0.5)), f4(q(0.75)), f4(q(1.0))]
            }
            None => vec![count, String::new(), String::new(), String::new(), String::new(), String::new(), String::new(), String::new()],
        };
        cols.push(Column::new(c.name().clone(), values));
    }
    DataFrame::new(cols)
}

/// Reads RDATAFRAME property `prop` (lower case); `None` when there's no such.
pub fn get_prop(name: &str, prop: &str) -> Option<Out> {
    let d = get(name);
    Some(match prop {
        "rowcount" | "height" | "nrows" => Out::Int(d.height() as i64),
        "colcount" | "width" | "ncols" => Out::Int(d.width() as i64),
        "columns" => Out::Str(d.get_column_names().iter().map(|c| c.as_str()).collect::<Vec<_>>().join(",")),
        "shape" => Out::Str(format!("({}, {})", d.height(), d.width())),
        "empty" => Out::Int(i64::from(d.height() == 0)),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(name: &str, method: &str, args: &[&str]) -> Reply {
        call(Call { name: name.into(), method: method.into(), args: args.iter().map(|a| Arg::text(a)).collect(), file: None, seed: 7 }).expect(method)
    }

    const STAFF: &str = "Name,Department,Salary\nAlice,Engineering,85000\nBob,Sales,62000\nCharlie,Engineering,92000\nDiana,Sales,58000\n";

    #[test]
    fn load_filter_sort_cells() {
        run("t1", "loadfromcsv", &[STAFF]);
        assert_eq!(get_prop("t1", "shape"), Some(Out::Str("(4, 3)".into())));
        assert_eq!(run("t1", "cell", &["0", "0"]).value, Out::Str("Alice".into()));
        assert_eq!(run("t1", "cellbyname", &["1", "salary"]).value, Out::Str("62000".into()));
        run("t1", "filter", &["department", "=", "Engineering"]);
        run("t1", "sort", &["Salary", "0"]);
        assert_eq!(run("t1", "column", &["Name"]).value, Out::Str("Charlie,Alice".into()));
        run("t1", "setcell", &["0", "2", "95000"]);
        assert_eq!(run("t1", "cell", &["0", "2"]).value, Out::Str("95000".into()));
        assert!(run("t1", "tostring", &[]).value != Out::Str(String::new()));
    }

    #[test]
    fn group_describe_grid_and_building() {
        run("t2", "loadfromcsv", &[STAFF]);
        run("t2", "groupby", &["Department", "mean"]);
        assert_eq!(run("t2", "columns", &[]).value, Out::Str("Department,Name,Salary".into()));
        assert_eq!(run("t2", "cell", &["1", "0"]).value, Out::Str("Sales".into()));
        assert_eq!(run("t2", "cell", &["1", "2"]).value, Out::Str("60000.0".into()));
        run("t3", "create", &[]);
        run("t3", "addcolumn", &["x", "1,2,3"]);
        run("t3", "addcolumn", &["y", "a,b"]);
        run("t3", "addrow", &["4", "d"]);
        assert_eq!(get_prop("t3", "shape"), Some(Out::Str("(4, 2)".into())));
        assert_eq!(run("t3", "cell", &["2", "1"]).value, Out::Str("".into()));
        let g = run("t3", "togrid", &["Grid1"]).grid.unwrap();
        assert_eq!(g.1[0], vec!["x", "y"]);
        assert_eq!(g.1[4], vec!["4", "d"]);
        run("t3", "describe", &[]);
        assert_eq!(run("t3", "cell", &["1", "1"]).value, Out::Str("2.5000".into()));
        assert!(call(Call { method: "nonsense".into(), ..Default::default() }).is_none());
    }
}
