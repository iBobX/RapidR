//! The media objects' devices in the browser (rapidr_value::objects::media;
//! docs/io-media-plan.md §4–§5), as runtime-core's `media.rs` on the
//! desktop: QMIDI's song sent to the page's first MIDI output (Web MIDI —
//! the browser asks the user once; none, refused, or a browser without Web
//! MIDI: RapidR's built-in synthesizer, `objects::synth`, rendered in wasm
//! for Web Audio as it plays), QWAVE played
//! through QDXSOUND's Web Audio device and recorded from the microphone
//! (getUserMedia, the browser asking). The tests' page sets
//! `RAPIDR_TEST_MIDI` (no MIDI) and `RAPIDR_TEST_WAVE_IN` (`tone:HZ`, a
//! scripted input; empty: none) — never a real device.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rapidr_value::objects::media::{self, MidiDevice, WaveInput};
use rapidr_value::objects::midifile::Song;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

fn page_var(name: &str) -> Option<JsValue> {
    web_sys::window().and_then(|w| js_sys::Reflect::get(&w, &name.into()).ok()).filter(|v| !v.is_undefined())
}

/// Installs the media devices, once (the first QMIDI or QWAVE made).
pub fn install() {
    thread_local! {
        static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    crate::directx_web::install_sound_device();
    // (the tests' page variables are read when a device is first used: the
    // test sets them once the program runs)
    media::set_midi_device(MidiDevice { play: midi_play, stop: midi_stop, volume: midi_volume });
    media::set_wave_input(WaveInput { start: mic_start, take: mic_take, stop: mic_stop });
}

// ----------------------------------------------------------------- MIDI --

thread_local! {
    /// The page's MIDIAccess, once the user allowed it.
    static ACCESS: RefCell<Option<JsValue>> = const { RefCell::new(None) };
    /// The output each QMIDI's song was sent to.
    static SENT: RefCell<HashMap<String, JsValue>> = RefCell::new(HashMap::new());
}

async fn first_output() -> Option<JsValue> {
    let access = match ACCESS.with(|a| a.borrow().clone()) {
        Some(a) => a,
        None => {
            let nav = web_sys::window()?.navigator();
            let request: js_sys::Function = js_sys::Reflect::get(&nav, &"requestMIDIAccess".into()).ok()?.dyn_into().ok()?;
            let a = JsFuture::from(js_sys::Promise::resolve(&request.call0(&nav).ok()?)).await.ok()?;
            ACCESS.with(|x| *x.borrow_mut() = Some(a.clone()));
            a
        }
    };
    let outputs = js_sys::Reflect::get(&access, &"outputs".into()).ok()?;
    let values: js_sys::Function = js_sys::Reflect::get(&outputs, &"values".into()).ok()?.dyn_into().ok()?;
    let iter = values.call0(&outputs).ok()?;
    let next: js_sys::Function = js_sys::Reflect::get(&iter, &"next".into()).ok()?.dyn_into().ok()?;
    let first = next.call0(&iter).ok()?;
    js_sys::Reflect::get(&first, &"value".into()).ok().filter(|v| !v.is_undefined())
}

fn now_ms() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map_or(0.0, |p| p.now())
}

fn midi_play(id: &str, song: Rc<Song>, from_us: u64, gain: f32) {
    midi_stop(id);
    if page_var("RAPIDR_TEST_MIDI").is_some() {
        return;
    }
    let id = id.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let Some(out) = first_output().await else {
            synth_play(&id, &song, from_us, gain);
            return;
        };
        let Ok(send) = js_sys::Reflect::get(&out, &"send".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) else { return };
        let t0 = now_ms();
        for e in song.events.iter().filter(|e| e.at_us >= from_us) {
            let mut m = e.bytes.clone();
            if m.len() == 3 && m[0] & 0xF0 == 0x90 && m[2] > 0 {
                m[2] = ((f32::from(m[2]) * gain).round() as u8).clamp(1, 127);
            }
            let at = t0 + (e.at_us - from_us) as f64 / 1000.0;
            let _ = send.call2(&out, &js_sys::Uint8Array::from(m.as_slice()), &at.into());
        }
        SENT.with(|s| s.borrow_mut().insert(id, out));
    });
}

fn midi_stop(id: &str) {
    synth_stop(id);
    let Some(out) = SENT.with(|s| s.borrow_mut().remove(id)) else { return };
    // (what was sent ahead dropped — MIDIOutput.clear, where there is one —
    // and every note off)
    if let Ok(clear) = js_sys::Reflect::get(&out, &"clear".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
        let _ = clear.call0(&out);
    }
    if let Ok(send) = js_sys::Reflect::get(&out, &"send".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
        for ch in 0..16u8 {
            let _ = send.call1(&out, &js_sys::Uint8Array::from(&[0xB0 | ch, 123, 0][..]));
        }
    }
}

// ------------------------------------------------- built-in synthesizer --

/// A song on the built-in synthesizer: its Web Audio node (a script
/// processor whose callback renders the next block) and that callback.
struct Synth {
    node: JsValue,
    _render: Closure<dyn FnMut(JsValue)>,
    /// Volume's gain, read by the callback.
    gain: Rc<std::cell::Cell<f32>>,
}

thread_local! {
    static SYNTHS: RefCell<HashMap<String, Synth>> = RefCell::new(HashMap::new());
}

/// QMIDI `id`'s song from `from_us` on the built-in synthesizer, at the
/// page's audio rate.
fn synth_play(id: &str, song: &Song, from_us: u64, gain: f32) {
    synth_stop(id);
    let Some(ctx) = crate::builtins::audio_context() else { return };
    let ctx_js: JsValue = ctx.clone().into();
    let Ok(create) = js_sys::Reflect::get(&ctx_js, &"createScriptProcessor".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) else { return };
    let Ok(node) = create.call3(&ctx_js, &4096.into(), &0.into(), &2.into()) else { return };
    let events = song.events.iter().map(|e| (e.at_us, e.bytes.clone())).collect();
    let mut player = rapidr_value::objects::synth::Player::new(events, ctx.sample_rate() as u32, from_us, gain.clamp(0.0, 1.0));
    let mut block: Vec<f32> = Vec::new();
    let (mut left, mut right): (Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new());
    let shared = Rc::new(std::cell::Cell::new(gain));
    let (volume, mut applied) = (shared.clone(), gain);
    let render = Closure::<dyn FnMut(JsValue)>::new(move |ev: JsValue| {
        if volume.get() != applied {
            applied = volume.get();
            player.set_gain(applied);
        }
        let Some(out) = js_sys::Reflect::get(&ev, &"outputBuffer".into()).ok().and_then(|b| b.dyn_into::<web_sys::AudioBuffer>().ok()) else { return };
        let n = out.length() as usize;
        block.resize(n * 2, 0.0);
        player.render(&mut block);
        left.clear();
        right.clear();
        for f in block.as_chunks::<2>().0 {
            left.push(f[0]);
            right.push(f[1]);
        }
        let _ = out.copy_to_channel(&left, 0);
        let _ = out.copy_to_channel(&right, 1);
    });
    let _ = js_sys::Reflect::set(&node, &"onaudioprocess".into(), render.as_ref().unchecked_ref());
    if let Ok(connect) = js_sys::Reflect::get(&node, &"connect".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
        let _ = connect.call1(&node, &ctx.destination());
    }
    let _ = ctx.resume();
    SYNTHS.with(|s| s.borrow_mut().insert(id.to_string(), Synth { node, _render: render, gain: shared }));
}

/// QMIDI's Volume while the built-in synthesizer plays its song (a MIDI
/// output's notes already sent keep theirs).
fn midi_volume(id: &str, gain: f32) {
    SYNTHS.with(|s| {
        if let Some(s) = s.borrow().get(id) {
            s.gain.set(gain.clamp(0.0, 1.0));
        }
    });
}

fn synth_stop(id: &str) {
    let Some(s) = SYNTHS.with(|s| s.borrow_mut().remove(id)) else { return };
    let _ = js_sys::Reflect::set(&s.node, &"onaudioprocess".into(), &JsValue::NULL);
    if let Ok(disconnect) = js_sys::Reflect::get(&s.node, &"disconnect".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
        let _ = disconnect.call0(&s.node);
    }
}

// ----------------------------------------------------------- microphone --

struct Capture {
    bytes: Rc<RefCell<Vec<u8>>>,
    stop: Rc<std::cell::Cell<bool>>,
}

thread_local! {
    static CAPTURES: RefCell<HashMap<String, Capture>> = RefCell::new(HashMap::new());
}

/// Records from the microphone (the browser asks the user), converted to
/// the wave's format: the input's first channel, resampled by nearest
/// frame. Starts at once; the sound comes once the user allowed it.
fn mic_start(id: &str, rate: u32, bits: u16, channels: u16) -> bool {
    // (a test's: its scripted input from now on, never the microphone)
    if let Some(script) = page_var("RAPIDR_TEST_WAVE_IN").and_then(|v| v.as_string()) {
        let Some(hz) = media::tone_script(&script) else { return false };
        let input = media::tone_input(hz);
        media::set_wave_input(input);
        return (input.start)(id, rate, bits, channels);
    }
    let Some(nav) = web_sys::window().map(|w| w.navigator()) else { return false };
    let Ok(devices) = js_sys::Reflect::get(&nav, &"mediaDevices".into()) else { return false };
    if devices.is_undefined() {
        return false;
    }
    let bytes = Rc::new(RefCell::new(Vec::new()));
    let stop = Rc::new(std::cell::Cell::new(false));
    CAPTURES.with(|c| c.borrow_mut().insert(id.to_string(), Capture { bytes: bytes.clone(), stop: stop.clone() }));
    wasm_bindgen_futures::spawn_local(async move {
        let constraints = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&constraints, &"audio".into(), &true.into());
        let Ok(get) = js_sys::Reflect::get(&devices, &"getUserMedia".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) else { return };
        let Ok(p) = get.call1(&devices, &constraints) else { return };
        let Ok(stream) = JsFuture::from(js_sys::Promise::resolve(&p)).await else { return };
        let Ok(ctx) = web_sys::AudioContext::new() else { return };
        let ctx_js: JsValue = ctx.clone().into();
        let call = |name: &str, args: &[JsValue]| -> Option<JsValue> {
            let f: js_sys::Function = js_sys::Reflect::get(&ctx_js, &name.into()).ok()?.dyn_into().ok()?;
            f.apply(&ctx_js, &args.iter().cloned().collect::<js_sys::Array>()).ok()
        };
        let Some(source) = call("createMediaStreamSource", std::slice::from_ref(&stream)) else { return };
        let Some(node) = call("createScriptProcessor", &[4096.into(), 1.into(), 1.into()]) else { return };
        let dev_rate = ctx.sample_rate() as f64;
        let step = dev_rate / f64::from(rate.max(1));
        let mut phase = 0.0f64;
        let (into, stopped) = (bytes.clone(), stop.clone());
        let onaudio = Closure::<dyn FnMut(JsValue)>::new(move |ev: JsValue| {
            if stopped.get() {
                return;
            }
            let Ok(buf) = js_sys::Reflect::get(&ev, &"inputBuffer".into()) else { return };
            let Ok(get_channel) = js_sys::Reflect::get(&buf, &"getChannelData".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) else { return };
            let Ok(data) = get_channel.call1(&buf, &0.into()) else { return };
            let data = js_sys::Float32Array::new(&data).to_vec();
            let mut out = into.borrow_mut();
            while (phase as usize) < data.len() {
                let s = data[phase as usize].clamp(-1.0, 1.0);
                for _ in 0..channels {
                    if bits == 8 {
                        out.push(((s * 127.0) + 128.0).round() as u8);
                    } else {
                        out.extend(((s * 32767.0).round() as i16).to_le_bytes());
                    }
                }
                phase += step;
            }
            phase -= data.len() as f64;
        });
        let _ = js_sys::Reflect::set(&node, &"onaudioprocess".into(), onaudio.as_ref().unchecked_ref());
        onaudio.forget();
        let connect = |from: &JsValue, to: &JsValue| {
            if let Ok(f) = js_sys::Reflect::get(from, &"connect".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
                let _ = f.call1(from, to);
            }
        };
        connect(&source, &node);
        if let Ok(dest) = js_sys::Reflect::get(&ctx_js, &"destination".into()) {
            connect(&node, &dest);
        }
        // (the microphone given back when recording stops)
        let tracks = js_sys::Reflect::get(&stream, &"getTracks".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok()).and_then(|f| f.call0(&stream).ok());
        STREAMS.with(|s| s.borrow_mut().push((stop, tracks.unwrap_or(JsValue::NULL), ctx)));
    });
    true
}

/// A recording's stop flag, its microphone's tracks, its audio context.
type Stream = (Rc<std::cell::Cell<bool>>, JsValue, web_sys::AudioContext);

thread_local! {
    static STREAMS: RefCell<Vec<Stream>> = const { RefCell::new(Vec::new()) };
}

fn mic_take(id: &str) -> Vec<u8> {
    CAPTURES.with(|c| c.borrow().get(id).map(|c| std::mem::take(&mut *c.bytes.borrow_mut()))).unwrap_or_default()
}

fn mic_stop(id: &str) {
    if let Some(c) = CAPTURES.with(|c| c.borrow_mut().remove(id)) {
        c.stop.set(true);
    }
    // (the stopped recordings' microphones and contexts let go)
    STREAMS.with(|s| {
        s.borrow_mut().retain(|(stopped, tracks, ctx)| {
            if !stopped.get() {
                return true;
            }
            for t in js_sys::Array::from(tracks).iter() {
                if let Ok(f) = js_sys::Reflect::get(&t, &"stop".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
                    let _ = f.call0(&t);
                }
            }
            let _ = ctx.close();
            false
        })
    });
}
