//! RJSON's document, one implementation for every runtime (the desktop's
//! and the web's kept their own copies before; the web's went through the
//! browser's JSON, whose numbers are doubles): a `serde_json` value, its
//! whole numbers exact to 64 bits both ways (docs/big-numbers-design.md).
//!
//! The runtimes keep a document per component and do the file and
//! property work (LoadFile / SaveFile / the `Text` property); the rest is
//! here: Parse, Stringify, Prettify, Get, Set, Has, Remove, Count, Keys,
//! Clear.

use crate::{v_int, v_str, wide, Value};
use serde_json::Value as Json;

/// A value set into a document: a 64-bit number as the number it is, a
/// whole number exactly; text that reads as a number becomes one, "true" /
/// "false" a boolean (as RJSON always set them), anything else text.
pub fn to_json(val: &Value) -> Json {
    match val {
        Value::Int64(n) => return Json::Number((*n).into()),
        Value::UInt64(n) => return Json::Number((*n).into()),
        Value::Integer(n) => return Json::Number((*n).into()),
        _ => {}
    }
    let s = val.to_string_val();
    if let Ok(i) = s.parse::<i64>() {
        Json::Number(i.into())
    } else if let Ok(u) = s.parse::<u64>() {
        Json::Number(u.into())
    } else if let Ok(f) = s.parse::<f64>() {
        serde_json::Number::from_f64(f).map(Json::Number).unwrap_or(Json::String(s))
    } else if s == "true" || s == "1" {
        Json::Bool(true)
    } else if s == "false" || s == "0" {
        Json::Bool(false)
    } else {
        Json::String(s)
    }
}

/// A document's value as a program reads it: a whole number exactly (an
/// INT64 beyond 32 bits, a UINT64 beyond 63: never cut or rounded), a
/// fraction a DOUBLE, true 1, null "", text as it is, an object or array as
/// its JSON.
pub fn from_json(v: &Json) -> Value {
    match v {
        Json::Null => v_str(""),
        Json::Bool(b) => v_int(i64::from(*b)),
        Json::Number(n) => {
            if let Some(i) = n.as_i64() {
                wide::whole_number(i)
            } else if let Some(u) = n.as_u64() {
                wide::whole_number_u(u)
            } else if let Some(f) = n.as_f64() {
                Value::Double(f)
            } else {
                v_str(&n.to_string())
            }
        }
        Json::String(s) => v_str(s),
        other => v_str(&other.to_string()),
    }
}

/// `Get(path)`: `a.b.0.c` — a key of an object, an index of an array.
pub fn get_path(root: &Json, path: &str) -> Value {
    let mut current = root;
    for part in path.split('.') {
        if part.is_empty() {
            continue;
        }
        let next = match current {
            Json::Array(a) => part.parse::<usize>().ok().and_then(|i| a.get(i)),
            Json::Object(o) => o.get(part),
            _ => None,
        };
        match next {
            Some(v) => current = v,
            None => return v_str(""),
        }
    }
    from_json(current)
}

/// `Set(path, value)`: the objects on the way made when missing.
pub fn set_path(root: &mut Json, path: &str, val: &Value) {
    let parts: Vec<&str> = path.split('.').filter(|p| !p.is_empty()).collect();
    let Some((last, before)) = parts.split_last() else { return };
    let mut current = root;
    for part in before {
        let Json::Object(obj) = current else { return };
        current = obj.entry(part.to_string()).or_insert_with(|| Json::Object(serde_json::Map::new()));
    }
    if let Json::Object(obj) = current {
        obj.insert(last.to_string(), to_json(val));
    }
}

/// The methods that only read or change the document (`None` for another
/// one: the runtime's). `doc` is the component's document, `None` before
/// one was parsed or set.
pub fn call(doc: &mut Option<Json>, method: &str, args: &[Value]) -> Option<Value> {
    let arg = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    Some(match method {
        "stringify" => v_str(&doc.as_ref().map_or_else(|| "{}".to_string(), Json::to_string)),
        "prettify" => v_str(&doc.as_ref().and_then(|d| serde_json::to_string_pretty(d).ok()).unwrap_or_else(|| "{}".into())),
        "get" => doc.as_ref().map_or_else(|| v_str(""), |d| get_path(d, &arg(0))),
        "set" => {
            let root = doc.get_or_insert_with(|| Json::Object(serde_json::Map::new()));
            set_path(root, &arg(0), &args.get(1).cloned().unwrap_or(Value::Null));
            Value::Null
        }
        "has" => v_int(i64::from(doc.as_ref().and_then(Json::as_object).is_some_and(|o| o.contains_key(&arg(0))))),
        "remove" => {
            if let Some(Json::Object(o)) = doc.as_mut() {
                // (the other keys keep their order)
                o.shift_remove(&arg(0));
            }
            Value::Null
        }
        "count" => v_int(match doc.as_ref() {
            Some(Json::Object(m)) => m.len() as i64,
            Some(Json::Array(a)) => a.len() as i64,
            _ => 0,
        }),
        "keys" => v_str(&doc.as_ref().and_then(Json::as_object).map(|o| o.keys().cloned().collect::<Vec<_>>().join(",")).unwrap_or_default()),
        "clear" => {
            *doc = Some(Json::Object(serde_json::Map::new()));
            Value::Null
        }
        _ => return None,
    })
}

/// `Parse(text)` / `LoadFile`'s reading: the document, or why not.
pub fn parse(text: &str) -> Result<Json, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// What `SaveFile` writes: the document pretty-printed ("{}" for none).
pub fn saved_text(doc: Option<&Json>) -> String {
    doc.and_then(|d| serde_json::to_string_pretty(d).ok()).unwrap_or_else(|| "{}".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_numbers_go_both_ways_exactly() {
        let mut doc = Some(parse(r#"{"big": 18446744073709551615, "neg": -9223372036854775808, "id": 9007199254740993, "small": 7, "x": 2.5}"#).unwrap());
        assert_eq!(call(&mut doc, "get", &[v_str("big")]), Some(Value::UInt64(u64::MAX)));
        assert_eq!(call(&mut doc, "get", &[v_str("neg")]), Some(Value::Int64(i64::MIN)));
        assert_eq!(call(&mut doc, "get", &[v_str("id")]), Some(Value::Int64(9007199254740993)));
        assert!(matches!(call(&mut doc, "get", &[v_str("small")]), Some(Value::Integer(7))), "a classic number when it fits 32 bits");
        assert!(matches!(call(&mut doc, "get", &[v_str("x")]), Some(Value::Double(f)) if f == 2.5));
        call(&mut doc, "set", &[v_str("out.u"), Value::UInt64(u64::MAX - 1)]);
        call(&mut doc, "set", &[v_str("out.i"), Value::Int64(-9007199254740993)]);
        let text = call(&mut doc, "stringify", &[]).unwrap().to_string_val();
        assert!(text.contains(r#""out":{"u":18446744073709551614,"i":-9007199254740993}"#), "{text}");
        assert!(text.starts_with(r#"{"big":18446744073709551615"#), "keys keep their order: {text}");
    }

    #[test]
    fn the_other_methods() {
        let mut doc = None;
        assert_eq!(call(&mut doc, "count", &[]), Some(v_int(0)));
        call(&mut doc, "set", &[v_str("a"), v_str("true")]);
        call(&mut doc, "set", &[v_str("b"), v_str("hello")]);
        assert_eq!(call(&mut doc, "keys", &[]).unwrap().to_string_val(), "a,b");
        assert_eq!(call(&mut doc, "has", &[v_str("b")]), Some(v_int(1)));
        call(&mut doc, "remove", &[v_str("a")]);
        assert_eq!(call(&mut doc, "stringify", &[]).unwrap().to_string_val(), r#"{"b":"hello"}"#);
        call(&mut doc, "clear", &[]);
        assert_eq!(call(&mut doc, "count", &[]), Some(v_int(0)));
        assert_eq!(call(&mut doc, "loadfile", &[]), None);
    }
}
