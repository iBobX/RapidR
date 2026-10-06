//! RPLOT: a chart in the manner of Matplotlib — its model (the series,
//! titles, limits, size) and every member. Drawing it is the runtime's
//! ([`super::Host::save_plot`], [`super::Host::show_plot`]), from
//! [`state`]: plotters on the desktop, an HTML canvas on the web.
//!
//! A series' data are RNUM arrays named by their components' names
//! (`plt.plot "x", "y"`), or numbers written in place (`"1,2,3"`).

use std::cell::RefCell;
use std::collections::HashMap;

use super::{arg_f, arg_i, arg_s, arg_s_or, key, num, Host};
use crate::{v_int, v_null, v_str, Value};

/// One series: its points, label, colour (a name or `#RRGGBB`) and kind —
/// `-` line, `--` dashed, `o` scatter, `bar`, `barh` (x and y swapped),
/// `step`, `area`, `hline` (y), `vline` (x), `pie` (x the values, `label`
/// and `color` comma-separated lists).
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub label: String,
    pub color: String,
    pub style: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub color: String,
}

/// A chart.
#[derive(Clone, Debug, PartialEq)]
pub struct Plot {
    pub title: String,
    pub xlabel: String,
    pub ylabel: String,
    pub grid: bool,
    /// Pixels, or inches (times `dpi`) when under 100 (`plt.width = 7`).
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub series: Vec<Series>,
    pub annotations: Vec<Annotation>,
    pub legend: bool,
    pub xlim: Option<(f64, f64)>,
    pub ylim: Option<(f64, f64)>,
    /// "linear" or "log".
    pub xscale: String,
    pub yscale: String,
}

impl Default for Plot {
    fn default() -> Self {
        Plot {
            title: String::new(),
            xlabel: String::new(),
            ylabel: String::new(),
            grid: false,
            width: 640,
            height: 480,
            dpi: 100,
            series: Vec::new(),
            annotations: Vec::new(),
            legend: false,
            xlim: None,
            ylim: None,
            xscale: "linear".into(),
            yscale: "linear".into(),
        }
    }
}

impl Plot {
    /// The chart's size in pixels.
    pub fn pixel_size(&self) -> (u32, u32) {
        let dpi = self.dpi.max(50);
        let px = |v: u32| if v < 100 { (v.max(1) * dpi).min(8000) } else { v.min(8000) };
        (px(self.width), px(self.height))
    }

    /// Whether the chart is a pie (drawn without axes).
    pub fn is_pie(&self) -> bool {
        self.series.iter().any(|s| s.style == "pie")
    }

    /// The axes' ranges, ((x from, to), (y from, to)): the data — whole bars,
    /// reference lines — 5% around it, from 0 where bars or areas stand on
    /// it; `xlim` / `ylim` as set. What both renderers draw.
    pub fn ranges(&self) -> ((f64, f64), (f64, f64)) {
        let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        let (mut zero_x, mut zero_y) = (false, false);
        let mut take = |x: Option<f64>, y: Option<f64>| {
            if let Some(x) = x.filter(|v| v.is_finite()) {
                x0 = x0.min(x);
                x1 = x1.max(x);
            }
            if let Some(y) = y.filter(|v| v.is_finite()) {
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        };
        for s in &self.series {
            match s.style.as_str() {
                "hline" => s.y.iter().for_each(|y| take(None, Some(*y))),
                "vline" => s.x.iter().for_each(|x| take(Some(*x), None)),
                "pie" => {}
                "bar" => {
                    let half = bar_width(&s.x) / 2.0;
                    zero_y = true;
                    for (x, y) in s.x.iter().zip(&s.y) {
                        take(Some(x - half), Some(*y));
                        take(Some(x + half), Some(0.0));
                    }
                }
                "barh" => {
                    let half = bar_width(&s.y) / 2.0;
                    zero_x = true;
                    for (x, y) in s.x.iter().zip(&s.y) {
                        take(Some(*x), Some(y - half));
                        take(Some(0.0), Some(y + half));
                    }
                }
                style => {
                    zero_y |= style == "area";
                    s.x.iter().zip(&s.y).for_each(|(x, y)| take(Some(*x), Some(*y)));
                    if style == "area" {
                        take(None, Some(0.0));
                    }
                }
            }
        }
        let axis = |lo: f64, hi: f64, zero: bool, lim: Option<(f64, f64)>| -> (f64, f64) {
            if let Some((a, b)) = lim.filter(|(a, b)| b > a) {
                return (a, b);
            }
            let (mut lo, mut hi) = if lo.is_finite() && hi.is_finite() { (lo, hi) } else { (0.0, 1.0) };
            if hi <= lo {
                let d = if lo == 0.0 { 1.0 } else { lo.abs() * 0.1 };
                (lo, hi) = (lo - d, hi + d);
            }
            let m = (hi - lo) * 0.05;
            // (bars stand on the axis: no margin below 0 when all are above it)
            let lo2 = if zero && lo >= 0.0 { lo.min(0.0) } else { lo - m };
            let hi2 = if zero && hi <= 0.0 { hi.max(0.0) } else { hi + m };
            (lo2, hi2)
        };
        (axis(x0, x1, zero_x, self.xlim), axis(y0, y1, zero_y, self.ylim))
    }
}

/// A bar's width for bars at `at`: 0.8 of the closest two's distance (0.8
/// for one bar).
pub fn bar_width(at: &[f64]) -> f64 {
    let mut v: Vec<f64> = at.iter().copied().filter(|x| x.is_finite()).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let gap = v.windows(2).map(|w| w[1] - w[0]).filter(|d| *d > 0.0).fold(f64::INFINITY, f64::min);
    if gap.is_finite() { gap * 0.8 } else { 0.8 }
}

/// The colours series without one take, in turn.
pub const PALETTE: &[&str] = &["blue", "red", "green", "orange", "purple", "cyan", "magenta", "steelblue", "brown", "pink", "teal", "gold", "navy", "coral"];

/// A colour name or `#RRGGBB` as red, green, blue (black if unknown).
pub fn rgb(color: &str) -> (u8, u8, u8) {
    let c = color.trim().to_ascii_lowercase();
    match c.as_str() {
        "red" => (255, 0, 0),
        "green" => (0, 128, 0),
        "blue" => (0, 0, 255),
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "orange" => (255, 165, 0),
        "purple" => (128, 0, 128),
        "cyan" => (0, 255, 255),
        "magenta" => (255, 0, 255),
        "yellow" => (255, 255, 0),
        "steelblue" => (70, 130, 180),
        "gray" | "grey" => (128, 128, 128),
        "lightblue" => (173, 216, 230),
        "lightgreen" => (144, 238, 144),
        "darkred" => (139, 0, 0),
        "darkblue" => (0, 0, 139),
        "darkgreen" => (0, 100, 0),
        "brown" => (139, 69, 19),
        "pink" => (255, 192, 203),
        "gold" => (255, 215, 0),
        "navy" => (0, 0, 128),
        "teal" => (0, 128, 128),
        "coral" => (255, 127, 80),
        "salmon" => (250, 128, 114),
        "olive" => (128, 128, 0),
        "maroon" => (128, 0, 0),
        "lime" => (0, 255, 0),
        "indigo" => (75, 0, 130),
        "violet" => (238, 130, 238),
        "silver" => (192, 192, 192),
        "tomato" => (255, 99, 71),
        s if s.len() == 7 && s.starts_with('#') => {
            let h = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0);
            (h(1), h(3), h(5))
        }
        _ => (0, 0, 0),
    }
}

/// A colour as CSS `#rrggbb`.
pub fn css(color: &str) -> String {
    let (r, g, b) = rgb(color);
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Round tick values (steps of 1, 2 or 5 times a power of ten) from `lo`
/// to `hi`, about `n` of them: where both renderers put an axis' ticks.
pub fn nice_ticks(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    if !(lo.is_finite() && hi.is_finite()) || hi <= lo {
        return Vec::new();
    }
    let raw = (hi - lo) / n.max(1) as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0].iter().map(|m| m * mag).find(|s| *s >= raw).unwrap_or(10.0 * mag);
    let first = (lo / step).ceil() as i64;
    let last = (hi / step).floor() as i64;
    (first..=last).take(100).map(|i| i as f64 * step).collect()
}

/// Where a chart's legend goes, as (right, top): the corner of the plot
/// area (x from `x0` to `x1`, y from `y0` to `y1`) with the fewest data
/// points under a legend covering `(fw, fh)` of it (fractions, its margin
/// included) — upper right first on a tie, then upper left, lower right,
/// lower left.
pub fn legend_corner(p: &Plot, xr: (f64, f64), yr: (f64, f64), size: (f64, f64)) -> (bool, bool) {
    corner_and_count(p, xr, yr, size).0
}

/// The axes' ranges and the legend's corner for a legend covering `size`
/// of the plot area (fractions; `None` without one): where no corner is
/// free of data, the y axis reaches higher, so the legend sits above the
/// data (as Matplotlib's headroom) — unless `ylim` is set.
pub fn layout(p: &Plot, size: Option<(f64, f64)>) -> (((f64, f64), (f64, f64)), (bool, bool)) {
    let (xr, yr) = p.ranges();
    let Some(size) = size else { return ((xr, yr), (true, true)) };
    let (corner, under) = corner_and_count(p, xr, yr, size);
    if under == 0 || p.ylim.is_some() || size.1 >= 0.6 {
        return ((xr, yr), corner);
    }
    let yr = (yr.0, yr.0 + (yr.1 - yr.0) / (1.0 - size.1));
    ((xr, yr), corner_and_count(p, xr, yr, size).0)
}

fn corner_and_count(p: &Plot, (x0, x1): (f64, f64), (y0, y1): (f64, f64), (fw, fh): (f64, f64)) -> ((bool, bool), usize) {
    let corners = [(true, true), (false, true), (true, false), (false, false)];
    let (wx, wy) = (x1 - x0, y1 - y0);
    if wx <= 0.0 || wy <= 0.0 {
        return ((true, true), 0);
    }
    let points: Vec<(f64, f64)> = p
        .series
        .iter()
        .filter(|s| !matches!(s.style.as_str(), "hline" | "vline" | "pie"))
        .flat_map(|s| {
            // (a bar covers its whole height, the base to the top)
            let steps: &[f64] = if matches!(s.style.as_str(), "bar" | "barh" | "area") { &[0.0, 0.25, 0.5, 0.75, 1.0] } else { &[1.0] };
            let barh = s.style == "barh";
            s.x.iter().zip(&s.y).flat_map(move |(x, y)| steps.iter().map(move |k| if barh { (x * k, *y) } else { (*x, y * k) }))
        })
        .map(|(x, y)| ((x - x0) / wx, (y - y0) / wy))
        .collect();
    let under = |(right, top): (bool, bool)| {
        points
            .iter()
            .filter(|(x, y)| (if right { *x >= 1.0 - fw } else { *x <= fw }) && (if top { *y >= 1.0 - fh } else { *y <= fh }))
            .count()
    };
    corners.into_iter().map(|c| (c, under(c))).min_by_key(|(_, n)| *n).unwrap_or(((true, true), 0))
}

/// A tick's label: whole numbers without a point, others with what their
/// step needs (`0.5`, `2.25`), very large or small ones in E notation.
pub fn tick_text(v: f64) -> String {
    if v.abs() < 1e-9 {
        return "0".into();
    }
    if v.abs() >= 1e6 || v.abs() < 1e-3 {
        return format!("{v:.1e}");
    }
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

thread_local! {
    static PLOTS: RefCell<HashMap<String, Plot>> = RefCell::new(HashMap::new());
}

/// Chart `name`'s model (a new chart's if it has none yet).
pub fn state(name: &str) -> Plot {
    PLOTS.with(|m| m.borrow().get(&key(name)).cloned().unwrap_or_default())
}

fn modify<R>(name: &str, f: impl FnOnce(&mut Plot) -> R) -> R {
    PLOTS.with(|m| f(m.borrow_mut().entry(key(name)).or_default()))
}

/// Adds a series of `style`: x array, y array, label, colour (the next of
/// the palette if none) — and for `plot` a line style (`-`, `--`, `o`).
fn add_xy(name: &str, args: &[Value], style: &str) -> Value {
    let x = num::series_data(&arg_s(args, 0));
    let y = num::series_data(&arg_s(args, 1));
    let label = arg_s(args, 2);
    let style = if style == "-" { arg_s_or(args, 4, "-") } else { style.to_string() };
    modify(name, |p| {
        let color = args.get(3).map(Value::to_string_val).filter(|c| !c.trim().is_empty()).unwrap_or_else(|| PALETTE[p.series.len() % PALETTE.len()].to_string());
        let (x, y) = if style == "barh" { (y, x) } else { (x, y) };
        p.series.push(Series { x, y, label, color, style });
    });
    v_null()
}

/// Calls RPLOT `name`'s `method` (lowercase).
pub fn plot_method(name: &str, method: &str, args: &[Value], host: &dyn Host) -> Value {
    match method {
        // --- Creation ---
        "create" | "new" | "init" => {
            modify(name, |_| ());
            v_null()
        }
        "clear" => {
            modify(name, |p| *p = Plot::default());
            v_null()
        }
        // --- Series ---
        "plot" => add_xy(name, args, "-"),
        "bar" => add_xy(name, args, "bar"),
        "barh" => add_xy(name, args, "barh"),
        "scatter" => add_xy(name, args, "o"),
        "step" => add_xy(name, args, "step"),
        "area" | "fill_between" => add_xy(name, args, "area"),
        "addseries" | "add_series" | "series" => {
            // addseries label, y values [, x values [, colour]]: a line,
            // drawn at once where the chart is shown
            let label = arg_s(args, 0);
            let y = num::series_data(&arg_s(args, 1));
            let xs = arg_s(args, 2);
            let x = if xs.trim().is_empty() { (0..y.len()).map(|i| i as f64).collect() } else { num::series_data(&xs) };
            modify(name, |p| {
                let color = args.get(3).map(Value::to_string_val).filter(|c| !c.trim().is_empty()).unwrap_or_else(|| PALETTE[p.series.len() % PALETTE.len()].to_string());
                p.series.push(Series { x, y, label, color, style: "-".into() });
            });
            host.show_plot(name);
            v_null()
        }
        "hist" | "histogram" => {
            // hist data, bins, label, colour: bars of the counts per bin
            let data = num::series_data(&arg_s(args, 0));
            if data.is_empty() {
                return v_null();
            }
            let bins = arg_i(args, 1, 10).clamp(1, 10_000) as usize;
            let mn = data.iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let width = if mx > mn { (mx - mn) / bins as f64 } else { 1.0 };
            let mut counts = vec![0.0; bins];
            for v in &data {
                counts[(((v - mn) / width) as usize).min(bins - 1)] += 1.0;
            }
            let centers = (0..bins).map(|i| mn + (i as f64 + 0.5) * width).collect();
            let label = arg_s(args, 2);
            modify(name, |p| {
                let color = args.get(3).map(Value::to_string_val).filter(|c| !c.trim().is_empty()).unwrap_or_else(|| PALETTE[p.series.len() % PALETTE.len()].to_string());
                p.series.push(Series { x: centers, y: counts, label, color, style: "bar".into() });
            });
            v_null()
        }
        "pie" => {
            // pie values, "label1,label2,…", "colour1,colour2,…"
            let data = num::series_data(&arg_s(args, 0));
            let (label, color) = (arg_s(args, 1), arg_s(args, 2));
            modify(name, |p| p.series.push(Series { x: data.clone(), y: data, label, color, style: "pie".into() }));
            v_null()
        }
        // --- Lines and text ---
        "hline" | "axhline" => {
            let y = arg_f(args, 0, 0.0);
            let color = arg_s_or(args, 1, "black");
            modify(name, |p| p.series.push(Series { x: Vec::new(), y: vec![y], label: String::new(), color, style: "hline".into() }));
            v_null()
        }
        "vline" | "axvline" => {
            let x = arg_f(args, 0, 0.0);
            let color = arg_s_or(args, 1, "black");
            modify(name, |p| p.series.push(Series { x: vec![x], y: Vec::new(), label: String::new(), color, style: "vline".into() }));
            v_null()
        }
        "annotate" => {
            let text = arg_s(args, 0);
            let (x, y) = (arg_f(args, 1, 0.0), arg_f(args, 2, 0.0));
            let color = arg_s_or(args, 3, "black");
            modify(name, |p| p.annotations.push(Annotation { text, x, y, color }));
            v_null()
        }
        // --- Look ---
        "legend" => {
            modify(name, |p| p.legend = true);
            v_null()
        }
        "title" | "settitle" | "set_title" => {
            if let Some(v) = args.first() {
                modify(name, |p| p.title = v.to_string_val());
            }
            v_str(&state(name).title)
        }
        "xlabel" | "setxlabel" | "set_xlabel" => {
            if let Some(v) = args.first() {
                modify(name, |p| p.xlabel = v.to_string_val());
            }
            v_str(&state(name).xlabel)
        }
        "ylabel" | "setylabel" | "set_ylabel" => {
            if let Some(v) = args.first() {
                modify(name, |p| p.ylabel = v.to_string_val());
            }
            v_str(&state(name).ylabel)
        }
        "grid" => {
            let on = arg_i(args, 0, 1) != 0;
            modify(name, |p| p.grid = on);
            v_null()
        }
        // --- Output ---
        "savefig" | "save" => {
            let file = arg_s_or(args, 0, "plot.png");
            host.save_plot(name, &file);
            v_null()
        }
        "render" | "show" => {
            host.show_plot(name);
            v_null()
        }
        // --- Size and axes ---
        "figsize" => {
            // figsize width, height in inches (Matplotlib's 6.4 x 4.8)
            let (w, h) = (arg_f(args, 0, 6.4), arg_f(args, 1, 4.8));
            modify(name, |p| {
                p.width = (w * p.dpi as f64).clamp(1.0, 8000.0) as u32;
                p.height = (h * p.dpi as f64).clamp(1.0, 8000.0) as u32;
            });
            v_null()
        }
        "xlim" => {
            let lim = (arg_f(args, 0, 0.0), arg_f(args, 1, 1.0));
            modify(name, |p| p.xlim = Some(lim));
            v_null()
        }
        "ylim" => {
            let lim = (arg_f(args, 0, 0.0), arg_f(args, 1, 1.0));
            modify(name, |p| p.ylim = Some(lim));
            v_null()
        }
        "xscale" => {
            let s = arg_s_or(args, 0, "linear").trim().to_ascii_lowercase();
            modify(name, |p| p.xscale = s);
            v_null()
        }
        "yscale" => {
            let s = arg_s_or(args, 0, "linear").trim().to_ascii_lowercase();
            modify(name, |p| p.yscale = s);
            v_null()
        }
        _ => {
            let v = plot_get_prop(name, method);
            if matches!(v, Value::Null) {
                host.warn(&format!("[WARN] RPlot.{method}() not implemented"));
            }
            v
        }
    }
}

/// Reads RPLOT `name`'s property `prop` (lowercase); Null if it has none.
pub fn plot_get_prop(name: &str, prop: &str) -> Value {
    let p = state(name);
    match prop {
        "title" => v_str(&p.title),
        "xlabel" => v_str(&p.xlabel),
        "ylabel" => v_str(&p.ylabel),
        "grid" => v_int(p.grid as i64),
        "width" => v_int(p.width as i64),
        "height" => v_int(p.height as i64),
        "dpi" => v_int(p.dpi as i64),
        _ => v_null(),
    }
}

/// Sets RPLOT `name`'s property `prop` (lowercase); false if it has none.
pub fn plot_set_prop(name: &str, prop: &str, val: &Value) -> bool {
    match prop {
        "title" => modify(name, |p| p.title = val.to_string_val()),
        "xlabel" => modify(name, |p| p.xlabel = val.to_string_val()),
        "ylabel" => modify(name, |p| p.ylabel = val.to_string_val()),
        "grid" => modify(name, |p| p.grid = val.to_f64() != 0.0),
        "width" => modify(name, |p| p.width = val.to_f64().clamp(1.0, 8000.0) as u32),
        "height" => modify(name, |p| p.height = val.to_f64().clamp(1.0, 8000.0) as u32),
        "dpi" => modify(name, |p| p.dpi = val.to_f64().clamp(50.0, 1200.0) as u32),
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::super::test_host::TestHost;
    use super::*;

    #[test]
    fn ticks_and_legend_place() {
        assert_eq!(nice_ticks(0.9, 3.1, 6), vec![1.0, 1.5, 2.0, 2.5, 3.0]);
        assert_eq!(nice_ticks(-4.0, 184.0, 6), vec![0.0, 50.0, 100.0, 150.0]);
        assert_eq!((tick_text(2.0), tick_text(2.5), tick_text(-0.25)), ("2".into(), "2.5".into(), "-0.25".into()));
        let mut p = Plot::default();
        p.series.push(Series { x: vec![1.0, 2.0, 3.0], y: vec![1.0, 2.0, 3.0], label: "up".into(), color: "red".into(), style: "-".into() });
        // (a rising line fills the upper right and lower left)
        assert_eq!(legend_corner(&p, (1.0, 3.0), (1.0, 3.0), (0.3, 0.3)), (false, true));
        // (no corner free: the axis reaches higher, the legend above the data)
        p.series.push(Series { x: vec![1.0, 3.0, 3.0], y: vec![3.0, 3.0, 1.0], label: "top".into(), color: "blue".into(), style: "o".into() });
        let ((_, (y0, y1)), (_, top)) = layout(&p, Some((0.3, 0.3)));
        assert!(top && y1 > 3.2 && y0 < 1.0, "{y0} {y1}");
    }

    #[test]
    fn series_and_properties() {
        let h = TestHost::default();
        num::set_data("px", vec![1.0, 2.0, 3.0]);
        num::set_data("py", vec![4.0, 5.0, 6.0]);
        plot_set_prop("P1", "title", &v_str("T"));
        plot_set_prop("p1", "width", &v_int(7));
        plot_method("p1", "plot", &[v_str("PX"), v_str("py"), v_str("line")], &h);
        plot_method("p1", "bar", &[v_str("px"), v_str("1,1,1")], &h);
        plot_method("p1", "savefig", &[v_str("a.png")], &h);
        let p = state("p1");
        assert_eq!(p.title, "T");
        assert_eq!(p.pixel_size(), (700, 480));
        assert_eq!(p.series[0].color, "blue");
        assert_eq!(p.series[1].color, "red");
        assert_eq!(p.series[1].y, vec![1.0, 1.0, 1.0]);
        assert_eq!(h.shown.borrow().as_slice(), ["save p1 a.png"]);
        assert_eq!(plot_method("p1", "title", &[], &h).to_string_val(), "T");
    }
}
