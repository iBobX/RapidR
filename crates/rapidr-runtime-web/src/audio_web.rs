//! RAudioOut in the browser: the model is `rapidr_value::objects::
//! audioout` (the desktop's too); this is its sound device and its
//! OnNeedData, as runtime-core's audio_out.rs on the desktop.
//!
//! - **The device.** A Web Audio script processor on the page's audio
//!   context (builtins.rs) pulls each block from the stream's queue,
//!   converted to the context's rate and to stereo (`Resampled`: linear
//!   interpolation). Its callback runs between the page's tasks and never
//!   waits for the program. (An AudioWorklet would play on the audio
//!   thread, but it needs a module file of its own and, to share the queue,
//!   a cross-origin-isolated page — which RapidR's pages are not: the
//!   script processor needs neither.)
//! - **A click or a key first.** Browsers start sound only after the person
//!   clicked or pressed a key on the page: until then Error says so (call
//!   Start from a button's OnClick); it plays — and Error clears — as soon
//!   as the page may.
//! - **Under the tests** (the test hooks' page) a clocked sink takes the
//!   samples in real time and keeps what it heard (`__Heard`); no audio
//!   context at all. Without Web Audio the same sink plays silently.
//! - **OnNeedData** is asked every 10 ms while a stream plays (a page
//!   interval), between the program's turns.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use rapidr_value::objects::audioout::{self, Consumer, Device, Resampled};
use rapidr_value::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// What Error says until the page may play sound.
pub const WAITING_FOR_CLICK: &str = "The browser plays sound only after a click or a key on the page: call Start from a button's OnClick (it plays as soon as the page may)";

/// How often the page asks for OnNeedData and feeds the sinks, in ms.
const TICK_MS: i32 = 10;

/// A stream playing on the page's audio context.
struct Player {
    node: JsValue,
    _render: Closure<dyn FnMut(JsValue)>,
    /// The stream stopped and faded out: the node can go.
    done: Rc<Cell<bool>>,
}

/// The clocked sink (tests; no Web Audio).
struct Sink {
    consumer: Consumer,
    began: f64,
    taken: u64,
}

thread_local! {
    static PLAYERS: RefCell<HashMap<String, Player>> = RefCell::new(HashMap::new());
    static SINKS: RefCell<HashMap<String, Sink>> = RefCell::new(HashMap::new());
    static TICK: Cell<Option<i32>> = const { Cell::new(None) };
}

fn now_ms() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map_or_else(js_sys::Date::now, |p| p.now())
}

fn page_var(name: &str) -> bool {
    web_sys::window().and_then(|w| js_sys::Reflect::get(&w, &name.into()).ok()).is_some_and(|v| !v.is_undefined())
}

/// Whether the page runs under the tests (no sound card, ever).
fn testing() -> bool {
    ["RAPIDR_CAPTURE", "RAPIDR_TEST_EVENTS", "RAPIDR_TEST_SOUND"].iter().any(|v| rapidr_ui_app::testhooks::var(v).is_some()) || page_var("RAPIDR_TEST_SOUND") || page_var("RAPIDR_TEST_MIDI")
}

/// Gives RAudioOut its device (once: the first stream made).
pub fn install() {
    thread_local! {
        static DONE: Cell<bool> = const { Cell::new(false) };
    }
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    audioout::set_device(Device { start, stop });
}

fn start(id: &str, consumer: Consumer) -> Result<(), String> {
    let key = id.to_lowercase();
    stop(&key);
    ensure_tick();
    if testing() {
        consumer.shared().record();
        SINKS.with(|s| s.borrow_mut().insert(key, Sink { consumer, began: now_ms(), taken: 0 }));
        return Ok(());
    }
    let Some(ctx) = crate::builtins::audio_context() else {
        SINKS.with(|s| s.borrow_mut().insert(key, Sink { consumer, began: now_ms(), taken: 0 }));
        return Err("This browser has no Web Audio: the stream plays silently (its timing goes on)".into());
    };
    let ctx_js: JsValue = ctx.clone().into();
    let node = js_sys::Reflect::get(&ctx_js, &"createScriptProcessor".into())
        .and_then(|f| f.dyn_into::<js_sys::Function>())
        .and_then(|create| create.call3(&ctx_js, &1024.into(), &0.into(), &2.into()));
    let Ok(node) = node else {
        SINKS.with(|s| s.borrow_mut().insert(key, Sink { consumer, began: now_ms(), taken: 0 }));
        return Err("This browser can't play a stream: it plays silently".into());
    };
    let mut player = Resampled::new(consumer, ctx.sample_rate() as u32);
    let done = Rc::new(Cell::new(false));
    let finished = done.clone();
    let (mut block, mut left, mut right): (Vec<f32>, Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new(), Vec::new());
    let render = Closure::<dyn FnMut(JsValue)>::new(move |ev: JsValue| {
        let Some(out) = js_sys::Reflect::get(&ev, &"outputBuffer".into()).ok().and_then(|b| b.dyn_into::<web_sys::AudioBuffer>().ok()) else { return };
        let n = out.length() as usize;
        block.resize(n * 2, 0.0);
        if !player.render(&mut block, 2) {
            finished.set(true);
        }
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
    PLAYERS.with(|p| p.borrow_mut().insert(key, Player { node, _render: render, done }));
    if running() {
        Ok(())
    } else {
        resume_on_gesture();
        Err(WAITING_FOR_CLICK.into())
    }
}

/// The page's next click or key (anywhere: a stream started before one)
/// lets the audio context run: it's resumed inside that gesture, as
/// browsers require.
fn resume_on_gesture() {
    thread_local! {
        static LISTENING: Cell<bool> = const { Cell::new(false) };
    }
    if LISTENING.with(|l| l.replace(true)) {
        return;
    }
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let resume = Closure::<dyn FnMut()>::new(|| {
        if let Some(ctx) = crate::builtins::audio_context() {
            let _ = ctx.resume();
        }
    });
    for event in ["pointerdown", "keydown", "touchend"] {
        let _ = doc.add_event_listener_with_callback_and_bool(event, resume.as_ref().unchecked_ref(), true);
    }
    resume.forget();
}

/// Whether the page's audio context plays (a click or key came).
fn running() -> bool {
    crate::builtins::audio_context().and_then(|c| js_sys::Reflect::get(&c, &"state".into()).ok()).and_then(|s| s.as_string()).is_some_and(|s| s == "running")
}

fn stop(id: &str) {
    let key = id.to_lowercase();
    if let Some(p) = PLAYERS.with(|p| p.borrow_mut().remove(&key)) {
        let _ = js_sys::Reflect::set(&p.node, &"onaudioprocess".into(), &JsValue::NULL);
        if let Ok(disconnect) = js_sys::Reflect::get(&p.node, &"disconnect".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
            let _ = disconnect.call0(&p.node);
        }
    }
    SINKS.with(|s| s.borrow_mut().remove(&key));
}

/// The page's interval, while anything plays.
fn ensure_tick() {
    if TICK.with(Cell::get).is_some() {
        return;
    }
    let Some(window) = web_sys::window() else { return };
    let tick = Closure::<dyn FnMut()>::new(tick);
    if let Ok(handle) = window.set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), TICK_MS) {
        TICK.with(|t| t.set(Some(handle)));
    }
    tick.forget();
}

fn tick() {
    let now = now_ms();
    // (the clocked sinks take what the time says)
    let ended: Vec<String> = SINKS.with(|s| {
        let mut s = s.borrow_mut();
        let mut ended = Vec::new();
        let mut buf = Vec::new();
        for (id, sink) in s.iter_mut() {
            let rate = f64::from(sink.consumer.rate());
            let due = ((now - sink.began).max(0.0) / 1000.0 * rate) as u64;
            let n = due.saturating_sub(sink.taken).min((rate / 5.0) as u64) as usize;
            sink.taken = due;
            if n == 0 {
                continue;
            }
            buf.resize(n * sink.consumer.channels(), 0.0);
            if !sink.consumer.render(&mut buf) {
                ended.push(id.clone());
            }
        }
        ended
    });
    for id in ended {
        SINKS.with(|s| s.borrow_mut().remove(&id));
    }
    // (players whose stream stopped and faded: their nodes go)
    let done: Vec<String> = PLAYERS.with(|p| p.borrow().iter().filter(|(_, p)| p.done.get()).map(|(k, _)| k.clone()).collect());
    for id in done {
        stop(&id);
    }
    // (the page may play now: the click came)
    if PLAYERS.with(|p| !p.borrow().is_empty()) && running() {
        for id in audioout::playing_ids() {
            if audioout::error(&id) == WAITING_FOR_CLICK {
                audioout::set_error(&id, "");
            }
        }
    }
    if !crate::object_web::debug_stopped() {
        for (id, count) in audioout::due(now, &|id| crate::object_web::rp_has_handler(id, "onneeddata")) {
            crate::object_web::rp_fire_event_args(&id, "onneeddata", &[Value::Integer(count)]);
        }
    }
    // (nothing plays: the interval stops)
    let idle = !audioout::active() && SINKS.with(|s| s.borrow().is_empty()) && PLAYERS.with(|p| p.borrow().is_empty());
    if idle {
        if let (Some(handle), Some(w)) = (TICK.with(|t| t.take()), web_sys::window()) {
            w.clear_interval_with_handle(handle);
        }
    }
}
