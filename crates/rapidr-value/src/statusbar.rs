//! QSTATUSBAR's panel methods, one implementation for every runtime: the
//! panels are the component's properties `panel(i).caption` /
//! `panel(i).width` (the keys `SB.Panel(i).Caption = …` writes) and
//! `panelcount`, which the UI kernel draws (components/statusbar.rs).

use crate::{v_int, v_str, Value};

/// A QSTATUSBAR method on its properties (`get` / `set`): `AddPanels
/// "Ready", "Line 1"` appends panels; `Clear` removes them all (RC.EXE:
/// panels added after it start again at Panel(0)). `None`: not one of
/// them.
pub fn call(method: &str, args: &[Value], get: &dyn Fn(&str) -> Value, set: &mut dyn FnMut(&str, Value)) -> Option<Value> {
    match method {
        "addpanels" => {
            let mut n = get("panelcount").to_i64().max(0);
            for a in args {
                set(&format!("panel({n}).caption"), v_str(&a.to_string_val()));
                n += 1;
            }
            set("panelcount", v_int(n));
        }
        "clear" => {
            for i in 0..get("panelcount").to_i64().clamp(0, 256) {
                set(&format!("panel({i}).caption"), v_str(""));
                set(&format!("panel({i}).width"), Value::Null);
            }
            set("panelcount", v_int(0));
        }
        _ => return None,
    }
    Some(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[test]
    fn add_clear_add() {
        let props = RefCell::new(HashMap::<String, Value>::new());
        let get = |k: &str| props.borrow().get(k).cloned().unwrap_or(Value::Null);
        let mut set = |k: &str, v: Value| {
            props.borrow_mut().insert(k.to_string(), v);
        };
        call("addpanels", &[v_str("one"), v_str("two")], &get, &mut set);
        set("panel(1).width", v_int(80));
        assert_eq!(get("panelcount").to_i64(), 2);
        call("clear", &[], &get, &mut set);
        assert_eq!(get("panelcount").to_i64(), 0);
        call("addpanels", &[v_str("three")], &get, &mut set);
        assert_eq!((get("panelcount").to_i64(), get("panel(0).caption").to_string_val()), (1, "three".to_string()));
        assert!(matches!(get("panel(1).width"), Value::Null));
    }
}
