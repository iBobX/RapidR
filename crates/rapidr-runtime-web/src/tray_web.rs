//! The system tray in the browser (`rapidr_value::tray`). A page has no
//! notification area, so the icons a program puts there with
//! QNOTIFYICONDATA and Shell_NotifyIcon show in a small strip of the page's
//! own, at its bottom right (`#rr-tray`): each icon its picture, its
//! tooltip as the title and as what a screen reader says, a button. Its
//! clicks are the mouse messages Windows sends (left and right presses and
//! releases, the double click, Enter / Space as a left click), heard in the
//! form's WndProc as on the desktop — so a program that hides its window
//! into the tray can always bring it back. Additive: no icon, no strip.

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use rapidr_value::{tray, v_int};

use crate::gui_web::{create_el, document};

/// The icons changed: the strip drawn again (after the program's code, so
/// several changes draw once).
pub fn changed() {
    let flush = Closure::once_into_js(render);
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(flush.unchecked_ref(), 0);
    }
}

/// Icon `key`'s mouse messages to its form's WndProc.
fn deliver(key: (i64, i64), mouse: &[i64]) {
    for [hwnd, msg, wparam, lparam] in tray::clicked(key, mouse) {
        let Some(form) = rapidr_value::handles::name_of(hwnd) else { continue };
        crate::object_web::rp_fire_event_args(&form.to_uppercase(), "onwndproc", &[v_int(hwnd), v_int(msg), v_int(wparam), v_int(lparam)]);
    }
}

fn render() {
    let doc = document();
    let shown = tray::shown();
    let strip = match doc.get_element_by_id("rr-tray") {
        Some(s) => s,
        None if shown.is_empty() => return,
        None => {
            let s = create_el("div");
            s.set_id("rr-tray");
            let _ = s.set_attribute("role", "toolbar");
            let _ = s.set_attribute("aria-label", "System tray");
            for (k, v) in [
                ("position", "fixed"),
                ("right", "4px"),
                ("bottom", "4px"),
                ("display", "flex"),
                ("gap", "4px"),
                ("padding", "3px"),
                ("background", "#d4d0c8"),
                ("border", "1px solid #808080"),
                ("z-index", "2147483000"),
            ] {
                let _ = s.style().set_property(k, v);
            }
            if let Some(body) = doc.body() {
                let _ = body.append_child(&s);
            }
            s.into()
        }
    };
    strip.set_inner_html("");
    let _ = strip.unchecked_ref::<web_sys::HtmlElement>().style().set_property("display", if shown.is_empty() { "none" } else { "flex" });
    for (key, tip, (w, h, rgba)) in shown {
        let Ok(img) = doc.create_element("img") else { continue };
        let Ok(img) = img.dyn_into::<web_sys::HtmlImageElement>() else { continue };
        img.set_class_name("rr-tray-icon");
        if let Some(url) = crate::gui_web::rgba_data_url(w, h, &rgba) {
            img.set_src(&url);
        }
        let _ = img.style().set_property("width", "16px");
        let _ = img.style().set_property("height", "16px");
        let _ = img.style().set_property("cursor", "pointer");
        img.set_title(&tip);
        let _ = img.set_attribute("alt", &tip);
        let _ = img.set_attribute("role", "button");
        let _ = img.set_attribute("tabindex", "0");
        let _ = img.set_attribute("aria-label", if tip.is_empty() { "Tray icon" } else { &tip });
        if let Some(form) = rapidr_value::handles::name_of(key.0) {
            let _ = img.set_attribute("data-form", &form);
        }
        let _ = img.set_attribute("data-uid", &key.1.to_string());
        let down = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            e.prevent_default();
            let m = match (e.button(), e.detail()) {
                (0, d) if d >= 2 && d % 2 == 0 => tray::WM_LBUTTONDBLCLK,
                (0, _) => tray::WM_LBUTTONDOWN,
                (1, _) => tray::WM_MBUTTONDOWN,
                _ => tray::WM_RBUTTONDOWN,
            };
            deliver(key, &[m]);
        });
        let up = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            e.prevent_default();
            let m = match e.button() {
                0 => tray::WM_LBUTTONUP,
                1 => tray::WM_MBUTTONUP,
                _ => tray::WM_RBUTTONUP,
            };
            deliver(key, &[m]);
        });
        let menu = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| e.prevent_default());
        let keys = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            if e.key() == "Enter" || e.key() == " " {
                e.prevent_default();
                deliver(key, tray::LEFT_CLICK);
            }
        });
        let _ = img.add_event_listener_with_callback("mousedown", down.as_ref().unchecked_ref());
        let _ = img.add_event_listener_with_callback("mouseup", up.as_ref().unchecked_ref());
        let _ = img.add_event_listener_with_callback("contextmenu", menu.as_ref().unchecked_ref());
        let _ = img.add_event_listener_with_callback("keydown", keys.as_ref().unchecked_ref());
        down.forget();
        up.forget();
        menu.forget();
        keys.forget();
        let _ = strip.append_child(&img);
    }
}
