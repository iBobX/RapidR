//! RapidQ's DirectX 2D objects in the browser (docs/directx-plan.md), as
//! the desktop's `rapidr-runtime-core/src/directx.rs`: the shared models
//! (`rapidr_value::objects::directx`) draw, a QDXSCREEN is a `<canvas>`
//! showing its last Flip (`gui_web::render_dxscreen`); this sets a form's
//! screens up when it's first shown (OnInitialize, OnInitializeSurface) or
//! when one is put on a form already shown, gives a FullScreen form the
//! page, and paces and counts a QDXTIMER (ActiveOnly: while the page
//! shows).

use wasm_bindgen::prelude::*;

use crate::object_web::{form_of, get_children_of, rp_comp_get_stored, rp_comp_type, rp_fire_event, SHOWN_BY_PROGRAM};

/// The QDXSCREENs on `form`, in its containers too.
fn screens_of(form: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (child, type_name) in get_children_of(form) {
        match type_name.to_ascii_uppercase().as_str() {
            "RDXSCREEN" => out.push(child),
            "RFORM" => {}
            _ => out.extend(screens_of(&child)),
        }
    }
    out
}

/// Form `form` shows for the first time: a FullScreen form takes the page,
/// and each QDXSCREEN on it not set up yet is, and told — OnInitialize,
/// then OnInitializeSurface (once; showing it again does nothing).
pub fn form_shown(form: &str) {
    let screens = screens_of(form);
    if screens.iter().any(|s| rp_comp_get_stored(s, "fullscreen").to_bool()) {
        crate::gui_web::form_fullscreen(form);
    }
    for screen in screens {
        if rapidr_value::objects::dxscreen_initialize(&screen, &|i, p| rp_comp_get_stored(i, p)) {
            crate::gui_web::render_dxscreen(&screen);
            rp_fire_event(&screen, "oninitialize");
            rp_fire_event(&screen, "oninitializesurface");
        }
    }
}

/// QDXSCREEN `name` was put on a form (its Parent): on a form the program
/// shows already it's set up once the program's code returns (its CREATE
/// block has given it its size and OnInitialize by then).
pub fn parented(name: &str) {
    let Some(form) = form_of(name) else { return };
    if !rp_comp_get_stored(&form, SHOWN_BY_PROGRAM).to_bool() {
        return;
    }
    let later = Closure::once_into_js(move || form_shown(&form));
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), 0);
    }
}

/// Whether `name` is a QDXTIMER.
pub fn is_dx_timer(name: &str) -> bool {
    rp_comp_type(name) == "RDXTIMER"
}

/// The milliseconds between timer `name`'s OnTimers for its Interval `ms`
/// (a QDXTIMER's 0 is a screen refresh).
pub fn timer_interval(name: &str, ms: i64) -> i64 {
    if is_dx_timer(name) {
        rapidr_value::objects::directx::timer_interval_ms(ms) as i64
    } else {
        ms
    }
}

/// Timer `name` is due: whether its OnTimer fires — a QDXTIMER with
/// ActiveOnly (the default) only while the page shows (a browser can't
/// tell more of "the active application") — and a QDXTIMER's frame
/// counted (FrameRate).
pub fn timer_fired(name: &str) -> bool {
    if !is_dx_timer(name) {
        return true;
    }
    let active_only = match rp_comp_get_stored(name, "activeonly") {
        rapidr_value::Value::Null => true,
        v => v.to_bool(),
    };
    let shown = web_sys::window().and_then(|w| w.document()).is_none_or(|d| !d.hidden());
    if !rapidr_value::objects::directx::DxTimer::fires(active_only, shown) {
        return false;
    }
    rapidr_value::objects::dxtimer_fired(name, js_sys::Date::now());
    true
}
