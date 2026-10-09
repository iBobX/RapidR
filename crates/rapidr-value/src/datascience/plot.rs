//! RPLOT: a chart in the manner of Matplotlib — its model (the series,
//! titles, limits, size) and every member. It's drawn by [`super::chart`]
//! (the UI kernel's ops, one renderer for every runtime); the runtime only
//! puts the pixels somewhere ([`super::Host::save_plot`], a QIMAGE).
//!
//! A series' data are RNUM arrays named by their components' names
//! (`plt.plot "x", "y"`), numbers written in place (`"1,2,3"`), or — for x
//! — names (`"North,South,East"`): bars over their categories.

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
    /// The names x stands for (bars over categories): x is 0, 1, 2 …
    pub categories: Vec<String>,
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
    /// None: the default look (light horizontal gridlines); Some(true):
    /// both ways (`Grid = 1`); Some(false): none (`Grid = 0`).
    pub grid: Option<bool>,
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
    /// `XTicks`: the x axis' tick positions and their names.
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
            annotations: Vec::new(),
            legend: false,
            xlim: None,
            ylim: None,
            xscale: "linear".into(),
            yscale: "linear".into(),
            xticks: None,
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
}

/// The colours series without one take, in turn: the chart palette
/// (`super::colors::PALETTE`, Matplotlib's `C0` … `C9` names for it).
pub const PALETTE: &[&str] = &["C0", "C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9"];

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

/// A series' x: an array, numbers, or names (categories: x is 0, 1, 2 …).
fn x_data(arg: &str) -> (Vec<f64>, Vec<String>) {
    if !num::exists(arg) {
        let items: Vec<String> = arg.split(',').map(|t| t.trim().to_string()).collect();
        if items.iter().any(|t| !t.is_empty() && super::parse_num(t).is_none()) {
            return ((0..items.len()).map(|i| i as f64).collect(), items);
        }
    }
    (num::series_data(arg), Vec::new())
}

/// Adds a series of `style`: x array, y array, label, colour (the next of
/// the palette if none) — and for `plot` a line style (`-`, `--`, `o`).
fn add_xy(name: &str, args: &[Value], style: &str) -> Value {
    let (x, categories) = x_data(&arg_s(args, 0));
    let y = num::series_data(&arg_s(args, 1));
    let label = arg_s(args, 2);
    let style = if style == "-" { arg_s_or(args, 4, "-") } else { style.to_string() };
    modify(name, |p| {
        let color = args.get(3).map(Value::to_string_val).filter(|c| !c.trim().is_empty()).unwrap_or_else(|| PALETTE[p.series.len() % PALETTE.len()].to_string());
        let (x, y) = if style == "barh" { (y, x) } else { (x, y) };
        p.series.push(Series { x, y, label, color, style, categories });
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
            // (the series, titles and settings go; the size stays, as
            // Matplotlib's clf keeps the figure's — an RPLOT on a form keeps
            // its place)
            modify(name, |p| {
                let (width, height, dpi) = (p.width, p.height, p.dpi);
                *p = Plot { width, height, dpi, ..Plot::default() };
            });
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
                p.series.push(Series { x, y, label, color, style: "-".into(), categories: Vec::new() });
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
                p.series.push(Series { x: centers, y: counts, label, color, style: "hist".into(), categories: Vec::new() });
            });
            v_null()
        }
        "pie" => {
            // pie values, "label1,label2,…", "colour1,colour2,…"
            let data = num::series_data(&arg_s(args, 0));
            let (label, color) = (arg_s(args, 1), arg_s(args, 2));
            modify(name, |p| p.series.push(Series { x: data.clone(), y: data, label, color, style: "pie".into(), categories: Vec::new() }));
            v_null()
        }
        // --- Lines and text ---
        "hline" | "axhline" => {
            let y = arg_f(args, 0, 0.0);
            let color = arg_s_or(args, 1, "black");
            modify(name, |p| p.series.push(Series { x: Vec::new(), y: vec![y], label: String::new(), color, style: "hline".into(), categories: Vec::new() }));
            v_null()
        }
        "vline" | "axvline" => {
            let x = arg_f(args, 0, 0.0);
            let color = arg_s_or(args, 1, "black");
            modify(name, |p| p.series.push(Series { x: vec![x], y: Vec::new(), label: String::new(), color, style: "vline".into(), categories: Vec::new() }));
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
            modify(name, |p| p.grid = Some(on));
            v_null()
        }
        // --- Output ---
        "savefig" | "save" => {
            let file = arg_s_or(args, 0, "plot.png");
            // (savefig file, scale: a PNG `scale` times the chart's size)
            host.save_plot(name, &file, arg_f(args, 1, 1.0).clamp(0.1, 8.0));
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
        "xticks" => {
            // xticks "Jan,Feb,Mar" [, positions]: the x axis' ticks named
            // (at the first series' x when there are as many, else 0, 1, 2 …)
            let names: Vec<String> = arg_s(args, 0).split(',').map(|t| t.trim().to_string()).collect();
            let at = args.get(1).map(|v| num::series_data(&v.to_string_val())).filter(|v| !v.is_empty());
            modify(name, |p| {
                let at = at.unwrap_or_else(|| match p.series.iter().find(|s| !matches!(s.style.as_str(), "hline" | "vline")) {
                    Some(s) if s.x.len() == names.len() => s.x.clone(),
                    _ => (0..names.len()).map(|i| i as f64).collect(),
                });
                p.xticks = (names.iter().any(|n| !n.is_empty())).then_some((at, names));
            });
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
        "grid" => v_int(i64::from(p.grid == Some(true))),
        "legend" => v_int(i64::from(p.legend)),
        "count" | "seriescount" => v_int(p.series.len() as i64),
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
        "grid" => modify(name, |p| p.grid = Some(val.to_f64() != 0.0)),
        "legend" => modify(name, |p| p.legend = val.to_f64() != 0.0),
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
        assert_eq!(p.series[0].color, "C0");
        assert_eq!(p.series[1].color, "C1");
        assert_eq!(p.series[1].y, vec![1.0, 1.0, 1.0]);
        assert_eq!(h.shown.borrow().as_slice(), ["save p1 a.png"]);
        assert_eq!(plot_method("p1", "title", &[], &h).to_string_val(), "T");
        // (names for x: bars over their categories)
        plot_method("p2", "bar", &[v_str("North, South,East"), v_str("3,5,2")], &h);
        let q = state("p2");
        assert_eq!(q.series[0].categories, ["North", "South", "East"]);
        assert_eq!(q.series[0].x, vec![0.0, 1.0, 2.0]);
    }
}
