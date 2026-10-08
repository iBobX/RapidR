//! The `Value` type — a dynamically-typed BASIC value.

use std::cell::RefCell;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::rc::Rc;

pub mod lprint;
pub mod strings;
pub mod numeric;
pub mod variadic;
pub mod data;
pub mod console;
pub mod dialogs;
pub mod color_dialog;
pub mod font_dialog;
pub mod statusbar;
pub mod window_state;
pub mod basic_files;
pub mod builtins;
pub mod memory;
pub mod handles;
pub mod tray;
pub mod mdi;
pub mod dock;
// (I1 / L-PANELS: RapidR Studio's panels as public components)
pub mod panels;
pub mod events;
pub mod input;
pub mod globals;
pub mod file_dialog;
pub mod format;
pub mod toggle_group;
pub mod objects;
pub mod layout;
pub mod autosize;
// (I4) RapidR Studio's visual designer model, GUI-free.
pub mod designer;
pub mod members;
pub mod scrollbars;
pub mod theme;
pub mod ide_theme;
pub mod registry;
pub mod resources;
pub mod environ;
pub mod command_line;
// RNUM, RDATAFRAME, RPLOT: one implementation for every runtime.
pub mod datascience;
// (Stage W3) What both runtimes' component registries give a new
// component, and what the UI kernel reads as unset.
pub mod component_defaults;

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
    /// separated, as the compilers pass them: `Name:STRING*5,Age:INTEGER`,
    /// each with its type for `memory`), all Null.
    pub fn new(id: &str, type_name: &str, names: &str) -> Rc<Self> {
        let names = FIELD_NAMES.with(|c| {
            c.borrow_mut()
                .entry(format!("{type_name}:{names}"))
                .or_insert_with(|| {
                    memory::register_layout(type_name, names);
                    names.split(',').filter(|n| !n.is_empty()).map(|n| n.split(':').next().unwrap_or(n).to_string()).collect::<Vec<_>>().into()
                })
                .clone()
        });
        let fields = RefCell::new(vec![Value::Null; names.len()]);
        let instance = Rc::new(Self { id: id.to_string(), type_name: type_name.to_string(), names, fields });
        note_made(type_name, Made::Instance(Rc::downgrade(&instance)));
        instance
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

/// The newest object of a type, for [`rp_last_of_type`].
pub enum Made {
    /// A component (by id).
    Component(String),
    /// An instance of a TYPE (one EXTENDS a component *is* that component).
    Instance(std::rc::Weak<Instance>),
}

thread_local! {
    /// Type (uppercase) → the object of that type created last.
    static LAST_MADE: RefCell<Vec<(String, Made)>> = const { RefCell::new(Vec::new()) };
}

/// Records `made` as the newest object of `type_name`: the runtimes call it
/// for every component they create (`objects::create`), and every TYPE
/// instance does.
pub fn note_made(type_name: &str, made: Made) {
    LAST_MADE.with(|l| {
        let mut l = l.borrow_mut();
        match l.iter_mut().find(|(t, _)| t.eq_ignore_ascii_case(type_name)) {
            Some(entry) => entry.1 = made,
            None => l.push((type_name.to_ascii_uppercase(), made)),
        }
    });
}

/// `__lastoftype(type)` (rapidr_ast::type_values): a component type's name
/// used as a value is, as in RapidQ, the object of that type created last
/// (`Parent = QFORM` → the newest form) — a component's id (lowercase, the
/// same whichever backend created it), a TYPE's instance — and "" while
/// there's none.
pub fn rp_last_of_type(type_name: &Value) -> Value {
    let t = type_name.to_string_val();
    LAST_MADE.with(|l| {
        l.borrow().iter().find(|(k, _)| k.eq_ignore_ascii_case(&t)).and_then(|(_, made)| match made {
            Made::Component(id) => Some(v_str(&id.to_lowercase())),
            Made::Instance(w) => w.upgrade().map(Value::Object),
        })
    })
    .unwrap_or_else(|| v_str(""))
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

    /// The element's place in the data, as RapidQ lays arrays out ("stored
    /// serially by the last DIM subscript"): RapidQ doesn't check each
    /// subscript ("There is no checking for limits on arrays"), so
    /// `Grid(i, NumX + 1)` is the next row's first element. Only a place
    /// outside the array is an error here.
    pub(crate) fn offset(&self, indices: &[i64]) -> Result<usize, String> {
        if indices.len() != self.bounds.len() {
            return Err(format!(
                "array has {} dimension(s) but {} index(es) were given",
                self.bounds.len(),
                indices.len()
            ));
        }
        let mut flat: i128 = 0;
        for (&i, &(lo, hi)) in indices.iter().zip(&self.bounds) {
            flat = flat * (hi - lo + 1) as i128 + (i as i128 - lo as i128);
        }
        if flat < 0 || flat >= self.data.len() as i128 {
            let (i, (lo, hi)) = indices.iter().zip(&self.bounds).find(|(&i, &(lo, hi))| i < lo || i > hi).map(|(&i, &b)| (i, b)).unwrap_or((indices[0], self.bounds[0]));
            return Err(format!("Subscript out of range: index {i} is outside {lo} TO {hi}"));
        }
        Ok(flat as usize)
    }

    /// An element; past the array's end — where RapidQ reads whatever memory
    /// follows — the element type's zero ("" for strings), so the program
    /// goes on as it did in RapidQ.
    pub fn get(&self, indices: &[i64]) -> Result<Value, String> {
        if indices.len() != self.bounds.len() {
            return self.offset(indices).map(|_| Value::Null);
        }
        Ok(match self.offset(indices) {
            Ok(i) => self.data[i].clone(),
            Err(_) => match self.data.first() {
                Some(Value::String(_)) => Value::String(String::new()),
                Some(Value::Double(_)) => Value::Double(0.0),
                Some(Value::Integer(_)) => Value::Integer(0),
                _ => Value::Null,
            },
        })
    }

    /// Stores an element; past the array's end (RapidQ would overwrite the
    /// memory after it) nothing is stored.
    pub fn set(&mut self, indices: &[i64], value: Value) -> Result<(), String> {
        if indices.len() != self.bounds.len() {
            return self.offset(indices).map(|_| ());
        }
        if let Ok(i) = self.offset(indices) {
            self.data[i] = value;
        }
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

/// A store into `STRING * n`: always `n` characters, as RapidQ has them
/// (RC.EXE: `DIM s AS STRING * 8 : s = "hi"` holds "hi" and six spaces,
/// LEN 8) — longer text cut, shorter padded with spaces; `STRING * 0` holds
/// nothing.
pub fn rp_fixed_string(val: &Value, n: usize) -> Value {
    let s = val.to_string_val();
    match s.char_indices().nth(n) {
        Some((cut, _)) => Value::String(s[..cut].to_string().into()),
        None => {
            let len = s.chars().count();
            let mut s = s;
            s.extend(std::iter::repeat_n(' ', n - len));
            Value::String(s.into())
        }
    }
}

/// A component property or method result as a program reads it: RapidQ's
/// are Delphi Booleans, true reads 1 (RC.EXE: `Check.Checked = -1 : PRINT
/// Check.Checked` shows 1, `Form.Enabled` 1; RAPIDQ.INC's `True` is 1, so
/// programs test `.Checked = True`). A comparison's own result stays -1.
pub fn property_read(v: Value) -> Value {
    match v {
        Value::Boolean(b) => Value::Integer(b as i64),
        v => v,
    }
}

/// CBOOL: numbers and numeric strings are true when non-zero; any other
/// string is true when it isn't empty.
pub fn cbool(val: &Value) -> Value {
    Value::Boolean(match val {
        Value::String(s) => match s.trim().parse::<f64>() {
            Ok(n) => n != 0.0,
            Err(_) => !s.is_empty(),
        },
        v => v.to_bool(),
    })
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
            // As RapidQ shows numbers (Delphi's FloatToStr, `format`): 15
            // significant digits, so PRINT 2 ^ 10 → 1024 and 0.1 + 0.2 → 0.3.
            Value::Double(n) => crate::format::float_to_str(*n),
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

    /// BASIC integer division (a \ b) as RapidQ does it (RC.EXE): each
    /// operand rounded as CINT rounds (`INT(x + 0.5)`), 32 bits, the
    /// quotient truncated; by zero, RapidQ's "Division by zero" error.
    pub fn checked_int_div(&self, rhs: &Value) -> Result<Value, &'static str> {
        let a = self.idiv_operand();
        let b = rhs.idiv_operand();
        numeric::checked_idiv(a, b).map(Value::Integer).ok_or(numeric::DIVISION_BY_ZERO)
    }

    /// `a \ b` in compiled programs (by zero, a run-time error).
    pub fn int_div(&self, rhs: &Value) -> Value {
        self.checked_int_div(rhs).unwrap_or_else(|e| runtime_error(e))
    }

    /// `a MOD b` as RapidQ does it: both rounded half to even to 32-bit
    /// integers (`7.5 MOD 2` is 0), the remainder has the dividend's sign;
    /// by zero, "Division by zero".
    pub fn checked_mod(&self, rhs: &Value) -> Result<Value, &'static str> {
        numeric::checked_imod(self.bits(), rhs.bits()).map(Value::Integer).ok_or(numeric::DIVISION_BY_ZERO)
    }

    fn idiv_operand(&self) -> i64 {
        match self {
            Value::Integer(n) => format::int32_of(*n),
            Value::Boolean(b) => -(*b as i64),
            other => numeric::idiv_operand(other.to_f64()),
        }
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

    /// The integer the bitwise operators and MOD work on, as RapidQ makes
    /// it (the x87's FISTP): true = -1, reals rounded half to even (`2.5 OR
    /// 0` is 2), 32 bits (beyond: -2147483648).
    fn bits(&self) -> i64 {
        match self {
            Value::Boolean(b) => -(*b as i64),
            Value::Integer(n) => format::int32_of(*n),
            other => format::int32(other.to_f64()),
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

    // (A NaN compares as the x87's FCOM leaves it in RapidQ: "less" and
    // "equal" at once — `=`, `<` and `<=` are true, `<>`, `>` and `>=`
    // false; RC.EXE prints that for `d = 0 / 0`.)

    pub fn rp_eq(&self, rhs: &Value) -> Value {
        Value::Boolean(self.nan_compare(rhs) || self.cmp_eq(rhs))
    }

    pub fn rp_ne(&self, rhs: &Value) -> Value {
        Value::Boolean(!self.nan_compare(rhs) && !self.cmp_eq(rhs))
    }

    pub fn rp_lt(&self, rhs: &Value) -> Value {
        Value::Boolean(self.nan_compare(rhs) || self.cmp_ord(rhs) == std::cmp::Ordering::Less)
    }

    pub fn rp_le(&self, rhs: &Value) -> Value {
        Value::Boolean(self.nan_compare(rhs) || matches!(self.cmp_ord(rhs), std::cmp::Ordering::Less | std::cmp::Ordering::Equal))
    }

    pub fn rp_gt(&self, rhs: &Value) -> Value {
        Value::Boolean(!self.nan_compare(rhs) && self.cmp_ord(rhs) == std::cmp::Ordering::Greater)
    }

    pub fn rp_ge(&self, rhs: &Value) -> Value {
        Value::Boolean(!self.nan_compare(rhs) && matches!(self.cmp_ord(rhs), std::cmp::Ordering::Greater | std::cmp::Ordering::Equal))
    }

    /// A numeric comparison with a NaN on either side.
    fn nan_compare(&self, rhs: &Value) -> bool {
        let num = |v: &Value| matches!(v, Value::Double(d) if d.is_nan());
        (num(self) || num(rhs)) && !matches!(self, Value::String(_) | Value::Object(_)) && !matches!(rhs, Value::String(_) | Value::Object(_))
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
    /// Floating point, by zero an infinity or NaN (RapidQ's `PRINT 7 / 0`
    /// shows -2147483648, as a whole number beyond 32 bits).
    fn div(self, rhs: Self) -> Value {
        Value::Double(self.to_f64() / rhs.to_f64())
    }
}

impl Rem for &Value {
    type Output = Value;
    /// `a MOD b` in compiled programs ([`Value::checked_mod`]; by zero a
    /// run-time error).
    fn rem(self, rhs: Self) -> Value {
        self.checked_mod(rhs).unwrap_or_else(|e| runtime_error(e))
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
/// (RC.EXE: the operands as the bitwise operators take them — `2.5 SHL 1`
/// is 4 — and the count as the x86 takes it, its low 5 bits: `1 SHL 32` is
/// 1, `1 SHL 31` -2147483648.)
pub fn rp_shl(a: &Value, n: &Value) -> Value {
    let shift = (n.bits() & 31) as u32;
    let bits = a.bits() as u32;
    Value::Integer((bits << shift) as i32 as i64)
}

/// `a SHR n` (RapidQ): logical shift right of the 32-bit value, as a LONG
/// (`-1 SHR 28` is 15; the count's low 5 bits).
pub fn rp_shr(a: &Value, n: &Value) -> Value {
    let shift = (n.bits() & 31) as u32;
    let bits = a.bits() as u32;
    Value::Integer((bits >> shift) as i32 as i64)
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
/// -1 when there is none (RC.EXE: `2 INV 4` is -1).
pub fn rp_inv(a: &Value, m: &Value) -> Value {
    // In 128 bits so no step overflows (`INV(MIN, -1)` must not crash).
    let (a, m) = (a.to_i64() as i128, m.to_i64() as i128);
    // (RC.EXE: `7 INV 1` and `5 INV 0` are -1 too — an inverse from 1 to
    // m - 1 or none)
    if m.abs() <= 1 {
        return Value::Integer(-1);
    }
    let (mut r0, mut r1) = (a.rem_euclid(m.abs()), m.abs());
    let (mut t0, mut t1) = (1i128, 0i128);
    while r1 != 0 {
        let q = r0 / r1;
        (r0, r1) = (r1, r0 - q * r1);
        (t0, t1) = (t1, t0 - q * t1);
    }
    Value::Integer(if r0 == 1 { t0.rem_euclid(m.abs()) as i64 } else { -1 })
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
mod fixed_string_tests {
    use super::*;

    #[test]
    fn fixed_strings_cut_by_characters_and_cbool() {
        assert_eq!(rp_fixed_string(&v_str("hello world"), 8).to_string_val(), "hello wo");
        assert_eq!(rp_fixed_string(&v_str("hi"), 8).to_string_val(), "hi      ", "padded, as RapidQ has it");
        assert_eq!(rp_fixed_string(&v_str("abc"), 0).to_string_val(), "");
        assert_eq!(rp_fixed_string(&v_str("héllo"), 2).to_string_val(), "hé");
        assert_eq!(rp_fixed_string(&v_int(12345), 3).to_string_val(), "123");
        assert!(cbool(&v_int(5)).to_bool() && !cbool(&v_int(0)).to_bool());
        assert!(!cbool(&v_str("0")).to_bool() && !cbool(&v_str("")).to_bool() && cbool(&v_str("x")).to_bool());
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
        assert_eq!(rp_inv(&Value::Integer(2), &Value::Integer(4)), Value::Integer(-1));
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
    // VARPTR, MEMCPY, SIZEOF, … (memory.rs).
    if let Some(r) = memory::shared(key, args) {
        return Some(r);
    }
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
        "__lastoftype" => return Some(Ok(rp_last_of_type(&arg(0)))),
        "__environ_set" => return Some(Ok(builtins::rp_environ_set(&arg(0)))),
        // The system tray (rapidr_ast::tray_calls): Shell_NotifyIcon.
        "__shell_notifyicon" => return Some(Ok(tray::shell_notify_icon_builtin(&arg(0), &arg(1)))),
        "__to_fixed" => return Some(Ok(rp_fixed_string(&arg(0), arg(1).to_i64().max(0) as usize))),
        "__arg_round" => return Some(Ok(numeric::arg_round(&arg(0)))),
        // Stores into declared numeric types (`numeric`, rapidr_ast::numeric).
        _ if key.starts_with("__to_") => {
            if let Some(kind) = numeric::NumKind::from_builtin(key) {
                return Some(Ok(numeric::convert(&arg(0), kind)));
            }
        }
        // `__newarray(fill, lo, hi [, lo, hi …])`: an array field's initial value.
        "__newarray" => {
            let dims: Vec<(i64, i64)> = args.get(1..).unwrap_or(&[]).chunks(2).map(|b| (b[0].to_i64(), b.get(1).map_or(0, Value::to_i64))).collect();
            return Some(v_array(dims, arg(0)));
        }
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
    if key == "__lprint" {
        return Some(Ok(lprint::lprint(args)));
    }
    if key == "lflush" {
        return Some(lprint::flush().map(|_| Value::Null));
    }
    if key == "__quicksort" {
        let idx: Vec<i64> = args.get(2..).unwrap_or(&[]).iter().map(Value::to_i64).collect();
        return Some(Ok(builtins::rp_quicksort(args.first().unwrap_or(&Value::Null), args.get(1).unwrap_or(&Value::Null), &idx)));
    }
    if key == "__inkey_trapall" {
        return Some(Ok(builtins::rp_inkey_trap_all(args.first().unwrap_or(&Value::Null))));
    }
    if key == "__decimal" {
        return Some(Ok(builtins::rp_set_decimal(args.first().unwrap_or(&Value::Null))));
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
