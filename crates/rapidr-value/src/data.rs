//! DATA / READ / RESTORE (RapidQ manual): the parser gathers every DATA item
//! of the program into one table, registered when the program starts;
//! READ takes the next item and RESTORE moves the read position.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::Value;

#[derive(Default)]
struct DataTable {
    items: Vec<Value>,
    /// Label (lowercase) → index of the first item of the DATA after it.
    labels: HashMap<String, usize>,
    next: usize,
}

thread_local! {
    static TABLE: RefCell<DataTable> = RefCell::new(DataTable::default());
}

/// Appends items to the table (the parser emits these calls in source order).
pub fn add(items: &[Value]) {
    TABLE.with(|t| t.borrow_mut().items.extend_from_slice(items));
}

/// `RESTORE label` targets item `index`.
pub fn label(name: &Value, index: &Value) {
    TABLE.with(|t| {
        t.borrow_mut().labels.insert(name.to_string_val().to_ascii_lowercase(), index.to_i64().max(0) as usize);
    });
}

/// READ: the next item, or `None` when every item has been read.
pub fn read() -> Option<Value> {
    TABLE.with(|t| {
        let mut t = t.borrow_mut();
        let item = t.items.get(t.next).cloned()?;
        t.next += 1;
        Some(item)
    })
}

/// RESTORE (no label: back to the first item). Errors for an unknown label.
pub fn restore(label: Option<&Value>) -> Result<(), String> {
    TABLE.with(|t| {
        let mut t = t.borrow_mut();
        t.next = match label {
            None => 0,
            Some(l) => {
                let key = l.to_string_val().to_ascii_lowercase();
                *t.labels.get(&key).ok_or_else(|| format!("RESTORE {key}: no DATA after that label"))?
            }
        };
        Ok(())
    })
}

/// The DATA builtins the parser inserts, for interpreter hosts: `None` if
/// `key` isn't one of them.
pub fn builtin(key: &str, args: &[Value]) -> Option<Result<Value, String>> {
    let null = Value::Null;
    Some(match key {
        "__data_reset" => {
            reset();
            Ok(null)
        }
        "__data_add" => {
            add(args);
            Ok(null)
        }
        "__data_label" => {
            label(args.first().unwrap_or(&null), args.get(1).unwrap_or(&null));
            Ok(null)
        }
        "__read" => read().ok_or_else(|| OUT_OF_DATA.to_string()),
        "__restore" => restore(args.first()).map(|()| null),
        _ => return None,
    })
}

const OUT_OF_DATA: &str = "Out of DATA: READ found no more DATA items";

/// READ in compiled programs (stops the program when the DATA is used up).
pub fn read_compiled() -> Value {
    read().unwrap_or_else(|| crate::runtime_error(OUT_OF_DATA))
}

/// RESTORE in compiled programs.
pub fn restore_compiled(label: Option<&Value>) -> Value {
    restore(label).unwrap_or_else(|e| crate::runtime_error(&e));
    Value::Null
}

/// Empties the table (a new program run in the same process, e.g. the IDE).
pub fn reset() {
    TABLE.with(|t| *t.borrow_mut() = DataTable::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_read_and_restore() {
        reset();
        let items = vec![
            Value::String("my dog ate my homework".into()),
            Value::String("oh, boy!".into()),
            Value::Double(34.4),
            Value::Integer(7),
        ];
        add(&items);
        label(&Value::String("Second".into()), &Value::Integer(2));
        assert_eq!(read(), Some(Value::String("my dog ate my homework".into())));
        restore(Some(&Value::String("second".into()))).unwrap();
        assert_eq!(read(), Some(Value::Double(34.4)));
        assert_eq!(read(), Some(Value::Integer(7)));
        assert_eq!(read(), None);
        restore(None).unwrap();
        assert_eq!(read(), Some(Value::String("my dog ate my homework".into())));
        assert!(restore(Some(&Value::String("nope".into()))).is_err());
        reset();
    }
}
