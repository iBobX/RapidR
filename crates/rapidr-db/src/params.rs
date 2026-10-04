//! A query's parameters: `DB.AddParam v` queues them per component until
//! the next query takes them (`DB.ClearParams` drops them), and the values
//! after the SQL in `DB.Query(sql, p1, p2, …)` follow them, an array
//! argument giving its elements.

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::Value;

thread_local! {
    /// The parameters each component (by lowercase name) has queued.
    static QUEUED: RefCell<HashMap<String, Vec<Value>>> = RefCell::new(HashMap::new());
}

/// The values of `args` in order, an array's elements in its place (in
/// storage order: a two-dimensional array row after row).
pub fn flatten(args: &[Value]) -> Vec<Value> {
    fn push(v: &Value, out: &mut Vec<Value>) {
        match v {
            Value::Array(a) => a.borrow().data.iter().for_each(|e| push(e, out)),
            other => out.push(other.clone()),
        }
    }
    let mut out = Vec::new();
    args.iter().for_each(|v| push(v, &mut out));
    out
}

/// `DB.AddParam v [, w …]`.
pub(crate) fn add(key: &str, args: &[Value]) {
    let values = flatten(args);
    QUEUED.with(|q| q.borrow_mut().entry(key.to_string()).or_default().extend(values));
}

/// `DB.ClearParams`.
pub(crate) fn clear(key: &str) {
    QUEUED.with(|q| q.borrow_mut().remove(key));
}

/// The parameters of a query: those queued, then `extra` (the arguments
/// after the SQL). The queue is emptied, whatever the query does.
pub(crate) fn take(key: &str, extra: &[Value]) -> Vec<Value> {
    let mut values = QUEUED.with(|q| q.borrow_mut().remove(key)).unwrap_or_default();
    values.extend(flatten(extra));
    values
}

/// The message when a query's values don't match its placeholders: the
/// same text on every runtime (OnError gets it).
pub fn wrong_count(placeholders: usize, given: usize) -> String {
    let s = |n: usize| if n == 1 { "" } else { "s" };
    format!(
        "wrong number of parameters: the SQL has {placeholders} placeholder{} (?) and {given} value{} {} given",
        s(placeholders),
        s(given),
        if given == 1 { "was" } else { "were" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidr_value::{v_array, v_int, v_str};

    #[test]
    fn queued_parameters_come_first_and_are_used_once() {
        add("db", &[v_int(1)]);
        add("db", &[v_str("a"), v_str("b")]);
        assert_eq!(take("db", &[v_int(9)]), vec![v_int(1), v_str("a"), v_str("b"), v_int(9)]);
        assert_eq!(take("db", &[]), vec![]);
        add("db", &[v_int(1)]);
        clear("db");
        assert_eq!(take("db", &[]), vec![]);
    }

    #[test]
    fn an_array_gives_its_elements() {
        let a = v_array(vec![(0, 2)], v_int(0)).unwrap();
        a.rp_set(&[1], v_str("x"));
        assert_eq!(flatten(&[v_str("s"), a]), vec![v_str("s"), v_int(0), v_str("x"), v_int(0)]);
    }

    #[test]
    fn the_message_counts_in_words() {
        assert_eq!(wrong_count(1, 2), "wrong number of parameters: the SQL has 1 placeholder (?) and 2 values were given");
        assert_eq!(wrong_count(2, 1), "wrong number of parameters: the SQL has 2 placeholders (?) and 1 value was given");
    }
}
