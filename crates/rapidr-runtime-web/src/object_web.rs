//! Component registry and event system for the web runtime.
//!
//! Mirrors the desktop `object.rs` API — same function signatures so that
//! generated code works identically on both targets.

use crate::gui_web;
use crate::value::{v_bool, v_int, v_null, v_str, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// ---------------------------------------------------------------------------
// Component struct
// ---------------------------------------------------------------------------

pub struct RpComponent {
    pub type_name: String,
    pub properties: HashMap<String, Value>,
    pub creation_order: u32,
}

// ---------------------------------------------------------------------------
// Thread-local storage (single-threaded in WASM, but keeps API compatible)
// ---------------------------------------------------------------------------

enum EventHandler {
    Arity0(fn()),
    Arity1(fn(Value)),
    Arity2(fn(Value, Value)),
    Arity3(fn(Value, Value, Value)),
    Arity4(fn(Value, Value, Value, Value)),
    Arity5(fn(Value, Value, Value, Value, Value)),
}

thread_local! {
    static COMPONENTS: RefCell<HashMap<String, RpComponent>> = RefCell::new(HashMap::new());
    static EVENT_HANDLERS: RefCell<HashMap<(String, String), EventHandler>> = RefCell::new(HashMap::new());
    static CREATION_COUNTER: RefCell<u32> = RefCell::new(0);
    static STRINGLISTS: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
    static TIMER_HANDLES: RefCell<HashMap<String, i32>> = RefCell::new(HashMap::new());
}

// ---------------------------------------------------------------------------
// Component creation
// ---------------------------------------------------------------------------

pub fn rp_create_component(name: &str, type_name: &str) {
    let uname = name.to_uppercase();
    let utype = type_name.to_uppercase();

    // Idempotent: skip if already created
    let already = COMPONENTS.with(|c| c.borrow().contains_key(&uname));
    if already {
        return;
    }

    let order = CREATION_COUNTER.with(|c| {
        let mut c = c.borrow_mut();
        *c += 1;
        *c
    });

    let mut props = HashMap::new();
    // Set default properties based on type
    match utype.as_str() {
        "RFORM" => {
            props.insert("caption".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(100));
            props.insert("top".to_string(), v_int(100));
            props.insert("width".to_string(), v_int(640));
            props.insert("height".to_string(), v_int(480));
            props.insert("visible".to_string(), v_bool(true));
        }
        "RBUTTON" => {
            props.insert("caption".to_string(), v_str("Button"));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(100));
            props.insert("height".to_string(), v_int(30));
        }
        "RLABEL" => {
            props.insert("caption".to_string(), v_str("Label"));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(100));
            props.insert("height".to_string(), v_int(20));
        }
        "REDIT" => {
            props.insert("text".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(120));
            props.insert("height".to_string(), v_int(25));
        }
        "RMEMO" | "RRICHEDIT" => {
            props.insert("text".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(200));
            props.insert("height".to_string(), v_int(150));
        }
        "RPANEL" | "RDESIGNSURFACE" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(200));
            props.insert("height".to_string(), v_int(150));
        }
        "RCHECKBOX" | "RRADIOBUTTON" => {
            props.insert("caption".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(120));
            props.insert("height".to_string(), v_int(25));
        }
        "RCOMBOBOX" | "RLISTBOX" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(150));
            props.insert("height".to_string(), v_int(25));
        }
        "RTIMER" => {
            props.insert("interval".to_string(), v_int(1000));
            props.insert("enabled".to_string(), v_bool(false));
        }
        "RIMAGE" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(100));
            props.insert("height".to_string(), v_int(100));
        }
        "RCANVAS" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(400));
            props.insert("height".to_string(), v_int(300));
        }
        "RSTRINGGRID" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(300));
            props.insert("height".to_string(), v_int(200));
            props.insert("rowcount".to_string(), v_int(5));
            props.insert("colcount".to_string(), v_int(3));
        }
        "RPROGRESS" | "RPROGRESSBAR" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(200));
            props.insert("height".to_string(), v_int(25));
            props.insert("min".to_string(), v_int(0));
            props.insert("max".to_string(), v_int(100));
            props.insert("position".to_string(), v_int(0));
        }
        "RWEBVIEW" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(400));
            props.insert("height".to_string(), v_int(300));
        }
        "RWEBAUDIO" | "RWEBVIDEO" => {
            props.insert("src".to_string(), v_str(""));
            props.insert("volume".to_string(), Value::Double(1.0));
        }
        "RWEBSTORAGE" => {
            props.insert("storagetype".to_string(), v_str("local"));
        }
        "RWEBNOTIFICATION" => {
            props.insert("title".to_string(), v_str("Notification"));
            props.insert("body".to_string(), v_str(""));
        }
        "RSTRINGLIST" => {
            STRINGLISTS.with(|sl| {
                sl.borrow_mut().insert(uname.clone(), Vec::new());
            });
        }
        "RNUM" => {
            // Non-visual component — no DOM element
        }
        "RDATAFRAME" => {
            // Non-visual component — no DOM element
            crate::datascience_web::init_dataframe(&uname);
        }
        "RPLOT" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(600));
            props.insert("height".to_string(), v_int(400));
        }
        "RSQLITE" => {
            props.insert("connected".to_string(), v_int(0));
            props.insert("db".to_string(), v_str(""));
            props.insert("rowcount".to_string(), v_int(0));
            props.insert("colcount".to_string(), v_int(0));
            props.insert("fieldcount".to_string(), v_int(0));
        }
        _ => {
            // Generic defaults
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(100));
            props.insert("height".to_string(), v_int(25));
        }
    }

    // Create the DOM element (skip for non-visual components)
    match utype.as_str() {
        "RNUM" | "RDATAFRAME" | "RSQLITE" => {
            // Non-visual: no DOM element
        }
        "RPLOT" => {
            crate::datascience_web::create_plot_widget(
                &format!("rr-{}", uname.to_lowercase()),
                &uname,
                &props,
            );
        }
        _ => {
            gui_web::gui_web_create_widget(&uname, &utype, &props);
        }
    }

    COMPONENTS.with(|c| {
        c.borrow_mut().insert(
            uname,
            RpComponent {
                type_name: utype,
                properties: props,
                creation_order: order,
            },
        );
    });
}

// ---------------------------------------------------------------------------
// Property access
// ---------------------------------------------------------------------------

/// Update the property in the component store only (no DOM side-effects).
/// Used by internal tab switching, etc.
pub fn rp_comp_set_prop_only(name: &str, prop: &str, val: Value) {
    let uname = name.to_uppercase();
    let lprop = prop.to_lowercase();
    COMPONENTS.with(|c| {
        if let Some(comp) = c.borrow_mut().get_mut(&uname) {
            comp.properties.insert(lprop, val);
        }
    });
}

pub fn rp_comp_set(name: &str, prop: &str, val: Value) {
    let uname = name.to_uppercase();
    let lprop = prop.to_lowercase();

    // Handle timer interval/enabled specially
    COMPONENTS.with(|c| {
        let mut comps = c.borrow_mut();
        if let Some(comp) = comps.get_mut(&uname) {
            comp.properties.insert(lprop.clone(), val.clone());

            if comp.type_name == "RTIMER" {
                if lprop == "enabled" || lprop == "interval" {
                    drop(comps);
                    update_timer(&uname);
                    return;
                }
            }
        }
    });

    // Handle StringList operations
    if lprop == "text" {
        COMPONENTS.with(|c| {
            let comps = c.borrow();
            if let Some(comp) = comps.get(&uname) {
                if comp.type_name == "RSTRINGLIST" {
                    let text = val.to_string_val();
                    let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
                    STRINGLISTS.with(|sl| {
                        sl.borrow_mut().insert(uname.clone(), lines);
                    });
                    return;
                }
            }
        });
    }

    // Handle parent re-parenting
    if lprop == "parent" {
        gui_web::gui_web_set_parent(&uname, &val.to_string_val().to_uppercase());
        return;
    }

    // Handle data-science / database component property sets
    let comp_type = rp_comp_type(&uname);
    match comp_type.as_str() {
        "RNUM" => {
            crate::datascience_web::num_set_prop(&uname, &lprop, &val);
            return;
        }
        "RPLOT" => {
            crate::datascience_web::plot_set_prop(&uname, &lprop, &val);
            // Also pass through for visual properties (left/top/width/height)
            if matches!(lprop.as_str(), "title" | "xlabel" | "ylabel" | "grid" | "dpi") {
                return;
            }
        }
        "RSQLITE" | "RDATAFRAME" => {
            // These use the generic property store only (already inserted above)
            return;
        }
        _ => {}
    }

    // Pass to GUI layer for DOM update
    gui_web::gui_web_set_prop(&uname, &lprop, &val);
}

pub fn rp_comp_get(name: &str, prop: &str) -> Value {
    let uname = name.to_uppercase();
    let lprop = prop.to_lowercase();

    // Check StringList first
    let is_stringlist = COMPONENTS.with(|c| {
        c.borrow()
            .get(&uname)
            .map(|comp| comp.type_name == "RSTRINGLIST")
            .unwrap_or(false)
    });

    if is_stringlist {
        return match lprop.as_str() {
            "count" => STRINGLISTS.with(|sl| {
                v_int(sl.borrow().get(&uname).map(|v| v.len()).unwrap_or(0) as i64)
            }),
            "text" => STRINGLISTS.with(|sl| {
                v_str(
                    &sl.borrow()
                        .get(&uname)
                        .map(|v| v.join("\n"))
                        .unwrap_or_default(),
                )
            }),
            _ => v_null(),
        };
    }

    // Check data-science / database component properties
    let comp_type = rp_comp_type(&uname);
    match comp_type.as_str() {
        "RNUM" => {
            let v = crate::datascience_web::num_get_prop(&uname, &lprop);
            if !matches!(v, Value::Null) { return v; }
        }
        "RDATAFRAME" => {
            let v = crate::datascience_web::dataframe_get_prop(&uname, &lprop);
            if !matches!(v, Value::Null) { return v; }
        }
        "RPLOT" => {
            let v = crate::datascience_web::plot_get_prop(&uname, &lprop);
            if !matches!(v, Value::Null) { return v; }
        }
        "RSQLITE" => {
            // Uses generic property store — fall through
        }
        _ => {}
    }

    // Check stored properties first (for non-DOM properties)
    let stored = COMPONENTS.with(|c| {
        c.borrow()
            .get(&uname)
            .and_then(|comp| comp.properties.get(&lprop).cloned())
    });

    // For visual properties, prefer live DOM values
    match lprop.as_str() {
        "caption" | "text" | "left" | "top" | "width" | "height" | "visible" | "enabled"
        | "checked" | "value" | "listindex" | "itemindex" | "listcount" | "position"
        | "min" | "max" | "selstart" | "seltext" | "innerhtml" | "innertext" | "tagname"
        | "volume" | "currenttime" | "duration" | "playing" | "paused" => {
            gui_web::gui_web_get_prop(&uname, &lprop)
        }
        _ => stored.unwrap_or_else(v_null),
    }
}

pub fn rp_comp_type(name: &str) -> String {
    let uname = name.to_uppercase();
    COMPONENTS.with(|c| {
        c.borrow()
            .get(&uname)
            .map(|comp| comp.type_name.clone())
            .unwrap_or_default()
    })
}

// ---------------------------------------------------------------------------
// Component methods
// ---------------------------------------------------------------------------

pub fn rp_comp_method(name: &str, method: &str, args: &[Value]) -> Value {
    let uname = name.to_uppercase();
    let lmethod = method.to_lowercase();

    let comp_type = rp_comp_type(&uname);
    if comp_type.is_empty() {
        web_sys::console::warn_1(&JsValue::from_str(&format!(
            "[WARN] Component '{}' not found",
            name
        )));
        return v_null();
    }

    // StringList special handling
    if comp_type == "RSTRINGLIST" {
        return stringlist_method(&uname, &lmethod, args);
    }

    // HTTP special handling
    if comp_type == "RHTTP" {
        return crate::network_web::http_method(&uname, &lmethod, args);
    }

    // WebSocket special handling
    if comp_type == "RSOCKET" || comp_type == "RSERVERSOCKET" {
        return crate::network_web::websocket_method(&uname, &lmethod, args);
    }

    // Data-science special handling
    if comp_type == "RNUM" {
        return crate::datascience_web::num_method(&uname, &lmethod, args);
    }
    if comp_type == "RDATAFRAME" {
        return crate::datascience_web::dataframe_method(&uname, &lmethod, args);
    }
    if comp_type == "RPLOT" {
        return crate::datascience_web::plot_method(&uname, &lmethod, args);
    }

    // Database special handling
    if comp_type == "RSQLITE" {
        return crate::database_web::sqlite_method(&uname, &lmethod, args);
    }

    // Delegate to GUI layer
    gui_web::gui_web_method(&uname, &comp_type, &lmethod, args)
}

fn stringlist_method(name: &str, method: &str, args: &[Value]) -> Value {
    STRINGLISTS.with(|sl| {
        let mut lists = sl.borrow_mut();
        let list = lists.entry(name.to_string()).or_insert_with(Vec::new);

        match method {
            "add" if args.len() >= 1 => {
                list.push(args[0].to_string_val());
                v_null()
            }
            "insert" if args.len() >= 2 => {
                let idx = args[0].to_i64() as usize;
                if idx <= list.len() {
                    list.insert(idx, args[1].to_string_val());
                }
                v_null()
            }
            "delete" | "remove" if args.len() >= 1 => {
                let idx = args[0].to_i64() as usize;
                if idx < list.len() {
                    list.remove(idx);
                }
                v_null()
            }
            "clear" => {
                list.clear();
                v_null()
            }
            "get" | "strings" if args.len() >= 1 => {
                let idx = args[0].to_i64() as usize;
                if idx < list.len() {
                    v_str(&list[idx])
                } else {
                    v_str("")
                }
            }
            "indexof" | "find" if args.len() >= 1 => {
                let needle = args[0].to_string_val();
                v_int(
                    list.iter()
                        .position(|s| s == &needle)
                        .map(|i| i as i64)
                        .unwrap_or(-1),
                )
            }
            "sort" => {
                list.sort();
                v_null()
            }
            "savetofile" if args.len() >= 1 => {
                // On web, we can't save files directly — offer download instead
                let content = list.join("\n");
                let filename = args[0].to_string_val();
                let js = format!(
                    r#"var a=document.createElement('a');a.href='data:text/plain,'+encodeURIComponent("{}");a.download="{}";a.click();"#,
                    content.replace('"', r#"\""#).replace('\n', "\\n"),
                    filename.replace('"', r#"\""#)
                );
                let _ = js_sys::eval(&js);
                v_null()
            }
            "loadfromfile" => {
                // Not supported on web
                web_sys::console::warn_1(&JsValue::from_str(
                    "[WARN] StringList.LoadFromFile not supported on web",
                ));
                v_null()
            }
            _ => v_null(),
        }
    })
}

// ---------------------------------------------------------------------------
// Event binding
// ---------------------------------------------------------------------------

pub fn rp_bind_event(name: &str, event: &str, handler: fn()) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity0(handler));
    });

    bind_dom_event(&uname, &levent);
}

pub fn rp_bind_event_1(name: &str, event: &str, handler: fn(Value)) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity1(handler));
    });

    bind_dom_event(&uname, &levent);
}

pub fn rp_bind_event_2(name: &str, event: &str, handler: fn(Value, Value)) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity2(handler));
    });

    bind_dom_event(&uname, &levent);
}

pub fn rp_bind_event_3(name: &str, event: &str, handler: fn(Value, Value, Value)) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity3(handler));
    });

    bind_dom_event(&uname, &levent);
}

pub fn rp_bind_event_4(name: &str, event: &str, handler: fn(Value, Value, Value, Value)) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity4(handler));
    });

    bind_dom_event(&uname, &levent);
}

pub fn rp_bind_event_5(name: &str, event: &str, handler: fn(Value, Value, Value, Value, Value)) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity5(handler));
    });

    bind_dom_event(&uname, &levent);
}

// ---------------------------------------------------------------------------
// Event firing
// ---------------------------------------------------------------------------

pub fn rp_fire_event(name: &str, event: &str) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        let handlers = eh.borrow();
        if let Some(handler) = handlers.get(&(uname.clone(), levent.clone())) {
            match handler {
                EventHandler::Arity0(f) => f(),
                EventHandler::Arity1(f) => f(v_null()),
                _ => {}
            }
        }
    });
}

pub fn rp_fire_event_1(name: &str, event: &str, arg: Value) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        let handlers = eh.borrow();
        if let Some(handler) = handlers.get(&(uname.clone(), levent.clone())) {
            match handler {
                EventHandler::Arity0(f) => f(),
                EventHandler::Arity1(f) => f(arg.clone()),
                _ => {}
            }
        }
    });
}

pub fn rp_fire_event_2(name: &str, event: &str, arg1: Value, arg2: Value) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        let handlers = eh.borrow();
        if let Some(handler) = handlers.get(&(uname.clone(), levent.clone())) {
            match handler {
                EventHandler::Arity0(f) => f(),
                EventHandler::Arity1(f) => f(arg1.clone()),
                EventHandler::Arity2(f) => f(arg1.clone(), arg2.clone()),
                _ => {}
            }
        }
    });
}

pub fn rp_fire_event_5(
    name: &str,
    event: &str,
    a1: Value,
    a2: Value,
    a3: Value,
    a4: Value,
    a5: Value,
) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        let handlers = eh.borrow();
        if let Some(handler) = handlers.get(&(uname.clone(), levent.clone())) {
            match handler {
                EventHandler::Arity0(f) => f(),
                EventHandler::Arity1(f) => f(a1.clone()),
                EventHandler::Arity2(f) => f(a1.clone(), a2.clone()),
                EventHandler::Arity3(f) => f(a1.clone(), a2.clone(), a3.clone()),
                EventHandler::Arity4(f) => f(a1.clone(), a2.clone(), a3.clone(), a4.clone()),
                EventHandler::Arity5(f) => {
                    f(a1.clone(), a2.clone(), a3.clone(), a4.clone(), a5.clone())
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// DOM event binding — wire up browser events to fire RapidR events
// ---------------------------------------------------------------------------

fn bind_dom_event(name: &str, event: &str) {
    let id = format!("rr-{}", name.to_lowercase());
    let name_owned = name.to_string();
    let event_owned = event.to_string();

    // Timer events are handled specially — they don't need DOM binding
    if event == "ontimer" {
        update_timer(name);
        return;
    }

    // Router events — bind to window hashchange
    if event == "onroutechange" {
        let name_for_closure = name_owned.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            rp_fire_event(&name_for_closure, "onroutechange");
        });
        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback(
                "hashchange",
                closure.as_ref().unchecked_ref(),
            );
        }
        closure.forget();
        return;
    }

    let doc = web_sys::window().unwrap().document().unwrap();
    let el = match doc.get_element_by_id(&id) {
        Some(e) => e,
        None => return, // Virtual components (Timer, etc.) don't have DOM elements
    };

    let dom_event_name = match event {
        "onclick" => "click",
        "ondblclick" | "ondoubleclick" => "dblclick",
        "onchange" => "input",
        "onkeypress" | "onkeydown" => "keydown",
        "onkeyup" => "keyup",
        "onmousedown" => "mousedown",
        "onmouseup" => "mouseup",
        "onmousemove" => "mousemove",
        "onmouseover" | "onmouseenter" => "mouseenter",
        "onmouseout" | "onmouseleave" => "mouseleave",
        "onfocus" | "ongotfocus" => "focus",
        "onblur" | "onlostfocus" => "blur",
        "onscroll" => "scroll",
        "onload" => "load",
        "onresize" => "resize",
        "onplay" => "play",
        "onpause" => "pause",
        "onended" => "ended",
        "ontimeupdate" => "timeupdate",
        "oninput" => "input",
        "onclose" => "close",
        "onpermissionchange" => return, // handled differently
        _ => return,
    };

    // Create a JavaScript closure that fires the RapidR event
    let name_for_closure = name_owned.clone();
    let event_for_closure = event_owned.clone();

    // For keyboard events, we pass the key code as an argument
    if dom_event_name == "keydown" || dom_event_name == "keyup" {
        let closure = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            let key_code = e.key_code() as i64;
            let shift = e.shift_key();
            let ctrl = e.ctrl_key();
            let alt = e.alt_key();
            rp_fire_event_5(
                &name_for_closure,
                &event_for_closure,
                v_int(key_code),
                v_bool(shift),
                v_bool(ctrl),
                v_bool(alt),
                v_null(),
            );
        });
        let _ = el.add_event_listener_with_callback(dom_event_name, closure.as_ref().unchecked_ref());
        closure.forget();
        return;
    }

    // For mouse events, pass coordinates
    if dom_event_name == "mousemove"
        || dom_event_name == "mousedown"
        || dom_event_name == "mouseup"
    {
        let closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let x = e.offset_x() as i64;
            let y = e.offset_y() as i64;
            let button = e.button() as i64;
            rp_fire_event_5(
                &name_for_closure,
                &event_for_closure,
                v_int(x),
                v_int(y),
                v_int(button),
                v_null(),
                v_null(),
            );
        });
        let _ = el.add_event_listener_with_callback(dom_event_name, closure.as_ref().unchecked_ref());
        closure.forget();
        return;
    }

    // For all other events, fire arity 0
    let closure = Closure::<dyn FnMut()>::new(move || {
        rp_fire_event(&name_for_closure, &event_for_closure);
    });
    let _ = el.add_event_listener_with_callback(dom_event_name, closure.as_ref().unchecked_ref());
    closure.forget();
}

// ---------------------------------------------------------------------------
// Timer management
// ---------------------------------------------------------------------------

fn update_timer(name: &str) {
    let uname = name.to_uppercase();

    // Clear existing timer
    TIMER_HANDLES.with(|th| {
        let mut handles = th.borrow_mut();
        if let Some(handle) = handles.remove(&uname) {
            if let Some(window) = web_sys::window() {
                window.clear_interval_with_handle(handle);
            }
        }
    });

    // Check if timer should be running
    let (enabled, interval) = COMPONENTS.with(|c| {
        let comps = c.borrow();
        if let Some(comp) = comps.get(&uname) {
            let enabled = comp
                .properties
                .get("enabled")
                .map(|v| v.to_bool())
                .unwrap_or(false);
            let interval = comp
                .properties
                .get("interval")
                .map(|v| v.to_i64())
                .unwrap_or(1000);
            (enabled, interval)
        } else {
            (false, 1000)
        }
    });

    // Check if we have an ontimer event handler registered
    let has_handler = EVENT_HANDLERS.with(|eh| {
        eh.borrow().contains_key(&(uname.clone(), "ontimer".to_string()))
    });

    if enabled && has_handler && interval > 0 {
        let name_for_closure = uname.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            rp_fire_event(&name_for_closure, "ontimer");
        });

        if let Some(window) = web_sys::window() {
            match window.set_interval_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                interval as i32,
            ) {
                Ok(handle) => {
                    TIMER_HANDLES.with(|th| {
                        th.borrow_mut().insert(uname, handle);
                    });
                }
                Err(e) => {
                    web_sys::console::error_1(&e);
                }
            }
        }
        closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Component type checking
// ---------------------------------------------------------------------------

pub fn is_component_type(type_name: &str) -> bool {
    matches!(
        type_name.to_uppercase().as_str(),
        "RFORM"
            | "RBUTTON"
            | "RLABEL"
            | "REDIT"
            | "RPANEL"
            | "RCHECKBOX"
            | "RRADIOBUTTON"
            | "RCOMBOBOX"
            | "RLISTBOX"
            | "RTIMER"
            | "RIMAGE"
            | "RCANVAS"
            | "RSTRINGGRID"
            | "RTABCONTROL"
            | "RTREEVIEW"
            | "RMAINMENU"
            | "RMENUITEM"
            | "RPOPUPMENU"
            | "RGROUPBOX"
            | "RDESIGNSURFACE"
            | "RCODEEDITOR"
            | "ROPENDIALOG"
            | "RSAVEDIALOG"
            | "RCOLORDIALOG"
            | "RFONTDIALOG"
            | "RSTATUSBAR"
            | "RPROGRESS"
            | "RPROGRESSBAR"
            | "RRICHEDIT"
            | "RMEMO"
            | "RFILESTREAM"
            | "RSTRINGLIST"
            | "RTOOLBAR"
            | "RSCROLLBAR"
            | "RDATETIMEPICKER"
            | "RTRACKBAR"
            | "RUPDOWN"
            | "RPRINTER"
            | "RSQLITE"
            | "RMYSQL"
            | "RSOCKET"
            | "RSERVERSOCKET"
            | "RHTTP"
            | "RFORMMDI"
            | "RSPLITTER"
            | "RSCROLLBOX"
            | "RLISTVIEW"
            | "RNUM"
            | "RDATAFRAME"
            | "RPLOT"
            // Web-exclusive
            | "RWEBVIEW"
            | "RDOM"
            | "RJAVASCRIPT"
            | "RWEBSTORAGE"
            | "RWEBAUDIO"
            | "RWEBVIDEO"
            | "RWEBNOTIFICATION"
            | "RWEBGEOLOCATION"
            | "RROUTER"
    )
}

pub fn is_component_method(member: &str) -> bool {
    matches!(
        member.to_lowercase().as_str(),
        "additem"
            | "clear"
            | "removeitem"
            | "deleteitem"
            | "setfocus"
            | "focus"
            | "refresh"
            | "repaint"
            | "invalidate"
            | "show"
            | "hide"
            | "close"
            | "cls"
            | "line"
            | "rect"
            | "rectangle"
            | "fillrect"
            | "circle"
            | "fillcircle"
            | "drawtext"
            | "textout"
            | "setpixel"
            | "pset"
            | "setcell"
            | "getcell"
            | "setrowcount"
            | "setcolcount"
            | "addtab"
            | "removetab"
            | "addnode"
            | "add"
            | "insert"
            | "delete"
            | "remove"
            | "get"
            | "strings"
            | "indexof"
            | "find"
            | "sort"
            | "savetofile"
            | "loadfromfile"
            // Network methods
            | "open"
            | "send"
            | "receive"
            | "connect"
            | "disconnect"
            | "listen"
            // Web-exclusive methods
            | "sethtml"
            | "navigate"
            | "create"
            | "appendto"
            | "setattribute"
            | "getattribute"
            | "addclass"
            | "removeclass"
            | "toggleclass"
            | "queryselector"
            | "queryselectorall"
            | "eval"
            | "call"
            | "set"
            | "haskey"
            | "keys"
            | "play"
            | "pause"
            | "stop"
            | "seek"
            | "fullscreen"
            | "requestpermission"
            | "getposition"
            | "watchposition"
            | "clearwatch"
            | "addroute"
            | "back"
            | "forward"
            // RNum methods
            | "arange"
            | "linspace"
            | "zeros"
            | "ones"
            | "full"
            | "fromlist"
            | "from_list"
            | "sum"
            | "mean"
            | "min"
            | "max"
            | "std"
            | "var"
            | "variance"
            | "median"
            | "argmin"
            | "argmax"
            | "count"
            | "ptp"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "arcsin"
            | "acos"
            | "arccos"
            | "atan"
            | "arctan"
            | "sqrt"
            | "abs"
            | "exp"
            | "log"
            | "ln"
            | "log2"
            | "log10"
            | "floor"
            | "ceil"
            | "round"
            | "sign"
            | "reciprocal"
            | "square"
            | "negative"
            | "neg"
            | "subtract"
            | "sub"
            | "multiply"
            | "mul"
            | "divide"
            | "div"
            | "power"
            | "pow"
            | "mod"
            | "fmod"
            | "clip"
            | "clamp"
            | "reverse"
            | "flip"
            | "unique"
            | "shuffle"
            | "append"
            | "concatenate"
            | "slice"
            | "cumsum"
            | "cumprod"
            | "diff"
            | "dot"
            | "norm"
            | "normalize"
            | "any"
            | "all"
            | "nonzero"
            | "searchsorted"
            | "rand"
            | "random"
            | "randn"
            | "random_normal"
            | "normal"
            | "uniform"
            | "random_uniform"
            | "randint"
            | "choice"
            | "tolist"
            | "tostring"
            | "print"
            // RDataFrame methods
            | "loadfromcsv"
            | "readcsv"
            | "read_csv"
            | "savetocsv"
            | "to_csv"
            | "head"
            | "tail"
            | "cell"
            | "cellbyname"
            | "at"
            | "iloc"
            | "select"
            | "sort_values"
            | "filter"
            | "query"
            | "groupby"
            | "group_by"
            | "drop_column"
            | "rename_column"
            | "addcolumn"
            | "add_column"
            | "set_column"
            | "fillna"
            | "fill_null"
            | "dropna"
            | "drop_nulls"
            | "describe"
            | "value_counts"
            | "nunique"
            | "corr"
            | "correlation"
            | "sample"
            | "nlargest"
            | "nsmallest"
            | "info"
            | "dtypes"
            | "shape"
            | "merge"
            | "join"
            | "concat"
            | "transpose"
            | "t"
            | "apply"
            | "replace"
            | "columns"
            | "rows"
            | "rowcount"
            | "len"
            | "togrid"
            | "to_grid"
            | "display"
            // RPlot methods
            | "plot"
            | "bar"
            | "barh"
            | "scatter"
            | "step"
            | "area"
            | "fill_between"
            | "hist"
            | "histogram"
            | "pie"
            | "hline"
            | "axhline"
            | "vline"
            | "axvline"
            | "annotate"
            | "legend"
            | "savefig"
            | "save"
            | "render"
            | "figsize"
            | "xlim"
            | "ylim"
            // RSQLite methods
            | "fetchrow"
            | "fetchfield"
            | "fieldseek"
            | "rowseek"
            | "row"
            | "escapestring"
            | "execute"
    )
}

pub fn get_children_of(parent_name: &str) -> Vec<(String, String)> {
    let uname = parent_name.to_uppercase();
    COMPONENTS.with(|c| {
        let comps = c.borrow();
        let mut children: Vec<(String, String, u32)> = comps
            .iter()
            .filter(|(_, comp)| {
                comp.properties
                    .get("parent")
                    .map(|v| v.to_string_val().to_uppercase() == uname)
                    .unwrap_or(false)
            })
            .map(|(name, comp)| (name.clone(), comp.type_name.clone(), comp.creation_order))
            .collect();
        children.sort_by_key(|c| c.2);
        children.into_iter().map(|(n, t, _)| (n, t)).collect()
    })
}

// ---------------------------------------------------------------------------
// Run app — no-op on web (the browser IS the event loop)
// ---------------------------------------------------------------------------

pub fn rp_run_app() {
    // On web, the browser event loop handles everything.
    // This is intentionally a no-op.
}

// ---------------------------------------------------------------------------
// Theme — no-op on web (Tailwind CSS handles styling)
// ---------------------------------------------------------------------------

pub fn set_theme(_theme: &str) {
    // Themes don't apply to web — Tailwind provides the styling
}

pub fn gui_register_timer(_name: &str, _interval: i32) {
    // Timers are handled via DOM setInterval in update_timer()
}
