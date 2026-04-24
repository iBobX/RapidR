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
    let name_owned = name.to_string();

    // Use synchronous XMLHttpRequest for compatibility with the generated code's
    // sequential execution model. Async fetch would require the generated code
    // to be restructured as async, which is a much larger change.
    let js = format!(
        r#"(function() {{
            var __ov = document.getElementById('rr-busy-overlay');
            if (!__ov) {{
                __ov = document.createElement('div');
                __ov.id = 'rr-busy-overlay';
                __ov.style.cssText = 'position:fixed;top:0;left:0;right:0;bottom:0;z-index:2147483647;background:rgba(0,0,0,0.35);display:flex;align-items:center;justify-content:center;font:14px sans-serif;color:#fff;cursor:progress;';
                __ov.innerHTML = '<div style=\"background:#222;padding:14px 22px;border-radius:8px;box-shadow:0 6px 20px rgba(0,0,0,0.4);\">⏳ Working… (network)</div>';
                document.body.appendChild(__ov);
            }}
            void document.body.offsetHeight; // force layout flush so overlay paints before sync XHR
            try {{
                var xhr = new XMLHttpRequest();
                xhr.open("GET", "{}", false);
                xhr.send(null);
                return JSON.stringify({{ status: xhr.status, body: xhr.responseText }});
            }} finally {{
                if (__ov && __ov.parentNode) __ov.parentNode.removeChild(__ov);
            }}
        }})()"#,
        url.replace('\\', "\\\\").replace('"', "\\\"")
    );

    match js_sys::eval(&js) {
        Ok(result) => {
            let json_str = result.as_string().unwrap_or_default();
            // Parse the JSON response
            let (status, body) = parse_xhr_response(&json_str);
            rp_comp_set(&name_owned, "statuscode", v_int(status));
            rp_comp_set(&name_owned, "responsetext", v_str(&body));
            rp_comp_set(&name_owned, "url", v_str(&url));
            v_str(&body)
        }
        Err(e) => {
            web_sys::console::error_1(&e);
            rp_comp_set(&name_owned, "statuscode", v_int(0));
            rp_comp_set(&name_owned, "responsetext", v_str(""));
            v_str("")
        }
    }
}

fn http_post(name: &str, args: &[Value]) -> Value {
    let url = args.first().map(|v| v.to_string_val()).unwrap_or_default();
    let body = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
    let name_owned = name.to_string();

    // Properly escape body for embedding inside a JS string literal — must
    // escape backslash, quote, CR, LF, and tab so multi-line bodies (e.g.
    // RapidR source code) survive the round-trip.
    fn js_escape(s: &str) -> String {
        let mut o = String::with_capacity(s.len() + 8);
        for c in s.chars() {
            match c {
                '\\' => o.push_str("\\\\"),
                '"' => o.push_str("\\\""),
                '\n' => o.push_str("\\n"),
                '\r' => o.push_str("\\r"),
                '\t' => o.push_str("\\t"),
                _ => o.push(c),
            }
        }
        o
    }

    let js = format!(
        r#"(function() {{
            var __ov = document.getElementById('rr-busy-overlay');
            if (!__ov) {{
                __ov = document.createElement('div');
                __ov.id = 'rr-busy-overlay';
                __ov.style.cssText = 'position:fixed;top:0;left:0;right:0;bottom:0;z-index:2147483647;background:rgba(0,0,0,0.35);display:flex;align-items:center;justify-content:center;font:14px sans-serif;color:#fff;cursor:progress;';
                __ov.innerHTML = '<div style=\"background:#222;padding:14px 22px;border-radius:8px;box-shadow:0 6px 20px rgba(0,0,0,0.4);\">⏳ Working… (network)</div>';
                document.body.appendChild(__ov);
            }}
            void document.body.offsetHeight; // force layout flush so overlay paints before sync XHR
            try {{
                var xhr = new XMLHttpRequest();
                xhr.open("POST", "{}", false);
                xhr.setRequestHeader("Content-Type", "text/plain; charset=utf-8");
                xhr.send("{}");
                return JSON.stringify({{ status: xhr.status, body: xhr.responseText }});
            }} finally {{
                if (__ov && __ov.parentNode) __ov.parentNode.removeChild(__ov);
            }}
        }})()"#,
        js_escape(&url),
        js_escape(&body)
    );

    match js_sys::eval(&js) {
        Ok(result) => {
            let json_str = result.as_string().unwrap_or_default();
            let (status, resp_body) = parse_xhr_response(&json_str);
            rp_comp_set(&name_owned, "statuscode", v_int(status));
            rp_comp_set(&name_owned, "responsetext", v_str(&resp_body));
            rp_comp_set(&name_owned, "url", v_str(&url));
            v_str(&resp_body)
        }
        Err(e) => {
            web_sys::console::error_1(&e);
            rp_comp_set(&name_owned, "statuscode", v_int(0));
            rp_comp_set(&name_owned, "responsetext", v_str(""));
            v_str("")
        }
    }
}

fn parse_xhr_response(json_str: &str) -> (i64, String) {
    // Minimal JSON parse — we know the exact shape: {"status":NNN,"body":"..."}
    let status = json_str
        .find("\"status\":")
        .and_then(|i| {
            let start = i + 9;
            let end = json_str[start..].find(',').map(|e| start + e).unwrap_or(json_str.len());
            json_str[start..end].trim().parse::<i64>().ok()
        })
        .unwrap_or(0);

    let body = json_str
        .find("\"body\":\"")
        .map(|i| {
            let start = i + 8;
            // Find the closing quote — handle escaped quotes
            let mut end = start;
            let bytes = json_str.as_bytes();
            while end < bytes.len() {
                if bytes[end] == b'"' && (end == start || bytes[end - 1] != b'\\') {
                    break;
                }
                end += 1;
            }
            // JSON-unescape the captured slice so callers see the raw response.
            let raw = &json_str[start..end];
            let mut out = String::with_capacity(raw.len());
            let mut chars = raw.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('n') => out.push('\n'),
                        Some('r') => out.push('\r'),
                        Some('t') => out.push('\t'),
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some('/') => out.push('/'),
                        Some('b') => out.push('\u{08}'),
                        Some('f') => out.push('\u{0C}'),
                        Some('u') => {
                            let hex: String = (&mut chars).take(4).collect();
                            if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                                if let Some(ch) = char::from_u32(cp) { out.push(ch); }
                            }
                        }
                        Some(other) => out.push(other),
                        None => {}
                    }
                } else {
                    out.push(c);
                }
            }
            out
        })
        .unwrap_or_default();

    (status, body)
}

// ---------------------------------------------------------------------------
// RSocket / WebSocket client
// ---------------------------------------------------------------------------

thread_local! {
    static WEBSOCKETS: RefCell<HashMap<String, web_sys::WebSocket>> = RefCell::new(HashMap::new());
}

pub fn websocket_method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "connect" | "open" => websocket_connect(name),
        "close" | "disconnect" => websocket_close(name),
        "write" | "writeline" | "send" => websocket_send(name, args),
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
    rp_comp_set(name, "connected", v_int(0));
    v_null()
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
