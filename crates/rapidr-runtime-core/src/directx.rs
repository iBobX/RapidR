//! RapidQ's DirectX 2D objects on the desktop (docs/directx-plan.md): the
//! shared models (`rapidr_value::objects::directx`) draw, the kernel shows
//! a QDXSCREEN's last Flip (`rapidr_ui_kernel::components::dxscreen`);
//! this is what's left to the runtime — a form's screens set up when it's
//! first shown (OnInitialize, OnInitializeSurface), or a screen put on a
//! form already shown; FullScreen's window; a QDXTIMER's pace, ActiveOnly
//! and frame count. The web runtime does the same (`directx_web.rs`).

use std::time::{Duration, Instant};

use crate::object::{form_of, get_children_of, rp_comp_get, rp_comp_type, rp_fire_event};

/// The QDXSCREENs on `form`, in its containers too.
fn screens_of(form: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (child, type_name) in get_children_of(form) {
        match type_name.to_ascii_uppercase().as_str() {
            "RDXSCREEN" => out.push(child),
            // (another form is a window of its own)
            "RFORM" => {}
            _ => out.extend(screens_of(&child)),
        }
    }
    out
}

/// Form `form`'s window was made (it's shown the first time): each
/// QDXSCREEN on it not set up yet is — as DirectX was once the window
/// existed — and told: OnInitialize, then OnInitializeSurface.
pub fn form_built(form: &str) {
    for screen in screens_of(form) {
        if rapidr_value::objects::dxscreen_initialize(&screen, &|i, p| rp_comp_get(i, p)) {
            rp_fire_event(&screen, "oninitialize");
            rp_fire_event(&screen, "oninitializesurface");
        }
    }
}

/// QDXSCREEN `name` was put on a form (its Parent): on a form whose window
/// exists already it's set up once the program's code returns (its CREATE
/// block has given it its size and OnInitialize by then).
pub fn parented(name: &str) {
    let Some(form) = form_of(name) else { return };
    if crate::ui::form_window_exists(&form) {
        crate::object::rp_defer_job(Box::new(move || form_built(&form)));
    }
}

/// Whether form `form` shows full screen: one of its QDXSCREENs has
/// FullScreen (manual: set before ShowModal; there's no way back).
pub fn form_fullscreen(form: &str) -> bool {
    screens_of(form).iter().any(|s| rp_comp_get(s, "fullscreen").to_bool())
}

/// Whether `name` is a QDXTIMER.
pub fn is_dx_timer(name: &str) -> bool {
    rp_comp_type(name) == "RDXTIMER"
}

/// A QDXTIMER's time between OnTimers (Interval 0: a screen refresh); a
/// QDXJOYSTICK's between its looks for events; `None` for other timers.
pub fn timer_interval(name: &str) -> Option<Duration> {
    if rp_comp_type(name) == "RDXJOYSTICK" {
        return Some(Duration::from_millis(rapidr_value::objects::joystick::LOOK_MS));
    }
    is_dx_timer(name).then(|| Duration::from_millis(rapidr_value::objects::directx::timer_interval_ms(rp_comp_get(name, "interval").to_i64())))
}

/// A QDXJOYSTICK's look: when the program has a handler for one of its
/// events, the joystick read and what changed fired (OnButtonUp /
/// OnButtonDown with the button's number, OnMove). No OnTimer.
fn joystick_look(name: &str) -> bool {
    use rapidr_value::objects::joystick::EVENTS;
    if EVENTS.iter().any(|e| crate::object::rp_has_handler(name, e)) {
        for (event, args) in rapidr_value::objects::dxjoystick_look(name) {
            crate::object::rp_fire_event_args(name, event, &args);
        }
    }
    false
}

/// Timer `name` is due: whether its OnTimer fires — a QDXTIMER with
/// ActiveOnly (the default) only while the program is the active
/// application — and a QDXTIMER's frame counted (FrameRate).
pub fn timer_fired(name: &str) -> bool {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    if rp_comp_type(name) == "RDXJOYSTICK" {
        return joystick_look(name);
    }
    if !is_dx_timer(name) {
        return true;
    }
    let active_only = match rp_comp_get(name, "activeonly") {
        crate::value::Value::Null => true,
        v => v.to_bool(),
    };
    if !rapidr_value::objects::directx::DxTimer::fires(active_only, crate::ui::app_active()) {
        return false;
    }
    let ms = START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0;
    rapidr_value::objects::dxtimer_fired(name, ms);
    true
}
