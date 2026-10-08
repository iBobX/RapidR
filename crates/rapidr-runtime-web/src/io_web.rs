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
    if rqlib::is_comport(name) {
        if method == "open" && WEB_SERIAL.with(|w| w.get()) {
            let v = web_serial_open(name);
            fire_events(name);
            return v;
        }
        if method == "filllist" {
            return fill_list(name, args);
        }
        if method == "readline" {
            if let Some(v) = read_line_wait(name, args) {
                return v;
            }
        }
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

thread_local! {
    /// Whether the ports are Web Serial's (else the tests' scripted ones).
    static WEB_SERIAL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The ports the page was allowed (`getPorts`), as ListPorts lists
    /// them: COM1, COM2 … in Web Serial's order, with their USB IDs. Read
    /// again when one is plugged in or out, or allowed.
    static GRANTED: RefCell<Vec<comport::PortInfo>> = const { RefCell::new(Vec::new()) };
}

/// The tests' scripted ports when the page (or the program's ENVIRON) has
/// RAPIDR_TEST_COMPORT; else Web Serial's: Open asks for the port.
fn install_ports() {
    if comport::ports_installed() {
        return;
    }
    let script = rapidr_value::environ::get("RAPIDR_TEST_COMPORT");
    let page = web_sys::window().and_then(|w| js_sys::Reflect::get(&w, &"RAPIDR_TEST_COMPORT".into()).ok()).and_then(|v| v.as_string());
    if let Some(s) = page.or((!script.is_empty()).then_some(script)) {
        comport::set_ports(Rc::new(comport::TestPorts::parse(&s)));
        return;
    }
    WEB_SERIAL.with(|w| w.set(true));
    comport::set_ports(Rc::new(WebPorts));
    let Some(serial) = web_serial() else { return };
    // (plugged in or out: the list read again — OnPortsChanged's look sees it)
    for event in ["connect", "disconnect"] {
        let refresh = Closure::<dyn FnMut()>::new(|| wasm_bindgen_futures::spawn_local(refresh_granted()));
        if let Ok(add) = js_sys::Reflect::get(&serial, &"addEventListener".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
            let _ = add.call2(&serial, &event.into(), refresh.as_ref().unchecked_ref());
        }
        refresh.forget();
    }
    wasm_bindgen_futures::spawn_local(refresh_granted());
}

fn web_serial() -> Option<JsValue> {
    web_sys::window().and_then(|w| js_sys::Reflect::get(&w.navigator(), &"serial".into()).ok()).filter(|s| !s.is_undefined())
}

/// The ports the page was allowed, read again (GRANTED).
async fn refresh_granted() {
    let Some(serial) = web_serial() else { return };
    let granted = call_promise(&serial, "getPorts", &[]).await.map(|a| js_sys::Array::from(&a)).unwrap_or_default();
    let mut list = Vec::new();
    for (i, port) in granted.iter().enumerate() {
        let info = js_sys::Reflect::get(&port, &"getInfo".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok()).and_then(|f| f.call0(&port).ok());
        let id = |k: &str| info.as_ref().and_then(|i| js_sys::Reflect::get(i, &k.into()).ok()).and_then(|v| v.as_f64()).unwrap_or(0.0) as u16;
        list.push(comport::PortInfo { name: format!("COM{}", i + 1), vid: id("usbVendorId"), pid: id("usbProductId"), ..comport::PortInfo::default() });
    }
    GRANTED.with(|g| *g.borrow_mut() = list);
}

/// Web Serial's ports: listed from GRANTED; opened by `web_serial_open`
/// (it waits for the browser), never here.
struct WebPorts;

impl comport::Ports for WebPorts {
    fn open(&self, _port: &str, _settings: &Settings) -> Result<Box<dyn Link>, PortError> {
        Err(PortError::NotFound)
    }
    fn list(&self) -> Vec<comport::PortInfo> {
        GRANTED.with(|g| g.borrow().clone())
    }
}

/// An open Web Serial port: what its reader put in, its writer, the lines
/// its look last read.
struct WebLink {
    device: JsValue,
    reader: JsValue,
    writer: JsValue,
    inbox: Rc<RefCell<VecDeque<u8>>>,
    signals: Rc<std::cell::Cell<comport::Signals>>,
    open: Rc<std::cell::Cell<bool>>,
}

/// `port.setSignals({…})`, not waited for (Web Serial runs them in order).
fn set_signals(device: &JsValue, key: &str, on: bool) -> Result<(), PortError> {
    let o = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&o, &key.into(), &on.into());
    let f: js_sys::Function = js_sys::Reflect::get(device, &"setSignals".into()).and_then(|f| f.dyn_into()).map_err(|_| PortError::InvalidParameter)?;
    f.call1(device, &o).map(|_| ()).map_err(|_| PortError::InvalidParameter)
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
    fn set_dtr(&mut self, on: bool) -> Result<(), PortError> {
        set_signals(&self.device, "dataTerminalReady", on)
    }
    fn set_rts(&mut self, on: bool) -> Result<(), PortError> {
        set_signals(&self.device, "requestToSend", on)
    }
    fn signals(&mut self) -> comport::Signals {
        self.signals.get()
    }
    fn send_break(&mut self, ms: u64) -> Result<(), PortError> {
        set_signals(&self.device, "break", true)?;
        let device = self.device.clone();
        wasm_bindgen_futures::spawn_local(async move {
            sleep_ms(ms as i32).await;
            let _ = set_signals(&device, "break", false);
        });
        Ok(())
    }
    fn line_length(&mut self, end: &[u8], _wait_ms: u64) -> Option<usize> {
        comport::find_end(self.inbox.borrow().iter(), end)
    }
}

impl Drop for WebLink {
    /// Close: the reader and writer let go, the port closed (so it can be
    /// opened again, here or by another program).
    fn drop(&mut self) {
        self.open.set(false);
        let (device, reader, writer) = (self.device.clone(), self.reader.clone(), self.writer.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let _ = call_promise(&reader, "cancel", &[]).await;
            let _ = call_promise(&reader, "releaseLock", &[]).await;
            let _ = call_promise(&writer, "releaseLock", &[]).await;
            let _ = call_promise(&device, "close", &[]).await;
        });
    }
}

/// A promise of `ms` milliseconds (setTimeout).
async fn sleep_ms(ms: i32) {
    let p = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms.max(0));
        }
    });
    let _ = JsFuture::from(p).await;
}

/// QCOMPORT's Open on Web Serial: the page's n-th port for `COMn` (one it
/// was given before), else the browser's chooser (it needs a click: an
/// OnClick handler's Open). No Web Serial, or nothing chosen: the port
/// isn't there.
fn web_serial_open(name: &str) -> Value {
    let Some((port, settings)) = rqlib::comport_open_begin(name) else { return v_null() };
    let Some(serial) = web_serial() else {
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
        refresh_granted().await;
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
    // (a port this page closed a moment ago may still be closing: a few
    // tries)
    let mut tries = 0;
    loop {
        match call_promise(&device, "open", &[options.clone().into()]).await {
            Ok(_) => break,
            Err(e) => {
                let n = js_sys::Reflect::get(&e, &"name".into()).ok().and_then(|n| n.as_string()).unwrap_or_default();
                tries += 1;
                if n == "InvalidStateError" && tries < 10 {
                    sleep_ms(50).await;
                    continue;
                }
                return Err(if n == "InvalidStateError" || n == "NetworkError" { PortError::AccessDenied } else { PortError::Other(n) });
            }
        }
    }
    let readable = js_sys::Reflect::get(&device, &"readable".into()).map_err(|_| PortError::NotFound)?;
    let writable = js_sys::Reflect::get(&device, &"writable".into()).map_err(|_| PortError::NotFound)?;
    let get_writer: js_sys::Function = js_sys::Reflect::get(&writable, &"getWriter".into()).and_then(|f| f.dyn_into()).map_err(|_| PortError::NotFound)?;
    let writer = get_writer.call0(&writable).map_err(|_| PortError::NotFound)?;
    let get_reader: js_sys::Function = js_sys::Reflect::get(&readable, &"getReader".into()).and_then(|f| f.dyn_into()).map_err(|_| PortError::NotFound)?;
    let reader = get_reader.call0(&readable).map_err(|_| PortError::NotFound)?;
    let inbox = Rc::new(RefCell::new(VecDeque::new()));
    let open = Rc::new(std::cell::Cell::new(true));
    let signals = Rc::new(std::cell::Cell::new(comport::Signals::default()));
    let (into, reading) = (inbox.clone(), reader.clone());
    wasm_bindgen_futures::spawn_local(async move {
        while let Ok(chunk) = call_promise(&reading, "read", &[]).await {
            if js_sys::Reflect::get(&chunk, &"done".into()).ok().and_then(|d| d.as_bool()).unwrap_or(true) {
                break;
            }
            if let Ok(v) = js_sys::Reflect::get(&chunk, &"value".into()) {
                into.borrow_mut().extend(js_sys::Uint8Array::new(&v).to_vec());
            }
        }
    });
    // (the lines the other end drives: `getSignals` is a promise, so read
    // every 100 ms while the port is open)
    let (watched, still, lines) = (device.clone(), open.clone(), signals.clone());
    wasm_bindgen_futures::spawn_local(async move {
        while still.get() {
            if let Ok(s) = call_promise(&watched, "getSignals", &[]).await {
                let on = |k: &str| js_sys::Reflect::get(&s, &k.into()).ok().and_then(|v| v.as_bool()).unwrap_or(false);
                lines.set(comport::Signals { cts: on("clearToSend"), dsr: on("dataSetReady"), cd: on("dataCarrierDetect"), ri: on("ringIndicator") });
            }
            sleep_ms(100).await;
        }
    });
    Ok(Box::new(WebLink { device, reader, writer, inbox, signals, open }))
}

/// ReadLine(Timeout) with no whole line there yet: the program waits (the
/// page paints, its timers tick) until one arrives or the time is up.
/// `None`: nothing to wait for — the call answers at once.
fn read_line_wait(name: &str, args: &[Value]) -> Option<Value> {
    let timeout = match args.first() {
        None | Some(Value::Null) => 1000.0,
        Some(v) => v.to_f64(),
    };
    let connected = rqlib::get(name, "connected").is_some_and(|v| v.to_i64() != 0);
    if timeout <= 0.0 || !connected || rqlib::comport_has_line(name) || !crate::dialog_web::wait_task() {
        return None;
    }
    let name = name.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let until = js_sys::Date::now() + timeout;
        while js_sys::Date::now() < until && !rqlib::comport_has_line(&name) {
            sleep_ms(10).await;
        }
        let line = rapidr_value::objects::call(&name, "readline", &[v_int(0)], &|id, p| crate::object_web::rp_comp_get(id, p)).and_then(Result::ok).unwrap_or_else(|| v_str(""));
        fire_events(&name);
        crate::dialog_web::task_done(line);
    });
    Some(v_null())
}

/// FillList(Control): the ports put in a list or combo box (cleared first;
/// Port's selected, else the first) — as the desktop's `io::fill_list`.
fn fill_list(name: &str, args: &[Value]) -> Value {
    let Some(items) = rqlib::comport_port_items(name) else { return v_int(0) };
    let control = args.first().map(Value::to_string_val).unwrap_or_default();
    if control.is_empty() || rp_comp_type(&control).is_empty() {
        return v_int(items.len() as i64);
    }
    rp_comp_method(&control, "clear", &[]);
    for item in &items {
        rp_comp_method(&control, "additems", &[v_str(item)]);
    }
    let port = rqlib::get(name, "port").map(|v| v.to_string_val()).unwrap_or_default();
    let at = items.iter().position(|i| i == &port || i.starts_with(&format!("{port} ("))).unwrap_or(0);
    if !items.is_empty() {
        rp_comp_set(&control, "itemindex", v_int(at as i64));
    }
    v_int(items.len() as i64)
}

/// The page's look at `name` (a QCOMPORT whose OnRxChar, OnLine or
/// OnPortsChanged the program handles): its events fired; never an OnTimer.
pub fn look(name: &str) {
    // (a media object's Timer: its tick — OnChange)
    if rqlib::media_timer(name).is_some() {
        rqlib::media_tick(name);
        fire_events(name);
        return;
    }
    let handled = |e: &str| crate::object_web::rp_has_handler(name, e);
    for (event, args) in rqlib::look(name, &handled) {
        rp_fire_event_args(name, event, &args);
    }
}
