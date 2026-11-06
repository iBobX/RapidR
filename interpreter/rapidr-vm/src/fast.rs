//! The operations the VM's instructions share (docs/vm-fast-design.md):
//! each with the cases a typed program has written out — two Integers, two
//! Doubles, an Integer and a Double — and `Value`'s own operation for
//! anything else, so a plain `Add` and a fused `AddLK` give the same answer
//! bit for bit. And the pure maths builtins the VM calls itself.

use rapidr_value::numeric::{self, NumKind};
use rapidr_value::Value;

use crate::VmError;

/// Whole numbers a Double holds exactly: two Integers within compare as
/// integers (what `Value`'s comparison of their floats gives).
const EXACT: i64 = 1 << 53;

#[inline(always)]
fn exact(n: i64) -> bool {
    (-EXACT..=EXACT).contains(&n)
}

/// Whether a value holds nothing to free: a number, a boolean, Null.
#[inline(always)]
pub fn plain(v: &Value) -> bool {
    matches!(v, Value::Integer(_) | Value::Double(_) | Value::Boolean(_) | Value::Null | Value::Int64(_) | Value::UInt64(_))
}

/// `*slot = v`. `Value`'s drop is a call the VM's loop doesn't inline: the
/// old value is only dropped when it holds something (a string, an array,
/// an object); a number is just overwritten (forgetting it frees nothing:
/// it owns nothing).
#[inline(always)]
pub fn put(slot: &mut Value, v: Value) {
    // (a number over a number of its kind: its payload only)
    match (&mut *slot, &v) {
        (Value::Double(d), Value::Double(f)) => *d = *f,
        (Value::Integer(d), Value::Integer(n)) => *d = *n,
        _ => {
            let old = std::mem::replace(slot, v);
            if plain(&old) {
                std::mem::forget(old);
            } else {
                drop(old);
            }
        }
    }
}

/// `ToNum kind; Store` into `slot` (`StoreLocalNum`): a Double into a
/// DOUBLE and a 32-bit Integer into an INTEGER / LONG are already what
/// the conversion gives — stored as they are.
#[inline(always)]
pub fn store_num(slot: &mut Value, v: Value, kind: u8) -> Result<(), VmError> {
    const DOUBLE: u8 = NumKind::Double as u8;
    const LONG: u8 = NumKind::Long as u8;
    match (kind, &v) {
        (DOUBLE, Value::Double(_)) => put(slot, v),
        (LONG, Value::Integer(n)) if (i32::MIN as i64..=i32::MAX as i64).contains(n) => put(slot, v),
        _ => put(slot, to_num(v, kind)?),
    }
    Ok(())
}

/// Drops `v` (taken off the stack) as [`put`] drops the old value.
#[inline(always)]
pub fn discard(v: Value) {
    if plain(&v) {
        std::mem::forget(v);
    } else {
        drop(v);
    }
}

// The operations: the cases a typed program meets — two Integers, two
// Doubles — inline, everything else (an Integer with a Double, text,
// booleans, Null) in a function of its own out of the VM's loop (`#[cold]`:
// the loop's code stays small, its registers its own).

/// `a + b` (the generic `Add` without its in-place string append, which
/// gives the same text).
#[inline(always)]
pub fn add(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => Value::Integer(x.wrapping_add(*y)),
        (Value::Double(x), Value::Double(y)) => Value::Double(x + y),
        _ => add_other(a, b),
    }
}

#[cold]
#[inline(never)]
fn add_other(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Integer(x), Value::Double(y)) => Value::Double(*x as f64 + y),
        (Value::Double(x), Value::Integer(y)) => Value::Double(x + *y as f64),
        _ => a + b,
    }
}

/// `a - b`.
#[inline(always)]
pub fn sub(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => Value::Integer(x.wrapping_sub(*y)),
        (Value::Double(x), Value::Double(y)) => Value::Double(x - y),
        _ => sub_other(a, b),
    }
}

#[cold]
#[inline(never)]
fn sub_other(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Integer(x), Value::Double(y)) => Value::Double(*x as f64 - y),
        (Value::Double(x), Value::Integer(y)) => Value::Double(x - *y as f64),
        _ => a - b,
    }
}

/// `a * b`.
#[inline(always)]
pub fn mul(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => Value::Integer(x.wrapping_mul(*y)),
        (Value::Double(x), Value::Double(y)) => Value::Double(x * y),
        _ => mul_other(a, b),
    }
}

#[cold]
#[inline(never)]
fn mul_other(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Integer(x), Value::Double(y)) => Value::Double(*x as f64 * y),
        (Value::Double(x), Value::Integer(y)) => Value::Double(x * *y as f64),
        _ => a * b,
    }
}

/// `a / b`: always a float (by zero an infinity or NaN).
#[inline(always)]
pub fn div(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Double(x), Value::Double(y)) => Value::Double(x / y),
        (Value::Integer(x), Value::Integer(y)) => Value::Double(*x as f64 / *y as f64),
        _ => div_other(a, b),
    }
}

#[cold]
#[inline(never)]
fn div_other(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Integer(x), Value::Double(y)) => Value::Double(*x as f64 / y),
        (Value::Double(x), Value::Integer(y)) => Value::Double(x / *y as f64),
        _ => a / b,
    }
}

/// `a MOD b` (by zero, RapidQ's "Division by zero").
#[inline(always)]
pub fn modulo(a: &Value, b: &Value) -> Result<Value, VmError> {
    match (a, b) {
        // (Value::checked_mod takes both to 32 bits first; checked_imod
        // does it again: the same)
        (Value::Integer(x), Value::Integer(y)) if *y != 0 => match numeric::checked_imod(*x, *y) {
            Some(r) => Ok(Value::Integer(r)),
            None => modulo_other(a, b),
        },
        _ => modulo_other(a, b),
    }
}

#[cold]
#[inline(never)]
fn modulo_other(a: &Value, b: &Value) -> Result<Value, VmError> {
    a.checked_mod(b).map_err(|e| VmError::Runtime(e.into()))
}

// (the fused instructions' forms: every operation as a `Result`)

#[inline(always)]
pub fn try_add(a: &Value, b: &Value) -> Result<Value, VmError> {
    Ok(add(a, b))
}

#[inline(always)]
pub fn try_sub(a: &Value, b: &Value) -> Result<Value, VmError> {
    Ok(sub(a, b))
}

#[inline(always)]
pub fn try_mul(a: &Value, b: &Value) -> Result<Value, VmError> {
    Ok(mul(a, b))
}

#[inline(always)]
pub fn try_div(a: &Value, b: &Value) -> Result<Value, VmError> {
    Ok(div(a, b))
}

// The comparisons, as `Value::rp_lt` & co. make them: two whole numbers a
// Double holds exactly as integers; two numbers as floats, a NaN on either
// side "less" and "equal" at once (RapidQ's FCOM: `=`, `<`, `<=` true,
// `<>`, `>`, `>=` false); anything else by `Value`. Two Integers and two
// Doubles inline, the rest out of the loop.

/// The two numbers as floats, when both are numbers.
#[inline(always)]
fn floats(a: &Value, b: &Value) -> Option<(f64, f64)> {
    match (a, b) {
        (Value::Double(x), Value::Double(y)) => Some((*x, *y)),
        (Value::Integer(x), Value::Double(y)) => Some((*x as f64, *y)),
        (Value::Double(x), Value::Integer(y)) => Some((*x, *y as f64)),
        (Value::Integer(x), Value::Integer(y)) => Some((*x as f64, *y as f64)),
        _ => None,
    }
}

macro_rules! comparison {
    ($name:ident, $other:ident, $int:expr, $float:expr, $value:ident) => {
        #[inline(always)]
        pub fn $name(a: &Value, b: &Value) -> bool {
            match (a, b) {
                (Value::Integer(x), Value::Integer(y)) if exact(*x) && exact(*y) => $int(*x, *y),
                (Value::Double(x), Value::Double(y)) => $float(*x, *y),
                _ => $other(a, b),
            }
        }

        #[cold]
        #[inline(never)]
        fn $other(a: &Value, b: &Value) -> bool {
            match floats(a, b) {
                Some((x, y)) => $float(x, y),
                None => a.$value(b).to_bool(),
            }
        }
    };
}

comparison!(eq, eq_other, |x: i64, y: i64| x == y, |x: f64, y: f64| x.is_nan() || y.is_nan() || x == y, rp_eq);
comparison!(ne, ne_other, |x: i64, y: i64| x != y, |x: f64, y: f64| !(x.is_nan() || y.is_nan()) && x != y, rp_ne);
comparison!(lt, lt_other, |x: i64, y: i64| x < y, |x: f64, y: f64| x.is_nan() || y.is_nan() || x < y, rp_lt);
comparison!(le, le_other, |x: i64, y: i64| x <= y, |x: f64, y: f64| x.is_nan() || y.is_nan() || x <= y, rp_le);
comparison!(gt, gt_other, |x: i64, y: i64| x > y, |x: f64, y: f64| !(x.is_nan() || y.is_nan()) && x > y, rp_gt);
comparison!(ge, ge_other, |x: i64, y: i64| x >= y, |x: f64, y: f64| !(x.is_nan() || y.is_nan()) && x >= y, rp_ge);

/// Whether a FOR counter is one of RapidR's 64-bit kinds (its step then
/// goes through [`for_next_wide`]).
#[inline(always)]
pub fn wide(v: &Value) -> bool {
    matches!(v, Value::Int64(_) | Value::UInt64(_))
}

/// A 64-bit FOR counter's step (`ForStep`): `var + step`, or `None` when
/// it would pass its type's limit — the loop ends there
/// (`rapidr_value::wide::for_step`).
#[cold]
#[inline(never)]
pub fn for_next_wide(var: &Value, step: &Value) -> Option<Value> {
    rapidr_value::wide::for_step(var, step)
}

/// `ToNum operand` on `v`: the conversion for a store into a declared
/// numeric type, or `ARG_ROUND`'s rounding. A value that already has the
/// kind isn't copied.
#[inline(always)]
pub fn to_num(v: Value, operand: u8) -> Result<Value, VmError> {
    const DOUBLE: u8 = NumKind::Double as u8;
    const LONG: u8 = NumKind::Long as u8;
    match (operand, v) {
        (DOUBLE, v @ Value::Double(_)) => Ok(v),
        // (a 32-bit Integer into an INTEGER / LONG: already what it holds)
        (LONG, Value::Integer(n)) if (i32::MIN as i64..=i32::MAX as i64).contains(&n) => Ok(Value::Integer(n)),
        (operand, v) => to_num_other(v, operand),
    }
}

#[cold]
#[inline(never)]
fn to_num_other(v: Value, operand: u8) -> Result<Value, VmError> {
    if operand == numeric::ARG_ROUND {
        return Ok(match v {
            Value::Double(f) => Value::Double(f.round_ties_even()),
            other => other,
        });
    }
    let kind = NumKind::from_code(operand).ok_or_else(|| VmError::BadOperand)?;
    Ok(match (kind, v) {
        (NumKind::Double, v @ Value::Double(_)) => v,
        (NumKind::Long | NumKind::Dword, Value::Integer(n)) => Value::Integer(numeric::int32_of(n)),
        (kind, v) => numeric::convert(&v, kind),
    })
}

/// An array index as `pop_indices` reads it.
#[inline(always)]
pub fn index(v: &Value) -> i64 {
    match v {
        Value::Integer(n) => *n,
        other => other.to_i64(),
    }
}

/// `arr(i)`, one index: an element of a one-dimension array found
/// directly, everything else as `AGet` does it.
#[inline(always)]
pub fn aget1(arr: &Value, i: i64) -> Result<Value, VmError> {
    if let Value::Array(a) = arr {
        let a = a.borrow();
        if let [(lo, hi)] = a.bounds[..] {
            if lo <= i && i <= hi {
                if let Some(v) = a.data.get(i.wrapping_sub(lo) as usize) {
                    return Ok(v.clone());
                }
            }
        }
    }
    aget1_other(arr, i)
}

#[cold]
#[inline(never)]
fn aget1_other(arr: &Value, i: i64) -> Result<Value, VmError> {
    match arr {
        Value::Array(a) => a.borrow().get(&[i]).map_err(VmError::Runtime),
        // (legacy: a string indexed as a comma-separated list)
        other => Ok(other.rp_index(&Value::Integer(i))),
    }
}

/// `arr(i) = v`, one index.
#[inline(always)]
pub fn aset1(arr: &Value, i: i64, v: Value) -> Result<(), VmError> {
    if let Value::Array(a) = arr {
        let mut a = a.borrow_mut();
        if let [(lo, hi)] = a.bounds[..] {
            if lo <= i && i <= hi {
                if let Some(slot) = a.data.get_mut(i.wrapping_sub(lo) as usize) {
                    put(slot, v);
                    return Ok(());
                }
            }
        }
    }
    aset1_other(arr, i, v)
}

#[cold]
#[inline(never)]
fn aset1_other(arr: &Value, i: i64, v: Value) -> Result<(), VmError> {
    match arr {
        Value::Array(a) => a.borrow_mut().set(&[i], v).map_err(VmError::Runtime),
        _ => Err(VmError::Runtime("assigning to an element of a variable that is not an array (DIM it with a size first)".into())),
    }
}

// ---------- pure maths builtins (docs/vm-fast-design.md) ----------

type Pure = fn(&Value) -> Value;

/// The builtins the VM calls itself when its host lets it
/// ([`crate::Host::DIRECT_MATHS`]): one argument, no state, no events, no
/// error — the very functions both hosts call for these names. A name's
/// code in [`rapidr_bytecode::Prepared::builtins`] is its index here + 1.
const DIRECT: [(&str, Pure); 22] = {
    use rapidr_value::builtins::*;
    [
        ("abs", rp_abs),
        ("int", rp_int),
        ("fix", rp_fix),
        ("frac", rp_frac),
        ("sgn", rp_sgn),
        ("sqr", rp_sqr),
        ("sin", rp_sin),
        ("cos", rp_cos),
        ("tan", rp_tan),
        ("atn", rp_atn),
        ("atan", rp_atn),
        ("asin", rp_asin),
        ("acos", rp_acos),
        ("log", rp_log),
        ("exp", rp_exp),
        ("ceil", rp_ceil),
        ("floor", rp_floor),
        ("round", rp_round),
        ("cint", rp_cint),
        ("clng", rp_clng),
        ("cdbl", rp_cdbl),
        ("csng", rp_csng),
    ]
};

type Pure2 = fn(&Value, &Value) -> Value;

/// The two-argument ones (RapidQ's SHL and SHR): codes from [`TWO`] on.
const DIRECT2: [(&str, Pure2); 2] = [("shl", rapidr_value::rp_shl), ("shr", rapidr_value::rp_shr)];

/// The first code of [`DIRECT2`]'s builtins.
const TWO: u8 = 128;

/// A pool string's builtin code: the [`DIRECT`] builtin it names (its
/// index + 1), a [`DIRECT2`] one ([`TWO`] + its index), or 0.
pub fn direct_code(name: &str) -> u8 {
    if !rapidr_bytecode::builtins::is_builtin(name) {
        return 0;
    }
    let key = rapidr_bytecode::builtins::builtin_key(name);
    if let Some(i) = DIRECT.iter().position(|(k, _)| *k == key) {
        return i as u8 + 1;
    }
    DIRECT2.iter().position(|(k, _)| *k == key).map_or(0, |i| TWO + i as u8)
}

/// Runs direct builtin `code` (non-zero, from [`direct_code`]) on its
/// arguments, as the hosts do (a missing argument is Null).
#[inline(always)]
pub fn call_direct(code: u8, args: &[Value]) -> Value {
    let null = Value::Null;
    let arg = |i: usize| args.get(i).unwrap_or(&null);
    if code >= TWO {
        return DIRECT2[(code - TWO) as usize].1(arg(0), arg(1));
    }
    DIRECT[code as usize - 1].1(arg(0))
}

/// The direct builtin for a name, if it is one with one argument (tests
/// compare it with the hosts).
pub fn direct_builtin(name: &str) -> Option<fn(&Value) -> Value> {
    let code = direct_code(name);
    (code != 0 && code < TWO).then(|| DIRECT[code as usize - 1].1)
}

/// The direct builtin for a name, if it is one with two arguments.
pub fn direct_builtin2(name: &str) -> Option<fn(&Value, &Value) -> Value> {
    let code = direct_code(name);
    (code >= TWO).then(|| DIRECT2[(code - TWO) as usize].1)
}

/// The names of the direct builtins.
pub fn direct_names() -> impl Iterator<Item = &'static str> {
    DIRECT.iter().map(|(k, _)| *k).chain(DIRECT2.iter().map(|(k, _)| *k))
}
