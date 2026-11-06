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
    /// `Value::Int64` (INT64, UINT32): an `i64` register.
    I64,
    /// `Value::UInt64`: an `i64` register holding its bits.
    U64,
    /// A local whose kind differs on the paths that meet at a point (a
    /// `DIM` inside one `CASE` branch: an Integer there, Null on the other
    /// way): its 64 bits and its kind travel at run time, two registers,
    /// and an exit writes back exactly what it holds. Compiled code stores
    /// into it (it becomes of the stored kind) but doesn't read it as a
    /// number: a read leaves for the interpreter. Never a stack entry, a
    /// parameter or a global; `K::of` never gives it.
    Dyn,
}

impl K {
    pub fn of(v: &Value) -> K {
        match v {
            Value::Null => K::Null,
            Value::Boolean(_) => K::Bool,
            Value::Integer(_) => K::Int,
            Value::Double(_) => K::Dbl,
            Value::Array(_) => K::Arr,
            Value::Int64(_) => K::I64,
            Value::UInt64(_) => K::U64,
            Value::String(_) | Value::Object(_) => K::Other,
        }
    }

    /// Null, a boolean or a number: a value compiled code holds.
    pub fn scalar(self) -> bool {
        matches!(self, K::Null | K::Bool | K::Int | K::Dbl | K::I64 | K::U64)
    }

    /// INT64 / UINT64 (RapidR's 64-bit kinds, docs/big-numbers-design.md).
    pub fn wide(self) -> bool {
        matches!(self, K::I64 | K::U64)
    }

    /// A whole number in an `i64` register: an Integer, an INT64, a UINT64.
    pub fn whole(self) -> bool {
        matches!(self, K::Int | K::I64 | K::U64)
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
            K::I64 => 6,
            K::U64 => 7,
            K::Dyn => 8,
        }
    }

    /// The 64-bit words a value of this kind takes in an exit's buffer:
    /// none for Null (nor for an array: the exit's record names the global
    /// that holds it), its bits and its kind's code for a [`K::Dyn`].
    pub fn words(self) -> usize {
        match self {
            K::Null | K::Arr | K::Other => 0,
            K::Dyn => 2,
            _ => 1,
        }
    }

    /// Two paths meeting with these kinds of a local: the kind it has
    /// there — the same one, or [`K::Dyn`] for two scalars (or a `Dyn`)
    /// that differ; `None` when one is an array or text (the second path
    /// leaves for the interpreter).
    pub fn meet(self, other: K) -> Option<K> {
        if self == other {
            return Some(self);
        }
        let joins = |k: K| k.scalar() || k == K::Dyn;
        (joins(self) && joins(other)).then_some(K::Dyn)
    }

    pub fn from_code(c: u32) -> K {
        match c {
            0 => K::Null,
            1 => K::Bool,
            2 => K::Int,
            3 => K::Dbl,
            4 => K::Arr,
            6 => K::I64,
            7 => K::U64,
            _ => K::Other,
        }
    }
}

/// A scalar's 64 bits (Null 0, a boolean 0 or 1, an Integer's `i64`, a
/// Double's IEEE bits).
pub fn bits_of(v: &Value) -> u64 {
    match v {
        Value::Boolean(b) => u64::from(*b),
        Value::Integer(n) | Value::Int64(n) => *n as u64,
        Value::UInt64(n) => *n,
        Value::Double(f) => f.to_bits(),
        _ => 0,
    }
}

/// The value of kind `k` (a scalar) whose bits are `bits`.
pub fn value_of(k: K, bits: u64) -> Value {
    match k {
        K::Bool => Value::Boolean(bits & 0xFF != 0),
        K::Int => Value::Integer(bits as i64),
        K::I64 => Value::Int64(bits as i64),
        K::U64 => Value::UInt64(bits),
        K::Dbl => Value::Double(f64::from_bits(bits)),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_go_there_and_back() {
        for v in [Value::Null, Value::Boolean(true), Value::Boolean(false), Value::Integer(i64::MIN), Value::Integer(-1), Value::Double(-0.0), Value::Double(f64::NAN), Value::Double(1e300), Value::Int64(i64::MIN), Value::UInt64(u64::MAX)] {
            let k = K::of(&v);
            let back = value_of(k, bits_of(&v));
            assert_eq!(K::of(&back), k);
            assert_eq!(bits_of(&back), bits_of(&v), "{v:?}");
            assert_eq!(K::from_code(k.code()), k);
        }
    }
}
