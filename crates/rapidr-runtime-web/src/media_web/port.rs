//! RMIDIPort in the browser: the model is `rapidr_value::objects::midiport`
//! (the desktop's too); these are its devices — the browser's MIDI
//! interface (the Web MIDI API), through RapidR's own code as QMIDI's
//! output is (never midir's web backend, which always asks for SysEx:
//! docs/legal/review-2026-11-05-midi.md, C2) — and its delivery.
//!
//! - **Nothing is asked when the page loads.** The first call that needs
//!   the devices (InputCount, OutputCount, InputDevice, FillInputs,
//!   OpenInput …) shows RapidR's own one-line explanation first (the
//!   kernel's message box; the page's confirm in a native web build), then
//!   the browser's prompt: `requestMIDIAccess()` **without** SysEx. The
//!   program waits for the answer (the page goes on); where it can't wait
//!   the devices come a moment later, with OnDevicesChanged.
//! - **SysEx is asked separately**, only when the program wants it
//!   (IgnoreSysEx = False, or a SysEx sent): a second explanation, then
//!   `requestMIDIAccess({sysex: true})`; the open ports move to that access.
//!   Refusing SysEx leaves notes working.
//! - **A refusal is a plain error** (Error, the model's REFUSED_* texts);
//!   "Not now" holds for the lists, and OpenInput / OpenOutput ask again.
//!   Nothing about the answer is kept beyond the browser's own memory, and
//!   the devices' names go nowhere.
//! - **Under the tests** the page's (or the program's ENVIRON)
//!   `RAPIDR_TEST_MIDIPORT` gives the model's scripted devices; nothing is
//!   asked.
//! - **Delivery**: `onmidimessage` hands the bytes to the input's inbox (the
//!   model's parser), and a task after it delivers them as the desktop's
//!   loop does; `onstatechange` says the devices changed.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use rapidr_value::objects::midiport::{self, Host, Inbox, MidiOut, MidiSystem, OpenError, TestMidi, Then};
use rapidr_value::{v_int, v_null, Value};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// What the page may use: nothing yet, notes and controls, or SysEx too.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Nothing,
    Notes,
    SysEx,
}

thread_local! {
    static TESTING: Cell<bool> = const { Cell::new(false) };
    /// The browser's MIDIAccess, once allowed, and what it allows.
    static ACCESS: RefCell<Option<JsValue>> = const { RefCell::new(None) };
    static LEVEL: Cell<Level> = const { Cell::new(Level::Nothing) };
    /// Why there are no devices (a refusal, "Not now", no MIDI here).
    static NO_MIDI: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Why SysEx can't go (refused, "Not now").
    static NO_SYSEX: RefCell<Option<String>> = const { RefCell::new(None) };
    static ASKING: Cell<bool> = const { Cell::new(false) };
    static PUMP_ASKED: Cell<bool> = const { Cell::new(false) };
    /// The open ports, moved to the new access when SysEx is allowed.
    static OPEN_INS: RefCell<Vec<Rc<WebIn>>> = const { RefCell::new(Vec::new()) };
    static OPEN_OUTS: RefCell<Vec<Rc<RefCell<JsValue>>>> = const { RefCell::new(Vec::new()) };
    static STATE_CHANGE: RefCell<Option<Closure<dyn FnMut()>>> = const { RefCell::new(None) };
}

fn now_ms() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map_or_else(js_sys::Date::now, |p| p.now())
}

fn page_var(name: &str) -> Option<String> {
    web_sys::window().and_then(|w| js_sys::Reflect::get(&w, &name.into()).ok()).and_then(|v| v.as_string())
}

/// Gives RMIDIPort its devices (once: the first one made).
pub fn install() {
    thread_local! {
        static DONE: Cell<bool> = const { Cell::new(false) };
    }
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    midiport::set_clock(now_ms);
    midiport::set_waker(wake);
    let script = rapidr_value::environ::get("RAPIDR_TEST_MIDIPORT");
    match page_var("RAPIDR_TEST_MIDIPORT").or((!script.is_empty()).then_some(script)) {
        Some(s) => {
            TESTING.with(|t| t.set(true));
            midiport::set_system(Rc::new(TestMidi::parse(&s)));
        }
        None => midiport::set_system(Rc::new(WebMidi)),
    }
    wake();
}

/// A delivery as soon as the page's current task is over (any number of
/// wakes, one delivery).
fn wake() {
    if PUMP_ASKED.with(|p| p.replace(true)) {
        return;
    }
    let cb = Closure::once_into_js(|| {
        PUMP_ASKED.with(|p| p.set(false));
        midiport::look(host());
        midiport::pump(host());
    });
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 0);
    }
}

/// DOEVENTS: what arrived delivered now (its handlers run right after the
/// DOEVENTS, as the desktop's do), not only when the page next has a turn —
/// a console program's `DO: DOEVENTS: LOOP` hears its MIDI as it comes.
pub fn poll() {
    if !midiport::system_installed() {
        return;
    }
    midiport::look(host());
    midiport::pump(host());
}

fn fire_then(name: &str, event: &str, args: &[Value], then: Then) {
    crate::object_web::rp_fire_event_then(name, event, args, then);
}

fn host() -> Host {
    Host { has_handler: crate::object_web::rp_has_handler, fire_then, now_ms }
}

// ----------------------------------------------------------- the methods --

/// RMIDIPort `name`'s method (`rp_comp_method`'s; `None`: not one): the
/// browser asked first when it needs to be.
pub fn method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    if !midiport::exists(name) {
        return None;
    }
    install();
    let devices = matches!(method, "inputcount" | "outputcount" | "inputdevice" | "outputdevice" | "fillinputs" | "filloutputs" | "openinput" | "openoutput");
    let sends_sysex = method == "sendsysex" || (method == "sendhex" && midiport::parse_hex(&args.first().map(Value::to_string_val).unwrap_or_default()).is_ok_and(|b| b.contains(&0xF0)));
    if (devices || sends_sysex) && !TESTING.with(Cell::get) {
        let opens = method.starts_with("open");
        let need = if sends_sysex || midiport::wants_sysex(name) { Level::SysEx } else { Level::Notes };
        if must_ask(need, opens) {
            let (n, m, a) = (name.to_string(), method.to_string(), args.to_vec());
            if crate::dialog_web::wait_task() {
                wasm_bindgen_futures::spawn_local(async move {
                    ask(need).await;
                    let v = run(&n, &m, &a);
                    crate::dialog_web::task_done(v);
                });
                return Some(v_null());
            }
            // (it can't wait here — a native web build, a handler inside
            // another call: asked now, the devices come a moment later)
            ASKING.with(|a| a.set(true));
            wasm_bindgen_futures::spawn_local(async move { ask(need).await });
        }
    }
    Some(run(name, method, args))
}

fn run(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "fillinputs" | "filloutputs" => fill(name, method == "fillinputs", args),
        _ => rapidr_value::objects::midiport::call(name, method, args).unwrap_or_else(v_null),
    }
}

/// FillInputs / FillOutputs(List), as the desktop's.
fn fill(name: &str, input: bool, args: &[Value]) -> Value {
    use crate::object_web::{rp_comp_method, rp_comp_set, rp_comp_type};
    let Some((items, at)) = midiport::items(name, input) else { return v_int(0) };
    let control = args.first().map(Value::to_string_val).unwrap_or_default();
    if control.is_empty() || rp_comp_type(&control).is_empty() {
        return v_int(items.len() as i64);
    }
    rp_comp_method(&control, "clear", &[]);
    for item in &items {
        rp_comp_method(&control, "additems", &[rapidr_value::v_str(item)]);
    }
    // (the open one selected; none when none is open)
    rp_comp_set(&control, "itemindex", v_int(at.map_or(-1, |i| i as i64)));
    v_int(items.len() as i64)
}

// ------------------------------------------------- asking the browser --

/// Whether the page must ask before this call: it doesn't have `need`, it
/// isn't asking already, and the answer wasn't no (`opens`: "Not now" asks
/// again for an OpenInput / OpenOutput).
fn must_ask(need: Level, opens: bool) -> bool {
    if ASKING.with(Cell::get) || LEVEL.with(Cell::get) >= need {
        return false;
    }
    let held = |slot: &'static std::thread::LocalKey<RefCell<Option<String>>>, not_now: &str| slot.with(|s| s.borrow().as_deref().is_some_and(|w| !(opens && w == not_now)));
    if held(&NO_MIDI, midiport::NOT_NOW) {
        return false;
    }
    !(need == Level::SysEx && LEVEL.with(Cell::get) == Level::Notes && held(&NO_SYSEX, midiport::NOT_NOW_SYSEX))
}

/// Asks for `need`: notes first (RapidR's explanation, then the browser's
/// prompt without SysEx), then, when SysEx is wanted, SysEx separately.
async fn ask(need: Level) {
    ASKING.with(|a| a.set(true));
    if LEVEL.with(Cell::get) < Level::Notes {
        ask_level(false).await;
    }
    if need == Level::SysEx && LEVEL.with(Cell::get) == Level::Notes {
        ask_level(true).await;
    }
    ASKING.with(|a| a.set(false));
    midiport::devices_changed();
    wake();
}

async fn ask_level(sysex: bool) {
    let slot = if sysex { &NO_SYSEX } else { &NO_MIDI };
    if !has_web_midi() {
        NO_MIDI.with(|n| *n.borrow_mut() = Some(midiport::NO_BROWSER_MIDI.into()));
        return;
    }
    let (title, text, [go, later]) = midiport::permission_text(sysex);
    if !explain(&title, &text, &[go.as_str(), later.as_str()]).await {
        slot.with(|s| *s.borrow_mut() = Some(if sysex { midiport::NOT_NOW_SYSEX } else { midiport::NOT_NOW }.into()));
        return;
    }
    match request(sysex).await {
        Ok(access) => {
            slot.with(|s| *s.borrow_mut() = None);
            take_access(access, if sysex { Level::SysEx } else { Level::Notes });
        }
        Err(why) => slot.with(|s| *s.borrow_mut() = Some(why)),
    }
}

/// RapidR's explanation: true for "Continue".
async fn explain(title: &str, text: &str, labels: &[&str; 2]) -> bool {
    let then = |b: Option<usize>| Value::Integer(b.map_or(-1, |i| i as i64));
    match crate::kernel_web::open_choice(title, text, labels, then) {
        Some(Ok(v)) => v.to_i64() == 0,
        Some(Err(id)) => loop {
            sleep_ms(40).await;
            if let Some(v) = rapidr_ui_app::dialogs::finished(id) {
                break v.to_i64() == 0;
            }
        },
        None => web_sys::window().and_then(|w| w.confirm_with_message(&format!("{title}\n\n{text}")).ok()).unwrap_or(false),
    }
}

async fn sleep_ms(ms: i32) {
    let p = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms.max(0));
        }
    });
    let _ = JsFuture::from(p).await;
}

fn has_web_midi() -> bool {
    web_sys::window().and_then(|w| js_sys::Reflect::get(&w.navigator(), &"requestMIDIAccess".into()).ok()).is_some_and(|f| f.is_function())
}

/// The browser's prompt: `requestMIDIAccess()` (no options: no SysEx), or
/// with `{sysex: true}`.
async fn request(sysex: bool) -> Result<JsValue, String> {
    let nav = web_sys::window().ok_or_else(|| midiport::NO_BROWSER_MIDI.to_string())?.navigator();
    let f: js_sys::Function = js_sys::Reflect::get(&nav, &"requestMIDIAccess".into()).ok().and_then(|f| f.dyn_into().ok()).ok_or_else(|| midiport::NO_BROWSER_MIDI.to_string())?;
    let called = if sysex {
        let o = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&o, &"sysex".into(), &true.into());
        f.call1(&nav, &o)
    } else {
        f.call0(&nav)
    };
    let refused = || if sysex { midiport::REFUSED_SYSEX } else { midiport::REFUSED_MIDI }.to_string();
    let p = called.map_err(|_| refused())?;
    JsFuture::from(js_sys::Promise::resolve(&p)).await.map_err(|e| {
        let n = js_sys::Reflect::get(&e, &"name".into()).ok().and_then(|n| n.as_string()).unwrap_or_default();
        if n == "NotSupportedError" {
            midiport::NO_BROWSER_MIDI.to_string()
        } else {
            refused()
        }
    })
}

/// The access the browser gave: kept, its plug-ins and -outs heard, the
/// open ports moved to it.
fn take_access(access: JsValue, level: Level) {
    ACCESS.with(|a| *a.borrow_mut() = Some(access.clone()));
    LEVEL.with(|l| l.set(level));
    STATE_CHANGE.with(|s| {
        let mut s = s.borrow_mut();
        let cb = s.get_or_insert_with(|| {
            Closure::<dyn FnMut()>::new(|| {
                midiport::devices_changed();
                wake();
            })
        });
        let _ = js_sys::Reflect::set(&access, &"onstatechange".into(), cb.as_ref().unchecked_ref());
    });
    for i in OPEN_INS.with(|o| o.borrow().clone()) {
        if let Some(p) = port_by_id("inputs", &i.id) {
            i.rebind(p);
        }
    }
    for o in OPEN_OUTS.with(|o| o.borrow().clone()) {
        let id = prop(&o.borrow(), "id");
        if let Some(p) = port_by_id("outputs", &id) {
            *o.borrow_mut() = p;
        }
    }
}

// ----------------------------------------------------------- the devices --

fn prop(v: &JsValue, k: &str) -> String {
    js_sys::Reflect::get(v, &k.into()).ok().and_then(|x| x.as_string()).unwrap_or_default()
}

/// The access's inputs or outputs (`kind`), in the browser's order.
fn ports(kind: &str) -> Vec<JsValue> {
    let Some(access) = ACCESS.with(|a| a.borrow().clone()) else { return Vec::new() };
    let Ok(map) = js_sys::Reflect::get(&access, &kind.into()) else { return Vec::new() };
    let Ok(values) = js_sys::Reflect::get(&map, &"values".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) else { return Vec::new() };
    let Ok(iter) = values.call0(&map) else { return Vec::new() };
    js_sys::try_iter(&iter).ok().flatten().map(|it| it.filter_map(Result::ok).take(256).collect()).unwrap_or_default()
}

/// A port's name (its maker's and a number when it has none).
fn name_of(p: &JsValue, i: usize, kind: &str) -> String {
    let n = prop(p, "name");
    if !n.trim().is_empty() {
        return n;
    }
    let m = prop(p, "manufacturer");
    let what = if kind == "inputs" { "MIDI input" } else { "MIDI output" };
    if m.trim().is_empty() {
        format!("{what} {}", i + 1)
    } else {
        format!("{m} {what} {}", i + 1)
    }
}

fn names(kind: &str) -> Vec<String> {
    ports(kind).iter().enumerate().map(|(i, p)| name_of(p, i, kind)).collect()
}

fn find(kind: &str, name: &str, nth: usize) -> Option<JsValue> {
    ports(kind).into_iter().enumerate().filter(|(i, p)| name_of(p, *i, kind) == name).map(|(_, p)| p).nth(nth)
}

fn port_by_id(kind: &str, id: &str) -> Option<JsValue> {
    ports(kind).into_iter().find(|p| prop(p, "id") == id)
}

/// Why there are no devices to list, if so.
fn unavailable() -> Option<String> {
    if let Some(why) = NO_MIDI.with(|n| n.borrow().clone()) {
        return Some(why);
    }
    (LEVEL.with(Cell::get) == Level::Nothing).then(|| if ASKING.with(Cell::get) { midiport::ASKING } else { midiport::NOT_NOW }.to_string())
}

struct WebMidi;

impl MidiSystem for WebMidi {
    fn inputs(&self) -> Result<Vec<String>, String> {
        unavailable().map_or_else(|| Ok(names("inputs")), Err)
    }

    fn outputs(&self) -> Result<Vec<String>, String> {
        unavailable().map_or_else(|| Ok(names("outputs")), Err)
    }

    fn open_input(&self, name: &str, nth: usize, inbox: Arc<Inbox>) -> Result<Box<dyn Any>, OpenError> {
        let port = find("inputs", name, nth).ok_or(OpenError::NotFound)?;
        let into = inbox.clone();
        let on_message = Closure::<dyn FnMut(JsValue)>::new(move |ev: JsValue| {
            let Ok(data) = js_sys::Reflect::get(&ev, &"data".into()) else { return };
            let bytes = js_sys::Uint8Array::new(&data).to_vec();
            into.push(&bytes, now_ms());
        });
        let i = Rc::new(WebIn { id: prop(&port, "id"), port: RefCell::new(JsValue::NULL), on_message, _inbox: inbox });
        i.rebind(port);
        OPEN_INS.with(|o| o.borrow_mut().push(i.clone()));
        Ok(Box::new(InLink(i)))
    }

    fn open_output(&self, name: &str, nth: usize) -> Result<Box<dyn MidiOut>, OpenError> {
        let port = find("outputs", name, nth).ok_or(OpenError::NotFound)?;
        let slot = Rc::new(RefCell::new(port));
        OPEN_OUTS.with(|o| o.borrow_mut().push(slot.clone()));
        Ok(Box::new(WebOut(slot)))
    }

    fn can_send_sysex(&self) -> Result<(), String> {
        if LEVEL.with(Cell::get) == Level::SysEx {
            return Ok(());
        }
        Err(NO_SYSEX.with(|n| n.borrow().clone()).unwrap_or_else(|| midiport::REFUSED_SYSEX.into()))
    }
}

/// An open input: its port (moved when SysEx is allowed) and the handler
/// that hands its messages to the inbox.
struct WebIn {
    id: String,
    port: RefCell<JsValue>,
    on_message: Closure<dyn FnMut(JsValue)>,
    _inbox: Arc<Inbox>,
}

impl WebIn {
    fn rebind(&self, port: JsValue) {
        let old = self.port.replace(port.clone());
        if !old.is_null() {
            let _ = js_sys::Reflect::set(&old, &"onmidimessage".into(), &JsValue::NULL);
        }
        // (setting the handler opens the port)
        let _ = js_sys::Reflect::set(&port, &"onmidimessage".into(), self.on_message.as_ref().unchecked_ref());
    }
}

struct InLink(Rc<WebIn>);

impl Drop for InLink {
    fn drop(&mut self) {
        let port = self.0.port.borrow().clone();
        let _ = js_sys::Reflect::set(&port, &"onmidimessage".into(), &JsValue::NULL);
        if let Ok(close) = js_sys::Reflect::get(&port, &"close".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
            let _ = close.call0(&port);
        }
        OPEN_INS.with(|o| o.borrow_mut().retain(|i| !Rc::ptr_eq(i, &self.0)));
    }
}

struct WebOut(Rc<RefCell<JsValue>>);

impl MidiOut for WebOut {
    fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        let port = self.0.borrow().clone();
        let send: js_sys::Function = js_sys::Reflect::get(&port, &"send".into()).ok().and_then(|f| f.dyn_into().ok()).ok_or("The MIDI output can't send")?;
        send.call1(&port, &js_sys::Uint8Array::from(bytes)).map(|_| ()).map_err(|e| {
            let n = js_sys::Reflect::get(&e, &"name".into()).ok().and_then(|n| n.as_string()).unwrap_or_default();
            if n == "InvalidAccessError" {
                midiport::REFUSED_SYSEX.to_string()
            } else {
                format!("The MIDI output didn't take the message ({n})")
            }
        })
    }
}

impl Drop for WebOut {
    fn drop(&mut self) {
        OPEN_OUTS.with(|o| o.borrow_mut().retain(|x| !Rc::ptr_eq(x, &self.0)));
    }
}
