//! What compiled code knows about a value: its kind, fixed at each point
//! of the code by the bytecode and the kinds it was entered with
//! (docs/jit-design.md §2), and how a value of that kind travels as 64
//! bits between compiled code and the VM.

use rapidr_value::Value;

/// A value's kind as compiled code sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum K {
    Null,
    Bool,
    /// `Value::Integer`: an `i64` register.
    Int,
    /// `Value::Double`: an `f64` register.
    Dbl,
    /// An array (in a local or a global the code only indexes: compiled
    /// code reaches its elements through the helpers).
    Arr,
    /// Text, an object: compiled code doesn't touch it.
    Other,
}

impl K {
    pub fn of(v: &Value) -> K {
        match v {
            Value::Null => K::Null,
            Value::Boolean(_) => K::Bool,
            Value::Integer(_) => K::Int,
            Value::Double(_) => K::Dbl,
            Value::Array(_) => K::Arr,
            Value::String(_) | Value::Object(_) => K::Other,
        }
    }

    /// Null, a boolean or a number: a value compiled code holds.
    pub fn scalar(self) -> bool {
        matches!(self, K::Null | K::Bool | K::Int | K::Dbl)
    }

    /// Its code for the helpers.
    pub fn code(self) -> u32 {
        match self {
            K::Null => 0,
            K::Bool => 1,
            K::Int => 2,
            K::Dbl => 3,
            K::Arr => 4,
            K::Other => 5,
        }
    }

    pub fn from_code(c: u32) -> K {
        match c {
            0 => K::Null,
            1 => K::Bool,
            2 => K::Int,
            3 => K::Dbl,
            4 => K::Arr,
            _ => K::Other,
        }
    }
}

/// A scalar's 64 bits (Null 0, a boolean 0 or 1, an Integer's `i64`, a
/// Double's IEEE bits).
pub fn bits_of(v: &Value) -> u64 {
    match v {
        Value::Boolean(b) => u64::from(*b),
        Value::Integer(n) => *n as u64,
        Value::Double(f) => f.to_bits(),
        _ => 0,
    }
}

/// The value of kind `k` (a scalar) whose bits are `bits`.
pub fn value_of(k: K, bits: u64) -> Value {
    match k {
        K::Bool => Value::Boolean(bits & 0xFF != 0),
        K::Int => Value::Integer(bits as i64),
        K::Dbl => Value::Double(f64::from_bits(bits)),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_go_there_and_back() {
        for v in [Value::Null, Value::Boolean(true), Value::Boolean(false), Value::Integer(i64::MIN), Value::Integer(-1), Value::Double(-0.0), Value::Double(f64::NAN), Value::Double(1e300)] {
            let k = K::of(&v);
            let back = value_of(k, bits_of(&v));
            assert_eq!(K::of(&back), k);
            assert_eq!(bits_of(&back), bits_of(&v), "{v:?}");
            assert_eq!(K::from_code(k.code()), k);
        }
    }
}
