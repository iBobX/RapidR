//! RNUM: a one-dimensional array of numbers in the manner of NumPy — one
//! implementation for every runtime (native, interpreted, the web). Arrays
//! live by component name (not case-sensitive); another array is named by
//! its component (or its name). Methods change the array in place or
//! answer about it.
//!
//! Where the runtimes' former implementations differed, the desktop's
//! meaning was kept (`Rand` makes 1 number by default, `RandInt` includes
//! its upper bound, `Reciprocal` of 0 is infinity); the web's extra members
//! (`Create`, `Set`, `Get`, `Push`, the `Count` / `Sum` / `Mean` … read as
//! properties) are everyone's now.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::builtins::{random_index, random_unit};
use crate::{v_dbl, v_int, v_null, v_str, Value};

thread_local! {
    static ARRAYS: RefCell<HashMap<String, Vec<f64>>> = RefCell::new(HashMap::new());
}

/// Array `name`'s numbers (none: empty).
pub fn get(name: &str) -> Vec<f64> {
    ARRAYS.with(|m| m.borrow().get(&name.to_lowercase()).cloned().unwrap_or_default())
}

/// Whether an array is called `name`.
pub fn exists(name: &str) -> bool {
    ARRAYS.with(|m| m.borrow().contains_key(&name.to_lowercase()))
}

/// Array `name` becomes `values`.
pub fn set(name: &str, values: Vec<f64>) {
    ARRAYS.with(|m| {
        m.borrow_mut().insert(name.to_lowercase(), values);
    });
}

/// The numbers `v` names: an array (by its component or name), else a
/// comma-separated list of numbers ("35,25,20").
pub fn values_of(v: &Value) -> Vec<f64> {
    let s = v.to_string_val();
    if exists(&s) {
        return get(&s);
    }
    parse_list(&s)
}

/// "1, 2.5,3" → [1, 2.5, 3] (what isn't a number is skipped).
pub fn parse_list(s: &str) -> Vec<f64> {
    s.split(',').filter_map(|v| v.trim().parse::<f64>().ok()).collect()
}

/// The numbers as text: "1,2.5,3".
pub fn to_list(a: &[f64]) -> String {
    a.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")
}

fn cmp(a: &f64, b: &f64) -> std::cmp::Ordering {
    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
}

fn mean(a: &[f64]) -> f64 {
    if a.is_empty() {
        0.0
    } else {
        a.iter().sum::<f64>() / a.len() as f64
    }
}

/// The population variance (NumPy's default, ddof = 0).
fn variance(a: &[f64]) -> f64 {
    if a.is_empty() {
        return 0.0;
    }
    let m = mean(a);
    a.iter().map(|x| (x - m).powi(2)).sum::<f64>() / a.len() as f64
}

fn min(a: &[f64]) -> f64 {
    a.iter().cloned().fold(f64::INFINITY, f64::min)
}

fn max(a: &[f64]) -> f64 {
    a.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
}

fn arg(args: &[Value], i: usize) -> Option<&Value> {
    args.get(i)
}

fn f(args: &[Value], i: usize, default: f64) -> f64 {
    arg(args, i).map(Value::to_f64).unwrap_or(default)
}

fn n(args: &[Value], i: usize, default: i64) -> usize {
    arg(args, i).map(Value::to_i64).unwrap_or(default).max(0) as usize
}

/// The other operand of an element-wise operation: an array (by its
/// component or name) or a number.
enum Operand {
    Array(Vec<f64>),
    Scalar(f64),
}

fn operand(v: Option<&Value>, default: f64) -> Operand {
    match v {
        None => Operand::Scalar(default),
        Some(Value::Integer(_) | Value::Double(_) | Value::Boolean(_)) => Operand::Scalar(v.unwrap().to_f64()),
        Some(v) => {
            let s = v.to_string_val();
            if let Ok(x) = s.trim().parse::<f64>() {
                Operand::Scalar(x)
            } else if exists(&s) {
                Operand::Array(get(&s))
            } else {
                Operand::Scalar(v.to_f64())
            }
        }
    }
}

/// Array `name` with `op` applied element by element with `other` (a
/// number, or an array of the same length — else the array is unchanged).
fn arith(name: &str, other: Option<&Value>, default: f64, op: impl Fn(f64, f64) -> f64) {
    let a = get(name);
    let out = match operand(other, default) {
        Operand::Scalar(s) => a.iter().map(|&x| op(x, s)).collect(),
        Operand::Array(b) if b.len() == a.len() => a.iter().zip(&b).map(|(&x, &y)| op(x, y)).collect(),
        Operand::Array(_) => return,
    };
    set(name, out);
}

fn map(name: &str, op: impl Fn(f64) -> f64) {
    let a = get(name);
    set(name, a.into_iter().map(op).collect());
}

/// Whether `method` prints what it returns (the runtime prints it, as
/// PRINT would).
pub fn prints(method: &str) -> bool {
    matches!(method, "print" | "show")
}

/// Calls RNUM method `method` (lower case) on array `name`; `None` when
/// RNUM has no such method.
pub fn method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let a = || get(name);
    let done = |values: Vec<f64>| {
        set(name, values);
        v_null()
    };
    Some(match method {
        // --- Creation ---
        "create" | "new" | "init" => done(vec![0.0; n(args, 0, 0)]),
        "arange" => {
            let (start, stop, step) = (f(args, 0, 0.0), f(args, 1, 10.0), f(args, 2, 1.0));
            let mut vals = Vec::new();
            if step != 0.0 && step.is_finite() {
                let count = ((stop - start) / step).ceil();
                if count > 0.0 && count < 1e8 {
                    let mut cur = start;
                    while (step > 0.0 && cur < stop) || (step < 0.0 && cur > stop) {
                        vals.push(cur);
                        cur += step;
                    }
                }
            }
            done(vals)
        }
        "linspace" => {
            let (start, stop, count) = (f(args, 0, 0.0), f(args, 1, 1.0), n(args, 2, 50));
            // (as ndarray's: start + step × i)
            let step = if count > 1 { (stop - start) / (count - 1) as f64 } else { 0.0 };
            done((0..count).map(|i| start + step * i as f64).collect())
        }
        "zeros" => done(vec![0.0; n(args, 0, 10)]),
        "ones" => done(vec![1.0; n(args, 0, 10)]),
        "full" => done(vec![f(args, 1, 0.0); n(args, 0, 10)]),
        "fromlist" | "from_list" => done(parse_list(&arg(args, 0).map(Value::to_string_val).unwrap_or_default())),
        "set" | "setvalue" | "setitem" => {
            let (i, v) = (n(args, 0, 0), f(args, 1, 0.0));
            let mut a = a();
            if a.len() <= i {
                a.resize(i + 1, 0.0);
            }
            a[i] = v;
            done(a)
        }
        "get" | "getvalue" | "getitem" => v_dbl(a().get(n(args, 0, 0)).copied().unwrap_or(0.0)),
        "push" => {
            let mut a = a();
            a.push(f(args, 0, 0.0));
            done(a)
        }

        // --- Aggregation ---
        "sum" => v_dbl(a().iter().sum()),
        "mean" => v_dbl(mean(&a())),
        "min" => v_dbl(min(&a())),
        "max" => v_dbl(max(&a())),
        "std" => v_dbl(variance(&a()).sqrt()),
        "var" | "variance" => v_dbl(variance(&a())),
        "median" => {
            let mut s = a();
            if s.is_empty() {
                return Some(v_dbl(0.0));
            }
            s.sort_by(cmp);
            let mid = s.len() / 2;
            v_dbl(if s.len() % 2 == 0 { (s[mid - 1] + s[mid]) / 2.0 } else { s[mid] })
        }
        "argmin" => v_int(a().iter().enumerate().min_by(|x, y| cmp(x.1, y.1)).map(|(i, _)| i as i64).unwrap_or(0)),
        "argmax" => v_int(a().iter().enumerate().max_by(|x, y| cmp(x.1, y.1)).map(|(i, _)| i as i64).unwrap_or(0)),
        "count" => v_int(a().len() as i64),
        "ptp" => {
            let a = a();
            v_dbl(max(&a) - min(&a))
        }

        // --- Element-wise math (in place) ---
        "sin" => { map(name, f64::sin); v_null() }
        "cos" => { map(name, f64::cos); v_null() }
        "tan" => { map(name, f64::tan); v_null() }
        "asin" | "arcsin" => { map(name, f64::asin); v_null() }
        "acos" | "arccos" => { map(name, f64::acos); v_null() }
        "atan" | "arctan" => { map(name, f64::atan); v_null() }
        "sqrt" => { map(name, f64::sqrt); v_null() }
        "abs" => { map(name, f64::abs); v_null() }
        "exp" => { map(name, f64::exp); v_null() }
        "log" | "ln" => { map(name, f64::ln); v_null() }
        "log2" => { map(name, f64::log2); v_null() }
        "log10" => { map(name, f64::log10); v_null() }
        "floor" => { map(name, f64::floor); v_null() }
        "ceil" => { map(name, f64::ceil); v_null() }
        "round" => {
            let factor = 10f64.powi(arg(args, 0).map(Value::to_i64).unwrap_or(0) as i32);
            map(name, |x| (x * factor).round() / factor);
            v_null()
        }
        "sign" => { map(name, |x| if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 }); v_null() }
        "reciprocal" => { map(name, |x| if x != 0.0 { 1.0 / x } else { f64::INFINITY }); v_null() }
        "square" => { map(name, |x| x * x); v_null() }
        "negative" | "neg" => { map(name, |x| -x); v_null() }

        // --- Element-wise arithmetic with a number or another array ---
        "add" => { arith(name, arg(args, 0), 0.0, |x, y| x + y); v_null() }
        "subtract" | "sub" => { arith(name, arg(args, 0), 0.0, |x, y| x - y); v_null() }
        "multiply" | "mul" => { arith(name, arg(args, 0), 1.0, |x, y| x * y); v_null() }
        "divide" | "div" => { arith(name, arg(args, 0), 1.0, |x, y| if y != 0.0 { x / y } else { f64::NAN }); v_null() }
        "power" | "pow" => { arith(name, arg(args, 0), 2.0, f64::powf); v_null() }
        "mod" | "fmod" => { arith(name, arg(args, 0), 1.0, |x, y| x % y); v_null() }
        "clip" | "clamp" => {
            let (lo, hi) = (f(args, 0, f64::NEG_INFINITY), f(args, 1, f64::INFINITY));
            map(name, |x| x.max(lo).min(hi));
            v_null()
        }

        // --- Ordering / manipulation ---
        "sort" => {
            let mut v = a();
            v.sort_by(cmp);
            done(v)
        }
        "reverse" | "flip" => {
            let mut v = a();
            v.reverse();
            done(v)
        }
        "unique" => {
            let mut v = a();
            v.sort_by(cmp);
            v.dedup();
            done(v)
        }
        "shuffle" => {
            let mut v = a();
            for i in (1..v.len()).rev() {
                v.swap(i, random_index(i + 1));
            }
            done(v)
        }
        "append" | "concatenate" => {
            let mut v = a();
            v.extend(arg(args, 0).map(values_of).unwrap_or_default());
            done(v)
        }
        "slice" => {
            let v = a();
            let end = arg(args, 1).map(|e| e.to_i64().max(0) as usize).unwrap_or(v.len()).min(v.len());
            let start = n(args, 0, 0).min(end);
            done(v[start..end].to_vec())
        }
        "reshape" => v_null(), // (one-dimensional: nothing to do)

        // --- Cumulative ---
        "cumsum" => {
            let mut acc = 0.0;
            done(a().iter().map(|&x| { acc += x; acc }).collect())
        }
        "cumprod" => {
            let mut acc = 1.0;
            done(a().iter().map(|&x| { acc *= x; acc }).collect())
        }
        "diff" => {
            let v = a();
            if v.len() < 2 {
                return Some(v_null());
            }
            done(v.windows(2).map(|w| w[1] - w[0]).collect())
        }

        // --- Linear algebra ---
        "dot" => {
            let b = arg(args, 0).map(values_of).unwrap_or_default();
            v_dbl(a().iter().zip(&b).map(|(x, y)| x * y).sum())
        }
        "norm" => v_dbl(a().iter().map(|x| x * x).sum::<f64>().sqrt()),
        "normalize" => {
            let v = a();
            let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm > 0.0 {
                set(name, v.into_iter().map(|x| x / norm).collect());
            }
            v_null()
        }

        // --- Boolean / search ---
        "any" => v_int(i64::from(a().iter().any(|&x| x != 0.0))),
        "all" => v_int(i64::from(a().iter().all(|&x| x != 0.0))),
        "where" | "nonzero" => done(a().iter().enumerate().filter(|(_, &x)| x != 0.0).map(|(i, _)| i as f64).collect()),
        "searchsorted" => {
            let val = f(args, 0, 0.0);
            let v = a();
            v_int(v.iter().position(|&x| x >= val).unwrap_or(v.len()) as i64)
        }

        // --- Random (the program's generator: RANDOMIZE seeds it) ---
        "rand" | "random" => done((0..n(args, 0, 1)).map(|_| random_unit()).collect()),
        "randn" | "random_normal" | "normal" => {
            let (count, mu, sigma) = (n(args, 0, 1), f(args, 1, 0.0), f(args, 2, 1.0));
            // (Box-Muller)
            done((0..count)
                .map(|_| {
                    let u1 = random_unit().max(1e-10);
                    let u2 = random_unit();
                    mu + sigma * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
                })
                .collect())
        }
        "uniform" | "random_uniform" => {
            let (lo, hi, count) = (f(args, 0, 0.0), f(args, 1, 1.0), n(args, 2, 1));
            done((0..count).map(|_| random_unit() * (hi - lo) + lo).collect())
        }
        "randint" => {
            let lo = arg(args, 0).map(Value::to_i64).unwrap_or(0);
            let hi = arg(args, 1).map(Value::to_i64).unwrap_or(10);
            let count = n(args, 2, 1);
            done((0..count).map(|_| (lo + random_index((hi - lo + 1).max(1) as usize) as i64) as f64).collect())
        }
        "choice" => {
            let v = a();
            if v.is_empty() {
                return Some(v_dbl(0.0));
            }
            let count = n(args, 0, 1);
            if count == 1 {
                return Some(v_dbl(v[random_index(v.len())]));
            }
            done((0..count).map(|_| v[random_index(v.len())]).collect())
        }

        // --- Output ---
        "tolist" | "tostring" => v_str(&to_list(&a())),
        "print" | "show" => v_str(&format!("[{}]", a().iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))),
        "clear" => done(Vec::new()),
        // (a property read as a method: `PRINT a.Shape`)
        other => return get_prop(name, other),
    })
}

/// Reads RNUM property `prop` (lower case) of array `name`; `None` when
/// RNUM has no such property.
pub fn get_prop(name: &str, prop: &str) -> Option<Value> {
    let a = get(name);
    Some(match prop {
        "size" | "length" | "len" | "count" => v_int(a.len() as i64),
        "data" => v_str(&to_list(&a)),
        "shape" => v_str(&format!("({},)", a.len())),
        "ndim" => v_int(1),
        "dtype" => v_str("float64"),
        "sum" => v_dbl(a.iter().sum()),
        "mean" | "avg" | "average" => v_dbl(mean(&a)),
        "min" => v_dbl(min(&a)),
        "max" => v_dbl(max(&a)),
        "std" => v_dbl(variance(&a).sqrt()),
        _ => return None,
    })
}

/// Sets RNUM property `prop` (lower case); false when RNUM has no such
/// settable property.
pub fn set_prop(name: &str, prop: &str, val: &Value) -> bool {
    match prop {
        "data" => {
            set(name, parse_list(&val.to_string_val()));
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(name: &str, m: &str, args: &[Value]) -> Value {
        method(name, m, args).expect(m)
    }

    #[test]
    fn creation_and_aggregates() {
        call("ta", "arange", &[v_int(0), v_int(10), v_int(2)]);
        assert_eq!(get_prop("ta", "data").unwrap().to_string_val(), "0,2,4,6,8");
        assert_eq!(call("ta", "sum", &[]).to_f64(), 20.0);
        call("ta", "linspace", &[v_int(0), v_int(1), v_int(5)]);
        assert_eq!(get("ta"), vec![0.0, 0.25, 0.5, 0.75, 1.0]);
        call("ta", "arange", &[v_int(5), v_int(0), v_int(-2)]);
        assert_eq!(get("ta"), vec![5.0, 3.0, 1.0]);
        call("ta", "arange", &[v_int(0), v_int(5), v_int(0)]);
        assert!(get("ta").is_empty());
        assert_eq!(get_prop("ta", "shape").unwrap().to_string_val(), "(0,)");
    }

    #[test]
    fn arithmetic_with_arrays_and_numbers() {
        call("tb", "fromlist", &[v_str("1,2,3")]);
        call("tc", "fromlist", &[v_str("10,20,30")]);
        call("tb", "add", &[v_str("tc")]);
        assert_eq!(get("tb"), vec![11.0, 22.0, 33.0]);
        call("tb", "multiply", &[v_int(2)]);
        assert_eq!(get("tb"), vec![22.0, 44.0, 66.0]);
        call("tb", "divide", &[v_dbl(0.0)]);
        assert!(get("tb").iter().all(|x| x.is_nan()));
        assert_eq!(call("tc", "dot", &[v_str("tc")]).to_f64(), 1400.0);
        call("tc", "push", &[v_int(4)]);
        call("tc", "set", &[v_int(5), v_int(9)]);
        assert_eq!(get("tc"), vec![10.0, 20.0, 30.0, 4.0, 0.0, 9.0]);
        assert_eq!(call("tc", "get", &[v_int(1)]).to_f64(), 20.0);
        assert!(method("tc", "nonsense", &[]).is_none());
    }
}
