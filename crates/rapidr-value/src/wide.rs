//! INT64 and UINT64, RapidR's 64-bit integers.
//!
//! A `Value::Integer` already holds 64 bits, and `+ - *` on two of them
//! already wrap at 64 bits. What the classic types do differently is
//! *showing* and a few operators: PRINT, STR$ and HEX$ show a whole number
//! as a 32-bit one, `\`, MOD and AND/OR/XOR/NOT/SHL/SHR work on 32 bits
//! (docs/rapidq-ground-truth.md). That stays as it is for every expression
//! without an INT64 or UINT64 in it; `rapidr_ast::numeric` rewrites an
//! expression with one into these builtins, the same for the interpreter,
//! native builds and the web:
//!
//! * `__str64(v, mode)`: a PRINT item or STR$ of a 64-bit value, in full
//!   (mode bit 0: unsigned; bit 1: STR$, else PRINT). A float (an INT64
//!   times a DOUBLE is a DOUBLE) shows as PRINT / STR$ show it.
//! * `__hex64(v)`: HEX$ of all 64 bits, in 16 digits.
//! * `__idiv64(a, b, mode)`, `__mod64(a, b, mode)`, `__cmp64(a, b, mode)`:
//!   `\`, MOD and the comparisons, worked out exactly (in 128 bits) with
//!   each side's own sign (mode bit 0: `a` is UINT64, bit 1: `b` is), the
//!   quotient and remainder then wrapped to 64 bits. `__cmp64` is -1, 0 or 1.
//! * `__bits64(op, a, b)`: AND (0), OR (1), XOR (2), NOT (3, of `a`), SHL
//!   (4), SHR (5, a logical shift; the count's low 6 bits) on 64 bits (`op`
//!   + 8: the result is a UINT64).
//!
//! A UINT64 keeps its bits in the `i64` (18446744073709551615 is held as
//! -1); only showing and the operators above read them unsigned.

use crate::{format, Value};

/// `v` as a 64-bit integer: an integer as it is (its bits: -1 stored into a
/// UINT64 is 18446744073709551615), TRUE -1, a float truncated toward zero
/// (out of the type's range, or NaN: an INT64 gets -9223372036854775808, a
/// UINT64 0), text read as VAL reads it, exactly to the last digit.
pub fn of(v: &Value, unsigned: bool) -> i64 {
    match v {
        Value::Integer(n) => *n,
        Value::Boolean(b) => -(*b as i64),
        Value::Double(f) => from_f64(*f, unsigned),
        Value::String(s) => parse(s.as_str(), unsigned),
        Value::Null | Value::Array(_) | Value::Object(_) => 0,
    }
}

/// A float stored into an INT64 (`unsigned` false) or a UINT64: truncated
/// toward zero; out of range or NaN, -9223372036854775808 (INT64) or 0
/// (UINT64).
pub fn from_f64(f: f64, unsigned: bool) -> i64 {
    let t = f.trunc();
    if unsigned {
        if t.is_nan() || !(0.0..18446744073709551616.0).contains(&t) {
            0
        } else {
            t as u64 as i64
        }
    } else if t.is_nan() || !(-9223372036854775808.0..9223372036854775808.0).contains(&t) {
        i64::MIN
    } else {
        t as i64
    }
}

/// Text as a 64-bit integer, VAL's way (spaces ignored, a sign, digits,
/// the rest ignored), the whole number exact: "18446744073709551615" is
/// every bit set. With an exponent ("1E3") or beyond 64 bits, VAL's
/// number, then [`from_f64`].
fn parse(s: &str, unsigned: bool) -> i64 {
    let t: String = s.chars().filter(|c| *c != ' ').collect();
    let (neg, body) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(&t)),
    };
    let end = body.find(|c: char| !c.is_ascii_digit()).unwrap_or(body.len());
    let (digits, rest) = body.split_at(end);
    let after_fraction = rest.strip_prefix(['.', ',']).map_or(rest, |r| r.trim_start_matches(|c: char| c.is_ascii_digit()));
    let exponent = after_fraction.starts_with(['e', 'E']);
    if !digits.is_empty() && !exponent && digits.len() <= 30 {
        if let Ok(n) = digits.parse::<u128>() {
            let n = if neg { -(n as i128) } else { n as i128 };
            if (i64::MIN as i128..=u64::MAX as i128).contains(&n) {
                // (the low 64 bits: a UINT64's above 2^63 too)
                return n as i64;
            }
        }
    }
    from_f64(crate::builtins::rp_val(&Value::String(s.into())).to_f64(), unsigned)
}

/// A 64-bit value's number, exactly: a UINT64's bits read unsigned.
fn exact(v: &Value, unsigned: bool) -> i128 {
    let n = of(v, unsigned);
    if unsigned {
        n as u64 as i128
    } else {
        n as i128
    }
}

/// A whole number's text, in full.
pub fn text(n: i64, unsigned: bool) -> String {
    if unsigned {
        (n as u64).to_string()
    } else {
        n.to_string()
    }
}

/// `__str64(v, mode)` (see the module docs).
pub fn str64(v: &Value, mode: i64) -> Value {
    let unsigned = mode & 1 != 0;
    let str_func = mode & 2 != 0;
    match v {
        Value::Integer(n) => Value::String(text(*n, unsigned).into()),
        _ if str_func => crate::builtins::rp_str(v),
        _ => Value::String(format::print_text(v).into()),
    }
}

/// `__hex64(v)`: HEX$ of all 64 bits, 16 digits (as HEX$ shows 32 bits
/// in 8): `HEX$(-1)` of an INT64 is FFFFFFFFFFFFFFFF.
pub fn hex64(v: &Value) -> Value {
    Value::String(format!("{:016X}", of(v, false) as u64).into())
}

/// `a \ b` (`modulo` false) or `a MOD b` on 64 bits.
pub fn div(a: &Value, b: &Value, mode: i64, modulo: bool) -> Result<Value, &'static str> {
    let x = exact(a, mode & 1 != 0);
    let y = exact(b, mode & 2 != 0);
    if y == 0 {
        return Err(crate::numeric::DIVISION_BY_ZERO);
    }
    let r = if modulo { x % y } else { x / y };
    Ok(Value::Integer(r as i64))
}

/// `__cmp64(a, b, mode)`: -1, 0 or 1. Text on either side compares as the
/// classic operators do; a float, as floats.
pub fn cmp(a: &Value, b: &Value, mode: i64) -> Value {
    use std::cmp::Ordering;
    let text = |v: &Value| matches!(v, Value::String(_) | Value::Object(_) | Value::Null | Value::Array(_));
    let ord = if text(a) || text(b) {
        if a == b {
            Ordering::Equal
        } else {
            a.partial_cmp(b).unwrap_or(Ordering::Equal)
        }
    } else if matches!(a, Value::Double(_)) || matches!(b, Value::Double(_)) {
        let f = |v: &Value, unsigned: bool| match v {
            Value::Double(d) => *d,
            _ => exact(v, unsigned) as f64,
        };
        f(a, mode & 1 != 0).partial_cmp(&f(b, mode & 2 != 0)).unwrap_or(Ordering::Equal)
    } else {
        exact(a, mode & 1 != 0).cmp(&exact(b, mode & 2 != 0))
    };
    Value::Integer(match ord {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    })
}

/// `__bits64(op, a, b)` (see the module docs).
pub fn bits(op: i64, a: &Value, b: &Value) -> Value {
    let (x, y) = (of(a, false), of(b, false));
    // (bit 3 of `op` is the result's type, for rapidr_ast::numeric)
    Value::Integer(match op & 7 {
        0 => x & y,
        1 => x | y,
        2 => x ^ y,
        3 => !x,
        4 => x.wrapping_shl((y & 63) as u32),
        _ => ((x as u64) >> ((y & 63) as u32)) as i64,
    })
}

/// The builtins (both hosts' `shared_builtin`), `None` for another name.
pub fn call(key: &str, args: &[Value]) -> Option<Result<Value, String>> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    let mode = |i: usize| args.get(i).map_or(0, Value::to_i64);
    Some(Ok(match key {
        "__str64" => str64(&arg(0), mode(1)),
        "__hex64" => hex64(&arg(0)),
        "__idiv64" | "__mod64" => return Some(div(&arg(0), &arg(1), mode(2), key == "__mod64").map_err(str::to_string)),
        "__cmp64" => cmp(&arg(0), &arg(1), mode(2)),
        "__bits64" => bits(mode(0), &arg(1), &arg(2)),
        _ => return None,
    }))
}

/// [`call`] in native builds (by zero, the program's run-time error).
pub fn rp(key: &str, args: &[Value]) -> Value {
    match call(key, args) {
        Some(Ok(v)) => v,
        Some(Err(e)) => crate::runtime_error(&e),
        None => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_dbl, v_int, v_str};

    #[test]
    fn stores() {
        assert_eq!(of(&v_str("18446744073709551615"), true), -1);
        assert_eq!(of(&v_str("9223372036854775807"), false), i64::MAX);
        assert_eq!(of(&v_str("-9223372036854775808"), false), i64::MIN);
        assert_eq!(of(&v_str(" 12abc"), false), 12);
        assert_eq!(of(&v_str("1E3"), false), 1000);
        assert_eq!(of(&v_dbl(-2.7), false), -2);
        assert_eq!(of(&v_dbl(1e30), false), i64::MIN);
        assert_eq!(of(&v_dbl(-1.0), true), 0);
        assert_eq!(of(&v_dbl(f64::NAN), true), 0);
        assert_eq!(of(&v_dbl(1.8e19), true) as u64, 18_000_000_000_000_000_000);
        assert_eq!(of(&v_int(-1), true), -1);
    }

    #[test]
    fn shows_and_operates() {
        assert_eq!(str64(&v_int(-1), 1).to_string_val(), "18446744073709551615");
        assert_eq!(str64(&v_int(i64::MIN), 0).to_string_val(), "-9223372036854775808");
        assert_eq!(str64(&v_dbl(2.5), 0).to_string_val(), "2.500000000");
        assert_eq!(hex64(&v_int(-1)).to_string_val(), "FFFFFFFFFFFFFFFF");
        assert_eq!(div(&v_int(-1), &v_int(2), 1, false).unwrap().to_i64(), i64::MAX);
        assert_eq!(div(&v_int(-7), &v_int(2), 0, true).unwrap().to_i64(), -1);
        assert!(div(&v_int(1), &v_int(0), 0, false).is_err());
        assert_eq!(cmp(&v_int(-1), &v_int(5), 1).to_i64(), 1, "UINT64 max > 5");
        assert_eq!(cmp(&v_int(-1), &v_int(5), 0).to_i64(), -1);
        assert_eq!(cmp(&v_int(i64::MAX), &v_int(i64::MAX - 1), 0).to_i64(), 1);
        assert_eq!(cmp(&v_int(-1), &v_int(-1), 1).to_i64(), 1, "UINT64 max > -1");
        assert_eq!(bits(5, &v_int(-1), &v_int(60)).to_i64(), 15);
        assert_eq!(bits(4, &v_int(1), &v_int(40)).to_i64(), 1 << 40);
        assert_eq!(bits(3, &v_int(0), &v_int(0)).to_i64(), -1);
    }
}
