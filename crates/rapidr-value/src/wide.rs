//! INT64, UINT64 and UINT32: RapidR's 64-bit whole numbers
//! (docs/big-numbers-design.md).
//!
//! A typed store makes a [`Value::Int64`] (INT64; UINT32's values too) or a
//! [`Value::UInt64`], and the value carries its type from there: through a
//! VARIANT, an array, a TYPE field, a parameter, a property, an event
//! argument, the debugger, JSON. Every generic operation of `Value` comes
//! here when either side is one of them — the interpreter, the JIT's
//! helpers, native builds and the web all call the same functions:
//!
//! * shown in full (a UINT64 unsigned);
//! * `+ - *` wrap at 64 bits; a UINT64 on either side makes a UINT64, else
//!   an INT64; a DOUBLE makes a DOUBLE; `/` and `^` are DOUBLE as ever;
//! * `\`, MOD and the comparisons are exact (worked out in 128 bits) with
//!   each side's own sign, the quotient and remainder wrapped to 64 bits; a
//!   comparison with a DOUBLE is exact too;
//! * AND OR XOR NOT SHL SHR on all 64 bits (SHR logical, the count's low 6
//!   bits).
//!
//! A classic `Value::Integer` meeting one is the whole number it holds. A
//! program without INT64 / UINT64 / UINT32 never makes these kinds, so
//! nothing classic changes (RapidQ's 32-bit rules, docs/rapidq-ground-truth.md).
//!
//! `__str64` & co. are 2.122.1's compiled rewrites, kept for its bytecode.

use crate::{format, Value};
use std::cmp::Ordering;

/// A 64-bit whole number with its type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W {
    I(i64),
    U(u64),
}

impl W {
    /// Its number, exactly.
    pub fn exact(self) -> i128 {
        match self {
            W::I(n) => n as i128,
            W::U(n) => n as i128,
        }
    }

    /// Its 64 bits.
    pub fn bits(self) -> u64 {
        match self {
            W::I(n) => n as u64,
            W::U(n) => n,
        }
    }

    pub fn unsigned(self) -> bool {
        matches!(self, W::U(_))
    }

    /// `bits` as a value of this type (`unsigned`).
    fn of_bits(bits: u64, unsigned: bool) -> Value {
        if unsigned {
            Value::UInt64(bits)
        } else {
            Value::Int64(bits as i64)
        }
    }
}

/// `v` as a 64-bit whole number when it is one of the 64-bit kinds.
#[inline]
pub fn kind_of(v: &Value) -> Option<W> {
    match v {
        Value::Int64(n) => Some(W::I(*n)),
        Value::UInt64(n) => Some(W::U(*n)),
        _ => None,
    }
}

/// Whether `v` is an INT64 or a UINT64.
#[inline]
pub fn is_wide(v: &Value) -> bool {
    matches!(v, Value::Int64(_) | Value::UInt64(_))
}

/// The other operand of a 64-bit operation, as a whole number: a classic
/// integer as it is, TRUE -1, Null 0; `None` for a DOUBLE (the operation is
/// then a DOUBLE's) or text and the rest.
fn whole(v: &Value) -> Option<W> {
    match v {
        Value::Int64(n) => Some(W::I(*n)),
        Value::UInt64(n) => Some(W::U(*n)),
        Value::Integer(n) => Some(W::I(*n)),
        Value::Boolean(b) => Some(W::I(-(*b as i64))),
        Value::Null => Some(W::I(0)),
        _ => None,
    }
}

/// A number made by a surface that reads whole numbers (a database
/// column, JSON, a file): a classic integer when it fits 32 bits — what a
/// RapidQ program reads — else an INT64, so it is never shown cut.
#[inline]
pub fn whole_number(n: i64) -> Value {
    if (i32::MIN as i64..=i32::MAX as i64).contains(&n) {
        Value::Integer(n)
    } else {
        Value::Int64(n)
    }
}

/// [`whole_number`] of an unsigned one: a UINT64 above 9223372036854775807.
#[inline]
pub fn whole_number_u(n: u64) -> Value {
    if n > i64::MAX as u64 {
        Value::UInt64(n)
    } else {
        whole_number(n as i64)
    }
}

// ---- Stores (`__to_int64`, `__to_uint64`, `__to_uint32`) ----

/// `v` as a 64-bit integer: an integer as it is (its bits: -1 stored into a
/// UINT64 is 18446744073709551615), TRUE -1, a float truncated toward zero
/// (out of the type's range, or NaN: an INT64 gets -9223372036854775808, a
/// UINT64 0), text read as VAL reads it, exactly to the last digit.
pub fn of(v: &Value, unsigned: bool) -> i64 {
    match v {
        Value::Integer(n) | Value::Int64(n) => *n,
        Value::UInt64(n) => *n as i64,
        Value::Boolean(b) => -(*b as i64),
        Value::Double(f) => from_f64(*f, unsigned),
        Value::String(s) => parse(s.as_str(), unsigned),
        Value::Null | Value::Array(_) | Value::Object(_) => 0,
    }
}

/// `v` stored into an INT64.
pub fn to_int64(v: &Value) -> Value {
    Value::Int64(of(v, false))
}

/// `v` stored into a UINT64.
pub fn to_uint64(v: &Value) -> Value {
    Value::UInt64(of(v, true) as u64)
}

/// `v` stored into a UINT32 (held as an INT64, 0 to 4294967295): a whole
/// number wraps to its low 32 bits (-1 is 4294967295); a float is
/// truncated toward zero, out of range (or NaN) 0, as a UINT64's.
pub fn to_uint32(v: &Value) -> Value {
    let n = match v {
        Value::Double(f) => {
            let t = f.trunc();
            if t.is_nan() || !(0.0..4294967296.0).contains(&t) {
                0
            } else {
                t as i64
            }
        }
        Value::String(s) => {
            let w = parse(s.as_str(), false);
            w as u32 as i64
        }
        other => of(other, false) as u32 as i64,
    };
    Value::Int64(n)
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
    match exact_text(s) {
        Some(n) => n as i64,
        None => from_f64(crate::builtins::rp_val(&Value::String(s.into())).to_f64(), unsigned),
    }
}

/// A whole number written in `s` (VAL's reading: spaces ignored, a sign,
/// digits, a fraction ignored), exactly, when it has no exponent and fits
/// -9223372036854775808 to 18446744073709551615.
pub fn exact_text(s: &str) -> Option<i128> {
    let t: String = s.chars().filter(|c| *c != ' ').collect();
    let (neg, body) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(&t)),
    };
    let end = body.find(|c: char| !c.is_ascii_digit()).unwrap_or(body.len());
    let (digits, rest) = body.split_at(end);
    let after_fraction = rest.strip_prefix(['.', ',']).map_or(rest, |r| r.trim_start_matches(|c: char| c.is_ascii_digit()));
    let exponent = after_fraction.starts_with(['e', 'E']) && after_fraction[1..].trim_start_matches(['+', '-']).starts_with(|c: char| c.is_ascii_digit());
    if digits.is_empty() || exponent || digits.len() > 30 {
        return None;
    }
    let n = digits.parse::<u128>().ok()?;
    let n = if neg { -(n as i128) } else { n as i128 };
    (i64::MIN as i128..=u64::MAX as i128).contains(&n).then_some(n)
}

/// What INPUT stores into a 64-bit variable (`current` its value now) from
/// the line typed: the number read exactly, VAL's way; `None` when
/// `current` isn't one.
pub fn input(line: &str, current: &Value) -> Option<Value> {
    match current {
        Value::Int64(_) => Some(Value::Int64(of(&Value::String(line.into()), false))),
        Value::UInt64(_) => Some(Value::UInt64(of(&Value::String(line.into()), true) as u64)),
        _ => None,
    }
}

// ---- Showing ----

/// A 64-bit number's text, in full.
pub fn text(w: W) -> String {
    match w {
        W::I(n) => n.to_string(),
        W::U(n) => n.to_string(),
    }
}

/// HEX$ of a 64-bit value: 8 digits as for every number, 16 when it needs
/// more than 32 bits (a negative INT64 needs all 64); at least `min`
/// digits when given (RapidR's `HEX$(x, digits)`).
pub fn hex(w: W, min: usize) -> String {
    let bits = w.bits();
    let width = if bits > u32::MAX as u64 { 16 } else { 8 };
    format!("{:0width$X}", bits, width = if min > 0 { min.min(64) } else { width })
}

/// BIN$ / OCT$ of a 64-bit value: all 64 bits, no leading zeros.
pub fn radix(w: W, base: u32) -> String {
    match base {
        2 => format!("{:b}", w.bits()),
        8 => format!("{:o}", w.bits()),
        _ => format!("{:X}", w.bits()),
    }
}

// ---- Operations (`Value`'s generic ones come here) ----

/// The type two operands make: a UINT64 when either is one.
fn unsigned_of(a: W, b: W) -> bool {
    a.unsigned() || b.unsigned()
}

/// `a + b`, `a - b` or `a * b` (`op` '+', '-', '*') when either side is a
/// 64-bit kind; `None` when the other isn't a whole number (a DOUBLE: the
/// caller's float arithmetic).
pub fn arith(op: u8, a: &Value, b: &Value) -> Option<Value> {
    let (x, y) = (whole(a)?, whole(b)?);
    let (p, q) = (x.bits(), y.bits());
    let r = match op {
        b'+' => p.wrapping_add(q),
        b'-' => p.wrapping_sub(q),
        _ => p.wrapping_mul(q),
    };
    Some(W::of_bits(r, unsigned_of(x, y)))
}

/// `-a` of a 64-bit value (wrapping; its type kept).
pub fn neg(w: W) -> Value {
    W::of_bits(w.bits().wrapping_neg(), w.unsigned())
}

/// `ABS(a)`: an INT64's absolute value (the smallest stays itself, as
/// RapidQ's LONG gives -2147483648 for `ABS(-2147483648)` stored back:
/// RC.EXE, 2026-11-05); a UINT64 as it is.
pub fn abs(w: W) -> Value {
    match w {
        W::I(n) => Value::Int64(n.wrapping_abs()),
        W::U(n) => Value::UInt64(n),
    }
}

/// A DOUBLE operand of `\` or MOD with a 64-bit one: truncated toward zero.
fn exact_operand(v: &Value) -> Option<(i128, bool)> {
    match whole(v) {
        Some(w) => Some((w.exact(), w.unsigned())),
        None => match v {
            Value::Double(f) if f.is_finite() && f.abs() < 1.8e19 => Some((f.trunc() as i128, false)),
            Value::Double(_) => Some((0, false)),
            Value::String(s) => Some((exact_text(s.as_str()).unwrap_or(0), false)),
            _ => Some((0, false)),
        },
    }
}

/// `a \ b` (`modulo` false) or `a MOD b` with a 64-bit side: exact, each
/// side's own sign, wrapped to 64 bits.
pub fn div_mod(a: &Value, b: &Value, modulo: bool) -> Result<Value, &'static str> {
    let (x, ux) = exact_operand(a).unwrap_or((0, false));
    let (y, uy) = exact_operand(b).unwrap_or((0, false));
    if y == 0 {
        return Err(crate::numeric::DIVISION_BY_ZERO);
    }
    let r = if modulo { x % y } else { x / y };
    Ok(W::of_bits(r as u64, ux || uy))
}

/// AND (0), OR (1), XOR (2) on 64 bits with a 64-bit side.
pub fn bitwise(op: u8, a: &Value, b: &Value) -> Value {
    let x = operand_bits(a);
    let y = operand_bits(b);
    let r = match op {
        0 => x.0 & y.0,
        1 => x.0 | y.0,
        _ => x.0 ^ y.0,
    };
    W::of_bits(r, x.1 || y.1)
}

/// An operand's 64 bits for the bit operators (a float truncated, text
/// read as VAL), and whether it is a UINT64.
fn operand_bits(v: &Value) -> (u64, bool) {
    match whole(v) {
        Some(w) => (w.bits(), w.unsigned()),
        None => (of(v, false) as u64, false),
    }
}

/// NOT of a 64-bit value.
pub fn not(w: W) -> Value {
    W::of_bits(!w.bits(), w.unsigned())
}

/// `a SHL n` / `a SHR n` (`right`) of a 64-bit value: the count's low 6
/// bits, SHR logical.
pub fn shift(w: W, n: &Value, right: bool) -> Value {
    let k = (operand_bits(n).0 & 63) as u32;
    let bits = if right { w.bits() >> k } else { w.bits() << k };
    W::of_bits(bits, w.unsigned())
}

/// An exact comparison with a 64-bit side; `None` when the other side
/// isn't a number (text: the caller compares as text).
pub fn compare(a: &Value, b: &Value) -> Option<Ordering> {
    match (whole(a), whole(b)) {
        (Some(x), Some(y)) => Some(x.exact().cmp(&y.exact())),
        (Some(x), None) => match b {
            Value::Double(f) => Some(int_vs_float(x.exact(), *f)),
            _ => None,
        },
        (None, Some(y)) => match a {
            Value::Double(f) => Some(int_vs_float(y.exact(), *f).reverse()),
            _ => None,
        },
        (None, None) => None,
    }
}

/// A whole number against a float, exactly (2^53 + 1 is above the float
/// 2^53). A NaN is the callers' (RapidQ's FCOM rule, before this).
fn int_vs_float(i: i128, f: f64) -> Ordering {
    if f.is_nan() {
        return Ordering::Equal;
    }
    if f >= 1.7e38 {
        return Ordering::Less;
    }
    if f <= -1.7e38 {
        return Ordering::Greater;
    }
    let t = f.trunc();
    match i.cmp(&(t as i128)) {
        Ordering::Equal if f > t => Ordering::Less,
        Ordering::Equal if f < t => Ordering::Greater,
        o => o,
    }
}

/// A FOR loop's step on a 64-bit counter: the next value, or `None` when
/// it would pass the type's limit (the loop ends there; 2.122.1 wrapped and
/// looped for ever: `FOR u = 10 TO 0 STEP -1` on a UINT64).
pub fn for_step(var: &Value, step: &Value) -> Option<Value> {
    let w = kind_of(var)?;
    let s = match whole(step) {
        Some(s) => s.exact(),
        None => match step {
            Value::Double(f) if f.is_finite() => f.trunc() as i128,
            _ => 0,
        },
    };
    let next = w.exact() + s;
    let fits = match w {
        W::I(_) => (i64::MIN as i128..=i64::MAX as i128).contains(&next),
        W::U(_) => (0..=u64::MAX as i128).contains(&next),
    };
    fits.then(|| W::of_bits(next as u64, w.unsigned()))
}

// ---- Jango's bit functions (groups.io #38944; RapidR's) ----

/// The width a value's bits are read in: 64 for a 64-bit kind, else 32.
fn width_of(v: &Value) -> u32 {
    if is_wide(v) {
        64
    } else {
        32
    }
}

/// A value's bits for the bit functions: a 64-bit kind's 64, a classic
/// number's 32 (as RapidQ's operators take it, rounded half to even).
fn bits_of(v: &Value) -> u64 {
    match v {
        Value::Int64(n) => *n as u64,
        Value::UInt64(n) => *n,
        Value::Integer(n) => *n as u32 as u64,
        Value::Boolean(b) => {
            if *b {
                u32::MAX as u64
            } else {
                0
            }
        }
        other => format::int32(other.to_f64()) as u32 as u64,
    }
}

/// The checked bit range of GETBITS / SETBITS.
fn range(v: &Value, start: &Value, count: &Value, name: &str) -> Result<(u32, u32), String> {
    let width = width_of(v);
    let (s, c) = (start.to_i64(), count.to_i64());
    if s < 0 || c < 0 || s + c > width as i64 {
        return Err(format!(
            "{name}: bits {s} to {} are outside the value's {width} bits (start 0 to {}, start + count up to {width})",
            s + c - 1,
            width - 1
        ));
    }
    Ok((s as u32, c as u32))
}

fn mask(count: u32) -> u64 {
    if count >= 64 {
        u64::MAX
    } else {
        (1u64 << count) - 1
    }
}

/// The type a bit function's result keeps: a UINT64 stays one, an INT64
/// one, a classic number gives a classic number (0 up).
fn keep_type(v: &Value, bits: u64) -> Value {
    match v {
        Value::UInt64(_) => Value::UInt64(bits),
        Value::Int64(_) => Value::Int64(bits as i64),
        _ => whole_number_u(bits),
    }
}

/// `GETBITS(value, start, count)`: `count` bits from bit `start` (bit 0
/// the rightmost), as a number from 0.
pub fn getbits(v: &Value, start: &Value, count: &Value) -> Result<Value, String> {
    let (s, c) = range(v, start, count, "GETBITS")?;
    if c == 0 {
        return Ok(keep_type(v, 0));
    }
    Ok(keep_type(v, (bits_of(v) >> s) & mask(c)))
}

/// `SETBITS(value, start, count, bits)`: `value` with `count` bits from
/// bit `start` replaced by `bits`' low `count` bits.
pub fn setbits(v: &Value, start: &Value, count: &Value, new: &Value) -> Result<Value, String> {
    let (s, c) = range(v, start, count, "SETBITS")?;
    if c == 0 {
        return Ok(v.clone());
    }
    let m = mask(c) << s;
    let b = (bits_of(new) << s) & m;
    let r = (bits_of(v) & !m) | b;
    Ok(match v {
        Value::UInt64(_) => Value::UInt64(r),
        Value::Int64(_) => Value::Int64(r as i64),
        // (a classic number's 32 bits, as a LONG: the bit 31 set is negative)
        _ => Value::Integer(r as u32 as i32 as i64),
    })
}

/// LOWORD (bits 0-15), HIWORD (16-31), LOBYTE (0-7), HIBYTE (8-15).
pub fn word_or_byte(name: &str, v: &Value) -> Value {
    let b = bits_of(v);
    Value::Integer(match name {
        "loword" => b & 0xFFFF,
        "hiword" => (b >> 16) & 0xFFFF,
        "lobyte" => b & 0xFF,
        _ => (b >> 8) & 0xFF,
    } as i64)
}

// ---- 2.122.1's compiled rewrites (its bytecode still calls them) ----

/// `__str64(v, mode)`: a PRINT item or STR$ of a 64-bit value, in full
/// (mode bit 0: unsigned; bit 1: STR$, else PRINT).
pub fn str64(v: &Value, mode: i64) -> Value {
    let unsigned = mode & 1 != 0;
    let str_func = mode & 2 != 0;
    match v {
        Value::Integer(n) | Value::Int64(n) => Value::String(text(if unsigned { W::U(*n as u64) } else { W::I(*n) }).into()),
        Value::UInt64(n) => Value::String(n.to_string().into()),
        _ if str_func => crate::builtins::rp_str(v),
        _ => Value::String(format::print_text(v).into()),
    }
}

/// A 2.122.1 rewrite's operand: its bits read as its compiled type said.
fn old_operand(v: &Value, unsigned: bool) -> Value {
    match v {
        Value::Integer(n) | Value::Int64(n) if unsigned => Value::UInt64(*n as u64),
        Value::Integer(n) => Value::Int64(*n),
        other => other.clone(),
    }
}

/// The builtins (both hosts' `shared_builtin`), `None` for another name.
pub fn call(key: &str, args: &[Value]) -> Option<Result<Value, String>> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    let mode = |i: usize| args.get(i).map_or(0, Value::to_i64);
    Some(Ok(match key {
        "__str64" => str64(&arg(0), mode(1)),
        "__hex64" => Value::String(format!("{:016X}", of(&arg(0), false) as u64).into()),
        "__idiv64" | "__mod64" => {
            let (a, b) = (old_operand(&arg(0), mode(2) & 1 != 0), old_operand(&arg(1), mode(2) & 2 != 0));
            return Some(div_mod(&a, &b, key == "__mod64").map_err(str::to_string));
        }
        "__cmp64" => {
            let (a, b) = (old_operand(&arg(0), mode(2) & 1 != 0), old_operand(&arg(1), mode(2) & 2 != 0));
            let ord = compare(&a, &b).unwrap_or_else(|| a.partial_cmp(&b).unwrap_or(Ordering::Equal));
            Value::Integer(ord as i64)
        }
        "__bits64" => {
            let op = mode(0);
            let unsigned = op & 8 != 0;
            let a = old_operand(&arg(1), unsigned);
            let r = match op & 7 {
                0..=2 => bitwise((op & 7) as u8, &a, &arg(2)),
                3 => not(kind_of(&a).unwrap_or(W::I(0))),
                4 => shift(kind_of(&a).unwrap_or(W::I(0)), &arg(2), false),
                _ => shift(kind_of(&a).unwrap_or(W::I(0)), &arg(2), true),
            };
            // (2.122.1's code holds the result in its classic integer)
            Value::Integer(of(&r, false))
        }
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

/// `GETBITS` & co. in native builds (an error is the program's).
pub fn rp_bits(key: &str, args: &[Value]) -> Value {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    let r = match key {
        "getbits" => getbits(&arg(0), &arg(1), &arg(2)),
        "setbits" => setbits(&arg(0), &arg(1), &arg(2), &arg(3)),
        other => Ok(word_or_byte(other, &arg(0))),
    };
    r.unwrap_or_else(|e| crate::runtime_error(&e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_dbl, v_int, v_str};

    fn i(n: i64) -> Value {
        Value::Int64(n)
    }
    fn u(n: u64) -> Value {
        Value::UInt64(n)
    }

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
        assert_eq!(to_uint32(&v_int(-1)), i(4294967295));
        assert_eq!(to_uint32(&v_int(-175453189)), i(4119514107));
        assert_eq!(to_uint32(&v_dbl(4119514107.0)), i(4119514107));
        assert_eq!(to_uint32(&v_dbl(-1.0)), i(0));
        assert_eq!(to_uint32(&u(u64::MAX)), i(4294967295));
        assert_eq!(to_uint32(&v_str("4119514107")), i(4119514107));
    }

    #[test]
    fn operates() {
        assert_eq!(arith(b'+', &i(i64::MAX), &v_int(1)), Some(i(i64::MIN)), "wraps");
        assert_eq!(arith(b'+', &u(u64::MAX), &v_int(1)), Some(u(0)));
        assert_eq!(arith(b'-', &u(0), &v_int(1)), Some(u(u64::MAX)));
        assert_eq!(arith(b'*', &i(4_000_000_000), &v_int(2)), Some(i(8_000_000_000)));
        assert_eq!(arith(b'+', &i(1), &u(1)), Some(u(2)), "UINT64 wins");
        assert_eq!(arith(b'+', &i(1), &v_dbl(0.5)), None, "a DOUBLE makes a DOUBLE");
        assert_eq!(div_mod(&u(u64::MAX), &v_int(2), false), Ok(u(i64::MAX as u64)));
        assert_eq!(div_mod(&i(-7), &v_int(2), true), Ok(i(-1)));
        assert!(div_mod(&i(1), &v_int(0), false).is_err());
        assert_eq!(compare(&u(u64::MAX), &v_int(5)), Some(Ordering::Greater));
        assert_eq!(compare(&u(u64::MAX), &v_int(-1)), Some(Ordering::Greater), "u > -1");
        assert_eq!(compare(&i(9007199254740993), &v_dbl(9007199254740992.0)), Some(Ordering::Greater), "exact against a DOUBLE");
        assert_eq!(compare(&v_dbl(0.5), &i(0)), Some(Ordering::Greater));
        assert_eq!(compare(&i(-1), &v_dbl(-0.5)), Some(Ordering::Less));
        assert_eq!(compare(&i(1), &v_str("1")), None, "text: the caller's");
        assert_eq!(bitwise(0, &u(u64::MAX), &v_int(255)), u(255));
        assert_eq!(not(W::I(0)), i(-1));
        assert_eq!(shift(W::U(u64::MAX), &v_int(60), true), u(15));
        assert_eq!(shift(W::I(1), &v_int(40), false), i(1 << 40));
        assert_eq!(neg(W::I(i64::MIN)), i(i64::MIN));
        assert_eq!(abs(W::I(i64::MIN)), i(i64::MIN));
        assert_eq!(abs(W::I(-5)), i(5));
    }

    #[test]
    fn for_steps_end_at_the_type() {
        assert_eq!(for_step(&u(1), &v_int(-1)), Some(u(0)));
        assert_eq!(for_step(&u(0), &v_int(-1)), None, "below 0: the loop ends");
        assert_eq!(for_step(&u(u64::MAX), &v_int(1)), None);
        assert_eq!(for_step(&i(i64::MAX), &v_int(1)), None);
        assert_eq!(for_step(&i(5), &v_int(2)), Some(i(7)));
        assert_eq!(for_step(&v_int(5), &v_int(2)), None, "not 64-bit: the caller's");
    }

    #[test]
    fn shows() {
        assert_eq!(text(W::U(u64::MAX)), "18446744073709551615");
        assert_eq!(hex(W::I(-1), 0), "FFFFFFFFFFFFFFFF");
        assert_eq!(hex(W::I(4119514107), 0), "F58ACBFB");
        assert_eq!(hex(W::U(255), 0), "000000FF");
        assert_eq!(hex(W::U(1 << 40), 0), "0000010000000000");
        assert_eq!(hex(W::U(255), 16), "00000000000000FF");
        assert_eq!(hex(W::U(u64::MAX), 4), "FFFFFFFFFFFFFFFF", "never cut");
        assert_eq!(radix(W::I(-1), 2).len(), 64);
        assert_eq!(radix(W::U(8), 8), "10");
    }

    #[test]
    fn bit_functions() {
        let d = v_int(-175453189); // &HF58ACBFB, a DWORD
        assert_eq!(word_or_byte("loword", &d), v_int(0xCBFB));
        assert_eq!(word_or_byte("hiword", &d), v_int(0xF58A));
        assert_eq!(word_or_byte("lobyte", &d), v_int(0xFB));
        assert_eq!(word_or_byte("hibyte", &d), v_int(0xCB));
        assert_eq!(getbits(&d, &v_int(28), &v_int(4)), Ok(v_int(0xF)));
        assert_eq!(getbits(&d, &v_int(0), &v_int(32)), Ok(i(4119514107)), "32 bits of a classic number: never negative");
        assert_eq!(getbits(&u(u64::MAX), &v_int(60), &v_int(4)), Ok(u(15)));
        assert!(getbits(&d, &v_int(30), &v_int(4)).is_err(), "past bit 31");
        assert!(getbits(&u(1), &v_int(-1), &v_int(2)).is_err());
        assert_eq!(setbits(&v_int(0), &v_int(4), &v_int(4), &v_int(0xF)), Ok(v_int(0xF0)));
        assert_eq!(setbits(&v_int(0), &v_int(28), &v_int(4), &v_int(0xF)), Ok(v_int(-268435456)), "a LONG's bit 31");
        assert_eq!(setbits(&u(0), &v_int(60), &v_int(4), &v_int(0xF)), Ok(u(0xF000000000000000)));
        assert_eq!(setbits(&i(-1), &v_int(0), &v_int(64), &v_int(0)), Ok(i(0)));
    }

    #[test]
    fn old_rewrites_still_answer() {
        assert_eq!(str64(&v_int(-1), 1).to_string_val(), "18446744073709551615");
        assert_eq!(str64(&u(u64::MAX), 1).to_string_val(), "18446744073709551615");
        assert_eq!(call("__cmp64", &[v_int(-1), v_int(5), v_int(1)]).unwrap().unwrap(), v_int(1));
        assert_eq!(call("__idiv64", &[v_int(-1), v_int(2), v_int(1)]).unwrap().unwrap(), u(i64::MAX as u64));
        assert_eq!(call("__bits64", &[v_int(5), v_int(-1), v_int(60)]).unwrap().unwrap(), v_int(15));
    }
}
