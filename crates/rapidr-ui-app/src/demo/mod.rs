//! Demo mode: a scripted tutorial played in the program's own windows and
//! recorded frame by frame, headless (docs/video-pipeline.md). RapidR
//! Studio's launch videos are made with it, again after any change, in
//! every language the script has; later it becomes the public RTutor
//! component (guided tours in users' apps), so its parts are the ones a
//! tour needs: a target found by name, a pointer that glides to it and
//! clicks with a soft ripple, typing with a natural rhythm, a highlight,
//! captions, title cards.
//!
//! - `RAPIDR_DEMO=<script.json>`: the script ([`script`]);
//!   `RAPIDR_DEMO_OUT=<folder>`: where the frames, `frames.json` and
//!   `timeline.json` go; `RAPIDR_DEMO_LANG=es`: the script's language
//!   (English by default). Test hooks: they act only in a development build
//!   or with the signed test-mode token (SEC-24,
//!   docs/security/test-mode.md) — demo mode is input automation.
//! - The host is headless (no window on the screen, the person's mouse and
//!   keyboard untouched; the screen is the script's size).
//! - **Time** is the video's: each turn of the loop is one frame (1/30 s
//!   of the kernel's clock, `tick::set_test_now`), never ahead of real
//!   time; a slow frame makes the recording slower, never the video
//!   choppy. Narration lines last their clips' lengths.
//! - The **program Studio runs** (its own process) plays along
//!   ([`app`]): its windows come over as layers, its controls are acted
//!   on through files in `<out>/app` (`app:Button1`).
//! - Studio's own commands (`do`) go through `<out>/inbox` to
//!   `ide/demo.inc`.

mod app;
mod input;
pub mod motion;
pub mod record;
pub mod script;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use rapidr_ui_kernel::demo_overlay::{self, Caption, Card, Highlight, Overlay, Pointer};
use rapidr_ui_kernel::Picture;
use rapidr_value::input::Button;
use serde_json::{json, Value as Json};

use crate::{Program, Windows};
use input::{Input, Spot};
use record::{Layer, Recorder};
use script::{Act, Action, CardSpec, Cond, Script, Target};

/// Which side of a demo this process plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The program the script drives and records (RapidR Studio).
    Main,
    /// The program Studio runs (`RAPIDR_SESSION_IDE` set): its windows and
    /// controls, for the main side.
    App,
}

/// The demo's settings, read once.
pub struct Config {
    pub script: Script,
    pub out: PathBuf,
    pub role: Role,
    /// The script's folder (its pictures' paths are from there).
    pub base: PathBuf,
}

static CONFIG: OnceLock<Option<Result<Config, String>>> = OnceLock::new();

fn config() -> Option<&'static Result<Config, String>> {
    CONFIG
        .get_or_init(|| {
            let file = crate::testhooks::var("RAPIDR_DEMO").filter(|f| !f.trim().is_empty())?;
            let out = crate::testhooks::var("RAPIDR_DEMO_OUT").filter(|f| !f.trim().is_empty()).unwrap_or_else(|| "demo-out".into());
            let lang = crate::testhooks::var("RAPIDR_DEMO_LANG").filter(|l| !l.trim().is_empty()).unwrap_or_else(|| "en".into());
            let role = if std::env::var_os("RAPIDR_SESSION_IDE").is_some() { Role::App } else { Role::Main };
            let path = PathBuf::from(&file);
            let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
            Some(std::fs::read_to_string(&path).map_err(|e| format!("can't read the demo script {file}: {e}")).and_then(|t| script::parse(&t, lang.trim())).map(|script| Config { script, out: PathBuf::from(out), role, base }))
        })
        .as_ref()
}

/// A demo plays in this process (`RAPIDR_DEMO`, while the test hooks act).
pub fn active() -> bool {
    config().is_some()
}

/// The headless screen the demo asks for (its frame), logical pixels.
pub fn screen() -> Option<(i64, i64)> {
    match config()? {
        Ok(c) => Some(c.script.screen),
        Err(_) => None,
    }
}

/// How long the window loop may wait before the demo's next turn.
pub fn wait() -> Option<Duration> {
    let c = config()?;
    let Ok(c) = c else { return Some(Duration::ZERO) };
    if c.role == Role::App {
        return Some(Duration::from_millis(15));
    }
    PLAYER.with(|p| {
        let Ok(p) = p.try_borrow() else { return Some(Duration::from_millis(5)) };
        Some(match p.as_ref().and_then(|p| p.clock.as_ref()) {
            Some(clock) => clock.next_at().saturating_duration_since(Instant::now()),
            None => Duration::from_millis(15),
        })
    })
}

/// The demo's turn (the window loop's, every turn): the script played, a
/// frame recorded — or, in the program Studio runs, its windows shown and
/// its requests served.
pub fn turn<R: Program + Windows>(rt: R) {
    let Some(c) = config() else { return };
    if rt.in_host_callback() {
        return;
    }
    let c = match c {
        Ok(c) => c,
        Err(e) => {
            if std::env::var_os("RAPIDR_SESSION_IDE").is_none() {
                fail_now(Path::new(&crate::testhooks::var("RAPIDR_DEMO_OUT").unwrap_or_else(|| "demo-out".into())), e);
            }
            return;
        }
    };
    if c.role == Role::App {
        app::turn(rt, &c.out.join("app"));
        return;
    }
    PLAYER.with(|p| {
        let Ok(mut p) = p.try_borrow_mut() else { return };
        let player = p.get_or_insert_with(|| Player::new(c));
        player.turn(rt);
    });
}

/// The demo can't go on: why, written to `<out>/error.txt` and said; the
/// process ends (the video pipeline reports it).
fn fail_now(out: &Path, why: &str) -> ! {
    let _ = std::fs::create_dir_all(out);
    let _ = std::fs::write(out.join("error.txt"), format!("{why}\n"));
    let _ = std::fs::create_dir_all(out.join("app"));
    let _ = std::fs::write(out.join("app").join("exit"), "exit");
    eprintln!("[rapidr] demo: {why}");
    std::process::exit(2);
}

thread_local! {
    static PLAYER: std::cell::RefCell<Option<Player>> = const { std::cell::RefCell::new(None) };
}

/// The video's clock: frame `n` is at `base + n / fps`.
struct Clock {
    base: Instant,
    frame: u64,
    fps: f64,
}

impl Clock {
    fn at(&self, frame: u64) -> Instant {
        self.base + Duration::from_secs_f64(frame as f64 / self.fps)
    }

    /// When the next frame may be made (never ahead of real time).
    fn next_at(&self) -> Instant {
        self.at(self.frame)
    }

    fn ms(&self, at: Instant) -> u64 {
        at.saturating_duration_since(self.base).as_millis() as u64
    }
}

/// One thing an action does, in turn.
#[derive(Clone, Debug)]
enum Micro {
    /// The pointer glides there (screen point); `carry`: a button is held
    /// (each frame the mouse moves with it: a drag).
    Glide { to: (f64, f64), ms: Option<u64>, carry: bool },
    Cue,
    Input { at: (f64, f64), input: Input },
    Char(char),
    Keys(String),
    Pause(u64),
    Until { cond: Cond, timeout: u64 },
    Do { cmds: String, seq: Option<u32> },
    /// A request to the program Studio runs; its answer awaited.
    App { req: Json, seq: Option<u32>, then: AppThen },
    /// A ring around `spot`; `form`: the window it is in (it goes when
    /// that window closes).
    Highlight { spot: Option<(f64, f64, f64, f64)>, spotlight: bool, for_ms: Option<u64>, form: Option<String> },
    /// A click's ring where the pointer is (a click in the program Studio
    /// runs, which presses it there).
    Ripple,
    Mark(String),
    Pointer(bool),
}

/// What follows an `app:` target's place being known.
#[derive(Clone, Debug)]
enum AppThen {
    /// Nothing: the request does it all (a click, typing).
    Nothing,
    /// Glide there, cue, then ask for this.
    Act(Json),
    /// Glide there and cue (a hover).
    Hover,
    Highlight { spotlight: bool, for_ms: Option<u64> },
}

struct Running {
    micros: VecDeque<Micro>,
    next: Instant,
    cue: Option<String>,
    /// A wait's deadline, a glide's start.
    deadline: Option<Instant>,
}

struct StepRun {
    step: usize,
    /// Each line's start and end (video instants).
    lines: Vec<(Option<Instant>, Option<Instant>)>,
    cues: HashSet<String>,
    actions: VecDeque<Action>,
    cur: Option<Running>,
    done_at: Option<Instant>,
}

/// A glide: from, to, when it started, how long it takes (ms).
type Glide = ((f64, f64), (f64, f64), Instant, u64);

#[derive(Default)]
struct Ptr {
    at: (f64, f64),
    shown: bool,
    /// When it was shown (fades in) or hidden (fades out).
    since: Option<Instant>,
    glide: Option<Glide>,
    pressed: bool,
    ripple: Option<((f64, f64), Instant)>,
}

struct Player {
    script: &'static Script,
    out: PathBuf,
    base: PathBuf,
    booted: Instant,
    settle: Option<Instant>,
    clock: Option<Clock>,
    rec: Option<Recorder>,
    step: usize,
    run: Option<StepRun>,
    ptr: Ptr,
    /// The ring, until when, and the window it is in.
    highlight: Option<(Highlight, Option<Instant>, Option<String>)>,
    /// The caption: its lines, width, from and until when.
    caption: Option<(Vec<String>, i64, Instant, Instant)>,
    card: Option<(CardSpec, Instant, Option<Instant>)>,
    pictures: HashMap<String, Option<Arc<Picture>>>,
    timeline: Vec<Json>,
    marks: serde_json::Map<String, Json>,
    seq_do: u32,
    seq_app: u32,
    layers: (Option<std::time::SystemTime>, Vec<Layer>),
    end_at: Option<Instant>,
}

const LINE_GAP: Duration = Duration::from_millis(300);
const RIPPLE: Duration = Duration::from_millis(450);
const FADE: Duration = Duration::from_millis(220);
const CARD_FADE: Duration = Duration::from_millis(500);

impl Player {
    fn new(c: &'static Config) -> Player {
        let _ = std::fs::remove_dir_all(c.out.join("inbox"));
        let _ = std::fs::remove_dir_all(c.out.join("app"));
        let _ = std::fs::remove_file(c.out.join("error.txt"));
        let _ = std::fs::create_dir_all(c.out.join("inbox"));
        let _ = std::fs::create_dir_all(c.out.join("app"));
        Player {
            script: &c.script,
            out: c.out.clone(),
            base: c.base.clone(),
            booted: Instant::now(),
            settle: None,
            clock: None,
            rec: None,
            step: 0,
            run: None,
            ptr: Ptr { at: (c.script.screen.0 as f64 * 0.62, c.script.screen.1 as f64 * 0.62), ..Ptr::default() },
            highlight: None,
            caption: None,
            card: None,
            pictures: HashMap::new(),
            timeline: Vec::new(),
            marks: serde_json::Map::new(),
            seq_do: 0,
            seq_app: 0,
            layers: (None, Vec::new()),
            end_at: None,
        }
    }

    fn fail(&mut self, why: &str) -> ! {
        let at = self.clock.as_ref().map(|c| format!(" (at {:.1} s of the video, step {})", c.ms(c.at(c.frame)) as f64 / 1000.0, self.step + 1)).unwrap_or_default();
        if let Some(rec) = self.rec.as_mut() {
            let _ = rec.finish();
        }
        self.write_timeline(false);
        fail_now(&self.out, &format!("{why}{at}"));
    }

    fn turn<R: Program + Windows>(&mut self, rt: R) {
        let real = Instant::now();
        if self.clock.is_none() {
            // (the program's main window up, then a moment for it to settle)
            let mut shown = false;
            let window = self.script.window.clone();
            rt.with_desk(&mut |desk, _| shown = desk.forms.get(&window).is_some_and(|f| f.shown));
            if !shown {
                if real.duration_since(self.booted) > Duration::from_secs(90) {
                    self.fail(&format!("the window \"{window}\" never showed (the script's \"window\")"));
                }
                return;
            }
            let settle = *self.settle.get_or_insert(real + Duration::from_millis(1500));
            if real < settle {
                return;
            }
            match Recorder::new(&self.out.join("frames"), self.script.screen, self.script.scale) {
                Ok(r) => self.rec = Some(r),
                Err(e) => self.fail(&e),
            }
            self.clock = Some(Clock { base: real, frame: 0, fps: self.script.fps });
        }
        let Some(clock) = self.clock.as_ref() else { return };
        let now = clock.next_at();
        if now > real {
            return;
        }
        rapidr_ui_kernel::tick::set_test_now(Some(now));
        self.advance(rt, now);
        self.frame(rt, now);
        if let Some(c) = self.clock.as_mut() {
            c.frame += 1;
        }
        if self.end_at.is_some_and(|e| now >= e) {
            self.finish();
        }
    }

    // ------------------------------------------------------------ the steps --

    fn advance<R: Program + Windows>(&mut self, rt: R, now: Instant) {
        for _ in 0..8 {
            if self.run.is_none() {
                if self.step >= self.script.steps.len() {
                    if self.end_at.is_none() {
                        self.end_at = Some(now + Duration::from_millis(700));
                        self.ptr_hide(now);
                    }
                    return;
                }
                self.start_step(now);
            }
            self.lines(rt, now);
            self.actions(rt, now);
            let Some(run) = self.run.as_mut() else { return };
            let lines_done = run.lines.iter().all(|(_, e)| e.is_some_and(|e| e <= now));
            if lines_done && run.cur.is_none() && run.actions.is_empty() {
                let gap = Duration::from_millis(self.script.steps[run.step].gap_ms);
                let at = *run.done_at.get_or_insert(now + gap);
                if now >= at {
                    self.run = None;
                    self.step += 1;
                    continue;
                }
            }
            return;
        }
    }

    fn start_step(&mut self, now: Instant) {
        let s = &self.script.steps[self.step];
        // (a card shown by the step before fades as this one starts)
        if let Some((spec, _, fade)) = self.card.as_mut() {
            if s.card.as_ref() != Some(&*spec) && fade.is_none() {
                *fade = Some(now);
            }
        }
        if let Some(card) = &s.card {
            if self.card.as_ref().map(|(c, _, f)| (c, f.is_none())) != Some((card, true)) {
                self.card = Some((card.clone(), now, None));
            }
        }
        self.run = Some(StepRun { step: self.step, lines: vec![(None, None); s.say.len()], cues: HashSet::new(), actions: s.actions.iter().cloned().collect(), cur: None, done_at: None });
    }

    fn lines<R: Program + Windows>(&mut self, rt: R, now: Instant) {
        let Some(run) = self.run.as_mut() else { return };
        let s = &self.script.steps[run.step];
        let mut started_line = None;
        for i in 0..s.say.len() {
            let (start, end) = run.lines[i];
            if end.is_some() {
                continue;
            }
            if start.is_none() {
                let after = if i == 0 { Some(now) } else { run.lines[i - 1].1.map(|e| e + LINE_GAP) };
                let cue_ok = s.say[i].cue.as_ref().is_none_or(|c| run.cues.contains(c));
                if after.is_some_and(|a| now >= a) && cue_ok {
                    run.lines[i] = (Some(now), Some(now + Duration::from_millis(s.say[i].ms)));
                    started_line = Some(i);
                }
            }
            break;
        }
        let Some(i) = started_line else { return };
        let line = &s.say[i];
        let ms = self.clock.as_ref().map_or(0, |c| c.ms(now));
        self.timeline.push(json!({"step": run.step + 1, "line": i + 1, "text": line.text, "spoken": line.spoken, "clip": line.clip, "start": ms, "ms": line.ms}));
        if line.captions && self.script.caption_px > 0 && !line.text.is_empty() {
            let px = self.script.caption_px;
            let max = (self.script.screen.0 as f64 * 0.72) as f32;
            let mut lines = Vec::new();
            let mut width = 0;
            rt.with_desk(&mut |desk, _| (lines, width) = wrap(&mut desk.text, &line.text, px, max));
            self.caption = Some((lines, width, now, now + Duration::from_millis(line.ms) + Duration::from_millis(250)));
        }
    }

    fn actions<R: Program + Windows>(&mut self, rt: R, now: Instant) {
        for _ in 0..32 {
            let Some(run) = self.run.as_mut() else { return };
            if run.cur.is_none() {
                let Some(a) = run.actions.front() else { return };
                if let Some(n) = a.after_line {
                    let said = run.lines.get(n.saturating_sub(1)).is_none_or(|(_, e)| e.is_some_and(|e| e <= now));
                    if !said {
                        return;
                    }
                }
                let a = run.actions.pop_front().expect("an action");
                let planned = self.plan(rt, &a);
                let micros = match planned {
                    Ok(m) => m,
                    Err(e) => self.fail(&format!("{}: {e}", describe(&a.act))),
                };
                if let Some(run) = self.run.as_mut() {
                    run.cur = Some(Running { micros, next: now, cue: a.cue.clone(), deadline: None });
                }
                continue;
            }
            let due = run.cur.as_ref().is_some_and(|c| c.next <= now);
            if !due {
                return;
            }
            let Some(m) = run.cur.as_ref().and_then(|c| c.micros.front().cloned()) else {
                run.cur = None;
                continue;
            };
            match self.exec(rt, m, now) {
                Next::Done => {
                    if let Some(c) = self.run.as_mut().and_then(|r| r.cur.as_mut()) {
                        c.micros.pop_front();
                        c.deadline = None;
                    }
                }
                Next::After(ms) => {
                    if let Some(c) = self.run.as_mut().and_then(|r| r.cur.as_mut()) {
                        c.micros.pop_front();
                        c.deadline = None;
                        c.next = now + Duration::from_millis(ms);
                    }
                    return;
                }
                Next::Again => return,
                Next::Replace(more) => {
                    if let Some(c) = self.run.as_mut().and_then(|r| r.cur.as_mut()) {
                        c.micros.pop_front();
                        c.deadline = None;
                        for m in more.into_iter().rev() {
                            c.micros.push_front(m);
                        }
                    }
                }
            }
        }
    }

    /// An action's micro-steps, its target found now.
    fn plan<R: Program + Windows>(&mut self, rt: R, a: &Action) -> Result<VecDeque<Micro>, String> {
        let mut m = VecDeque::new();
        let speed_ms = a.glide_ms;
        let find = |t: &Target| -> Result<Spot, String> {
            let mut r = Err("the program's windows aren't ready".to_string());
            rt.with_desk(&mut |desk, store| r = input::locate(desk, store, t));
            r
        };
        let target_json = |t: &Target| json!({"what": t.what, "item": t.item, "x": t.fx, "y": t.fy, "window": t.window, "at": t.at});
        match &a.act {
            Act::Move(t) if t.app => m.push_back(app_find(t, AppThen::Hover)),
            Act::Move(t) => {
                let s = find(t)?;
                m.extend([Micro::Glide { to: s.at, ms: speed_ms, carry: false }, Micro::Cue, Micro::Input { at: s.at, input: Input::Move }]);
            }
            Act::Click { target, double, right } if target.app => {
                m.push_back(app_find(target, AppThen::Act(json!({"op": "click", "target": target_json(target), "double": double, "right": right}))));
            }
            Act::Click { target, double, right } => {
                let s = find(target)?;
                let button = if *right { Button::Right } else { Button::Left };
                m.extend([Micro::Glide { to: s.at, ms: speed_ms, carry: false }, Micro::Cue, Micro::Input { at: s.at, input: Input::Move }, Micro::Pause(110)]);
                m.extend([Micro::Input { at: s.at, input: Input::Press { button, single: true } }, Micro::Pause(90), Micro::Input { at: s.at, input: Input::Release { button } }]);
                if *double {
                    m.extend([Micro::Pause(80), Micro::Input { at: s.at, input: Input::Press { button, single: false } }, Micro::Pause(70), Micro::Input { at: s.at, input: Input::Release { button } }]);
                }
            }
            Act::Drag { from, to } => {
                if from.app || to.app {
                    return Err("a drag in the program Studio runs isn't supported yet: drag in Studio".into());
                }
                let a0 = find(from)?;
                let b0 = find(to)?;
                m.extend([Micro::Glide { to: a0.at, ms: speed_ms, carry: false }, Micro::Cue, Micro::Input { at: a0.at, input: Input::Move }, Micro::Pause(160)]);
                m.extend([Micro::Input { at: a0.at, input: Input::Press { button: Button::Left, single: true } }, Micro::Pause(180)]);
                // (a small first move: what starts a drag in the kernel)
                let nudge = (a0.at.0 + 6.0, a0.at.1 + 3.0);
                m.extend([Micro::Glide { to: nudge, ms: Some(120), carry: true }, Micro::Glide { to: b0.at, ms: None, carry: true }, Micro::Pause(320)]);
                m.push_back(Micro::Input { at: b0.at, input: Input::Release { button: Button::Left } });
            }
            Act::Type { into, text } => {
                if let Some(t) = into {
                    if t.app && t.what.is_empty() {
                        m.extend([Micro::Cue, Micro::App { req: json!({"op": "type", "text": text, "speed": self.script.typing}), seq: None, then: AppThen::Nothing }, Micro::Pause(a.pause_ms)]);
                        return Ok(m);
                    }
                    if t.app {
                        m.push_back(app_find(t, AppThen::Act(json!({"op": "type", "target": target_json(t), "text": text, "speed": self.script.typing}))));
                        m.push_back(Micro::Pause(a.pause_ms));
                        return Ok(m);
                    }
                    let s = find(t)?;
                    m.extend([Micro::Glide { to: s.at, ms: speed_ms, carry: false }, Micro::Input { at: s.at, input: Input::Move }, Micro::Pause(100)]);
                    m.extend([Micro::Input { at: s.at, input: Input::Press { button: Button::Left, single: true } }, Micro::Pause(80), Micro::Input { at: s.at, input: Input::Release { button: Button::Left } }, Micro::Pause(200)]);
                }
                m.push_back(Micro::Cue);
                let delays = motion::typing_delays(text, self.script.typing, motion::seed_of(text));
                for (c, d) in text.chars().zip(delays) {
                    m.push_back(Micro::Char(c));
                    m.push_back(Micro::Pause(d));
                }
            }
            Act::Key { into, keys } => {
                if let Some(t) = into {
                    // ("app:" alone: what has the keyboard in the program Studio runs)
                    if t.app && t.what.is_empty() {
                        m.extend([Micro::Cue, Micro::App { req: json!({"op": "key", "keys": keys}), seq: None, then: AppThen::Nothing }, Micro::Pause(a.pause_ms)]);
                        return Ok(m);
                    }
                    if t.app {
                        m.push_back(app_find(t, AppThen::Act(json!({"op": "key", "target": target_json(t), "keys": keys}))));
                        m.push_back(Micro::Pause(a.pause_ms));
                        return Ok(m);
                    }
                    let s = find(t)?;
                    m.extend([Micro::Glide { to: s.at, ms: speed_ms, carry: false }, Micro::Input { at: s.at, input: Input::Press { button: Button::Left, single: true } }, Micro::Input { at: s.at, input: Input::Release { button: Button::Left } }, Micro::Pause(150)]);
                }
                m.push_back(Micro::Cue);
                for k in keys.split_whitespace() {
                    crate::hands::parse_key(k)?;
                    m.push_back(Micro::Keys(k.to_string()));
                    m.push_back(Micro::Pause(160));
                }
            }
            Act::Scroll { target, by } => {
                let s = find(target)?;
                m.extend([Micro::Glide { to: s.at, ms: speed_ms, carry: false }, Micro::Cue, Micro::Input { at: s.at, input: Input::Move }]);
                // (a notch at a time, as a wheel turns)
                let n = by.abs().round().max(1.0) as usize;
                for _ in 0..n {
                    m.push_back(Micro::Input { at: s.at, input: Input::Wheel { dy: by.signum() } });
                    m.push_back(Micro::Pause(90));
                }
            }
            Act::Wait(ms) => m.extend([Micro::Cue, Micro::Pause(*ms)]),
            Act::WaitFor { cond, timeout_ms } => m.extend([Micro::Until { cond: cond.clone(), timeout: *timeout_ms }, Micro::Cue]),
            Act::Highlight { target: None, .. } => m.push_back(Micro::Highlight { spot: None, spotlight: false, for_ms: None, form: None }),
            Act::Highlight { target: Some(t), spotlight, for_ms } if t.app => m.push_back(app_find(t, AppThen::Highlight { spotlight: *spotlight, for_ms: *for_ms })),
            Act::Highlight { target: Some(t), spotlight, for_ms } => {
                let s = find(t)?;
                m.extend([Micro::Highlight { spot: Some(s.rect), spotlight: *spotlight, for_ms: *for_ms, form: Some(s.form) }, Micro::Cue]);
            }
            Act::Do(cmds) => m.extend([Micro::Cue, Micro::Do { cmds: cmds.clone(), seq: None }]),
            Act::Mark(name) => m.push_back(Micro::Mark(name.clone())),
            Act::Pointer(on) => m.push_back(Micro::Pointer(*on)),
        }
        if !matches!(a.act, Act::Mark(_) | Act::Pointer(_) | Act::Highlight { .. }) {
            m.push_back(Micro::Pause(a.pause_ms));
        }
        Ok(m)
    }

    fn cue(&mut self) {
        let cue = self.run.as_ref().and_then(|r| r.cur.as_ref()).and_then(|c| c.cue.clone());
        if let (Some(cue), Some(run)) = (cue, self.run.as_mut()) {
            run.cues.insert(cue);
        }
    }

    fn deadline(&mut self, now: Instant, ms: u64) -> Instant {
        let c = self.run.as_mut().and_then(|r| r.cur.as_mut()).expect("an action");
        *c.deadline.get_or_insert(now + Duration::from_millis(ms))
    }

    fn exec<R: Program + Windows>(&mut self, rt: R, m: Micro, now: Instant) -> Next {
        match m {
            Micro::Glide { to, ms, carry } => {
                if let Some((_, _, start, dur)) = self.ptr.glide {
                    let k = now.saturating_duration_since(start).as_secs_f64() * 1000.0 / dur.max(1) as f64;
                    let at = self.ptr_at(now);
                    if carry {
                        rt.with_desk(&mut |desk, store| {
                            input::at_point(desk, store, at, &Input::Move);
                        });
                        rt.dispatch_pending();
                    }
                    if k >= 1.0 {
                        self.ptr.glide = None;
                        self.ptr.at = to;
                        return Next::Done;
                    }
                    return Next::Again;
                }
                if !self.ptr.shown {
                    self.ptr.shown = true;
                    self.ptr.since = Some(now);
                }
                let from = self.ptr.at;
                let d = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
                let dur = ms.unwrap_or_else(|| motion::glide_ms(d, self.script.speed));
                if dur == 0 {
                    self.ptr.at = to;
                    return Next::Done;
                }
                self.ptr.glide = Some((from, to, now, dur));
                Next::Again
            }
            Micro::Cue => {
                self.cue();
                Next::Done
            }
            Micro::Input { at, input } => {
                if matches!(input, Input::Press { .. }) {
                    self.ptr.pressed = true;
                    self.ptr.ripple = Some((at, now));
                }
                if matches!(input, Input::Release { .. }) {
                    self.ptr.pressed = false;
                }
                self.ptr.at = at;
                let mut hit = true;
                rt.with_desk(&mut |desk, store| hit = input::at_point(desk, store, at, &input));
                rt.dispatch_pending();
                if !hit {
                    self.fail(&format!("no window of the program is at {:.0},{:.0} of the screen", at.0, at.1));
                }
                Next::Done
            }
            Micro::Char(c) => {
                rt.with_desk(&mut |desk, store| input::type_char(desk, store, c));
                rt.dispatch_pending();
                Next::Done
            }
            Micro::Keys(k) => {
                let mut r = Ok(());
                rt.with_desk(&mut |desk, store| r = input::keys(desk, store, &k));
                rt.dispatch_pending();
                if let Err(e) = r {
                    self.fail(&e);
                }
                Next::Done
            }
            Micro::Pause(ms) => Next::After(ms),
            Micro::Until { cond, timeout } => {
                let deadline = self.deadline(now, timeout);
                if self.holds(rt, &cond) {
                    return Next::Done;
                }
                if now >= deadline {
                    self.fail(&format!("waited {:.0} s for {} — it never happened", timeout as f64 / 1000.0, describe_cond(&cond)));
                }
                Next::Again
            }
            Micro::Do { cmds, seq: None } => {
                self.seq_do += 1;
                let n = self.seq_do;
                let _ = std::fs::write(self.out.join("inbox").join(format!("do-{n}.txt")), format!("{}\n", cmds.replace('\n', ",")));
                Next::Replace(vec![Micro::Do { cmds, seq: Some(n) }])
            }
            Micro::Do { cmds, seq: Some(n) } => {
                // (Studio takes a command each macro tick, 0.4 s of real time)
                let deadline = self.deadline(now, 90_000);
                if self.out.join("inbox").join(format!("done-{n}")).exists() {
                    return Next::Done;
                }
                if now >= deadline {
                    self.fail(&format!("Studio never ran \"{cmds}\" (was it started with --do demo:poll?)"));
                }
                Next::Again
            }
            Micro::App { req, seq: None, then } => {
                self.seq_app += 1;
                let n = self.seq_app;
                let dir = self.out.join("app");
                let tmp = dir.join(format!("in-{n}.tmp"));
                let _ = std::fs::write(&tmp, req.to_string());
                let _ = std::fs::rename(&tmp, dir.join(format!("in-{n}.json")));
                Next::Replace(vec![Micro::App { req, seq: Some(n), then }])
            }
            Micro::App { req, seq: Some(n), then } => {
                let deadline = self.deadline(now, 60_000);
                let file = self.out.join("app").join(format!("out-{n}.json"));
                let Some(reply) = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<Json>(&t).ok()) else {
                    if now >= deadline {
                        self.fail("the program Studio runs never answered (is it running? `waitFor`: { \"app\": true } first)");
                    }
                    return Next::Again;
                };
                if reply["ok"] != Json::Bool(true) {
                    self.fail(&format!("in the program Studio runs: {}", reply["text"].as_str().unwrap_or("it failed")));
                }
                let pt = |k: &str| reply[k].as_array().map(|a| a.iter().filter_map(Json::as_f64).collect::<Vec<f64>>()).unwrap_or_default();
                let at = pt("at");
                let rect = pt("rect");
                let at = if at.len() == 2 { (at[0], at[1]) } else { self.ptr.at };
                let _ = req;
                Next::Replace(match then {
                    AppThen::Nothing => vec![],
                    AppThen::Hover => vec![Micro::Glide { to: at, ms: None, carry: false }, Micro::Cue],
                    AppThen::Act(next) => {
                        // (a click's ring where it presses, as in Studio)
                        let mut v = vec![Micro::Glide { to: at, ms: None, carry: false }, Micro::Cue, Micro::Pause(110)];
                        if next["op"] == "click" {
                            v.push(Micro::Ripple);
                        }
                        v.push(Micro::App { req: next, seq: None, then: AppThen::Nothing });
                        v
                    }
                    AppThen::Highlight { spotlight, for_ms } => {
                        let r = if rect.len() == 4 { Some((rect[0], rect[1], rect[2], rect[3])) } else { None };
                        vec![Micro::Highlight { spot: r, spotlight, for_ms, form: None }, Micro::Cue]
                    }
                })
            }
            Micro::Highlight { spot, spotlight, for_ms, form } => {
                self.highlight = spot.map(|(x, y, w, h)| (Highlight { rect: (x.round() as i64, y.round() as i64, w.round() as i64, h.round() as i64), spotlight, alpha: 255 }, for_ms.map(|ms| now + Duration::from_millis(ms)), form));
                Next::Done
            }
            Micro::Mark(name) => {
                let ms = self.clock.as_ref().map_or(0, |c| c.ms(now));
                self.marks.insert(name, json!(ms));
                Next::Done
            }
            Micro::Ripple => {
                self.ptr.ripple = Some((self.ptr.at, now));
                Next::Done
            }
            Micro::Pointer(on) => {
                if on {
                    if !self.ptr.shown {
                        self.ptr.shown = true;
                        self.ptr.since = Some(now);
                    }
                } else {
                    self.ptr_hide(now);
                }
                Next::Done
            }
        }
    }

    fn holds<R: Program + Windows>(&mut self, rt: R, cond: &Cond) -> bool {
        match cond {
            Cond::App(want) => {
                let has = !self.app_layers().is_empty();
                has == *want
            }
            _ => {
                let mut ok = false;
                rt.with_desk(&mut |desk, store| {
                    ok = match cond {
                        Cond::Window(w) => crate::eyes::find_window(desk, w).is_ok(),
                        Cond::Gone(w) => crate::eyes::find_window(desk, w).is_err(),
                        Cond::Shows(t) => input::locate(desk, store, t).is_ok(),
                        Cond::Prop { path, value, exact } => {
                            let (c, p) = path.split_once('.').unwrap_or((path, ""));
                            let v = rapidr_ui_kernel::store::string(store, &c.to_lowercase(), &p.to_lowercase());
                            if *exact { v == *value } else { v.contains(value.as_str()) }
                        }
                        Cond::App(_) => false,
                    }
                });
                ok
            }
        }
    }

    // ---------------------------------------------------------- the pointer --

    fn ptr_at(&self, now: Instant) -> (f64, f64) {
        match self.ptr.glide {
            Some((from, to, start, ms)) => {
                let k = now.saturating_duration_since(start).as_secs_f64() * 1000.0 / ms.max(1) as f64;
                motion::glide_at(from, to, k.min(1.0))
            }
            None => self.ptr.at,
        }
    }

    fn ptr_hide(&mut self, now: Instant) {
        if self.ptr.shown {
            self.ptr.shown = false;
            self.ptr.since = Some(now);
        }
    }

    // ----------------------------------------------------------- the frame --

    fn app_layers(&mut self) -> Vec<Layer> {
        let file = self.out.join("app").join("layers.bin");
        let Ok(meta) = std::fs::metadata(&file) else {
            self.layers = (None, Vec::new());
            return Vec::new();
        };
        let modified = meta.modified().ok();
        // (stale: the program has ended — it writes them a few times a second)
        if modified.and_then(|m| m.elapsed().ok()).is_some_and(|age| age > Duration::from_millis(900)) || self.out.join("app").join("ended").exists() {
            return Vec::new();
        }
        if modified != self.layers.0 {
            if let Ok(bytes) = std::fs::read(&file) {
                if let Some(l) = app::read_layers(&bytes) {
                    self.layers = (modified, l);
                }
            }
        }
        self.layers.1.clone()
    }

    fn picture(&mut self, path: &str, height: Option<i64>) -> Option<Arc<Picture>> {
        let key = format!("{path}@{height:?}");
        if let Some(p) = self.pictures.get(&key) {
            return p.clone();
        }
        let full = if Path::new(path).is_absolute() { PathBuf::from(path) } else { self.base.join(path) };
        let pic = std::fs::read(&full).ok().and_then(|b| {
            let scale = self.script.scale as f32;
            let (px, alpha) = if path.to_lowercase().ends_with(".svg") {
                // (drawn at the frame's scale, sharp; to the height asked)
                let probe = rapidr_value::objects::codec::decode_svg(&b, 1.0).ok()?;
                let k = height.map_or(1.0, |h| h as f32 / probe.0.height.max(1) as f32);
                let (px, a) = rapidr_value::objects::codec::decode_svg(&b, k * scale).ok()?;
                (px, Some(a))
            } else {
                rapidr_value::objects::codec::decode_png(&b).ok()?
            };
            let mut rgba = Vec::with_capacity(px.pixels.len() * 4);
            for (i, c) in px.pixels.iter().enumerate() {
                rgba.extend_from_slice(&[(c & 0xFF) as u8, (c >> 8 & 0xFF) as u8, (c >> 16 & 0xFF) as u8, alpha.as_ref().map_or(255, |a| a[i])]);
            }
            Some(Arc::new(Picture { width: px.width, height: px.height, rgba }))
        });
        if pic.is_none() {
            eprintln!("[rapidr] demo: can't read the picture {}", full.display());
        }
        self.pictures.insert(key, pic.clone());
        pic
    }

    fn overlay(&mut self, now: Instant) -> Overlay {
        let size = self.script.screen;
        let mut o = Overlay { size, ..Overlay::default() };
        let fade = |since: Option<Instant>, on: bool| -> u8 {
            let k = since.map_or(1.0, |s| (now.saturating_duration_since(s).as_secs_f64() / FADE.as_secs_f64()).min(1.0));
            let a = if on { k } else { 1.0 - k };
            (a * 255.0).round() as u8
        };
        if let Some((h, until, _)) = &self.highlight {
            if until.is_some_and(|u| now >= u) {
                self.highlight = None;
            } else {
                o.highlight = Some(h.clone());
            }
        }
        let alpha = fade(self.ptr.since, self.ptr.shown);
        if alpha > 0 {
            let ripple = self.ptr.ripple.and_then(|(at, t)| {
                let k = now.saturating_duration_since(t).as_secs_f64() / RIPPLE.as_secs_f64();
                (k < 1.0).then_some((at, k))
            });
            if ripple.is_none() {
                self.ptr.ripple = None;
            }
            o.pointer = Some(Pointer { at: self.ptr_at(now), alpha, pressed: self.ptr.pressed, ripple, size: self.script.pointer_size });
        }
        if let Some((lines, width, from, until)) = &self.caption {
            if now >= *until + FADE {
                self.caption = None;
            } else {
                let a = if now >= *until { fade(Some(*until), false) } else { fade(Some(*from), true) };
                o.caption = Some(Caption { lines: lines.clone(), px: self.script.caption_px, width: *width, alpha: a });
            }
        }
        if let Some((spec, from, fading)) = self.card.clone() {
            let a = match fading {
                Some(f) => {
                    let k = now.saturating_duration_since(f).as_secs_f64() / CARD_FADE.as_secs_f64();
                    if k >= 1.0 {
                        self.card = None;
                        0
                    } else {
                        (255.0 * (1.0 - k)).round() as u8
                    }
                }
                // (a card the video opens on shows whole from its first frame)
                None if self.clock.as_ref().is_some_and(|c| from <= c.base) => 255,
                None => (255.0 * (now.saturating_duration_since(from).as_secs_f64() / CARD_FADE.as_secs_f64()).min(1.0)).round() as u8,
            };
            if a > 0 {
                let bg = spec.background.as_deref().and_then(|p| self.picture(p, None)).map(|p| (p, 1));
                let logo = spec.logo.as_deref().and_then(|p| self.picture(p, Some(spec.logo_height))).map(|p| {
                    let (lw, lh) = ((p.width as f64 / self.script.scale).round() as i64, (p.height as f64 / self.script.scale).round() as i64);
                    (p, 1, (lw, lh))
                });
                o.card = Some(Card { title: spec.title.clone(), lines: spec.lines.clone(), small: spec.small.clone(), background: bg, logo, light_text: spec.light_text, alpha: a });
                // (a card covers the pointer and the captions' strip: the
                // narration's words are in the .srt still)
                if a == 255 {
                    o.pointer = None;
                    o.caption = None;
                }
            }
        }
        o
    }

    fn frame<R: Program + Windows>(&mut self, rt: R, now: Instant) {
        // (a ring around a control of a window that has closed — a dialog
        // dismissed meanwhile — goes with it, never left over what's behind)
        if let Some(form) = self.highlight.as_ref().and_then(|(_, _, f)| f.clone()) {
            let mut shown = true;
            rt.with_desk(&mut |desk, _| shown = desk.forms.get(&form).is_some_and(|f| f.shown));
            if !shown {
                self.highlight = None;
            }
        }
        let overlay = self.overlay(now);
        let layers = self.app_layers();
        let ms = self.clock.as_ref().map_or(0, |c| c.ms(now));
        let Some(mut rec) = self.rec.take() else { return };
        let mut px = None;
        rt.with_desk(&mut |desk, store| px = Some(rec.compose(desk, store, &layers, &overlay)));
        if let Some(px) = px {
            rec.push(ms, px);
        }
        self.rec = Some(rec);
    }

    fn write_timeline(&self, done: bool) {
        let (frames, end) = match (&self.rec, &self.clock) {
            (Some(r), Some(c)) => (r.index.len(), c.ms(c.at(c.frame))),
            _ => (0, 0),
        };
        let s = self.script;
        let doc = json!({
            "id": s.id, "title": s.title, "fps": s.fps, "screen": [s.screen.0, s.screen.1], "scale": s.scale,
            "size": [((s.screen.0 as f64) * s.scale).round(), ((s.screen.1 as f64) * s.scale).round()],
            "frames": frames, "end": end, "done": done,
            "lines": self.timeline, "marks": self.marks,
        });
        let _ = std::fs::write(self.out.join("timeline.json"), serde_json::to_string_pretty(&doc).unwrap_or_default());
    }

    fn finish(&mut self) -> ! {
        let written = self.rec.as_ref().map_or(0, Recorder::written);
        let frames = self.rec.as_ref().map_or(0, |r| r.index.len());
        if let Some(rec) = self.rec.as_mut() {
            if let Err(e) = rec.finish() {
                self.fail(&e);
            }
            if let Some(last) = &rec.last {
                if let Ok(png) = rapidr_value::objects::codec::encode_png(last) {
                    let _ = std::fs::write(self.out.join("last.png"), png);
                }
            }
        }
        self.write_timeline(true);
        let _ = std::fs::write(self.out.join("app").join("exit"), "exit");
        let secs = self.clock.as_ref().map_or(0.0, |c| c.ms(c.at(c.frame)) as f64 / 1000.0);
        eprintln!("[rapidr] demo: recorded {secs:.1} s, {frames} frames ({written} distinct) into {}", self.out.display());
        let _ = std::fs::write(self.out.join("done"), "done");
        std::process::exit(0);
    }
}

/// What `exec` says next.
enum Next {
    /// Done: the next micro-step now.
    Done,
    /// Done: the next one after this long.
    After(u64),
    /// Not yet: again next frame.
    Again,
    /// Done, these first.
    Replace(Vec<Micro>),
}

fn app_find(t: &Target, then: AppThen) -> Micro {
    Micro::App { req: json!({"op": "find", "target": {"what": t.what, "item": t.item, "x": t.fx, "y": t.fy, "window": t.window, "at": t.at}}), seq: None, then }
}

fn describe(a: &Act) -> String {
    let t = |t: &Target| format!("{}{}{}", if t.app { "app:" } else { "" }, t.what, t.item.as_ref().map(|i| format!(" > {i}")).unwrap_or_default());
    match a {
        Act::Move(x) => format!("move to {}", t(x)),
        Act::Click { target, double: true, .. } => format!("double-click {}", t(target)),
        Act::Click { target, .. } => format!("click {}", t(target)),
        Act::Drag { from, to } => format!("drag {} to {}", t(from), t(to)),
        Act::Type { text, .. } => format!("type \"{}\"", text.chars().take(30).collect::<String>()),
        Act::Key { keys, .. } => format!("key {keys}"),
        Act::Scroll { target, .. } => format!("scroll {}", t(target)),
        Act::Wait(ms) => format!("wait {ms} ms"),
        Act::WaitFor { cond, .. } => format!("wait for {}", describe_cond(cond)),
        Act::Highlight { target: Some(x), .. } => format!("highlight {}", t(x)),
        Act::Highlight { .. } => "highlight off".into(),
        Act::Do(c) => format!("do {c}"),
        Act::Mark(m) => format!("mark {m}"),
        Act::Pointer(_) => "pointer".into(),
    }
}

fn describe_cond(c: &Cond) -> String {
    match c {
        Cond::Window(w) => format!("the window {w}"),
        Cond::Gone(w) => format!("the window {w} to close"),
        Cond::Shows(t) => format!("{} to show", t.what),
        Cond::Prop { path, value, exact: true } => format!("{path} to be \"{value}\""),
        Cond::Prop { path, value, .. } => format!("{path} to have \"{value}\""),
        Cond::App(true) => "the program Studio runs to show its window".into(),
        Cond::App(false) => "the program Studio runs to end".into(),
    }
}

/// `text` in lines no wider than `max` (logical pixels) at `px`: the lines
/// and the widest one's width.
fn wrap(ts: &mut rapidr_ui_kernel::TextSystem, text: &str, px: i64, max: f32) -> (Vec<String>, i64) {
    let font = demo_overlay::font(px, false);
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let next = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if !cur.is_empty() && ts.measure(&next, &font).0 > max {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        } else {
            cur = next;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    // (two lines balanced rather than a long one and a word)
    if lines.len() == 2 {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut best = (f32::MAX, lines.clone());
        for i in 1..words.len() {
            let (a, b) = (words[..i].join(" "), words[i..].join(" "));
            let w = ts.measure(&a, &font).0.max(ts.measure(&b, &font).0);
            if w <= max && w < best.0 {
                best = (w, vec![a, b]);
            }
        }
        lines = best.1;
    }
    let width = lines.iter().map(|l| ts.measure(l, &font).0.ceil() as i64).max().unwrap_or(0);
    (lines, width)
}
