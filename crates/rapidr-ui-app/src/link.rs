//! RapidR Studio's link to the program it runs, for its eyes and hands
//! (`app_screenshot`, `app_layout`, `app_click` … — RapidR Studio's AI apps
//! and RAI): the session's `app` requests (rapidr-session's protocol: the
//! desktop's standard input, the web's frame port) as JSON, served by the
//! runtime on the program's own thread, the same on every host.
//!
//! ```json
//! {"do":"screenshot","window":"","scale":"screen","frame":true}
//! {"do":"layout","window":"calc"}
//! {"do":"click","caption":"7"}            (or "name", "label", or "window" + "x", "y")
//! {"do":"type","label":"Name","text":"Ada"}
//! {"do":"key","keys":"Ctrl+S"}
//! {"do":"drag","name":"slider","to_x":180,"to_y":20}
//! {"do":"scroll","name":"list","dy":3}
//! {"do":"wait","ms":500}  {"do":"wait","idle":true,"ms":5000}
//! {"do":"cursor","show":false}
//! ```
//!
//! A reply is `{"ok":…, "text":…}` — a sentence for a model or a person —,
//! with the report (`layout`) or the picture (`png`, base64, and its
//! `width` and `height`). A look (screenshot, layout) is answered at once,
//! paused or not; input is played as the hands' job and answered when the
//! handlers it started have run ([`ends`]) — refused while the program
//! is paused in the debugger, with where it is and what to do.

use serde_json::{json, Value as Json};

use crate::eyes::{self, Scale};
use crate::hands::{self, Act, Owner, Target};
use crate::{Program, Windows};

/// A field as text, as written: a number or true / false too (a caption
/// "7" a tool's JSON builder made a number).
fn raw(j: &Json, k: &str) -> Option<String> {
    match j.get(k)? {
        Json::String(t) => Some(t.clone()),
        Json::Number(n) => Some(n.to_string()),
        Json::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// [`raw`], trimmed, `None` when empty.
fn s(j: &Json, k: &str) -> Option<String> {
    raw(j, k).map(|t| t.trim().to_string()).filter(|v| !v.is_empty())
}

fn n(j: &Json, k: &str) -> Option<f64> {
    j.get(k).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|t| t.trim().parse().ok())))
}

fn b(j: &Json, k: &str, default: bool) -> bool {
    match j.get(k) {
        Some(Json::Bool(v)) => *v,
        Some(Json::String(t)) => !matches!(t.trim().to_ascii_lowercase().as_str(), "false" | "0" | "no" | ""),
        Some(Json::Number(v)) => v.as_f64() != Some(0.0),
        _ => default,
    }
}

/// The target a request names: `name`, `caption`, `label`, or `window` +
/// `x`, `y` (`prefix` for a drag's end: `to_x` …).
pub fn target_of(j: &Json, prefix: &str) -> Target {
    let k = |key: &str| format!("{prefix}{key}");
    let at = match (n(j, &k("x")), n(j, &k("y"))) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    };
    Target { name: s(j, &k("name")), caption: s(j, &k("caption")), label: s(j, &k("label")), window: s(j, &k("window")).or_else(|| s(j, "window")), at }
}

fn fail(text: impl Into<String>) -> Json {
    json!({"ok": false, "text": text.into()})
}

/// What the program shows, as a picture: `window` (`""` / `"all"`: every
/// one side by side), its `scale`, `frame`d or not, the virtual `cursor`
/// in it or not. The reply's `png` is base64.
pub fn screenshot<R: Windows>(rt: R, j: &Json) -> Json {
    let which = s(j, "window").unwrap_or_else(|| "all".into());
    let scale = Scale::parse(&s(j, "scale").unwrap_or_else(|| "screen".into()));
    let (frame, cursor) = (b(j, "frame", true), b(j, "cursor", false));
    let mut out = None;
    let ok = rt.with_desk(&mut |desk, store| {
        out = Some((|| -> Result<Json, String> {
            let (px, said) = if which.eq_ignore_ascii_case("all") {
                let px = eyes::capture_all(desk, store, scale, frame, cursor)?;
                (px, format!("The program's windows ({}), side by side, back to front", eyes::window_list(desk)))
            } else {
                let form = eyes::find_window(desk, &which)?;
                let px = eyes::capture(desk, store, &form, scale, frame, cursor)?;
                let title = desk.forms.get(&form).map(|w| w.spec.title.clone()).unwrap_or_default();
                (px, format!("{form} (\"{title}\")"))
            };
            let png = eyes::png(&px)?;
            Ok(json!({
                "ok": true,
                "text": format!("{said}: a {} × {} pixel picture{}.", px.width, px.height, if matches!(scale, Scale::Screen) { " at the screen's scale" } else { "" }),
                "png": rapidr_value::objects::codec::base64_encode(&png),
                "width": px.width, "height": px.height,
            }))
        })());
    });
    if !ok {
        return fail("The program's windows can't be looked at right now (it is starting, or busy): try again in a moment.");
    }
    out.unwrap_or_else(|| Ok(fail("no picture"))).unwrap_or_else(fail)
}

/// The layout report: `text` in words, `layout` as JSON.
pub fn layout<R: Windows>(rt: R, j: &Json) -> Json {
    let which = s(j, "window");
    let mut out = None;
    let ok = rt.with_desk(&mut |desk, store| {
        let only = match which.as_deref() {
            Some(w) if !w.eq_ignore_ascii_case("all") => match eyes::find_window(desk, w) {
                Ok(f) => Some(f),
                Err(e) => {
                    out = Some(fail(e));
                    return;
                }
            },
            _ => None,
        };
        let report = eyes::layout(desk, store, only.as_deref());
        out = Some(json!({"ok": true, "text": eyes::layout_text(&report), "layout": report}));
    });
    if !ok {
        return fail("The program's windows can't be looked at right now (it is starting, or busy): try again in a moment.");
    }
    out.unwrap_or_else(|| fail("no layout"))
}

/// The input job a request asks for, or why it can't be one.
pub fn act_of(j: &Json) -> Result<Act, String> {
    let what = s(j, "do").unwrap_or_default().to_ascii_lowercase();
    let target = target_of(j, "");
    let button = match s(j, "button").map(|b| b.to_ascii_lowercase()).as_deref() {
        Some("right") => rapidr_value::input::Button::Right,
        Some("middle") => rapidr_value::input::Button::Middle,
        _ => rapidr_value::input::Button::Left,
    };
    Ok(match what.as_str() {
        "click" => Act::Click { target, button, double: b(j, "double", false) },
        "double_click" | "doubleclick" | "dblclick" => Act::Click { target, button, double: true },
        "right_click" | "rightclick" => Act::Click { target, button: rapidr_value::input::Button::Right, double: false },
        "hover" | "move" => Act::Hover { target },
        "type" => Act::Type { target, text: raw(j, "text").unwrap_or_default() },
        "key" | "keys" => Act::Key { target, keys: s(j, "keys").or_else(|| s(j, "key")).unwrap_or_default() },
        "drag" => {
            let mut to = target_of(j, "to_");
            if to.window.as_deref() == target.window.as_deref() && to.at.is_none() && to.name.is_none() && to.caption.is_none() && to.label.is_none() {
                return Err("A drag needs where to: to_x and to_y (in the same window), or to_name / to_caption.".into());
            }
            if to.window.is_none() {
                to.window = target.window.clone();
            }
            Act::Drag { from: target, to }
        }
        "scroll" | "wheel" => Act::Scroll { target, dx: n(j, "dx").unwrap_or(0.0), dy: n(j, "dy").or_else(|| n(j, "notches")).unwrap_or(3.0) },
        "wait" => Act::Wait { ms: n(j, "ms").unwrap_or(if b(j, "idle", false) { 5000.0 } else { 500.0 }).max(0.0) as u64, idle: b(j, "idle", false) },
        "" => return Err("Say what to do: \"do\" is screenshot, layout, click, type, key, drag, scroll, wait or cursor.".into()),
        other => return Err(format!("\"{other}\" isn't something the program's hands do: screenshot, layout, click, double_click, right_click, hover, type, key, drag, scroll, wait or cursor.")),
    })
}

/// A request from RapidR Studio (`seq`: its number in the session).
/// Looks are answered now (`Some`); input becomes a job answered later by
/// [`ends`] (`None`) — or is refused now: while `paused` (where the
/// debugger holds the program, in words: "at line 12 of calc.rr") nothing
/// of the program may run.
pub fn request<R: Program + Windows>(rt: R, seq: u64, j: &Json, paused: Option<&str>) -> Option<Json> {
    let what = s(j, "do").unwrap_or_default().to_ascii_lowercase();
    match what.as_str() {
        "screenshot" | "capture" | "picture" => return Some(screenshot(rt, j)),
        "layout" => return Some(layout(rt, j)),
        "windows" => {
            let mut list = String::new();
            rt.with_desk(&mut |desk, _| list = eyes::window_list(desk));
            return Some(json!({"ok": true, "text": format!("The program's windows, back to front: {list}.")}));
        }
        "cursor" => {
            hands::set_cursor_shown(b(j, "show", true));
            if let Some(speed) = n(j, "speed") {
                hands::set_speed(speed);
            }
            return Some(json!({"ok": true, "text": format!("The virtual cursor is {}.", if hands::cursor_shown() { "shown" } else { "hidden" })}));
        }
        _ => {}
    }
    let act = match act_of(j) {
        Ok(a) => a,
        Err(e) => return Some(fail(e)),
    };
    if let Some(at) = paused {
        if !matches!(act, Act::Wait { .. }) {
            return Some(fail(format!("The program is paused in the debugger {at}: continue it first (the debug tool's continue), then {} again. Looking (screenshot, layout) works while it is paused.", act.verb())));
        }
    }
    hands::submit(Owner::Link(seq), act);
    None
}

/// The jobs that ended since the last call: the link's, as (its
/// request's number, the reply), and the program's RAppHands' (theirs to
/// fire OnDone for). The runtime takes them once a turn.
pub fn ends() -> (Vec<(u64, Json)>, Vec<hands::Done>) {
    let mut links = Vec::new();
    let mut theirs = Vec::new();
    for d in hands::take_done() {
        match d.owner {
            Owner::Link(seq) => links.push((seq, json!({"ok": d.ok, "text": d.text}))),
            Owner::Hands(..) => theirs.push(d),
        }
    }
    (links, theirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_become_jobs_or_say_what_is_wrong() {
        let j = |t: &str| serde_json::from_str::<Json>(t).unwrap();
        assert!(matches!(act_of(&j(r#"{"do":"click","caption":"7"}"#)), Ok(Act::Click { double: false, .. })));
        // (a caption a JSON builder made a number, text typed as one)
        let Ok(Act::Click { target, .. }) = act_of(&j(r#"{"do":"click","caption":7}"#)) else { panic!() };
        assert_eq!(target.caption.as_deref(), Some("7"));
        assert!(matches!(act_of(&j(r#"{"do":"type","text":42}"#)), Ok(Act::Type { text, .. }) if text == "42"));
        assert!(matches!(act_of(&j(r#"{"do":"double_click","name":"list"}"#)), Ok(Act::Click { double: true, .. })));
        let Ok(Act::Click { target, .. }) = act_of(&j(r#"{"do":"click","window":"calc","x":"30","y":55}"#)) else { panic!() };
        assert_eq!((target.window.as_deref(), target.at), (Some("calc"), Some((30.0, 55.0))));
        let Ok(Act::Drag { to, .. }) = act_of(&j(r#"{"do":"drag","name":"knob","to_x":100,"to_y":5}"#)) else { panic!() };
        assert_eq!(to.at, Some((100.0, 5.0)));
        assert!(act_of(&j(r#"{"do":"drag","name":"knob"}"#)).unwrap_err().contains("to_x"));
        assert!(act_of(&j(r#"{"do":"fly"}"#)).unwrap_err().contains("isn't something"));
        assert!(matches!(act_of(&j(r#"{"do":"wait","idle":true}"#)), Ok(Act::Wait { ms: 5000, idle: true })));
    }
}
