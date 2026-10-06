//! RapidR's data-science components — RNUM (a NumPy-style array), RDATAFRAME
//! (a pandas-style table) and RPLOT (a Matplotlib-style chart) — one
//! implementation for native builds, interpreted programs and the web.
//!
//! Every member's behaviour lives here: the arrays, the frames and the
//! charts' models, their methods and properties, the CSV and JSON readers
//! and writers, the text a frame prints as. What a runtime adds is only
//! what can't be shared ([`Host`]): where `PRINT`ed text goes, how a
//! QSTRINGGRID is filled, and where a chart's pixels go. Charts are drawn
//! once for all ([`chart`]: the UI kernel's ops, rendered by
//! rapidr-ui-render — the same pixels on the desktop and the web).
//!
//! **Why our own engine and not polars / ndarray** (docs/ide-plan.md, D7).
//! Until 2.117 the desktop answered these components with ndarray and
//! polars and the web with a second, smaller implementation of its own; the
//! two drifted apart (members missing on the web, different results, a
//! frame that printed only its shape on the desktop). polars can't be the
//! one model: it doesn't fit the browser (a wasm build of its lazy, CSV and
//! JSON engine is several megabytes and needs threads the page's runtime
//! doesn't have), and RapidR's programs work on tables of hundreds or
//! thousands of rows, where a plain row store is as fast as anyone notices.
//! So the model is plain Rust with no dependencies — the same results on
//! all three runtimes, and a smaller program (polars and ndarray are no
//! longer compiled into every native build). A columnar engine can replace
//! [`frame::Frame`] behind the same members if programs ever need it.

pub mod chart;
pub mod colors;
pub mod frame;
pub mod num;
pub mod plot;

use crate::Value;

/// What a runtime does for the shared model.
pub trait Host {
    /// A line of program output, as `PRINT` writes it (`text` has no final
    /// newline; it may have several lines).
    fn print(&self, text: &str);
    /// A diagnostic (a file that can't be read, a column that isn't there):
    /// stderr on the desktop, the console on the web — never the program's
    /// output.
    fn warn(&self, text: &str);
    /// `Frame.ToGrid Grid`: fill QSTRINGGRID `grid` with `rows` (the header
    /// row first, the grid's one fixed row; no fixed column; every row as
    /// long as the first).
    fn to_grid(&self, grid: &str, rows: &[Vec<String>]);
    /// `Plot.SaveFig file [, scale]`: chart `plot` (its model is
    /// [`plot::state`], drawn by [`chart`]) as a PNG in `file`, `scale`
    /// times its size.
    fn save_plot(&self, plot: &str, file: &str, scale: f64);
    /// `Plot.Show` / `Plot.Render` (and a series added with `AddSeries`):
    /// nothing on every runtime today — a chart is shown by
    /// `Image.LoadFromPlot` (kept for a runtime that shows charts itself).
    fn show_plot(&self, plot: &str);
}

/// The key a component's state is stored under (names aren't
/// case-sensitive; the web passes them uppercase, the desktop as written).
pub(crate) fn key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

/// Argument `i` as text ("" if missing).
pub(crate) fn arg_s(args: &[Value], i: usize) -> String {
    args.get(i).map(Value::to_string_val).unwrap_or_default()
}

/// Argument `i` as text, or `default` if missing.
pub(crate) fn arg_s_or(args: &[Value], i: usize, default: &str) -> String {
    args.get(i).map(Value::to_string_val).unwrap_or_else(|| default.to_string())
}

/// Argument `i` as a number, or `default` if missing.
pub(crate) fn arg_f(args: &[Value], i: usize, default: f64) -> f64 {
    args.get(i).map(Value::to_f64).unwrap_or(default)
}

/// Argument `i` as an integer, or `default` if missing (`"3"` and 3.0 are 3).
pub(crate) fn arg_i(args: &[Value], i: usize, default: i64) -> i64 {
    match args.get(i) {
        Some(Value::String(s)) => s.trim().parse::<f64>().map(|f| f as i64).unwrap_or(default),
        Some(v) => v.to_i64(),
        None => default,
    }
}

/// A number as the components show one they computed: RapidR's own text
/// conversion (15 significant digits, whole numbers without a point, so
/// `0.1 + 0.2` is `0.3`).
pub fn num_text(v: f64) -> String {
    crate::format::float_to_str(v)
}

/// Text that is a number (`12`, `-3.5`, `1e3`) — not words like `inf` or
/// `NaN` that Rust's parser also takes.
pub(crate) fn parse_num(s: &str) -> Option<f64> {
    let t = s.trim();
    let first = t.chars().next()?;
    if !(first.is_ascii_digit() || matches!(first, '-' | '+' | '.')) {
        return None;
    }
    t.parse::<f64>().ok().filter(|f| f.is_finite())
}

/// The numbers in `text` separated by commas (others skipped).
pub(crate) fn parse_list(text: &str) -> Vec<f64> {
    text.split(',').filter_map(parse_num).collect()
}

/// Random numbers: the runtime's RANDOMIZE'd generator (the same one RND
/// uses).
pub(crate) fn random_unit() -> f64 {
    crate::builtins::random_unit()
}

pub(crate) fn random_index(n: usize) -> usize {
    if n == 0 {
        0
    } else {
        crate::builtins::random_index(n).min(n - 1)
    }
}

#[cfg(test)]
pub(crate) mod test_host {
    use std::cell::RefCell;

    /// A host that records what the model asked of it.
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
