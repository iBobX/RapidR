//! The browser transport of the components that offer a program's tools
//! to AI agents (RMcpServer — RapidR Studio's on the web — and
//! RAgentTools): the page's tools offered to the AI agents of the browser
//! (its assistant, an extension, the developer tools' agents) through the
//! browser API of the WebMCP draft of the W3C Web Machine Learning
//! Community Group (not a W3C Standard): `document.modelContext.
//! registerTool(tool, { signal })` for each tool, aborting the signal to
//! take it away. What the tools are and how their calls go is the core's
//! ([`rapidr_value::agent_tools`], the desktop's http transport shares it);
//! this file is only the browser's side.
//!
//! The draft changed shape five times in 2026, so everything that knows it
//! is here and nowhere else: where the model context is ([`model_context`]:
//! `document`, else `navigator` as Chromium 149 had it), what a tool
//! carries (name, title, description, inputSchema, annotations, execute),
//! how a call comes (`execute(input, { signal })`, the input an object or
//! a JSON text) and goes (a promise; a failure as `{ error }` data, since
//! an exception's message never reaches the agent; an aborted call
//! rejected with its signal's reason).
//!
//! The browser says nothing of who calls: every call is
//! [`tools::BROWSER_AGENT`]'s. No token, no address, no prompts, resources
//! or instructions (the draft has none); Progress has nowhere to go. The
//! tools are registered for this page's own origin only (no `exposedTo`).
//! Where the browser doesn't offer it (or the page isn't a secure context,
//! or has no origin of its own), Available is False and Start fails saying
//! why; nothing else changes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use rapidr_value::agent_tools::{self as tools, Done, Dropped, Kind, Queued, ToolDef};
use serde_json::{json, Value as Json};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::object_web::{rp_comp_get, rp_comp_set};
use crate::value::{v_int, v_null, v_str, Value};

/// The longest text an answer has in a browser when MaxResultSize is 0:
/// the browser makers' advice (an agent may refuse a longer one).
pub const BROWSER_MAX_RESULT: usize = 1500;

/// The runtime as the core sees it.
#[derive(Clone, Copy)]
struct Page;

impl tools::Host for Page {
    fn get(self, name: &str, prop: &str) -> Value {
        rp_comp_get(name, prop)
    }
    fn set(self, name: &str, prop: &str, v: Value) {
        rp_comp_set(name, prop, v);
    }
    fn has_handler(self, name: &str, event: &str) -> bool {
        crate::object_web::rp_has_handler(name, event)
    }
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Box<dyn FnOnce()>) {
        crate::object_web::rp_fire_event_then(name, event, args, move |_| then());
    }
    fn wake(self) {
        poll_later();
    }
    fn now(self) -> f64 {
        web_sys::window().and_then(|w| w.performance()).map_or_else(js_sys::Date::now, |p| p.now()) / 1000.0
    }
    // (a key a tool read never reaches an agent)
    fn scrub(self, text: &str) -> String {
        rapidr_ai::keys::redact(text)
    }
    fn components(self) -> Vec<(String, String)> {
        crate::object_web::all_components()
    }
    fn program_set(self, name: &str, prop: &str, v: Value) {
        crate::object_web::rp_prop_set(name, prop, v);
    }
    // (ConfirmChanges: the browser's own question — it blocks, which is
    // what asking first means; a screen reader reads it)
    fn confirm(self, text: &str) -> Option<bool> {
        web_sys::window().and_then(|w| w.confirm_with_message(text).ok())
    }
}

/// One tool the browser has: what it was registered as, the controller
/// whose abort takes it away, and its `execute`, kept alive as long.
struct Registered {
    fingerprint: String,
    controller: web_sys::AbortController,
    _execute: Closure<dyn FnMut(JsValue, JsValue) -> JsValue>,
}

thread_local! {
    /// Each server's tools as the browser has them (server, then tool).
    static REGISTERED: RefCell<HashMap<String, HashMap<String, Registered>>> = RefCell::new(HashMap::new());
    static POLL_ASKED: Cell<bool> = const { Cell::new(false) };
}

// ------------------------------------------------- the draft's shape --

/// The page's model context: `document.modelContext` (the draft since
/// 2026-05), else `navigator.modelContext` (Chromium 149's).
fn model_context() -> Option<JsValue> {
    let w = web_sys::window()?;
    let has = |o: &JsValue| js_sys::Reflect::get(o, &"modelContext".into()).ok().filter(|m| m.is_object());
    w.document().and_then(|d| has(&d.into())).or_else(|| has(&w.navigator().into()))
}

/// Whether this page can offer tools to the browser's agents; why not.
pub fn available() -> Result<(), String> {
    let w = web_sys::window().ok_or("There is no page to offer tools from.")?;
    if !w.is_secure_context() {
        return Err("This page isn't in a secure context (HTTPS, or this computer): only there can a page offer tools to AI agents.".into());
    }
    if w.origin() == "null" {
        return Err("This page has no origin of its own (a sandboxed frame): run the web app in its own page to offer its tools.".into());
    }
    let mc = model_context().ok_or("This browser doesn't give web pages a way to offer tools to AI agents.")?;
    match js_sys::Reflect::get(&mc, &"registerTool".into()) {
        Ok(f) if f.is_function() => Ok(()),
        _ => Err("This browser's way of offering tools to AI agents isn't one RapidR knows (it has no registerTool).".into()),
    }
}

fn js_json(v: &Json) -> JsValue {
    js_sys::JSON::parse(&v.to_string()).unwrap_or(JsValue::NULL)
}

fn set(o: &js_sys::Object, k: &str, v: &JsValue) {
    let _ = js_sys::Reflect::set(o, &k.into(), v);
}

fn js_message(e: &JsValue) -> String {
    js_sys::Reflect::get(e, &"message".into()).ok().and_then(|m| m.as_string()).or_else(|| e.as_string()).unwrap_or_else(|| "no reason given".into())
}

/// A call's input as JSON: an object (the draft since 2026-08), or the
/// JSON text of one (before); nothing is no arguments.
fn input_json(input: &JsValue) -> Result<Json, String> {
    if input.is_undefined() || input.is_null() {
        return Ok(json!({}));
    }
    let text = match input.as_string() {
        Some(t) => t,
        None => js_sys::JSON::stringify(input).ok().and_then(|s| s.as_string()).ok_or("The arguments must be a JSON object.")?,
    };
    match serde_json::from_str::<Json>(&text) {
        Ok(v) if v.is_object() || v.is_null() => Ok(v),
        _ => Err("The arguments must be a JSON object.".into()),
    }
}

/// `tool` registered with the browser for server `name`.
fn register(name: &str, def: &ToolDef) -> Result<Registered, String> {
    let mc = model_context().ok_or("This browser doesn't give web pages a way to offer tools to AI agents.")?;
    let f = js_sys::Reflect::get(&mc, &"registerTool".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok()).ok_or("no registerTool")?;
    let controller = web_sys::AbortController::new().map_err(|e| js_message(&e))?;
    let (server, tool) = (name.to_string(), def.name.clone());
    let execute = Closure::<dyn FnMut(JsValue, JsValue) -> JsValue>::new(move |input: JsValue, options: JsValue| -> JsValue { called(&server, &tool, &input, &options).into() });
    let t = js_sys::Object::new();
    set(&t, "name", &def.name.as_str().into());
    if !def.title.is_empty() {
        set(&t, "title", &def.title.as_str().into());
    }
    set(&t, "description", &def.description.as_str().into());
    set(&t, "inputSchema", &js_json(&def.schema));
    set(&t, "annotations", &js_json(&def.browser_annotations()));
    set(&t, "execute", execute.as_ref());
    let options = js_sys::Object::new();
    set(&options, "signal", &controller.signal().into());
    let done = f.call2(&mc, &t, &options).map_err(|e| js_message(&e))?;
    // (refused later — a name taken, a policy that says no: Error says so)
    if let Ok(p) = done.dyn_into::<js_sys::Promise>() {
        let (server, tool) = (name.to_string(), def.name.clone());
        let refused = Closure::once(move |e: JsValue| {
            let why = format!("AddTool {tool}: the browser didn't take it ({})", js_message(&e));
            web_sys::console::warn_1(&why.as_str().into());
            rp_comp_set(&server, "error", v_str(&why));
        });
        let _ = p.catch(&refused);
        // (kept by the promise: it runs once, if ever)
        refused.forget();
    }
    Ok(Registered { fingerprint: def.fingerprint(), controller, _execute: execute })
}

/// The browser's tools for `name` made the core's: each changed one
/// registered again, each gone taken away.
fn sync(name: &str) {
    let key = name.to_lowercase();
    let wanted: Vec<ToolDef> = if tools::is_open(name) { tools::tools(name) } else { Vec::new() };
    let gone: Vec<Registered> = REGISTERED.with(|r| {
        let mut r = r.borrow_mut();
        let have = r.entry(key.clone()).or_default();
        let stale: Vec<String> = have.iter().filter(|(t, reg)| !wanted.iter().any(|w| &w.name == *t && w.fingerprint() == reg.fingerprint)).map(|(t, _)| t.clone()).collect();
        stale.iter().filter_map(|t| have.remove(t)).collect()
    });
    for g in gone {
        g.controller.abort();
    }
    let missing: Vec<ToolDef> = REGISTERED.with(|r| {
        let r = r.borrow();
        wanted.iter().filter(|w| !r.get(&key).is_some_and(|h| h.contains_key(&w.name))).cloned().collect()
    });
    let mut failed = Vec::new();
    for def in missing {
        match register(name, &def) {
            Ok(reg) => REGISTERED.with(|r| {
                r.borrow_mut().entry(key.clone()).or_default().insert(def.name.clone(), reg);
            }),
            Err(why) => failed.push(format!("AddTool {}: the browser didn't take it ({why})", def.name)),
        }
    }
    if !failed.is_empty() {
        rp_comp_set(name, "error", v_str(&failed.join("; ")));
    }
}

// ---------------------------------------------------------- the calls --

/// Runs `f` once the page's script has returned (a microtask): the
/// browser may call a tool from anywhere, and the runtime is entered only
/// when nothing of it is on the stack — a moment later while it is in use.
fn later(f: impl FnOnce() + 'static) {
    let Some(w) = web_sys::window() else { return };
    let run = Closure::once_into_js(move || {
        if crate::studio_web::busy() {
            let again = Closure::once_into_js(f);
            if let Some(w) = web_sys::window() {
                let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(again.unchecked_ref(), 10);
            }
        } else {
            f();
        }
    });
    w.queue_microtask(run.unchecked_ref());
}

/// The core's turn soon (a handler ran: the next call may go).
fn poll_later() {
    if POLL_ASKED.with(|p| p.replace(true)) {
        return;
    }
    later(|| {
        POLL_ASKED.with(|p| p.set(false));
        for name in tools::open_names() {
            tools::poll(Page, &name);
        }
    });
}

/// The browser called `tool` of server `name`: a promise, settled with
/// the program's answer.
fn called(name: &str, tool: &str, input: &JsValue, options: &JsValue) -> js_sys::Promise {
    let signal = js_sys::Reflect::get(options, &"signal".into()).ok().and_then(|s| s.dyn_into::<web_sys::AbortSignal>().ok());
    if let Some(s) = signal.as_ref().filter(|s| s.aborted()) {
        return js_sys::Promise::reject(&reason(s));
    }
    let args = input_json(input);
    let mut ends = None;
    let promise = js_sys::Promise::new(&mut |resolve, reject| ends = Some((resolve, reject)));
    let Some((resolve, reject)) = ends else { return promise };
    let (name, tool) = (name.to_string(), tool.to_string());
    later(move || take(&name, &tool, args, resolve, reject, signal));
    promise
}

/// Why an aborted call ended (its signal's reason, else an AbortError).
fn reason(s: &web_sys::AbortSignal) -> JsValue {
    js_sys::Reflect::get(s, &"reason".into()).ok().filter(|r| !r.is_undefined()).unwrap_or_else(|| {
        let e = js_sys::Error::new("The call was cancelled.");
        e.set_name("AbortError");
        e.into()
    })
}

/// A call taken in: checked (the core's), queued with its promise, handed
/// to the program; a refusal answered at once, as data.
fn take(name: &str, tool: &str, args: Result<Json, String>, resolve: js_sys::Function, reject: js_sys::Function, signal: Option<web_sys::AbortSignal>) {
    let now = |v: Json| {
        let _ = resolve.call1(&JsValue::NULL, &js_json(&v));
    };
    if !tools::is_open(name) {
        return now(json!({"error": "This page stopped offering its tools."}));
    }
    if tools::shut_out(name, tools::BROWSER_AGENT) {
        return now(json!({"error": "Disconnected by the person using the app."}));
    }
    let args = match args {
        Ok(a) => a,
        Err(why) => return now(json!({"error": why})),
    };
    tools::heard(Page, name, tools::BROWSER_AGENT, "", true);
    let (target, args) = match tools::check_call(name, Kind::Tool, tool, args) {
        Ok(c) => c,
        Err(why) => return now(json!({"error": why.sentence()})),
    };
    let listener: Rc<RefCell<Option<Listener>>> = Rc::new(RefCell::new(None));
    let sink = Promised { resolve: resolve.clone(), reject, signal: signal.clone(), max: max_result(name), listener: listener.clone() };
    let Some(id) = tools::enqueue(Page, name, Queued { kind: Kind::Tool, target, args, client: tools::BROWSER_AGENT.into() }, Box::new(sink)) else {
        return now(json!({"error": "This page stopped offering its tools."}));
    };
    // (the caller gives up: the call goes, OnToolCancel says so)
    if let Some(s) = signal {
        let server = name.to_string();
        let on_abort = Closure::<dyn FnMut()>::new(move || {
            let server = server.clone();
            later(move || {
                tools::cancel(Page, &server, id);
            });
        });
        let _ = s.add_event_listener_with_callback("abort", on_abort.as_ref().unchecked_ref());
        *listener.borrow_mut() = Some(on_abort);
    }
    tools::poll(Page, name);
}

/// MaxResultSize, or the browser's advice when it is 0.
fn max_result(name: &str) -> usize {
    match rp_comp_get(name, "maxresultsize").to_i64() {
        n if n > 0 => n as usize,
        _ => BROWSER_MAX_RESULT,
    }
}

type Listener = Closure<dyn FnMut()>;

/// Where one call's answer goes: its promise.
struct Promised {
    resolve: js_sys::Function,
    reject: js_sys::Function,
    signal: Option<web_sys::AbortSignal>,
    max: usize,
    listener: Rc<RefCell<Option<Listener>>>,
}

impl tools::Sink for Promised {
    fn finish(self: Box<Self>, done: Done) -> bool {
        if let (Some(s), Some(l)) = (&self.signal, self.listener.borrow_mut().take()) {
            let _ = s.remove_event_listener_with_callback("abort", l.as_ref().unchecked_ref());
        }
        let value = match done {
            Done::Answer { outcome, .. } => outcome.browser_value(self.max),
            Done::Dropped(Dropped::Stopped) => json!({"error": "This page stopped offering its tools before this call was answered."}),
            Done::Dropped(Dropped::Disconnected) => json!({"error": "Disconnected by the person using the app."}),
            Done::Dropped(Dropped::Cancelled) => {
                let why = self.signal.as_ref().map(reason).unwrap_or_else(|| js_sys::Error::new("The call was cancelled.").into());
                return self.reject.call1(&JsValue::NULL, &why).is_ok();
            }
        };
        self.resolve.call1(&JsValue::NULL, &js_json(&value)).is_ok()
    }
}

// ------------------------------------------------- the component's side --

fn start(name: &str) -> Value {
    stop(name);
    if let Err(why) = available() {
        rp_comp_set(name, "error", v_str(&why));
        rp_comp_set(name, "listening", v_int(0));
        return v_int(0);
    }
    let idle = rp_comp_get(name, "idleseconds").to_f64();
    tools::open(Page, name, tools::Info::of(Page, name), idle);
    rp_comp_set(name, "error", v_str(""));
    rp_comp_set(name, "url", v_str(""));
    rp_comp_set(name, "port", v_int(0));
    rp_comp_set(name, "listening", v_int(1));
    sync(name);
    v_int(1)
}

/// Every tool taken away (the agents see them go), each waiting call told.
fn stop(name: &str) {
    let gone: Vec<Registered> = REGISTERED.with(|r| r.borrow_mut().remove(&name.to_lowercase()).map(|m| m.into_values().collect()).unwrap_or_default());
    for g in gone {
        g.controller.abort();
    }
    tools::close(Page, name);
}

/// RMcpServer `name`'s method in a browser: the transport's own (Start,
/// Stop; NewToken makes none — no token here), else the core's, the
/// browser's tools following the definitions.
pub fn method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "start" => start(name),
        "stop" => {
            stop(name);
            v_null()
        }
        "newtoken" => v_str(""),
        _ => {
            let r = tools::method(Page, name, method, args).unwrap_or_else(v_null);
            if matches!(method, "addtool" | "removetool" | "cleartools") && tools::is_open(name) {
                sync(name);
            }
            r
        }
    }
}

/// RMcpServer's properties the transport answers: Available, Transport.
pub fn get(prop: &str) -> Option<Value> {
    match prop {
        "available" => Some(v_int(available().is_ok() as i64)),
        "transport" => Some(v_str("browser")),
        _ => None,
    }
}
