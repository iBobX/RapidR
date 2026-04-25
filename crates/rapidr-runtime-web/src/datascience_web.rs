//! Data-science components for the web runtime: RNum, RDataFrame, RPlot.
//!
//! Pure Rust + web-sys implementations — no polars, no ndarray, no plotters.
//! RNum   → Vec<f64>
//! RDataFrame → column-oriented Vec<Vec<String>>
//! RPlot  → HTML5 Canvas via web-sys

use crate::gui_web;
use crate::object_web;
use crate::value::{v_dbl, v_int, v_null, v_str, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

// ======================================================================
// Storage
// ======================================================================

thread_local! {
    static NUM_STORE: RefCell<HashMap<String, Vec<f64>>> = RefCell::new(HashMap::new());
    static DF_STORE: RefCell<HashMap<String, DataFrame>> = RefCell::new(HashMap::new());
    static PLOT_STORE: RefCell<HashMap<String, PlotState>> = RefCell::new(HashMap::new());
}

// ======================================================================
// RNum
// ======================================================================

pub fn num_method(name: &str, method: &str, args: &[Value]) -> Value {
    let uname = name.to_uppercase();
    match method {
        // --- Creation ---
        "create" | "new" | "init" => {
            // Create a vector of given length, zero-filled. With no arg, create empty.
            let n = args.first().map(|v| v.to_i64()).unwrap_or(0).max(0) as usize;
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, vec![0.0; n]));
            v_null()
        }
        "set" | "setvalue" | "setitem" => {
            // set(index, value)
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(0).max(0) as usize;
            let val = args.get(1).map(|v| v.to_f64()).unwrap_or(0.0);
            NUM_STORE.with(|s| {
                let mut store = s.borrow_mut();
                let arr = store.entry(uname.clone()).or_insert_with(Vec::new);
                if arr.len() <= idx { arr.resize(idx + 1, 0.0); }
                arr[idx] = val;
            });
            v_null()
        }
        "get" | "getvalue" | "getitem" | "at" => {
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(0).max(0) as usize;
            with_num(&uname, |a| v_dbl(a.get(idx).copied().unwrap_or(0.0)))
        }
        "push" => {
            let val = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            NUM_STORE.with(|s| {
                let mut store = s.borrow_mut();
                store.entry(uname.clone()).or_insert_with(Vec::new).push(val);
            });
            v_null()
        }
        "arange" => {
            let start = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let stop = args.get(1).map(|v| v.to_f64()).unwrap_or(10.0);
            let step = args.get(2).map(|v| v.to_f64()).unwrap_or(1.0);
            if step == 0.0 { return v_null(); }
            let mut arr = Vec::new();
            let mut v = start;
            if step > 0.0 {
                while v < stop { arr.push(v); v += step; }
            } else {
                while v > stop { arr.push(v); v += step; }
            }
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, arr));
            v_null()
        }
        "linspace" => {
            let start = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let stop = args.get(1).map(|v| v.to_f64()).unwrap_or(1.0);
            let n = args.get(2).map(|v| v.to_i64()).unwrap_or(50) as usize;
            let arr: Vec<f64> = if n <= 1 {
                vec![start]
            } else {
                (0..n).map(|i| start + (stop - start) * (i as f64) / ((n - 1) as f64)).collect()
            };
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, arr));
            v_null()
        }
        "zeros" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(10) as usize;
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, vec![0.0; n]));
            v_null()
        }
        "ones" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(10) as usize;
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, vec![1.0; n]));
            v_null()
        }
        "full" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(10) as usize;
            let fill = args.get(1).map(|v| v.to_f64()).unwrap_or(0.0);
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, vec![fill; n]));
            v_null()
        }
        "fromlist" | "from_list" => {
            let text = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let arr: Vec<f64> = text.split(',')
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .collect();
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, arr));
            v_null()
        }
        // --- Aggregation ---
        "sum" => with_num(&uname, |a| v_dbl(a.iter().sum())),
        "mean" => with_num(&uname, |a| {
            if a.is_empty() { v_dbl(0.0) } else { v_dbl(a.iter().sum::<f64>() / a.len() as f64) }
        }),
        "min" => with_num(&uname, |a| v_dbl(a.iter().cloned().fold(f64::INFINITY, f64::min))),
        "max" => with_num(&uname, |a| v_dbl(a.iter().cloned().fold(f64::NEG_INFINITY, f64::max))),
        "std" => with_num(&uname, |a| {
            if a.is_empty() { return v_dbl(0.0); }
            let m = a.iter().sum::<f64>() / a.len() as f64;
            let var = a.iter().map(|x| (x - m).powi(2)).sum::<f64>() / a.len() as f64;
            v_dbl(var.sqrt())
        }),
        "var" | "variance" => with_num(&uname, |a| {
            if a.is_empty() { return v_dbl(0.0); }
            let m = a.iter().sum::<f64>() / a.len() as f64;
            v_dbl(a.iter().map(|x| (x - m).powi(2)).sum::<f64>() / a.len() as f64)
        }),
        "median" => with_num(&uname, |a| {
            if a.is_empty() { return v_dbl(0.0); }
            let mut s = a.to_vec();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = s.len() / 2;
            if s.len() % 2 == 0 { v_dbl((s[mid - 1] + s[mid]) / 2.0) } else { v_dbl(s[mid]) }
        }),
        "argmin" => with_num(&uname, |a| {
            v_int(a.iter().enumerate().min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).map(|(i, _)| i as i64).unwrap_or(0))
        }),
        "argmax" => with_num(&uname, |a| {
            v_int(a.iter().enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).map(|(i, _)| i as i64).unwrap_or(0))
        }),
        "count" => with_num(&uname, |a| v_int(a.len() as i64)),
        "ptp" => with_num(&uname, |a| {
            let mn = a.iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = a.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            v_dbl(mx - mn)
        }),
        // --- Element-wise math ---
        "sin" => num_map(&uname, |x| x.sin()),
        "cos" => num_map(&uname, |x| x.cos()),
        "tan" => num_map(&uname, |x| x.tan()),
        "asin" | "arcsin" => num_map(&uname, |x| x.asin()),
        "acos" | "arccos" => num_map(&uname, |x| x.acos()),
        "atan" | "arctan" => num_map(&uname, |x| x.atan()),
        "sqrt" => num_map(&uname, |x| x.sqrt()),
        "abs" => num_map(&uname, |x| x.abs()),
        "exp" => num_map(&uname, |x| x.exp()),
        "log" | "ln" => num_map(&uname, |x| x.ln()),
        "log2" => num_map(&uname, |x| x.log2()),
        "log10" => num_map(&uname, |x| x.log10()),
        "floor" => num_map(&uname, |x| x.floor()),
        "ceil" => num_map(&uname, |x| x.ceil()),
        "round" => {
            let decimals = args.first().map(|v| v.to_i64()).unwrap_or(0);
            let factor = 10f64.powi(decimals as i32);
            num_map(&uname, move |x| (x * factor).round() / factor)
        }
        "sign" => num_map(&uname, |x| if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 }),
        "reciprocal" => num_map(&uname, |x| if x != 0.0 { 1.0 / x } else { f64::NAN }),
        "square" => num_map(&uname, |x| x * x),
        "negative" | "neg" => num_map(&uname, |x| -x),
        // --- Arithmetic with scalar or other array ---
        "add" => num_arith(&uname, args, |a, b| a + b),
        "subtract" | "sub" => num_arith(&uname, args, |a, b| a - b),
        "multiply" | "mul" => num_arith(&uname, args, |a, b| a * b),
        "divide" | "div" => num_arith(&uname, args, |a, b| if b != 0.0 { a / b } else { f64::NAN }),
        "power" | "pow" => num_arith(&uname, args, |a, b| a.powf(b)),
        "mod" | "fmod" => num_arith(&uname, args, |a, b| if b != 0.0 { a % b } else { f64::NAN }),
        "clip" | "clamp" => {
            let lo = args.first().map(|v| v.to_f64()).unwrap_or(f64::NEG_INFINITY);
            let hi = args.get(1).map(|v| v.to_f64()).unwrap_or(f64::INFINITY);
            num_map(&uname, move |x| x.max(lo).min(hi))
        }
        // --- Ordering ---
        "sort" => {
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(&uname) {
                    a.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                }
            });
            v_null()
        }
        "reverse" | "flip" => {
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(&uname) {
                    a.reverse();
                }
            });
            v_null()
        }
        "unique" => {
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(&uname) {
                    a.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    a.dedup();
                }
            });
            v_null()
        }
        "shuffle" => {
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(&uname) {
                    // Simple Fisher-Yates using js_sys::Math::random
                    for i in (1..a.len()).rev() {
                        let j = (js_sys::Math::random() * (i + 1) as f64) as usize;
                        a.swap(i, j);
                    }
                }
            });
            v_null()
        }
        "append" | "concatenate" => {
            if let Some(other_name) = args.first() {
                let other_uname = other_name.to_string_val().to_uppercase();
                let other = NUM_STORE.with(|s| s.borrow().get(&other_uname).cloned());
                if let Some(other_arr) = other {
                    NUM_STORE.with(|s| {
                        if let Some(a) = s.borrow_mut().get_mut(&uname) {
                            a.extend(other_arr);
                        }
                    });
                }
            }
            v_null()
        }
        "slice" => {
            let start = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            let end = args.get(1).map(|v| v.to_i64() as usize).unwrap_or(usize::MAX);
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(&uname) {
                    let e = end.min(a.len());
                    let st = start.min(e);
                    *a = a[st..e].to_vec();
                }
            });
            v_null()
        }
        // --- Cumulative ---
        "cumsum" => {
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(&uname) {
                    let mut sum = 0.0;
                    for v in a.iter_mut() { sum += *v; *v = sum; }
                }
            });
            v_null()
        }
        "cumprod" => {
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(&uname) {
                    let mut prod = 1.0;
                    for v in a.iter_mut() { prod *= *v; *v = prod; }
                }
            });
            v_null()
        }
        "diff" => {
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(&uname) {
                    if a.len() > 1 {
                        let mut d = Vec::with_capacity(a.len() - 1);
                        for i in 1..a.len() { d.push(a[i] - a[i - 1]); }
                        *a = d;
                    }
                }
            });
            v_null()
        }
        // --- Linear algebra ---
        "dot" => {
            if let Some(other_name) = args.first() {
                let other_uname = other_name.to_string_val().to_uppercase();
                let other = NUM_STORE.with(|s| s.borrow().get(&other_uname).cloned());
                let this = NUM_STORE.with(|s| s.borrow().get(&uname).cloned());
                if let (Some(a), Some(b)) = (this, other) {
                    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
                    return v_dbl(dot);
                }
            }
            v_dbl(0.0)
        }
        "norm" => with_num(&uname, |a| v_dbl(a.iter().map(|x| x * x).sum::<f64>().sqrt())),
        "normalize" => {
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(&uname) {
                    let n = a.iter().map(|x| x * x).sum::<f64>().sqrt();
                    if n > 0.0 { for v in a.iter_mut() { *v /= n; } }
                }
            });
            v_null()
        }
        // --- Boolean/search ---
        "any" => with_num(&uname, |a| v_int(if a.iter().any(|x| *x != 0.0) { 1 } else { 0 })),
        "all" => with_num(&uname, |a| v_int(if a.iter().all(|x| *x != 0.0) { 1 } else { 0 })),
        "where" | "nonzero" => {
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(&uname) {
                    let indices: Vec<f64> = a.iter().enumerate()
                        .filter(|(_, x)| **x != 0.0)
                        .map(|(i, _)| i as f64)
                        .collect();
                    *a = indices;
                }
            });
            v_null()
        }
        "searchsorted" => {
            let val = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            with_num(&uname, |a| {
                let pos = a.partition_point(|x| *x < val);
                v_int(pos as i64)
            })
        }
        // --- Random ---
        "rand" | "random" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(10) as usize;
            let arr: Vec<f64> = (0..n).map(|_| js_sys::Math::random()).collect();
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, arr));
            v_null()
        }
        "randn" | "random_normal" | "normal" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(10) as usize;
            let mean = args.get(1).map(|v| v.to_f64()).unwrap_or(0.0);
            let std = args.get(2).map(|v| v.to_f64()).unwrap_or(1.0);
            let arr: Vec<f64> = (0..n).map(|_| {
                // Box-Muller transform
                let u1 = js_sys::Math::random().max(1e-10);
                let u2 = js_sys::Math::random();
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                mean + std * z
            }).collect();
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, arr));
            v_null()
        }
        "uniform" | "random_uniform" => {
            let lo = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let hi = args.get(1).map(|v| v.to_f64()).unwrap_or(1.0);
            let n = args.get(2).map(|v| v.to_i64()).unwrap_or(10) as usize;
            let arr: Vec<f64> = (0..n).map(|_| lo + js_sys::Math::random() * (hi - lo)).collect();
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, arr));
            v_null()
        }
        "randint" => {
            let lo = args.first().map(|v| v.to_i64()).unwrap_or(0);
            let hi = args.get(1).map(|v| v.to_i64()).unwrap_or(100);
            let n = args.get(2).map(|v| v.to_i64()).unwrap_or(10) as usize;
            let range = (hi - lo) as f64;
            let arr: Vec<f64> = (0..n).map(|_| lo as f64 + (js_sys::Math::random() * range).floor()).collect();
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, arr));
            v_null()
        }
        "choice" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(1) as usize;
            let chosen = NUM_STORE.with(|s| {
                let store = s.borrow();
                let a = match store.get(&uname) { Some(a) => a, None => return vec![] };
                if a.is_empty() { return vec![]; }
                (0..n).map(|_| {
                    let idx = (js_sys::Math::random() * a.len() as f64) as usize;
                    a[idx.min(a.len() - 1)]
                }).collect()
            });
            if n == 1 { v_dbl(chosen.first().copied().unwrap_or(0.0)) } else {
                NUM_STORE.with(|s| s.borrow_mut().insert(uname, chosen));
                v_null()
            }
        }
        // --- Output ---
        "tolist" | "tostring" => with_num(&uname, |a| {
            v_str(&a.iter().map(|x| format!("{}", x)).collect::<Vec<_>>().join(","))
        }),
        "print" | "show" => with_num(&uname, |a| {
            let s = format!("[{}]", a.iter().map(|x| format!("{}", x)).collect::<Vec<_>>().join(", "));
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&s));
            crate::builtins::rp_print(&[v_str(&s)], true);
            v_str(&s)
        }),
        "clear" => {
            NUM_STORE.with(|s| s.borrow_mut().insert(uname, Vec::new()));
            v_null()
        }
        _ => {
            web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
                &format!("[WARN] RNum.{} not implemented on web", method)
            ));
            v_null()
        }
    }
}

pub fn num_get_prop(name: &str, prop: &str) -> Value {
    let uname = name.to_uppercase();
    match prop {
        "size" | "length" | "len" | "count" => with_num(&uname, |a| v_int(a.len() as i64)),
        "sum" => with_num(&uname, |a| v_dbl(a.iter().sum())),
        "mean" | "avg" | "average" => with_num(&uname, |a| {
            if a.is_empty() { v_dbl(0.0) } else { v_dbl(a.iter().sum::<f64>() / a.len() as f64) }
        }),
        "min" => with_num(&uname, |a| v_dbl(a.iter().cloned().fold(f64::INFINITY, f64::min))),
        "max" => with_num(&uname, |a| v_dbl(a.iter().cloned().fold(f64::NEG_INFINITY, f64::max))),
        "std" => with_num(&uname, |a| {
            if a.is_empty() { return v_dbl(0.0); }
            let m = a.iter().sum::<f64>() / a.len() as f64;
            let var = a.iter().map(|x| (x - m).powi(2)).sum::<f64>() / a.len() as f64;
            v_dbl(var.sqrt())
        }),
        "data" => with_num(&uname, |a| {
            v_str(&a.iter().map(|x| format!("{}", x)).collect::<Vec<_>>().join(","))
        }),
        "shape" => with_num(&uname, |a| v_str(&format!("({})", a.len()))),
        "ndim" => v_int(1),
        "dtype" => v_str("float64"),
        _ => v_null(),
    }
}

pub fn num_set_prop(name: &str, prop: &str, val: &Value) {
    let uname = name.to_uppercase();
    if prop == "data" {
        let text = val.to_string_val();
        let arr: Vec<f64> = text.split(',')
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();
        NUM_STORE.with(|s| s.borrow_mut().insert(uname, arr));
    }
}

/// Get raw data for use by RPlot.
/// First tries to look up `name` as an RNum component in NUM_STORE.
/// If not found, tries to parse `name` as a comma-separated list of numbers
/// (e.g. "35,25,20,15,5").
pub fn get_num_data(name: &str) -> Option<Vec<f64>> {
    let uname = name.to_uppercase();
    let stored = NUM_STORE.with(|s| s.borrow().get(&uname).cloned());
    if stored.is_some() {
        return stored;
    }
    // Fallback: try parsing as inline CSV numbers
    let parsed: Vec<f64> = name.split(',')
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect();
    if !parsed.is_empty() {
        Some(parsed)
    } else {
        None
    }
}

// --- Helpers ---

fn with_num<F: FnOnce(&[f64]) -> Value>(name: &str, f: F) -> Value {
    NUM_STORE.with(|s| {
        let store = s.borrow();
        match store.get(name) {
            Some(a) => f(a),
            None => v_null(),
        }
    })
}

fn num_map<F: Fn(f64) -> f64>(name: &str, f: F) -> Value {
    NUM_STORE.with(|s| {
        if let Some(a) = s.borrow_mut().get_mut(name) {
            for v in a.iter_mut() { *v = f(*v); }
        }
    });
    v_null()
}

fn num_arith<F: Fn(f64, f64) -> f64>(name: &str, args: &[Value], f: F) -> Value {
    if let Some(other) = args.first() {
        let other_str = other.to_string_val();
        // Try as component name first
        let other_uname = other_str.to_uppercase();
        let other_arr = NUM_STORE.with(|s| s.borrow().get(&other_uname).cloned());
        if let Some(b) = other_arr {
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(name) {
                    for (i, v) in a.iter_mut().enumerate() {
                        let bv = b.get(i).copied().unwrap_or(0.0);
                        *v = f(*v, bv);
                    }
                }
            });
        } else {
            // Treat as scalar
            let scalar = other.to_f64();
            NUM_STORE.with(|s| {
                if let Some(a) = s.borrow_mut().get_mut(name) {
                    for v in a.iter_mut() { *v = f(*v, scalar); }
                }
            });
        }
    }
    v_null()
}

// ======================================================================
// RDataFrame
// ======================================================================

struct DataFrame {
    columns: Vec<String>,
    data: Vec<Vec<String>>, // rows of values
}

impl DataFrame {
    fn new() -> Self {
        DataFrame { columns: Vec::new(), data: Vec::new() }
    }
}

/// Initialize an empty DataFrame in DF_STORE so that subsequent methods (addcolumn, setcell, etc.) can find it.
pub fn init_dataframe(name: &str) {
    let uname = name.to_uppercase();
    DF_STORE.with(|s| {
        s.borrow_mut().entry(uname).or_insert_with(DataFrame::new);
    });
}

pub fn dataframe_method(name: &str, method: &str, args: &[Value]) -> Value {
    let uname = name.to_uppercase();
    match method {
        "create" | "new" | "init" => {
            DF_STORE.with(|s| s.borrow_mut().insert(uname, DataFrame::new()));
            v_null()
        }
        "addrow" | "add_row" | "appendrow" | "push_row" => {
            // Variadic: collect all args as cell strings
            let row: Vec<String> = args.iter().map(|v| v.to_string_val()).collect();
            DF_STORE.with(|s| {
                let mut store = s.borrow_mut();
                let df = store.entry(uname.clone()).or_insert_with(DataFrame::new);
                df.data.push(row);
            });
            v_null()
        }
        "loadfromcsv" | "readcsv" | "read_csv" => {
            // On web, interpret the argument as inline CSV text
            let csv_text = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let mut lines = csv_text.lines();
            let header = match lines.next() {
                Some(h) => h,
                None => return v_null(),
            };
            let columns: Vec<String> = header.split(',').map(|s| s.trim().to_string()).collect();
            let mut data = Vec::new();
            for line in lines {
                let line = line.trim();
                if line.is_empty() { continue; }
                let row: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();
                data.push(row);
            }
            DF_STORE.with(|s| s.borrow_mut().insert(uname, DataFrame { columns, data }));
            v_null()
        }
        "head" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(5) as usize;
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    df.data.truncate(n);
                }
            });
            v_null()
        }
        "tail" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(5) as usize;
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    let len = df.data.len();
                    if n < len { df.data = df.data.split_off(len - n); }
                }
            });
            v_null()
        }
        "cell" => {
            // Convention: cell(col, row) — column first, then row
            let col = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            let row = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as usize;
            DF_STORE.with(|s| {
                let store = s.borrow();
                if let Some(df) = store.get(&uname) {
                    if let Some(r) = df.data.get(row) {
                        if let Some(c) = r.get(col) {
                            return v_str(c);
                        }
                    }
                }
                v_str("")
            })
        }
        "cellbyname" | "at" => {
            let row = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            let col_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            DF_STORE.with(|s| {
                let store = s.borrow();
                if let Some(df) = store.get(&uname) {
                    if let Some(ci) = df.columns.iter().position(|c| c.eq_ignore_ascii_case(&col_name)) {
                        if let Some(r) = df.data.get(row) {
                            if let Some(c) = r.get(ci) {
                                return v_str(c);
                            }
                        }
                    }
                }
                v_str("")
            })
        }
        "setcell" => {
            // Convention: setcell(col, row, value) — column first, then row
            let col = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            let row = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as usize;
            let val = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    // Auto-expand rows if needed
                    let ncols = df.columns.len().max(1);
                    while df.data.len() <= row {
                        df.data.push(vec![String::new(); ncols]);
                    }
                    let r = &mut df.data[row];
                    while r.len() <= col { r.push(String::new()); }
                    r[col] = val;
                }
            });
            v_null()
        }
        "select" => {
            let cols_str = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let cols: Vec<String> = cols_str.split(',').map(|s| s.trim().to_string()).collect();
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    let indices: Vec<usize> = cols.iter()
                        .filter_map(|c| df.columns.iter().position(|col| col.eq_ignore_ascii_case(c)))
                        .collect();
                    df.columns = indices.iter().map(|&i| df.columns[i].clone()).collect();
                    df.data = df.data.iter().map(|row| {
                        indices.iter().map(|&i| row.get(i).cloned().unwrap_or_default()).collect()
                    }).collect();
                }
            });
            v_null()
        }
        "sort" => {
            let col = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let asc = args.get(1).map(|v| v.to_i64()).unwrap_or(1) != 0;
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    if let Some(ci) = df.columns.iter().position(|c| c.eq_ignore_ascii_case(&col)) {
                        df.data.sort_by(|a, b| {
                            let av = a.get(ci).cloned().unwrap_or_default();
                            let bv = b.get(ci).cloned().unwrap_or_default();
                            // Try numeric sort
                            if let (Ok(an), Ok(bn)) = (av.parse::<f64>(), bv.parse::<f64>()) {
                                let cmp = an.partial_cmp(&bn).unwrap_or(std::cmp::Ordering::Equal);
                                if asc { cmp } else { cmp.reverse() }
                            } else {
                                let cmp = av.cmp(&bv);
                                if asc { cmp } else { cmp.reverse() }
                            }
                        });
                    }
                }
            });
            v_null()
        }
        "filter" => {
            let col = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let op = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let val = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    if let Some(ci) = df.columns.iter().position(|c| c.eq_ignore_ascii_case(&col)) {
                        df.data.retain(|row| {
                            let cv = row.get(ci).cloned().unwrap_or_default();
                            match op.as_str() {
                                "==" => cv == val,
                                "!=" => cv != val,
                                "contains" => cv.contains(&val),
                                ">" | ">=" | "<" | "<=" => {
                                    if let (Ok(a), Ok(b)) = (cv.parse::<f64>(), val.parse::<f64>()) {
                                        match op.as_str() {
                                            ">" => a > b,
                                            ">=" => a >= b,
                                            "<" => a < b,
                                            "<=" => a <= b,
                                            _ => false,
                                        }
                                    } else { false }
                                }
                                _ => true,
                            }
                        });
                    }
                }
            });
            v_null()
        }
        "drop" | "drop_column" => {
            let col = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    if let Some(ci) = df.columns.iter().position(|c| c.eq_ignore_ascii_case(&col)) {
                        df.columns.remove(ci);
                        for row in df.data.iter_mut() {
                            if ci < row.len() { row.remove(ci); }
                        }
                    }
                }
            });
            v_null()
        }
        "rename" | "rename_column" => {
            let old = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let new = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    if let Some(ci) = df.columns.iter().position(|c| c.eq_ignore_ascii_case(&old)) {
                        df.columns[ci] = new;
                    }
                }
            });
            v_null()
        }
        "addcolumn" | "add_column" | "set_column" => {
            let col_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let values = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let vals: Vec<String> = values.split(',').map(|s| s.trim().to_string()).collect();
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    df.columns.push(col_name);
                    for (i, row) in df.data.iter_mut().enumerate() {
                        row.push(vals.get(i).cloned().unwrap_or_default());
                    }
                }
            });
            v_null()
        }
        "fillna" | "fill_null" => {
            let fill = args.first().map(|v| v.to_string_val()).unwrap_or_else(|| "0".to_string());
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    for row in df.data.iter_mut() {
                        for cell in row.iter_mut() {
                            if cell.is_empty() || cell == "null" || cell == "NULL" || cell == "NA" {
                                *cell = fill.clone();
                            }
                        }
                    }
                }
            });
            v_null()
        }
        "dropna" | "drop_nulls" => {
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    df.data.retain(|row| {
                        !row.iter().any(|c| c.is_empty() || c == "null" || c == "NULL" || c == "NA")
                    });
                }
            });
            v_null()
        }
        "describe" => {
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    let ncols = df.columns.len();
                    let mut new_cols = vec!["stat".to_string()];
                    new_cols.extend(df.columns.clone());
                    let mut new_data = Vec::new();
                    // count, mean, std, min, max
                    let stats = ["count", "mean", "std", "min", "max"];
                    for stat in &stats {
                        let mut row = vec![stat.to_string()];
                        for ci in 0..ncols {
                            let vals: Vec<f64> = df.data.iter()
                                .filter_map(|r| r.get(ci).and_then(|v| v.parse::<f64>().ok()))
                                .collect();
                            let val = match *stat {
                                "count" => vals.len() as f64,
                                "mean" => if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 },
                                "std" => {
                                    if vals.is_empty() { 0.0 } else {
                                        let m = vals.iter().sum::<f64>() / vals.len() as f64;
                                        (vals.iter().map(|x| (x - m).powi(2)).sum::<f64>() / vals.len() as f64).sqrt()
                                    }
                                }
                                "min" => vals.iter().cloned().fold(f64::INFINITY, f64::min),
                                "max" => vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                                _ => 0.0,
                            };
                            row.push(format!("{:.2}", val));
                        }
                        new_data.push(row);
                    }
                    df.columns = new_cols;
                    df.data = new_data;
                }
            });
            v_null()
        }
        "columns" => {
            DF_STORE.with(|s| {
                let store = s.borrow();
                if let Some(df) = store.get(&uname) {
                    v_str(&df.columns.join(","))
                } else {
                    v_str("")
                }
            })
        }
        "rows" | "rowcount" | "len" => {
            DF_STORE.with(|s| {
                let store = s.borrow();
                if let Some(df) = store.get(&uname) {
                    v_int(df.data.len() as i64)
                } else {
                    v_int(0)
                }
            })
        }
        "tostring" | "show" | "print" => {
            DF_STORE.with(|s| {
                let store = s.borrow();
                if let Some(df) = store.get(&uname) {
                    let mut out = df.columns.join(" | ") + "\n";
                    out += &format!("{}\n", "-".repeat(out.len()));
                    for row in &df.data {
                        out += &row.join(" | ");
                        out += "\n";
                    }
                    crate::builtins::rp_print(&[v_str(&out)], true);
                    // Also render into the visual widget if it exists.
                    let id = format!("rr-{}", uname.to_lowercase());
                    if let Some(el) = gui_web::document().get_element_by_id(&id) {
                        let mut html = String::from("<table style=\"border-collapse:collapse;width:100%;font-size:11px;\"><thead><tr>");
                        for c in &df.columns {
                            html += &format!("<th style=\"border:1px solid #bbb;background:#eee;padding:2px 6px;text-align:left;\">{}</th>", html_escape(c));
                        }
                        html += "</tr></thead><tbody>";
                        for row in &df.data {
                            html += "<tr>";
                            for cell in row {
                                html += &format!("<td style=\"border:1px solid #ddd;padding:2px 6px;\">{}</td>", html_escape(cell));
                            }
                            html += "</tr>";
                        }
                        html += "</tbody></table>";
                        el.set_inner_html(&html);
                    }
                    v_str(&out)
                } else {
                    v_str("")
                }
            })
        }
        "togrid" | "to_grid" | "display" => {
            if let Some(grid_name) = args.first() {
                let grid_uname = grid_name.to_string_val().to_uppercase();
                DF_STORE.with(|s| {
                    let store = s.borrow();
                    if let Some(df) = store.get(&uname) {
                        let ncols = df.columns.len();
                        let nrows = df.data.len() + 1; // +1 for header
                        // Set grid dimensions
                        object_web::rp_comp_method(&grid_uname, "setcolcount", &[v_int(ncols as i64)]);
                        object_web::rp_comp_method(&grid_uname, "setrowcount", &[v_int(nrows as i64)]);
                        // Header row
                        for (ci, col) in df.columns.iter().enumerate() {
                            object_web::rp_comp_method(&grid_uname, "setcell",
                                &[v_int(ci as i64), v_int(0), v_str(col)]);
                        }
                        // Data rows
                        for (ri, row) in df.data.iter().enumerate() {
                            for (ci, cell) in row.iter().enumerate() {
                                object_web::rp_comp_method(&grid_uname, "setcell",
                                    &[v_int(ci as i64), v_int((ri + 1) as i64), v_str(cell)]);
                            }
                        }
                    }
                });
            }
            v_null()
        }
        "clear" => {
            DF_STORE.with(|s| s.borrow_mut().insert(uname, DataFrame::new()));
            v_null()
        }
        "info" => {
            DF_STORE.with(|s| {
                let store = s.borrow();
                if let Some(df) = store.get(&uname) {
                    let msg = format!("DataFrame: {} rows x {} cols\nColumns: {}",
                        df.data.len(), df.columns.len(), df.columns.join(", "));
                    crate::builtins::rp_print(&[v_str(&msg)], true);
                    v_str(&msg)
                } else {
                    v_str("")
                }
            })
        }
        "sample" => {
            let n = args.first().map(|v| v.to_i64()).unwrap_or(5) as usize;
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    if df.data.len() > n {
                        let mut sampled = Vec::new();
                        for _ in 0..n {
                            let idx = (js_sys::Math::random() * df.data.len() as f64) as usize;
                            sampled.push(df.data[idx.min(df.data.len() - 1)].clone());
                        }
                        df.data = sampled;
                    }
                }
            });
            v_null()
        }
        "transpose" | "t" => {
            DF_STORE.with(|s| {
                if let Some(df) = s.borrow_mut().get_mut(&uname) {
                    if df.data.is_empty() { return; }
                    let nrows = df.data.len();
                    let ncols = df.columns.len();
                    let mut new_data = Vec::new();
                    let mut new_cols = vec!["index".to_string()];
                    for i in 0..nrows { new_cols.push(format!("{}", i)); }
                    for ci in 0..ncols {
                        let mut row = vec![df.columns[ci].clone()];
                        for ri in 0..nrows {
                            row.push(df.data[ri].get(ci).cloned().unwrap_or_default());
                        }
                        new_data.push(row);
                    }
                    df.columns = new_cols;
                    df.data = new_data;
                }
            });
            v_null()
        }
        _ => {
            web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
                &format!("[WARN] RDataFrame.{} not implemented on web", method)
            ));
            v_null()
        }
    }
}

pub fn dataframe_get_prop(name: &str, prop: &str) -> Value {
    let uname = name.to_uppercase();
    DF_STORE.with(|s| {
        let store = s.borrow();
        match store.get(&uname) {
            Some(df) => match prop {
                "rowcount" | "height" | "nrows" => v_int(df.data.len() as i64),
                "colcount" | "width" | "ncols" => v_int(df.columns.len() as i64),
                "columns" => v_str(&df.columns.join(",")),
                "shape" => v_str(&format!("({}, {})", df.data.len(), df.columns.len())),
                "empty" => v_int(if df.data.is_empty() { 1 } else { 0 }),
                _ => v_null(),
            },
            None => v_null(),
        }
    })
}

// ======================================================================
// RPlot — renders to HTML5 Canvas
// ======================================================================

#[derive(Clone)]
struct PlotSeries {
    kind: String,      // "line", "bar", "scatter", "step", "area"
    x: Vec<f64>,
    y: Vec<f64>,
    label: String,
    color: String,
}

#[derive(Clone)]
struct PlotState {
    title: String,
    xlabel: String,
    ylabel: String,
    width: f64,
    height: f64,
    series: Vec<PlotSeries>,
    show_grid: bool,
    show_legend: bool,
    annotations: Vec<(String, f64, f64, String)>, // text, x, y, color
    hlines: Vec<(f64, String)>,
    vlines: Vec<(f64, String)>,
    pie_values: Vec<f64>,
    pie_labels: Vec<String>,
    pie_colors: Vec<String>,
}

impl PlotState {
    fn new() -> Self {
        PlotState {
            title: String::new(),
            xlabel: String::new(),
            ylabel: String::new(),
            width: 600.0,
            height: 400.0,
            series: Vec::new(),
            show_grid: true,
            show_legend: false,
            annotations: Vec::new(),
            hlines: Vec::new(),
            vlines: Vec::new(),
            pie_values: Vec::new(),
            pie_labels: Vec::new(),
            pie_colors: Vec::new(),
        }
    }
}

const DEFAULT_COLORS: &[&str] = &[
    "#2196F3", "#F44336", "#4CAF50", "#FF9800", "#9C27B0",
    "#00BCD4", "#795548", "#E91E63", "#3F51B5", "#009688",
];

pub fn plot_method(name: &str, method: &str, args: &[Value]) -> Value {
    let uname = name.to_uppercase();

    // Ensure state exists
    PLOT_STORE.with(|s| {
        let mut store = s.borrow_mut();
        if !store.contains_key(&uname) {
            store.insert(uname.clone(), PlotState::new());
        }
    });

    match method {
        "create" | "new" | "init" => {
            // State already ensured at top of function — just acknowledge.
            v_null()
        }
        "settitle" | "set_title" => {
            let title = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            PLOT_STORE.with(|s| {
                if let Some(ps) = s.borrow_mut().get_mut(&uname) { ps.title = title; }
            });
            v_null()
        }
        "setxlabel" | "set_xlabel" | "xlabel" => {
            let v = args.first().map(|x| x.to_string_val()).unwrap_or_default();
            PLOT_STORE.with(|s| { if let Some(ps) = s.borrow_mut().get_mut(&uname) { ps.xlabel = v; } });
            v_null()
        }
        "setylabel" | "set_ylabel" | "ylabel" => {
            let v = args.first().map(|x| x.to_string_val()).unwrap_or_default();
            PLOT_STORE.with(|s| { if let Some(ps) = s.borrow_mut().get_mut(&uname) { ps.ylabel = v; } });
            v_null()
        }
        "addseries" | "add_series" | "series" => {
            // addseries(label, csv_y [, csv_x [, color]])
            let label = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_str = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let x_str = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let color = args.get(3).map(|v| v.to_string_val()).unwrap_or_default();
            let y_data: Vec<f64> = y_str.split(',')
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .collect();
            let x_data: Vec<f64> = if x_str.is_empty() {
                (0..y_data.len()).map(|i| i as f64).collect()
            } else {
                x_str.split(',').filter_map(|s| s.trim().parse::<f64>().ok()).collect()
            };
            PLOT_STORE.with(|s| {
                if let Some(ps) = s.borrow_mut().get_mut(&uname) {
                    let idx = ps.series.len();
                    let c = if color.is_empty() {
                        DEFAULT_COLORS[idx % DEFAULT_COLORS.len()].to_string()
                    } else { color };
                    ps.series.push(PlotSeries {
                        kind: "line".to_string(), x: x_data, y: y_data, label, color: c,
                    });
                }
            });
            // Auto-render after each series add so RPlot shows up immediately.
            render_plot(&uname);
            v_null()
        }
        "clear" => {
            PLOT_STORE.with(|s| s.borrow_mut().insert(uname, PlotState::new()));
            v_null()
        }
        "plot" | "bar" | "barh" | "scatter" | "step" | "area" | "fill_between" => {
            let x_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let y_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let color = args.get(3).map(|v| v.to_string_val()).unwrap_or_default();

            let x_data = get_num_data(&x_name).unwrap_or_default();
            let y_data = get_num_data(&y_name).unwrap_or_default();

            let kind = match method {
                "barh" => "barh",
                "bar" => "bar",
                "scatter" => "scatter",
                "step" => "step",
                "area" | "fill_between" => "area",
                _ => "line",
            };

            PLOT_STORE.with(|s| {
                if let Some(ps) = s.borrow_mut().get_mut(&uname) {
                    let idx = ps.series.len();
                    let c = if color.is_empty() {
                        DEFAULT_COLORS[idx % DEFAULT_COLORS.len()].to_string()
                    } else { color };
                    ps.series.push(PlotSeries {
                        kind: kind.to_string(), x: x_data, y: y_data, label, color: c,
                    });
                }
            });
            v_null()
        }
        "hist" | "histogram" => {
            let data_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let bins = args.get(1).map(|v| v.to_i64()).unwrap_or(10) as usize;
            let label = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            let color = args.get(3).map(|v| v.to_string_val()).unwrap_or_default();

            let raw = get_num_data(&data_name).unwrap_or_default();
            if raw.is_empty() { return v_null(); }

            let mn = raw.iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = raw.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let bin_w = if mx > mn { (mx - mn) / bins as f64 } else { 1.0 };

            let mut counts = vec![0.0f64; bins];
            let mut centers = Vec::with_capacity(bins);
            for i in 0..bins { centers.push(mn + bin_w * (i as f64 + 0.5)); }
            for v in &raw {
                let mut idx = ((v - mn) / bin_w) as usize;
                if idx >= bins { idx = bins - 1; }
                counts[idx] += 1.0;
            }

            PLOT_STORE.with(|s| {
                if let Some(ps) = s.borrow_mut().get_mut(&uname) {
                    let idx = ps.series.len();
                    let c = if color.is_empty() {
                        DEFAULT_COLORS[idx % DEFAULT_COLORS.len()].to_string()
                    } else { color };
                    ps.series.push(PlotSeries {
                        kind: "bar".to_string(), x: centers, y: counts, label, color: c,
                    });
                }
            });
            v_null()
        }
        "pie" => {
            let vals_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let labels_csv = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let colors_csv = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();

            let vals = get_num_data(&vals_name).unwrap_or_default();
            let labels: Vec<String> = if labels_csv.is_empty() {
                (0..vals.len()).map(|i| format!("Slice {}", i + 1)).collect()
            } else {
                labels_csv.split(',').map(|s| s.trim().to_string()).collect()
            };
            let colors: Vec<String> = if colors_csv.is_empty() {
                DEFAULT_COLORS.iter().map(|s| s.to_string()).collect()
            } else {
                colors_csv.split(',').map(|s| s.trim().to_string()).collect()
            };

            PLOT_STORE.with(|s| {
                if let Some(ps) = s.borrow_mut().get_mut(&uname) {
                    ps.pie_values = vals;
                    ps.pie_labels = labels;
                    ps.pie_colors = colors;
                }
            });
            v_null()
        }
        "hline" | "axhline" => {
            let y = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let color = args.get(1).map(|v| v.to_string_val()).unwrap_or_else(|| "red".to_string());
            PLOT_STORE.with(|s| {
                if let Some(ps) = s.borrow_mut().get_mut(&uname) {
                    ps.hlines.push((y, color));
                }
            });
            v_null()
        }
        "vline" | "axvline" => {
            let x = args.first().map(|v| v.to_f64()).unwrap_or(0.0);
            let color = args.get(1).map(|v| v.to_string_val()).unwrap_or_else(|| "red".to_string());
            PLOT_STORE.with(|s| {
                if let Some(ps) = s.borrow_mut().get_mut(&uname) {
                    ps.vlines.push((x, color));
                }
            });
            v_null()
        }
        "annotate" => {
            let text = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let x = args.get(1).map(|v| v.to_f64()).unwrap_or(0.0);
            let y = args.get(2).map(|v| v.to_f64()).unwrap_or(0.0);
            let color = args.get(3).map(|v| v.to_string_val()).unwrap_or_else(|| "black".to_string());
            PLOT_STORE.with(|s| {
                if let Some(ps) = s.borrow_mut().get_mut(&uname) {
                    ps.annotations.push((text, x, y, color));
                }
            });
            v_null()
        }
        "legend" => {
            PLOT_STORE.with(|s| {
                if let Some(ps) = s.borrow_mut().get_mut(&uname) {
                    ps.show_legend = true;
                }
            });
            v_null()
        }
        "figsize" => {
            let w = args.first().map(|v| v.to_f64()).unwrap_or(8.0);
            let h = args.get(1).map(|v| v.to_f64()).unwrap_or(6.0);
            PLOT_STORE.with(|s| {
                if let Some(ps) = s.borrow_mut().get_mut(&uname) {
                    ps.width = w * 80.0; // rough px conversion
                    ps.height = h * 80.0;
                }
            });
            v_null()
        }
        "xlim" | "ylim" => {
            // Stored but auto-scaling is used in render
            v_null()
        }
        "savefig" | "save" | "render" | "show" => {
            // Render to the plot's canvas
            render_plot(&uname);
            v_null()
        }
        _ => {
            web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
                &format!("[WARN] RPlot.{} not implemented on web", method)
            ));
            v_null()
        }
    }
}

pub fn plot_get_prop(name: &str, prop: &str) -> Value {
    let uname = name.to_uppercase();
    PLOT_STORE.with(|s| {
        let store = s.borrow();
        match store.get(&uname) {
            Some(ps) => match prop {
                "title" => v_str(&ps.title),
                "xlabel" => v_str(&ps.xlabel),
                "ylabel" => v_str(&ps.ylabel),
                "grid" => v_int(if ps.show_grid { 1 } else { 0 }),
                "width" => v_int(ps.width as i64),
                "height" => v_int(ps.height as i64),
                _ => v_null(),
            },
            None => v_null(),
        }
    })
}

pub fn plot_set_prop(name: &str, prop: &str, val: &Value) {
    let uname = name.to_uppercase();
    PLOT_STORE.with(|s| {
        let mut store = s.borrow_mut();
        let ps = store.entry(uname).or_insert_with(PlotState::new);
        match prop {
            "title" => ps.title = val.to_string_val(),
            "xlabel" => ps.xlabel = val.to_string_val(),
            "ylabel" => ps.ylabel = val.to_string_val(),
            "grid" => ps.show_grid = val.to_i64() != 0,
            "width" => ps.width = val.to_f64().max(100.0),
            "height" => ps.height = val.to_f64().max(100.0),
            _ => {}
        }
    });
}

// ======================================================================
// Plot rendering to HTML5 Canvas
// ======================================================================

fn render_plot(name: &str) {
    let state = PLOT_STORE.with(|s| s.borrow().get(name).cloned());
    let state = match state { Some(s) => s, None => return };

    // Find or create a canvas element for this plot
    let doc = web_sys::window().unwrap().document().unwrap();
    let canvas_id = format!("rr-{}-canvas", name.to_lowercase());

    let canvas_el = match doc.get_element_by_id(&canvas_id) {
        Some(el) => el,
        None => {
            // Look for the plot container div
            let container_id = format!("rr-{}", name.to_lowercase());
            if let Some(container) = doc.get_element_by_id(&container_id) {
                // Hide the visual placeholder so the chart shows by itself.
                if let Some(ph) = container.query_selector(".rr-plot-placeholder").ok().flatten() {
                    if let Ok(html_ph) = ph.dyn_into::<web_sys::HtmlElement>() {
                        let _ = html_ph.style().set_property("display", "none");
                    }
                }
                let c = doc.create_element("canvas").unwrap();
                c.set_id(&canvas_id);
                c.set_class_name("rr-plot-container");
                let _ = container.append_child(&c);
                c
            } else {
                return;
            }
        }
    };

    let canvas: web_sys::HtmlCanvasElement = match canvas_el.dyn_into() {
        Ok(c) => c,
        Err(_) => return,
    };

    let w = state.width;
    let h = state.height;
    canvas.set_width(w as u32);
    canvas.set_height(h as u32);

    let ctx: web_sys::CanvasRenderingContext2d = match canvas.get_context("2d") {
        Ok(Some(c)) => c.dyn_into().unwrap(),
        _ => return,
    };

    // Clear
    ctx.set_fill_style_str("white");
    ctx.fill_rect(0.0, 0.0, w, h);

    // If pie chart, render that instead
    if !state.pie_values.is_empty() {
        render_pie(&ctx, &state, w, h);
        return;
    }

    // Plot area margins
    let ml = 60.0;  // left
    let mr = 20.0;  // right
    let mt = if state.title.is_empty() { 20.0 } else { 40.0 }; // top
    let mb = if state.xlabel.is_empty() { 40.0 } else { 55.0 }; // bottom
    let pw = w - ml - mr;
    let ph = h - mt - mb;

    // Compute data range
    let (mut xmin, mut xmax, mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for s in &state.series {
        for &x in &s.x { if x < xmin { xmin = x; } if x > xmax { xmax = x; } }
        for &y in &s.y { if y < ymin { ymin = y; } if y > ymax { ymax = y; } }
    }
    for &(y, _) in &state.hlines { if y < ymin { ymin = y; } if y > ymax { ymax = y; } }
    for &(x, _) in &state.vlines { if x < xmin { xmin = x; } if x > xmax { xmax = x; } }

    if xmin == f64::INFINITY { xmin = 0.0; xmax = 1.0; }
    if ymin == f64::INFINITY { ymin = 0.0; ymax = 1.0; }
    if (xmax - xmin).abs() < 1e-10 { xmin -= 1.0; xmax += 1.0; }
    if (ymax - ymin).abs() < 1e-10 { ymin -= 1.0; ymax += 1.0; }

    // Add 5% padding
    let xpad = (xmax - xmin) * 0.05;
    let ypad = (ymax - ymin) * 0.05;
    xmin -= xpad; xmax += xpad;
    ymin -= ypad; ymax += ypad;
    if ymin > 0.0 && ymin < ypad * 2.0 { ymin = 0.0; }

    let to_px_x = |x: f64| -> f64 { ml + (x - xmin) / (xmax - xmin) * pw };
    let to_px_y = |y: f64| -> f64 { mt + (1.0 - (y - ymin) / (ymax - ymin)) * ph };

    // Draw grid
    if state.show_grid {
        ctx.set_stroke_style_str("#e0e0e0");
        ctx.set_line_width(0.5);
        let ny = 5;
        for i in 0..=ny {
            let y = ymin + (ymax - ymin) * i as f64 / ny as f64;
            let py = to_px_y(y);
            ctx.begin_path();
            ctx.move_to(ml, py);
            ctx.line_to(ml + pw, py);
            ctx.stroke();
        }
        let nx = 5;
        for i in 0..=nx {
            let x = xmin + (xmax - xmin) * i as f64 / nx as f64;
            let px = to_px_x(x);
            ctx.begin_path();
            ctx.move_to(px, mt);
            ctx.line_to(px, mt + ph);
            ctx.stroke();
        }
    }

    // Draw axes
    ctx.set_stroke_style_str("#333");
    ctx.set_line_width(1.0);
    ctx.begin_path();
    ctx.move_to(ml, mt);
    ctx.line_to(ml, mt + ph);
    ctx.line_to(ml + pw, mt + ph);
    ctx.stroke();

    // Axis labels
    ctx.set_fill_style_str("#555");
    ctx.set_font("11px sans-serif");
    let ny = 5;
    for i in 0..=ny {
        let y = ymin + (ymax - ymin) * i as f64 / ny as f64;
        let py = to_px_y(y);
        let label = format_number(y);
        ctx.set_text_align("right");
        let _ = ctx.fill_text(&label, ml - 5.0, py + 3.0);
    }
    let nx = 5;
    for i in 0..=nx {
        let x = xmin + (xmax - xmin) * i as f64 / nx as f64;
        let px = to_px_x(x);
        let label = format_number(x);
        ctx.set_text_align("center");
        let _ = ctx.fill_text(&label, px, mt + ph + 15.0);
    }

    // Draw hlines / vlines
    for (y, color) in &state.hlines {
        let py = to_px_y(*y);
        ctx.set_stroke_style_str(color);
        ctx.set_line_width(1.5);
        ctx.begin_path();
        ctx.move_to(ml, py);
        ctx.line_to(ml + pw, py);
        ctx.stroke();
    }
    for (x, color) in &state.vlines {
        let px = to_px_x(*x);
        ctx.set_stroke_style_str(color);
        ctx.set_line_width(1.5);
        ctx.begin_path();
        ctx.move_to(px, mt);
        ctx.line_to(px, mt + ph);
        ctx.stroke();
    }

    // Draw series
    for s in &state.series {
        ctx.set_stroke_style_str(&s.color);
        ctx.set_fill_style_str(&s.color);
        match s.kind.as_str() {
            "line" => {
                ctx.set_line_width(2.0);
                ctx.begin_path();
                for (i, (&x, &y)) in s.x.iter().zip(s.y.iter()).enumerate() {
                    let px = to_px_x(x);
                    let py = to_px_y(y);
                    if i == 0 { ctx.move_to(px, py); } else { ctx.line_to(px, py); }
                }
                ctx.stroke();
            }
            "scatter" => {
                for (&x, &y) in s.x.iter().zip(s.y.iter()) {
                    let px = to_px_x(x);
                    let py = to_px_y(y);
                    ctx.begin_path();
                    let _ = ctx.arc(px, py, 4.0, 0.0, 2.0 * std::f64::consts::PI);
                    ctx.fill();
                }
            }
            "bar" => {
                let n = s.x.len();
                let bar_w = if n > 1 {
                    (to_px_x(s.x[1]) - to_px_x(s.x[0])) * 0.7
                } else {
                    pw * 0.7 / n.max(1) as f64
                };
                let base_y = to_px_y(0f64.max(ymin));
                for (&x, &y) in s.x.iter().zip(s.y.iter()) {
                    let px = to_px_x(x) - bar_w / 2.0;
                    let py = to_px_y(y);
                    let bh = (base_y - py).abs();
                    ctx.set_global_alpha(0.8);
                    ctx.fill_rect(px, py.min(base_y), bar_w, bh);
                    ctx.set_global_alpha(1.0);
                    ctx.stroke_rect(px, py.min(base_y), bar_w, bh);
                }
            }
            "step" => {
                ctx.set_line_width(2.0);
                ctx.begin_path();
                for (i, (&x, &y)) in s.x.iter().zip(s.y.iter()).enumerate() {
                    let px = to_px_x(x);
                    let py = to_px_y(y);
                    if i == 0 {
                        ctx.move_to(px, py);
                    } else {
                        let prev_y = to_px_y(s.y[i - 1]);
                        ctx.line_to(px, prev_y);
                        ctx.line_to(px, py);
                    }
                }
                ctx.stroke();
            }
            "area" => {
                ctx.set_global_alpha(0.3);
                ctx.begin_path();
                let base_y = to_px_y(0f64.max(ymin));
                if let Some((&fx, &fy)) = s.x.iter().zip(s.y.iter()).next() {
                    ctx.move_to(to_px_x(fx), base_y);
                    ctx.line_to(to_px_x(fx), to_px_y(fy));
                }
                for (&x, &y) in s.x.iter().zip(s.y.iter()).skip(1) {
                    ctx.line_to(to_px_x(x), to_px_y(y));
                }
                if let Some(&lx) = s.x.last() {
                    ctx.line_to(to_px_x(lx), base_y);
                }
                ctx.close_path();
                ctx.fill();
                ctx.set_global_alpha(1.0);
                // Line on top
                ctx.set_line_width(2.0);
                ctx.begin_path();
                for (i, (&x, &y)) in s.x.iter().zip(s.y.iter()).enumerate() {
                    let px = to_px_x(x);
                    let py = to_px_y(y);
                    if i == 0 { ctx.move_to(px, py); } else { ctx.line_to(px, py); }
                }
                ctx.stroke();
            }
            _ => {}
        }
    }

    // Annotations
    for (text, x, y, color) in &state.annotations {
        ctx.set_fill_style_str(color);
        ctx.set_font("12px sans-serif");
        ctx.set_text_align("left");
        let _ = ctx.fill_text(text, to_px_x(*x) + 5.0, to_px_y(*y) - 5.0);
    }

    // Title
    if !state.title.is_empty() {
        ctx.set_fill_style_str("#222");
        ctx.set_font("bold 14px sans-serif");
        ctx.set_text_align("center");
        let _ = ctx.fill_text(&state.title, w / 2.0, 20.0);
    }

    // X/Y labels
    if !state.xlabel.is_empty() {
        ctx.set_fill_style_str("#555");
        ctx.set_font("12px sans-serif");
        ctx.set_text_align("center");
        let _ = ctx.fill_text(&state.xlabel, ml + pw / 2.0, h - 5.0);
    }
    if !state.ylabel.is_empty() {
        ctx.save();
        ctx.set_fill_style_str("#555");
        ctx.set_font("12px sans-serif");
        ctx.set_text_align("center");
        let _ = ctx.translate(12.0, mt + ph / 2.0);
        let _ = ctx.rotate(-std::f64::consts::FRAC_PI_2);
        let _ = ctx.fill_text(&state.ylabel, 0.0, 0.0);
        ctx.restore();
    }

    // Legend
    if state.show_legend && !state.series.is_empty() {
        let labeled: Vec<&PlotSeries> = state.series.iter().filter(|s| !s.label.is_empty()).collect();
        if !labeled.is_empty() {
            let lx = ml + pw - 120.0;
            let ly = mt + 10.0;
            ctx.set_fill_style_str("rgba(255,255,255,0.9)");
            ctx.fill_rect(lx, ly, 115.0, (labeled.len() as f64) * 18.0 + 6.0);
            ctx.set_stroke_style_str("#ccc");
            ctx.stroke_rect(lx, ly, 115.0, (labeled.len() as f64) * 18.0 + 6.0);
            for (i, s) in labeled.iter().enumerate() {
                let ey = ly + 12.0 + i as f64 * 18.0;
                ctx.set_fill_style_str(&s.color);
                ctx.fill_rect(lx + 5.0, ey - 6.0, 12.0, 12.0);
                ctx.set_fill_style_str("#333");
                ctx.set_font("11px sans-serif");
                ctx.set_text_align("left");
                let _ = ctx.fill_text(&s.label, lx + 22.0, ey + 3.0);
            }
        }
    }
}

fn render_pie(ctx: &web_sys::CanvasRenderingContext2d, state: &PlotState, w: f64, h: f64) {
    let total: f64 = state.pie_values.iter().sum();
    if total == 0.0 { return; }

    let cx = w / 2.0;
    let cy = if state.title.is_empty() { h / 2.0 } else { h / 2.0 + 15.0 };
    let radius = (w.min(h) / 2.0 - 40.0).max(50.0);

    // Title
    if !state.title.is_empty() {
        ctx.set_fill_style_str("#222");
        ctx.set_font("bold 14px sans-serif");
        ctx.set_text_align("center");
        let _ = ctx.fill_text(&state.title, w / 2.0, 20.0);
    }

    let mut start_angle = -std::f64::consts::FRAC_PI_2;
    for (i, &val) in state.pie_values.iter().enumerate() {
        let sweep = (val / total) * 2.0 * std::f64::consts::PI;
        let color = state.pie_colors.get(i).map(|s| s.as_str())
            .unwrap_or(DEFAULT_COLORS[i % DEFAULT_COLORS.len()]);

        ctx.set_fill_style_str(color);
        ctx.begin_path();
        ctx.move_to(cx, cy);
        let _ = ctx.arc(cx, cy, radius, start_angle, start_angle + sweep);
        ctx.close_path();
        ctx.fill();
        ctx.set_stroke_style_str("white");
        ctx.set_line_width(2.0);
        ctx.stroke();

        // Label
        let mid = start_angle + sweep / 2.0;
        let lx = cx + (radius * 0.65) * mid.cos();
        let ly = cy + (radius * 0.65) * mid.sin();
        let pct = format!("{:.0}%", val / total * 100.0);
        ctx.set_fill_style_str("white");
        ctx.set_font("bold 12px sans-serif");
        ctx.set_text_align("center");
        let _ = ctx.fill_text(&pct, lx, ly + 4.0);

        start_angle += sweep;
    }

    // Legend for pie
    let lx = 10.0;
    let ly = h - 10.0 - state.pie_labels.len() as f64 * 18.0;
    for (i, label) in state.pie_labels.iter().enumerate() {
        let color = state.pie_colors.get(i).map(|s| s.as_str())
            .unwrap_or(DEFAULT_COLORS[i % DEFAULT_COLORS.len()]);
        let ey = ly + i as f64 * 18.0;
        ctx.set_fill_style_str(color);
        ctx.fill_rect(lx, ey, 12.0, 12.0);
        ctx.set_fill_style_str("#333");
        ctx.set_font("11px sans-serif");
        ctx.set_text_align("left");
        let _ = ctx.fill_text(label, lx + 16.0, ey + 10.0);
    }
}

fn format_number(v: f64) -> String {
    if v.abs() < 1e-10 { return "0".to_string(); }
    if v.abs() >= 1000.0 || v.abs() < 0.01 {
        format!("{:.1e}", v)
    } else if v == v.floor() {
        format!("{:.0}", v)
    } else {
        format!("{:.2}", v)
    }
}

// ======================================================================
// Ensure RPLOT creates a canvas container in DOM
// ======================================================================

pub fn create_plot_widget(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = gui_web::create_el("div");
    el.set_class_name("rr-widget rr-plot-container");
    let _ = el.style().set_property("background", "white");
    let _ = el.style().set_property("border", "1px solid #ccc");
    let _ = el.style().set_property("display", "flex");
    let _ = el.style().set_property("align-items", "center");
    let _ = el.style().set_property("justify-content", "center");
    let _ = el.style().set_property("color", "#888");
    let _ = el.style().set_property("font-family", "sans-serif");
    let _ = el.style().set_property("font-size", "12px");
    el.set_inner_html(&format!(
        "<span class=\"rr-plot-placeholder\">📈 {}<br/><small>(call .render() / .plot())</small></span>",
        name
    ));
    gui_web::setup_widget(&el, id, name, props);
}

// ======================================================================
// Ensure RDataFrame creates a table container in DOM (visual placeholder).
// Populated when .show() / .togrid() / etc. are called.
// ======================================================================

pub fn create_dataframe_widget(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = gui_web::create_el("div");
    el.set_class_name("rr-widget rr-dataframe-container");
    let _ = el.style().set_property("background", "white");
    let _ = el.style().set_property("border", "1px solid #ccc");
    let _ = el.style().set_property("overflow", "auto");
    let _ = el.style().set_property("font-family", "monospace");
    let _ = el.style().set_property("font-size", "12px");
    el.set_inner_html(&format!(
        "<div class=\"rr-df-placeholder\" style=\"padding:8px;color:#888;\">▦ {}<br/><small>(call .show() to render)</small></div>",
        name
    ));
    gui_web::setup_widget(&el, id, name, props);
}

// ======================================================================
// RNum visual placeholder (numeric array — usually non-visual but if a
// designer marks it visible we render a tiny chip showing the count).
// ======================================================================

pub fn create_num_widget(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = gui_web::create_el("div");
    el.set_class_name("rr-widget rr-num-container");
    let _ = el.style().set_property("background", "#f8f8f8");
    let _ = el.style().set_property("border", "1px dashed #aaa");
    let _ = el.style().set_property("color", "#555");
    let _ = el.style().set_property("font-family", "monospace");
    let _ = el.style().set_property("font-size", "11px");
    let _ = el.style().set_property("padding", "4px 8px");
    el.set_inner_html(&format!("ƒ {} (RNum)", name));
    gui_web::setup_widget(&el, id, name, props);
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
