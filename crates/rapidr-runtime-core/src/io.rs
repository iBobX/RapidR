//! The I/O and media objects on the desktop (rapidr_value::objects::rqlib;
//! docs/io-media-plan.md): their devices installed when the first is made,
//! the events their models leave fired, QDOWNLOAD's transfer, the looks
//! the runtime takes for QCOMPORT's OnRxChar (as for a joystick's events:
//! `directx::timer_fired`), and QVIDEO's window — a QCANVAS the frames are
//! drawn on, paced by a timer of its own while it plays.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rapidr_value::objects::download::Outcome;
use rapidr_value::objects::{media, rqlib};

use crate::object::{rp_comp_get, rp_comp_set, rp_comp_type, rp_fire_event_args};
use crate::value::{v_bool, v_int, v_str, Value};

/// Object `name` of type `type_name` was made: its device, ready.
pub fn created(name: &str, type_name: &str) {
    match type_name.to_ascii_uppercase().as_str() {
        "RCOMPORT" => crate::serial::install(),
        "RMIDI" | "RWAVE" | "RCDAUDIO" => crate::media::install(),
        // (its frames paced while it plays: a timer of its own)
        "RVIDEO" => {
            crate::media::install();
            #[cfg(feature = "gui")]
            crate::ui::gui_register_timer(&rqlib::frames_timer(name));
        }
        // (its QGAUGE and QLABEL: components of their own, `name.StateGauge`
        // and `name.SpeedLbl`, which the program places — the library's
        // StateGauge was 200 × 20)
        "RDOWNLOAD" => {
            let gauge = format!("{name}.stategauge");
            crate::object::rp_create_component(&gauge, "RPROGRESSBAR");
            rp_comp_set(&gauge, "width", v_int(200));
            rp_comp_set(&gauge, "height", v_int(20));
            crate::object::rp_create_component(&format!("{name}.speedlbl"), "RLABEL");
        }
        _ => {}
    }
}

/// The events object `name`'s model left, fired (after each of its calls).
pub fn fire_events(name: &str) {
    // (a media object's Timer turned on or off by Play, Stop …)
    #[cfg(feature = "gui")]
    if rqlib::take_timer_changed(name) {
        crate::ui::gui_timer_changed(name);
    }
    if rqlib::is_video(name) {
        video_changed(name);
    }
    for (event, args) in rqlib::take_events(name) {
        rp_fire_event_args(name, event, &args);
    }
}

/// QVIDEO `name` after a call: its window's components follow the model
/// (`media::VideoWindow`), its frames timer starts or stops, and the
/// frame it's at is shown.
fn video_changed(name: &str) {
    #[cfg(feature = "gui")]
    if rqlib::take_frames_changed(name) {
        crate::ui::gui_timer_changed(&rqlib::frames_timer(name));
    }
    if let Some(w) = rqlib::take_video_window(name) {
        video_window(name, &w);
    }
    video_frame(name);
}

/// QVIDEO `name`'s window as its components: `<name>.screen`, a QCANVAS
/// (black) the frames are drawn on — on Parent's form (a child window at
/// Left / Top, Width × Height), or filling `<name>.window`, a QFORM of its
/// own (no frame for a popup) — made at the first Open, hidden by Close.
fn video_window(name: &str, w: &media::VideoWindow) {
    let (screen, form) = (media::VideoWindow::screen(name), media::VideoWindow::form(name));
    let own = w.parent.is_empty();
    if !w.open {
        if !rp_comp_type(&screen).is_empty() {
            rp_comp_set(&screen, "visible", v_bool(false));
        }
        if rp_comp_type(&form) == "RFORM" {
            crate::object::rp_comp_method(&form, "close", &[]);
        }
        return;
    }
    if rp_comp_type(&screen).is_empty() {
        crate::object::rp_create_component(&screen, "RCANVAS");
        rp_comp_set(&screen, "color", v_int(0));
    }
    if own {
        if rp_comp_type(&form).is_empty() {
            crate::object::rp_create_component(&form, "RFORM");
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
            crate::object::rp_comp_method(&form, "show", &[]);
        }
    } else {
        if rp_comp_type(&form) == "RFORM" {
            crate::object::rp_comp_method(&form, "close", &[]);
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
        #[cfg(feature = "gui")]
        crate::ui::canvas_redraw(&screen);
    }
}

/// `name` is a QVIDEO's frames timer: its Enabled (frames due) and
/// Interval (the file's time a frame), as the runtime's timers read them.
pub fn frames_timer_get(name: &str, prop: &str) -> Option<Value> {
    if !matches!(prop, "enabled" | "interval") || !name.ends_with(".frames") {
        return None;
    }
    let (_, ms, due) = rqlib::video_frames(name)?;
    Some(if prop == "enabled" { v_bool(due) } else { v_int(ms) })
}

/// A QVIDEO's frames timer fired: the frame the clock is at shown.
#[cfg(feature = "gui")]
pub fn frames_tick(name: &str) -> bool {
    let Some((video, _, _)) = rqlib::video_frames(name) else { return false };
    video_frame(&video);
    true
}

/// `QDOWNLOAD.StateGauge.Parent = Form`: a member of its gauge or label.
pub fn sub_component(name: &str, prop: &str) -> Option<(String, String)> {
    if !rqlib::is_download(name) {
        return None;
    }
    let (sub, member) = prop.split_once('.')?;
    matches!(sub, "stategauge" | "speedlbl").then(|| (format!("{}.{sub}", name.to_lowercase()), member.to_string()))
}

/// What a transfer has done so far (the thread's, read by the wait).
#[derive(Default)]
struct Transfer {
    received: u64,
    length: Option<u64>,
    done: Option<Outcome>,
}

/// LeechFile: the library's checks, then the file fetched while the
/// program waits (its windows paint, its timers tick); True (1) or False
/// (0) with LastError / LastStringError.
pub fn leech_file(name: &str) -> Value {
    let request = match rqlib::download_begin(name, &crate::builtins::rp_time().to_string_val()) {
        Ok(r) => r,
        Err(result) => return v_int(result),
    };
    let shared = Arc::new(Mutex::new(Transfer::default()));
    let t = shared.clone();
    let spawned = std::thread::Builder::new().name("rapidr-download".into()).spawn(move || fetch(&request.url, &t));
    if spawned.is_err() {
        shared.lock().unwrap().done = Some(Outcome::NoConnection);
    }
    let name = name.to_string();
    let mut shown = (u64::MAX, None);
    let poll = move || -> Option<Value> {
        let (received, length, done) = {
            let mut t = shared.lock().unwrap();
            (t.received, t.length, t.done.take())
        };
        if (received, length) != shown {
            shown = (received, length);
            show_progress(&name, received, length);
        }
        let outcome = done?;
        let (result, file) = rqlib::download_finish(&name, outcome);
        if let Some((path, bytes)) = file {
            // (the library's QFILESTREAM fmCreate: the file made or emptied)
            if let Err(e) = std::fs::write(&path, &bytes) {
                eprintln!("[rapidr] QDOWNLOAD: can't write {path}: {e}");
            }
        }
        Some(v_int(result))
    };
    #[cfg(feature = "gui")]
    {
        crate::ui::gui_wait_task(poll)
    }
    #[cfg(not(feature = "gui"))]
    {
        let mut poll = poll;
        loop {
            if let Some(v) = poll() {
                return v;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn show_progress(name: &str, received: u64, length: Option<u64>) {
    use rapidr_value::objects::download::Shown;
    if let Some(Shown::Gauge { position, speed }) = rqlib::download_progress(name, received, length, &crate::builtins::rp_time().to_string_val()) {
        rp_comp_set(&format!("{name}.stategauge"), "position", v_int(position));
        rp_comp_set(&format!("{name}.speedlbl"), "caption", v_str(&speed));
    }
}

/// The GET, on the transfer's thread: the response's status, length and
/// body, the progress counted as it comes.
fn fetch(url: &str, t: &Mutex<Transfer>) {
    let outcome = fetch_inner(url, t);
    t.lock().unwrap().done = Some(outcome);
}

#[cfg(feature = "network")]
fn fetch_inner(url: &str, t: &Mutex<Transfer>) -> Outcome {
    use std::io::Read;
    let agent = crate::network::http_agent(Duration::from_secs(60));
    let response = match agent.get(url).call() {
        Ok(r) => r,
        // (any status: the library took what came; a 404 is its error 11)
        Err(ureq::Error::Status(_, r)) => r,
        Err(ureq::Error::Transport(e)) => {
            return match e.kind() {
                ureq::ErrorKind::Dns | ureq::ErrorKind::ConnectionFailed | ureq::ErrorKind::InvalidUrl | ureq::ErrorKind::UnknownScheme => Outcome::NoConnection,
                _ => Outcome::Closed,
            }
        }
    };
    let status = response.status();
    let length = response.header("content-length").and_then(|l| l.trim().parse::<u64>().ok());
    t.lock().unwrap().length = length;
    let mut body = Vec::new();
    let mut reader = response.into_reader().take(1 << 31);
    let mut buf = [0u8; 32000];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                body.extend_from_slice(&buf[..n]);
                t.lock().unwrap().received = body.len() as u64;
            }
            Err(_) => return Outcome::Closed,
        }
    }
    Outcome::Response { status, length, body }
}

#[cfg(not(feature = "network"))]
fn fetch_inner(_url: &str, _t: &Mutex<Transfer>) -> Outcome {
    Outcome::NoConnection
}

/// Whether `name` is looked at like a timer (a QCOMPORT whose OnRxChar the
/// program handles), and how often.
#[cfg(feature = "gui")]
pub fn look_interval(name: &str) -> Option<Duration> {
    // (a QVIDEO's frames: the file's time a frame)
    if let Some((_, ms, _)) = rqlib::video_frames(name) {
        return Some(Duration::from_millis(ms as u64));
    }
    // (a media object's Timer: its Interval)
    if let Some((interval, _)) = rqlib::media_timer(name) {
        return Some(Duration::from_millis(if interval > 0 { interval as u64 } else { 1000 }));
    }
    rqlib::is_comport(name).then(|| Duration::from_millis(rapidr_value::objects::rqlib::LOOK_MS))
}

/// The runtime's look at `name`: its events fired (never an OnTimer).
#[cfg(feature = "gui")]
pub fn look(name: &str) -> bool {
    // (a media object's Timer: its tick — OnChange)
    if rqlib::media_timer(name).is_some() {
        rqlib::media_tick(name);
        fire_events(name);
        return false;
    }
    if rqlib::look_events().iter().any(|e| crate::object::rp_has_handler(name, e)) {
        for (event, args) in rqlib::look(name) {
            rp_fire_event_args(name, event, &args);
        }
    }
    false
}
