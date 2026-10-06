//! RPLOT: charts in the manner of Matplotlib — one implementation for every
//! runtime. A chart is a model (its series, axes and labels, by component
//! name) that draws itself as the UI kernel's vector ops ([`ops`]): the
//! kernel's renderer turns them into pixels at the screen's scale (crisp at
//! 1×, 2×, 3×), in the kernel's fonts, the same on the desktop and the web.
//! The runtimes put them into a QIMAGE (`LoadFromPlot`) or a PNG
//! (`SaveFig`).
//!
//! The look: nice tick steps (1, 2, 2.5, 5 × 10ⁿ), category axes (bars over
//! their names), light horizontal gridlines (`Grid = 1`: both ways,
//! `Grid = 0`: none), a legend with swatches where it covers the fewest
//! points, grouped bars, a modern palette, and the current theme's colours
//! (a dark theme draws a dark chart).

use std::cell::RefCell;
use std::collections::HashMap;

use super::colors::{self, mix};
use super::num;
use crate::objects::font::Font;
use crate::objects::ops::{Op, Place, Rect};
use crate::objects::text::text_size;
use crate::theme::Theme;
use crate::{v_int, v_null, v_str, Value};

/// What a series draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Line,
    Dashed,
    Dotted,
    /// A line with a marker at each point (`"o-"`).
    LineMarkers,
    Scatter,
    Bar,
    /// Horizontal bars: `x` their positions (down the y axis), `y` their lengths.
    Barh,
    Hist,
    Step,
    Area,
    Pie,
    HLine,
    VLine,
}

#[derive(Clone, Debug)]
pub struct Series {
    pub kind: Kind,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// The category names `x` stands for (bars over names), if any.
    pub categories: Vec<String>,
    pub label: String,
    /// None: the palette's next.
    pub color: Option<u32>,
    /// A pie's slice names and colours.
    pub slice_labels: Vec<String>,
    pub slice_colors: Vec<u32>,
    /// A histogram's bin width.
    pub bin: f64,
}

impl Series {
    fn new(kind: Kind, x: Vec<f64>, y: Vec<f64>, label: String, color: Option<u32>) -> Self {
        Series { kind, x, y, categories: Vec::new(), label, color, slice_labels: Vec::new(), slice_colors: Vec::new(), bin: 0.0 }
    }

    fn is_bar(&self) -> bool {
        matches!(self.kind, Kind::Bar | Kind::Barh | Kind::Hist)
    }
}

/// Text at a data point (`Annotate`).
#[derive(Clone, Debug)]
pub struct Note {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub color: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Plot {
    pub title: String,
    pub xlabel: String,
    pub ylabel: String,
    /// None: horizontal gridlines (the default look); Some(true): both
    /// ways; Some(false): none.
    pub grid: Option<bool>,
    /// In pixels (below 100: inches, at `dpi`).
    pub width: i64,
    pub height: i64,
    pub dpi: i64,
    pub series: Vec<Series>,
    pub notes: Vec<Note>,
    pub legend: bool,
    pub xlim: Option<(f64, f64)>,
    pub ylim: Option<(f64, f64)>,
    pub xlog: bool,
    pub ylog: bool,
    /// `XTicks`: the x axis' tick positions and names.
    pub xticks: Option<(Vec<f64>, Vec<String>)>,
}

impl Default for Plot {
    fn default() -> Self {
        Plot {
            title: String::new(),
            xlabel: String::new(),
            ylabel: String::new(),
            grid: None,
            width: 640,
            height: 480,
            dpi: 100,
            series: Vec::new(),
            notes: Vec::new(),
            legend: false,
            xlim: None,
            ylim: None,
            xlog: false,
            ylog: false,
            xticks: None,
        }
    }
}

impl Plot {
    /// Its size in pixels.
    pub fn size(&self) -> (i64, i64) {
        let dpi = self.dpi.max(50);
        let px = |v: i64| if v < 100 { v.max(1) * dpi } else { v };
        (px(self.width).min(8192), px(self.height).min(8192))
    }
}

thread_local! {
    static PLOTS: RefCell<HashMap<String, Plot>> = RefCell::new(HashMap::new());
}

/// Chart `name` (a new one's defaults when there is none).
pub fn get(name: &str) -> Plot {
    PLOTS.with(|m| m.borrow().get(&name.to_lowercase()).cloned().unwrap_or_default())
}

fn with(name: &str, f: impl FnOnce(&mut Plot)) {
    PLOTS.with(|m| f(m.borrow_mut().entry(name.to_lowercase()).or_default()));
}

/// Whether `method` makes the chart's picture (the runtime's to do: a PNG
/// file, the web's page).
pub fn is_output(method: &str) -> bool {
    matches!(method, "savefig" | "save" | "render" | "show")
}

fn text(args: &[Value], i: usize) -> String {
    args.get(i).map(Value::to_string_val).unwrap_or_default()
}

fn color_arg(args: &[Value], i: usize) -> Option<u32> {
    match args.get(i)? {
        Value::Integer(n) => Some(colors::bgr_to_rgb((*n as u32) & 0xFF_FFFF)),
        Value::Double(f) => Some(colors::bgr_to_rgb((*f as u32) & 0xFF_FFFF)),
        v => colors::parse(&v.to_string_val()),
    }
}

/// Category names, when `v` is a list of names (not an array, not numbers).
fn categories_of(v: &Value) -> Option<Vec<String>> {
    if matches!(v, Value::Integer(_) | Value::Double(_) | Value::Null) {
        return None;
    }
    let s = v.to_string_val();
    if s.trim().is_empty() || num::exists(&s) {
        return None;
    }
    let items: Vec<String> = s.split(',').map(|t| t.trim().to_string()).collect();
    items.iter().any(|t| !t.is_empty() && t.parse::<f64>().is_err()).then_some(items)
}

/// A series' x and y from its first two arguments: arrays (by component or
/// name), lists of numbers, or names (categories). One argument only: the
/// values, at 0, 1, 2 …
fn xy(args: &[Value]) -> (Vec<f64>, Vec<f64>, Vec<String>) {
    let ya = args.get(1).filter(|v| !v.to_string_val().is_empty());
    let Some(ya) = ya else {
        let y = args.first().map(num::values_of).unwrap_or_default();
        return ((0..y.len()).map(|i| i as f64).collect(), y, Vec::new());
    };
    let y = num::values_of(ya);
    match args.first().and_then(categories_of) {
        Some(cats) => ((0..cats.len()).map(|i| i as f64).collect(), y, cats),
        None => (args.first().map(num::values_of).unwrap_or_default(), y, Vec::new()),
    }
}

fn add(name: &str, mut s: Series) {
    with(name, |p| {
        if s.color.is_none() && s.kind != Kind::Pie {
            let used = p.series.iter().filter(|o| !matches!(o.kind, Kind::HLine | Kind::VLine | Kind::Pie)).count();
            s.color = Some(colors::auto(used));
        }
        p.series.push(s)
    });
}

/// Calls RPLOT method `method` (lower case) on chart `name`; `None` when
/// RPLOT has no such method (the output methods are the runtime's:
/// [`is_output`]).
pub fn method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    match method {
        "create" | "new" | "init" => with(name, |_| {}),
        "clear" => PLOTS.with(|m| {
            // (the size stays: it's the picture's)
            let mut m = m.borrow_mut();
            let old = m.remove(&name.to_lowercase()).unwrap_or_default();
            m.insert(name.to_lowercase(), Plot { width: old.width, height: old.height, dpi: old.dpi, ..Plot::default() });
        }),
        "plot" | "line" => {
            let (x, y, cats) = xy(args);
            let kind = match text(args, 4).trim() {
                "--" | "dashed" => Kind::Dashed,
                ":" | "dotted" => Kind::Dotted,
                "o" | "." | "scatter" => Kind::Scatter,
                "o-" | "-o" | ".-" | "-." => Kind::LineMarkers,
                "bar" => Kind::Bar,
                "step" => Kind::Step,
                "area" => Kind::Area,
                _ => Kind::Line,
            };
            add(name, Series { categories: cats, ..Series::new(kind, x, y, text(args, 2), color_arg(args, 3)) });
        }
        "bar" | "barh" | "scatter" | "step" | "area" | "fill_between" => {
            let (x, y, cats) = xy(args);
            let kind = match method {
                "bar" => Kind::Bar,
                "barh" => Kind::Barh,
                "scatter" => Kind::Scatter,
                "step" => Kind::Step,
                _ => Kind::Area,
            };
            add(name, Series { categories: cats, ..Series::new(kind, x, y, text(args, 2), color_arg(args, 3)) });
        }
        "addseries" | "add_series" | "series" => {
            // (the web's: AddSeries label, "y,y,…" [, "x,x,…" [, colour]])
            let y = num::parse_list(&text(args, 1));
            let x = match args.get(2).map(num::values_of) {
                Some(x) if !x.is_empty() => x,
                _ => (0..y.len()).map(|i| i as f64).collect(),
            };
            add(name, Series::new(Kind::Line, x, y, text(args, 0), color_arg(args, 3)));
        }
        "hist" | "histogram" => {
            let data: Vec<f64> = args.first().map(num::values_of).unwrap_or_default().into_iter().filter(|v| v.is_finite()).collect();
            if !data.is_empty() {
                let bins = args.get(1).map(Value::to_i64).unwrap_or(10).clamp(1, 1000) as usize;
                let lo = data.iter().cloned().fold(f64::INFINITY, f64::min);
                let hi = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let width = if hi > lo { (hi - lo) / bins as f64 } else { 1.0 };
                let mut counts = vec![0.0; bins];
                for v in &data {
                    counts[(((v - lo) / width) as usize).min(bins - 1)] += 1.0;
                }
                let centres = (0..bins).map(|i| lo + (i as f64 + 0.5) * width).collect();
                add(name, Series { bin: width, ..Series::new(Kind::Hist, centres, counts, text(args, 2), color_arg(args, 3)) });
            }
        }
        "pie" => {
            let data = args.first().map(num::values_of).unwrap_or_default();
            let labels: Vec<String> = text(args, 1).split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            let cols: Vec<u32> = text(args, 2).split(',').filter_map(colors::parse).collect();
            add(name, Series { slice_labels: labels, slice_colors: cols, ..Series::new(Kind::Pie, data.clone(), data, String::new(), None) });
        }
        "hline" | "axhline" => {
            let y = args.first().map(Value::to_f64).unwrap_or(0.0);
            add(name, Series::new(Kind::HLine, vec![], vec![y], text(args, 2), color_arg(args, 1).or(Some(0x808080))));
        }
        "vline" | "axvline" => {
            let x = args.first().map(Value::to_f64).unwrap_or(0.0);
            add(name, Series::new(Kind::VLine, vec![x], vec![], text(args, 2), color_arg(args, 1).or(Some(0x808080))));
        }
        "annotate" => {
            let note = Note { text: text(args, 0), x: args.get(1).map(Value::to_f64).unwrap_or(0.0), y: args.get(2).map(Value::to_f64).unwrap_or(0.0), color: color_arg(args, 3) };
            with(name, |p| p.notes.push(note));
        }
        "legend" => {
            let on = args.first().map(|v| v.to_i64() != 0).unwrap_or(true);
            with(name, |p| p.legend = on);
        }
        "grid" => {
            let on = args.first().map(|v| v.to_i64() != 0).unwrap_or(true);
            with(name, |p| p.grid = Some(on));
        }
        "figsize" => {
            let (w, h) = (args.first().map(Value::to_f64).unwrap_or(6.4), args.get(1).map(Value::to_f64).unwrap_or(4.8));
            with(name, |p| {
                p.width = (w * p.dpi.max(50) as f64).round().max(1.0) as i64;
                p.height = (h * p.dpi.max(50) as f64).round().max(1.0) as i64;
            });
        }
        "xlim" | "ylim" => {
            let lim = (args.first().map(Value::to_f64).unwrap_or(0.0), args.get(1).map(Value::to_f64).unwrap_or(1.0));
            let lim = (lim.0 != lim.1).then_some(lim);
            with(name, |p| if method == "xlim" { p.xlim = lim } else { p.ylim = lim });
        }
        "xscale" | "yscale" => {
            let log = text(args, 0).trim().eq_ignore_ascii_case("log");
            with(name, |p| if method == "xscale" { p.xlog = log } else { p.ylog = log });
        }
        "xticks" => {
            let names: Vec<String> = text(args, 0).split(',').map(|s| s.trim().to_string()).collect();
            let at = args.get(1).map(num::values_of).filter(|v| !v.is_empty());
            with(name, |p| {
                let at = at.unwrap_or_else(|| match p.series.iter().find(|s| !matches!(s.kind, Kind::HLine | Kind::VLine)) {
                    Some(s) if s.x.len() == names.len() => s.x.clone(),
                    _ => (0..names.len()).map(|i| i as f64).collect(),
                });
                p.xticks = (!names.is_empty() && !names[0].is_empty()).then_some((at, names));
            });
        }
        "settitle" | "set_title" | "title" => {
            let t = text(args, 0);
            with(name, |p| p.title = t);
        }
        "setxlabel" | "set_xlabel" | "xlabel" => {
            let t = text(args, 0);
            with(name, |p| p.xlabel = t);
        }
        "setylabel" | "set_ylabel" | "ylabel" => {
            let t = text(args, 0);
            with(name, |p| p.ylabel = t);
        }
        // (a property read as a method)
        other => return get_prop(name, other),
    }
    Some(v_null())
}

/// Reads RPLOT property `prop` (lower case); `None` when there's no such.
pub fn get_prop(name: &str, prop: &str) -> Option<Value> {
    let p = get(name);
    Some(match prop {
        "title" => v_str(&p.title),
        "xlabel" => v_str(&p.xlabel),
        "ylabel" => v_str(&p.ylabel),
        "grid" => v_int(i64::from(p.grid == Some(true))),
        "width" => v_int(p.size().0),
        "height" => v_int(p.size().1),
        "dpi" => v_int(p.dpi),
        "legend" => v_int(i64::from(p.legend)),
        "count" | "seriescount" => v_int(p.series.len() as i64),
        _ => return None,
    })
}

/// Sets RPLOT property `prop` (lower case); false when there's no such.
pub fn set_prop(name: &str, prop: &str, val: &Value) -> bool {
    let known = matches!(prop, "title" | "xlabel" | "ylabel" | "grid" | "width" | "height" | "dpi" | "legend");
    if known {
        with(name, |p| match prop {
            "title" => p.title = val.to_string_val(),
            "xlabel" => p.xlabel = val.to_string_val(),
            "ylabel" => p.ylabel = val.to_string_val(),
            "grid" => p.grid = Some(val.to_i64() != 0),
            "width" => p.width = val.to_i64().max(1),
            "height" => p.height = val.to_i64().max(1),
            "dpi" => p.dpi = val.to_i64().max(50),
            _ => p.legend = val.to_i64() != 0,
        });
    }
    known
}

// ------------------------------------------------------------- drawing --

/// Chart `name` as the kernel's ops in its own pixels (0, 0 at its top
/// left), in `theme`'s colours, and its size.
pub fn ops(name: &str, theme: &Theme) -> (Vec<Op>, (i64, i64)) {
    let p = get(name);
    let size = p.size();
    (draw(&p, size, theme), size)
}

/// The colours a chart draws with, from a theme.
struct Ink {
    bg: u32,
    fg: u32,
    /// Axis lines and ticks.
    axis: u32,
    grid: u32,
    /// Tick labels.
    muted: u32,
}

impl Ink {
    fn of(theme: &Theme) -> Ink {
        let (bg, fg) = (theme.window, theme.text);
        Ink { bg, fg, axis: mix(fg, bg, 0.55), grid: mix(fg, bg, if theme.dark { 0.82 } else { 0.9 }), muted: mix(fg, bg, 0.3) }
    }
}

fn font(px: i64, bold: bool) -> Font {
    Font { name: "Arial".into(), size: -px, color: 0, styles: u8::from(bold) }
}

/// The text sizes for a chart of this size.
struct Fonts {
    tick: Font,
    label: Font,
    title: Font,
}

impl Fonts {
    fn for_size(w: i64, h: i64) -> Fonts {
        let tick = (w.min(h) as f64 / 26.0).round().clamp(10.0, 14.0) as i64;
        Fonts { tick: font(tick, false), label: font(tick + 1, false), title: font(tick + 3, true) }
    }
}

/// A nice tick step for `span` in about `count` steps: 1, 2, 2.5 or 5 × 10ⁿ.
fn nice_step(span: f64, count: f64) -> f64 {
    let raw = (span / count.max(1.0)).abs();
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let mag = 10f64.powf(raw.log10().floor());
    let r = raw / mag;
    let m = if r <= 1.0 {
        1.0
    } else if r <= 2.0 {
        2.0
    } else if r <= 2.5 {
        2.5
    } else if r <= 5.0 {
        5.0
    } else {
        10.0
    };
    m * mag
}

/// About how many ticks an axis `len` pixels long gets (a vertical one's
/// labels are closer together).
fn tick_count(len: f64, vertical: bool) -> f64 {
    if vertical {
        (len / 26.0).clamp(4.0, 10.0)
    } else {
        (len / 50.0).clamp(4.0, 12.0)
    }
}

/// The ticks in [lo, hi] at `step`.
fn ticks(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let mut t = (lo / step - 1e-9).ceil() * step;
    while t <= hi + step * 1e-9 && out.len() < 200 {
        out.push(if t.abs() < step * 1e-9 { 0.0 } else { t });
        t += step;
    }
    out
}

/// Thousands grouped: 85000 → "85,000".
fn group(int: &str) -> String {
    let (sign, digits) = int.strip_prefix('-').map(|d| ("-", d)).unwrap_or(("", int));
    if digits.len() <= 4 {
        return int.to_string();
    }
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{sign}{out}")
}

/// A tick's label for value `v` on an axis stepping by `step`.
fn tick_label(v: f64, step: f64) -> String {
    if v.abs() >= 1e15 || (v != 0.0 && v.abs() < 1e-6) {
        return format!("{v:e}");
    }
    let mut decimals = 0usize;
    let mut s = step.abs();
    while decimals < 8 && (s - s.round()).abs() > 1e-9 * s.max(1.0) {
        s *= 10.0;
        decimals += 1;
    }
    let text = format!("{v:.decimals$}");
    let text = if text.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') { text.trim_start_matches('-').to_string() } else { text };
    let text = match text.split_once('.') {
        Some((int, frac)) => format!("{}.{frac}", group(int)),
        None => group(&text),
    };
    // (a typographic minus, as charts set it)
    match text.strip_prefix('-') {
        Some(rest) => format!("\u{2212}{rest}"),
        None => text,
    }
}

/// One axis: data → pixels.
#[derive(Clone, Copy)]
struct Scale {
    lo: f64,
    hi: f64,
    /// Pixels at lo and hi.
    a: f64,
    b: f64,
    log: bool,
}

impl Scale {
    fn px(&self, v: f64) -> f64 {
        let v = if self.log { if v > 0.0 { v.log10() } else { self.lo - 1.0 } } else { v };
        self.a + (v - self.lo) / (self.hi - self.lo) * (self.b - self.a)
    }
}

/// An axis' extent and ticks in its own units (log10 for a log axis).
struct Axis {
    lo: f64,
    hi: f64,
    /// (position, label)
    ticks: Vec<(f64, String)>,
    log: bool,
}

/// The extent of `values` (finite ones; log10 of positive ones on a log
/// axis), with `extra` values it must show.
fn extent(values: impl Iterator<Item = f64>, log: bool) -> Option<(f64, f64)> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for v in values {
        let v = if log {
            if v > 0.0 {
                v.log10()
            } else {
                continue;
            }
        } else {
            v
        };
        if v.is_finite() {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    (lo <= hi).then_some((lo, hi))
}

/// A value axis over `ext` (padded 5 % unless `lim` fixes it; bars keep
/// their baseline at 0), with about `count` nice ticks.
fn value_axis(ext: Option<(f64, f64)>, lim: Option<(f64, f64)>, log: bool, zero: bool, count: f64) -> Axis {
    let (lo, mut hi) = match lim {
        Some((a, b)) if log && a > 0.0 && b > 0.0 => (a.min(b).log10(), a.max(b).log10()),
        Some((a, b)) if !log => (a.min(b), a.max(b)),
        _ => {
            let (mut lo, mut hi) = ext.unwrap_or((0.0, 1.0));
            if zero && !log {
                lo = lo.min(0.0);
                hi = hi.max(0.0);
            }
            if hi - lo < 1e-12 {
                let d = if lo.abs() > 1e-12 { lo.abs() * 0.1 } else { 1.0 };
                lo -= d;
                hi += d;
            }
            let pad = (hi - lo) * 0.05;
            // (bars stand on 0: no padding below them)
            let lo2 = if zero && !log && lo >= 0.0 { lo } else { lo - pad };
            let hi2 = if zero && !log && hi <= 0.0 { hi } else { hi + pad };
            (lo2, hi2)
        }
    };
    if hi - lo < 1e-12 {
        hi = lo + 1.0;
    }
    if log {
        let (a, b) = (lo.floor() as i64, hi.ceil() as i64);
        let step = (((b - a) as f64) / count).ceil().max(1.0) as i64;
        let ticks = (a..=b).filter(|k| (k - a) % step == 0).map(|k| k as f64).filter(|k| *k >= lo - 1e-9 && *k <= hi + 1e-9).map(|k| (k, tick_label(10f64.powf(k), if k < 0.0 { 10f64.powf(k) } else { 1.0 }))).collect();
        return Axis { lo, hi, ticks, log };
    }
    let step = nice_step(hi - lo, count);
    let ticks = ticks(lo, hi, step).into_iter().map(|t| (t, tick_label(t, step))).collect();
    Axis { lo, hi, ticks, log }
}

/// The ops for a text in `font` placed with its top-left at (x, y).
fn text_at(ops: &mut Vec<Op>, s: &str, font: &Font, color: u32, x: f64, y: f64) {
    let (w, h) = text_size(s, font);
    ops.push(Op::Text { rect: (x.round() as i64, y.round() as i64, w + 2, h), text: s.to_string(), font: font.clone(), color, angle: 0, place: Place::TopLeft });
}

/// A polygon approximating a circle.
fn circle(cx: f64, cy: f64, r: f64) -> Vec<(f64, f64)> {
    (0..24).map(|i| {
        let a = i as f64 / 24.0 * std::f64::consts::TAU;
        (cx + r * a.cos(), cy + r * a.sin())
    }).collect()
}

/// `points` as dashes `on` long with `off` gaps.
fn dashes(points: &[(f64, f64)], on: f64, off: f64) -> Vec<Vec<(f64, f64)>> {
    let mut out = Vec::new();
    let mut cur: Vec<(f64, f64)> = Vec::new();
    let mut drawing = true;
    let mut left = on;
    for w in points.windows(2) {
        let (mut a, b) = (w[0], w[1]);
        let mut len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        while len > 1e-9 {
            let step = left.min(len);
            let t = step / len;
            let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            if drawing {
                if cur.is_empty() {
                    cur.push(a);
                }
                cur.push(p);
            }
            a = p;
            len -= step;
            left -= step;
            if left <= 1e-9 {
                if drawing && cur.len() > 1 {
                    out.push(std::mem::take(&mut cur));
                }
                cur.clear();
                drawing = !drawing;
                left = if drawing { on } else { off };
            }
        }
    }
    if drawing && cur.len() > 1 {
        out.push(cur);
    }
    out
}

/// What a legend entry shows beside its name.
#[derive(Clone, Copy)]
enum Swatch {
    Line(Kind),
    Box,
    Dot,
}

fn swatch_of(kind: Kind) -> Swatch {
    match kind {
        Kind::Bar | Kind::Barh | Kind::Hist | Kind::Area | Kind::Pie => Swatch::Box,
        Kind::Scatter => Swatch::Dot,
        k => Swatch::Line(k),
    }
}

/// A legend: its entries in a box at (x, y).
fn legend_size(entries: &[(String, u32, Swatch)], f: &Font) -> (f64, f64, f64) {
    let line_h = text_size("Ag", f).1 as f64 + 4.0;
    let w = entries.iter().map(|(t, _, _)| text_size(t, f).0).max().unwrap_or(0) as f64 + 22.0 + 16.0;
    (w, entries.len() as f64 * line_h + 10.0, line_h)
}

fn draw_legend(ops: &mut Vec<Op>, ink: &Ink, entries: &[(String, u32, Swatch)], f: &Font, x: f64, y: f64, framed: bool) {
    let (w, h, line_h) = legend_size(entries, f);
    if framed {
        ops.push(Op::Round { rect: (x.round() as i64, y.round() as i64, w.round() as i64, h.round() as i64), radius: 3.0, fill: Some(ink.bg), stroke: Some(ink.grid), width: 1.0 });
    }
    for (i, (t, c, sw)) in entries.iter().enumerate() {
        let cy = y + 5.0 + line_h * (i as f64 + 0.5);
        let sx = x + 8.0;
        match sw {
            Swatch::Box => ops.push(Op::Round { rect: (sx.round() as i64, (cy - 5.0).round() as i64, 14, 10), radius: 2.0, fill: Some(*c), stroke: None, width: 0.0 }),
            Swatch::Dot => ops.push(Op::Polygon { points: circle(sx + 7.0, cy, 3.5), color: *c }),
            Swatch::Line(k) => {
                let pts = vec![(sx, cy), (sx + 16.0, cy)];
                match k {
                    Kind::Dashed => {
                        for d in dashes(&pts, 5.0, 3.0) {
                            ops.push(Op::Stroke { points: d, color: *c, width: 2.0 });
                        }
                    }
                    Kind::Dotted => {
                        for d in dashes(&pts, 1.5, 2.5) {
                            ops.push(Op::Stroke { points: d, color: *c, width: 2.0 });
                        }
                    }
                    _ => ops.push(Op::Stroke { points: pts, color: *c, width: 2.0 }),
                }
                if matches!(k, Kind::LineMarkers) {
                    ops.push(Op::Polygon { points: circle(sx + 8.0, cy, 3.0), color: *c });
                }
            }
        }
        let th = text_size(t, f).1 as f64;
        text_at(ops, t, f, ink.fg, sx + 22.0, cy - th / 2.0);
    }
}

fn draw(p: &Plot, (w, h): (i64, i64), theme: &Theme) -> Vec<Op> {
    let ink = Ink::of(theme);
    let fonts = Fonts::for_size(w, h);
    let mut ops = vec![Op::Fill { rect: (0, 0, w, h), color: ink.bg }];
    let pad = 10.0;
    let mut top = pad;
    if !p.title.is_empty() {
        let (tw, th) = text_size(&p.title, &fonts.title);
        text_at(&mut ops, &p.title, &fonts.title, ink.fg, (w as f64 - tw as f64) / 2.0, top);
        top += th as f64 + 8.0;
    }
    if p.series.iter().any(|s| s.kind == Kind::Pie) {
        draw_pie(&mut ops, p, &ink, &fonts, (pad, top, w as f64 - pad, h as f64 - pad));
        return ops;
    }
    draw_cartesian(&mut ops, p, &ink, &fonts, (pad, top, w as f64 - pad, h as f64 - pad));
    ops
}

fn draw_pie(ops: &mut Vec<Op>, p: &Plot, ink: &Ink, fonts: &Fonts, (x0, y0, x1, y1): (f64, f64, f64, f64)) {
    let Some(s) = p.series.iter().find(|s| s.kind == Kind::Pie) else { return };
    let values: Vec<f64> = s.y.iter().map(|v| if v.is_finite() && *v > 0.0 { *v } else { 0.0 }).collect();
    let total: f64 = values.iter().sum();
    if total <= 0.0 {
        return;
    }
    let names: Vec<String> = (0..values.len()).map(|i| s.slice_labels.get(i).cloned().unwrap_or_else(|| format!("Slice {}", i + 1))).collect();
    let cols: Vec<u32> = (0..values.len()).map(|i| s.slice_colors.get(i).copied().unwrap_or_else(|| colors::auto(i))).collect();
    // The names beside the pie (a legend without a frame).
    let entries: Vec<(String, u32, Swatch)> = names.iter().zip(&cols).map(|(n, c)| (n.clone(), *c, Swatch::Box)).collect();
    let (lw, lh, _) = legend_size(&entries, &fonts.label);
    let room = x1 - x0;
    let legend_fits = lw < room * 0.45;
    let pie_w = if legend_fits { room - lw - 8.0 } else { room };
    let r = ((pie_w).min(y1 - y0) / 2.0 - 4.0).max(10.0);
    let (cx, cy) = (x0 + pie_w / 2.0, (y0 + y1) / 2.0);
    let mut a = -std::f64::consts::FRAC_PI_2;
    let mut bounds = Vec::new();
    for (i, v) in values.iter().enumerate() {
        let sweep = v / total * std::f64::consts::TAU;
        if sweep <= 0.0 {
            continue;
        }
        let steps = ((sweep.to_degrees() / 2.0).ceil() as usize).max(2);
        let mut pts = vec![(cx, cy)];
        for k in 0..=steps {
            let t = a + sweep * k as f64 / steps as f64;
            pts.push((cx + r * t.cos(), cy + r * t.sin()));
        }
        ops.push(Op::Polygon { points: pts, color: cols[i] });
        bounds.push(a);
        let frac = v / total;
        if frac >= 0.05 {
            let mid = a + sweep / 2.0;
            let label = format!("{}%", tick_label(frac * 100.0, if frac * 100.0 >= 10.0 { 1.0 } else { 0.1 }));
            let f = font(fonts.label.pixel_size(), true);
            let (tw, th) = text_size(&label, &f);
            let col = if colors::luminance(cols[i]) > 0.45 { 0x1A1A1A } else { 0xFFFFFF };
            text_at(ops, &label, &f, col, cx + r * 0.62 * mid.cos() - tw as f64 / 2.0, cy + r * 0.62 * mid.sin() - th as f64 / 2.0);
        }
        a += sweep;
    }
    // Thin gaps between the slices, in the background's colour.
    if bounds.len() > 1 {
        for b in bounds {
            ops.push(Op::Stroke { points: vec![(cx, cy), (cx + (r + 1.0) * b.cos(), cy + (r + 1.0) * b.sin())], color: ink.bg, width: 2.0 });
        }
    }
    if legend_fits {
        draw_legend(ops, ink, &entries, &fonts.label, x1 - lw, cy - lh / 2.0, false);
    }
}

fn draw_cartesian(ops: &mut Vec<Op>, p: &Plot, ink: &Ink, fonts: &Fonts, (x0, y0, x1, y1): (f64, f64, f64, f64)) {
    let plotted: Vec<&Series> = p.series.iter().filter(|s| !matches!(s.kind, Kind::HLine | Kind::VLine)).collect();
    let horizontal = plotted.iter().any(|s| s.kind == Kind::Barh);
    // Category names: the first series' that has some (bars over names).
    let cats: Vec<String> = plotted.iter().find(|s| !s.categories.is_empty()).map(|s| s.categories.clone()).unwrap_or_default();
    // "position" is the axis bars stand along (x, or y for Barh); "value"
    // the other.
    let bar_series: Vec<&&Series> = plotted.iter().filter(|s| matches!(s.kind, Kind::Bar | Kind::Barh)).collect();
    // The bars' slot: the closest two positions (1 with categories).
    let slot = if !cats.is_empty() {
        1.0
    } else {
        let mut xs: Vec<f64> = bar_series.iter().flat_map(|s| s.x.iter().copied()).filter(|v| v.is_finite()).collect();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        xs.dedup();
        xs.windows(2).map(|w| w[1] - w[0]).fold(f64::INFINITY, f64::min).min(f64::MAX).max(1e-12)
    };
    let slot = if slot.is_finite() { slot } else { 1.0 };
    let half = |s: &Series| match s.kind {
        Kind::Bar | Kind::Barh => slot * 0.4,
        Kind::Hist => s.bin / 2.0,
        _ => 0.0,
    };
    let any_bars = plotted.iter().any(|s| s.is_bar() || s.kind == Kind::Area);
    // Extents: positions (x), values (y) — swapped on screen for Barh.
    // (the position axis' limits and scale, the value axis': x's and y's,
    // or y's and x's for Barh)
    let (xlim, ylim, xlog, ylog) = if horizontal { (p.ylim, p.xlim, p.ylog, p.xlog) } else { (p.xlim, p.ylim, p.xlog, p.ylog) };
    let xlog = xlog && cats.is_empty();
    let pos_ext = extent(
        plotted.iter().flat_map(|s| s.x.iter().flat_map(move |&x| [x - half(s), x + half(s)])).chain(p.series.iter().filter(|s| s.kind == Kind::VLine).flat_map(|s| s.x.iter().copied())),
        xlog,
    );
    let val_ext = extent(plotted.iter().flat_map(|s| s.y.iter().copied()).chain(p.series.iter().filter(|s| s.kind == Kind::HLine).flat_map(|s| s.y.iter().copied())), ylog);

    // Measure first with a guess of the tick counts, then lay out.
    let tick_h = text_size("0", &fonts.tick).1 as f64;
    let label_h = text_size("Ag", &fonts.label).1 as f64;
    let mut bottom = y1 - 7.0 - tick_h;
    if !p.xlabel.is_empty() {
        bottom -= label_h + 4.0;
    }
    let ylabel_room = if p.ylabel.is_empty() { 0.0 } else { label_h + 6.0 };
    let est_h = (bottom - y0).max(10.0);
    // The vertical axis' (values, or positions for Barh)
    let make_pos_axis = |len: f64| -> Axis {
        if !cats.is_empty() {
            let n = cats.len() as f64;
            let (lo, hi) = xlim.map(|(a, b)| (a.min(b), a.max(b))).unwrap_or((-0.5, n - 0.5));
            Axis { lo, hi, ticks: cats.iter().enumerate().map(|(i, c)| (i as f64, c.clone())).collect(), log: false }
        } else {
            let lim = xlim;
            let mut ax = value_axis(pos_ext, lim, xlog, false, tick_count(len, horizontal));
            if lim.is_none() && any_bars && !xlog {
                // (bars and histograms: no padding beyond their edges)
                if let Some((lo, hi)) = pos_ext {
                    let pad = (hi - lo) * 0.02;
                    ax.lo = lo - pad;
                    ax.hi = hi + pad;
                    let step = nice_step(ax.hi - ax.lo, tick_count(len, horizontal));
                    ax.ticks = ticks(ax.lo, ax.hi, step).into_iter().map(|t| (t, tick_label(t, step))).collect();
                }
            }
            ax
        }
    };
    let make_val_axis = |len: f64| value_axis(val_ext, ylim, ylog, any_bars, tick_count(len, !horizontal));
    let (mut xaxis, yaxis) = if horizontal { (make_val_axis(x1 - x0), make_pos_axis(est_h)) } else { (make_pos_axis(x1 - x0), make_val_axis(est_h)) };
    if let (Some((at, names)), false) = (&p.xticks, horizontal) {
        xaxis.ticks = at.iter().zip(names).map(|(a, n)| (*a, n.clone())).collect();
    }
    let ytick_w = yaxis.ticks.iter().map(|(_, t)| text_size(t, &fonts.tick).0).max().unwrap_or(0) as f64;
    let left = x0 + ylabel_room + ytick_w + 7.0;
    let last_half = xaxis.ticks.last().map(|(_, t)| text_size(t, &fonts.tick).0 as f64 / 2.0).unwrap_or(0.0);
    let right = x1 - (last_half - 4.0).max(0.0).min(40.0);
    let top = y0 + 4.0;
    if right - left < 20.0 || bottom - top < 20.0 {
        return;
    }
    let sx = Scale { lo: xaxis.lo, hi: xaxis.hi, a: left, b: right, log: xaxis.log };
    let sy = Scale { lo: yaxis.lo, hi: yaxis.hi, a: bottom, b: top, log: yaxis.log };
    let plot_rect: Rect = (left.floor() as i64, top.floor() as i64, (right - left).ceil() as i64 + 1, (bottom - top).ceil() as i64 + 1);

    // Gridlines: along the value axis by default; both ways with Grid = 1.
    let (grid_x, grid_y) = match p.grid {
        Some(true) => (true, true),
        Some(false) => (false, false),
        None => (horizontal, !horizontal),
    };
    let in_range = |a: &Axis, t: f64| t >= a.lo - 1e-9 && t <= a.hi + 1e-9;
    if grid_y {
        for (t, _) in yaxis.ticks.iter().filter(|(t, _)| in_range(&yaxis, *t)) {
            if cats.is_empty() || !horizontal {
                let y = sy.px_raw(*t).round();
                ops.push(Op::Fill { rect: (left.round() as i64, y as i64, (right - left).round() as i64, 1), color: ink.grid });
            }
        }
    }
    if grid_x {
        for (t, _) in xaxis.ticks.iter().filter(|(t, _)| in_range(&xaxis, *t)) {
            if cats.is_empty() || horizontal {
                let x = sx.px_raw(*t).round();
                ops.push(Op::Fill { rect: (x as i64, top.round() as i64, 1, (bottom - top).round() as i64), color: ink.grid });
            }
        }
    }

    ops.push(Op::ClipPush { rect: plot_rect });
    // Screen point of a data point (x along positions, y values; swapped for Barh).
    let pt = |x: f64, y: f64| -> (f64, f64) { if horizontal { (sx.px(y), sy.px(x)) } else { (sx.px(x), sy.px(y)) } };
    let nbars = bar_series.len().max(1) as f64;
    let mut bar_index = 0.0;
    for s in &plotted {
        let c = s.color.unwrap_or(0x4E79A7);
        let pts: Vec<(f64, f64)> = s.x.iter().zip(&s.y).filter(|(x, y)| x.is_finite() && y.is_finite()).map(|(&x, &y)| pt(x, y)).collect();
        match s.kind {
            Kind::Bar | Kind::Barh => {
                // (several bar series side by side in each slot)
                let group = slot * 0.8;
                let bw = group / nbars;
                let off = -group / 2.0 + bw * (bar_index + 0.5);
                bar_index += 1.0;
                for (&x, &y) in s.x.iter().zip(&s.y) {
                    if !(x.is_finite() && y.is_finite()) {
                        continue;
                    }
                    let (a, b) = (x + off - bw / 2.0, x + off + bw / 2.0);
                    let (p0, p1) = (pt(a, 0.0), pt(b, y));
                    let (l, r) = (p0.0.min(p1.0).round(), p0.0.max(p1.0).round());
                    let (t, btm) = (p0.1.min(p1.1).round(), p0.1.max(p1.1).round());
                    ops.push(Op::Fill { rect: (l as i64, t as i64, ((r - l) as i64).max(1), ((btm - t) as i64).max(1)), color: c });
                }
            }
            Kind::Hist => {
                for (&x, &y) in s.x.iter().zip(&s.y) {
                    let (p0, p1) = (pt(x - s.bin / 2.0, 0.0), pt(x + s.bin / 2.0, y));
                    let (l, r) = (p0.0.min(p1.0).round(), p0.0.max(p1.0).round());
                    let (t, btm) = (p0.1.min(p1.1).round(), p0.1.max(p1.1).round());
                    // (a pixel's gap between the bins, in the background's colour)
                    ops.push(Op::Fill { rect: (l as i64, t as i64, ((r - l) as i64 - 1).max(1), ((btm - t) as i64).max(1)), color: c });
                }
            }
            Kind::Area => {
                if pts.len() >= 2 {
                    let base = sy.px(0.0f64.max(sy.lo_value()).min(sy.hi_value()));
                    let mut poly = vec![(pts[0].0, base)];
                    poly.extend(pts.iter().copied());
                    poly.push((pts[pts.len() - 1].0, base));
                    ops.push(Op::Polygon { points: poly, color: mix(c, ink.bg, 0.7) });
                    ops.push(Op::Stroke { points: pts.clone(), color: c, width: 2.0 });
                }
            }
            Kind::Step => {
                let mut steps = Vec::new();
                for (i, &q) in pts.iter().enumerate() {
                    if i > 0 {
                        steps.push((q.0, pts[i - 1].1));
                    }
                    steps.push(q);
                }
                if steps.len() >= 2 {
                    ops.push(Op::Stroke { points: steps, color: c, width: 2.0 });
                }
            }
            Kind::Scatter => {
                for &(x, y) in &pts {
                    ops.push(Op::Polygon { points: circle(x, y, 3.5), color: c });
                }
            }
            Kind::Dashed | Kind::Dotted => {
                let (on, off) = if s.kind == Kind::Dashed { (6.0, 4.0) } else { (1.5, 3.0) };
                for d in dashes(&pts, on, off) {
                    ops.push(Op::Stroke { points: d, color: c, width: 2.0 });
                }
            }
            Kind::Line | Kind::LineMarkers => {
                if pts.len() >= 2 {
                    ops.push(Op::Stroke { points: pts.clone(), color: c, width: 2.0 });
                } else if let Some(&(x, y)) = pts.first() {
                    ops.push(Op::Polygon { points: circle(x, y, 3.0), color: c });
                }
                if s.kind == Kind::LineMarkers {
                    for &(x, y) in &pts {
                        ops.push(Op::Polygon { points: circle(x, y, 3.5), color: c });
                    }
                }
            }
            Kind::Pie | Kind::HLine | Kind::VLine => {}
        }
    }
    // Reference lines across the plot.
    for s in p.series.iter().filter(|s| matches!(s.kind, Kind::HLine | Kind::VLine)) {
        let c = s.color.unwrap_or(ink.axis);
        if s.kind == Kind::HLine {
            for &y in &s.y {
                let py = sy.px(y);
                ops.push(Op::Stroke { points: vec![(left, py), (right, py)], color: c, width: 1.5 });
            }
        } else {
            for &x in &s.x {
                let px = sx.px(x);
                ops.push(Op::Stroke { points: vec![(px, top), (px, bottom)], color: c, width: 1.5 });
            }
        }
    }
    ops.push(Op::ClipPop);
    for n in &p.notes {
        let (x, y) = (sx.px(n.x), sy.px(n.y));
        let th = text_size(&n.text, &fonts.tick).1 as f64;
        text_at(ops, &n.text, &fonts.tick, n.color.unwrap_or(ink.fg), x + 4.0, y - th - 2.0);
    }

    // The axes: the left and bottom lines, ticks outward, their labels.
    let (l, b) = (left.round() as i64, bottom.round() as i64);
    ops.push(Op::Fill { rect: (l, top.round() as i64, 1, b - top.round() as i64 + 1), color: ink.axis });
    ops.push(Op::Fill { rect: (l, b, (right - left).round() as i64 + 1, 1), color: ink.axis });
    let tw = |t: &str| text_size(t, &fonts.tick).0 as f64;
    // x tick labels: skip some when they would overlap.
    let xt: Vec<&(f64, String)> = xaxis.ticks.iter().filter(|(t, _)| in_range(&xaxis, *t)).collect();
    let widest = xt.iter().map(|(_, s)| tw(s)).fold(0.0, f64::max) + 6.0;
    let spacing = if xt.len() > 1 { (sx.px_raw(xt[1].0) - sx.px_raw(xt[0].0)).abs() } else { f64::INFINITY };
    let every = if spacing.is_finite() && spacing > 0.0 { (widest / spacing).ceil().max(1.0) as usize } else { 1 };
    for (i, (t, s)) in xt.iter().enumerate() {
        let x = sx.px_raw(*t).round();
        ops.push(Op::Fill { rect: (x as i64, b + 1, 1, 4), color: ink.axis });
        if i % every == 0 {
            text_at(ops, s, &fonts.tick, ink.muted, x - tw(s) / 2.0, bottom + 7.0);
        }
    }
    for (t, s) in yaxis.ticks.iter().filter(|(t, _)| in_range(&yaxis, *t)) {
        let y = sy.px_raw(*t).round();
        ops.push(Op::Fill { rect: (l - 4, y as i64, 4, 1), color: ink.axis });
        text_at(ops, s, &fonts.tick, ink.muted, left - 7.0 - tw(s), y - tick_h / 2.0);
    }
    if !p.xlabel.is_empty() {
        let (w, _) = text_size(&p.xlabel, &fonts.label);
        text_at(ops, &p.xlabel, &fonts.label, ink.fg, (left + right) / 2.0 - w as f64 / 2.0, bottom + 7.0 + tick_h + 4.0);
    }
    if !p.ylabel.is_empty() {
        let (w, hh) = text_size(&p.ylabel, &fonts.label);
        let cy = (top + bottom) / 2.0;
        ops.push(Op::Text { rect: ((x0).round() as i64, (cy - w as f64 / 2.0).round() as i64, hh, w), text: p.ylabel.clone(), font: fonts.label.clone(), color: ink.fg, angle: 90, place: Place::Center });
    }

    // The legend, in the corner the fewest points fall in.
    if p.legend {
        let entries: Vec<(String, u32, Swatch)> = p.series.iter().filter(|s| !s.label.is_empty()).map(|s| (s.label.clone(), s.color.unwrap_or(ink.axis), swatch_of(s.kind))).collect();
        if !entries.is_empty() {
            let (lw, lh, _) = legend_size(&entries, &fonts.tick);
            let m = 6.0;
            let corners = [(right - lw - m, top + m), (left + m, top + m), (right - lw - m, bottom - lh - m), (left + m, bottom - lh - m)];
            let points: Vec<(f64, f64)> = plotted.iter().flat_map(|s| s.x.iter().zip(&s.y).map(|(&x, &y)| pt(x, y)).collect::<Vec<_>>()).collect();
            let covered = |(cx, cy): (f64, f64)| -> usize {
                let mut n = points.iter().filter(|(x, y)| *x >= cx && *x <= cx + lw && *y >= cy && *y <= cy + lh).count();
                // (bars: their whole height counts)
                for s in plotted.iter().filter(|s| s.is_bar()) {
                    for (&x, &y) in s.x.iter().zip(&s.y) {
                        let (a, b0) = (pt(x, 0.0), pt(x, y));
                        let (xmin, xmax) = (a.0.min(b0.0), a.0.max(b0.0));
                        let (ymin, ymax) = (a.1.min(b0.1), a.1.max(b0.1));
                        if xmax >= cx && xmin <= cx + lw && ymax >= cy && ymin <= cy + lh {
                            n += 3;
                        }
                    }
                }
                n
            };
            let best = corners.iter().copied().min_by_key(|c| covered(*c)).unwrap();
            draw_legend(ops, ink, &entries, &fonts.tick, best.0, best.1, true);
        }
    }
}

impl Scale {
    /// A tick's position (already in axis units: log10 on a log axis).
    fn px_raw(&self, t: f64) -> f64 {
        self.a + (t - self.lo) / (self.hi - self.lo) * (self.b - self.a)
    }
    fn lo_value(&self) -> f64 {
        if self.log { 10f64.powf(self.lo) } else { self.lo }
    }
    fn hi_value(&self) -> f64 {
        if self.log { 10f64.powf(self.hi) } else { self.hi }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v_dbl;

    #[test]
    fn nice_ticks() {
        assert_eq!(nice_step(85.0, 5.0), 20.0);
        assert_eq!(nice_step(1.0, 4.0), 0.25);
        assert_eq!(ticks(0.0, 85.0, 20.0), vec![0.0, 20.0, 40.0, 60.0, 80.0]);
        assert_eq!(tick_label(0.25, 0.25), "0.25");
        assert_eq!(tick_label(85000.0, 5000.0), "85,000");
        assert_eq!(tick_label(-0.0, 0.5), "0.0");
        assert_eq!(tick_label(1.0, 0.5), "1.0");
    }

    #[test]
    fn categories_and_series() {
        num::set("pv", vec![85.0, 70.0, 61.0]);
        method("pc", "bar", &[v_str("Engineering,Marketing,Sales"), v_str("pv"), v_str("avg")]).unwrap();
        let p = get("pc");
        assert_eq!(p.series[0].categories, vec!["Engineering", "Marketing", "Sales"]);
        assert_eq!(p.series[0].x, vec![0.0, 1.0, 2.0]);
        assert_eq!(p.series[0].color, Some(colors::PALETTE[0]));
        let (ops, size) = ops("pc", &crate::theme::CLASSIC);
        assert_eq!(size, (640, 480));
        let texts: Vec<&str> = ops.iter().filter_map(|o| if let Op::Text { text, .. } = o { Some(text.as_str()) } else { None }).collect();
        assert!(texts.contains(&"Marketing"), "{texts:?}");
        assert!(texts.contains(&"80"), "{texts:?}");
        assert!(method("pc", "nonsense", &[]).is_none());
        set_prop("pc", "width", &v_dbl(6.0));
        assert_eq!(get_prop("pc", "width").unwrap().to_i64(), 600);
    }
}
