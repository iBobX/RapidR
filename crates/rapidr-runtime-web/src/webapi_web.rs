//! The web-only components without a window of their own, over the
//! browser's APIs: RJAVASCRIPT (the page's JavaScript), RWEBSTORAGE
//! (localStorage / sessionStorage), RWEBNOTIFICATION, RWEBGEOLOCATION and
//! RROUTER (the address's `#hash`). Their methods and live properties come
//! here before anything the UI kernel does (`object_web::rp_comp_method`,
//! `rp_comp_get`), so a name a form's method also has (Show) is theirs.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::object_web::{rp_comp_get_stored, rp_comp_set, rp_fire_event};
use crate::value::{v_null, v_str, Value};

/// Whether `type_name` is one of them.
pub fn is_webapi(type_name: &str) -> bool {
    matches!(type_name.to_ascii_uppercase().as_str(), "RJAVASCRIPT" | "RWEBSTORAGE" | "RWEBNOTIFICATION" | "RWEBGEOLOCATION" | "RROUTER")
}

/// A BASIC value for JavaScript.
pub fn value_to_jsvalue(v: &Value) -> JsValue {
    match v {
        Value::Null => JsValue::NULL,
        Value::Boolean(b) => JsValue::from_bool(*b),
        Value::Integer(n) => JsValue::from_f64(*n as f64),
        Value::Double(d) => JsValue::from_f64(*d),
        Value::String(s) => JsValue::from_str(s),
        Value::Array(a) => a.borrow().data.iter().map(value_to_jsvalue).collect::<js_sys::Array>().into(),
        Value::Object(o) => JsValue::from_str(&o.id),
    }
}

/// A JavaScript value for BASIC: numbers (whole ones as integers), strings,
/// booleans; anything else as its JSON (or its `toString()`).
pub fn jsvalue_to_value(v: &JsValue) -> Value {
    if v.is_null() || v.is_undefined() {
        return Value::Null;
    }
    if let Some(b) = v.as_bool() {
        return Value::Boolean(b);
    }
    if let Some(n) = v.as_f64() {
        return if n.fract() == 0.0 && n >= (i64::MIN as f64) && n <= (i64::MAX as f64) { Value::Integer(n as i64) } else { Value::Double(n) };
    }
    if let Some(s) = v.as_string() {
        return v_str(&s);
    }
    let s = js_sys::JSON::stringify(v).ok().and_then(|j| j.as_string()).unwrap_or_else(|| "null".to_string());
    if s != "null" {
        return v_str(&s);
    }
    js_sys::Reflect::get(v, &JsValue::from_str("toString"))
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .and_then(|f| f.call0(v).ok())
        .and_then(|r| r.as_string())
        .map_or(Value::Null, |s| v_str(&s))
}

/// One of their live properties (`None`: the stored one answers).
pub fn get_prop(name: &str, type_name: &str, prop: &str) -> Option<Value> {
    match (type_name.to_ascii_uppercase().as_str(), prop) {
        // (the route: the address's #hash without the #)
        ("RROUTER", "route" | "hash") => {
            let hash = web_sys::window().and_then(|w| w.location().hash().ok()).unwrap_or_default();
            Some(v_str(hash.trim_start_matches('#')))
        }
        _ => {
            let _ = name;
            None
        }
    }
}

/// A geolocation callback.
type Listener = Closure<dyn FnMut(JsValue)>;

/// A position's coordinates into the component, then its OnChange.
fn position_listener(name: &str) -> (Listener, Listener) {
    let owner = name.to_string();
    let ok = Closure::<dyn FnMut(JsValue)>::new(move |pos: JsValue| {
        if let Ok(pos) = pos.dyn_into::<web_sys::Position>() {
            let c = pos.coords();
            rp_comp_set(&owner, "latitude", Value::Double(c.latitude()));
            rp_comp_set(&owner, "longitude", Value::Double(c.longitude()));
            rp_comp_set(&owner, "accuracy", Value::Double(c.accuracy()));
            rp_fire_event(&owner, "onchange");
        }
    });
    let failed = Closure::<dyn FnMut(JsValue)>::new(move |err: JsValue| {
        if let Ok(err) = err.dyn_into::<web_sys::PositionError>() {
            web_sys::console::error_1(&err.message().into());
        }
    });
    (ok, failed)
}

/// One of their methods (`None`: not theirs).
pub fn method(name: &str, type_name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let arg = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    let storage = || rp_comp_get_stored(name, "storagetype").to_string_val();
    let window = web_sys::window();
    Some(match (type_name.to_ascii_uppercase().as_str(), method) {
        // (running the developer's own JavaScript is what it's for)
        ("RJAVASCRIPT", "eval") if !args.is_empty() => {
            #[allow(clippy::disallowed_methods)]
            let evaluated = js_sys::eval(&arg(0));
            match evaluated {
                Ok(result) => jsvalue_to_value(&result),
                Err(e) => {
                    web_sys::console::error_1(&e);
                    v_null()
                }
            }
        }
        ("RJAVASCRIPT", "call") if !args.is_empty() => {
            let js_args: js_sys::Array = args.iter().skip(1).map(value_to_jsvalue).collect();
            let func = window.as_ref().and_then(|w| js_sys::Reflect::get(w, &JsValue::from_str(&arg(0))).ok()).and_then(|f| f.dyn_into::<js_sys::Function>().ok());
            match func.map(|f| f.apply(&JsValue::NULL, &js_args)) {
                Some(Ok(result)) => jsvalue_to_value(&result),
                Some(Err(e)) => {
                    web_sys::console::error_1(&e);
                    v_null()
                }
                None => v_null(),
            }
        }
        ("RWEBSTORAGE", "set") if args.len() >= 2 => {
            crate::storage_web::storage_set(&storage(), &arg(0), &arg(1));
            v_null()
        }
        ("RWEBSTORAGE", "get") if !args.is_empty() => crate::storage_web::storage_get(&storage(), &arg(0)),
        ("RWEBSTORAGE", "remove") if !args.is_empty() => {
            crate::storage_web::storage_remove(&storage(), &arg(0));
            v_null()
        }
        ("RWEBSTORAGE", "clear") => {
            crate::storage_web::storage_clear(&storage());
            v_null()
        }
        ("RWEBSTORAGE", "keys") => crate::storage_web::storage_keys(&storage()),
        ("RWEBSTORAGE", "haskey") if !args.is_empty() => crate::storage_web::storage_has_key(&storage(), &arg(0)),
        ("RWEBNOTIFICATION", "show") => {
            let options = web_sys::NotificationOptions::new();
            options.set_body(&rp_comp_get_stored(name, "body").to_string_val());
            let _ = web_sys::Notification::new_with_options(&rp_comp_get_stored(name, "title").to_string_val(), &options);
            v_null()
        }
        ("RWEBNOTIFICATION", "requestpermission") => {
            let _ = web_sys::Notification::request_permission();
            v_null()
        }
        ("RWEBGEOLOCATION", "getposition") => {
            if let Some(geo) = window.and_then(|w| w.navigator().geolocation().ok()) {
                let (ok, failed) = position_listener(name);
                let _ = geo.get_current_position_with_error_callback(ok.as_ref().unchecked_ref(), Some(failed.as_ref().unchecked_ref()));
                ok.forget();
                failed.forget();
            }
            v_null()
        }
        ("RWEBGEOLOCATION", "watchposition") => {
            let Some(geo) = window.and_then(|w| w.navigator().geolocation().ok()) else { return Some(v_null()) };
            let (ok, failed) = position_listener(name);
            match geo.watch_position_with_error_callback(ok.as_ref().unchecked_ref(), Some(failed.as_ref().unchecked_ref())) {
                Ok(id) => {
                    ok.forget();
                    failed.forget();
                    rp_comp_set(name, "watchid", Value::Integer(i64::from(id)));
                    Value::Integer(i64::from(id))
                }
                Err(_) => v_null(),
            }
        }
        ("RWEBGEOLOCATION", "clearwatch") => {
            let id = rp_comp_get_stored(name, "watchid").to_i64();
            if id != 0 {
                if let Some(geo) = window.and_then(|w| w.navigator().geolocation().ok()) {
                    geo.clear_watch(id as i32);
                    rp_comp_set(name, "watchid", Value::Integer(0));
                }
            }
            v_null()
        }
        ("RROUTER", "navigate") if !args.is_empty() => {
            if let Some(w) = window {
                let _ = w.location().set_hash(&arg(0));
            }
            v_null()
        }
        ("RROUTER", "back") => {
            if let Some(h) = window.and_then(|w| w.history().ok()) {
                let _ = h.back();
            }
            v_null()
        }
        ("RROUTER", "forward") => {
            if let Some(h) = window.and_then(|w| w.history().ok()) {
                let _ = h.forward();
            }
            v_null()
        }
        _ => return None,
    })
}
