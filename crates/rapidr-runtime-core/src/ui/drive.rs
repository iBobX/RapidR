//! The program's eyes and hands on the desktop (`rapidr_ui_app::{eyes,
//! hands, link}`, shared with the web runtime): RapidR Studio's link — the
//! session's `app` requests, which its reader thread hands over
//! ([`push`]) — and the program's own RAppCapture and RAppDriver.
//!
//! Everything runs on the program's thread, in the window loop's turn
//! ([`turn`], from `kernel::step`): requests looked at, the hands' jobs
//! played, what ended answered (the link's reply, a driver's OnDone).
//! While the debugger holds the program ([`turn_paused`], from its paused
//! wait) only looking is served: input is refused with where it is
//! paused. A program busy in its own code (a long loop with no DOEVENTS)
//! answers when it next waits for its events.

use std::collections::VecDeque;
use std::sync::Mutex;

use rapidr_ui_app::{hands, link};
use serde_json::{json, Value as Json};

use super::kernel_store::RtStore;
use super::program::Rt;
use crate::value::{v_int, v_str, Value};

/// How RapidR Studio is answered: the request's number and the reply
/// (JSON), from the program's thread.
pub type Reply = Box<dyn Fn(u64, String) + Send>;

static INBOX: Mutex<VecDeque<(u64, String)>> = Mutex::new(VecDeque::new());
static REPLY: Mutex<Option<Reply>> = Mutex::new(None);
/// Where the debugger holds the program, in words ("at line 12 of
/// calc.rr"), while it does.
static PAUSED_AT: Mutex<Option<String>> = Mutex::new(None);

/// RapidR Studio's link is up (a session): its replies go to `reply`.
pub fn set_link(reply: Reply) {
    if let Ok(mut r) = REPLY.lock() {
        *r = Some(reply);
    }
}

/// Whether a session's link is up (a headless program under one keeps
/// waiting for its requests rather than ending as nothing is scheduled).
pub fn linked() -> bool {
    REPLY.lock().map(|r| r.is_some()).unwrap_or(false)
}

/// A request from RapidR Studio (any thread: the session's reader); the
/// window loop is woken to serve it.
pub fn push(seq: u64, request: String) {
    if let Ok(mut q) = INBOX.lock() {
        q.push_back((seq, request));
    }
    crate::object::rp_wake();
}

/// The debugger holds the program `at` ("at line 12 of calc.rr"), or let
/// it go on (`None`).
pub fn set_paused(at: Option<String>) {
    if let Ok(mut p) = PAUSED_AT.lock() {
        *p = at;
    }
}

fn paused_at() -> Option<String> {
    PAUSED_AT.lock().ok().and_then(|p| p.clone())
}

fn answer(seq: u64, reply: &Json) {
    if let Ok(r) = REPLY.lock() {
        if let Some(r) = r.as_ref() {
            r(seq, reply.to_string());
        }
    }
}

fn take_inbox() -> Vec<(u64, String)> {
    INBOX.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
}

fn serve(paused: Option<&str>) {
    for (seq, text) in take_inbox() {
        let reply = match serde_json::from_str::<Json>(&text) {
            Ok(j) => link::request(Rt, seq, &j, paused),
            Err(e) => Some(json!({"ok": false, "text": format!("The request isn't JSON: {e}")})),
        };
        if let Some(r) = reply {
            answer(seq, &r);
        }
    }
}

/// What ended: the link's answered, the drivers' OnDone fired.
fn deliver() {
    let (links, drivers) = link::ends();
    for (seq, r) in links {
        answer(seq, &r);
    }
    for d in drivers {
        if let hands::Owner::Driver(name, id) = &d.owner {
            DRIVER.with(|m| {
                let mut m = m.borrow_mut();
                let s = m.entry(name.to_lowercase()).or_default();
                s.last = d.text.clone();
                if !d.ok {
                    s.error = d.text.clone();
                }
            });
            crate::object::rp_fire_event_args(name, "ondone", &[v_int(*id), v_int(if d.ok { -1 } else { 0 }), v_str(&d.text)]);
        }
    }
}

/// The window loop's turn: requests served, the hands' jobs played, what
/// ended delivered.
pub fn turn() {
    let paused = paused_at();
    serve(paused.as_deref());
    if paused.is_none() {
        hands::step(Rt);
    }
    deliver();
}

/// A turn while the debugger holds the program: looking is served (its
/// windows as they are), input refused; nothing of it runs.
pub fn turn_paused() {
    let at = paused_at().unwrap_or_else(|| "where it stopped".into());
    serve(Some(&at));
    let (links, _) = link::ends();
    for (seq, r) in links {
        answer(seq, &r);
    }
}

// ------------------------------------------------- RAppCapture, RAppDriver --

#[derive(Default)]
struct DriverState {
    last: String,
    error: String,
    next: i64,
}

#[derive(Default)]
struct CaptureState {
    error: String,
    size: (usize, usize),
}

thread_local! {
    static DRIVER: std::cell::RefCell<std::collections::HashMap<String, DriverState>> = std::cell::RefCell::new(std::collections::HashMap::new());
    static CAPTURE: std::cell::RefCell<std::collections::HashMap<String, CaptureState>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

fn text_arg(args: &[Value], i: usize) -> String {
    args.get(i).map(Value::to_string_val).unwrap_or_default()
}

fn prop_num(name: &str, prop: &str, default: f64) -> f64 {
    match crate::object::rp_comp_get(name, prop) {
        Value::Null => default,
        v => v.to_f64(),
    }
}

fn prop_flag(name: &str, prop: &str, default: bool) -> bool {
    match crate::object::rp_comp_get(name, prop) {
        Value::Null => default,
        v => v.to_bool(),
    }
}

/// Runs `f` on the program's windows (the host's forms) now.
fn on_desk<R>(mut f: impl FnMut(&mut rapidr_ui_app::desktop::Desktop, &dyn rapidr_ui_kernel::Store) -> R) -> Option<R> {
    super::kernel::with_desk_now(&mut |d| f(d, &RtStore))
}

/// RAppCapture's properties the runtime answers.
pub fn capture_get(name: &str, prop: &str) -> Option<Value> {
    let key = name.to_lowercase();
    match prop {
        "error" => Some(v_str(&CAPTURE.with(|c| c.borrow().get(&key).map(|s| s.error.clone()).unwrap_or_default()))),
        "width" => Some(v_int(CAPTURE.with(|c| c.borrow().get(&key).map_or(0, |s| s.size.0 as i64)))),
        "height" => Some(v_int(CAPTURE.with(|c| c.borrow().get(&key).map_or(0, |s| s.size.1 as i64)))),
        "windows" => Some(v_str(&on_desk(|d, _| d.stacking().iter().map(|f| format!("{f}\t{}\n", d.forms.get(f).map(|w| w.spec.title.as_str()).unwrap_or(""))).collect::<String>()).unwrap_or_default())),
        _ => None,
    }
}

fn capture_said(name: &str, error: String, size: (usize, usize)) {
    CAPTURE.with(|c| {
        let mut c = c.borrow_mut();
        let s = c.entry(name.to_lowercase()).or_default();
        s.error = error;
        s.size = size;
    });
}

/// RAppCapture's methods: SaveToFile, LayoutReport, LayoutJson, Problems.
pub fn capture_method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    use rapidr_ui_app::eyes::{self, Scale};
    let scale = match prop_num(name, "scale", 0.0) {
        s if s > 0.0 => Scale::Times(s),
        _ => Scale::Screen,
    };
    let framed = prop_flag(name, "frame", true);
    Some(match method {
        "savetofile" => {
            let file = text_arg(args, 0);
            let window = text_arg(args, 1);
            if file.trim().is_empty() {
                capture_said(name, "no file name: SaveToFile \"picture.png\"".into(), (0, 0));
                return Some(v_int(0));
            }
            let picture = on_desk(|d, store| -> Result<rapidr_value::objects::codec::Pixels, String> {
                if window.trim().is_empty() || window.eq_ignore_ascii_case("all") {
                    eyes::capture_all(d, store, scale, framed, false)
                } else {
                    let f = eyes::find_window(d, &window)?;
                    eyes::capture(d, store, &f, scale, framed, false)
                }
            })
            .unwrap_or_else(|| Err("the program's windows can't be drawn now".into()));
            let saved = picture.and_then(|px| {
                let bytes = if file.to_ascii_lowercase().ends_with(".bmp") { rapidr_value::objects::codec::encode_bmp(&px) } else { eyes::png(&px)? };
                std::fs::write(&file, bytes).map_err(|e| format!("can't write {file}: {e}"))?;
                Ok((px.width, px.height))
            });
            match saved {
                Ok(size) => {
                    capture_said(name, String::new(), size);
                    v_int(-1)
                }
                Err(e) => {
                    capture_said(name, e, (0, 0));
                    v_int(0)
                }
            }
        }
        "layoutreport" | "layoutjson" | "problems" => {
            let window = text_arg(args, 0);
            let out = on_desk(|d, store| -> Result<Json, String> {
                let only = if window.trim().is_empty() || window.eq_ignore_ascii_case("all") { None } else { Some(eyes::find_window(d, &window)?) };
                Ok(eyes::layout(d, store, only.as_deref()))
            })
            .unwrap_or_else(|| Err("the program's windows can't be looked at now".into()));
            match out {
                Ok(report) => {
                    capture_said(name, String::new(), (0, 0));
                    v_str(&match method {
                        "layoutreport" => eyes::layout_text(&report),
                        "layoutjson" => report.to_string(),
                        _ => report["windows"].as_array().into_iter().flatten().flat_map(|w| w["problems"].as_array().cloned().unwrap_or_default()).filter_map(|p| p.as_str().map(|s| format!("{s}\n"))).collect(),
                    })
                }
                Err(e) => {
                    capture_said(name, e, (0, 0));
                    v_str("")
                }
            }
        }
        _ => return None,
    })
}

/// RAppDriver's properties the runtime answers.
pub fn driver_get(name: &str, prop: &str) -> Option<Value> {
    let key = name.to_lowercase();
    match prop {
        "busy" => Some(v_int(if hands::pending_of(&key) > 0 { -1 } else { 0 })),
        "pending" => Some(v_int(hands::pending_of(&key) as i64)),
        "lastresult" => Some(v_str(&DRIVER.with(|m| m.borrow().get(&key).map(|s| s.last.clone()).unwrap_or_default()))),
        "error" => Some(v_str(&DRIVER.with(|m| m.borrow().get(&key).map(|s| s.error.clone()).unwrap_or_default()))),
        _ => None,
    }
}

/// RAppDriver's methods: each queues a job (its number; OnDone(Id, Ok,
/// Result) when it ended), Cancel drops the ones waiting.
pub fn driver_method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    use hands::{Act, Target};
    let key = name.to_lowercase();
    let button = rapidr_value::input::Button::Left;
    let t = |i: usize| Target::parse(&text_arg(args, i));
    let num = |i: usize| args.get(i).map(Value::to_f64).unwrap_or(0.0);
    let act = match method {
        "click" => Act::Click { target: t(0), button, double: false },
        "doubleclick" => Act::Click { target: t(0), button, double: true },
        "rightclick" => Act::Click { target: t(0), button: rapidr_value::input::Button::Right, double: false },
        "clickat" => Act::Click { target: Target { window: Some(text_arg(args, 0)), at: Some((num(1), num(2))), ..Target::default() }, button, double: false },
        "hover" => Act::Hover { target: t(0) },
        "typetext" => {
            let target = if text_arg(args, 0).trim().is_empty() { Target::default() } else { t(0) };
            Act::Type { target, text: text_arg(args, 1) }
        }
        "presskeys" => {
            let target = if text_arg(args, 1).trim().is_empty() { Target::default() } else { t(1) };
            Act::Key { target, keys: text_arg(args, 0) }
        }
        "drag" => {
            let from = t(0);
            Act::Drag { from, to: Target { at: Some((num(1), num(2))), ..Target::default() } }
        }
        "scroll" => Act::Scroll { target: t(0), dx: 0.0, dy: if args.len() > 1 { num(1) } else { 3.0 } },
        "wait" => Act::Wait { ms: num(0).max(0.0) as u64, idle: false },
        "cancel" => {
            return Some(v_int(hands::cancel_driver(&key) as i64));
        }
        _ => return None,
    };
    // (its settings, as the program set them, for its jobs)
    hands::set_cursor_shown(prop_flag(name, "showcursor", true));
    hands::set_speed(prop_num(name, "speed", 1.0));
    let id = DRIVER.with(|m| {
        let mut m = m.borrow_mut();
        let s = m.entry(key.clone()).or_default();
        s.next += 1;
        s.error.clear();
        s.next
    });
    hands::submit(hands::Owner::Driver(key, id), act);
    crate::object::rp_wake();
    Some(v_int(id))
}
