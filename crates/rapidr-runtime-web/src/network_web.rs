//! Network backends for the web runtime — Fetch API and WebSocket.

use crate::object_web::{rp_comp_set, rp_fire_event_1, rp_fire_event_2};
use crate::value::{v_int, v_null, v_str, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// ---------------------------------------------------------------------------
// RHTTP — Fetch API wrapper
// ---------------------------------------------------------------------------

pub fn http_method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "get" => http_get(name, args),
        "post" => http_post(name, args),
        _ => {
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[WARN] RHTTP.{}() not implemented on web",
                method
            )));
            v_null()
        }
    }
}

fn http_get(name: &str, args: &[Value]) -> Value {
    let url = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    http_request(name, "GET", &url, None)
}

fn http_post(name: &str, args: &[Value]) -> Value {
    let url = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let body = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
    http_request(name, "POST", &url, Some(&body))
}

/// Performs the request and stores `StatusCode` / `ResponseText` / `URL` on
/// the component. Returns the response body ("" on network failure).
fn http_request(name: &str, method: &str, url: &str, body: Option<&str>) -> Value {
    let (status, text) = match sync_xhr(method, url, body) {
        Ok(r) => r,
        Err(e) => {
            web_sys::console::error_1(&e);
            (0, String::new())
        }
    };
    rp_comp_set(name, "statuscode", v_int(status));
    rp_comp_set(name, "responsetext", v_str(&text));
    rp_comp_set(name, "url", v_str(url));
    v_str(&text)
}

/// Synchronous XMLHttpRequest, kept synchronous to match the sequential
/// execution model of generated code (async fetch would require restructuring
/// it). URL and body are passed as data through web_sys — never spliced into
/// JavaScript source — so this works under a CSP without 'unsafe-eval'.
fn sync_xhr(method: &str, url: &str, body: Option<&str>) -> Result<(i64, String), JsValue> {
    let overlay = show_busy_overlay();
    let result = (|| {
        let xhr = web_sys::XmlHttpRequest::new()?;
        xhr.open_with_async(method, url, false)?;
        if body.is_some() {
            xhr.set_request_header("Content-Type", "text/plain; charset=utf-8")?;
        }
        xhr.send_with_opt_str(body)?;
        Ok((xhr.status()? as i64, xhr.response_text()?.unwrap_or_default()))
    })();
    if let Some(el) = overlay {
        el.remove();
    }
    result
}

/// Small "network…" badge so the user sees why the UI is blocked during the
/// synchronous request. Forces a layout flush so it paints before the XHR.
fn show_busy_overlay() -> Option<web_sys::Element> {
    let doc = crate::page_web::document();
    let body = doc.body()?;
    let el = doc.create_element("div").ok()?;
    el.set_id("rr-busy-overlay");
    let _ = el.set_attribute(
        "style",
        "position:fixed;bottom:8px;right:8px;z-index:2147483647;background:rgba(50,50,50,0.85);\
         color:#fff;font:12px sans-serif;padding:6px 10px;border-radius:4px;pointer-events:none;\
         box-shadow:0 2px 8px rgba(0,0,0,0.25);",
    );
    el.set_text_content(Some("⏳ network…"));
    body.append_child(&el).ok()?;
    let _ = body.offset_height();
    Some(el)
}

// ---------------------------------------------------------------------------
// RSocket / WebSocket client
// ---------------------------------------------------------------------------

thread_local! {
    static WEBSOCKETS: RefCell<HashMap<String, web_sys::WebSocket>> = RefCell::new(HashMap::new());
    static SOCKET_QUEUES: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

pub fn websocket_method(name: &str, method: &str, args: &[Value]) -> Value {
    // RapidQ's numbered QSOCKET calls (`Connect(Server$, Port%)`, `Open(Port%)`,
    // `ReadLine(Sock%)`, …): a page can't open TCP sockets, so they fail as
    // RapidQ reports it — no socket (-1), nothing ready, nothing read.
    let qsocket = matches!(
        (method, args.len()),
        ("connect", 2) | ("open", 1) | ("accept", 1) | ("connectionready", 1) | ("isserverready", 1) | ("isclientready", 2)
            | ("read" | "peek", 2) | ("readbyte", 1) | ("readline", 1) | ("write", 3) | ("writeline" | "writebyte", 2)
            | ("close", 1) | ("getpeername", 1) | ("gethostname" | "gethostip", 0)
    );
    if qsocket {
        thread_local! { static WARNED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
        if !WARNED.with(|w| w.replace(true)) {
            web_sys::console::warn_1(&JsValue::from_str("[WARN] QSOCKET: a web page can't open TCP sockets (use RSOCKET's WebSocket or RHTTP)"));
        }
        return match method {
            "connect" | "open" | "accept" => crate::value::v_int(-1),
            "connectionready" | "isserverready" | "isclientready" | "readbyte" | "write" | "writeline" | "writebyte" => crate::value::v_int(0),
            "gethostname" => crate::value::v_str("localhost"),
            "gethostip" => crate::value::v_str("127.0.0.1"),
            "close" => v_null(),
            _ => crate::value::v_str(""),
        };
    }
    match method {
        "connect" | "open" => websocket_connect(name),
        "close" | "disconnect" => websocket_close(name),
        "write" | "writeline" | "send" => websocket_send(name, args),
        "read" | "readline" | "receive" => websocket_read(name),
        _ => {
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[WARN] WebSocket.{}() not implemented",
                method
            )));
            v_null()
        }
    }
}

fn websocket_connect(name: &str) -> Value {
    let host = rp_comp_get_raw(name, "host");
    let port = rp_comp_get_raw(name, "port");

    // Build WebSocket URL
    let url = if host.starts_with("ws://") || host.starts_with("wss://") {
        host
    } else {
        let port_str = if port.is_empty() || port == "0" {
            String::new()
        } else {
            format!(":{}", port)
        };
        format!("ws://{}{}", host, port_str)
    };

    match web_sys::WebSocket::new(&url) {
        Ok(ws) => {
            let name_owned = name.to_string();

            // onopen
            let n = name_owned.clone();
            let onopen = Closure::<dyn FnMut()>::new(move || {
                rp_comp_set(&n, "connected", v_int(1));
                rp_fire_event_1(&n, "onconnect", v_str("connected"));
            });
            ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();

            // onmessage
            let n = name_owned.clone();
            let onmessage = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
                move |e: web_sys::MessageEvent| {
                    let data = e
                        .data()
                        .as_string()
                        .unwrap_or_else(|| format!("{:?}", e.data()));
                    let data_clone = data.clone();
                    SOCKET_QUEUES.with(|q| {
                        q.borrow_mut()
                            .entry(n.clone())
                            .or_insert_with(Vec::new)
                            .push(data_clone);
                    });
                    rp_fire_event_2(&n, "ondatareceived", v_str(""), v_str(&data));
                },
            );
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();

            // onclose
            let n = name_owned.clone();
            let onclose = Closure::<dyn FnMut()>::new(move || {
                rp_comp_set(&n, "connected", v_int(0));
                rp_fire_event_1(&n, "ondisconnect", v_str("closed"));
            });
            ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            onclose.forget();

            // onerror
            let n = name_owned.clone();
            let onerror = Closure::<dyn FnMut()>::new(move || {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "[WebSocket] Error on {}",
                    n
                )));
            });
            ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();

            WEBSOCKETS.with(|sockets| {
                sockets.borrow_mut().insert(name_owned, ws);
            });

            v_int(1)
        }
        Err(e) => {
            web_sys::console::error_1(&e);
            rp_comp_set(name, "connected", v_int(0));
            v_int(0)
        }
    }
}

fn websocket_close(name: &str) -> Value {
    WEBSOCKETS.with(|sockets| {
        if let Some(ws) = sockets.borrow_mut().remove(name) {
            let _ = ws.close();
        }
    });
    SOCKET_QUEUES.with(|q| {
        q.borrow_mut().remove(name);
    });
    rp_comp_set(name, "connected", v_int(0));
    v_null()
}

fn websocket_read(name: &str) -> Value {
    let msg = SOCKET_QUEUES.with(|q| {
        let mut queues = q.borrow_mut();
        if let Some(queue) = queues.get_mut(name) {
            if !queue.is_empty() {
                Some(queue.remove(0))
            } else {
                None
            }
        } else {
            None
        }
    });
    match msg {
        Some(s) => v_str(&s),
        None => v_str(""),
    }
}

fn websocket_send(name: &str, args: &[Value]) -> Value {
    let data = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    WEBSOCKETS.with(|sockets| {
        let sockets = sockets.borrow();
        if let Some(ws) = sockets.get(name) {
            match ws.send_with_str(&data) {
                Ok(()) => v_int(1),
                Err(_) => v_int(0),
            }
        } else {
            v_int(0)
        }
    })
}

/// Helper to read a component property as a raw string without going through
/// the full DOM read-back path (avoids circular dependency for network setup).
fn rp_comp_get_raw(name: &str, prop: &str) -> String {
    crate::object_web::rp_comp_get(name, prop).to_string_val()
}
