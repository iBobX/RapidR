//! Menus in the page: QMAINMENU bars and QPOPUPMENU pop-ups drawn from the
//! shared model (rapidr_value::objects::menu) — check marks, radio items,
//! disabled items, separators, submenus and ShortCut keys — drawn again
//! after each change, as the desktop's.

use std::cell::Cell;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use rapidr_value::objects::menu;

use crate::gui_web::{comp_id, create_el, document};

thread_local! {
    static QUEUED: Cell<bool> = const { Cell::new(false) };
    static DRAWN: Cell<u64> = const { Cell::new(u64::MAX) };
    static LISTENING: Cell<bool> = const { Cell::new(false) };
}

/// A caption with its `&x` shown as an underlined x ("&&" is a `&`).
fn caption_html(caption: &str) -> String {
    let esc = |c: char| match c {
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        '&' => "&amp;".to_string(),
        '"' => "&quot;".to_string(),
        c => c.to_string(),
    };
    let mut out = String::new();
    let mut chars = caption.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.next() {
                Some('&') => out.push_str("&amp;"),
                Some(n) => out.push_str(&format!("<u>{}</u>", esc(n))),
                None => {}
            }
        } else {
            out.push_str(&esc(c));
        }
    }
    out
}

/// The menu changed: it's drawn again once the program's code returns.
pub fn schedule() {
    listen();
    if QUEUED.with(|q| q.replace(true)) {
        return;
    }
    let cb = Closure::once_into_js(|| {
        QUEUED.with(|q| q.set(false));
        render_all();
    });
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 0);
    }
}

/// Every main menu bar in the page, from the model (if it changed).
pub fn render_all() {
    let rev = menu::revision();
    if DRAWN.with(|d| d.replace(rev)) == rev {
        return;
    }
    let Ok(bars) = document().query_selector_all("nav[data-rr-type=\"RMAINMENU\"]") else { return };
    for i in 0..bars.length() {
        if let Some(bar) = bars.item(i).and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok()) {
            let name = bar.get_attribute("data-rr-name").unwrap_or_default();
            bar.set_inner_html("");
            fill(&bar, &name, true);
            // (a11y_web: its ARIA follows)
            crate::a11y_web::changed(&name);
        }
    }
}

/// The items under `parent`, into `container` (a bar's tops, or a list).
fn fill(container: &web_sys::HtmlElement, parent: &str, top: bool) {
    for child in menu::children(parent) {
        let Some(n) = menu::with(&child, |n| n.clone()) else { continue };
        if n.caption == "-" {
            if !top {
                let sep = create_el("div");
                sep.set_class_name("rr-menu-sep");
                let _ = container.append_child(&sep);
            }
            continue;
        }
        let el = create_el("div");
        el.set_class_name(if top { "rr-menu-item-top" } else { "rr-menu-item-sub" });
        el.set_id(&comp_id(&child));
        let _ = el.set_attribute("data-rr-name", &child);
        let _ = el.set_attribute("data-rr-type", "RMENUITEM");
        if !n.enabled {
            let _ = el.class_list().add_1("rr-menu-disabled");
        }
        if !n.hint.is_empty() {
            let _ = el.set_attribute("title", &n.hint);
        }
        let submenu = !menu::children(&child).is_empty();
        let mark = if !n.checked || top {
            ""
        } else if n.radio {
            "\u{25CF}"
        } else {
            "\u{2713}"
        };
        let keys = if top || submenu { String::new() } else { menu::parse_shortcut(&n.shortcut).map(|s| s.text()).unwrap_or_default() };
        let arrow = if submenu && !top { "\u{25B8}" } else { "" };
        el.set_inner_html(&if top {
            caption_html(&n.caption)
        } else {
            format!(
                "<span class=\"rr-menu-mark\">{mark}</span><span class=\"rr-menu-text\">{}</span><span class=\"rr-menu-keys\">{keys}{arrow}</span>",
                caption_html(&n.caption)
            )
        });
        if submenu {
            let list = create_el("div");
            list.set_class_name("rr-dropdown-menu");
            fill(&list, &child, false);
            let _ = el.append_child(&list);
        } else if n.enabled {
            let item = child.clone();
            let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                e.stop_propagation();
                close_all();
                crate::object_web::rp_fire_event(&item, "onclick");
                schedule();
            });
            let _ = el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
            cb.forget();
        }
        let _ = container.append_child(&el);
    }
}

/// Closes open menus: the pop-ups, and a bar's dropdown until the mouse
/// leaves it (they open on hover).
fn close_all() {
    if let Ok(pops) = document().query_selector_all(".rr-popup-menu") {
        for i in 0..pops.length() {
            if let Some(p) = pops.item(i).and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok()) {
                let _ = p.style().set_property("display", "none");
            }
        }
    }
    if let Ok(bars) = document().query_selector_all("nav[data-rr-type=\"RMAINMENU\"]") {
        for i in 0..bars.length() {
            if let Some(b) = bars.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                let _ = b.class_list().add_1("rr-menu-closed");
            }
        }
    }
}

/// `PopupMenu.Popup(X, Y)`: OnPopup, then the menu at (X, Y) of the page.
pub fn popup(name: &str, x: i64, y: i64) {
    listen();
    crate::object_web::rp_fire_event(name, "onpopup");
    close_all();
    let id = comp_id(name);
    let el = match document().get_element_by_id(&id).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        Some(el) => el,
        None => {
            let el = create_el("div");
            el.set_id(&id);
            let _ = el.set_attribute("data-rr-name", name);
            if let Some(body) = document().body() {
                let _ = body.append_child(&el);
            }
            el
        }
    };
    el.set_class_name("rr-popup-menu rr-dropdown-menu");
    el.set_inner_html("");
    fill(&el, name, false);
    let align = menu::with(name, |n| n.alignment).unwrap_or(0);
    let style = el.style();
    let _ = style.set_property("position", "fixed");
    let _ = style.set_property("display", "block");
    let _ = style.set_property("z-index", "10000");
    let w = el.offset_width() as i64;
    let x = match align {
        1 => x - w,
        2 => x - w / 2,
        _ => x,
    };
    let _ = style.set_property("left", &format!("{x}px"));
    let _ = style.set_property("top", &format!("{y}px"));
}

/// The page's listeners, once: menu ShortCut keys, a right click on a
/// component with an AutoPopup PopupMenu, a click elsewhere closing pop-ups,
/// and a bar's dropdown opening again on the next hover.
fn listen() {
    if LISTENING.with(|l| l.replace(true)) {
        return;
    }
    let doc = document();
    let keys = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(|e: web_sys::KeyboardEvent| {
        let Some(vk) = rapidr_value::input::vk_of_key(&e.key(), &e.code()) else { return };
        let Ok(bars) = document().query_selector_all("nav[data-rr-type=\"RMAINMENU\"]") else { return };
        for i in 0..bars.length() {
            let Some(bar) = bars.item(i).and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok()) else { continue };
            if bar.offset_parent().is_none() {
                continue;
            }
            let root = bar.get_attribute("data-rr-name").unwrap_or_default();
            if let Some(item) = menu::item_for_shortcut(&root, vk, e.ctrl_key() || e.meta_key(), e.shift_key(), e.alt_key()) {
                e.prevent_default();
                crate::object_web::rp_fire_event(&item, "onclick");
                schedule();
                return;
            }
        }
    });
    let _ = doc.add_event_listener_with_callback("keydown", keys.as_ref().unchecked_ref());
    keys.forget();
    let context = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| {
        let mut node = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        while let Some(el) = node {
            if let Some(name) = el.get_attribute("data-rr-name") {
                let pm = crate::object_web::rp_comp_get(&name, "popupmenu").to_string_val();
                if !pm.is_empty() && menu::with(&pm, |n| n.kind == menu::Kind::Popup && n.auto_popup).unwrap_or(false) {
                    e.prevent_default();
                    popup(&pm, e.client_x() as i64, e.client_y() as i64);
                    return;
                }
            }
            node = el.parent_element();
        }
    });
    let _ = doc.add_event_listener_with_callback("contextmenu", context.as_ref().unchecked_ref());
    context.forget();
    let down = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| {
        let inside = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest(".rr-popup-menu").ok().flatten()).is_some();
        if !inside && e.button() == 0 {
            if let Ok(pops) = document().query_selector_all(".rr-popup-menu") {
                for i in 0..pops.length() {
                    if let Some(p) = pops.item(i).and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok()) {
                        let _ = p.style().set_property("display", "none");
                    }
                }
            }
        }
    });
    let _ = doc.add_event_listener_with_callback("mousedown", down.as_ref().unchecked_ref());
    down.forget();
    let over = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| {
        // (a bar closed by a click opens again once the mouse left it)
        let on_bar = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest("nav[data-rr-type=\"RMAINMENU\"]").ok().flatten());
        if let Ok(bars) = document().query_selector_all("nav.rr-menu-closed") {
            for i in 0..bars.length() {
                if let Some(b) = bars.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                    if on_bar.as_ref() != Some(&b) {
                        let _ = b.class_list().remove_1("rr-menu-closed");
                    }
                }
            }
        }
    });
    let _ = doc.add_event_listener_with_callback("mouseover", over.as_ref().unchecked_ref());
    over.forget();
}
