//! A QFORM's and a QSCROLLBOX's scroll bars in the page, as the desktop's
//! (rapidr-runtime-core's scroll.rs): the shared model
//! (rapidr_value::scrollbars) works out the ranges and the bars; scrolling
//! moves the components (their Left / Top); the bars are drawn over the
//! components from the model's ops and take the mouse before them.

use std::cell::{Cell, RefCell};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use rapidr_value::layout::Align;
use rapidr_value::scrollbars::{self, Child, Shift};

use crate::gui_web::{comp_id, create_el, document, get_el};
use crate::object_web::{get_children_of, rp_comp_get_stored, rp_comp_set, rp_comp_type};
use crate::value::{v_int, Value};

thread_local! {
    static UPDATING: Cell<bool> = const { Cell::new(false) };
    /// The bars held down: their container and the mouse's last place in
    /// its client area.
    static CAPTURE: RefCell<Option<(String, i64, i64)>> = const { RefCell::new(None) };
    static LISTENING: Cell<bool> = const { Cell::new(false) };
}

/// (not a QFORMMDI: its children's MDI client area is Windows' own)
pub fn scrolls(name: &str) -> bool {
    !name.is_empty() && scrollbars::scrolls(&rp_comp_type(name)) && !rapidr_value::mdi::is_mdi(name)
}

/// A QSCROLLBOX's edge (bsSingle, its default: 2 pixels).
pub fn border(name: &str) -> i64 {
    if rp_comp_type(name) != "RSCROLLBOX" {
        return 0;
    }
    match rp_comp_get_stored(name, "borderstyle") {
        Value::Null => 2,
        v if v.to_i64() == 0 => 0,
        _ => 2,
    }
}

/// The area the bars and the components share.
pub fn area(name: &str) -> (i64, i64) {
    if rp_comp_type(name) == "RFORM" {
        return crate::layout_web::form_area(name);
    }
    let b = border(name);
    let n = |p: &str| rp_comp_get_stored(name, p).to_i64();
    ((n("width") - 2 * b).max(0), (n("height") - 2 * b).max(0))
}

/// ClientWidth / ClientHeight: the area less the bars shown.
pub fn client(name: &str) -> (i64, i64) {
    let (w, h) = area(name);
    scrollbars::with(name, |s| s.client(w, h)).unwrap_or((w, h))
}

fn children(name: &str) -> Vec<(String, Child)> {
    get_children_of(name)
        .into_iter()
        .filter(|(c, t)| !matches!(t.as_str(), "RMAINMENU" | "RPOPUPMENU" | "RMENUITEM") && !matches!(rp_comp_get_stored(c, "width"), Value::Null))
        .map(|(c, _)| {
            let n = |p: &str| rp_comp_get_stored(&c, p).to_i64();
            let visible = match rp_comp_get_stored(&c, "visible") {
                Value::Null => true,
                v => v.to_bool(),
            };
            let child = Child { left: n("left"), top: n("top"), width: n("width"), height: n("height"), align: Align::from_value(n("align")), visible };
            (c, child)
        })
        .collect()
}

/// The bars worked out again for `name` (as the desktop's `scroll::update`).
pub fn update(name: &str) {
    if !scrolls(name) || UPDATING.with(Cell::get) {
        return;
    }
    UPDATING.with(|u| u.set(true));
    for _ in 0..2 {
        let (w, h) = area(name);
        let kids: Vec<Child> = children(name).into_iter().map(|(_, c)| c).collect();
        let before = scrollbars::with(name, |s| (s.horz.shown, s.vert.shown));
        let shift = scrollbars::with_mut(name, |s| s.update(w, h, &kids));
        move_children(name, shift);
        let after = scrollbars::with(name, |s| (s.horz.shown, s.vert.shown));
        if before == after {
            break;
        }
        crate::layout_web::realign(name, None);
    }
    UPDATING.with(|u| u.set(false));
    render(name);
}

/// A form shown: its bars (and its scroll boxes') worked out and drawn.
pub fn shown(form: &str) {
    update(form);
    let mut stack = vec![form.to_uppercase()];
    while let Some(p) = stack.pop() {
        for (c, t) in get_children_of(&p) {
            if t == "RSCROLLBOX" {
                update(&c);
            }
            stack.push(c);
        }
    }
}

/// The components moved by a scroll: Left by -dx, Top by -dy.
fn move_children(name: &str, (dx, dy): Shift) {
    if (dx, dy) == (0, 0) {
        return;
    }
    let kids = children(name);
    crate::layout_web::quietly(|| {
        for (c, k) in &kids {
            if dx != 0 {
                rp_comp_set(c, "left", v_int(k.left - dx));
            }
            if dy != 0 {
                rp_comp_set(c, "top", v_int(k.top - dy));
            }
        }
    });
}

pub fn user_scrolled(name: &str, shift: Shift) {
    move_children(name, shift);
    render(name);
}

pub fn get(name: &str, prop: &str) -> Option<Value> {
    if !scrolls(name) {
        return None;
    }
    scrollbars::with_mut(name, |s| s.get(prop))
}

pub fn set(name: &str, prop: &str, val: &Value) -> bool {
    if !scrolls(name) || scrollbars::with_mut(name, |s| s.set(prop, val)).is_none() {
        return false;
    }
    update(name);
    true
}

/// The element the components (and the bars) are in: a form's client
/// area, a scroll box itself.
fn client_el(name: &str) -> Option<web_sys::HtmlElement> {
    let id = comp_id(name);
    if rp_comp_type(name) == "RFORM" {
        get_el(&format!("{id}-client"))
    } else {
        get_el(&id)
    }
}

/// The bars drawn again (an SVG over the components).
pub fn render(name: &str) {
    let Some(host) = client_el(name) else { return };
    listen(name, &host);
    let overlay = match host.query_selector(":scope > .rr-scrollbars").ok().flatten().and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        Some(o) => o,
        None => {
            let o = create_el("div");
            o.set_class_name("rr-scrollbars");
            for (k, v) in [("position", "absolute"), ("left", "0"), ("top", "0"), ("pointer-events", "none"), ("z-index", "100000")] {
                let _ = o.style().set_property(k, v);
            }
            let _ = host.append_child(&o);
            o
        }
    };
    let (w, h) = area(name);
    let ops = scrollbars::with(name, |s| (s.vert.shown || s.horz.shown).then(|| s.ops(w, h))).flatten();
    let Some(ops) = ops else {
        overlay.set_inner_html("");
        return;
    };
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" style=\"display:block\">");
    for op in ops {
        match op {
            rapidr_value::objects::tabcontrol::Op::Fill { rect: (x, y, rw, rh), color } => {
                svg.push_str(&format!("<rect x=\"{x}\" y=\"{y}\" width=\"{rw}\" height=\"{rh}\" fill=\"#{color:06x}\" shape-rendering=\"crispEdges\"/>"));
            }
            rapidr_value::objects::tabcontrol::Op::Arrow { points, color } => {
                let pts: Vec<String> = points.iter().map(|(px, py)| format!("{px},{py}")).collect();
                svg.push_str(&format!("<polygon points=\"{}\" fill=\"#{color:06x}\"/>", pts.join(" ")));
            }
            _ => {}
        }
    }
    svg.push_str("</svg>");
    overlay.set_inner_html(&svg);
}

/// Where the mouse is in `name`'s client area.
fn local(host: &web_sys::HtmlElement, e: &web_sys::MouseEvent) -> (i64, i64) {
    let r = host.get_bounding_client_rect();
    ((e.client_x() as f64 - r.left() - host.client_left() as f64).floor() as i64, (e.client_y() as f64 - r.top() - host.client_top() as f64).floor() as i64)
}

/// The container's listeners, once: a press on its bars (before its
/// components see it), the wheel.
fn listen(name: &str, host: &web_sys::HtmlElement) {
    if host.get_attribute("data-rr-scroll").is_some() {
        return;
    }
    let _ = host.set_attribute("data-rr-scroll", "1");
    listen_document();
    {
        let (name, el) = (name.to_uppercase(), host.clone());
        let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            if e.button() != 0 {
                return;
            }
            let (x, y) = local(&el, &e);
            let (w, h) = area(&name);
            if !scrollbars::with(&name, |s| s.on_bars(x, y, w, h)).unwrap_or(false) {
                return;
            }
            // (no OnMouseDown: the bars aren't the client area, as Windows')
            e.stop_immediate_propagation();
            e.prevent_default();
            let shift = scrollbars::with_mut(&name, |s| s.mouse_down(x, y, w, h)).unwrap_or_default();
            user_scrolled(&name, shift);
            CAPTURE.with(|c| *c.borrow_mut() = Some((name.clone(), x, y)));
            schedule_repeat(name.clone(), 400);
        });
        let opts = web_sys::AddEventListenerOptions::new();
        opts.set_capture(true);
        let _ = host.add_event_listener_with_callback_and_add_event_listener_options("mousedown", cb.as_ref().unchecked_ref(), &opts);
        cb.forget();
    }
    {
        let (name, el) = (name.to_uppercase(), host.clone());
        let cb = Closure::<dyn FnMut(web_sys::WheelEvent)>::new(move |e: web_sys::WheelEvent| {
            let (x, y) = local(&el, &e);
            let (w, h) = area(&name);
            if x < 0 || y < 0 || x >= w || y >= h {
                return;
            }
            let (dx, dy) = (e.delta_x(), e.delta_y());
            let horizontal = dx.abs() > dy.abs() || e.shift_key();
            let d = if dx.abs() > dy.abs() { dx } else { dy };
            if d == 0.0 {
                return;
            }
            let shift = scrollbars::with_mut(&name, |s| s.wheel(if d > 0.0 { 1 } else { -1 }, horizontal, w, h));
            if shift != (0, 0) {
                e.prevent_default();
                e.stop_propagation();
                user_scrolled(&name, shift);
            }
        });
        let opts = web_sys::AddEventListenerOptions::new();
        opts.set_passive(false);
        let _ = host.add_event_listener_with_callback_and_add_event_listener_options("wheel", cb.as_ref().unchecked_ref(), &opts);
        cb.forget();
    }
}

/// The bars held down: the part repeats (as Windows' do).
fn schedule_repeat(name: String, ms: i32) {
    let cb = Closure::once_into_js(move || {
        let Some((held, x, y)) = CAPTURE.with(|c| c.borrow().clone()) else { return };
        if held != name {
            return;
        }
        let (w, h) = area(&name);
        let shift = scrollbars::with_mut(&name, |s| s.repeat(x, y, w, h));
        user_scrolled(&name, shift);
        schedule_repeat(name, 50);
    });
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
    }
}

/// The page's drag and release, once: a held thumb follows the mouse.
fn listen_document() {
    if LISTENING.with(|l| l.replace(true)) {
        return;
    }
    let doc = document();
    let moved = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| {
        let Some((name, _, _)) = CAPTURE.with(|c| c.borrow().clone()) else { return };
        let Some(host) = client_el(&name) else { return };
        let (x, y) = local(&host, &e);
        CAPTURE.with(|c| *c.borrow_mut() = Some((name.clone(), x, y)));
        let (w, h) = area(&name);
        let shift = scrollbars::with_mut(&name, |s| s.mouse_drag(x, y, w, h));
        user_scrolled(&name, shift);
        e.prevent_default();
    });
    let _ = doc.add_event_listener_with_callback("mousemove", moved.as_ref().unchecked_ref());
    moved.forget();
    let up = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|_: web_sys::MouseEvent| {
        let Some((name, _, _)) = CAPTURE.with(|c| c.borrow_mut().take()) else { return };
        let (w, h) = area(&name);
        let shift = scrollbars::with_mut(&name, |s| s.mouse_up(w, h));
        user_scrolled(&name, shift);
    });
    let _ = doc.add_event_listener_with_callback("mouseup", up.as_ref().unchecked_ref());
    up.forget();
}
