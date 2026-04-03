//! Data science component backends: RNum (ndarray), RDataFrame (polars), RPlot (plotters).
//!
//! Each component stores its internal state in thread-local maps and exposes
//! a `*_method(name, method, args) -> Value` entry point for the component system.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::value::{v_dbl, v_int, v_null, v_str, Value};

// ---------------------------------------------------------------------------
// RNum — backed by ndarray
// ---------------------------------------------------------------------------

use ndarray::Array1;

thread_local! {
    static NUMPY_ARRAYS: RefCell<HashMap<String, Array1<f64>>> = RefCell::new(HashMap::new());
}

fn num_arr_get(name: &str) -> Array1<f64> {
    NUMPY_ARRAYS.with(|m| {
        m.borrow().get(&name.to_lowercase()).cloned().unwrap_or_else(|| Array1::zeros(0))
    })
}

fn num_arr_set(name: &str, arr: Array1<f64>) {
    NUMPY_ARRAYS.with(|m| {
        m.borrow_mut().insert(name.to_lowercase(), arr);
    });
}

/// Dispatch method calls on RNum components.
pub fn num_method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "arange" => {
            let start = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let stop = args.get(1).map(|v| v.to_f64()).unwrap_or(10.0);
            let step = args.get(2).map(|v| v.to_f64()).unwrap_or(1.0);
            let mut vals = Vec::new();
            let mut cur = start;
            while cur < stop {
                vals.push(cur);
                cur += step;
            }
            num_arr_set(name, Array1::from(vals));
            v_null()
        }
        "linspace" => {
            let start = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let stop = args.get(1).map(|v| v.to_f64()).unwrap_or(1.0);
            let num = args.get(2).map(|v| v.to_i64()).unwrap_or(50) as usize;
            let arr = Array1::linspace(start, stop, num);
            num_arr_set(name, arr);
            v_null()
        }
        "zeros" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(10) as usize;
            num_arr_set(name, Array1::zeros(n));
            v_null()
        }
        "ones" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(10) as usize;
            num_arr_set(name, Array1::ones(n));
            v_null()
        }
        "sum" => {
            let arr = num_arr_get(name);
            v_dbl(arr.sum())
        }
        "mean" => {
            let arr = num_arr_get(name);
            let len = arr.len();
            if len == 0 { v_dbl(0.0) } else { v_dbl(arr.sum() / len as f64) }
        }
        "min" => {
            let arr = num_arr_get(name);
            v_dbl(arr.iter().cloned().fold(f64::INFINITY, f64::min))
        }
        "max" => {
            let arr = num_arr_get(name);
            v_dbl(arr.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
        }
        "std" => {
            let arr = num_arr_get(name);
            let len = arr.len() as f64;
            if len <= 0.0 {
                v_dbl(0.0)
            } else {
                let mean = arr.sum() / len;
                let var = arr.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / len;
                v_dbl(var.sqrt())
            }
        }
        "dot" => {
            // dot product with another named array
            let other_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let a = num_arr_get(name);
            let b = num_arr_get(&other_name);
            let result = a.dot(&b);
            // Store result back into the calling array (or we return it)
            v_dbl(result)
        }
        "tolist" => {
            // Return all values as a comma-separated string
            let arr = num_arr_get(name);
            let s: String = arr.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
            v_str(&s)
        }
        "sin" => {
            let arr = num_arr_get(name);
            let result = arr.mapv(f64::sin);
            num_arr_set(name, result);
            v_null()
        }
        "cos" => {
            let arr = num_arr_get(name);
            let result = arr.mapv(f64::cos);
            num_arr_set(name, result);
            v_null()
        }
        "sqrt" => {
            let arr = num_arr_get(name);
            let result = arr.mapv(f64::sqrt);
            num_arr_set(name, result);
            v_null()
        }
        "abs" => {
            let arr = num_arr_get(name);
            let result = arr.mapv(f64::abs);
            num_arr_set(name, result);
            v_null()
        }
        "reshape" => {
            // For 1D arrays, reshape is a no-op but we store the shape info
            v_null()
        }
        _ => {
            eprintln!("[WARN] RNum.{}() not implemented", method);
            v_null()
        }
    }
}

/// Get a RNum property.
pub fn num_get_prop(name: &str, prop: &str) -> Value {
    match prop {
        "size" => {
            let arr = num_arr_get(name);
            v_int(arr.len() as i64)
        }
        "data" => {
            // Return as comma-separated string
            let arr = num_arr_get(name);
            let s: String = arr.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
            v_str(&s)
        }
        _ => v_null(),
    }
}

/// Set a RNum property.
pub fn num_set_prop(name: &str, prop: &str, val: &Value) {
    match prop {
        "data" => {
            // Parse comma-separated float values
            let s = val.to_string_val();
            let vals: Vec<f64> = s.split(',')
                .filter_map(|v| v.trim().parse::<f64>().ok())
                .collect();
            num_arr_set(name, Array1::from(vals));
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// RDataFrame — backed by polars
// ---------------------------------------------------------------------------

use polars::prelude::*;

thread_local! {
    static PANDAS_FRAMES: RefCell<HashMap<String, DataFrame>> = RefCell::new(HashMap::new());
}

fn df_store_get(name: &str) -> DataFrame {
    PANDAS_FRAMES.with(|m| {
        m.borrow().get(&name.to_lowercase()).cloned().unwrap_or_else(|| DataFrame::empty())
    })
}

fn df_store_set(name: &str, df: DataFrame) {
    PANDAS_FRAMES.with(|m| {
        m.borrow_mut().insert(name.to_lowercase(), df);
    });
}

/// Dispatch method calls on RDataFrame components.
pub fn dataframe_method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "loadfromcsv" | "readcsv" => {
            let path = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            match CsvReadOptions::default()
                .with_has_header(true)
                .try_into_reader_with_file_path(Some(path.into()))
            {
                Ok(reader) => match reader.finish() {
                    Ok(df) => {
                        df_store_set(name, df);
                    }
                    Err(e) => eprintln!("[ERROR] RDataFrame.loadfromcsv: {}", e),
                },
                Err(e) => eprintln!("[ERROR] RDataFrame.loadfromcsv: {}", e),
            }
            v_null()
        }
        "sort" => {
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let ascending = args.get(1).map(|v| v.to_i64() != 0).unwrap_or(true);
            let df = df_store_get(name);
            match df.sort([&col_name], SortMultipleOptions::default().with_order_descending(!ascending)) {
                Ok(sorted) => df_store_set(name, sorted),
                Err(e) => eprintln!("[ERROR] RDataFrame.sort: {}", e),
            }
            v_null()
        }
        "filter" => {
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let op = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let val = args.get(2).cloned().unwrap_or_else(v_null);
            let df = df_store_get(name);

            // Use lazy API for filtering
            let lazy = df.lazy();
            let col_expr = polars::lazy::dsl::col(&col_name);
            let filter_expr = {
                let val_str = val.to_string_val();
                let is_num = val_str.parse::<f64>().is_ok();
                match op.as_str() {
                    "==" if is_num => col_expr.cast(DataType::Float64).eq(lit(val.to_f64())),
                    "==" => col_expr.cast(DataType::String).eq(lit(val_str)),
                    ">" => col_expr.cast(DataType::Float64).gt(lit(val.to_f64())),
                    "<" => col_expr.cast(DataType::Float64).lt(lit(val.to_f64())),
                    ">=" => col_expr.cast(DataType::Float64).gt_eq(lit(val.to_f64())),
                    "<=" => col_expr.cast(DataType::Float64).lt_eq(lit(val.to_f64())),
                    "!=" if is_num => col_expr.cast(DataType::Float64).neq(lit(val.to_f64())),
                    "!=" => col_expr.cast(DataType::String).neq(lit(val_str)),
                    _ => lit(true), // no-op filter
                }
            };

            match lazy.filter(filter_expr).collect() {
                Ok(filtered) => df_store_set(name, filtered),
                Err(e) => eprintln!("[ERROR] RDataFrame.filter: {}", e),
            }
            v_null()
        }
        "groupby" => {
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let agg = args.get(1).map(|v| v.to_string_val()).unwrap_or_else(|| "mean".to_string());
            let df = df_store_get(name);
            let lazy = df.lazy();
            let group_col = polars::lazy::dsl::col(&col_name);
            let agg_expr = polars::lazy::dsl::all().exclude([&col_name]);
            let grouped = match agg.as_str() {
                "mean" => lazy.group_by([group_col]).agg([agg_expr.mean()]),
                "sum" => lazy.group_by([group_col]).agg([agg_expr.sum()]),
                "count" => lazy.group_by([group_col]).agg([agg_expr.count()]),
                "min" => lazy.group_by([group_col]).agg([agg_expr.min()]),
                "max" => lazy.group_by([group_col]).agg([agg_expr.max()]),
                _ => lazy.group_by([group_col]).agg([agg_expr.mean()]),
            };
            match grouped.collect() {
                Ok(result) => df_store_set(name, result),
                Err(e) => eprintln!("[ERROR] RDataFrame.groupby: {}", e),
            }
            v_null()
        }
        "head" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(5) as usize;
            let df = df_store_get(name);
            df_store_set(name, df.head(Some(n)));
            v_null()
        }
        "cell" => {
            let row = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            let col_idx = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as usize;
            let df = df_store_get(name);
            if col_idx < df.width() && row < df.height() {
                let series = df.get_columns()[col_idx].as_materialized_series();
                match series.get(row) {
                    Ok(av) => v_str(&format!("{}", av)),
                    Err(_) => v_str(""),
                }
            } else {
                v_str("")
            }
        }
        "describe" => {
            // Polars 0.46 may not have describe; provide manual summary
            let df = df_store_get(name);
            let mut cols_vec: Vec<Column> = Vec::new();
            let stat_names: Column = Series::new(
                PlSmallStr::from("statistic"),
                &["count", "mean", "min", "max"],
            ).into();
            cols_vec.push(stat_names);

            for series_col in df.get_columns() {
                let s = series_col.as_materialized_series();
                let count_val = format!("{}", s.len());
                let (mean_val, min_val, max_val) = if let Ok(f) = s.cast(&DataType::Float64) {
                    let ca = f.f64().unwrap();
                    (
                        ca.mean().map(|v| format!("{:.2}", v)).unwrap_or_default(),
                        ca.min().map(|v| format!("{:.2}", v)).unwrap_or_default(),
                        ca.max().map(|v| format!("{:.2}", v)).unwrap_or_default(),
                    )
                } else {
                    (String::new(), String::new(), String::new())
                };
                let col_series: Column = Series::new(
                    PlSmallStr::from(s.name().as_str()),
                    &[count_val.as_str(), mean_val.as_str(), min_val.as_str(), max_val.as_str()],
                ).into();
                cols_vec.push(col_series);
            }
            if let Ok(desc_df) = DataFrame::new(cols_vec) {
                df_store_set(name, desc_df);
            }
            v_null()
        }
        "clear" => {
            df_store_set(name, DataFrame::empty());
            v_null()
        }
        "columns" => {
            let df = df_store_get(name);
            let cols: Vec<&str> = df.get_column_names().into_iter().map(|c| c.as_str()).collect();
            v_str(&cols.join(","))
        }
        "rows" | "rowcount" => {
            let df = df_store_get(name);
            v_int(df.height() as i64)
        }
        "tostring" | "show" | "print" => {
            let df = df_store_get(name);
            let s = format!("{}", df);
            println!("{}", s);
            v_str(&s)
        }
        _ => {
            eprintln!("[WARN] RDataFrame.{}() not implemented", method);
            v_null()
        }
    }
}

/// Get a RDataFrame property.
pub fn dataframe_get_prop(name: &str, prop: &str) -> Value {
    let df = df_store_get(name);
    match prop {
        "rowcount" => v_int(df.height() as i64),
        "colcount" => v_int(df.width() as i64),
        "columns" => {
            let cols: Vec<&str> = df.get_column_names().into_iter().map(|c| c.as_str()).collect();
            v_str(&cols.join(","))
        }
        _ => v_null(),
    }
}

// ---------------------------------------------------------------------------
// RPlot — backed by plotters (bitmap output)
// ---------------------------------------------------------------------------

use plotters::prelude::*;

#[derive(Clone, Debug)]
struct PlotSeries {
    x_data: Vec<f64>,
    y_data: Vec<f64>,
    label: String,
    color: String,
    style: String, // "-" for line, "o" for scatter, "bar" for bar
}

#[derive(Clone, Debug)]
struct PlotState {
    title: String,
    xlabel: String,
    ylabel: String,
    grid: bool,
    width: u32,
    height: u32,
    series: Vec<PlotSeries>,
    show_legend: bool,
}

impl Default for PlotState {
    fn default() -> Self {
        Self {
            title: String::new(),
            xlabel: String::new(),
            ylabel: String::new(),
            grid: false,
            width: 640,
            height: 480,
            series: Vec::new(),
            show_legend: false,
        }
    }
}

thread_local! {
    static PLOT_STATES: RefCell<HashMap<String, PlotState>> = RefCell::new(HashMap::new());
}

fn plot_get(name: &str) -> PlotState {
    PLOT_STATES.with(|m| {
        m.borrow().get(&name.to_lowercase()).cloned().unwrap_or_default()
    })
}

fn plot_set(name: &str, state: PlotState) {
    PLOT_STATES.with(|m| {
        m.borrow_mut().insert(name.to_lowercase(), state);
    });
}

fn parse_color(color_name: &str) -> RGBColor {
    match color_name.to_lowercase().as_str() {
        "red" => RGBColor(255, 0, 0),
        "green" => RGBColor(0, 128, 0),
        "blue" => RGBColor(0, 0, 255),
        "black" => RGBColor(0, 0, 0),
        "white" => RGBColor(255, 255, 255),
        "orange" => RGBColor(255, 165, 0),
        "purple" => RGBColor(128, 0, 128),
        "cyan" => RGBColor(0, 255, 255),
        "magenta" => RGBColor(255, 0, 255),
        "yellow" => RGBColor(255, 255, 0),
        "steelblue" => RGBColor(70, 130, 180),
        "gray" | "grey" => RGBColor(128, 128, 128),
        _ => RGBColor(0, 0, 0),
    }
}

/// Dispatch method calls on RPlot components.
pub fn plot_method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "clear" => {
            plot_set(name, PlotState::default());
            v_null()
        }
        "plot" => {
            // plot xArray, yArray, label, color, style
            let x_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let color = args.get(3).map(|v| v.to_string_val()).unwrap_or_else(|| "blue".to_string());
            let style = args.get(4).map(|v| v.to_string_val()).unwrap_or_else(|| "-".to_string());

            let x_data: Vec<f64> = NUMPY_ARRAYS.with(|m| {
                m.borrow().get(&x_name.to_lowercase()).map(|a| a.to_vec()).unwrap_or_default()
            });
            let y_data: Vec<f64> = NUMPY_ARRAYS.with(|m| {
                m.borrow().get(&y_name.to_lowercase()).map(|a| a.to_vec()).unwrap_or_default()
            });

            let mut state = plot_get(name);
            state.series.push(PlotSeries { x_data, y_data, label, color, style });
            plot_set(name, state);
            v_null()
        }
        "bar" => {
            let x_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let color = args.get(3).map(|v| v.to_string_val()).unwrap_or_else(|| "steelblue".to_string());

            let x_data: Vec<f64> = NUMPY_ARRAYS.with(|m| {
                m.borrow().get(&x_name.to_lowercase()).map(|a| a.to_vec()).unwrap_or_default()
            });
            let y_data: Vec<f64> = NUMPY_ARRAYS.with(|m| {
                m.borrow().get(&y_name.to_lowercase()).map(|a| a.to_vec()).unwrap_or_default()
            });

            let mut state = plot_get(name);
            state.series.push(PlotSeries { x_data, y_data, label, color, style: "bar".to_string() });
            plot_set(name, state);
            v_null()
        }
        "scatter" => {
            let x_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let color = args.get(3).map(|v| v.to_string_val()).unwrap_or_else(|| "red".to_string());

            let x_data: Vec<f64> = NUMPY_ARRAYS.with(|m| {
                m.borrow().get(&x_name.to_lowercase()).map(|a| a.to_vec()).unwrap_or_default()
            });
            let y_data: Vec<f64> = NUMPY_ARRAYS.with(|m| {
                m.borrow().get(&y_name.to_lowercase()).map(|a| a.to_vec()).unwrap_or_default()
            });

            let mut state = plot_get(name);
            state.series.push(PlotSeries { x_data, y_data, label, color, style: "o".to_string() });
            plot_set(name, state);
            v_null()
        }
        "legend" => {
            let mut state = plot_get(name);
            state.show_legend = true;
            plot_set(name, state);
            v_null()
        }
        "savefig" => {
            let filename = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| "plot.png".to_string());
            render_plot(name, &filename);
            v_null()
        }
        _ => {
            eprintln!("[WARN] RPlot.{}() not implemented", method);
            v_null()
        }
    }
}

/// Get a RPlot property.
pub fn plot_get_prop(name: &str, prop: &str) -> Value {
    let state = plot_get(name);
    match prop {
        "title" => v_str(&state.title),
        "xlabel" => v_str(&state.xlabel),
        "ylabel" => v_str(&state.ylabel),
        "grid" => v_int(if state.grid { 1 } else { 0 }),
        "width" => v_int(state.width as i64),
        "height" => v_int(state.height as i64),
        _ => v_null(),
    }
}

/// Set a RPlot property.
pub fn plot_set_prop(name: &str, prop: &str, val: &Value) {
    let mut state = plot_get(name);
    match prop {
        "title" => state.title = val.to_string_val(),
        "xlabel" => state.xlabel = val.to_string_val(),
        "ylabel" => state.ylabel = val.to_string_val(),
        "grid" => state.grid = val.to_i64() != 0,
        "width" => state.width = val.to_i64() as u32,
        "height" => state.height = val.to_i64() as u32,
        _ => {}
    }
    plot_set(name, state);
}

/// Render the accumulated plot state to a PNG file.
fn render_plot(name: &str, filename: &str) {
    let state = plot_get(name);
    let w = state.width;
    let h = state.height;

    let root = BitMapBackend::new(filename, (w, h)).into_drawing_area();
    if root.fill(&WHITE).is_err() { return; }

    // Compute data bounds
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for s in &state.series {
        for &x in &s.x_data {
            if x < x_min { x_min = x; }
            if x > x_max { x_max = x; }
        }
        for &y in &s.y_data {
            if y < y_min { y_min = y; }
            if y > y_max { y_max = y; }
        }
    }
    if x_min >= x_max { x_min = 0.0; x_max = 1.0; }
    if y_min >= y_max { y_min = 0.0; y_max = 1.0; }
    // Add 5% margin
    let x_margin = (x_max - x_min) * 0.05;
    let y_margin = (y_max - y_min) * 0.05;
    x_min -= x_margin;
    x_max += x_margin;
    y_min -= y_margin;
    y_max += y_margin;

    let mut chart_builder = ChartBuilder::on(&root);
    if !state.title.is_empty() {
        chart_builder.caption(&state.title, ("sans-serif", 20));
    }
    chart_builder.margin(10)
        .x_label_area_size(35)
        .y_label_area_size(45);

    let chart_result = chart_builder.build_cartesian_2d(
        x_min as f32..x_max as f32,
        y_min as f32..y_max as f32,
    );
    let mut chart = match chart_result {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] RPlot chart build: {}", e);
            return;
        }
    };

    let mut mesh = chart.configure_mesh();
    if !state.xlabel.is_empty() {
        mesh.x_desc(&state.xlabel);
    }
    if !state.ylabel.is_empty() {
        mesh.y_desc(&state.ylabel);
    }
    if state.grid {
        let _ = mesh.draw();
    } else {
        mesh.disable_mesh();
        let _ = mesh.draw();
    }

    for s in &state.series {
        let color = parse_color(&s.color);
        let points: Vec<(f32, f32)> = s.x_data.iter().zip(s.y_data.iter())
            .map(|(&x, &y)| (x as f32, y as f32))
            .collect();

        match s.style.as_str() {
            "-" | "--" | "line" => {
                let line = LineSeries::new(points.clone(), &color);
                if !s.label.is_empty() {
                    let _ = chart.draw_series(line)
                        .map(|a| a.label(&s.label).legend(move |(x, y)| {
                            PathElement::new(vec![(x, y), (x + 20, y)], color)
                        }));
                } else {
                    let _ = chart.draw_series(line);
                }
            }
            "o" | "scatter" => {
                let _ = chart.draw_series(
                    points.iter().map(|&(x, y)| Circle::new((x, y), 3, color.filled()))
                );
            }
            "bar" => {
                let bar_width = if points.len() > 1 {
                    ((points[1].0 - points[0].0) * 0.8) as f32
                } else {
                    0.8f32
                };
                let _ = chart.draw_series(
                    points.iter().map(|&(x, y)| {
                        Rectangle::new(
                            [(x - bar_width / 2.0, 0.0f32), (x + bar_width / 2.0, y)],
                            color.filled(),
                        )
                    })
                );
            }
            _ => {
                let line = LineSeries::new(points, &color);
                let _ = chart.draw_series(line);
            }
        }
    }

    if state.show_legend {
        let _ = chart.configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw();
    }

    let _ = root.present();
}

/// Render plot to a temp file and return the path.
pub fn plot_render_to_file(name: &str) -> String {
    let filename = format!("/tmp/rapidr_plot_{}.png", name.to_lowercase());
    render_plot(name, &filename);
    filename
}
