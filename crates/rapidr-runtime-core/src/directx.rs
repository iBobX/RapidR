//! RapidQ's DirectX 2D objects on the desktop (docs/directx-plan.md): the
//! shared models (`rapidr_value::objects::directx`) draw, the kernel shows
//! a QDXSCREEN's last Flip (`rapidr_ui_kernel::components::dxscreen`);
//! this is what's left to the runtime — a form's screens set up when it's
//! first shown (OnInitialize, OnInitializeSurface) and a QDXTIMER's pace
//! and frame count. The web runtime does the same (`directx_web.rs`).

use std::time::{Duration, Instant};

use crate::object::{get_children_of, rp_comp_get, rp_comp_type, rp_fire_event};

/// Form `form`'s window was made (it's shown the first time): each
/// QDXSCREEN on it, in its containers too, is set up — as DirectX was once
/// the window existed — and told: OnInitialize, then OnInitializeSurface.
pub fn form_built(form: &str) {
    for (child, type_name) in get_children_of(form) {
        match type_name.to_ascii_uppercase().as_str() {
            "RDXSCREEN" => {
                if rapidr_value::objects::dxscreen_initialize(&child, &|i, p| rp_comp_get(i, p)) {
                    rp_fire_event(&child, "oninitialize");
                    rp_fire_event(&child, "oninitializesurface");
                }
            }
            // (another form is a window of its own)
            "RFORM" => {}
            _ => form_built(&child),
        }
    }
}

/// Whether `name` is a QDXTIMER.
pub fn is_dx_timer(name: &str) -> bool {
    rp_comp_type(name) == "RDXTIMER"
}

/// A QDXTIMER's time between OnTimers (Interval 0: a screen refresh);
/// `None` for other timers.
pub fn timer_interval(name: &str) -> Option<Duration> {
    is_dx_timer(name).then(|| Duration::from_millis(rapidr_value::objects::directx::timer_interval_ms(rp_comp_get(name, "interval").to_i64())))
}

/// Timer `name` fires: a QDXTIMER counts the frame (FrameRate).
pub fn timer_fired(name: &str) {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    if is_dx_timer(name) {
        let ms = START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0;
        rapidr_value::objects::dxtimer_fired(name, ms);
    }
}
