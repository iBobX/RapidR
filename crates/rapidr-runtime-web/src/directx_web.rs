//! RapidQ's DirectX 2D objects in the browser (docs/directx-plan.md), as
//! the desktop's `rapidr-runtime-core/src/directx.rs`: the shared models
//! (`rapidr_value::objects::directx`) draw, a QDXSCREEN is a `<canvas>`
//! showing its last Flip (`gui_web::render_dxscreen`); this sets a form's
//! screens up when it's first shown (OnInitialize, OnInitializeSurface) and
//! paces and counts a QDXTIMER.

use crate::object_web::{get_children_of, rp_comp_get_stored, rp_comp_type, rp_fire_event};

/// Form `form` shows for the first time: each QDXSCREEN on it, in its
/// containers too, is set up and told — OnInitialize, then
/// OnInitializeSurface (once; showing it again does nothing).
pub fn form_shown(form: &str) {
    for (child, type_name) in get_children_of(form) {
        match type_name.to_ascii_uppercase().as_str() {
            "RDXSCREEN" => {
                if rapidr_value::objects::dxscreen_initialize(&child, &|i, p| rp_comp_get_stored(i, p)) {
                    crate::gui_web::render_dxscreen(&child);
                    rp_fire_event(&child, "oninitialize");
                    rp_fire_event(&child, "oninitializesurface");
                }
            }
            "RFORM" => {}
            _ => form_shown(&child),
        }
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

/// Timer `name` fires: a QDXTIMER counts the frame (FrameRate).
pub fn timer_fired(name: &str) {
    if is_dx_timer(name) {
        rapidr_value::objects::dxtimer_fired(name, js_sys::Date::now());
    }
}
