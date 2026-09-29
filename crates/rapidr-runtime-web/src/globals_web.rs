//! RapidQ's global objects in the browser (rapidr_value::globals): the
//! screen and the mouse as the page sees them; the clipboard is the
//! program's own, also written to the system clipboard when the browser
//! allows (reading it back needs a permission prompt, so it isn't).

use rapidr_value::globals::Platform;
use rapidr_value::Value;
use std::cell::{Cell, RefCell};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

struct Web;

thread_local! {
    static CLIPBOARD: RefCell<String> = const { RefCell::new(String::new()) };
    /// The mouse on the screen, from the page's last mouse move.
    static MOUSE: Cell<(i64, i64)> = const { Cell::new((0, 0)) };
    static TRACKING: Cell<bool> = const { Cell::new(false) };
}

/// Follows the mouse over the page (Screen.MouseX / MouseY).
pub fn track_mouse() {
    if TRACKING.with(|t| t.replace(true)) {
        return;
    }
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| {
        MOUSE.with(|m| m.set((i64::from(e.screen_x()), i64::from(e.screen_y()))));
    });
    let _ = doc.add_event_listener_with_callback_and_bool("mousemove", cb.as_ref().unchecked_ref(), true);
    cb.forget();
}

fn screen_prop(prop: &str) -> i64 {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"screen".into()).ok())
        .and_then(|s| js_sys::Reflect::get(&s, &prop.into()).ok())
        .and_then(|v| v.as_f64())
        .map_or(0, |v| v as i64)
}

impl Platform for Web {
    fn screen_size(&self) -> (i64, i64) {
        (screen_prop("width"), screen_prop("height"))
    }

    fn work_area(&self) -> (i64, i64) {
        (screen_prop("availWidth"), screen_prop("availHeight"))
    }

    fn mouse(&self) -> (i64, i64) {
        track_mouse();
        MOUSE.with(Cell::get)
    }

    fn clipboard_text(&self) -> String {
        CLIPBOARD.with(|c| c.borrow().clone())
    }

    fn set_clipboard_text(&self, text: &str) {
        CLIPBOARD.with(|c| *c.borrow_mut() = text.to_string());
        // navigator.clipboard.writeText (a promise; refused without focus)
        let Some(window) = web_sys::window() else { return };
        let clipboard = js_sys::Reflect::get(&window.navigator(), &"clipboard".into()).ok().filter(|c| !c.is_undefined());
        if let Some(clipboard) = clipboard {
            if let Some(write) = js_sys::Reflect::get(&clipboard, &"writeText".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok()) {
                if let Ok(promise) = write.call1(&clipboard, &text.into()).and_then(|p| p.dyn_into::<js_sys::Promise>()) {
                    let ignore = Closure::<dyn FnMut(JsValue)>::new(|_| {});
                    let _ = promise.catch(&ignore);
                    ignore.forget();
                }
            }
        }
    }

    fn exe_path(&self) -> String {
        web_sys::window().and_then(|w| w.location().pathname().ok()).unwrap_or_default()
    }

    fn terminate(&self) {
        crate::object_web::end_program();
    }

    fn set_cursor(&self, cursor: i64) {
        // Every element shows Screen.Cursor while it's set.
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
        let style = match doc.get_element_by_id("rr-screen-cursor") {
            Some(el) => el,
            None => {
                let Ok(el) = doc.create_element("style") else { return };
                el.set_id("rr-screen-cursor");
                if let Some(head) = doc.document_element() {
                    let _ = head.append_child(&el);
                }
                el
            }
        };
        let css = rapidr_value::input::Cursor::of(cursor).css();
        style.set_text_content(Some(&if cursor == 0 { String::new() } else { format!("* {{ cursor: {css} !important; }}") }));
    }

    fn set_title(&self, title: &str) {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            doc.set_title(title);
        }
    }
}

/// Whether `name` is a global object rather than a component the program
/// made with that name.
fn is_global(name: &str) -> bool {
    rapidr_value::globals::global(name).is_some() && !matches!(crate::object_web::rp_comp_type(name).as_str(), t if !t.is_empty() && t != "RUDT")
}

pub fn get(name: &str, prop: &str) -> Option<Value> {
    if !is_global(name) {
        return None;
    }
    rapidr_value::globals::get(&Web, name, prop)
}

pub fn set(name: &str, prop: &str, value: &Value) -> bool {
    is_global(name) && rapidr_value::globals::set(&Web, name, prop, value)
}

pub fn call(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    if !is_global(name) {
        return None;
    }
    rapidr_value::globals::call(&Web, name, method, args)
}
