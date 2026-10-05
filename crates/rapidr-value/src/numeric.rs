//! Declared numeric types (RapidQ manual, Appendix C: "a data type … is
//! used to bind a variable to its definition and cannot be changed once
//! bound"). Storing into a variable, array element, field or parameter
//! declared BYTE, WORD, SHORT, INTEGER/LONG, DWORD, SINGLE or DOUBLE
//! converts the value to that type — the same in both backends
//! (`rapidr_ast::numeric` inserts the conversions; the VM runs them as one
//! opcode, native builds as a direct call). How is what RapidQ's own
//! compiler does (RC.EXE, docs/rapidq-ground-truth.md):
//!
//! * A store into an integer type truncates toward zero (`i = 2.7` holds 2,
//!   `i = -2.7` -2); a number beyond 32 bits (or NaN) becomes -2147483648
//!   (the x87's "integer indefinite": `i = 1E10`); the narrow types then
//!   wrap to their width: BYTE 0..255, WORD 0..65535, SHORT -32768..32767.
//!   INTEGER, LONG and DWORD are all 32-bit signed (`DIM d AS DWORD : d =
//!   -1` prints -1, `d = 3000000000` holds -2147483648).
//! * A BYVAL parameter rounds instead, half to even (`SUB P (n AS
//!   INTEGER)`: `P 2.5` gets 2, `P 3.5` 4, `P 2.7` 3) — `__arg_round` before
//!   the store's conversion. A FUNCTION's result isn't converted at all
//!   (`FUNCTION F AS INTEGER : F = 2.7` returns 2.7).
//! * SINGLE is a 32-bit float (`s = 0.1` holds 0.100000001490116), DOUBLE
//!   a 64-bit one.
//! * A string stored into a number converts like VAL; TRUE is -1. Arrays and
//!   objects are left alone. (RapidQ stores 0 for a string: RapidR keeps
//!   VAL's reading — a judgment call, see the ground-truth doc.)

use crate::Value;

/// A declared numeric type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumKind {
    Byte,
    Word,
    Short,
    /// INTEGER and LONG: 32-bit signed in RapidQ.
    Long,
    /// 32-bit signed too, as RapidQ has it (not unsigned).
    Dword,
    Double,
    /// A 32-bit float.
    Single,
}

const ALL: [NumKind; 7] = [NumKind::Byte, NumKind::Word, NumKind::Short, NumKind::Long, NumKind::Dword, NumKind::Double, NumKind::Single];

impl NumKind {
    /// The kind for a declared type name (`AS INTEGER`), if numeric.
    pub fn of_type(type_name: &str) -> Option<NumKind> {
        Some(match type_name.trim().to_ascii_uppercase().as_str() {
            "BYTE" => NumKind::Byte,
            "WORD" => NumKind::Word,
            "SHORT" => NumKind::Short,
            "INTEGER" | "LONG" => NumKind::Long,
            "DWORD" => NumKind::Dword,
            "DOUBLE" => NumKind::Double,
            "SINGLE" => NumKind::Single,
            _ => return None,
        })
    }

    /// The builtin both backends compile a conversion to (`__to_long`).
    pub fn builtin(self) -> &'static str {
        match self {
            NumKind::Byte => "__to_byte",
            NumKind::Word => "__to_word",
            NumKind::Short => "__to_short",
            NumKind::Long => "__to_long",
            NumKind::Dword => "__to_dword",
            NumKind::Double => "__to_double",
            NumKind::Single => "__to_single",
        }
    }

    pub fn from_builtin(name: &str) -> Option<NumKind> {
        ALL.into_iter().find(|k| k.builtin().eq_ignore_ascii_case(name))
    }

    /// Operand of the VM's conversion opcode.
    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Option<NumKind> {
        ALL.get(code as usize).copied()
    }

    /// Whether values of this kind are integers.
    pub fn is_integer(self) -> bool {
        !matches!(self, NumKind::Double | NumKind::Single)
    }

    /// `n` (a 32-bit number) stored into this integer type: the narrow
    /// types wrap to their width.
    #[inline]
    pub fn wrap(self, n: i64) -> i64 {
        match self {
            NumKind::Byte => n as u8 as i64,
            NumKind::Word => n as u16 as i64,
            NumKind::Short => n as i16 as i64,
            NumKind::Long | NumKind::Dword => n as i32 as i64,
            NumKind::Double | NumKind::Single => n,
        }
    }
}

/// A float rounded half to even, as an integer (NaN → 0; out of range
/// saturates). For geometry and the like — a store into a typed variable
/// truncates ([`trunc_to_int`]).
#[inline]
pub fn round_to_int(f: f64) -> i64 {
    if f.is_nan() {
        0
    } else {
        f.round_ties_even() as i64
    }
}

/// A float stored into an integer variable, as RapidQ does it: truncated
/// toward zero; beyond 32 bits, NaN or infinite, -2147483648.
#[inline]
pub fn trunc_to_int(f: f64) -> i64 {
    let t = f.trunc();
    if t.is_nan() || !(-2147483648.0..=2147483647.0).contains(&t) {
        i32::MIN as i64
    } else {
        t as i64
    }
}

/// An integer RapidQ holds (32 bits): beyond, -2147483648.
#[inline]
pub fn int32_of(n: i64) -> i64 {
    crate::format::int32_of(n)
}

/// A float as a 32-bit float (SINGLE).
#[inline]
pub fn single(f: f64) -> f64 {
    f as f32 as f64
}

/// `v` stored into a variable of kind `kind`.
#[inline]
pub fn convert(v: &Value, kind: NumKind) -> Value {
    match (kind, v) {
        (NumKind::Double, Value::Double(_)) => v.clone(),
        (NumKind::Double | NumKind::Single, Value::Array(_) | Value::Object(_)) => v.clone(),
        (NumKind::Double, _) => Value::Double(v.to_f64()),
        (NumKind::Single, _) => Value::Double(single(v.to_f64())),
        (_, Value::Integer(n)) => Value::Integer(kind.wrap(crate::format::int32_of(*n))),
        (_, Value::Array(_) | Value::Object(_)) => v.clone(),
        (_, _) => Value::Integer(kind.wrap(trunc_to_int(v.to_f64()))),
    }
}

/// The VM's conversion opcode operand for `__arg_round` (beside the kinds'
/// codes).
pub const ARG_ROUND: u8 = 0x80;

/// `__arg_round(v)`: a number passed to a BYVAL integer parameter, rounded
/// half to even first (the x87's rounding: RapidQ's `P 2.5` gets 2, `P 2.7`
/// 3); the store's own conversion follows.
#[inline]
pub fn arg_round(v: &Value) -> Value {
    match v {
        Value::Double(f) => Value::Double(f.round_ties_even()),
        _ => v.clone(),
    }
}

/// `__to_long(v)` & co. in native builds.
pub fn to_byte(v: &Value) -> Value {
    convert(v, NumKind::Byte)
}
pub fn to_word(v: &Value) -> Value {
    convert(v, NumKind::Word)
}
pub fn to_short(v: &Value) -> Value {
    convert(v, NumKind::Short)
}
pub fn to_long(v: &Value) -> Value {
    convert(v, NumKind::Long)
}
pub fn to_dword(v: &Value) -> Value {
    convert(v, NumKind::Dword)
}
pub fn to_double(v: &Value) -> Value {
    convert(v, NumKind::Double)
}
pub fn to_single(v: &Value) -> Value {
    convert(v, NumKind::Single)
}

// ---- Native builds' typed variables (rapidr-codegen-rust `typed`) ----
//
// A variable native code keeps as an `i64` / `f64` works with these, which
// match `Value`'s operators exactly.

/// `v` stored into an integer variable of `kind`, as its number.
#[inline]
pub fn int_of(v: &Value, kind: NumKind) -> i64 {
    match v {
        Value::Integer(n) => kind.wrap(crate::format::int32_of(*n)),
        Value::Array(_) | Value::Object(_) => 0,
        _ => kind.wrap(trunc_to_int(v.to_f64())),
    }
}

/// `v` passed to a BYVAL integer parameter of `kind` (rounded half to
/// even first, [`arg_round`]).
#[inline]
pub fn arg_int_of(v: &Value, kind: NumKind) -> i64 {
    int_of(&arg_round(v), kind)
}

/// `v` stored into a SINGLE/DOUBLE variable, as its number.
#[inline]
pub fn double_of(v: &Value) -> f64 {
    v.to_f64()
}

/// `a / b` (always floating point; by zero an infinity or NaN, as in
/// RapidQ: `PRINT 7 / 0` shows -2147483648).
#[inline]
pub fn fdiv(a: f64, b: f64) -> f64 {
    a / b
}

/// RapidQ's `\` operand: rounded as its CINT rounds (`INT(x + 0.5)`: 7.5 →
/// 8, -7.5 → -7, -7.9 → -7), 32 bits.
#[inline]
pub fn idiv_operand(f: f64) -> i64 {
    trunc_to_int(f + 0.5)
}

/// RapidQ's MOD and AND / OR / XOR operand: rounded half to even (the
/// x87's FISTP), 32 bits.
#[inline]
pub fn fistp(f: f64) -> i64 {
    crate::format::int32(f)
}

/// RapidQ's message for `\` and MOD by zero (it stops the program).
pub const DIVISION_BY_ZERO: &str = "Division by zero";

/// `a \ b` on 32-bit integers, or `None` when `b` is 0.
#[inline]
pub fn checked_idiv(a: i64, b: i64) -> Option<i64> {
    let (a, b) = (crate::format::int32_of(a), crate::format::int32_of(b));
    (b != 0).then(|| a.wrapping_div(b))
}

/// `a MOD b` on 32-bit integers, or `None` when `b` is 0.
#[inline]
pub fn checked_imod(a: i64, b: i64) -> Option<i64> {
    let (a, b) = (crate::format::int32_of(a), crate::format::int32_of(b));
    (b != 0).then(|| a.wrapping_rem(b))
}

/// `a \ b` on integers (by zero, a run-time error).
#[inline]
pub fn idiv(a: i64, b: i64) -> i64 {
    checked_idiv(a, b).unwrap_or_else(|| crate::runtime_error(DIVISION_BY_ZERO))
}

/// `a \ b` on floats.
#[inline]
pub fn idiv_f(a: f64, b: f64) -> i64 {
    idiv(idiv_operand(a), idiv_operand(b))
}

/// `a MOD b` on integers (by zero, a run-time error).
#[inline]
pub fn imod(a: i64, b: i64) -> i64 {
    checked_imod(a, b).unwrap_or_else(|| crate::runtime_error(DIVISION_BY_ZERO))
}

/// `a MOD b` on floats: RapidQ rounds both to integers first (`7.5 MOD 2`
/// is 0).
#[inline]
pub fn fmod(a: f64, b: f64) -> i64 {
    imod(fistp(a), fistp(b))
}

// Comparisons as `Value` makes them: a NaN on either side is "less" and
// "equal" at once, as the x87's FCOM leaves it in RapidQ (`=`, `<`, `<=`
// true; `<>`, `>`, `>=` false).

/// `a = b`.
#[inline]
pub fn eq(a: f64, b: f64) -> bool {
    a == b || a.is_nan() || b.is_nan()
}

/// `a <> b`.
#[inline]
pub fn ne(a: f64, b: f64) -> bool {
    !eq(a, b)
}

/// `a < b`.
#[inline]
pub fn lt(a: f64, b: f64) -> bool {
    a < b || a.is_nan() || b.is_nan()
}

/// `a > b`.
#[inline]
pub fn gt(a: f64, b: f64) -> bool {
    a > b
}

/// `a <= b`.
#[inline]
pub fn le(a: f64, b: f64) -> bool {
    a <= b || a.is_nan() || b.is_nan()
}

/// `a >= b`.
#[inline]
pub fn ge(a: f64, b: f64) -> bool {
    a >= b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_dbl, v_int, v_str};

    fn long(v: Value) -> i64 {
        convert(&v, NumKind::Long).to_i64()
    }

    #[test]
    fn stores_truncate_like_rapidq() {
        // RC.EXE: `i = 2.7` → 2, `i = -2.7` → -2, `i = 2.5` → 2.
        for (f, n) in [(2.5, 2), (3.5, 3), (-2.5, -2), (-1.5, -1), (0.5, 0), (2.7, 2), (-2.7, -2), (3.49, 3)] {
            assert_eq!(long(v_dbl(f)), n, "{f}");
        }
        // Beyond 32 bits: -2147483648, then the narrow types wrap.
        assert_eq!(long(v_dbl(1e10)), -2147483648);
        assert_eq!(long(v_dbl(f64::NAN)), -2147483648);
        assert_eq!(convert(&v_dbl(1e10), NumKind::Short).to_i64(), 0);
        assert_eq!(convert(&v_dbl(300.7), NumKind::Byte).to_i64(), 44);
    }

    #[test]
    fn parameters_round_half_to_even() {
        for (f, n) in [(2.5, 2), (3.5, 4), (-2.5, -2), (2.7, 3), (-2.7, -3)] {
            assert_eq!(arg_int_of(&v_dbl(f), NumKind::Long), n, "{f}");
        }
        assert_eq!(arg_int_of(&v_dbl(1e10), NumKind::Long), -2147483648);
    }

    #[test]
    fn integers_wrap_to_their_width() {
        assert_eq!(long(v_int(2147483648)), -2147483648, "beyond 32 bits");
        assert_eq!(long(v_int(4294967296)), -2147483648);
        assert_eq!(convert(&v_int(300), NumKind::Byte).to_i64(), 44);
        assert_eq!(convert(&v_int(-1), NumKind::Byte).to_i64(), 255);
        assert_eq!(convert(&v_int(40000), NumKind::Short).to_i64(), -25536);
        assert_eq!(convert(&v_int(70000), NumKind::Word).to_i64(), 4464);
        assert_eq!(convert(&v_int(-1), NumKind::Dword).to_i64(), -1, "DWORD is signed in RapidQ");
        assert_eq!(convert(&v_dbl(4294967295.0), NumKind::Dword).to_i64(), -2147483648);
    }

    #[test]
    fn singles_are_32_bit() {
        assert_eq!(convert(&v_dbl(0.1), NumKind::Single).to_f64(), 0.1f32 as f64);
        assert_eq!(convert(&v_int(16777217), NumKind::Single).to_f64(), 16777216.0);
    }

    #[test]
    fn other_values() {
        assert_eq!(long(v_str("12")), 12);
        assert_eq!(long(Value::Boolean(true)), -1);
        assert_eq!(long(Value::Null), 0);
        assert!(matches!(convert(&v_int(5), NumKind::Double), Value::Double(d) if d == 5.0));
        assert!(matches!(convert(&v_str("2.5"), NumKind::Double), Value::Double(d) if d == 2.5));
    }

    #[test]
    fn kinds_round_trip() {
        for k in ALL {
            assert_eq!(NumKind::from_code(k.code()), Some(k));
            assert_eq!(NumKind::from_builtin(k.builtin()), Some(k));
        }
        assert_eq!(NumKind::of_type("Integer"), Some(NumKind::Long));
        assert_eq!(NumKind::of_type("single"), Some(NumKind::Single));
        assert_eq!(NumKind::of_type("STRING"), None);
    }

    #[test]
    fn integer_division_and_mod_as_rapidq() {
        // RC.EXE: `\` rounds its operands as CINT (INT(x + 0.5)), MOD half
        // to even.
        for (a, b, q) in [(7.9, 2.0, 4), (-7.9, 2.0, -3), (7.5, 2.0, 4), (-7.5, 2.0, -3), (6.5, 2.0, 3), (7.0, 2.5, 2), (-7.1, 1.0, -6), (7.1, 1.0, 7)] {
            assert_eq!(idiv_f(a, b), q, "{a} \\ {b}");
        }
        for (a, b, r) in [(7.5, 2.0, 0), (6.5, 4.0, 2), (-6.5, 4.0, -2), (5.5, 2.5, 0), (8.5, 3.0, 2)] {
            assert_eq!(fmod(a, b), r, "{a} mod {b}");
        }
        assert_eq!(checked_idiv(7, 0), None);
        assert_eq!(checked_imod(7, 0), None);
        assert_eq!(idiv(-7, 2), -3);
        assert_eq!(imod(-7, 3), -1);
        assert_eq!(imod(7, -3), 1);
    }

    #[test]
    fn typed_helpers_match_value_operators() {
        let nums = [0.0, 1.0, -1.0, 2.5, -7.0, 7.0, 1e300, f64::NAN, 3.0];
        for &a in &nums {
            for &b in &nums {
                let (va, vb) = (v_dbl(a), v_dbl(b));
                let same = |x: f64, y: Value| (x.is_nan() && y.to_f64().is_nan()) || x == y.to_f64();
                assert!(same(fdiv(a, b), &va / &vb), "{a} / {b}");
                if fistp(b) != 0 {
                    assert_eq!(fmod(a, b), (&va % &vb).to_i64(), "{a} mod {b}");
                }
                assert_eq!(le(a, b), va.rp_le(&vb).to_bool(), "{a} <= {b}");
                assert_eq!(ge(a, b), va.rp_ge(&vb).to_bool(), "{a} >= {b}");
            }
        }
        for (a, b) in [(7, 2), (-7, 2), (0, 5), (i64::from(i32::MIN), 3)] {
            assert_eq!(idiv(a, b), v_int(a).int_div(&v_int(b)).to_i64());
            assert_eq!(imod(a, b), (&v_int(a) % &v_int(b)).to_i64());
        }
        assert_eq!(int_of(&v_dbl(2.5), NumKind::Long), 2);
        assert_eq!(int_of(&v_str("300"), NumKind::Byte), 44);
    }
}
