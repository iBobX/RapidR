//! The I/O and media objects in the browser (rapidr_value::objects::rqlib;
//! docs/io-media-plan.md), as runtime-core's `io.rs` does them on the
//! desktop: QDOWNLOAD's transfer by `fetch` (the program waits as for a
//! dialog: `dialog_web::wait_task`), QCOMPORT's ports by Web Serial — or
//! the tests' scripted ones —, the events their models leave, and QVIDEO's
//! window: a QCANVAS (an HTML canvas) the frames are drawn on by the same
//! decoder as the desktop's (`objects::avi`, in wasm), paced by an
//! interval of its own while it plays.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use rapidr_value::objects::comport::{self, Flow, Link, PortError, Settings};
use rapidr_value::objects::download::{Outcome, Shown};
use rapidr_value::objects::{media, rqlib};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::object_web::{rp_comp_get, rp_comp_method, rp_comp_set, rp_comp_type, rp_create_component, rp_fire_event_args};
use crate::value::{v_bool, v_int, v_null, v_str, Value};

fn time_now() -> String {
    crate::builtins::rp_time().to_string_val()
}

/// Object `name` of type `type_name` was made: its device, ready; a
/// QDOWNLOAD's QGAUGE and QLABEL made (`name.StateGauge`, `name.SpeedLbl`).
pub fn created(name: &str, type_name: &str) {
    match type_name.to_ascii_uppercase().as_str() {
        "RCOMPORT" => install_ports(),
        "RMIDI" | "RWAVE" | "RVIDEO" | "RCDAUDIO" => crate::media_web::install(),
        "RDOWNLOAD" => {
            let gauge = format!("{name}.stategauge");
            rp_create_component(&gauge, "RPROGRESSBAR");
            rp_comp_set(&gauge, "width", v_int(200));
            rp_comp_set(&gauge, "height", v_int(20));
            rp_create_component(&format!("{name}.speedlbl"), "RLABEL");
        }
        _ => {}
    }
}

/// The events object `name`'s model left, fired.
pub fn fire_events(name: &str) {
    // (a media object's Timer turned on or off by Play, Stop …)
    if rqlib::take_timer_changed(name) {
        crate::object_web::update_timer(name);
    }
    if rqlib::is_video(name) {
        video_changed(name);
    }
    for (event, args) in rqlib::take_events(name) {
        rp_fire_event_args(name, event, &args);
    }
}

thread_local! {
    /// The intervals pacing the playing QVIDEOs' frames.
    static FRAME_TIMERS: RefCell<std::collections::HashMap<String, i32>> = RefCell::new(Default::default());
}

/// QVIDEO `name` after a call, as on the desktop (runtime-core's io.rs):
/// its window's components follow the model, its frames are paced while
/// it plays, the frame it's at is shown.
fn video_changed(name: &str) {
    if rqlib::take_frames_changed(name) {
        frames_paced(name);
    }
    if let Some(w) = rqlib::take_video_window(name) {
        video_window(name, &w);
    }
    video_frame(name);
}

/// QVIDEO `name`'s frames interval started (playing) or stopped.
fn frames_paced(name: &str) {
    let key = name.to_lowercase();
    let Some(window) = web_sys::window() else { return };
    if let Some(handle) = FRAME_TIMERS.with(|t| t.borrow_mut().remove(&key)) {
        window.clear_interval_with_handle(handle);
    }
    let Some((_, ms, true)) = rqlib::video_frames(&rqlib::frames_timer(&key)) else { return };
    let video = key.clone();
    let tick = Closure::<dyn FnMut()>::new(move || video_frame(&video));
    if let Ok(handle) = window.set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), ms as i32) {
        FRAME_TIMERS.with(|t| t.borrow_mut().insert(key, handle));
    }
    tick.forget();
}

/// QVIDEO `name`'s window as its components (runtime-core's io.rs: the
/// same): `<name>.screen`, a black QCANVAS on Parent's form or filling
/// `<name>.window`, a QFORM of its own.
fn video_window(name: &str, w: &media::VideoWindow) {
    let (screen, form) = (media::VideoWindow::screen(name), media::VideoWindow::form(name));
    let own = w.parent.is_empty();
    if !w.open {
        if !rp_comp_type(&screen).is_empty() {
            rp_comp_set(&screen, "visible", v_bool(false));
        }
        if rp_comp_type(&form) == "RFORM" {
            rp_comp_method(&form, "close", &[]);
        }
        return;
    }
    if rp_comp_type(&screen).is_empty() {
        rp_create_component(&screen, "RCANVAS");
        rp_comp_set(&screen, "color", v_int(0));
    }
    if own {
        if rp_comp_type(&form).is_empty() {
            rp_create_component(&form, "RFORM");
        }
        rp_comp_set(&form, "borderstyle", v_int(if w.popup { 0 } else { 2 }));
        rp_comp_set(&form, "caption", v_str(&w.caption));
        if w.placed {
            rp_comp_set(&form, "left", v_int(w.left));
            rp_comp_set(&form, "top", v_int(w.top));
        }
        rp_comp_set(&form, "width", v_int(w.width));
        rp_comp_set(&form, "height", v_int(w.height));
        rp_comp_set(&screen, "parent", v_str(&form));
        rp_comp_set(&screen, "visible", v_bool(true));
        if w.popup {
            // (a popup's 1-pixel black border around the picture)
            rp_comp_set(&form, "color", v_int(0));
            rp_comp_set(&screen, "align", v_int(0));
            rp_comp_set(&screen, "left", v_int(1));
            rp_comp_set(&screen, "top", v_int(1));
            rp_comp_set(&screen, "width", v_int((w.width - 2).max(0)));
            rp_comp_set(&screen, "height", v_int((w.height - 2).max(0)));
        } else {
            // (alClient: the picture fills the window)
            rp_comp_set(&screen, "align", v_int(5));
        }
        if w.visible {
            rp_comp_set(&form, "windowstate", v_int(w.state));
            rp_comp_method(&form, "show", &[]);
        }
    } else {
        if rp_comp_type(&form) == "RFORM" {
            rp_comp_method(&form, "close", &[]);
        }
        rp_comp_set(&screen, "align", v_int(0));
        rp_comp_set(&screen, "parent", v_str(&w.parent));
        rp_comp_set(&screen, "left", v_int(w.left));
        rp_comp_set(&screen, "top", v_int(w.top));
        rp_comp_set(&screen, "width", v_int(w.width));
        rp_comp_set(&screen, "height", v_int(w.height));
        rp_comp_set(&screen, "visible", v_bool(w.visible));
    }
}

/// QVIDEO `name`'s frame now on its screen (when it changed).
fn video_frame(name: &str) {
    let screen = media::VideoWindow::screen(name);
    if rp_comp_type(&screen).is_empty() {
        return;
    }
    let (w, h) = (rp_comp_get(&screen, "width").to_i64(), rp_comp_get(&screen, "height").to_i64());
    if rqlib::video_draw(name, w, h) {
        crate::kernel_web::redraw();
    }
}

/// `QDOWNLOAD.StateGauge.Parent = Form`: a member of its gauge or label.
pub fn sub_component(name: &str, prop: &str) -> Option<(String, String)> {
    if !rqlib::is_download(name) {
        return None;
    }
    let (sub, member) = prop.split_once('.')?;
    matches!(sub, "stategauge" | "speedlbl").then(|| (format!("{}.{sub}", name.to_lowercase()), member.to_string()))
}

/// A method of one of these objects (`rp_comp_method`'s).
pub fn method(name: &str, method: &str, args: &[Value]) -> Value {
    if method == "leechfile" && rqlib::is_download(name) {
        return leech_file(name);
    }
    if method == "open" && rqlib::is_comport(name) && !comport::ports_installed() {
        let v = web_serial_open(name);
        fire_events(name);
        return v;
    }
    let v = match rapidr_value::objects::call(name, method, args, &|id, p| crate::object_web::rp_comp_get(id, p)) {
        Some(Ok(v)) => v,
        Some(Err(e)) => {
            web_sys::console::warn_1(&JsValue::from_str(&format!("[rapidr] {name}.{method}: {e}")));
            v_null()
        }
        None => v_null(),
    };
    fire_events(name);
    // (QCOMPORT's Wait, ms: the page can't sleep — the program pauses after
    // the call instead, its result kept)
    if rqlib::is_comport(name) {
        let wait = match method {
            "writestring" | "readstring" => args.get(1),
            "write" | "read" => args.get(2),
            _ => None,
        };
        if let Some(ms) = wait.map(Value::to_f64).filter(|ms| *ms > 0.0) {
            crate::dialog_web::pause_with(ms, v.clone());
        }
    }
    v
}

// ------------------------------------------------------------ QDOWNLOAD --

fn show_progress(name: &str, received: u64, length: Option<u64>) {
    if let Some(Shown::Gauge { position, speed }) = rqlib::download_progress(name, received, length, &time_now()) {
        rp_comp_set(&format!("{name}.stategauge"), "position", v_int(position));
        rp_comp_set(&format!("{name}.speedlbl"), "caption", v_str(&speed));
    }
}

/// LeechFile's end: the model told, the file written (OutDevice 2).
fn finish(name: &str, outcome: Outcome) -> i64 {
    let (result, file) = rqlib::download_finish(name, outcome);
    if let Some((path, bytes)) = file {
        let _ = crate::object_web::web_write_file(&path, &bytes);
    }
    result
}

/// LeechFile: the library's checks, then the file fetched while the program
/// waits (the page paints, the program's timers tick). Where the program
/// can't wait (a compiled page's code), a synchronous request instead.
fn leech_file(name: &str) -> Value {
    let request = match rqlib::download_begin(name, &time_now()) {
        Ok(r) => r,
        Err(result) => return v_int(result),
    };
    if !crate::dialog_web::wait_task() {
        let outcome = fetch_now(&request.url);
        if let Outcome::Response { length, body, .. } = &outcome {
            show_progress(name, body.len() as u64, *length);
        }
        return v_int(finish(name, outcome));
    }
    let name = name.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let outcome = fetch_async(&name, &request.url).await;
        let result = finish(&name, outcome);
        crate::dialog_web::task_done(v_int(result));
    });
    v_null()
}

async fn fetch_async(name: &str, url: &str) -> Outcome {
    let Some(window) = web_sys::window() else { return Outcome::NoConnection };
    let Ok(resp) = JsFuture::from(window.fetch_with_str(url)).await else { return Outcome::NoConnection };
    let resp: web_sys::Response = resp.unchecked_into();
    let status = resp.status();
    let length = resp.headers().get("content-length").ok().flatten().and_then(|l| l.trim().parse::<u64>().ok());
    show_progress(name, 0, length);
    let mut body = Vec::new();
    if let Some(stream) = resp.body() {
        let reader: web_sys::ReadableStreamDefaultReader = stream.get_reader().unchecked_into();
        loop {
            let Ok(chunk) = JsFuture::from(reader.read()).await else { return Outcome::Closed };
            if js_sys::Reflect::get(&chunk, &"done".into()).ok().and_then(|d| d.as_bool()).unwrap_or(true) {
                break;
            }
            if let Ok(v) = js_sys::Reflect::get(&chunk, &"value".into()) {
                body.extend(js_sys::Uint8Array::new(&v).to_vec());
            }
            show_progress(name, body.len() as u64, length);
        }
    }
    Outcome::Response { status, length, body }
}

/// A synchronous GET (the bytes as `x-user-defined` text: each character's
/// low byte).
fn fetch_now(url: &str) -> Outcome {
    let Ok(xhr) = web_sys::XmlHttpRequest::new() else { return Outcome::NoConnection };
    if xhr.open_with_async("GET", url, false).is_err() {
        return Outcome::NoConnection;
    }
    let _ = xhr.override_mime_type("text/plain; charset=x-user-defined");
    if xhr.send().is_err() {
        return Outcome::NoConnection;
    }
    let status = xhr.status().unwrap_or(0);
    if status == 0 {
        return Outcome::NoConnection;
    }
    let text = xhr.response_text().ok().flatten().unwrap_or_default();
    let body: Vec<u8> = text.encode_utf16().map(|c| c as u8).collect();
    let length = xhr.get_response_header("content-length").ok().flatten().and_then(|l| l.trim().parse::<u64>().ok());
    Outcome::Response { status, length, body }
}

// ------------------------------------------------------------- QCOMPORT --

/// The tests' scripted ports when the page (or the program's ENVIRON) has
/// RAPIDR_TEST_COMPORT; else none installed: Open asks Web Serial.
fn install_ports() {
    if comport::ports_installed() {
        return;
    }
    let script = rapidr_value::environ::get("RAPIDR_TEST_COMPORT");
    let page = web_sys::window().and_then(|w| js_sys::Reflect::get(&w, &"RAPIDR_TEST_COMPORT".into()).ok()).and_then(|v| v.as_string());
    if let Some(s) = page.or((!script.is_empty()).then_some(script)) {
        comport::set_ports(Rc::new(comport::TestPorts::parse(&s)));
    }
}

/// What a Web Serial port's reader put in, and its writer.
struct WebLink {
    inbox: Rc<RefCell<VecDeque<u8>>>,
    writer: JsValue,
}

impl Link for WebLink {
    fn write(&mut self, bytes: &[u8]) -> Result<(), PortError> {
        let data = js_sys::Uint8Array::from(bytes);
        let write = js_sys::Reflect::get(&self.writer, &"write".into()).map_err(|_| PortError::Other("no writer".into()))?;
        let write: js_sys::Function = write.dyn_into().map_err(|_| PortError::Other("no writer".into()))?;
        write.call1(&self.writer, &data).map(|_| ()).map_err(|e| PortError::Other(format!("{e:?}")))
    }
    fn read(&mut self, max: usize, _wait_ms: u64) -> Result<Vec<u8>, PortError> {
        let mut q = self.inbox.borrow_mut();
        let n = max.min(q.len());
        Ok(q.drain(..n).collect())
    }
    fn in_queue(&mut self) -> usize {
        self.inbox.borrow().len()
    }
    fn purge(&mut self, input: bool, _output: bool) {
        if input {
            self.inbox.borrow_mut().clear();
        }
    }
}

/// QCOMPORT's Open on Web Serial: the page's n-th port for `COMn` (one it
/// was given before), else the browser's chooser (it needs a click: an
/// OnClick handler's Open). No Web Serial, or nothing chosen: the port
/// isn't there.
fn web_serial_open(name: &str) -> Value {
    let Some((port, settings)) = rqlib::comport_open_begin(name) else { return v_null() };
    let serial = web_sys::window().and_then(|w| js_sys::Reflect::get(&w.navigator(), &"serial".into()).ok()).filter(|s| !s.is_undefined());
    let Some(serial) = serial else {
        rqlib::comport_open_end(name, Err(PortError::NotFound));
        return v_null();
    };
    if !crate::dialog_web::wait_task() {
        rqlib::comport_open_end(name, Err(PortError::NotFound));
        return v_null();
    }
    let name = name.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let opened = open_port(&serial, &port, &settings).await;
        rqlib::comport_open_end(&name, opened);
        fire_events(&name);
        crate::dialog_web::task_done(v_null());
    });
    v_null()
}

async fn call_promise(target: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    let f: js_sys::Function = js_sys::Reflect::get(target, &method.into())?.dyn_into()?;
    let p = f.apply(target, &args.iter().cloned().collect::<js_sys::Array>())?;
    JsFuture::from(js_sys::Promise::resolve(&p)).await
}

async fn open_port(serial: &JsValue, port: &str, s: &Settings) -> Result<Box<dyn Link>, PortError> {
    let index = port.trim().to_ascii_uppercase().strip_prefix("COM").and_then(|n| n.parse::<u32>().ok()).unwrap_or(1);
    let granted = call_promise(serial, "getPorts", &[]).await.map(|a| js_sys::Array::from(&a)).unwrap_or_default();
    let device = match granted.get(index.saturating_sub(1)) {
        d if !d.is_undefined() => d,
        _ => call_promise(serial, "requestPort", &[]).await.map_err(|_| PortError::NotFound)?,
    };
    let options = js_sys::Object::new();
    let set = |k: &str, v: JsValue| {
        let _ = js_sys::Reflect::set(&options, &k.into(), &v);
    };
    set("baudRate", s.baud.into());
    set("dataBits", u32::from(s.data_bits.clamp(7, 8)).into());
    set("stopBits", u32::from(s.stop_bits).into());
    set("parity", ["none", "odd", "even"].get(usize::from(s.parity)).copied().unwrap_or("none").into());
    // (XON / XOFF: Web Serial has none)
    set("flowControl", if s.flow == Flow::Hardware { "hardware" } else { "none" }.into());
    call_promise(&device, "open", &[options.into()]).await.map_err(|e| {
        let n = js_sys::Reflect::get(&e, &"name".into()).ok().and_then(|n| n.as_string()).unwrap_or_default();
        if n == "InvalidStateError" || n == "NetworkError" { PortError::AccessDenied } else { PortError::Other(n) }
    })?;
    let readable = js_sys::Reflect::get(&device, &"readable".into()).map_err(|_| PortError::NotFound)?;
    let writable = js_sys::Reflect::get(&device, &"writable".into()).map_err(|_| PortError::NotFound)?;
    let get_writer: js_sys::Function = js_sys::Reflect::get(&writable, &"getWriter".into()).and_then(|f| f.dyn_into()).map_err(|_| PortError::NotFound)?;
    let writer = get_writer.call0(&writable).map_err(|_| PortError::NotFound)?;
    let get_reader: js_sys::Function = js_sys::Reflect::get(&readable, &"getReader".into()).and_then(|f| f.dyn_into()).map_err(|_| PortError::NotFound)?;
    let reader = get_reader.call0(&readable).map_err(|_| PortError::NotFound)?;
    let inbox = Rc::new(RefCell::new(VecDeque::new()));
    let into = inbox.clone();
    wasm_bindgen_futures::spawn_local(async move {
        while let Ok(chunk) = call_promise(&reader, "read", &[]).await {
            if js_sys::Reflect::get(&chunk, &"done".into()).ok().and_then(|d| d.as_bool()).unwrap_or(true) {
                break;
            }
            if let Ok(v) = js_sys::Reflect::get(&chunk, &"value".into()) {
                into.borrow_mut().extend(js_sys::Uint8Array::new(&v).to_vec());
            }
        }
    });
    Ok(Box::new(WebLink { inbox, writer }))
}

/// The page's look at `name` (a QCOMPORT whose OnRxChar the program
/// handles): its events fired; never an OnTimer.
pub fn look(name: &str) {
    // (a media object's Timer: its tick — OnChange)
    if rqlib::media_timer(name).is_some() {
        rqlib::media_tick(name);
        fire_events(name);
        return;
    }
    for (event, args) in rqlib::look(name) {
        rp_fire_event_args(name, event, &args);
    }
}
