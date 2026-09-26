//! Declared numeric types (RapidQ manual, Appendix C: "a data type … is
//! used to bind a variable to its definition and cannot be changed once
//! bound"). Storing into a variable, array element, field, parameter or
//! FUNCTION result declared BYTE, WORD, SHORT, INTEGER/LONG, DWORD, SINGLE
//! or DOUBLE converts the value to that type — the same in both backends
//! (`rapidr_ast::numeric` inserts the conversions; the VM runs them as one
//! opcode, native builds as a direct call).
//!
//! * Integer types round a fractional value half to even, the x86 FPU's
//!   rounding that RapidQ documents for ROUND (2.5 → 2, 3.5 → 4, -2.5 → -2),
//!   and wrap to their width: BYTE 0..255, WORD 0..65535, SHORT
//!   -32768..32767, INTEGER/LONG 32-bit signed, DWORD 32-bit unsigned.
//! * SINGLE and DOUBLE hold floating-point numbers (SINGLE keeps double
//!   precision for now).
//! * A string stored into a number converts like VAL; TRUE is -1. Arrays and
//!   objects are left alone.

use crate::Value;

/// A declared numeric type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumKind {
    Byte,
    Word,
    Short,
    /// INTEGER and LONG: 32-bit signed in RapidQ.
    Long,
    Dword,
    /// SINGLE and DOUBLE.
    Double,
}

const ALL: [NumKind; 6] = [NumKind::Byte, NumKind::Word, NumKind::Short, NumKind::Long, NumKind::Dword, NumKind::Double];

impl NumKind {
    /// The kind for a declared type name (`AS INTEGER`), if numeric.
    pub fn of_type(type_name: &str) -> Option<NumKind> {
        Some(match type_name.trim().to_ascii_uppercase().as_str() {
            "BYTE" => NumKind::Byte,
            "WORD" => NumKind::Word,
            "SHORT" => NumKind::Short,
            "INTEGER" | "LONG" => NumKind::Long,
            "DWORD" => NumKind::Dword,
            "SINGLE" | "DOUBLE" => NumKind::Double,
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
        self != NumKind::Double
    }

    /// `n` wrapped to this integer type's width.
    #[inline]
    pub fn wrap(self, n: i64) -> i64 {
        match self {
            NumKind::Byte => n as u8 as i64,
            NumKind::Word => n as u16 as i64,
            NumKind::Short => n as i16 as i64,
            NumKind::Long => n as i32 as i64,
            NumKind::Dword => n as u32 as i64,
            NumKind::Double => n,
        }
    }
}

/// A float rounded half to even, as an integer (NaN → 0; out of range
/// saturates before wrapping).
#[inline]
pub fn round_to_int(f: f64) -> i64 {
    if f.is_nan() {
        0
    } else {
        f.round_ties_even() as i64
    }
}

/// `v` stored into a variable of kind `kind`.
#[inline]
pub fn convert(v: &Value, kind: NumKind) -> Value {
    match (kind, v) {
        (NumKind::Double, Value::Double(_)) => v.clone(),
        (NumKind::Double, Value::Array(_) | Value::Object(_)) => v.clone(),
        (NumKind::Double, _) => Value::Double(v.to_f64()),
        (_, Value::Integer(n)) => Value::Integer(kind.wrap(*n)),
        (_, Value::Array(_) | Value::Object(_)) => v.clone(),
        (_, _) => Value::Integer(kind.wrap(round_to_int(v.to_f64()))),
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

// ---- Native builds' typed variables (rapidr-codegen-rust `typed`) ----
//
// A variable native code keeps as an `i64` / `f64` works with these, which
// match `Value`'s operators exactly (`/` and `\` by zero give 0, float
// comparisons treat NaN as equal like `Value::cmp_ord`).

/// `v` stored into an integer variable of `kind`, as its number.
#[inline]
pub fn int_of(v: &Value, kind: NumKind) -> i64 {
    match v {
        Value::Integer(n) => kind.wrap(*n),
        Value::Array(_) | Value::Object(_) => 0,
        _ => kind.wrap(round_to_int(v.to_f64())),
    }
}

/// `v` stored into a SINGLE/DOUBLE variable, as its number.
#[inline]
pub fn double_of(v: &Value) -> f64 {
    v.to_f64()
}

/// `a / b` (always floating point; 0 when `b` is 0).
#[inline]
pub fn fdiv(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        0.0
    } else {
        a / b
    }
}

/// `a \ b` on integers (0 when `b` is 0; wrapping).
#[inline]
pub fn idiv(a: i64, b: i64) -> i64 {
    if b == 0 {
        0
    } else {
        a.wrapping_div(b)
    }
}

/// `a MOD b` on integers (0 when `b` is 0; wrapping).
#[inline]
pub fn imod(a: i64, b: i64) -> i64 {
    if b == 0 {
        0
    } else {
        a.wrapping_rem(b)
    }
}

/// `a MOD b` on floats (0 when `b` is 0).
#[inline]
pub fn fmod(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        0.0
    } else {
        a % b
    }
}

/// `a <= b` as `Value` compares numbers (NaN compares equal).
#[inline]
pub fn le(a: f64, b: f64) -> bool {
    !matches!(a.partial_cmp(&b), Some(std::cmp::Ordering::Greater))
}

/// `a >= b` as `Value` compares numbers (NaN compares equal).
#[inline]
pub fn ge(a: f64, b: f64) -> bool {
    !matches!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_dbl, v_int, v_str};

    fn long(v: Value) -> i64 {
        convert(&v, NumKind::Long).to_i64()
    }

    #[test]
    fn integers_round_half_to_even() {
        // RapidQ's ROUND documentation (IEEE round-to-even).
        for (f, n) in [(2.5, 2), (3.5, 4), (-2.5, -2), (-1.5, -2), (0.5, 0), (1.5, 2), (2.7, 3), (-2.7, -3), (3.49, 3)] {
            assert_eq!(long(v_dbl(f)), n, "{f}");
        }
    }

    #[test]
    fn integers_wrap_to_their_width() {
        assert_eq!(long(v_int(2147483648)), -2147483648);
        assert_eq!(convert(&v_int(300), NumKind::Byte).to_i64(), 44);
        assert_eq!(convert(&v_int(-1), NumKind::Byte).to_i64(), 255);
        assert_eq!(convert(&v_int(40000), NumKind::Short).to_i64(), -25536);
        assert_eq!(convert(&v_int(70000), NumKind::Word).to_i64(), 4464);
        assert_eq!(convert(&v_int(-1), NumKind::Dword).to_i64(), 4294967295);
    }

    #[test]
    fn other_values() {
        assert_eq!(long(v_str("12")), 12);
        assert_eq!(long(Value::Boolean(true)), -1);
        assert_eq!(long(Value::Null), 0);
        assert_eq!(long(v_dbl(f64::NAN)), 0);
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
        assert_eq!(NumKind::of_type("STRING"), None);
    }

    #[test]
    fn typed_helpers_match_value_operators() {
        let nums = [0.0, 1.0, -1.0, 2.5, -7.0, 7.0, 1e300, f64::NAN, 3.0];
        for &a in &nums {
            for &b in &nums {
                let (va, vb) = (v_dbl(a), v_dbl(b));
                let same = |x: f64, y: Value| (x.is_nan() && y.to_f64().is_nan()) || x == y.to_f64();
                assert!(same(fdiv(a, b), &va / &vb), "{a} / {b}");
                assert!(same(fmod(a, b), &va % &vb), "{a} mod {b}");
                assert_eq!(le(a, b), va.rp_le(&vb).to_bool(), "{a} <= {b}");
                assert_eq!(ge(a, b), va.rp_ge(&vb).to_bool(), "{a} >= {b}");
            }
        }
        for (a, b) in [(7, 2), (-7, 2), (7, 0), (i64::MIN, -1), (0, 5)] {
            assert_eq!(idiv(a, b), v_int(a).int_div(&v_int(b)).to_i64());
            assert_eq!(imod(a, b), (&v_int(a) % &v_int(b)).to_i64());
        }
        assert_eq!(int_of(&v_dbl(2.5), NumKind::Long), 2);
        assert_eq!(int_of(&v_str("300"), NumKind::Byte), 44);
    }
}
