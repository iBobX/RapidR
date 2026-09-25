//! The `Value` type — a dynamically-typed BASIC value.

use std::cell::RefCell;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::rc::Rc;

pub mod strings;
pub mod variadic;
pub mod data;
pub mod console;

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    Double(f64),
    String(String),
    Boolean(bool),
    Null,
    /// A DIMmed array. Shared (cloning the Value aliases the same array), so
    /// an array passed to a SUB is modified in place, as BASIC arrays are.
    Array(Rc<RefCell<BasicArray>>),
}

/// Largest array allowed (elements), so `DIM a(1E12)` fails cleanly instead
/// of exhausting memory.
pub const MAX_ARRAY_ELEMENTS: usize = 50_000_000;

/// A BASIC array: any number of dimensions, each with inclusive bounds
/// (`DIM a(10)` → 0..=10, `DIM b(1 TO 5, 3)` → 1..=5 × 0..=3), row-major.
#[derive(Debug, Clone)]
pub struct BasicArray {
    pub bounds: Vec<(i64, i64)>,
    pub data: Vec<Value>,
}

impl BasicArray {
    pub fn new(bounds: Vec<(i64, i64)>, fill: Value) -> Result<Self, String> {
        if bounds.is_empty() {
            return Err("an array needs at least one dimension".into());
        }
        let mut len: usize = 1;
        for &(lo, hi) in &bounds {
            if hi < lo {
                return Err(format!("array bounds {lo} TO {hi}: the upper bound is below the lower bound"));
            }
            let extent = usize::try_from(hi - lo + 1).map_err(|_| "array too large".to_string())?;
            len = len
                .checked_mul(extent)
                .filter(|&n| n <= MAX_ARRAY_ELEMENTS)
                .ok_or_else(|| format!("array too large (limit {MAX_ARRAY_ELEMENTS} elements)"))?;
        }
        Ok(Self { bounds, data: vec![fill; len] })
    }

    fn offset(&self, indices: &[i64]) -> Result<usize, String> {
        if indices.len() != self.bounds.len() {
            return Err(format!(
                "array has {} dimension(s) but {} index(es) were given",
                self.bounds.len(),
                indices.len()
            ));
        }
        let mut offset: usize = 0;
        for (&i, &(lo, hi)) in indices.iter().zip(&self.bounds) {
            if i < lo || i > hi {
                return Err(format!("Subscript out of range: index {i} is outside {lo} TO {hi}"));
            }
            offset = offset * (hi - lo + 1) as usize + (i - lo) as usize;
        }
        Ok(offset)
    }

    pub fn get(&self, indices: &[i64]) -> Result<Value, String> {
        Ok(self.data[self.offset(indices)?].clone())
    }

    pub fn set(&mut self, indices: &[i64], value: Value) -> Result<(), String> {
        let i = self.offset(indices)?;
        self.data[i] = value;
        Ok(())
    }

    /// Bounds of 1-based dimension `dim` (LBOUND/UBOUND's second argument).
    pub fn dim_bounds(&self, dim: usize) -> Option<(i64, i64)> {
        dim.checked_sub(1).and_then(|d| self.bounds.get(d)).copied()
    }
}

// --- Convenience constructors ---

pub fn v_int(n: i64) -> Value {
    Value::Integer(n)
}
pub fn v_dbl(n: f64) -> Value {
    Value::Double(n)
}
pub fn v_str(s: &str) -> Value {
    Value::String(s.to_string())
}
/// Run-time error in compiled (codegen) programs: report it BASIC-style and
/// stop, rather than continuing with a wrong value.
fn runtime_error(message: &str) -> ! {
    eprintln!("run-time error: {message}");
    std::process::exit(1);
}

/// `DIM a(…)` in compiled programs.
pub fn rp_new_array(bounds: &[(i64, i64)], fill: Value) -> Value {
    v_array(bounds.to_vec(), fill).unwrap_or_else(|e| runtime_error(&e))
}

impl Value {
    /// `a(i, j)` in compiled programs.
    pub fn rp_get(&self, indices: &[i64]) -> Value {
        match self {
            Value::Array(a) => a.borrow().get(indices).unwrap_or_else(|e| runtime_error(&e)),
            other if indices.len() == 1 => other.rp_index(&Value::Integer(indices[0])),
            _ => runtime_error("indexing a value that is not an array"),
        }
    }

    /// `a(i, j) = v` in compiled programs (in place: arrays are shared).
    pub fn rp_set(&self, indices: &[i64], value: Value) {
        match self {
            Value::Array(a) => a.borrow_mut().set(indices, value).unwrap_or_else(|e| runtime_error(&e)),
            _ => runtime_error("assigning to an element of a variable that is not an array (DIM it with a size first)"),
        }
    }

    /// LBOUND(a [, dim]) / UBOUND(a [, dim]) in compiled programs.
    pub fn rp_bound(&self, dim: &Value, upper: bool) -> Value {
        let d = if matches!(dim, Value::Null) { 1 } else { dim.to_i64() };
        match array_bound(self, d, upper) {
            Some(b) => Value::Integer(b),
            None => runtime_error(&format!(
                "{}: argument is not an array, or it has no dimension {d}",
                if upper { "UBOUND" } else { "LBOUND" }
            )),
        }
    }
}

/// LBOUND/UBOUND of dimension `dim` (1-based) when `v` is an array.
pub fn array_bound(v: &Value, dim: i64, upper: bool) -> Option<i64> {
    let Value::Array(a) = v else { return None };
    let (lo, hi) = a.borrow().dim_bounds(usize::try_from(dim).ok()?)?;
    Some(if upper { hi } else { lo })
}

pub fn v_bool(b: bool) -> Value {
    Value::Boolean(b)
}
pub fn v_null() -> Value {
    Value::Null
}
pub fn v_array(bounds: Vec<(i64, i64)>, fill: Value) -> Result<Value, String> {
    Ok(Value::Array(Rc::new(RefCell::new(BasicArray::new(bounds, fill)?))))
}

// --- Conversions ---

impl Value {
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Integer(n) => *n != 0,
            Value::Double(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Boolean(b) => *b,
            Value::Null | Value::Array(_) => false,
        }
    }

    pub fn to_i64(&self) -> i64 {
        match self {
            Value::Integer(n) => *n,
            Value::Double(n) => *n as i64,
            Value::String(s) => s.parse::<i64>().unwrap_or(0),
            Value::Boolean(b) => if *b { -1 } else { 0 },
            Value::Null | Value::Array(_) => 0,
        }
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Value::Integer(n) => *n as f64,
            Value::Double(n) => *n,
            Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
            Value::Boolean(b) => if *b { -1.0 } else { 0.0 },
            Value::Null | Value::Array(_) => 0.0,
        }
    }

    pub fn to_string_val(&self) -> String {
        match self {
            Value::Integer(n) => n.to_string(),
            Value::Double(n) => {
                // BASIC shows whole numbers without a decimal part:
                // PRINT 2 ^ 10 → 1024, not 1024.0.
                if *n == n.trunc() && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    n.to_string()
                }
            }
            Value::String(s) => s.clone(),
            // RapidQ has no boolean type: a comparison is -1 (true) or 0.
            Value::Boolean(b) => if *b { "-1".to_string() } else { "0".to_string() },
            Value::Null => String::new(),
            // Elements joined by commas: what runtime components received
            // back when interpreter arrays were comma-separated strings.
            Value::Array(a) => a.borrow().data.iter().map(|v| v.to_string_val()).collect::<Vec<_>>().join(","),
        }
    }

    /// BASIC integer division  (a \ b)
    pub fn int_div(&self, rhs: &Value) -> Value {
        let a = self.to_i64();
        let b = rhs.to_i64();
        if b == 0 { Value::Integer(0) } else { Value::Integer(a / b) }
    }

    /// BASIC exponentiation (a ^ b)
    pub fn power(&self, rhs: &Value) -> Value {
        Value::Double(self.to_f64().powf(rhs.to_f64()))
    }

    /// Index into a Value (for variant array access like `listA(i)`).
    /// Strings are treated as comma-separated arrays.
    pub fn rp_index(&self, idx: &Value) -> Value {
        let i = idx.to_i64();
        match self {
            Value::Array(a) => a.borrow().get(&[i]).unwrap_or(Value::Null),
            Value::String(s) => {
                // Try comma-separated splitting
                let parts: Vec<&str> = s.split(',').collect();
                if i >= 0 && (i as usize) < parts.len() {
                    Value::String(parts[i as usize].trim().to_string())
                } else {
                    Value::Null
                }
            }
            _ => Value::Null,
        }
    }

    /// BASIC string concatenation (&)
    pub fn concat(&self, rhs: &Value) -> Value {
        Value::String(format!("{}{}", self.to_string_val(), rhs.to_string_val()))
    }

    /// BASIC logical AND
    pub fn and(&self, rhs: &Value) -> Value {
        Value::Boolean(self.to_bool() && rhs.to_bool())
    }

    /// BASIC logical OR
    pub fn or(&self, rhs: &Value) -> Value {
        Value::Boolean(self.to_bool() || rhs.to_bool())
    }

    /// BASIC logical XOR
    pub fn xor(&self, rhs: &Value) -> Value {
        Value::Boolean(self.to_bool() ^ rhs.to_bool())
    }

    /// BASIC logical NOT
    pub fn not(&self) -> Value {
        Value::Boolean(!self.to_bool())
    }

    // Comparisons — return Value::Boolean for use in expressions,
    // but also impl PartialEq / PartialOrd below.

    pub fn rp_eq(&self, rhs: &Value) -> Value {
        Value::Boolean(self.cmp_eq(rhs))
    }

    pub fn rp_ne(&self, rhs: &Value) -> Value {
        Value::Boolean(!self.cmp_eq(rhs))
    }

    pub fn rp_lt(&self, rhs: &Value) -> Value {
        Value::Boolean(self.cmp_ord(rhs) == std::cmp::Ordering::Less)
    }

    pub fn rp_le(&self, rhs: &Value) -> Value {
        Value::Boolean(matches!(self.cmp_ord(rhs), std::cmp::Ordering::Less | std::cmp::Ordering::Equal))
    }

    pub fn rp_gt(&self, rhs: &Value) -> Value {
        Value::Boolean(self.cmp_ord(rhs) == std::cmp::Ordering::Greater)
    }

    pub fn rp_ge(&self, rhs: &Value) -> Value {
        Value::Boolean(matches!(self.cmp_ord(rhs), std::cmp::Ordering::Greater | std::cmp::Ordering::Equal))
    }

    // --- internal comparison helpers ---

    fn cmp_eq(&self, rhs: &Value) -> bool {
        match (self, rhs) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::String(a), _) => *a == rhs.to_string_val(),
            (_, Value::String(b)) => self.to_string_val() == *b,
            _ => self.to_f64() == rhs.to_f64(),
        }
    }

    fn cmp_ord(&self, rhs: &Value) -> std::cmp::Ordering {
        match (self, rhs) {
            (Value::String(a), Value::String(b)) => a.cmp(b),
            _ => self.to_f64().partial_cmp(&rhs.to_f64()).unwrap_or(std::cmp::Ordering::Equal),
        }
    }
}

// --- Display ---

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_val())
    }
}

// --- PartialEq ---

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.cmp_eq(other)
    }
}

// --- PartialOrd ---

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp_ord(other))
    }
}

// --- Arithmetic ops ---

impl Add for &Value {
    type Output = Value;
    fn add(self, rhs: Self) -> Value {
        match (self, rhs) {
            (Value::String(a), Value::String(b)) => Value::String(format!("{a}{b}")),
            (Value::String(a), _) => Value::String(format!("{a}{}", rhs.to_string_val())),
            (_, Value::String(b)) => Value::String(format!("{}{b}", self.to_string_val())),
            (Value::Integer(a), Value::Integer(b)) => Value::Integer(a.wrapping_add(*b)),
            _ => Value::Double(self.to_f64() + rhs.to_f64()),
        }
    }
}

impl Sub for &Value {
    type Output = Value;
    fn sub(self, rhs: Self) -> Value {
        match (self, rhs) {
            // RapidQ: "jello" - "l" removes every "l" → "jeo".
            (Value::String(a), Value::String(b)) if !b.is_empty() => Value::String(a.replace(b.as_str(), "")),
            (Value::String(a), Value::String(_)) => Value::String(a.clone()),
            (Value::Integer(a), Value::Integer(b)) => Value::Integer(a.wrapping_sub(*b)),
            _ => Value::Double(self.to_f64() - rhs.to_f64()),
        }
    }
}

impl Mul for &Value {
    type Output = Value;
    fn mul(self, rhs: Self) -> Value {
        match (self, rhs) {
            (Value::Integer(a), Value::Integer(b)) => Value::Integer(a.wrapping_mul(*b)),
            _ => Value::Double(self.to_f64() * rhs.to_f64()),
        }
    }
}

impl Div for &Value {
    type Output = Value;
    fn div(self, rhs: Self) -> Value {
        let b = rhs.to_f64();
        if b == 0.0 { Value::Double(0.0) } else { Value::Double(self.to_f64() / b) }
    }
}

impl Rem for &Value {
    type Output = Value;
    fn rem(self, rhs: Self) -> Value {
        match (self, rhs) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b == 0 { Value::Integer(0) } else { Value::Integer(a % b) }
            }
            _ => {
                let b = rhs.to_f64();
                if b == 0.0 { Value::Double(0.0) } else { Value::Double(self.to_f64() % b) }
            }
        }
    }
}

impl Neg for &Value {
    type Output = Value;
    fn neg(self) -> Value {
        match self {
            Value::Integer(n) => Value::Integer(-n),
            _ => Value::Double(-self.to_f64()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_arithmetic() {
        assert_eq!(&v_int(3) + &v_int(4), v_int(7));
        assert_eq!(&v_int(10) - &v_int(3), v_int(7));
        assert_eq!(&v_int(6) * &v_int(7), v_int(42));
    }

    #[test]
    fn string_concat() {
        let a = v_str("Hello ");
        let b = v_str("World");
        assert_eq!((&a + &b).to_string_val(), "Hello World");
        assert_eq!(a.concat(&b).to_string_val(), "Hello World");
    }

    #[test]
    fn comparisons() {
        assert!(v_int(5).rp_gt(&v_int(3)).to_bool());
        assert!(v_int(3).rp_le(&v_int(5)).to_bool());
        assert!(v_str("abc").rp_eq(&v_str("abc")).to_bool());
        assert!(v_str("abc").rp_ne(&v_str("xyz")).to_bool());
    }

    #[test]
    fn power_and_int_div() {
        assert_eq!(v_int(2).power(&v_int(10)).to_f64(), 1024.0);
        assert_eq!(v_int(7).int_div(&v_int(2)), v_int(3));
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", v_int(42)), "42");
        assert_eq!(format!("{}", v_str("hello")), "hello");
    }
}

/// `a SHL n` (RapidQ): shift the 32-bit value left; bits past bit 31 are
/// lost and the result is a signed LONG, like RapidQ's integers.
pub fn rp_shl(a: &Value, n: &Value) -> Value {
    let shift = n.to_i64().clamp(0, 32) as u32;
    let bits = a.to_i64() as u32;
    Value::Integer(if shift >= 32 { 0 } else { (bits << shift) as i32 as i64 })
}

/// `a SHR n` (RapidQ): logical shift right of the 32-bit value, as a LONG.
pub fn rp_shr(a: &Value, n: &Value) -> Value {
    let shift = n.to_i64().clamp(0, 32) as u32;
    let bits = a.to_i64() as u32;
    Value::Integer(if shift >= 32 { 0 } else { (bits >> shift) as i32 as i64 })
}
