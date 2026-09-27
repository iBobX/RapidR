//! The `Value` type — a dynamically-typed BASIC value.

use std::cell::RefCell;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::rc::Rc;

pub mod strings;
pub mod numeric;
pub mod variadic;
pub mod data;
pub mod console;
pub mod dialogs;
pub mod objects;
pub mod layout;

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
    /// An instance of a user TYPE: shared by reference, its fields in fixed
    /// slots resolved at compile time (see [`Instance`]).
    Object(Rc<Instance>),
}

/// An instance of a user TYPE (RapidQ manual ch. 10). Both backends lower
/// objects the same way (`rapidr_ast::objects`): every field has a slot
/// number fixed at compile time, ancestors' fields first, so reading or
/// setting a field is an index into `fields` — no lookup by name.
///
/// `id` names the instance (`c`, `a(2)`, `c.Engine`): what PRINT shows, and
/// the id of the component when the TYPE EXTENDS one (a TYPE EXTENDS QFORM
/// *is* that form, whose properties live in the runtime's component
/// registry under this id).
#[derive(Debug)]
pub struct Instance {
    pub id: String,
    pub type_name: String,
    /// Field names by slot (shared by every instance of the type): for the
    /// debugger and messages, never for access.
    pub names: Rc<[String]>,
    pub fields: RefCell<Vec<Value>>,
}

thread_local! {
    static FIELD_NAMES: RefCell<std::collections::HashMap<String, Rc<[String]>>> = RefCell::new(Default::default());
}

impl Instance {
    /// A new instance of `type_name` whose slots are `names` (comma
    /// separated, as the compilers pass them), all Null.
    pub fn new(id: &str, type_name: &str, names: &str) -> Rc<Self> {
        let names = FIELD_NAMES.with(|c| {
            c.borrow_mut()
                .entry(format!("{type_name}:{names}"))
                .or_insert_with(|| names.split(',').filter(|n| !n.is_empty()).map(str::to_string).collect::<Vec<_>>().into())
                .clone()
        });
        let fields = RefCell::new(vec![Value::Null; names.len()]);
        Rc::new(Self { id: id.to_string(), type_name: type_name.to_string(), names, fields })
    }

    pub fn get(&self, slot: usize) -> Value {
        self.fields.borrow().get(slot).cloned().unwrap_or(Value::Null)
    }

    pub fn set(&self, slot: usize, value: Value) {
        if let Some(f) = self.fields.borrow_mut().get_mut(slot) {
            *f = value;
        }
    }
}

/// `__newobject(id, type, names)` in compiled code: a new instance.
pub fn rp_new_object(id: &Value, type_name: &Value, names: &Value) -> Value {
    Value::Object(Instance::new(&id.to_string_val(), &type_name.to_string_val(), &names.to_string_val()))
}

/// `__objectarray(name, type, names, lo, hi, …)` in compiled code.
pub fn rp_new_object_array(name: &Value, type_name: &Value, names: &Value, bounds: &[(i64, i64)]) -> Value {
    new_object_array(&name.to_string_val(), &type_name.to_string_val(), &names.to_string_val(), bounds).unwrap_or_else(|e| runtime_error(&e))
}

/// A value as JSON for debuggers: arrays as lists, objects as
/// `{"$type":…, "$id":…, field: value, …}` (nested objects up to a depth).
pub fn debug_json(v: &Value) -> String {
    fn go(v: &Value, depth: usize) -> String {
        match v {
            Value::Null => "null".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Integer(n) => n.to_string(),
            Value::Double(d) => d.to_string(),
            Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")),
            Value::Array(a) => format!("[{}]", a.borrow().data.iter().map(|x| go(x, depth)).collect::<Vec<_>>().join(",")),
            Value::Object(o) if depth >= 4 => format!("\"<{}>\"", o.id),
            Value::Object(o) => {
                let mut parts = vec![format!("\"$type\":\"{}\"", o.type_name), format!("\"$id\":\"{}\"", o.id)];
                for (name, f) in o.names.iter().zip(o.fields.borrow().iter()) {
                    parts.push(format!("\"{name}\":{}", go(f, depth + 1)));
                }
                format!("{{{}}}", parts.join(","))
            }
        }
    }
    go(v, 0)
}

/// Field `slot` of an object (Null for anything else) — compiled field reads.
pub fn obj_field(obj: &Value, slot: usize) -> Value {
    match obj {
        Value::Object(o) => o.get(slot),
        _ => runtime_error(&format!("this variable does not refer to an object (reading field {slot} of {})", describe(obj))),
    }
}

/// Sets field `slot` of an object — compiled field stores.
pub fn set_obj_field(obj: &Value, slot: usize, value: Value) {
    match obj {
        Value::Object(o) => o.set(slot, value),
        _ => runtime_error(&format!("this variable does not refer to an object (setting field {slot} of {})", describe(obj))),
    }
}

fn describe(v: &Value) -> String {
    match v {
        Value::Null => "an empty variable".into(),
        other => format!("\"{}\"", other.to_string_val()),
    }
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
pub fn runtime_error(message: &str) -> ! {
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
            Value::Object(_) => true,
        }
    }

    pub fn to_i64(&self) -> i64 {
        match self {
            Value::Integer(n) => *n,
            Value::Double(n) => *n as i64,
            Value::String(s) => s.parse::<i64>().unwrap_or(0),
            Value::Boolean(b) => if *b { -1 } else { 0 },
            Value::Null | Value::Array(_) | Value::Object(_) => 0,
        }
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Value::Integer(n) => *n as f64,
            Value::Double(n) => *n,
            Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
            Value::Boolean(b) => if *b { -1.0 } else { 0.0 },
            Value::Null | Value::Array(_) | Value::Object(_) => 0.0,
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
            Value::Object(o) => o.id.clone(),
        }
    }

    /// BASIC integer division  (a \ b)
    pub fn int_div(&self, rhs: &Value) -> Value {
        let a = self.to_i64();
        let b = rhs.to_i64();
        // Wrapping: `MIN \ -1` must not crash the program.
        if b == 0 { Value::Integer(0) } else { Value::Integer(a.wrapping_div(b)) }
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
    /// AND/OR/XOR/NOT are bitwise on integers, as in RapidQ (manual,
    /// Appendix C: `5 AND 3 = 1`, `5 OR 3 = 7`, `NOT -1 = 0`). Comparisons
    /// are -1/0, so the same operators combine conditions. Two booleans
    /// give a boolean (which prints as -1/0 anyway).
    fn bitwise(&self, rhs: &Value, op: fn(i64, i64) -> i64) -> Value {
        match (self, rhs) {
            (Value::Boolean(a), Value::Boolean(b)) => Value::Boolean(op(-(*a as i64), -(*b as i64)) != 0),
            _ => Value::Integer(op(self.bits(), rhs.bits())),
        }
    }

    /// The integer bitwise operators work on (true = -1, reals rounded).
    fn bits(&self) -> i64 {
        match self {
            Value::Boolean(b) => -(*b as i64),
            Value::Double(d) => d.round() as i64,
            other => other.to_i64(),
        }
    }

    /// BASIC AND (bitwise)
    pub fn and(&self, rhs: &Value) -> Value {
        self.bitwise(rhs, |a, b| a & b)
    }

    /// BASIC OR (bitwise)
    pub fn or(&self, rhs: &Value) -> Value {
        self.bitwise(rhs, |a, b| a | b)
    }

    /// BASIC XOR (bitwise)
    pub fn xor(&self, rhs: &Value) -> Value {
        self.bitwise(rhs, |a, b| a ^ b)
    }

    /// BASIC NOT (bitwise complement: NOT 0 = -1, NOT -1 = 0, NOT 5 = -6)
    pub fn not(&self) -> Value {
        match self {
            Value::Boolean(b) => Value::Boolean(!b),
            other => Value::Integer(!other.bits()),
        }
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
            // The same instance; an object and a name compare by its id
            // (`IF Sender = Btn1`).
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Object(o), other) | (other, Value::Object(o)) => o.id == other.to_string_val(),
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
            (Value::String(a), Value::String(b)) => joined(a, b),
            (Value::String(a), _) => joined(a, &rhs.to_string_val()),
            (_, Value::String(b)) => joined(&self.to_string_val(), b),
            (Value::Integer(a), Value::Integer(b)) => Value::Integer(a.wrapping_add(*b)),
            _ => Value::Double(self.to_f64() + rhs.to_f64()),
        }
    }
}

/// `a + b` for strings, within [`strings::MAX_STRING_LEN`].
fn joined(a: &str, b: &str) -> Value {
    // By bytes, so the check costs nothing (a character is 1–4 bytes).
    if a.len() + b.len() > strings::MAX_STRING_LEN * 4 {
        runtime_error(&format!("string too long: strings are limited to {} characters", strings::MAX_STRING_LEN));
    }
    let mut out = String::with_capacity(a.len() + b.len());
    out.push_str(a);
    out.push_str(b);
    Value::String(out)
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
                if *b == 0 { Value::Integer(0) } else { Value::Integer(a.wrapping_rem(*b)) }
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
            Value::Integer(n) => Value::Integer(n.wrapping_neg()),
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

/// `REDIM a(bounds) AS T` (RapidQ manual, REDIM): resizes an existing array
/// in place (everyone holding it sees the new size), keeping each element
/// whose index is still inside the new bounds; creates the array when `old`
/// isn't one yet (REDIM without DIM is DIM).
pub fn redim(old: &Value, bounds: &[(i64, i64)], fill: Value) -> Result<Value, String> {
    let fresh = BasicArray::new(bounds.to_vec(), fill)?;
    let Value::Array(existing) = old else {
        return Ok(Value::Array(Rc::new(RefCell::new(fresh))));
    };
    let mut resized = fresh;
    {
        let previous = existing.borrow();
        if previous.bounds.len() == resized.bounds.len() {
            // Walk the old elements in storage order with their indices.
            let mut index: Vec<i64> = previous.bounds.iter().map(|b| b.0).collect();
            for value in &previous.data {
                let _ = resized.set(&index, value.clone());
                for d in (0..index.len()).rev() {
                    index[d] += 1;
                    if index[d] <= previous.bounds[d].1 || d == 0 {
                        break;
                    }
                    index[d] = previous.bounds[d].0;
                }
            }
        }
    }
    *existing.borrow_mut() = resized;
    Ok(old.clone())
}

/// `REDIM` in compiled programs.
pub fn rp_redim(old: &Value, bounds: &[(i64, i64)], fill: Value) -> Value {
    redim(old, bounds, fill).unwrap_or_else(|e| runtime_error(&e))
}

/// `a INV m` (RapidQ): the inverse of `a` modulo `m`, e.g. 3 INV 26 = 9;
/// 0 when there is none.
pub fn rp_inv(a: &Value, m: &Value) -> Value {
    // In 128 bits so no step overflows (`INV(MIN, -1)` must not crash).
    let (a, m) = (a.to_i64() as i128, m.to_i64() as i128);
    if m == 0 {
        return Value::Integer(0);
    }
    let (mut r0, mut r1) = (a.rem_euclid(m.abs()), m.abs());
    let (mut t0, mut t1) = (1i128, 0i128);
    while r1 != 0 {
        let q = r0 / r1;
        (r0, r1) = (r1, r0 - q * r1);
        (t0, t1) = (t1, t0 - q * t1);
    }
    Value::Integer(if r0 == 1 { t0.rem_euclid(m.abs()) as i64 } else { 0 })
}

#[cfg(test)]
mod input_tests {
    use super::*;

    #[test]
    fn input_converts_by_type_then_suffix() {
        let line = v_str("42 apples");
        assert_eq!(input_value(&line, &v_str(""), "").to_string_val(), "42 apples");
        assert_eq!(input_value(&line, &v_int(0), "").to_i64(), 42);
        assert_eq!(input_value(&v_str("3.5"), &v_dbl(0.0), "").to_f64(), 3.5);
        assert_eq!(input_value(&v_str("007"), &Value::Null, "$").to_string_val(), "007");
        assert_eq!(input_value(&v_str("-7.9"), &Value::Null, "%").to_i64(), -7);
        assert!(matches!(input_value(&v_str(" 12 "), &Value::Null, ""), Value::Integer(12)));
        assert!(matches!(input_value(&v_str("Bob"), &Value::Null, ""), Value::String(_)));
        assert_eq!(input_value(&v_str("abc"), &v_int(5), "").to_i64(), 0);
    }
}

#[cfg(test)]
mod redim_tests {
    use super::*;

    #[test]
    fn redim_keeps_data_in_place_and_inv() {
        let a = rp_new_array(&[(0, 2)], Value::Integer(0));
        a.rp_set(&[1], Value::Integer(7));
        let alias = a.clone();
        rp_redim(&a, &[(0, 5)], Value::Integer(0));
        assert_eq!(alias.rp_get(&[1]), Value::Integer(7));
        assert_eq!(alias.rp_get(&[5]), Value::Integer(0));
        rp_redim(&a, &[(0, 0)], Value::Integer(0));
        assert_eq!(array_bound(&a, 1, true), Some(0));
        assert!(matches!(rp_redim(&Value::Null, &[(1, 3)], Value::Null), Value::Array(_)));
        assert_eq!(rp_inv(&Value::Integer(3), &Value::Integer(26)), Value::Integer(9));
        assert_eq!(rp_inv(&Value::Integer(2), &Value::Integer(4)), Value::Integer(0));
    }
}

/// Builtins implemented here that interpreter hosts dispatch before their own
/// tables: DATA/READ/RESTORE and `__redim(old, fill, lo1, hi1, …)`.
/// The value `INPUT var` stores from the line the user typed (RapidQ reads
/// a whole line, "string or numeric"). A variable DIMmed as a number or
/// string starts at 0 or "", so `current` tells its type; otherwise the name's
/// suffix does (`$` text; `%` `&` whole number; `!` `#` number); a plain
/// untyped name gets a number when the line is one, and text otherwise.
pub fn input_value(line: &Value, current: &Value, suffix: &str) -> Value {
    let text = line.to_string_val();
    let number = || leading_number(&text);
    match (current, suffix) {
        (_, "$") | (Value::String(_), "") => v_str(&text),
        (_, "%" | "&") | (Value::Integer(_), "") => v_int(number().map_or(0, |n| n.trunc() as i64)),
        (_, "!" | "#") | (Value::Double(_), "") => v_dbl(number().unwrap_or(0.0)),
        _ => match text.trim().parse::<i64>() {
            Ok(n) => v_int(n),
            Err(_) => text.trim().parse::<f64>().map_or_else(|_| v_str(&text), v_dbl),
        },
    }
}

/// The number at the start of `s`, as VAL reads it ("12abc" → 12).
fn leading_number(s: &str) -> Option<f64> {
    let t = s.trim_start();
    let end = t
        .char_indices()
        .take_while(|&(i, c)| c.is_ascii_digit() || c == '.' || (i == 0 && (c == '-' || c == '+')))
        .map(|(i, c)| i + c.len_utf8())
        .last()?;
    (1..=end).rev().find_map(|e| t[..e].parse::<f64>().ok())
}

/// `DIM a(1 TO 3) AS TType`: an array of new (not yet set-up) instances with
/// ids `a(1)`, … and the fields `fields` (comma separated).
pub fn new_object_array(name: &str, type_name: &str, fields: &str, bounds: &[(i64, i64)]) -> Result<Value, String> {
    let (ids, _) = objects::object_ids(name, bounds)?;
    let Value::Array(a) = &ids else { return Ok(ids) };
    for v in a.borrow_mut().data.iter_mut() {
        *v = Value::Object(Instance::new(&v.to_string_val(), type_name, fields));
    }
    Ok(ids)
}

pub fn shared_builtin(key: &str, args: &[Value]) -> Option<Result<Value, String>> {
    // Objects (rapidr_ast::objects): instances and their field slots.
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    match key {
        "__newobject" => {
            return Some(Ok(Value::Object(Instance::new(&arg(0).to_string_val(), &arg(1).to_string_val(), &arg(2).to_string_val()))));
        }
        "__getfield" | "__setfield" => {
            let Value::Object(o) = arg(0) else {
                return Some(Err(format!("this variable does not refer to an object ({})", describe(&arg(0)))));
            };
            let slot = arg(1).to_i64().max(0) as usize;
            return Some(Ok(if key == "__getfield" {
                o.get(slot)
            } else {
                o.set(slot, arg(2));
                Value::Null
            }));
        }
        "__null" => return Some(Ok(Value::Null)),
        // Stores into declared numeric types (`numeric`, rapidr_ast::numeric).
        _ if key.starts_with("__to_") => {
            if let Some(kind) = numeric::NumKind::from_builtin(key) {
                return Some(Ok(numeric::convert(&arg(0), kind)));
            }
        }
        // `__newarray(fill, lo, hi)`: an array field's initial value.
        "__newarray" => return Some(v_array(vec![(arg(1).to_i64(), arg(2).to_i64())], arg(0))),
        "__aget" | "__aset" => {
            let Value::Array(a) = arg(0) else {
                return Some(Err(format!("this is not an array ({})", describe(&arg(0)))));
            };
            let n = if key == "__aset" { args.len().saturating_sub(2) } else { args.len().saturating_sub(1) };
            let idx: Vec<i64> = args[1..1 + n].iter().map(Value::to_i64).collect();
            return Some(if key == "__aget" {
                a.borrow().get(&idx)
            } else {
                a.borrow_mut().set(&idx, arg(1 + n)).map(|_| Value::Null)
            });
        }
        "__objectarray" => {
            let bounds: Vec<(i64, i64)> = args.get(3..).unwrap_or(&[]).chunks(2).map(|b| (b[0].to_i64(), b.get(1).map_or(0, Value::to_i64))).collect();
            return Some(new_object_array(&arg(0).to_string_val(), &arg(1).to_string_val(), &arg(2).to_string_val(), &bounds));
        }
        _ => {}
    }
    if key == "__input_value" {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        return Some(Ok(input_value(&arg(0), &arg(1), &arg(2).to_string_val())));
    }
    if key == "__redim" {
        let (old, rest) = args.split_first()?;
        let (fill, bounds) = rest.split_first()?;
        let bounds: Vec<(i64, i64)> = bounds.chunks(2).map(|b| (b[0].to_i64(), b.get(1).map_or(0, |v| v.to_i64()))).collect();
        return Some(redim(old, &bounds, fill.clone()));
    }
    data::builtin(key, args)
}
