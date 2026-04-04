//! Data science component backends: RNum (ndarray), RDataFrame (polars), RPlot (plotters).
//!
//! Each component stores its internal state in thread-local maps and exposes
//! a `*_method(name, method, args) -> Value` entry point for the component system.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::value::{v_dbl, v_int, v_null, v_str, Value};

// ---------------------------------------------------------------------------
// RNum — backed by ndarray  (With some NumPy-compatibility)
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
        // --- Creation ---
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
        "full" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(10) as usize;
            let fill = args.get(1).map(|v| v.to_f64()).unwrap_or(0.0);
            num_arr_set(name, Array1::from_elem(n, fill));
            v_null()
        }
        "fromlist" | "from_list" => {
            // Parse comma-separated values
            let s = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let vals: Vec<f64> = s.split(',').filter_map(|v| v.trim().parse::<f64>().ok()).collect();
            num_arr_set(name, Array1::from(vals));
            v_null()
        }

        // --- Aggregation ---
        "sum" => { v_dbl(num_arr_get(name).sum()) }
        "mean" => {
            let arr = num_arr_get(name);
            if arr.is_empty() { v_dbl(0.0) } else { v_dbl(arr.sum() / arr.len() as f64) }
        }
        "min" => { v_dbl(num_arr_get(name).iter().cloned().fold(f64::INFINITY, f64::min)) }
        "max" => { v_dbl(num_arr_get(name).iter().cloned().fold(f64::NEG_INFINITY, f64::max)) }
        "std" => {
            let arr = num_arr_get(name);
            let n = arr.len() as f64;
            if n <= 0.0 { return v_dbl(0.0); }
            let mean = arr.sum() / n;
            let var = arr.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
            v_dbl(var.sqrt())
        }
        "var" | "variance" => {
            let arr = num_arr_get(name);
            let n = arr.len() as f64;
            if n <= 0.0 { return v_dbl(0.0); }
            let mean = arr.sum() / n;
            v_dbl(arr.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n)
        }
        "median" => {
            let arr = num_arr_get(name);
            if arr.is_empty() { return v_dbl(0.0); }
            let mut sorted: Vec<f64> = arr.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = sorted.len() / 2;
            if sorted.len() % 2 == 0 {
                v_dbl((sorted[mid - 1] + sorted[mid]) / 2.0)
            } else {
                v_dbl(sorted[mid])
            }
        }
        "argmin" => {
            let arr = num_arr_get(name);
            let idx = arr.iter().enumerate()
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i).unwrap_or(0);
            v_int(idx as i64)
        }
        "argmax" => {
            let arr = num_arr_get(name);
            let idx = arr.iter().enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i).unwrap_or(0);
            v_int(idx as i64)
        }
        "count" => { v_int(num_arr_get(name).len() as i64) }
        "ptp" => {
            // Peak to peak (max - min)
            let arr = num_arr_get(name);
            let mn = arr.iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = arr.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            v_dbl(mx - mn)
        }

        // --- Element-wise math (in-place) ---
        "sin" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::sin)); v_null() }
        "cos" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::cos)); v_null() }
        "tan" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::tan)); v_null() }
        "asin" | "arcsin" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::asin)); v_null() }
        "acos" | "arccos" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::acos)); v_null() }
        "atan" | "arctan" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::atan)); v_null() }
        "sqrt" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::sqrt)); v_null() }
        "abs" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::abs)); v_null() }
        "exp" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::exp)); v_null() }
        "log" | "ln" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::ln)); v_null() }
        "log2" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::log2)); v_null() }
        "log10" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::log10)); v_null() }
        "floor" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::floor)); v_null() }
        "ceil" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(f64::ceil)); v_null() }
        "round" => {
            let decimals = args.first().map(|v| v.to_i64()).unwrap_or(0);
            let factor = 10f64.powi(decimals as i32);
            let arr = num_arr_get(name);
            num_arr_set(name, arr.mapv(|x| (x * factor).round() / factor));
            v_null()
        }
        "sign" => {
            let arr = num_arr_get(name);
            num_arr_set(name, arr.mapv(|x| if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 }));
            v_null()
        }
        "reciprocal" => {
            let arr = num_arr_get(name);
            num_arr_set(name, arr.mapv(|x| if x != 0.0 { 1.0 / x } else { f64::INFINITY }));
            v_null()
        }
        "square" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(|x| x * x)); v_null() }
        "negative" | "neg" => { let arr = num_arr_get(name); num_arr_set(name, arr.mapv(|x| -x)); v_null() }

        // --- Element-wise arithmetic with another array or scalar ---
        "add" => {
            let other = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let arr = num_arr_get(name);
            if let Ok(scalar) = other.parse::<f64>() {
                num_arr_set(name, arr.mapv(|x| x + scalar));
            } else {
                let b = num_arr_get(&other);
                if arr.len() == b.len() { num_arr_set(name, &arr + &b); }
            }
            v_null()
        }
        "subtract" | "sub" => {
            let other = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let arr = num_arr_get(name);
            if let Ok(scalar) = other.parse::<f64>() {
                num_arr_set(name, arr.mapv(|x| x - scalar));
            } else {
                let b = num_arr_get(&other);
                if arr.len() == b.len() { num_arr_set(name, &arr - &b); }
            }
            v_null()
        }
        "multiply" | "mul" => {
            let other = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let arr = num_arr_get(name);
            if let Ok(scalar) = other.parse::<f64>() {
                num_arr_set(name, arr.mapv(|x| x * scalar));
            } else {
                let b = num_arr_get(&other);
                if arr.len() == b.len() { num_arr_set(name, &arr * &b); }
            }
            v_null()
        }
        "divide" | "div" => {
            let other = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let arr = num_arr_get(name);
            if let Ok(scalar) = other.parse::<f64>() {
                num_arr_set(name, arr.mapv(|x| if scalar != 0.0 { x / scalar } else { f64::NAN }));
            } else {
                let b = num_arr_get(&other);
                if arr.len() == b.len() {
                    let result = arr.iter().zip(b.iter()).map(|(a, b)| if *b != 0.0 { a / b } else { f64::NAN }).collect::<Vec<_>>();
                    num_arr_set(name, Array1::from(result));
                }
            }
            v_null()
        }
        "power" | "pow" => {
            let exp = args.first().map(|v| v.to_f64()).unwrap_or(2.0);
            let arr = num_arr_get(name);
            num_arr_set(name, arr.mapv(|x| x.powf(exp)));
            v_null()
        }
        "mod" | "fmod" => {
            let divisor = args.first().map(|v| v.to_f64()).unwrap_or(1.0);
            let arr = num_arr_get(name);
            num_arr_set(name, arr.mapv(|x| x % divisor));
            v_null()
        }
        "clip" | "clamp" => {
            let lo = args.first().map(|v| v.to_f64()).unwrap_or(f64::NEG_INFINITY);
            let hi = args.get(1).map(|v| v.to_f64()).unwrap_or(f64::INFINITY);
            let arr = num_arr_get(name);
            num_arr_set(name, arr.mapv(|x| x.max(lo).min(hi)));
            v_null()
        }

        // --- Ordering / manipulation ---
        "sort" => {
            let arr = num_arr_get(name);
            let mut v = arr.to_vec();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            num_arr_set(name, Array1::from(v));
            v_null()
        }
        "reverse" | "flip" => {
            let arr = num_arr_get(name);
            let mut v = arr.to_vec();
            v.reverse();
            num_arr_set(name, Array1::from(v));
            v_null()
        }
        "unique" => {
            let arr = num_arr_get(name);
            let mut v = arr.to_vec();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            v.dedup();
            num_arr_set(name, Array1::from(v));
            v_null()
        }
        "shuffle" => {
            use rand::seq::SliceRandom;
            let arr = num_arr_get(name);
            let mut v = arr.to_vec();
            v.shuffle(&mut rand::rng());
            num_arr_set(name, Array1::from(v));
            v_null()
        }
        "append" | "concatenate" => {
            let other_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let a = num_arr_get(name);
            let b = num_arr_get(&other_name);
            let mut combined = a.to_vec();
            combined.extend(b.to_vec());
            num_arr_set(name, Array1::from(combined));
            v_null()
        }
        "slice" => {
            let start = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            let end = args.get(1).map(|v| v.to_i64() as usize).unwrap_or_else(|| num_arr_get(name).len());
            let arr = num_arr_get(name);
            let end = end.min(arr.len());
            let start = start.min(end);
            num_arr_set(name, arr.slice(ndarray::s![start..end]).to_owned());
            v_null()
        }

        // --- Cumulative ---
        "cumsum" => {
            let arr = num_arr_get(name);
            let mut acc = 0.0;
            let result: Vec<f64> = arr.iter().map(|&x| { acc += x; acc }).collect();
            num_arr_set(name, Array1::from(result));
            v_null()
        }
        "cumprod" => {
            let arr = num_arr_get(name);
            let mut acc = 1.0;
            let result: Vec<f64> = arr.iter().map(|&x| { acc *= x; acc }).collect();
            num_arr_set(name, Array1::from(result));
            v_null()
        }
        "diff" => {
            let arr = num_arr_get(name);
            if arr.len() < 2 { return v_null(); }
            let v = arr.to_vec();
            let result: Vec<f64> = v.windows(2).map(|w| w[1] - w[0]).collect();
            num_arr_set(name, Array1::from(result));
            v_null()
        }

        // --- Dot product / linear algebra ---
        "dot" => {
            let other_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let a = num_arr_get(name);
            let b = num_arr_get(&other_name);
            v_dbl(a.dot(&b))
        }
        "norm" => {
            let arr = num_arr_get(name);
            v_dbl(arr.iter().map(|x| x * x).sum::<f64>().sqrt())
        }
        "normalize" => {
            let arr = num_arr_get(name);
            let norm = arr.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm > 0.0 { num_arr_set(name, arr.mapv(|x| x / norm)); }
            v_null()
        }

        // --- Boolean / search ---
        "any" => {
            let arr = num_arr_get(name);
            v_int(if arr.iter().any(|&x| x != 0.0) { 1 } else { 0 })
        }
        "all" => {
            let arr = num_arr_get(name);
            v_int(if arr.iter().all(|&x| x != 0.0) { 1 } else { 0 })
        }
        "where" | "nonzero" => {
            let arr = num_arr_get(name);
            let indices: Vec<f64> = arr.iter().enumerate()
                .filter(|(_, &x)| x != 0.0).map(|(i, _)| i as f64).collect();
            num_arr_set(name, Array1::from(indices));
            v_null()
        }
        "searchsorted" => {
            let val = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let arr = num_arr_get(name);
            let idx = arr.iter().position(|&x| x >= val).unwrap_or(arr.len());
            v_int(idx as i64)
        }

        // --- Random (instance methods) ---
        "rand" | "random" => {
            use rand::Rng;
            let n = args.first().map(|v| v.to_i64()).unwrap_or(1) as usize;
            let mut rng = rand::rng();
            let vals: Vec<f64> = (0..n).map(|_| rng.random::<f64>()).collect();
            num_arr_set(name, Array1::from(vals));
            v_null()
        }
        "randn" | "random_normal" | "normal" => {
            use rand::Rng;
            let n = args.first().map(|v| v.to_i64()).unwrap_or(1) as usize;
            let mean = args.get(1).map(|v| v.to_f64()).unwrap_or(0.0);
            let std_dev = args.get(2).map(|v| v.to_f64()).unwrap_or(1.0);
            let mut rng = rand::rng();
            // Box-Muller transform for normal distribution
            let vals: Vec<f64> = (0..n).map(|_| {
                let u1: f64 = rng.random();
                let u2: f64 = rng.random();
                mean + std_dev * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
            }).collect();
            num_arr_set(name, Array1::from(vals));
            v_null()
        }
        "uniform" | "random_uniform" => {
            use rand::Rng;
            let lo = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let hi = args.get(1).map(|v| v.to_f64()).unwrap_or(1.0);
            let n = args.get(2).map(|v| v.to_i64()).unwrap_or(1) as usize;
            let mut rng = rand::rng();
            let vals: Vec<f64> = (0..n).map(|_| rng.random::<f64>() * (hi - lo) + lo).collect();
            num_arr_set(name, Array1::from(vals));
            v_null()
        }
        "randint" => {
            use rand::Rng;
            let lo = args.first().map(|v| v.to_i64()).unwrap_or(0);
            let hi = args.get(1).map(|v| v.to_i64()).unwrap_or(10);
            let n = args.get(2).map(|v| v.to_i64()).unwrap_or(1) as usize;
            let mut rng = rand::rng();
            let vals: Vec<f64> = (0..n).map(|_| rng.random_range(lo..=hi) as f64).collect();
            num_arr_set(name, Array1::from(vals));
            v_null()
        }
        "choice" => {
            use rand::Rng;
            let arr = num_arr_get(name);
            if arr.is_empty() { return v_dbl(0.0); }
            let n = args.first().map(|v| v.to_i64()).unwrap_or(1) as usize;
            let mut rng = rand::rng();
            if n == 1 {
                let idx = rng.random_range(0..arr.len());
                return v_dbl(arr[idx]);
            }
            let vals: Vec<f64> = (0..n).map(|_| arr[rng.random_range(0..arr.len())]).collect();
            num_arr_set(name, Array1::from(vals));
            v_null()
        }

        // --- Output ---
        "tolist" | "tostring" => {
            let arr = num_arr_get(name);
            v_str(&arr.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","))
        }
        "print" | "show" => {
            let arr = num_arr_get(name);
            let s = format!("[{}]", arr.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "));
            println!("{}", s);
            v_str(&s)
        }

        "reshape" => { v_null() } // 1D no-op

        "clear" => {
            num_arr_set(name, Array1::zeros(0));
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
        "size" | "length" | "len" => v_int(num_arr_get(name).len() as i64),
        "data" => {
            let arr = num_arr_get(name);
            v_str(&arr.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","))
        }
        "shape" => v_str(&format!("({})", num_arr_get(name).len())),
        "ndim" => v_int(1),
        "dtype" => v_str("float64"),
        _ => v_null(),
    }
}

/// Set a RNum property.
pub fn num_set_prop(name: &str, prop: &str, val: &Value) {
    match prop {
        "data" => {
            let s = val.to_string_val();
            let vals: Vec<f64> = s.split(',').filter_map(|v| v.trim().parse::<f64>().ok()).collect();
            num_arr_set(name, Array1::from(vals));
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// RDataFrame — backed by polars  (With some Pandas-compatibility)
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

/// Resolve a file path: try as-is, then relative to the executable directory.
fn resolve_data_path(path: &str) -> String {
    if std::path::Path::new(path).exists() {
        return path.to_string();
    }
    // Try relative to executable directory
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(path);
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    // Try relative to current working directory (already checked above), then RAPIDR_HOME
    if let Ok(home) = std::env::var("RAPIDR_HOME") {
        let candidate = std::path::Path::new(&home).join("examples").join(path);
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
        let candidate2 = std::path::Path::new(&home).join(path);
        if candidate2.exists() {
            return candidate2.to_string_lossy().to_string();
        }
    }
    path.to_string()
}

/// Dispatch method calls on RDataFrame components.
pub fn dataframe_method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        // --- I/O ---
        "loadfromcsv" | "readcsv" | "read_csv" => {
            let raw_path = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let path = resolve_data_path(&raw_path);
            match CsvReadOptions::default()
                .with_has_header(true)
                .try_into_reader_with_file_path(Some(path.into()))
            {
                Ok(reader) => match reader.finish() {
                    Ok(df) => { df_store_set(name, df); }
                    Err(e) => eprintln!("[ERROR] RDataFrame.readcsv: {}", e),
                },
                Err(e) => eprintln!("[ERROR] RDataFrame.readcsv: {}", e),
            }
            v_null()
        }
        "savetocsv" | "to_csv" | "writecsv" => {
            let path = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| "output.csv".to_string());
            let df = df_store_get(name);
            let mut file = match std::fs::File::create(&path) {
                Ok(f) => f,
                Err(e) => { eprintln!("[ERROR] RDataFrame.to_csv: {}", e); return v_null(); }
            };
            if let Err(e) = CsvWriter::new(&mut file).finish(&mut df.clone()) {
                eprintln!("[ERROR] RDataFrame.to_csv: {}", e);
            }
            v_null()
        }
        "loadfromjson" | "read_json" => {
            let raw_path = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let path = resolve_data_path(&raw_path);
            match std::fs::File::open(&path) {
                Ok(file) => {
                    let reader = std::io::BufReader::new(file);
                    match JsonReader::new(reader).finish() {
                        Ok(df) => { df_store_set(name, df); }
                        Err(e) => eprintln!("[ERROR] RDataFrame.read_json: {}", e),
                    }
                }
                Err(e) => eprintln!("[ERROR] RDataFrame.read_json: {}", e),
            }
            v_null()
        }
        "savetojson" | "to_json" => {
            let path = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| "output.json".to_string());
            let mut df = df_store_get(name);
            let mut file = match std::fs::File::create(&path) {
                Ok(f) => f,
                Err(e) => { eprintln!("[ERROR] RDataFrame.to_json: {}", e); return v_null(); }
            };
            if let Err(e) = JsonWriter::new(&mut file).finish(&mut df) {
                eprintln!("[ERROR] RDataFrame.to_json: {}", e);
            }
            v_null()
        }

        // --- Selection / Indexing ---
        "head" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(5) as usize;
            let df = df_store_get(name);
            df_store_set(name, df.head(Some(n)));
            v_null()
        }
        "tail" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(5) as usize;
            let df = df_store_get(name);
            df_store_set(name, df.tail(Some(n)));
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
        "cellbyname" | "at" => {
            // cell by row index and column name
            let row = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            let col_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let df = df_store_get(name);
            if let Ok(col) = df.column(&col_name) {
                let s = col.as_materialized_series();
                if row < s.len() {
                    match s.get(row) {
                        Ok(av) => return v_str(&format!("{}", av)),
                        Err(_) => {}
                    }
                }
            }
            v_str("")
        }
        "setcell" => {
            let row = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            let col_idx = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as usize;
            let val = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let df = df_store_get(name);
            if col_idx < df.width() && row < df.height() {
                // Replace cell value by reconstructing the column
                let col = df.get_columns()[col_idx].as_materialized_series();
                let col_name = col.name().to_string();
                let mut values: Vec<String> = (0..col.len()).map(|i| {
                    col.get(i).map(|av| format!("{}", av)).unwrap_or_default()
                }).collect();
                if row < values.len() { values[row] = val; }
                let new_col: Column = Series::new(PlSmallStr::from(col_name.as_str()), &values).into();
                let mut new_df = df.clone();
                let _ = new_df.replace_column(col_idx, new_col);
                df_store_set(name, new_df);
            }
            v_null()
        }
        "iloc" => {
            // Select rows by index range: iloc start, end
            let start = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            let end = args.get(1).map(|v| v.to_i64() as usize).unwrap_or_else(|| df_store_get(name).height());
            let df = df_store_get(name);
            let end = end.min(df.height());
            let start = start.min(end);
            let sliced = df.slice(start as i64, end - start);
            df_store_set(name, sliced);
            v_null()
        }
        "select" => {
            // Select specific columns by name (comma-separated or multiple args)
            let col_str = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let col_names: Vec<&str> = col_str.split(',').map(|s| s.trim()).collect();
            let df = df_store_get(name);
            match df.select(col_names) {
                Ok(selected) => df_store_set(name, selected),
                Err(e) => eprintln!("[ERROR] RDataFrame.select: {}", e),
            }
            v_null()
        }

        // --- Sorting ---
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
        "sort_values" => {
            // Pandas-style: sort_values("col1,col2", ascending)
            let cols_str = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let ascending = args.get(1).map(|v| v.to_i64() != 0).unwrap_or(true);
            let cols: Vec<String> = cols_str.split(',').map(|s| s.trim().to_string()).collect();
            let df = df_store_get(name);
            let col_refs: Vec<&str> = cols.iter().map(|s| s.as_str()).collect();
            match df.sort(col_refs, SortMultipleOptions::default().with_order_descending(!ascending)) {
                Ok(sorted) => df_store_set(name, sorted),
                Err(e) => eprintln!("[ERROR] RDataFrame.sort_values: {}", e),
            }
            v_null()
        }

        // --- Filtering ---
        "filter" => {
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let op = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let val = args.get(2).cloned().unwrap_or_else(v_null);
            let df = df_store_get(name);
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
                    "contains" => {
                        // Use equality as fallback since regex feature not enabled
                        col_expr.cast(DataType::String).eq(lit(val_str))
                    }
                    _ => lit(true),
                }
            };
            match lazy.filter(filter_expr).collect() {
                Ok(filtered) => df_store_set(name, filtered),
                Err(e) => eprintln!("[ERROR] RDataFrame.filter: {}", e),
            }
            v_null()
        }
        "query" => {
            // Simplified query: "column op value"  e.g. "age > 30"
            let expr_str = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let parts: Vec<&str> = expr_str.splitn(3, ' ').collect();
            if parts.len() == 3 {
                return dataframe_method(name, "filter", &[v_str(parts[0]), v_str(parts[1]), v_str(parts[2])]);
            }
            v_null()
        }

        // --- Grouping / Aggregation ---
        "groupby" | "group_by" => {
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
                "first" => lazy.group_by([group_col]).agg([agg_expr.first()]),
                "last" => lazy.group_by([group_col]).agg([agg_expr.last()]),
                _ => lazy.group_by([group_col]).agg([agg_expr.mean()]),
            };
            match grouped.collect() {
                Ok(result) => df_store_set(name, result),
                Err(e) => eprintln!("[ERROR] RDataFrame.groupby: {}", e),
            }
            v_null()
        }

        // --- Column operations ---
        "drop" | "drop_column" => {
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let df = df_store_get(name);
            match df.drop(&col_name) {
                Ok(new_df) => df_store_set(name, new_df),
                Err(e) => eprintln!("[ERROR] RDataFrame.drop: {}", e),
            }
            v_null()
        }
        "rename" | "rename_column" => {
            let old_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let new_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let mut df = df_store_get(name);
            if let Err(e) = df.rename(&old_name, PlSmallStr::from(new_name.as_str())) {
                eprintln!("[ERROR] RDataFrame.rename: {}", e);
            }
            df_store_set(name, df);
            v_null()
        }
        "addcolumn" | "add_column" | "set_column" => {
            // addcolumn "col_name", "val1,val2,val3"
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let values_str = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let df = df_store_get(name);
            let vals: Vec<String> = values_str.split(',').map(|s| s.trim().to_string()).collect();
            // Pad or truncate to match df height
            let h = df.height();
            let mut final_vals = vals;
            final_vals.resize(h, String::new());
            let new_col: Column = Series::new(PlSmallStr::from(col_name.as_str()), &final_vals[..h]).into();
            let mut new_df = df;
            match new_df.with_column(new_col) {
                Ok(df_with) => df_store_set(name, df_with.clone()),
                Err(e) => eprintln!("[ERROR] RDataFrame.addcolumn: {}", e),
            }
            v_null()
        }

        // --- Missing data ---
        "fillna" | "fill_null" => {
            let fill_val = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| "0".to_string());
            let df = df_store_get(name);
            let lazy = df.lazy();
            match lazy.with_columns([polars::lazy::dsl::all().fill_null(lit(fill_val))]).collect() {
                Ok(filled) => df_store_set(name, filled),
                Err(e) => eprintln!("[ERROR] RDataFrame.fillna: {}", e),
            }
            v_null()
        }
        "dropna" | "drop_nulls" => {
            let df = df_store_get(name);
            let lazy = df.lazy();
            match lazy.drop_nulls(None::<Vec<Expr>>).collect() {
                Ok(cleaned) => df_store_set(name, cleaned),
                Err(e) => eprintln!("[ERROR] RDataFrame.dropna: {}", e),
            }
            v_null()
        }

        // --- Statistics ---
        "describe" => {
            let df = df_store_get(name);
            let mut cols_vec: Vec<Column> = Vec::new();
            let stat_names: Column = Series::new(
                PlSmallStr::from("statistic"),
                &["count", "mean", "std", "min", "25%", "50%", "75%", "max"],
            ).into();
            cols_vec.push(stat_names);

            for series_col in df.get_columns() {
                let s = series_col.as_materialized_series();
                let count_val = format!("{}", s.len());
                if let Ok(f) = s.cast(&DataType::Float64) {
                    let ca = f.f64().unwrap();
                    let mut sorted: Vec<f64> = ca.into_no_null_iter().collect();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let n = sorted.len();
                    let mean = ca.mean().unwrap_or(0.0);
                    let std_val = if n > 0 {
                        (sorted.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64).sqrt()
                    } else { 0.0 };
                    let percentile = |p: f64| -> f64 {
                        if sorted.is_empty() { return 0.0; }
                        let idx = (p * (n as f64 - 1.0)) as usize;
                        sorted.get(idx).copied().unwrap_or(0.0)
                    };
                    let col_series: Column = Series::new(
                        PlSmallStr::from(s.name().as_str()),
                        &[
                            count_val.as_str(),
                            &format!("{:.4}", mean),
                            &format!("{:.4}", std_val),
                            &format!("{:.4}", sorted.first().copied().unwrap_or(0.0)),
                            &format!("{:.4}", percentile(0.25)),
                            &format!("{:.4}", percentile(0.50)),
                            &format!("{:.4}", percentile(0.75)),
                            &format!("{:.4}", sorted.last().copied().unwrap_or(0.0)),
                        ],
                    ).into();
                    cols_vec.push(col_series);
                } else {
                    let col_series: Column = Series::new(
                        PlSmallStr::from(s.name().as_str()),
                        &[count_val.as_str(), "", "", "", "", "", "", ""],
                    ).into();
                    cols_vec.push(col_series);
                }
            }
            if let Ok(desc_df) = DataFrame::new(cols_vec) {
                df_store_set(name, desc_df);
            }
            v_null()
        }
        "value_counts" => {
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let df = df_store_get(name);
            if let Ok(col) = df.column(&col_name) {
                let s = col.as_materialized_series();
                match s.value_counts(false, false, "count".into(), false) {
                    Ok(vc_df) => df_store_set(name, vc_df),
                    Err(e) => eprintln!("[ERROR] RDataFrame.value_counts: {}", e),
                }
            }
            v_null()
        }
        "nunique" => {
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let df = df_store_get(name);
            if let Ok(col) = df.column(&col_name) {
                let s = col.as_materialized_series();
                return v_int(s.n_unique().unwrap_or(0) as i64);
            }
            v_int(0)
        }
        "corr" | "correlation" => {
            // Correlation between two columns
            let col1 = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let col2 = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let df = df_store_get(name);
            let a = df.column(&col1).ok().and_then(|c| c.as_materialized_series().cast(&DataType::Float64).ok());
            let b = df.column(&col2).ok().and_then(|c| c.as_materialized_series().cast(&DataType::Float64).ok());
            if let (Some(a_f), Some(b_f)) = (a, b) {
                let ca = a_f.f64().unwrap();
                let cb = b_f.f64().unwrap();
                let va: Vec<f64> = ca.into_no_null_iter().collect();
                let vb: Vec<f64> = cb.into_no_null_iter().collect();
                let n = va.len().min(vb.len()) as f64;
                if n > 1.0 {
                    let ma = va.iter().sum::<f64>() / n;
                    let mb = vb.iter().sum::<f64>() / n;
                    let cov: f64 = va.iter().zip(vb.iter()).map(|(a, b)| (a - ma) * (b - mb)).sum::<f64>() / n;
                    let sa = (va.iter().map(|x| (x - ma).powi(2)).sum::<f64>() / n).sqrt();
                    let sb = (vb.iter().map(|x| (x - mb).powi(2)).sum::<f64>() / n).sqrt();
                    if sa > 0.0 && sb > 0.0 { return v_dbl(cov / (sa * sb)); }
                }
            }
            v_dbl(0.0)
        }

        // --- Sampling ---
        "sample" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(5) as usize;
            let df = df_store_get(name);
            match df.sample_n_literal(n, false, true, None) {
                Ok(sampled) => df_store_set(name, sampled),
                Err(e) => eprintln!("[ERROR] RDataFrame.sample: {}", e),
            }
            v_null()
        }
        "nlargest" => {
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let n = args.get(1).map(|v| v.to_i64()).unwrap_or(5) as usize;
            let df = df_store_get(name);
            match df.sort([&col_name], SortMultipleOptions::default().with_order_descending(true)) {
                Ok(sorted) => df_store_set(name, sorted.head(Some(n))),
                Err(e) => eprintln!("[ERROR] RDataFrame.nlargest: {}", e),
            }
            v_null()
        }
        "nsmallest" => {
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let n = args.get(1).map(|v| v.to_i64()).unwrap_or(5) as usize;
            let df = df_store_get(name);
            match df.sort([&col_name], SortMultipleOptions::default()) {
                Ok(sorted) => df_store_set(name, sorted.head(Some(n))),
                Err(e) => eprintln!("[ERROR] RDataFrame.nsmallest: {}", e),
            }
            v_null()
        }

        // --- Shape / info ---
        "info" => {
            let df = df_store_get(name);
            println!("DataFrame: {} rows x {} columns", df.height(), df.width());
            for col in df.get_columns() {
                let s = col.as_materialized_series();
                println!("  {}: {} ({})", s.name(), s.dtype(), s.len());
            }
            v_null()
        }
        "dtypes" => {
            let df = df_store_get(name);
            let types: Vec<String> = df.get_columns().iter()
                .map(|c| format!("{}: {}", c.name(), c.dtype())).collect();
            v_str(&types.join(","))
        }
        "shape" => {
            let df = df_store_get(name);
            v_str(&format!("({}, {})", df.height(), df.width()))
        }

        // --- Merge / Join ---
        "merge" | "join" => {
            let other_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let on_col = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let how = args.get(2).map(|v| v.to_string_val()).unwrap_or_else(|| "inner".to_string());
            let other_df = df_store_get(&other_name);
            let df = df_store_get(name);
            let join_type = match how.as_str() {
                "left" => JoinType::Left,
                "right" => JoinType::Right,
                "outer" | "full" => JoinType::Full,
                "cross" => JoinType::Cross,
                _ => JoinType::Inner,
            };
            match df.join(&other_df, [&on_col], [&on_col], JoinArgs::new(join_type), None) {
                Ok(joined) => df_store_set(name, joined),
                Err(e) => eprintln!("[ERROR] RDataFrame.merge: {}", e),
            }
            v_null()
        }
        "concat" | "append" => {
            let other_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let other_df = df_store_get(&other_name);
            let df = df_store_get(name);
            match df.vstack(&other_df) {
                Ok(combined) => df_store_set(name, combined),
                Err(e) => eprintln!("[ERROR] RDataFrame.concat: {}", e),
            }
            v_null()
        }

        // --- Transform ---
        "transpose" | "t" => {
            let mut df = df_store_get(name);
            match df.transpose(None, None) {
                Ok(transposed) => df_store_set(name, transposed),
                Err(e) => eprintln!("[ERROR] RDataFrame.transpose: {}", e),
            }
            v_null()
        }
        "apply" => {
            // Simple column transformation: apply "column", "operation"
            // Supported operations: upper, lower, abs, round, sqrt, log
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let op = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let df = df_store_get(name);
            let lazy = df.lazy();
            let col_expr = polars::lazy::dsl::col(&col_name);
            let transformed = match op.as_str() {
                "upper" => lazy.with_columns([col_expr.cast(DataType::String).str().to_uppercase()]),
                "lower" => lazy.with_columns([col_expr.cast(DataType::String).str().to_lowercase()]),
                "abs" => lazy.with_columns([col_expr.abs()]),
                "round" => lazy.with_columns([col_expr.round(2)]),
                "sqrt" => lazy.with_columns([col_expr.sqrt()]),
                "log" => lazy.with_columns([col_expr.log(std::f64::consts::E)]),
                _ => { eprintln!("[WARN] RDataFrame.apply: unknown op '{}'", op); return v_null(); }
            };
            match transformed.collect() {
                Ok(result) => df_store_set(name, result),
                Err(e) => eprintln!("[ERROR] RDataFrame.apply: {}", e),
            }
            v_null()
        }
        "replace" => {
            // replace "column", "old_value", "new_value"
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let old_val = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let new_val = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let df = df_store_get(name);
            let lazy = df.lazy();
            let col_expr = polars::lazy::dsl::col(&col_name).cast(DataType::String);
            let replaced = lazy.with_columns([
                polars::lazy::dsl::when(col_expr.clone().eq(lit(old_val.clone())))
                    .then(lit(new_val.clone()))
                    .otherwise(col_expr)
                    .alias(&col_name)
            ]);
            match replaced.collect() {
                Ok(result) => df_store_set(name, result),
                Err(e) => eprintln!("[ERROR] RDataFrame.replace: {}", e),
            }
            v_null()
        }

        // --- Display / Info ---
        "clear" => { df_store_set(name, DataFrame::empty()); v_null() }
        "columns" => {
            let df = df_store_get(name);
            let cols: Vec<&str> = df.get_column_names().into_iter().map(|c| c.as_str()).collect();
            v_str(&cols.join(","))
        }
        "rows" | "rowcount" | "len" => { v_int(df_store_get(name).height() as i64) }
        "tostring" | "show" | "print" => {
            let df = df_store_get(name);
            let s = format!("{}", df);
            println!("{}", s);
            v_str(&s)
        }

        // --- Populate RStringGrid ---
        "togrid" | "to_grid" | "display" => {
            // Send DataFrame contents to a RStringGrid component
            #[cfg(feature = "gui")]
            {
                let grid_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                if !grid_name.is_empty() {
                    let df = df_store_get(name);
                    populate_grid_from_df(&grid_name, &df);
                }
            }
            v_null()
        }

        _ => {
            eprintln!("[WARN] RDataFrame.{}() not implemented", method);
            v_null()
        }
    }
}

/// Populate a RStringGrid from a DataFrame.
#[cfg(feature = "gui")]
fn populate_grid_from_df(grid_name: &str, df: &DataFrame) {
    use crate::object::{rp_comp_set, rp_comp_method};
    // Set column count
    let ncols = df.width();
    let nrows = df.height();
    rp_comp_set(grid_name, "cols", v_int(ncols as i64));

    // Clear existing data
    rp_comp_method(grid_name, "clear", &[]);

    // Set column headers (row 0)
    for (ci, col) in df.get_columns().iter().enumerate() {
        crate::gui::string_grid_method(grid_name, "setcell", &[v_int(0), v_int(ci as i64), v_str(col.name().as_str())]);
    }

    // Set data rows
    for ri in 0..nrows {
        // Add a row
        rp_comp_method(grid_name, "addrow", &[]);
        for (ci, col) in df.get_columns().iter().enumerate() {
            let s = col.as_materialized_series();
            let val_str = match s.get(ri) {
                Ok(av) => format!("{}", av),
                Err(_) => String::new(),
            };
            crate::gui::string_grid_method(grid_name, "setcell", &[
                v_int((ri + 1) as i64), v_int(ci as i64), v_str(&val_str),
            ]);
        }
    }
}

/// Get a RDataFrame property.
pub fn dataframe_get_prop(name: &str, prop: &str) -> Value {
    let df = df_store_get(name);
    match prop {
        "rowcount" | "height" | "nrows" => v_int(df.height() as i64),
        "colcount" | "width" | "ncols" => v_int(df.width() as i64),
        "columns" => {
            let cols: Vec<&str> = df.get_column_names().into_iter().map(|c| c.as_str()).collect();
            v_str(&cols.join(","))
        }
        "shape" => v_str(&format!("({}, {})", df.height(), df.width())),
        "empty" => v_int(if df.is_empty() { 1 } else { 0 }),
        _ => v_null(),
    }
}

// ---------------------------------------------------------------------------
// RPlot — backed by plotters (With some Matplotlib-compatibility)
// ---------------------------------------------------------------------------

use plotters::prelude::*;

#[derive(Clone, Debug)]
struct PlotSeries {
    x_data: Vec<f64>,
    y_data: Vec<f64>,
    label: String,
    color: String,
    style: String, // "-" line, "--" dashed, "o" scatter, "bar", "hist", "pie", "step", "area"
}

#[derive(Clone, Debug)]
struct PlotAnnotation {
    text: String,
    x: f64,
    y: f64,
    color: String,
}

#[derive(Clone, Debug)]
struct PlotState {
    title: String,
    xlabel: String,
    ylabel: String,
    grid: bool,
    width: u32,
    height: u32,
    dpi: u32,
    series: Vec<PlotSeries>,
    annotations: Vec<PlotAnnotation>,
    show_legend: bool,
    xlim: Option<(f64, f64)>,
    ylim: Option<(f64, f64)>,
    xscale: String, // "linear" or "log"
    yscale: String,
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
            dpi: 100,
            series: Vec::new(),
            annotations: Vec::new(),
            show_legend: false,
            xlim: None,
            ylim: None,
            xscale: "linear".to_string(),
            yscale: "linear".to_string(),
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
        "lightblue" => RGBColor(173, 216, 230),
        "lightgreen" => RGBColor(144, 238, 144),
        "darkred" => RGBColor(139, 0, 0),
        "darkblue" => RGBColor(0, 0, 139),
        "darkgreen" => RGBColor(0, 100, 0),
        "brown" => RGBColor(139, 69, 19),
        "pink" => RGBColor(255, 192, 203),
        "gold" => RGBColor(255, 215, 0),
        "navy" => RGBColor(0, 0, 128),
        "teal" => RGBColor(0, 128, 128),
        "coral" => RGBColor(255, 127, 80),
        "salmon" => RGBColor(250, 128, 114),
        "olive" => RGBColor(128, 128, 0),
        "maroon" => RGBColor(128, 0, 0),
        "lime" => RGBColor(0, 255, 0),
        "indigo" => RGBColor(75, 0, 130),
        "violet" => RGBColor(238, 130, 238),
        "silver" => RGBColor(192, 192, 192),
        "tomato" => RGBColor(255, 99, 71),
        s if s.starts_with('#') && s.len() == 7 => {
            let r = u8::from_str_radix(&s[1..3], 16).unwrap_or(0);
            let g = u8::from_str_radix(&s[3..5], 16).unwrap_or(0);
            let b = u8::from_str_radix(&s[5..7], 16).unwrap_or(0);
            RGBColor(r, g, b)
        }
        _ => RGBColor(0, 0, 0),
    }
}

/// Auto-pick a color for series index
fn auto_color(idx: usize) -> &'static str {
    const PALETTE: &[&str] = &[
        "blue", "red", "green", "orange", "purple", "cyan", "magenta",
        "steelblue", "brown", "pink", "teal", "gold", "navy", "coral",
    ];
    PALETTE[idx % PALETTE.len()]
}

/// Dispatch method calls on RPlot components.
pub fn plot_method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "clear" => {
            plot_set(name, PlotState::default());
            v_null()
        }
        "plot" => {
            let x_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let state = plot_get(name);
            let color = args.get(3).map(|v| v.to_string_val())
                .unwrap_or_else(|| auto_color(state.series.len()).to_string());
            let style = args.get(4).map(|v| v.to_string_val()).unwrap_or_else(|| "-".to_string());

            let x_data = get_num_data(&x_name);
            let y_data = get_num_data(&y_name);

            let mut state = state;
            state.series.push(PlotSeries { x_data, y_data, label, color, style });
            plot_set(name, state);
            v_null()
        }
        "bar" => {
            let x_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let state = plot_get(name);
            let color = args.get(3).map(|v| v.to_string_val())
                .unwrap_or_else(|| auto_color(state.series.len()).to_string());

            let x_data = get_num_data(&x_name);
            let y_data = get_num_data(&y_name);

            let mut state = state;
            state.series.push(PlotSeries { x_data, y_data, label, color, style: "bar".to_string() });
            plot_set(name, state);
            v_null()
        }
        "barh" => {
            // Horizontal bar chart (swap x/y at render time)
            let x_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let state = plot_get(name);
            let color = args.get(3).map(|v| v.to_string_val())
                .unwrap_or_else(|| auto_color(state.series.len()).to_string());
            let x_data = get_num_data(&x_name);
            let y_data = get_num_data(&y_name);
            let mut state = state;
            state.series.push(PlotSeries { x_data: y_data, y_data: x_data, label, color, style: "barh".to_string() });
            plot_set(name, state);
            v_null()
        }
        "scatter" => {
            let x_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let state = plot_get(name);
            let color = args.get(3).map(|v| v.to_string_val())
                .unwrap_or_else(|| auto_color(state.series.len()).to_string());
            let x_data = get_num_data(&x_name);
            let y_data = get_num_data(&y_name);
            let mut state = state;
            state.series.push(PlotSeries { x_data, y_data, label, color, style: "o".to_string() });
            plot_set(name, state);
            v_null()
        }
        "step" => {
            let x_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let state = plot_get(name);
            let color = args.get(3).map(|v| v.to_string_val())
                .unwrap_or_else(|| auto_color(state.series.len()).to_string());
            let x_data = get_num_data(&x_name);
            let y_data = get_num_data(&y_name);
            let mut state = state;
            state.series.push(PlotSeries { x_data, y_data, label, color, style: "step".to_string() });
            plot_set(name, state);
            v_null()
        }
        "area" | "fill_between" => {
            let x_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let state = plot_get(name);
            let color = args.get(3).map(|v| v.to_string_val())
                .unwrap_or_else(|| auto_color(state.series.len()).to_string());
            let x_data = get_num_data(&x_name);
            let y_data = get_num_data(&y_name);
            let mut state = state;
            state.series.push(PlotSeries { x_data, y_data, label, color, style: "area".to_string() });
            plot_set(name, state);
            v_null()
        }
        "hist" | "histogram" => {
            // hist dataArray, bins, label, color
            let data_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let bins = args.get(1).map(|v| v.to_i64()).unwrap_or(10) as usize;
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let state = plot_get(name);
            let color = args.get(3).map(|v| v.to_string_val())
                .unwrap_or_else(|| auto_color(state.series.len()).to_string());

            let raw_data = get_num_data(&data_name);
            if raw_data.is_empty() { return v_null(); }

            // Compute histogram bins
            let mn = raw_data.iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = raw_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let bins = bins.max(1);
            let bin_width = if mx > mn { (mx - mn) / bins as f64 } else { 1.0 };
            let mut counts = vec![0.0f64; bins];
            for &v in &raw_data {
                let idx = ((v - mn) / bin_width) as usize;
                let idx = idx.min(bins - 1);
                counts[idx] += 1.0;
            }
            let x_data: Vec<f64> = (0..bins).map(|i| mn + (i as f64 + 0.5) * bin_width).collect();

            let mut state = state;
            state.series.push(PlotSeries { x_data, y_data: counts, label, color, style: "bar".to_string() });
            plot_set(name, state);
            v_null()
        }
        "pie" => {
            // pie valuesArray, labelsStr, colorsStr
            let data_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let color = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let data = get_num_data(&data_name);
            let mut state = plot_get(name);
            state.series.push(PlotSeries { x_data: data.clone(), y_data: data, label, color, style: "pie".to_string() });
            plot_set(name, state);
            v_null()
        }
        "hline" | "axhline" => {
            let y = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let color = args.get(1).map(|v| v.to_string_val()).unwrap_or_else(|| "black".to_string());
            let mut state = plot_get(name);
            state.series.push(PlotSeries { x_data: vec![], y_data: vec![y], label: String::new(), color, style: "hline".to_string() });
            plot_set(name, state);
            v_null()
        }
        "vline" | "axvline" => {
            let x = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let color = args.get(1).map(|v| v.to_string_val()).unwrap_or_else(|| "black".to_string());
            let mut state = plot_get(name);
            state.series.push(PlotSeries { x_data: vec![x], y_data: vec![], label: String::new(), color, style: "vline".to_string() });
            plot_set(name, state);
            v_null()
        }
        "annotate" => {
            let text = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let x = args.get(1).map(|v| v.to_f64()).unwrap_or(0.0);
            let y = args.get(2).map(|v| v.to_f64()).unwrap_or(0.0);
            let color = args.get(3).map(|v| v.to_string_val()).unwrap_or_else(|| "black".to_string());
            let mut state = plot_get(name);
            state.annotations.push(PlotAnnotation { text, x, y, color });
            plot_set(name, state);
            v_null()
        }
        "legend" => {
            let mut state = plot_get(name);
            state.show_legend = true;
            plot_set(name, state);
            v_null()
        }
        "savefig" | "save" => {
            let filename = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| "plot.png".to_string());
            render_plot(name, &filename);
            v_null()
        }
        "figsize" => {
            let w = args.first().map(|v| v.to_f64()).unwrap_or(6.4);
            let h = args.get(1).map(|v| v.to_f64()).unwrap_or(4.8);
            let mut state = plot_get(name);
            state.width = (w * state.dpi as f64) as u32;
            state.height = (h * state.dpi as f64) as u32;
            plot_set(name, state);
            v_null()
        }
        "xlim" => {
            let lo = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let hi = args.get(1).map(|v| v.to_f64()).unwrap_or(1.0);
            let mut state = plot_get(name);
            state.xlim = Some((lo, hi));
            plot_set(name, state);
            v_null()
        }
        "ylim" => {
            let lo = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let hi = args.get(1).map(|v| v.to_f64()).unwrap_or(1.0);
            let mut state = plot_get(name);
            state.ylim = Some((lo, hi));
            plot_set(name, state);
            v_null()
        }
        "xscale" => {
            let scale = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| "linear".to_string());
            let mut state = plot_get(name);
            state.xscale = scale;
            plot_set(name, state);
            v_null()
        }
        "yscale" => {
            let scale = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| "linear".to_string());
            let mut state = plot_get(name);
            state.yscale = scale;
            plot_set(name, state);
            v_null()
        }
        _ => {
            eprintln!("[WARN] RPlot.{}() not implemented", method);
            v_null()
        }
    }
}

/// Helper to get numeric array data from NUMPY_ARRAYS by name.
fn get_num_data(arr_name: &str) -> Vec<f64> {
    NUMPY_ARRAYS.with(|m| {
        m.borrow().get(&arr_name.to_lowercase()).map(|a| a.to_vec()).unwrap_or_default()
    })
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
        "dpi" => v_int(state.dpi as i64),
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
        "dpi" => state.dpi = val.to_i64().max(50) as u32,
        _ => {}
    }
    plot_set(name, state);
}

/// Render the accumulated plot state to a PNG file.
fn render_plot(name: &str, filename: &str) {
    let state = plot_get(name);
    // If width/height look like matplotlib-style inches (< 100), convert to pixels at DPI
    let dpi = state.dpi.max(50);
    let w = if state.width < 100 { state.width * dpi } else { state.width };
    let h = if state.height < 100 { state.height * dpi } else { state.height };

    // Check for pie chart (special rendering)
    if state.series.iter().any(|s| s.style == "pie") {
        render_pie_chart(&state, filename, w, h);
        return;
    }

    let root = BitMapBackend::new(filename, (w, h)).into_drawing_area();
    if root.fill(&WHITE).is_err() { return; }

    // Compute data bounds
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = 0.0f64; // Include 0 for bar charts
    let mut y_max = f64::NEG_INFINITY;
    let has_bars = state.series.iter().any(|s| s.style == "bar" || s.style == "barh");

    for s in &state.series {
        if s.style == "hline" || s.style == "vline" { continue; }
        for &x in &s.x_data {
            if x < x_min { x_min = x; }
            if x > x_max { x_max = x; }
        }
        for &y in &s.y_data {
            if y < y_min { y_min = y; }
            if y > y_max { y_max = y; }
        }
    }
    // Handle hlines/vlines
    for s in &state.series {
        if s.style == "hline" { for &y in &s.y_data { if y < y_min { y_min = y; } if y > y_max { y_max = y; } } }
        if s.style == "vline" { for &x in &s.x_data { if x < x_min { x_min = x; } if x > x_max { x_max = x; } } }
    }

    if x_min >= x_max { x_min = 0.0; x_max = 1.0; }
    if y_min >= y_max { y_min = 0.0; y_max = 1.0; }

    // Override with user-defined limits
    if let Some((lo, hi)) = state.xlim { x_min = lo; x_max = hi; }
    if let Some((lo, hi)) = state.ylim { y_min = lo; y_max = hi; }

    // Add margin
    let x_margin = (x_max - x_min) * 0.05;
    let y_margin = (y_max - y_min) * 0.05;
    if state.xlim.is_none() { x_min -= x_margin; x_max += x_margin; }
    if state.ylim.is_none() { y_min -= y_margin; y_max += y_margin; }
    // For bar charts, ensure y starts at 0
    if has_bars && state.ylim.is_none() && y_min > 0.0 { y_min = 0.0; }

    let mut chart_builder = ChartBuilder::on(&root);
    if !state.title.is_empty() {
        chart_builder.caption(&state.title, ("sans-serif", 20));
    }
    chart_builder.margin(10).x_label_area_size(35).y_label_area_size(45);

    let chart_result = chart_builder.build_cartesian_2d(
        x_min as f32..x_max as f32,
        y_min as f32..y_max as f32,
    );
    let mut chart = match chart_result {
        Ok(c) => c,
        Err(e) => { eprintln!("[ERROR] RPlot chart build: {}", e); return; }
    };

    let mut mesh = chart.configure_mesh();
    if !state.xlabel.is_empty() { mesh.x_desc(&state.xlabel); }
    if !state.ylabel.is_empty() { mesh.y_desc(&state.ylabel); }
    if state.grid { let _ = mesh.draw(); } else { mesh.disable_mesh(); let _ = mesh.draw(); }

    for s in &state.series {
        let color = parse_color(&s.color);
        let points: Vec<(f32, f32)> = s.x_data.iter().zip(s.y_data.iter())
            .map(|(&x, &y)| (x as f32, y as f32))
            .collect();

        match s.style.as_str() {
            "-" | "line" => {
                let line = LineSeries::new(points.clone(), &color);
                if !s.label.is_empty() {
                    let _ = chart.draw_series(line).map(|a| {
                        a.label(&s.label).legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color))
                    });
                } else { let _ = chart.draw_series(line); }
            }
            "--" | "dashed" => {
                let line = LineSeries::new(points.clone(), color.stroke_width(2));
                if !s.label.is_empty() {
                    let _ = chart.draw_series(line).map(|a| {
                        a.label(&s.label).legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color))
                    });
                } else { let _ = chart.draw_series(line); }
            }
            "o" | "scatter" => {
                if !s.label.is_empty() {
                    let _ = chart.draw_series(points.iter().map(|&(x, y)| Circle::new((x, y), 3, color.filled())))
                        .map(|a| a.label(&s.label).legend(move |(x, y)| Circle::new((x, y), 3, color.filled())));
                } else {
                    let _ = chart.draw_series(points.iter().map(|&(x, y)| Circle::new((x, y), 3, color.filled())));
                }
            }
            "bar" => {
                let bar_width = if points.len() > 1 {
                    (points[1].0 - points[0].0).abs() * 0.8
                } else {
                    0.8f32
                };
                if !s.label.is_empty() {
                    let _ = chart.draw_series(points.iter().map(|&(x, y)| {
                        Rectangle::new([(x - bar_width / 2.0, 0.0f32), (x + bar_width / 2.0, y)], color.filled())
                    })).map(|a| a.label(&s.label).legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 20, y + 5)], color.filled())));
                } else {
                    let _ = chart.draw_series(points.iter().map(|&(x, y)| {
                        Rectangle::new([(x - bar_width / 2.0, 0.0f32), (x + bar_width / 2.0, y)], color.filled())
                    }));
                }
            }
            "barh" => {
                let bar_width = if points.len() > 1 {
                    (points[1].1 - points[0].1).abs() * 0.8
                } else {
                    0.8f32
                };
                let _ = chart.draw_series(points.iter().map(|&(x, y)| {
                    Rectangle::new([(0.0f32, y - bar_width / 2.0), (x, y + bar_width / 2.0)], color.filled())
                }));
            }
            "step" => {
                // Step plot: horizontal-first step interpolation
                if points.len() >= 2 {
                    let mut step_pts = Vec::with_capacity(points.len() * 2);
                    for i in 0..points.len() {
                        step_pts.push(points[i]);
                        if i + 1 < points.len() {
                            step_pts.push((points[i + 1].0, points[i].1));
                        }
                    }
                    let _ = chart.draw_series(LineSeries::new(step_pts, &color));
                }
            }
            "area" => {
                let _ = chart.draw_series(AreaSeries::new(points.clone(), 0.0f32, color.mix(0.3)));
                let _ = chart.draw_series(LineSeries::new(points, &color));
            }
            "hline" => {
                if let Some(&y) = s.y_data.first() {
                    let _ = chart.draw_series(LineSeries::new(
                        vec![(x_min as f32, y as f32), (x_max as f32, y as f32)], &color
                    ));
                }
            }
            "vline" => {
                if let Some(&x) = s.x_data.first() {
                    let _ = chart.draw_series(LineSeries::new(
                        vec![(x as f32, y_min as f32), (x as f32, y_max as f32)], &color
                    ));
                }
            }
            _ => {
                let _ = chart.draw_series(LineSeries::new(points, &color));
            }
        }
    }

    // Annotations
    for ann in &state.annotations {
        let color = parse_color(&ann.color);
        let _ = chart.draw_series(std::iter::once(
            plotters::element::Text::new(ann.text.clone(), (ann.x as f32, ann.y as f32), ("sans-serif", 14).into_font().color(&color))
        ));
    }

    if state.show_legend {
        let _ = chart.configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw();
    }

    let _ = root.present();
}

/// Render pie chart (special case - no cartesian axes)
fn render_pie_chart(state: &PlotState, filename: &str, w: u32, h: u32) {
    let root = BitMapBackend::new(filename, (w, h)).into_drawing_area();
    if root.fill(&WHITE).is_err() { return; }

    if !state.title.is_empty() {
        let _ = root.titled(&state.title, ("sans-serif", 20));
    }

    let pie_data: Vec<f64> = state.series.iter()
        .flat_map(|s| s.x_data.iter().cloned()).collect();
    if pie_data.is_empty() { return; }

    let total: f64 = pie_data.iter().sum();
    if total <= 0.0 { return; }

    let labels_str = state.series.first().map(|s| &s.label).cloned().unwrap_or_default();
    let labels: Vec<&str> = if labels_str.is_empty() {
        (0..pie_data.len()).map(|i| Box::leak(format!("Slice {}", i + 1).into_boxed_str()) as &str).collect()
    } else {
        labels_str.split(',').map(|s| s.trim()).collect()
    };
    let colors_str = state.series.first().map(|s| &s.color).cloned().unwrap_or_default();
    let color_names: Vec<&str> = if colors_str.is_empty() {
        vec!["blue", "red", "green", "orange", "purple", "cyan", "magenta", "steelblue", "brown", "pink"]
    } else {
        colors_str.split(',').map(|s| s.trim()).collect()
    };

    // Draw pie slices manually
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let radius = (w.min(h) as f64 * 0.35).max(50.0);
    let mut start_angle = 0.0f64;

    for (i, &value) in pie_data.iter().enumerate() {
        let sweep = value / total * 2.0 * std::f64::consts::PI;
        let color = parse_color(color_names.get(i).copied().unwrap_or("gray"));
        let label = labels.get(i).copied().unwrap_or("");
        let pct = value / total * 100.0;

        // Draw filled arc as a polygon
        let mut polygon_pts: Vec<(i32, i32)> = vec![(cx as i32, cy as i32)];
        let steps = (sweep * 50.0).max(3.0) as usize;
        for s in 0..=steps {
            let angle = start_angle + sweep * s as f64 / steps as f64;
            let px = cx + radius * angle.cos();
            let py = cy + radius * angle.sin();
            polygon_pts.push((px as i32, py as i32));
        }
        let _ = root.draw(&plotters::element::Polygon::new(polygon_pts, color.filled()));

        // Draw label at midpoint
        let mid_angle = start_angle + sweep / 2.0;
        let lx = cx + radius * 0.7 * mid_angle.cos();
        let ly = cy + radius * 0.7 * mid_angle.sin();
        let text = format!("{} ({:.1}%)", label, pct);
        let _ = root.draw(&plotters::element::Text::new(
            text, (lx as i32, ly as i32),
            ("sans-serif", 11).into_font().color(&BLACK),
        ));

        start_angle += sweep;
    }

    let _ = root.present();
}

/// Render plot to a temp file and return the path.
pub fn plot_render_to_file(name: &str) -> String {
    let filename = format!("/tmp/rapidr_plot_{}.png", name.to_lowercase());
    render_plot(name, &filename);
    filename
}
