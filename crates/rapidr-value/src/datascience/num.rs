//! RNUM: a one-dimensional array of numbers, in the manner of NumPy.
//!
//! Methods change the array in place (`a.sqrt`) or answer a value
//! (`a.sum`). Another array is named by its component's name
//! (`a.add "b"`, `a.dot("b")`); a number is a scalar (`a.add 2`).

use std::cell::RefCell;
use std::collections::HashMap;

use super::{arg_f, arg_i, arg_s, key, num_text, parse_list, parse_num, random_index, random_unit, Host};
use crate::{v_dbl, v_int, v_null, v_str, Value};

thread_local! {
    static ARRAYS: RefCell<HashMap<String, Vec<f64>>> = RefCell::new(HashMap::new());
}

/// Array `name`'s numbers (empty if it has none yet).
pub fn data(name: &str) -> Vec<f64> {
    ARRAYS.with(|m| m.borrow().get(&key(name)).cloned().unwrap_or_default())
}

/// Whether `name` is an array that has numbers (or was made empty).
pub fn exists(name: &str) -> bool {
    ARRAYS.with(|m| m.borrow().contains_key(&key(name)))
}

/// What a chart's data argument names: array `name`, or numbers written in
/// place (`"35,25,20"`).
pub fn series_data(name: &str) -> Vec<f64> {
    if exists(name) {
        data(name)
    } else {
        parse_list(name)
    }
}

pub fn set_data(name: &str, values: Vec<f64>) {
    ARRAYS.with(|m| {
        m.borrow_mut().insert(key(name), values);
    });
}

/// Every element through `f`, in place.
fn map(name: &str, f: impl FnMut(f64) -> f64) -> Value {
    set_data(name, data(name).into_iter().map(f).collect());
    v_null()
}

/// The other operand of element-wise arithmetic: a number, or another
/// array named by its component's name.
enum Operand {
    Scalar(f64),
    Array(Vec<f64>),
}

fn operand(v: Option<&Value>) -> Operand {
    match v {
        None => Operand::Scalar(0.0),
        Some(Value::Integer(n)) => Operand::Scalar(*n as f64),
        Some(Value::Double(d)) => Operand::Scalar(*d),
        Some(Value::Boolean(b)) => Operand::Scalar(if *b { -1.0 } else { 0.0 }),
        Some(other) => {
            let s = other.to_string_val();
            match parse_num(&s) {
                Some(x) => Operand::Scalar(x),
                None => Operand::Array(data(&s)),
            }
        }
    }
}

/// `a OP other`, element by element. Arrays of different lengths leave `a`
/// as it was (NumPy refuses them).
fn arith(name: &str, other: Option<&Value>, f: impl Fn(f64, f64) -> f64) -> Value {
    let a = data(name);
    match operand(other) {
        Operand::Scalar(s) => set_data(name, a.into_iter().map(|x| f(x, s)).collect()),
        Operand::Array(b) => {
            if a.len() == b.len() {
                set_data(name, a.iter().zip(&b).map(|(x, y)| f(*x, *y)).collect());
            }
        }
    }
    v_null()
}

fn sorted(a: &[f64]) -> Vec<f64> {
    let mut s = a.to_vec();
    s.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    s
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

fn median(a: &[f64]) -> f64 {
    if a.is_empty() {
        return 0.0;
    }
    let s = sorted(a);
    let mid = s.len() / 2;
    if s.len().is_multiple_of(2) {
        (s[mid - 1] + s[mid]) / 2.0
    } else {
        s[mid]
    }
}

fn min(a: &[f64]) -> f64 {
    a.iter().cloned().fold(f64::INFINITY, f64::min)
}

fn max(a: &[f64]) -> f64 {
    a.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
}

/// The array as text: `1,2.5,3`.
pub fn list_text(a: &[f64]) -> String {
    a.iter().map(|x| num_text(*x)).collect::<Vec<_>>().join(",")
}

/// Calls RNUM `name`'s `method` (lowercase).
pub fn num_method(name: &str, method: &str, args: &[Value], host: &dyn Host) -> Value {
    match method {
        // --- Creation ---
        "create" | "new" | "init" => {
            let n = arg_i(args, 0, 0).max(0) as usize;
            set_data(name, vec![0.0; n]);
            v_null()
        }
        "arange" => {
            let start = arg_f(args, 0, 0.0);
            let stop = arg_f(args, 1, 10.0);
            let step = arg_f(args, 2, 1.0);
            let mut v = Vec::new();
            if step != 0.0 && step.is_finite() {
                // (by index, so 0.1 steps don't drift; at most 10 million)
                let n = ((stop - start) / step).ceil();
                if n.is_finite() && n > 0.0 {
                    v = (0..(n as usize).min(10_000_000)).map(|i| start + step * i as f64).collect();
                }
            }
            set_data(name, v);
            v_null()
        }
        "linspace" => {
            let start = arg_f(args, 0, 0.0);
            let stop = arg_f(args, 1, 1.0);
            let n = arg_i(args, 2, 50).clamp(0, 10_000_000) as usize;
            let v = match n {
                0 => Vec::new(),
                1 => vec![start],
                _ => (0..n).map(|i| start + (stop - start) * i as f64 / (n - 1) as f64).collect(),
            };
            set_data(name, v);
            v_null()
        }
        "zeros" => {
            set_data(name, vec![0.0; arg_i(args, 0, 10).clamp(0, 10_000_000) as usize]);
            v_null()
        }
        "ones" => {
            set_data(name, vec![1.0; arg_i(args, 0, 10).clamp(0, 10_000_000) as usize]);
            v_null()
        }
        "full" => {
            let n = arg_i(args, 0, 10).clamp(0, 10_000_000) as usize;
            set_data(name, vec![arg_f(args, 1, 0.0); n]);
            v_null()
        }
        "fromlist" | "from_list" => {
            set_data(name, parse_list(&arg_s(args, 0)));
            v_null()
        }

        // --- Indexing ---
        "set" | "setvalue" | "setitem" => {
            // (an index past the end grows the array with zeros)
            let i = arg_i(args, 0, 0);
            let x = arg_f(args, 1, 0.0);
            let mut a = data(name);
            if i >= 0 && (i as usize) < 10_000_000 {
                let i = i as usize;
                if a.len() <= i {
                    a.resize(i + 1, 0.0);
                }
                a[i] = x;
                set_data(name, a);
            }
            v_null()
        }
        "get" | "getvalue" | "getitem" | "at" => {
            let i = arg_i(args, 0, 0);
            let a = data(name);
            v_dbl(if i >= 0 { a.get(i as usize).copied().unwrap_or(0.0) } else { 0.0 })
        }
        "push" => {
            let mut a = data(name);
            a.push(arg_f(args, 0, 0.0));
            set_data(name, a);
            v_null()
        }

        // --- Aggregation ---
        "sum" => v_dbl(data(name).iter().sum()),
        "mean" | "avg" | "average" => v_dbl(mean(&data(name))),
        "min" => v_dbl(min(&data(name))),
        "max" => v_dbl(max(&data(name))),
        "std" => v_dbl(variance(&data(name)).sqrt()),
        "var" | "variance" => v_dbl(variance(&data(name))),
        "median" => v_dbl(median(&data(name))),
        "argmin" => {
            let a = data(name);
            v_int(a.iter().enumerate().min_by(|x, y| x.1.partial_cmp(y.1).unwrap_or(std::cmp::Ordering::Equal)).map_or(0, |(i, _)| i as i64))
        }
        "argmax" => {
            let a = data(name);
            // (the first of equal maxima, as NumPy's)
            let mut best: Option<(usize, f64)> = None;
            for (i, x) in a.iter().enumerate() {
                if best.is_none_or(|(_, b)| *x > b) {
                    best = Some((i, *x));
                }
            }
            v_int(best.map_or(0, |(i, _)| i as i64))
        }
        "count" => v_int(data(name).len() as i64),
        "ptp" => {
            let a = data(name);
            v_dbl(if a.is_empty() { 0.0 } else { max(&a) - min(&a) })
        }

        // --- Element-wise math (in place) ---
        "sin" => map(name, f64::sin),
        "cos" => map(name, f64::cos),
        "tan" => map(name, f64::tan),
        "asin" | "arcsin" => map(name, f64::asin),
        "acos" | "arccos" => map(name, f64::acos),
        "atan" | "arctan" => map(name, f64::atan),
        "sqrt" => map(name, f64::sqrt),
        "abs" => map(name, f64::abs),
        "exp" => map(name, f64::exp),
        "log" | "ln" => map(name, f64::ln),
        "log2" => map(name, f64::log2),
        "log10" => map(name, f64::log10),
        "floor" => map(name, f64::floor),
        "ceil" => map(name, f64::ceil),
        "round" => {
            let factor = 10f64.powi(arg_i(args, 0, 0).clamp(-15, 15) as i32);
            map(name, move |x| (x * factor).round() / factor)
        }
        "sign" => map(name, |x| if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 }),
        "reciprocal" => map(name, |x| if x != 0.0 { 1.0 / x } else { f64::INFINITY }),
        "square" => map(name, |x| x * x),
        "negative" | "neg" => map(name, |x| -x),

        // --- Element-wise arithmetic with another array or a number ---
        "add" => arith(name, args.first(), |a, b| a + b),
        "subtract" | "sub" => arith(name, args.first(), |a, b| a - b),
        "multiply" | "mul" => arith(name, args.first(), |a, b| a * b),
        "divide" | "div" => arith(name, args.first(), |a, b| if b != 0.0 { a / b } else { f64::NAN }),
        "power" | "pow" => {
            if args.is_empty() {
                map(name, |x| x * x)
            } else {
                arith(name, args.first(), f64::powf)
            }
        }
        "mod" | "fmod" => arith(name, args.first(), |a, b| if b != 0.0 { a % b } else { f64::NAN }),
        "clip" | "clamp" => {
            let lo = arg_f(args, 0, f64::NEG_INFINITY);
            let hi = arg_f(args, 1, f64::INFINITY);
            map(name, move |x| x.max(lo).min(hi))
        }

        // --- Ordering / manipulation ---
        "sort" => {
            set_data(name, sorted(&data(name)));
            v_null()
        }
        "reverse" | "flip" => {
            let mut a = data(name);
            a.reverse();
            set_data(name, a);
            v_null()
        }
        "unique" => {
            let mut a = sorted(&data(name));
            a.dedup();
            set_data(name, a);
            v_null()
        }
        "shuffle" => {
            let mut a = data(name);
            for i in (1..a.len()).rev() {
                a.swap(i, random_index(i + 1));
            }
            set_data(name, a);
            v_null()
        }
        "append" | "concatenate" => {
            // (another array, or numbers: `a.append "4,5"`, `a.append 6`)
            let mut a = data(name);
            match operand(args.first()) {
                Operand::Scalar(x) if !args.is_empty() => a.push(x),
                Operand::Scalar(_) => {}
                Operand::Array(b) => {
                    let s = arg_s(args, 0);
                    if b.is_empty() && !exists(&s) {
                        a.extend(parse_list(&s));
                    } else {
                        a.extend(b);
                    }
                }
            }
            set_data(name, a);
            v_null()
        }
        "slice" => {
            let a = data(name);
            let end = (arg_i(args, 1, a.len() as i64).max(0) as usize).min(a.len());
            let start = (arg_i(args, 0, 0).max(0) as usize).min(end);
            set_data(name, a[start..end].to_vec());
            v_null()
        }
        "reshape" => v_null(), // (one dimension: nothing to reshape)

        // --- Cumulative ---
        "cumsum" => {
            let mut acc = 0.0;
            map(name, move |x| {
                acc += x;
                acc
            })
        }
        "cumprod" => {
            let mut acc = 1.0;
            map(name, move |x| {
                acc *= x;
                acc
            })
        }
        "diff" => {
            let a = data(name);
            if a.len() >= 2 {
                set_data(name, a.windows(2).map(|w| w[1] - w[0]).collect());
            }
            v_null()
        }

        // --- Dot product / linear algebra ---
        "dot" => {
            let a = data(name);
            let b = match operand(args.first()) {
                Operand::Scalar(s) => vec![s; a.len()],
                Operand::Array(b) => b,
            };
            v_dbl(a.iter().zip(&b).map(|(x, y)| x * y).sum())
        }
        "norm" => v_dbl(data(name).iter().map(|x| x * x).sum::<f64>().sqrt()),
        "normalize" => {
            let n = data(name).iter().map(|x| x * x).sum::<f64>().sqrt();
            if n > 0.0 {
                map(name, move |x| x / n);
            }
            v_null()
        }

        // --- Boolean / search ---
        "any" => v_int(data(name).iter().any(|x| *x != 0.0) as i64),
        "all" => v_int(data(name).iter().all(|x| *x != 0.0) as i64),
        "where" | "nonzero" => {
            let a = data(name);
            set_data(name, a.iter().enumerate().filter(|(_, x)| **x != 0.0).map(|(i, _)| i as f64).collect());
            v_null()
        }
        "searchsorted" => {
            let x = arg_f(args, 0, 0.0);
            v_int(data(name).partition_point(|v| *v < x) as i64)
        }

        // --- Random (instance methods) ---
        "rand" | "random" => {
            let n = arg_i(args, 0, 1).clamp(0, 10_000_000) as usize;
            set_data(name, (0..n).map(|_| random_unit()).collect());
            v_null()
        }
        "randn" | "random_normal" | "normal" => {
            let n = arg_i(args, 0, 1).clamp(0, 10_000_000) as usize;
            let m = arg_f(args, 1, 0.0);
            let sd = arg_f(args, 2, 1.0);
            // (Box–Muller)
            let v = (0..n)
                .map(|_| {
                    let u1 = random_unit().max(1e-12);
                    let u2 = random_unit();
                    m + sd * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
                })
                .collect();
            set_data(name, v);
            v_null()
        }
        "uniform" | "random_uniform" => {
            let lo = arg_f(args, 0, 0.0);
            let hi = arg_f(args, 1, 1.0);
            let n = arg_i(args, 2, 1).clamp(0, 10_000_000) as usize;
            set_data(name, (0..n).map(|_| lo + random_unit() * (hi - lo)).collect());
            v_null()
        }
        "randint" => {
            // (low to high, both included)
            let lo = arg_i(args, 0, 0);
            let hi = arg_i(args, 1, 10).max(lo);
            let n = arg_i(args, 2, 1).clamp(0, 10_000_000) as usize;
            let span = (hi - lo + 1).max(1) as usize;
            set_data(name, (0..n).map(|_| (lo + random_index(span) as i64) as f64).collect());
            v_null()
        }
        "choice" => {
            // (one: the value; more: the array becomes them)
            let a = data(name);
            if a.is_empty() {
                return v_dbl(0.0);
            }
            let n = arg_i(args, 0, 1).clamp(0, 10_000_000) as usize;
            if n == 1 {
                return v_dbl(a[random_index(a.len())]);
            }
            set_data(name, (0..n).map(|_| a[random_index(a.len())]).collect());
            v_null()
        }

        // --- Output ---
        "tolist" | "tostring" => v_str(&list_text(&data(name))),
        "print" | "show" => {
            let s = format!("[{}]", data(name).iter().map(|x| num_text(*x)).collect::<Vec<_>>().join(", "));
            host.print(&s);
            v_str(&s)
        }
        "clear" => {
            set_data(name, Vec::new());
            v_null()
        }

        _ => {
            // (a property read the way methods are: `PRINT a.Shape`)
            let v = num_get_prop(name, method);
            if matches!(v, Value::Null) {
                host.warn(&format!("[WARN] RNum.{method}() not implemented"));
            }
            v
        }
    }
}

/// Reads RNUM `name`'s property `prop` (lowercase); Null if it has none.
pub fn num_get_prop(name: &str, prop: &str) -> Value {
    match prop {
        "size" | "length" | "len" | "count" => v_int(data(name).len() as i64),
        "data" => v_str(&list_text(&data(name))),
        "shape" => v_str(&format!("({},)", data(name).len())),
        "ndim" => v_int(1),
        "dtype" => v_str("float64"),
        "sum" => v_dbl(data(name).iter().sum()),
        "mean" | "avg" | "average" => v_dbl(mean(&data(name))),
        "min" => v_dbl(min(&data(name))),
        "max" => v_dbl(max(&data(name))),
        "std" => v_dbl(variance(&data(name)).sqrt()),
        _ => v_null(),
    }
}

/// Sets RNUM `name`'s property `prop` (lowercase); false if it has none.
pub fn num_set_prop(name: &str, prop: &str, val: &Value) -> bool {
    match prop {
        "data" => {
            set_data(name, parse_list(&val.to_string_val()));
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_host::TestHost;
    use super::*;

    fn call(name: &str, m: &str, args: &[Value]) -> Value {
        num_method(name, m, args, &TestHost::default())
    }

    #[test]
    fn creation_and_aggregates() {
        call("A", "arange", &[v_int(0), v_int(10), v_int(1)]);
        assert_eq!(list_text(&data("a")), "0,1,2,3,4,5,6,7,8,9");
        assert_eq!(call("a", "sum", &[]).to_f64(), 45.0);
        assert_eq!(call("a", "mean", &[]).to_f64(), 4.5);
        call("a", "arange", &[v_int(0), v_int(1), v_dbl(0.1)]);
        assert_eq!(list_text(&data("a")), "0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9");
        call("a", "arange", &[v_int(5), v_int(0), v_int(-2)]);
        assert_eq!(list_text(&data("a")), "5,3,1");
        call("a", "linspace", &[v_int(0), v_int(1), v_int(5)]);
        assert_eq!(list_text(&data("a")), "0,0.25,0.5,0.75,1");
    }

    #[test]
    fn shape_is_a_property_read_as_a_method() {
        call("s", "fromlist", &[v_str("1,2,3")]);
        assert_eq!(call("s", "shape", &[]).to_string_val(), "(3,)");
        assert_eq!(num_get_prop("s", "size").to_i64(), 3);
    }

    #[test]
    fn arithmetic_with_arrays_and_numbers() {
        call("x", "fromlist", &[v_str("1,2,3")]);
        call("y", "fromlist", &[v_str("10,20,30")]);
        call("x", "add", &[v_str("Y")]);
        assert_eq!(list_text(&data("x")), "11,22,33");
        call("x", "multiply", &[v_int(2)]);
        assert_eq!(list_text(&data("x")), "22,44,66");
        call("x", "power", &[v_int(2)]);
        assert_eq!(list_text(&data("x")), "484,1936,4356");
        assert_eq!(call("y", "dot", &[v_str("y")]).to_f64(), 1400.0);
        call("x", "add", &[v_str("nothing")]);
        assert_eq!(list_text(&data("x")), "484,1936,4356");
    }

    #[test]
    fn get_set_push() {
        call("g", "create", &[v_int(2)]);
        call("g", "set", &[v_int(3), v_dbl(1.5)]);
        call("g", "push", &[v_int(7)]);
        assert_eq!(list_text(&data("g")), "0,0,0,1.5,7");
        assert_eq!(call("g", "get", &[v_int(3)]).to_f64(), 1.5);
    }
}
