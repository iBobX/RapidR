//! The program RapidR Studio runs, during a demo (its own process,
//! `RAPIDR_SESSION_IDE` set, the demo's variables inherited): it plays
//! along through files in `<out>/app` —
//!
//! - `layers.bin`: its windows as the kernel draws them (framed, at the
//!   screen's scale) and where they are, written a few times a second and
//!   whenever they change; Studio's recorder puts them over its own;
//! - `in-<n>.json` → `out-<n>.json`: a request from the demo player
//!   (`find` a target: its place on the screen; `click`, `type`, `key`)
//!   played through the kernel as the user's input, answered once its
//!   handlers have run;
//! - `exit`: the recording is over — the program ends.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rapidr_value::input::Button;
use rapidr_value::objects::codec::Pixels;
use serde_json::{json, Value as Json};

use super::input::{self, Input};
use super::record::{self, Layer};
use super::script::Target;
use crate::{Program, Windows};

const MAGIC: &[u8; 4] = b"RRDL";

#[derive(Clone, Debug)]
enum Step {
    Input { at: (f64, f64), input: Input },
    Char(char),
    Keys(String),
    Pause(u64),
}

#[derive(Default)]
struct Side {
    next: u32,
    cur: Option<(u32, VecDeque<Step>, Instant, u8)>,
    exported: Option<Instant>,
    last: u64,
}

thread_local! {
    static SIDE: RefCell<Side> = RefCell::new(Side { next: 1, ..Side::default() });
}

pub fn turn<R: Program + Windows>(rt: R, dir: &Path) {
    if dir.join("exit").exists() {
        let _ = std::fs::write(dir.join("ended"), "ended");
        std::process::exit(0);
    }
    let now = Instant::now();
    SIDE.with(|s| {
        let Ok(mut s) = s.try_borrow_mut() else { return };
        let acted = serve(&mut s, rt, dir, now);
        export(&mut s, rt, dir, now, acted);
    });
}

fn target_of(v: &Json) -> Target {
    let mut t = super::script::target_of(v["what"].as_str().unwrap_or(""));
    t.item = v["item"].as_str().map(str::to_string).or(t.item);
    t.fx = v["x"].as_f64().unwrap_or(0.5);
    t.fy = v["y"].as_f64().unwrap_or(0.5);
    t.window = v["window"].as_str().map(str::to_string);
    if let Some(a) = v["at"].as_array().filter(|a| a.len() == 2) {
        t.at = Some((a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(0.0)));
    }
    t
}

fn reply(dir: &Path, n: u32, v: &Json) {
    let tmp = dir.join(format!("out-{n}.tmp"));
    let _ = std::fs::write(&tmp, v.to_string());
    let _ = std::fs::rename(&tmp, dir.join(format!("out-{n}.json")));
}

/// A request taken, its steps played; true when input went in this turn.
fn serve<R: Program + Windows>(s: &mut Side, rt: R, dir: &Path, now: Instant) -> bool {
    if s.cur.is_none() {
        let file = dir.join(format!("in-{}.json", s.next));
        let Some(req) = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<Json>(&t).ok()) else { return false };
        let n = s.next;
        s.next += 1;
        let target = req.get("target").map(target_of);
        let mut found = Err("no target".to_string());
        if let Some(t) = &target {
            rt.with_desk(&mut |desk, store| found = input::locate(desk, store, t));
        }
        let op = req["op"].as_str().unwrap_or("");
        let spot = match (&found, op) {
            (Ok(spot), _) => Some(spot.clone()),
            (Err(e), "find" | "click") => {
                reply(dir, n, &json!({"ok": false, "text": e}));
                return false;
            }
            _ => None,
        };
        let mut steps = VecDeque::new();
        let click = |steps: &mut VecDeque<Step>, at: (f64, f64), button: Button, double: bool| {
            steps.extend([Step::Input { at, input: Input::Move }, Step::Pause(60), Step::Input { at, input: Input::Press { button, single: true } }, Step::Pause(90), Step::Input { at, input: Input::Release { button } }]);
            if double {
                steps.extend([Step::Pause(80), Step::Input { at, input: Input::Press { button, single: false } }, Step::Pause(70), Step::Input { at, input: Input::Release { button } }]);
            }
        };
        match op {
            "find" => {
                let sp = spot.expect("found");
                reply(dir, n, &json!({"ok": true, "at": [sp.at.0, sp.at.1], "rect": [sp.rect.0, sp.rect.1, sp.rect.2, sp.rect.3], "form": sp.form}));
                return false;
            }
            "click" => {
                let sp = spot.expect("found");
                let button = if req["right"] == Json::Bool(true) { Button::Right } else { Button::Left };
                click(&mut steps, sp.at, button, req["double"] == Json::Bool(true));
            }
            "type" | "key" => {
                if let Some(sp) = &spot {
                    click(&mut steps, sp.at, Button::Left, false);
                    steps.push_back(Step::Pause(150));
                }
                if op == "type" {
                    let text = req["text"].as_str().unwrap_or("");
                    let delays = super::motion::typing_delays(text, req["speed"].as_f64().unwrap_or(1.0), super::motion::seed_of(text));
                    for (c, d) in text.chars().zip(delays) {
                        steps.push_back(Step::Char(c));
                        steps.push_back(Step::Pause(d));
                    }
                } else {
                    steps.push_back(Step::Keys(req["keys"].as_str().unwrap_or("").to_string()));
                }
            }
            other => {
                reply(dir, n, &json!({"ok": false, "text": format!("\"{other}\" isn't a request the program knows")}));
                return false;
            }
        }
        s.cur = Some((n, steps, now, 0));
    }
    let mut acted = false;
    let Some((n, steps, next, settle)) = s.cur.as_mut() else { return false };
    while *next <= now {
        let Some(step) = steps.pop_front() else {
            // (done once its handlers have had their turns)
            if rt.handlers_pending() || *settle < 3 {
                *settle += 1;
                return acted;
            }
            reply(dir, *n, &json!({"ok": true}));
            s.cur = None;
            return acted;
        };
        match step {
            Step::Pause(ms) => *next = now + Duration::from_millis(ms),
            Step::Input { at, input } => {
                rt.with_desk(&mut |desk, store| {
                    input::at_point(desk, store, at, &input);
                });
                rt.dispatch_pending();
                acted = true;
            }
            Step::Char(c) => {
                rt.with_desk(&mut |desk, store| input::type_char(desk, store, c));
                rt.dispatch_pending();
                acted = true;
            }
            Step::Keys(k) => {
                let mut r = Ok(());
                rt.with_desk(&mut |desk, store| r = input::keys(desk, store, &k));
                rt.dispatch_pending();
                if let Err(e) = r {
                    let n = *n;
                    reply(dir, n, &json!({"ok": false, "text": e}));
                    s.cur = None;
                    return true;
                }
                acted = true;
            }
        }
    }
    acted
}

/// The windows written for Studio's recorder: when they changed, and a
/// few times a second anyway (so it knows the program still runs).
fn export<R: Program + Windows>(s: &mut Side, rt: R, dir: &Path, now: Instant, acted: bool) {
    if !acted && s.exported.is_some_and(|t| now.duration_since(t) < Duration::from_millis(60)) {
        return;
    }
    let mut layers: Vec<((i64, i64), Pixels)> = Vec::new();
    rt.with_desk(&mut |desk, store| {
        for f in desk.stacking() {
            let Some(((x, y), _, _)) = record::placement(desk, store, &f) else { continue };
            if let Ok(px) = crate::eyes::capture(desk, store, &f, crate::eyes::Scale::Screen, true, false) {
                layers.push(((x, y), px));
            }
        }
    });
    let mut bytes = Vec::with_capacity(16 + layers.iter().map(|(_, p)| 16 + p.pixels.len() * 4).sum::<usize>());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(layers.len() as u32).to_le_bytes());
    for ((x, y), px) in &layers {
        bytes.extend_from_slice(&(*x as i32).to_le_bytes());
        bytes.extend_from_slice(&(*y as i32).to_le_bytes());
        bytes.extend_from_slice(&(px.width as u32).to_le_bytes());
        bytes.extend_from_slice(&(px.height as u32).to_le_bytes());
        for p in &px.pixels {
            bytes.extend_from_slice(&p.to_le_bytes());
        }
    }
    let h = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3));
    // (unchanged: written again only to say it still runs)
    if h == s.last && s.exported.is_some_and(|t| now.duration_since(t) < Duration::from_millis(250)) {
        return;
    }
    let tmp = dir.join("layers.tmp");
    if std::fs::write(&tmp, &bytes).is_ok() {
        let _ = std::fs::rename(&tmp, dir.join("layers.bin"));
    }
    s.last = h;
    s.exported = Some(now);
}

/// `layers.bin` read back (Studio's side).
pub fn read_layers(b: &[u8]) -> Option<Vec<Layer>> {
    if b.len() < 8 || &b[..4] != MAGIC {
        return None;
    }
    let u32_at = |i: usize| b.get(i..i + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]));
    let n = u32_at(4)? as usize;
    let mut at = 8;
    let mut out = Vec::new();
    for _ in 0..n.min(64) {
        let x = u32_at(at)? as i32;
        let y = u32_at(at + 4)? as i32;
        let (w, h) = (u32_at(at + 8)? as usize, u32_at(at + 12)? as usize);
        at += 16;
        let len = w.checked_mul(h)?.checked_mul(4)?;
        let px = b.get(at..at + len)?;
        let pixels = px.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        at += len;
        out.push(Layer { at: (i64::from(x), i64::from(y)), pixels: Arc::new(Pixels { width: w, height: h, pixels }) });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_read_back_and_refuse_garbage() {
        let mut b = Vec::new();
        b.extend_from_slice(MAGIC);
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&(-5i32).to_le_bytes());
        b.extend_from_slice(&7i32.to_le_bytes());
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0x00FF_0000u32.to_le_bytes());
        b.extend_from_slice(&0x0000_00FFu32.to_le_bytes());
        let l = read_layers(&b).unwrap();
        assert_eq!((l.len(), l[0].at, l[0].pixels.pixels.clone()), (1, (-5, 7), vec![0x00FF_0000, 0xFF]));
        assert!(read_layers(b"nope").is_none());
        // (cut short: refused, never read past the end)
        assert!(read_layers(&b[..b.len() - 2]).is_none());
    }
}
