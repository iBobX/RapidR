//! The `Value` type — a dynamically-typed BASIC value.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    Double(f64),
    String(String),
    Boolean(bool),
    Null,
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
pub fn v_bool(b: bool) -> Value {
    Value::Boolean(b)
}
pub fn v_null() -> Value {
    Value::Null
}

// --- Conversions ---

impl Value {
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Integer(n) => *n != 0,
            Value::Double(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Boolean(b) => *b,
            Value::Null => false,
        }
    }

    pub fn to_i64(&self) -> i64 {
        match self {
            Value::Integer(n) => *n,
            Value::Double(n) => *n as i64,
            Value::String(s) => s.parse::<i64>().unwrap_or(0),
            Value::Boolean(b) => if *b { -1 } else { 0 },
            Value::Null => 0,
        }
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Value::Integer(n) => *n as f64,
            Value::Double(n) => *n,
            Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
            Value::Boolean(b) => if *b { -1.0 } else { 0.0 },
            Value::Null => 0.0,
        }
    }

    pub fn to_string_val(&self) -> String {
        match self {
            Value::Integer(n) => n.to_string(),
            Value::Double(n) => {
                if *n == n.trunc() && n.abs() < 1e15 {
                    format!("{:.1}", n)
                } else {
                    n.to_string()
                }
            }
            Value::String(s) => s.clone(),
            Value::Boolean(b) => if *b { "True".to_string() } else { "False".to_string() },
            Value::Null => String::new(),
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
