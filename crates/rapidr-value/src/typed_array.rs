//! Native builds' typed arrays (rapidr-codegen-rust `typed_arrays`): an
//! array declared with a numeric type (`DIM v(31) AS DOUBLE`, `DIM mem(65535)
//! AS BYTE`) and only used by element, by LBOUND / UBOUND and passed to a
//! parameter of the same type is a plain vector of its numbers — `f64`, `u8`,
//! … — instead of a [`BasicArray`](crate::BasicArray) of `Value`s: no
//! boxing, no shared `Value` cloned for each element.
//!
//! It behaves exactly as a `BasicArray` does (the interpreter's arrays):
//! the same bounds and errors, RapidQ's unchecked subscripts (an element's
//! place is worked out from all its indices; only a place outside the whole
//! array is out of it, and what the program stores there is kept apart and
//! read back), REDIM keeping each element whose indices are still inside,
//! and an array not DIMmed yet as an empty variable. The tests below compare
//! the two.
//!
//! An array that is used any other way (passed whole to a builtin, used as
//! a value, its address taken) stays a `BasicArray`, so the interpreter's
//! and every builtin's view of it never meets a second copy.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::{runtime_error, Value, MAX_ARRAY_ELEMENTS};

/// Most elements an array keeps outside its bounds (as `BasicArray`'s).
const MAX_OVERFLOW: usize = 1 << 16;

/// The numbers a typed array holds: one Rust type per declared type (BYTE
/// `u8`, WORD `u16`, SHORT `i16`, INTEGER / LONG / DWORD `i32`, INT64
/// `i64`, UINT64 `u64`, SINGLE `f32`, DOUBLE `f64`). Stores are already converted
/// to the type (`rapidr_ast::numeric`), so each fits exactly.
pub trait Elem: Copy + Default + PartialEq + std::fmt::Debug + 'static {
    /// The element as the `Value` a `BasicArray` of its type holds.
    fn to_value(self) -> Value;
}

macro_rules! int_elem {
    ($($t:ty),*) => {$(
        impl Elem for $t {
            #[inline]
            fn to_value(self) -> Value {
                Value::Integer(self as i64)
            }
        }
    )*};
}
int_elem!(u8, u16, i16, i32);

// (RapidR's 64-bit kinds: an element reads as the INT64 / UINT64 it is)
impl Elem for i64 {
    #[inline]
    fn to_value(self) -> Value {
        Value::Int64(self)
    }
}

impl Elem for u64 {
    #[inline]
    fn to_value(self) -> Value {
        Value::UInt64(self)
    }
}

impl Elem for f32 {
    #[inline]
    fn to_value(self) -> Value {
        Value::Double(self as f64)
    }
}

impl Elem for f64 {
    #[inline]
    fn to_value(self) -> Value {
        Value::Double(self)
    }
}

/// A typed array (see the module docs). Every access copies a number in or
/// out; nothing hands out a reference to its storage, so a store from
/// anywhere (a SUB it was passed to, a REDIM) is seen by everyone.
#[derive(Debug, Default)]
pub struct TypedArray<T: Elem> {
    inner: RefCell<Inner<T>>,
}

#[derive(Debug, Default, Clone)]
struct Inner<T> {
    /// DIMmed (until then the variable is empty, as an undeclared one is).
    dimmed: bool,
    bounds: Vec<(i64, i64)>,
    /// Fast paths: the dimension count, the first two lower bounds and the
    /// second dimension's extent.
    rank: usize,
    lo0: i64,
    lo1: i64,
    ext1: i64,
    data: Vec<T>,
    /// Elements stored outside the array, by place (`BasicArray::overflow`).
    overflow: BTreeMap<i128, T>,
}

/// The element count of an array of `bounds`, with `BasicArray::new`'s
/// errors.
fn checked_len(bounds: &[(i64, i64)]) -> Result<usize, String> {
    if bounds.is_empty() {
        return Err("an array needs at least one dimension".into());
    }
    let mut len: usize = 1;
    for &(lo, hi) in bounds {
        if hi < lo {
            return Err(format!("array bounds {lo} TO {hi}: the upper bound is below the lower bound"));
        }
        let extent = usize::try_from(hi - lo + 1).map_err(|_| "array too large".to_string())?;
        len = len
            .checked_mul(extent)
            .filter(|&n| n <= MAX_ARRAY_ELEMENTS)
            .ok_or_else(|| format!("array too large (limit {MAX_ARRAY_ELEMENTS} elements)"))?;
    }
    Ok(len)
}

impl<T: Elem> Inner<T> {
    fn new(bounds: &[(i64, i64)]) -> Result<Self, String> {
        let len = checked_len(bounds)?;
        Ok(Self {
            dimmed: true,
            bounds: bounds.to_vec(),
            rank: bounds.len(),
            lo0: bounds[0].0,
            lo1: bounds.get(1).map_or(0, |b| b.0),
            ext1: bounds.get(1).map_or(0, |b| b.1 - b.0 + 1),
            data: vec![T::default(); len],
            overflow: BTreeMap::new(),
        })
    }

    /// The element's place as RapidQ computes it (no subscript checked),
    /// when as many indices as dimensions are given (`BasicArray::place`).
    fn place(&self, indices: &[i64]) -> Option<i128> {
        if indices.len() != self.bounds.len() {
            return None;
        }
        let mut flat: i128 = 0;
        for (&i, &(lo, hi)) in indices.iter().zip(&self.bounds) {
            flat = flat * (hi - lo + 1) as i128 + (i as i128 - lo as i128);
        }
        Some(flat)
    }

    /// `BasicArray::offset`'s message for the wrong number of indices.
    fn count_error(&self, given: usize) -> String {
        format!("array has {} dimension(s) but {} index(es) were given", self.bounds.len(), given)
    }

    /// `BasicArray::get`: inside, the element; outside, what was stored
    /// there, else zero.
    fn get(&self, indices: &[i64]) -> Result<T, String> {
        let p = self.place(indices).ok_or_else(|| self.count_error(indices.len()))?;
        Ok(match usize::try_from(p).ok().and_then(|i| self.data.get(i)) {
            Some(&x) => x,
            None => self.overflow.get(&p).copied().unwrap_or_default(),
        })
    }

    /// `BasicArray::set`: outside the array, kept apart (up to
    /// `MAX_OVERFLOW` places).
    fn set(&mut self, indices: &[i64], value: T) -> Result<(), String> {
        let p = self.place(indices).ok_or_else(|| self.count_error(indices.len()))?;
        match usize::try_from(p).ok().and_then(|i| self.data.get_mut(i)) {
            Some(x) => *x = value,
            None => {
                if self.overflow.len() < MAX_OVERFLOW || self.overflow.contains_key(&p) {
                    self.overflow.insert(p, value);
                }
            }
        }
        Ok(())
    }
}

const NOT_AN_ARRAY_READ: &str = "indexing a value that is not an array";
const NOT_AN_ARRAY_WRITE: &str = "assigning to an element of a variable that is not an array (DIM it with a size first)";

impl<T: Elem> TypedArray<T> {
    /// An array not DIMmed yet.
    pub fn new() -> Self {
        Self { inner: RefCell::new(Inner::default()) }
    }

    /// `a(i)`.
    #[inline]
    pub fn get1(&self, i: i64) -> T {
        {
            let a = self.inner.borrow();
            if a.rank == 1 {
                if let Some(k) = i.checked_sub(a.lo0) {
                    // (a negative k is far beyond the data)
                    if let Some(&x) = a.data.get(k as usize) {
                        return x;
                    }
                }
            }
        }
        self.get_slow(&[i])
    }

    /// `a(i) = v`.
    #[inline]
    pub fn set1(&self, i: i64, v: T) {
        {
            let mut a = self.inner.borrow_mut();
            if a.rank == 1 {
                if let Some(k) = i.checked_sub(a.lo0) {
                    if let Some(x) = a.data.get_mut(k as usize) {
                        *x = v;
                        return;
                    }
                }
            }
        }
        self.set_slow(&[i], v)
    }

    /// The place of `a(i, j)` in a two-dimensional array's data.
    #[inline]
    fn place2(a: &Inner<T>, i: i64, j: i64) -> Option<usize> {
        if a.rank != 2 {
            return None;
        }
        let k = i.checked_sub(a.lo0)?.checked_mul(a.ext1)?.checked_add(j.checked_sub(a.lo1)?)?;
        usize::try_from(k).ok().filter(|&k| k < a.data.len())
    }

    /// `a(i, j)`.
    #[inline]
    pub fn get2(&self, i: i64, j: i64) -> T {
        {
            let a = self.inner.borrow();
            if let Some(k) = Self::place2(&a, i, j) {
                return a.data[k];
            }
        }
        self.get_slow(&[i, j])
    }

    /// `a(i, j) = v`.
    #[inline]
    pub fn set2(&self, i: i64, j: i64, v: T) {
        {
            let mut a = self.inner.borrow_mut();
            if let Some(k) = Self::place2(&a, i, j) {
                a.data[k] = v;
                return;
            }
        }
        self.set_slow(&[i, j], v)
    }

    /// `a(i, j, …)` with any number of indices.
    pub fn get(&self, indices: &[i64]) -> T {
        match indices {
            [i] => self.get1(*i),
            [i, j] => self.get2(*i, *j),
            _ => self.get_slow(indices),
        }
    }

    /// `a(i, j, …) = v` with any number of indices.
    pub fn set(&self, indices: &[i64], v: T) {
        match indices {
            [i] => self.set1(*i, v),
            [i, j] => self.set2(*i, *j, v),
            _ => self.set_slow(indices, v),
        }
    }

    #[cold]
    fn get_slow(&self, indices: &[i64]) -> T {
        let result = {
            let a = self.inner.borrow();
            if !a.dimmed {
                // (an empty variable: one index reads it as empty, as
                // `Value::rp_get` does — a number reads 0)
                if indices.len() == 1 {
                    return T::default();
                }
                Err(NOT_AN_ARRAY_READ.to_string())
            } else {
                a.get(indices)
            }
        };
        result.unwrap_or_else(|e| runtime_error(&e))
    }

    #[cold]
    fn set_slow(&self, indices: &[i64], v: T) {
        let result = {
            let mut a = self.inner.borrow_mut();
            if !a.dimmed {
                Err(NOT_AN_ARRAY_WRITE.to_string())
            } else {
                a.set(indices, v)
            }
        };
        result.unwrap_or_else(|e| runtime_error(&e))
    }

    /// `a(i, …)` as the `Value` a `BasicArray` gives (an array not DIMmed
    /// yet reads empty with one index).
    pub fn value(&self, indices: &[i64]) -> Value {
        if indices.len() == 1 && !self.inner.borrow().dimmed {
            return Value::Null;
        }
        self.get(indices).to_value()
    }

    /// `a(i)` as a `Value` ([`Self::value`]).
    #[inline]
    pub fn value1(&self, i: i64) -> Value {
        {
            let a = self.inner.borrow();
            if a.rank == 1 {
                if let Some(k) = i.checked_sub(a.lo0) {
                    if let Some(&x) = a.data.get(k as usize) {
                        return x.to_value();
                    }
                }
            }
        }
        self.value(&[i])
    }

    /// `DIM a(bounds)`: a new array of zeros (in place: there is one per
    /// variable).
    pub fn dim(&self, bounds: &[(i64, i64)]) {
        match Inner::new(bounds) {
            Ok(fresh) => *self.inner.borrow_mut() = fresh,
            Err(e) => runtime_error(&e),
        }
    }

    /// `REDIM a(bounds)` (`crate::redim`): resized in place, each element
    /// whose indices are still inside kept (with as many dimensions as
    /// before; one now outside is kept apart, as a store there would be);
    /// a DIM when there's no array yet.
    pub fn redim(&self, bounds: &[(i64, i64)]) {
        let mut resized = match Inner::new(bounds) {
            Ok(fresh) => fresh,
            Err(e) => runtime_error(&e),
        };
        let mut a = self.inner.borrow_mut();
        if a.dimmed && a.bounds.len() == resized.bounds.len() {
            // Walk the old elements in storage order with their indices.
            let mut index: Vec<i64> = a.bounds.iter().map(|b| b.0).collect();
            for &value in &a.data {
                let _ = resized.set(&index, value);
                for d in (0..index.len()).rev() {
                    index[d] += 1;
                    if index[d] <= a.bounds[d].1 || d == 0 {
                        break;
                    }
                    index[d] = a.bounds[d].0;
                }
            }
        }
        *a = resized;
    }

    /// LBOUND(a [, dim]) / UBOUND(a [, dim]) (`Value::rp_bound`).
    pub fn bound(&self, dim: &Value, upper: bool) -> Value {
        let d = if matches!(dim, Value::Null) { 1 } else { dim.to_i64() };
        Value::Integer(self.bound_of(d, upper))
    }

    /// LBOUND / UBOUND of dimension `d` (1-based).
    pub fn bound_of(&self, d: i64, upper: bool) -> i64 {
        let found = {
            let a = self.inner.borrow();
            let b = usize::try_from(d).ok().and_then(|d| d.checked_sub(1)).and_then(|i| a.bounds.get(i)).filter(|_| a.dimmed);
            b.map(|&(lo, hi)| if upper { hi } else { lo })
        };
        found.unwrap_or_else(|| {
            runtime_error(&format!("{}: argument is not an array, or it has no dimension {d}", if upper { "UBOUND" } else { "LBOUND" }))
        })
    }

    /// A copy as the interpreter's array (`Value::Array`), or `Value::Null`
    /// before its DIM: for tests and messages, never shared.
    pub fn to_value(&self) -> Value {
        let a = self.inner.borrow();
        if !a.dimmed {
            return Value::Null;
        }
        let mut copy = crate::BasicArray::new(a.bounds.clone(), Value::Null).unwrap_or_else(|e| runtime_error(&e));
        copy.data = a.data.iter().map(|x| x.to_value()).collect();
        copy.overflow = a.overflow.iter().map(|(&p, &x)| (p, x.to_value())).collect();
        Value::Array(std::rc::Rc::new(RefCell::new(copy)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BasicArray;

    fn same(t: &TypedArray<f64>, b: &BasicArray, indices: &[i64]) {
        assert_eq!(t.get(indices).to_value().to_f64(), b.get(indices).unwrap().to_f64(), "at {indices:?}");
    }

    #[test]
    fn one_dimension_reads_and_writes_as_a_basic_array() {
        let t = TypedArray::<f64>::new();
        t.dim(&[(1, 5)]);
        let mut b = BasicArray::new(vec![(1, 5)], Value::Double(0.0)).unwrap();
        for i in -3..12 {
            t.set1(i, i as f64 * 1.5);
            b.set(&[i], Value::Double(i as f64 * 1.5)).unwrap();
        }
        for i in -5..14 {
            same(&t, &b, &[i]);
            assert_eq!(t.value1(i).to_f64(), b.get(&[i]).unwrap().to_f64());
        }
        assert_eq!(t.get1(3), 4.5);
        assert!(matches!(t.value1(3), Value::Double(x) if x == 4.5));
    }

    #[test]
    fn unchecked_subscripts_reach_the_next_row() {
        // RapidQ: `Grid(i, NumX + 1)` is the next row's first element.
        let t = TypedArray::<i32>::new();
        t.dim(&[(0, 2), (0, 3)]);
        let mut b = BasicArray::new(vec![(0, 2), (0, 3)], Value::Integer(0)).unwrap();
        t.set2(0, 4, 7);
        b.set(&[0, 4], Value::Integer(7)).unwrap();
        assert_eq!(t.get2(1, 0), 7);
        assert_eq!(b.get(&[1, 0]).unwrap().to_i64(), 7);
        for i in -1..4 {
            for j in -2..6 {
                assert_eq!(t.get2(i, j) as i64, b.get(&[i, j]).unwrap().to_i64(), "({i}, {j})");
            }
        }
        // Three dimensions, through the general path.
        let t3 = TypedArray::<u8>::new();
        t3.dim(&[(1, 2), (1, 3), (0, 1)]);
        t3.set(&[2, 3, 1], 9);
        assert_eq!(t3.get(&[2, 3, 1]), 9);
        assert_eq!(t3.get(&[1, 1, 0]), 0);
    }

    #[test]
    fn redim_keeps_what_a_basic_array_keeps() {
        for (from, to) in [(vec![(0, 9)], vec![(0, 4)]), (vec![(0, 4)], vec![(-2, 12)]), (vec![(1, 3), (1, 4)], vec![(1, 2), (1, 6)]), (vec![(0, 3)], vec![(0, 1), (0, 1)])] {
            let t = TypedArray::<f64>::new();
            t.dim(&from);
            let old = crate::rp_new_array(&from, Value::Double(0.0));
            let Value::Array(b) = &old else { unreachable!() };
            let n = b.borrow().data.len();
            for k in 0..n {
                t.inner.borrow_mut().data[k] = k as f64 + 1.0;
                b.borrow_mut().data[k] = Value::Double(k as f64 + 1.0);
            }
            t.redim(&to);
            let new = crate::rp_redim(&old, &to, Value::Double(0.0));
            let Value::Array(b) = &new else { unreachable!() };
            let b = b.borrow();
            assert_eq!(t.inner.borrow().bounds, b.bounds);
            for (k, v) in b.data.iter().enumerate() {
                assert_eq!(t.inner.borrow().data[k], v.to_f64(), "{from:?} -> {to:?} at {k}");
            }
            assert_eq!(t.inner.borrow().overflow.len(), b.overflow.len());
            for (p, v) in &b.overflow {
                assert_eq!(t.inner.borrow().overflow[p], v.to_f64());
            }
        }
        // REDIM without a DIM is a DIM.
        let t = TypedArray::<i16>::new();
        t.redim(&[(0, 3)]);
        assert_eq!(t.bound_of(1, true), 3);
    }

    #[test]
    fn bounds_and_errors_are_the_basic_arrays() {
        let t = TypedArray::<f64>::new();
        t.dim(&[(-3, 7), (2, 4)]);
        assert_eq!(t.bound(&Value::Null, false).to_i64(), -3);
        assert_eq!(t.bound(&Value::Integer(1), true).to_i64(), 7);
        assert_eq!(t.bound(&Value::Integer(2), false).to_i64(), 2);
        assert_eq!(t.bound(&Value::Integer(2), true).to_i64(), 4);
        for bounds in [vec![], vec![(5, 2)], vec![(0, 100_000), (0, 100_000)]] {
            let ours = checked_len(&bounds).unwrap_err();
            let theirs = BasicArray::new(bounds.clone(), Value::Null).unwrap_err();
            assert_eq!(ours, theirs);
        }
        let b = BasicArray::new(vec![(0, 3), (0, 3)], Value::Null).unwrap();
        let inner: Inner<f64> = Inner::new(&[(0, 3), (0, 3)]).unwrap();
        assert_eq!(inner.get(&[1]).unwrap_err(), b.get(&[1]).unwrap_err());
    }

    #[test]
    fn an_array_not_dimmed_yet_is_an_empty_variable() {
        let t = TypedArray::<f64>::new();
        assert!(matches!(t.value1(4), Value::Null));
        assert!(matches!(Value::Null.rp_get(&[4]), Value::Null));
        assert_eq!(t.get1(4), 0.0);
        assert!(matches!(t.to_value(), Value::Null));
    }

    #[test]
    fn the_overflow_is_capped_as_a_basic_arrays() {
        let t = TypedArray::<i64>::new();
        t.dim(&[(0, 0)]);
        let mut b = BasicArray::new(vec![(0, 0)], Value::Integer(0)).unwrap();
        for i in 1..(MAX_OVERFLOW as i64 + 10) {
            t.set1(i, i);
            b.set(&[i], Value::Integer(i)).unwrap();
        }
        for i in [1, 500, MAX_OVERFLOW as i64, MAX_OVERFLOW as i64 + 5] {
            assert_eq!(t.get1(i), b.get(&[i]).unwrap().to_i64(), "{i}");
        }
    }

    #[test]
    fn each_type_reads_as_its_value() {
        let t = TypedArray::<f32>::new();
        t.dim(&[(0, 1)]);
        t.set1(0, 0.1_f64 as f32);
        assert_eq!(t.value1(0).to_f64(), crate::numeric::single(0.1));
        let b = TypedArray::<u8>::new();
        b.dim(&[(0, 1)]);
        b.set1(1, 255);
        assert!(matches!(b.value1(1), Value::Integer(255)));
        let copy = b.to_value();
        assert_eq!(copy.rp_get(&[1]).to_i64(), 255);
    }
}
