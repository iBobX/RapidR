//! RapidQ's DirectX 2D objects in the browser (docs/directx-plan.md), as
//! the desktop's `rapidr-runtime-core/src/directx.rs`: the shared models
//! (`rapidr_value::objects::directx`) draw, a QDXSCREEN is a `<canvas>`
//! showing its last Flip (`gui_web::render_dxscreen`); this sets a form's
//! screens up when it's first shown (OnInitialize, OnInitializeSurface) or
//! when one is put on a form already shown, gives a FullScreen form the
//! page, paces and counts a QDXTIMER (ActiveOnly: while the page
//! shows), and plays a QDXSOUND's WAV through Web Audio.

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

/// Whether form `form` shows full screen (a QDXSCREEN on it has
/// FullScreen; the kernel host's question, as the desktop's).
pub fn fullscreen(form: &str) -> bool {
    screens_of(form).iter().any(|s| rp_comp_get_stored(s, "fullscreen").to_bool())
}

/// QDXSCREEN `name` was put on a form (its Parent): on a form the program
/// shows already it's set up once the program's code returns (its CREATE
/// block has given it its size and OnInitialize by then).
pub fn parented(name: &str) {
    let Some(form) = form_of(name) else { return };
    // (the kernel host: its window made, as the desktop asks)
    #[cfg(feature = "kernel")]
    let shown = if crate::kernel_web::on() { crate::kernel_web::form_window_exists(&form) } else { rp_comp_get_stored(&form, SHOWN_BY_PROGRAM).to_bool() };
    #[cfg(not(feature = "kernel"))]
    let shown = rp_comp_get_stored(&form, SHOWN_BY_PROGRAM).to_bool();
    if !shown {
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
/// (a QDXTIMER's 0 is a screen refresh; a QDXJOYSTICK's looks for events).
pub fn timer_interval(name: &str, ms: i64) -> i64 {
    // (the I/O lane's: a QCOMPORT's looks for OnRxChar)
    if rapidr_value::objects::rqlib::is_comport(name) {
        return rapidr_value::objects::rqlib::LOOK_MS as i64;
    }
    if let Some((interval, _)) = rapidr_value::objects::rqlib::media_timer(name) {
        return if interval > 0 { interval } else { 1000 };
    }
    if rp_comp_type(name) == "RDXJOYSTICK" {
        return rapidr_value::objects::joystick::LOOK_MS as i64;
    }
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
    // A QDXJOYSTICK's look: what changed fired (OnButtonUp / OnButtonDown
    // with the button's number, OnMove); no OnTimer.
    if rapidr_value::objects::rqlib::is_comport(name) || rapidr_value::objects::rqlib::media_timer(name).is_some() {
        crate::io_web::look(name);
        return false;
    }
    if rp_comp_type(name) == "RDXJOYSTICK" {
        // (only when the program has a handler for one of its events, as
        // the desktop's: a look reads the gamepads — the tests' script's
        // next step)
        if rapidr_value::objects::joystick::EVENTS.iter().any(|e| crate::object_web::rp_has_handler(name, e)) {
            for (event, args) in rapidr_value::objects::dxjoystick_look(name) {
                crate::object_web::rp_fire_event_args(name, event, &args);
            }
        }
        return false;
    }
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

// ---------------------------------------------------------- QDXSOUND --
//
// The model (rapidr_value::objects::directx::DxSound) keeps Playing and
// Position by the page's clock; Web Audio plays what it asks for: the WAV's
// frames as a stereo buffer with Pan's gains, from a frame, at a playback
// rate (Frequency over the file's rate), looped or once, through a gain
// node (Volume, changed while it plays).

thread_local! {
    static PLAYING: std::cell::RefCell<std::collections::HashMap<String, (web_sys::AudioBufferSourceNode, web_sys::GainNode)>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Gives QDXSOUND the page's clock and Web Audio (once).
pub fn install_sound_device() {
    rapidr_value::objects::directx::set_clock(js_sys::Date::now);
    rapidr_value::objects::directx::set_sound_device(rapidr_value::objects::directx::SoundDevice { play: sound_play, volume: sound_volume, stop: sound_stop });
}

fn sound_play(p: &rapidr_value::objects::directx::SoundPlay) {
    sound_stop(&p.id);
    let Some(ctx) = crate::builtins::audio_context() else { return };
    let wav = &p.wav;
    let frames = wav.frames();
    if frames == 0 {
        return;
    }
    let Ok(buffer) = ctx.create_buffer(2, frames as u32, wav.rate as f32) else { return };
    let stereo = wav.stereo(p.pan);
    let (left, right): (Vec<f32>, Vec<f32>) = stereo.as_chunks::<2>().0.iter().map(|f| (f[0], f[1])).unzip();
    if buffer.copy_to_channel(&left, 0).is_err() || buffer.copy_to_channel(&right, 1).is_err() {
        return;
    }
    let (Ok(source), Ok(gain)) = (ctx.create_buffer_source(), ctx.create_gain()) else { return };
    source.set_buffer(Some(&buffer));
    source.set_loop(p.looped);
    source.playback_rate().set_value(p.speed as f32);
    gain.gain().set_value(p.gain);
    if source.connect_with_audio_node(&gain).is_err() || gain.connect_with_audio_node(&ctx.destination()).is_err() {
        return;
    }
    let _ = ctx.resume();
    if source.start_with_when_and_grain_offset(0.0, p.from as f64 / f64::from(wav.rate.max(1))).is_ok() {
        PLAYING.with(|m| m.borrow_mut().insert(p.id.clone(), (source, gain)));
    }
}

fn sound_volume(id: &str, gain: f32) {
    PLAYING.with(|m| {
        if let Some((_, g)) = m.borrow().get(id) {
            g.gain().set_value(gain);
        }
    });
}

fn sound_stop(id: &str) {
    PLAYING.with(|m| {
        if let Some((source, _)) = m.borrow_mut().remove(id) {
            #[allow(deprecated)]
            let _ = source.stop();
        }
    });
}

// ------------------------------------------------------- QDXJOYSTICK --
//
// The page's gamepads (the Gamepad API's `navigator.getGamepads()`, its
// "standard" layout) for rapidr_value::objects::joystick; the tests' script
// when the page has a `RAPIDR_TEST_JOYSTICK` string (read at each look, so a
// test can set it after the program started).

struct WebPads {
    script: Option<(String, rapidr_value::objects::joystick::Script)>,
}

impl rapidr_value::objects::joystick::Source for WebPads {
    fn pads(&mut self) -> Vec<rapidr_value::objects::joystick::Pad> {
        use rapidr_value::objects::joystick::{Pad, Script, Standard};
        let window = web_sys::window();
        let test = window.as_ref().and_then(|w| js_sys::Reflect::get(w, &JsValue::from_str("RAPIDR_TEST_JOYSTICK")).ok()).and_then(|v| v.as_string());
        if let Some(text) = test {
            if self.script.as_ref().is_none_or(|(t, _)| *t != text) {
                match Script::parse(&text) {
                    Ok(s) => self.script = Some((text, s)),
                    Err(e) => {
                        web_sys::console::warn_1(&JsValue::from_str(&e));
                        self.script = None;
                    }
                }
            }
            return self.script.as_mut().map(|(_, s)| rapidr_value::objects::joystick::Source::pads(s)).unwrap_or_default();
        }
        let Some(list) = window.and_then(|w| w.navigator().get_gamepads().ok()) else { return Vec::new() };
        list.iter()
            .filter_map(|g| g.dyn_into::<web_sys::Gamepad>().ok())
            .filter(|g| g.connected())
            .map(|g| {
                let mut s = Standard::new(&g.id());
                for (i, a) in g.axes().iter().take(4).enumerate() {
                    s.axes[i] = a.as_f64().unwrap_or(0.0) as f32;
                }
                for (i, b) in g.buttons().iter().take(17).enumerate() {
                    if let Ok(b) = b.dyn_into::<web_sys::GamepadButton>() {
                        s.buttons[i] = (b.pressed(), b.value() as f32);
                    }
                }
                Pad::from_standard(&s)
            })
            .collect()
    }
}

/// QDXJOYSTICK's gamepads, once: the first one made.
pub fn install_joystick_source() {
    if !rapidr_value::objects::joystick::has_source() {
        rapidr_value::objects::joystick::set_source(Box::new(WebPads { script: None }));
    }
}

