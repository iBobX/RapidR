//! RAppCapture and RAppDriver — the program's own eyes and hands as
//! components — for every runtime (runtime-core's and the web's call
//! these): what they answer and do, over [`crate::eyes`] and
//! [`crate::hands`]; and the turn's end every runtime runs ([`deliver`]):
//! RapidR Studio's link answered, the drivers' OnDone fired.

use std::cell::RefCell;
use std::collections::HashMap;

use rapidr_value::{v_int, v_str, Value};
use serde_json::Value as Json;

use crate::eyes::{self, Scale};
use crate::hands::{self, Act, Owner, Target};
use crate::{link, Program, Windows};

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
    static DRIVERS: RefCell<HashMap<String, DriverState>> = RefCell::new(HashMap::new());
    static CAPTURES: RefCell<HashMap<String, CaptureState>> = RefCell::new(HashMap::new());
}

fn text_arg(args: &[Value], i: usize) -> String {
    args.get(i).map(Value::to_string_val).unwrap_or_default()
}

fn num<P: Program>(p: P, name: &str, prop: &str, default: f64) -> f64 {
    match p.get(name, prop) {
        Value::Null => default,
        v => v.to_f64(),
    }
}

/// What ended in the hands since the last turn: RapidR Studio's link
/// answered with `reply` (the request's number, the JSON), the program's
/// drivers' OnDone(Id, Ok, Text) fired.
pub fn deliver<P: Program>(p: P, reply: &dyn Fn(u64, &Json)) {
    let (links, drivers) = link::ends();
    for (seq, r) in links {
        reply(seq, &r);
    }
    for d in drivers {
        if let Owner::Driver(name, id) = &d.owner {
            DRIVERS.with(|m| {
                let mut m = m.borrow_mut();
                let s = m.entry(name.to_lowercase()).or_default();
                s.last = d.text.clone();
                if !d.ok {
                    s.error = d.text.clone();
                }
            });
            p.fire_args(name, "ondone", &[v_int(*id), v_int(if d.ok { -1 } else { 0 }), v_str(&d.text)]);
        }
    }
}

// ------------------------------------------------------------ RAppCapture --

/// RAppCapture's properties the runtime answers.
pub fn capture_get<R: Windows>(rt: R, name: &str, prop: &str) -> Option<Value> {
    let key = name.to_lowercase();
    let state = |f: &dyn Fn(&CaptureState) -> Value| CAPTURES.with(|c| c.borrow().get(&key).map_or(Value::Null, f));
    Some(match prop {
        "error" => match state(&|s| v_str(&s.error)) {
            Value::Null => v_str(""),
            v => v,
        },
        "width" => match state(&|s| v_int(s.size.0 as i64)) {
            Value::Null => v_int(0),
            v => v,
        },
        "height" => match state(&|s| v_int(s.size.1 as i64)) {
            Value::Null => v_int(0),
            v => v,
        },
        "windows" => {
            let mut list = String::new();
            rt.with_desk(&mut |d, _| list = d.stacking().iter().map(|f| format!("{f}\t{}\n", d.forms.get(f).map(|w| w.spec.title.as_str()).unwrap_or(""))).collect());
            v_str(&list)
        }
        _ => return None,
    })
}

fn said(name: &str, error: String, size: (usize, usize)) {
    CAPTURES.with(|c| {
        let mut c = c.borrow_mut();
        let s = c.entry(name.to_lowercase()).or_default();
        s.error = error;
        s.size = size;
    });
}

/// A picture of `window` (`""` / `"all"`: every one side by side) as
/// RAppCapture `name` takes it (its Scale and Frame).
pub fn capture_picture<R: Program + Windows>(rt: R, name: &str, window: &str) -> Result<rapidr_value::objects::codec::Pixels, String> {
    let scale = match num(rt, name, "scale", 0.0) {
        s if s > 0.0 => Scale::Times(s),
        _ => Scale::Screen,
    };
    let framed = match rt.get(name, "frame") {
        Value::Null => true,
        v => v.to_bool(),
    };
    let mut out = None;
    let ok = rt.with_desk(&mut |d, store| {
        out = Some(if window.trim().is_empty() || window.eq_ignore_ascii_case("all") {
            eyes::capture_all(d, store, scale, framed, false)
        } else {
            eyes::find_window(d, window).and_then(|f| eyes::capture(d, store, &f, scale, framed, false))
        });
    });
    if !ok {
        return Err("the program's windows can't be drawn now (none has shown yet)".into());
    }
    out.unwrap_or_else(|| Err("nothing drawn".into()))
}

/// RAppCapture's methods: SaveToFile, LayoutReport, LayoutJson, Problems
/// (`save`: how the runtime writes a file — the desktop's file system, the
/// web's download / file bridge).
pub fn capture_method<R: Program + Windows>(rt: R, name: &str, method: &str, args: &[Value], save: &dyn Fn(&str, &[u8]) -> Result<(), String>) -> Option<Value> {
    Some(match method {
        "savetofile" => {
            let file = text_arg(args, 0);
            if file.trim().is_empty() {
                said(name, "no file name: SaveToFile \"picture.png\"".into(), (0, 0));
                return Some(v_int(0));
            }
            let saved = capture_picture(rt, name, &text_arg(args, 1)).and_then(|px| {
                let bytes = if file.to_ascii_lowercase().ends_with(".bmp") { rapidr_value::objects::codec::encode_bmp(&px) } else { eyes::png(&px)? };
                save(&file, &bytes)?;
                Ok((px.width, px.height))
            });
            match saved {
                Ok(size) => {
                    said(name, String::new(), size);
                    v_int(-1)
                }
                Err(e) => {
                    said(name, e, (0, 0));
                    v_int(0)
                }
            }
        }
        "layoutreport" | "layoutjson" | "problems" => {
            let window = text_arg(args, 0);
            let mut out = None;
            let ok = rt.with_desk(&mut |d, store| {
                out = Some((|| -> Result<Json, String> {
                    let only = if window.trim().is_empty() || window.eq_ignore_ascii_case("all") { None } else { Some(eyes::find_window(d, &window)?) };
                    Ok(eyes::layout(d, store, only.as_deref()))
                })());
            });
            let out = if ok { out.unwrap_or_else(|| Err("nothing laid out".into())) } else { Err("the program's windows can't be looked at now (none has shown yet)".into()) };
            match out {
                Ok(report) => {
                    said(name, String::new(), (0, 0));
                    v_str(&match method {
                        "layoutreport" => eyes::layout_text(&report),
                        "layoutjson" => report.to_string(),
                        _ => report["windows"].as_array().into_iter().flatten().flat_map(|w| w["problems"].as_array().cloned().unwrap_or_default()).filter_map(|p| p.as_str().map(|s| format!("{s}\n"))).collect(),
                    })
                }
                Err(e) => {
                    said(name, e, (0, 0));
                    v_str("")
                }
            }
        }
        _ => return None,
    })
}

// ------------------------------------------------------------- RAppDriver --

/// RAppDriver's properties the runtime answers.
pub fn driver_get(name: &str, prop: &str) -> Option<Value> {
    let key = name.to_lowercase();
    let text = |f: &dyn Fn(&DriverState) -> String| DRIVERS.with(|m| m.borrow().get(&key).map(f).unwrap_or_default());
    Some(match prop {
        "busy" => v_int(if hands::pending_of(&key) > 0 { -1 } else { 0 }),
        "pending" => v_int(hands::pending_of(&key) as i64),
        "lastresult" => v_str(&text(&|s| s.last.clone())),
        "error" => v_str(&text(&|s| s.error.clone())),
        _ => return None,
    })
}

/// RAppDriver's methods: each queues a job and returns its number (OnDone
/// when it ended); Cancel drops the ones waiting.
pub fn driver_method<P: Program>(p: P, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let key = name.to_lowercase();
    let button = rapidr_value::input::Button::Left;
    let t = |i: usize| Target::parse(&text_arg(args, i));
    let n = |i: usize| args.get(i).map(Value::to_f64).unwrap_or(0.0);
    let act = match method {
        "click" => Act::Click { target: t(0), button, double: false },
        "doubleclick" => Act::Click { target: t(0), button, double: true },
        "rightclick" => Act::Click { target: t(0), button: rapidr_value::input::Button::Right, double: false },
        "clickat" => Act::Click { target: Target { window: Some(text_arg(args, 0)), at: Some((n(1), n(2))), ..Target::default() }, button, double: false },
        "hover" => Act::Hover { target: t(0) },
        "typetext" => Act::Type { target: if text_arg(args, 0).trim().is_empty() { Target::default() } else { t(0) }, text: text_arg(args, 1) },
        "presskeys" => Act::Key { target: if text_arg(args, 1).trim().is_empty() { Target::default() } else { t(1) }, keys: text_arg(args, 0) },
        "drag" => Act::Drag { from: t(0), to: Target { at: Some((n(1), n(2))), ..Target::default() } },
        "scroll" => Act::Scroll { target: t(0), dx: 0.0, dy: if args.len() > 1 { n(1) } else { 3.0 } },
        "wait" => Act::Wait { ms: n(0).max(0.0) as u64, idle: false },
        "cancel" => return Some(v_int(hands::cancel_driver(&key) as i64)),
        _ => return None,
    };
    // (its settings, as the program set them, for its jobs)
    hands::set_cursor_shown(match p.get(name, "showcursor") {
        Value::Null => true,
        v => v.to_bool(),
    });
    hands::set_speed(num(p, name, "speed", 1.0));
    let id = DRIVERS.with(|m| {
        let mut m = m.borrow_mut();
        let s = m.entry(key.clone()).or_default();
        s.next += 1;
        s.error.clear();
        s.next
    });
    hands::submit(Owner::Driver(key, id), act);
    Some(v_int(id))
}
