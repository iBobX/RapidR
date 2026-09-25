//! Arguments of RapidQ's SUBI / FUNCTIONI (manual chapter 9): a call passes
//! any number of values, packed into one array; the body reads the strings
//! and the numbers separately, 1-based, through ParamStr$(i) / ParamVal(i)
//! and ParamStrCount / ParamValCount.

use std::cell::RefCell;
use std::rc::Rc;

use crate::{BasicArray, Value};

/// Packs a call's arguments (the hidden parameter of a SUBI/FUNCTIONI).
pub fn pack(args: &[Value]) -> Value {
    let n = args.len() as i64;
    Value::Array(Rc::new(RefCell::new(BasicArray {
        bounds: vec![(0, n - 1)],
        data: args.to_vec(),
    })))
}

fn strings_and_numbers(pack: &Value) -> (Vec<Value>, Vec<Value>) {
    let Value::Array(a) = pack else { return (Vec::new(), Vec::new()) };
    a.borrow().data.iter().cloned().partition(|v| matches!(v, Value::String(_)))
}

/// ParamStr$(i): the i-th string argument (1-based), "" past the end.
pub fn param_str(pack: &Value, i: &Value) -> Value {
    let (strings, _) = strings_and_numbers(pack);
    let i = i.to_i64();
    (i >= 1).then(|| strings.get(i as usize - 1).cloned()).flatten().unwrap_or_else(|| Value::String(String::new()))
}

/// ParamVal(i): the i-th numeric argument (1-based), 0 past the end.
pub fn param_val(pack: &Value, i: &Value) -> Value {
    let (_, numbers) = strings_and_numbers(pack);
    let i = i.to_i64();
    (i >= 1).then(|| numbers.get(i as usize - 1).cloned()).flatten().unwrap_or(Value::Integer(0))
}

/// ParamStrCount: how many string arguments were passed.
pub fn param_str_count(pack: &Value) -> Value {
    Value::Integer(strings_and_numbers(pack).0.len() as i64)
}

/// ParamValCount: how many numeric arguments were passed.
pub fn param_val_count(pack: &Value) -> Value {
    Value::Integer(strings_and_numbers(pack).1.len() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_strings_and_numbers_one_based() {
        let p = pack(&[Value::String("Hello".into()), Value::Integer(1234), Value::String("Hmmm".into()), Value::Double(2.5)]);
        assert_eq!(param_str_count(&p), Value::Integer(2));
        assert_eq!(param_val_count(&p), Value::Integer(2));
        assert_eq!(param_str(&p, &Value::Integer(2)), Value::String("Hmmm".into()));
        assert_eq!(param_val(&p, &Value::Integer(2)), Value::Double(2.5));
        assert_eq!(param_str(&p, &Value::Integer(3)), Value::String(String::new()));
        assert_eq!(param_val(&p, &Value::Integer(0)), Value::Integer(0));
        assert_eq!(param_val_count(&pack(&[])), Value::Integer(0));
    }
}
