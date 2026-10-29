//! The hands: input for the program's own windows from an AI (RapidR
//! Studio's AI apps, RAI) or from the program itself (RAppDriver) —
//! clicks, double clicks, typing, keys with modifiers, drags, the wheel,
//! waits — found by a component's name, by what it shows (a button's
//! caption, the label beside an edit, as a person finds them) or at a
//! point of a window, and played through the kernel's own routing, as the
//! user's input is ([`Desktop::mouse_down`] …: hit test, focus, the
//! models, the program's events). Never the system's mouse or keyboard:
//! nothing outside the program can be touched, and no permission of the
//! system's (Accessibility, Screen Recording) is needed.
//!
//! A visible virtual cursor glides to what it clicks
//! (`rapidr_ui_kernel::virtual_cursor`, drawn by the kernel over the
//! window), a ring shows each click; where the person asked the system
//! for reduced motion it jumps and the ring stays still. It can be turned
//! off ([`set_cursor_shown`]).
//!
//! Work comes as jobs ([`submit`]): each one's target is found when its
//! turn comes (what an earlier one opened is there by then), its steps
//! are played one per turn of the runtime's loop ([`step`], with the
//! host's [`Windows::with_desk`]), the program's handlers run between
//! them, and once they have run its outcome waits for the caller
//! ([`take_done`]): a sentence, or an error that says what to do next
//! ("No button shows \"Go\". Buttons now: Start, Reset.").
//!
//! A window under a modal one takes no input, as the user's wouldn't: the
//! error names the window that waits for an answer.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;

use rapidr_ui_kernel::tick::Instant;
use rapidr_ui_kernel::virtual_cursor::{self, VirtualCursor};
use rapidr_ui_kernel::{Mods, Store};
use rapidr_value::input::Button;

use crate::desktop::{Desktop, Source};
use crate::eyes;
use crate::{Program, Windows};

/// What a job acts on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Target {
    /// A component's name (any case).
    pub name: Option<String>,
    /// What it shows: a button's caption, a check box's (`&` and case
    /// don't matter).
    pub caption: Option<String>,
    /// The label beside it: the edit (or list …) nearest a label that says
    /// this, to its right or under it.
    pub label: Option<String>,
    /// A window (a form's name or title; the front one when none is given)
    /// for a point.
    pub window: Option<String>,
    /// A point in that window's inside (logical pixels; an in-window menu
    /// bar included): what the layout report's x, y are.
    pub at: Option<(f64, f64)>,
}

impl Target {
    /// One string as a program writes it (`Click "7"`): a component's name,
    /// else a caption, else a label — or forced with `name:`, `caption:`,
    /// `label:`.
    pub fn parse(s: &str) -> Target {
        let t = s.trim();
        for (p, k) in [("name:", 0), ("caption:", 1), ("label:", 2)] {
            if t.len() >= p.len() && t[..p.len()].eq_ignore_ascii_case(p) {
                let v = Some(t[p.len()..].trim().to_string());
                return match k {
                    0 => Target { name: v, ..Target::default() },
                    1 => Target { caption: v, ..Target::default() },
                    _ => Target { label: v, ..Target::default() },
                };
            }
        }
        Target { name: Some(t.to_string()), caption: Some(t.to_string()), label: Some(t.to_string()), ..Target::default() }
    }

    fn is_empty(&self) -> bool {
        self.name.is_none() && self.caption.is_none() && self.label.is_none() && self.at.is_none()
    }

    /// How it was asked for, in words.
    pub fn said(&self) -> String {
        if let Some((x, y)) = self.at {
            return format!("{},{} of {}", x, y, self.window.as_deref().unwrap_or("the front window"));
        }
        self.caption.clone().or_else(|| self.name.clone()).or_else(|| self.label.clone()).unwrap_or_default()
    }
}

/// What a job does.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// A click (`double`: two), with the left or right button.
    Click { target: Target, button: Button, double: bool },
    /// The mouse moved onto it (its hint, its hover look).
    Hover { target: Target },
    /// Text typed into it (clicked first) — or into what has the keyboard
    /// when no target is given.
    Type { target: Target, text: String },
    /// Keys pressed: `Enter`, `Ctrl+S`, `Shift+Tab`, `Alt+F4`, several
    /// separated by spaces (`Tab Tab Enter`); into the target when given
    /// (it gets the keyboard), else what has it.
    Key { target: Target, keys: String },
    /// Pressed on `from`, moved to `to` (a target, or a point of the same
    /// window), let go there.
    Drag { from: Target, to: Target },
    /// The wheel turned over it: `dy` notches down (negative: up), `dx`
    /// right.
    Scroll { target: Target, dx: f64, dy: f64 },
    /// The program runs on its own for `ms` milliseconds (its timers, its
    /// work); `idle`: until nothing of it is left to run, at most `ms`.
    Wait { ms: u64, idle: bool },
}

impl Act {
    pub fn verb(&self) -> &'static str {
        match self {
            Act::Click { double: true, .. } => "double-click",
            Act::Click { button: Button::Right, .. } => "right-click",
            Act::Click { .. } => "click",
            Act::Hover { .. } => "hover",
            Act::Type { .. } => "type",
            Act::Key { .. } => "key",
            Act::Drag { .. } => "drag",
            Act::Scroll { .. } => "scroll",
            Act::Wait { .. } => "wait",
        }
    }
}

/// Who asked for a job, and how they hear its end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Owner {
    /// RapidR Studio's link (the session's request number).
    Link(u64),
    /// A program's RAppDriver (its name, its job's number).
    Driver(String, i64),
}

/// A job's end: what it did, in a sentence, or why it couldn't.
#[derive(Clone, Debug, PartialEq)]
pub struct Done {
    pub owner: Owner,
    pub ok: bool,
    pub text: String,
}

/// One thing a job does in a turn.
#[derive(Clone, Debug, PartialEq)]
enum Step {
    /// The cursor glides to `to` in `form`'s window over `ms` (from where
    /// it is; jumps when it was in another window or hidden).
    Glide { form: String, to: (f64, f64), ms: u64 },
    /// The mouse moved there (hover), as the user's.
    Move { form: String, at: (f64, f64) },
    Press { form: String, at: (f64, f64), button: Button, single: bool },
    Release { form: String, at: (f64, f64), button: Button },
    Focus { form: String, comp: String },
    Key { form: String, vk: i64, text: String, mods: Mods },
    /// Text no key types here (é, 中): an input method's commit.
    Commit { form: String, text: String },
    Wheel { form: String, at: (f64, f64), d: (f64, f64) },
    Pause(u64),
    /// Until the program has nothing left to run, at most this long.
    Idle(u64),
}

struct Running {
    owner: Owner,
    steps: VecDeque<Step>,
    next: Instant,
    /// The sentence said at its end.
    said: String,
    /// The windows shown when it started (what it opened or closed is said).
    windows: Vec<String>,
    /// An idle wait's deadline, and the turns seen quiet.
    until: Option<Instant>,
    quiet: u8,
    /// Its last step's handlers are being given a turn to run.
    settling: u8,
}

/// A glide: from, to, when it started, how long (ms).
type Glide = ((f64, f64), (f64, f64), Instant, u64);

/// The cursor as the hands keep it (the kernel draws
/// `virtual_cursor::get`).
#[derive(Clone, Debug, Default)]
struct Cursor {
    form: String,
    at: (f64, f64),
    /// A glide: from, to, when it started, how long.
    glide: Option<Glide>,
    ripple: Option<((f64, f64), Instant)>,
    pressed: bool,
    /// When it was last used (it fades out a while after).
    used: Option<Instant>,
    shown: bool,
}

struct Hands {
    queue: VecDeque<(Owner, Act)>,
    running: Option<Running>,
    done: Vec<Done>,
    cursor: Cursor,
    /// The cursor is drawn (off: input is played at once, unseen).
    show: bool,
    /// How fast: 1 normal, 2 twice as fast, 0 at once.
    speed: f64,
}

impl Default for Hands {
    fn default() -> Self {
        Hands { queue: VecDeque::new(), running: None, done: Vec::new(), cursor: Cursor::default(), show: true, speed: 1.0 }
    }
}

thread_local! {
    static HANDS: RefCell<Hands> = RefCell::new(Hands::default());
}

fn hands<R>(f: impl FnOnce(&mut Hands) -> R) -> R {
    HANDS.with(|h| f(&mut h.borrow_mut()))
}

/// How long the cursor stays after the last action, and how long it fades.
const LINGER: Duration = Duration::from_millis(1500);
const FADE: Duration = Duration::from_millis(400);
/// A click's ring.
const RIPPLE: Duration = Duration::from_millis(450);
/// A frame (animations' turns).
const FRAME: Duration = Duration::from_millis(16);

/// A job, after those already waiting.
pub fn submit(owner: Owner, act: Act) {
    hands(|h| h.queue.push_back((owner, act)));
}

/// Jobs waiting or running.
pub fn busy() -> bool {
    hands(|h| h.running.is_some() || !h.queue.is_empty())
}

/// How many jobs of `driver` (a RAppDriver) wait or run.
pub fn pending_of(driver: &str) -> usize {
    let mine = |o: &Owner| matches!(o, Owner::Driver(d, _) if d.eq_ignore_ascii_case(driver));
    hands(|h| h.queue.iter().filter(|(o, _)| mine(o)).count() + usize::from(h.running.as_ref().is_some_and(|r| mine(&r.owner))))
}

/// Drops the jobs waiting (not the one running) of `owner`'s kind: a
/// driver's (by name) or every link's.
pub fn cancel_driver(driver: &str) -> usize {
    hands(|h| {
        let before = h.queue.len();
        h.queue.retain(|(o, _)| !matches!(o, Owner::Driver(d, _) if d.eq_ignore_ascii_case(driver)));
        before - h.queue.len()
    })
}

/// The jobs ended since the last call.
pub fn take_done() -> Vec<Done> {
    hands(|h| std::mem::take(&mut h.done))
}

/// The virtual cursor drawn (on by default) or not.
pub fn set_cursor_shown(on: bool) {
    hands(|h| h.show = on);
}

pub fn cursor_shown() -> bool {
    hands(|h| h.show)
}

/// How fast the cursor moves: 1 normal, 2 twice as fast, 0 at once.
pub fn set_speed(speed: f64) {
    hands(|h| h.speed = speed.clamp(0.0, 20.0));
}

pub fn speed() -> f64 {
    hands(|h| h.speed)
}

/// When the hands want the next turn (`None`: nothing to do).
pub fn next_wake() -> Option<Instant> {
    hands(|h| {
        let now = rapidr_ui_kernel::tick::now();
        let mut at: Option<Instant> = None;
        let mut soon = |t: Instant| at = Some(at.map_or(t, |a: Instant| a.min(t)));
        if let Some(r) = &h.running {
            soon(r.next);
        } else if !h.queue.is_empty() {
            soon(now);
        }
        let c = &h.cursor;
        if c.glide.is_some() || c.ripple.is_some() {
            soon(now + FRAME);
        }
        if c.shown {
            if let Some(used) = c.used {
                let gone = used + LINGER;
                soon(if now < gone { gone } else { now + FRAME });
            }
        }
        at
    })
}

// ------------------------------------------------------------ the targets --

/// What was found: the window, the component (none for a bare point), the
/// point clicked (the window's inside), and how to say it.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub form: String,
    pub comp: Option<String>,
    pub at: (f64, f64),
    pub what: String,
}

fn clean(s: &str) -> String {
    s.replace('&', "").trim().to_string()
}

fn button_like(ty: &str) -> bool {
    matches!(ty, "RBUTTON" | "RBITBTN" | "RSPEEDBUTTON" | "ROVALBTN" | "RCOOLBTN")
}

fn input_like(ty: &str) -> bool {
    matches!(ty, "REDIT" | "RMEMO" | "RRICHEDIT" | "RCOMBOBOX" | "RSPINEDIT" | "RMASKEDIT" | "RLISTBOX" | "RCHECKLISTBOX" | "RLISTVIEW" | "RSTRINGGRID" | "RTREEVIEW" | "RTRACKBAR" | "RCODEEDITOR" | "RDATETIMEPICKER")
}

/// A component as a person reads it: `b7 (RButton "7")`.
fn said_comp(store: &dyn Store, id: &str, ty: &str) -> String {
    let cap = clean(&rapidr_ui_kernel::store::string(store, id, "caption"));
    if cap.is_empty() {
        format!("{id} ({})", eyes::type_name(ty))
    } else {
        format!("{id} ({} \"{cap}\")", eyes::type_name(ty))
    }
}

/// The windows that take input now, front first (under a modal window
/// only it, and windows shown from it).
fn input_windows(desk: &Desktop) -> Vec<String> {
    desk.stacking().into_iter().rev().filter(|f| desk.accepts_input(f)).collect()
}

/// Why `form` takes no input now (a modal window over it), if it doesn't.
fn blocked(desk: &Desktop, form: &str) -> Option<String> {
    if desk.accepts_input(form) {
        return None;
    }
    let m = desk.modal.last().cloned().unwrap_or_default();
    let title = desk.forms.get(&m).map(|w| w.spec.title.clone()).unwrap_or_default();
    Some(format!("{m} (\"{title}\") is open in front and waits for an answer (it's modal), so {form} takes no input: act in {m} first (close it or press one of its buttons)."))
}

/// The component (or point) `t` names, in the windows shown.
pub fn find(desk: &mut Desktop, store: &dyn Store, t: &Target) -> Result<Found, String> {
    let shown = desk.stacking();
    if shown.is_empty() {
        return Err("The program shows no window now: nothing to act on.".into());
    }
    for f in &shown {
        if let Some(w) = desk.forms.get_mut(f) {
            w.ui.sync(store);
        }
    }
    // (a point of a window)
    if let Some((x, y)) = t.at {
        let form = eyes::find_window(desk, t.window.as_deref().unwrap_or(""))?;
        let ui = &desk.forms[&form].ui;
        let (w, h) = (ui.client.0, ui.client.1 + ui.menu_offset);
        if x < 0.0 || y < 0.0 || x >= w as f64 || y >= h as f64 {
            return Err(format!("{x},{y} is outside {form}'s inside ({w} × {h}): give a point inside it (app_layout gives each component's x, y, width and height)."));
        }
        let comp = ui.nodes.iter().rev().find(|n| n.shown && (x as i64) >= n.abs.0 && (y as i64) >= n.abs.1 && (x as i64) < n.abs.0 + n.abs.2 && (y as i64) < n.abs.1 + n.abs.3).map(|n| n.id.clone());
        let what = match &comp {
            Some(c) => format!("{x},{y} of {form} ({})", said_comp(store, c, &ui.node(c).map(|n| n.type_name.clone()).unwrap_or_default())),
            None => format!("{x},{y} of {form}"),
        };
        return Ok(Found { form, comp, at: (x, y), what });
    }
    // (the windows looked in: the one named, else every one shown, front first)
    let forms: Vec<String> = match &t.window {
        Some(w) if !w.trim().is_empty() => vec![eyes::find_window(desk, w)?],
        _ => shown.iter().rev().cloned().collect(),
    };
    let center = |n: &rapidr_ui_kernel::tree::Node| (n.abs.0 as f64 + n.abs.2 as f64 / 2.0, n.abs.1 as f64 + n.abs.3 as f64 / 2.0);
    // (by name)
    if let Some(name) = t.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        for f in &forms {
            if f.eq_ignore_ascii_case(name) {
                let ui = &desk.forms[f].ui;
                let at = (ui.client.0 as f64 / 2.0, (ui.client.1 + ui.menu_offset) as f64 / 2.0);
                return Ok(Found { form: f.clone(), comp: None, at, what: format!("the window {f}") });
            }
            let ui = &desk.forms[f].ui;
            if let Some(n) = ui.nodes.iter().find(|n| n.id.eq_ignore_ascii_case(name)) {
                if !n.shown {
                    return Err(format!("{} is hidden (it or what holds it has Visible = False): show it first, or act on what shows.", n.id));
                }
                return Ok(Found { form: f.clone(), comp: Some(n.id.clone()), at: center(n), what: said_comp(store, &n.id, &n.type_name) });
            }
        }
    }
    // (by caption: buttons first, then whatever shows that caption)
    if let Some(cap) = t.caption.as_deref().map(clean).filter(|c| !c.is_empty()) {
        for pass in 0..2 {
            for f in &forms {
                let ui = &desk.forms[f].ui;
                let hit = ui.nodes.iter().find(|n| n.shown && (pass == 1 || button_like(&n.type_name)) && clean(&rapidr_ui_kernel::store::string(store, &n.id, "caption")).eq_ignore_ascii_case(&cap));
                if let Some(n) = hit {
                    return Ok(Found { form: f.clone(), comp: Some(n.id.clone()), at: center(n), what: said_comp(store, &n.id, &n.type_name) });
                }
            }
        }
    }
    // (by the label beside it)
    if let Some(lab) = t.label.as_deref().map(clean).filter(|c| !c.is_empty()) {
        let bare = |s: &str| clean(s).trim_end_matches(':').trim().to_lowercase();
        for f in &forms {
            let ui = &desk.forms[f].ui;
            let Some(l) = ui.nodes.iter().find(|n| n.shown && n.type_name == "RLABEL" && bare(&rapidr_ui_kernel::store::string(store, &n.id, "caption")) == bare(&lab)) else { continue };
            let (lx, ly) = (l.abs.0, l.abs.1);
            let near = ui.nodes.iter().filter(|n| n.shown && input_like(&n.type_name) && n.abs.1 >= ly - 12 && n.abs.0 >= lx - 12).min_by_key(|n| (n.abs.1 - ly).abs() * 3 + (n.abs.0 - lx).abs());
            if let Some(n) = near {
                return Ok(Found { form: f.clone(), comp: Some(n.id.clone()), at: center(n), what: format!("{} (beside the label \"{}\")", said_comp(store, &n.id, &n.type_name), clean(&rapidr_ui_kernel::store::string(store, &l.id, "caption"))) });
            }
        }
    }
    Err(not_found(desk, store, t, &forms))
}

/// What to say when nothing matches: what was asked for, and what there is
/// (the buttons' captions, the names), so the next call can be right.
fn not_found(desk: &Desktop, store: &dyn Store, t: &Target, forms: &[String]) -> String {
    let mut buttons = Vec::new();
    let mut others = Vec::new();
    let mut names = Vec::new();
    let mut labels = Vec::new();
    for f in forms {
        let ui = &desk.forms[f].ui;
        for n in ui.nodes.iter().filter(|n| n.shown) {
            let cap = clean(&rapidr_ui_kernel::store::string(store, &n.id, "caption"));
            if button_like(&n.type_name) && !cap.is_empty() {
                buttons.push(cap.clone());
            } else if n.type_name == "RLABEL" && !cap.is_empty() {
                labels.push(cap.clone());
            } else if !cap.is_empty() {
                others.push(cap.clone());
            }
            names.push(n.id.clone());
        }
    }
    let list = |v: &[String]| {
        let mut v: Vec<String> = v.iter().take(16).map(|s| format!("\"{s}\"")).collect();
        v.dedup();
        if v.is_empty() { "none".to_string() } else { v.join(", ") }
    };
    let asked = match (&t.caption, &t.name, &t.label) {
        (Some(c), Some(n), Some(_)) if c == n => format!("No component named {n} and nothing shows \"{c}\""),
        (Some(c), _, _) => format!("No button (or other control) shows \"{c}\""),
        (_, Some(n), _) => format!("No component named {n}"),
        (_, _, Some(l)) => format!("No edit or list beside a label \"{l}\""),
        _ => "Nothing to act on was named (give a name, a caption, a label, or x and y)".into(),
    };
    let names: Vec<String> = names.into_iter().take(24).collect();
    format!("{asked} in {}. Buttons now: {}. Other captions: {}. Labels: {}. Names: {}.", forms.join(", "), list(&buttons), list(&others), list(&labels), if names.is_empty() { "none".into() } else { names.join(", ") })
}

// ----------------------------------------------------------------- the keys --

/// `Ctrl+S`, `Shift+Tab`, `Enter`, `F5`, `a` → the virtual key, the text
/// it types and the modifiers held. Ctrl is the shortcut key on every OS
/// (as a GUI test's `__key_N_16`): a Mac's Command shortcuts too.
pub fn parse_key(spec: &str) -> Result<(i64, String, Mods), String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err("no key given".into());
    }
    let parts: Vec<&str> = if spec == "+" { vec!["+"] } else { spec.split('+').map(str::trim).collect() };
    let (last, mods_part) = parts.split_last().ok_or("no key given")?;
    let mut mods = Mods::NONE;
    for m in mods_part {
        match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => {
                mods.ctrl = true;
                mods.command = true;
            }
            "shift" => mods.shift = true,
            "alt" | "option" | "opt" => mods.alt = true,
            "cmd" | "command" | "meta" | "super" | "win" => mods.command = true,
            other => return Err(format!("\"{other}\" isn't a modifier: use Ctrl, Shift, Alt or Cmd (Ctrl+S, Shift+Tab)")),
        }
    }
    let name = match last.to_ascii_lowercase().as_str() {
        "enter" | "return" => "Enter".to_string(),
        "esc" | "escape" => "Escape".into(),
        "space" | "spacebar" => " ".into(),
        "tab" => "Tab".into(),
        "backspace" | "bksp" => "Backspace".into(),
        "delete" | "del" => "Delete".into(),
        "insert" | "ins" => "Insert".into(),
        "home" => "Home".into(),
        "end" => "End".into(),
        "pageup" | "pgup" => "PageUp".into(),
        "pagedown" | "pgdn" => "PageDown".into(),
        "up" | "arrowup" => "ArrowUp".into(),
        "down" | "arrowdown" => "ArrowDown".into(),
        "left" | "arrowleft" => "ArrowLeft".into(),
        "right" | "arrowright" => "ArrowRight".into(),
        k if k.len() > 1 && k.starts_with('f') && k[1..].parse::<u8>().is_ok() => k.to_ascii_uppercase(),
        _ => last.to_string(),
    };
    let vk = rapidr_value::input::vk_of_key(&name, "").ok_or_else(|| format!("\"{last}\" isn't a key RapidR knows: use Enter, Tab, Escape, Space, Backspace, Delete, Up, Down, Left, Right, Home, End, PageUp, PageDown, F1 … F12, or one character"))?;
    // (what it types: a character's, shifted; Enter, Tab … type nothing as text)
    let one = name.chars().count() == 1;
    let text = if mods.ctrl || mods.alt || mods.command || !one {
        String::new()
    } else {
        let c = name.chars().next().unwrap_or(' ');
        if mods.shift { c.to_uppercase().collect() } else { c.to_string() }
    };
    // (an upper-case letter or a shifted symbol: Shift held, as the user's)
    if one && mods_part.is_empty() && name.chars().next().is_some_and(|c| c.is_ascii_uppercase() || "!@#$%^&*()_+{}|:\"<>?~".contains(c)) {
        mods.shift = true;
    }
    Ok((vk, text, mods))
}

/// The key steps for typing `text` into `form`.
fn typing(form: &str, text: &str) -> Vec<Step> {
    let mut out = Vec::new();
    for c in text.chars() {
        if c == '\n' {
            out.push(Step::Key { form: form.into(), vk: 13, text: String::new(), mods: Mods::NONE });
            continue;
        }
        if c == '\t' {
            out.push(Step::Key { form: form.into(), vk: 9, text: String::new(), mods: Mods::NONE });
            continue;
        }
        match rapidr_value::input::vk_of_char(c) {
            Some(vk) => {
                let shift = c.is_ascii_uppercase() || "!@#$%^&*()_+{}|:\"<>?~".contains(c);
                out.push(Step::Key { form: form.into(), vk, text: c.to_string(), mods: if shift { Mods::SHIFT } else { Mods::NONE } });
            }
            None => out.push(Step::Commit { form: form.into(), text: c.to_string() }),
        }
    }
    out
}

// --------------------------------------------------------------- the turns --

/// How long the cursor takes from `a` to `b`: a quarter second and a bit
/// more for long ways, at most 0.65 s; nothing with reduced motion, the
/// cursor off or speed 0.
fn glide_ms(a: (f64, f64), b: (f64, f64), speed: f64, show: bool) -> u64 {
    if !show || speed <= 0.0 || rapidr_value::theme::reduced_motion() {
        return 0;
    }
    let d = ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
    ((250.0 + d * 0.6).min(650.0) / speed).round() as u64
}

fn ease(k: f64) -> f64 {
    let k = k.clamp(0.0, 1.0);
    if k < 0.5 { 4.0 * k * k * k } else { 1.0 - (-2.0 * k + 2.0).powi(3) / 2.0 }
}

/// The steps of a job whose target was found; the sentence at its end.
fn plan(desk: &mut Desktop, store: &dyn Store, act: &Act, from: Option<(String, (f64, f64))>, speed: f64, show: bool) -> Result<(Vec<Step>, String), String> {
    let glide = |f: &Found| {
        let start = from.as_ref().filter(|(form, _)| *form == f.form).map_or(f.at, |(_, at)| *at);
        Step::Glide { form: f.form.clone(), to: f.at, ms: glide_ms(start, f.at, speed, show) }
    };
    let check = |desk: &Desktop, f: &Found| -> Result<(), String> {
        if let Some(why) = blocked(desk, &f.form) {
            return Err(why);
        }
        if let Some(c) = &f.comp {
            if desk.forms.get(&f.form).and_then(|w| w.ui.node(c)).is_some_and(|n| !n.enabled) {
                return Err(format!("{} is disabled (Enabled = False): clicking or typing there does nothing now. Do what enables it first.", f.what));
            }
        }
        Ok(())
    };
    let title = |desk: &Desktop, form: &str| desk.forms.get(form).map(|w| w.spec.title.clone()).unwrap_or_default();
    Ok(match act {
        Act::Click { target, button, double } => {
            let f = find(desk, store, target)?;
            check(desk, &f)?;
            let mut s = vec![glide(&f), Step::Move { form: f.form.clone(), at: f.at }, Step::Press { form: f.form.clone(), at: f.at, button: *button, single: true }, Step::Release { form: f.form.clone(), at: f.at, button: *button }];
            if *double {
                s.push(Step::Press { form: f.form.clone(), at: f.at, button: *button, single: false });
                s.push(Step::Release { form: f.form.clone(), at: f.at, button: *button });
            }
            let verb = match (double, button) {
                (true, _) => "Double-clicked",
                (_, Button::Right) => "Right-clicked",
                _ => "Clicked",
            };
            (s, format!("{verb} {} in {} (\"{}\") at {},{}.", f.what, f.form, title(desk, &f.form), f.at.0.round(), f.at.1.round()))
        }
        Act::Hover { target } => {
            let f = find(desk, store, target)?;
            (vec![glide(&f), Step::Move { form: f.form.clone(), at: f.at }, Step::Pause(400)], format!("The mouse is over {} in {}.", f.what, f.form))
        }
        Act::Type { target, text } => {
            if text.is_empty() {
                return Err("No text to type: give Text.".into());
            }
            let mut s = Vec::new();
            let (form, said) = if target.is_empty() {
                let form = input_windows(desk).into_iter().next().ok_or("The program shows no window that takes input now.")?;
                let comp = desk.forms[&form].ui.focused().map(str::to_string);
                (form.clone(), comp.unwrap_or_else(|| format!("{form} (nothing in it has the keyboard)")))
            } else {
                let f = find(desk, store, target)?;
                check(desk, &f)?;
                s.extend([glide(&f), Step::Move { form: f.form.clone(), at: f.at }, Step::Press { form: f.form.clone(), at: f.at, button: Button::Left, single: true }, Step::Release { form: f.form.clone(), at: f.at, button: Button::Left }]);
                if let Some(c) = &f.comp {
                    s.push(Step::Focus { form: f.form.clone(), comp: c.clone() });
                }
                (f.form.clone(), f.what.clone())
            };
            let per = if text.chars().count() > 40 || speed <= 0.0 || !show { 0 } else { (35.0 / speed.max(0.1)) as u64 };
            for k in typing(&form, text) {
                s.push(k);
                if per > 0 {
                    s.push(Step::Pause(per));
                }
            }
            let shown: String = text.chars().take(60).collect();
            (s, format!("Typed \"{shown}{}\" into {said}.", if text.chars().count() > 60 { "…" } else { "" }))
        }
        Act::Key { target, keys } => {
            let mut parsed = Vec::new();
            let chunks: Vec<&str> = if keys.trim() == "+" { vec!["+"] } else { keys.split_whitespace().collect() };
            if chunks.is_empty() {
                return Err("No key given: Keys is Enter, Ctrl+S, Shift+Tab, Escape, F5 … (several separated by spaces).".into());
            }
            for k in chunks {
                parsed.push(parse_key(k)?);
            }
            let mut s = Vec::new();
            let (form, said) = if target.is_empty() {
                let form = input_windows(desk).into_iter().next().ok_or("The program shows no window that takes input now.")?;
                let comp = desk.forms[&form].ui.focused().map(str::to_string);
                (form.clone(), comp.unwrap_or(form))
            } else {
                let f = find(desk, store, target)?;
                check(desk, &f)?;
                if let Some(c) = &f.comp {
                    s.push(Step::Focus { form: f.form.clone(), comp: c.clone() });
                }
                (f.form.clone(), f.what.clone())
            };
            for (vk, text, mods) in parsed {
                s.push(Step::Key { form: form.clone(), vk, text, mods });
                s.push(Step::Pause(30));
            }
            (s, format!("Pressed {} in {said}.", keys.trim()))
        }
        Act::Drag { from: a, to } => {
            let f = find(desk, store, a)?;
            check(desk, &f)?;
            let mut to = to.clone();
            if to.at.is_some() && to.window.is_none() {
                to.window = Some(f.form.clone());
            }
            let g = find(desk, store, &to)?;
            if g.form != f.form {
                return Err(format!("A drag stays in one window: {} is in {}, {} in {}.", f.what, f.form, g.what, g.form));
            }
            let mut s = vec![glide(&f), Step::Move { form: f.form.clone(), at: f.at }, Step::Press { form: f.form.clone(), at: f.at, button: Button::Left, single: true }];
            let n = 8;
            for k in 1..=n {
                let p = k as f64 / n as f64;
                let at = (f.at.0 + (g.at.0 - f.at.0) * p, f.at.1 + (g.at.1 - f.at.1) * p);
                s.push(Step::Glide { form: f.form.clone(), to: at, ms: glide_ms(f.at, g.at, speed, show) / n });
                s.push(Step::Move { form: f.form.clone(), at });
            }
            s.push(Step::Release { form: f.form.clone(), at: g.at, button: Button::Left });
            (s, format!("Dragged from {} to {} in {}.", f.what, g.what, f.form))
        }
        Act::Scroll { target, dx, dy } => {
            let f = find(desk, store, target)?;
            check(desk, &f)?;
            (vec![glide(&f), Step::Move { form: f.form.clone(), at: f.at }, Step::Wheel { form: f.form.clone(), at: f.at, d: (*dx, *dy) }], format!("Turned the wheel {} over {}.", if *dy < 0.0 { format!("{} notches up", -dy) } else { format!("{dy} notches down") }, f.what))
        }
        Act::Wait { ms, idle } => {
            let ms = (*ms).min(60_000);
            if *idle {
                (vec![Step::Idle(if ms == 0 { 5_000 } else { ms })], "Waited until the program had nothing left to run.".into())
            } else {
                (vec![Step::Pause(ms)], format!("Waited {ms} ms."))
            }
        }
    })
}

/// The cursor drawn as it is now (the kernel's [`virtual_cursor`]), the
/// windows it was and is in drawn again.
fn show_cursor(desk: &mut Desktop, h: &mut Hands, now: Instant) {
    let c = &mut h.cursor;
    let reduced = rapidr_value::theme::reduced_motion();
    if let Some((from, to, start, ms)) = c.glide {
        let k = if ms == 0 { 1.0 } else { now.saturating_duration_since(start).as_secs_f64() * 1000.0 / ms as f64 };
        let e = ease(k);
        c.at = (from.0 + (to.0 - from.0) * e, from.1 + (to.1 - from.1) * e);
        if k >= 1.0 {
            c.glide = None;
        }
    }
    let ripple = c.ripple.and_then(|(at, t)| {
        let k = now.saturating_duration_since(t).as_secs_f64() / RIPPLE.as_secs_f64();
        (k < 1.0).then_some((at, if reduced { 0.3 } else { k }))
    });
    if ripple.is_none() {
        c.ripple = None;
    }
    let busy = h.running.is_some() || !h.queue.is_empty();
    let alpha = match c.used {
        _ if busy => 255,
        Some(used) => {
            let since = now.saturating_duration_since(used);
            if since < LINGER {
                255
            } else if reduced || since >= LINGER + FADE {
                0
            } else {
                (255.0 * (1.0 - (since - LINGER).as_secs_f64() / FADE.as_secs_f64())).round() as u8
            }
        }
        None => 0,
    };
    let before = virtual_cursor::get();
    let now_shown = (h.show && alpha > 0 && !c.form.is_empty()).then(|| VirtualCursor { form: c.form.clone(), at: c.at, alpha, ripple, pressed: c.pressed });
    c.shown = now_shown.is_some();
    if before == now_shown {
        return;
    }
    for f in [before.map(|b| b.form), now_shown.as_ref().map(|n| n.form.clone())].into_iter().flatten() {
        if let Some(w) = desk.forms.get_mut(&f) {
            w.ui.dirty = true;
        }
    }
    virtual_cursor::set(now_shown);
    crate::windows::invalidate();
}

/// One step of a job on the desk: false when its window has gone.
fn play(desk: &mut Desktop, store: &dyn Store, h: &mut Hands, step: &Step, now: Instant) -> bool {
    let has = |desk: &Desktop, f: &str| desk.forms.get(f).is_some_and(|w| w.shown);
    match step {
        Step::Glide { form, to, ms } => {
            if !has(desk, form) {
                return false;
            }
            let c = &mut h.cursor;
            let from = if c.form == *form && c.shown { c.at } else {
                // (into a window it wasn't in: from the inside's lower middle)
                let ui = &desk.forms[form].ui;
                if *ms == 0 { *to } else { (ui.client.0 as f64 * 0.5, (ui.client.1 + ui.menu_offset) as f64 * 0.85) }
            };
            c.form = form.clone();
            c.at = from;
            c.glide = Some((from, *to, now, *ms));
            c.used = Some(now);
        }
        Step::Move { form, at } => {
            if !has(desk, form) {
                return false;
            }
            desk.mouse_move(store, form, at.0, at.1, Mods::NONE, Source::Script);
        }
        Step::Press { form, at, button, single } => {
            if !has(desk, form) {
                return false;
            }
            if *single {
                if let Some(f) = desk.form(form) {
                    f.ui.forget_clicks();
                }
            }
            h.cursor.pressed = true;
            h.cursor.ripple = Some((*at, now));
            h.cursor.used = Some(now);
            desk.mouse_down(store, form, *at, *button, Mods::NONE, Source::Script);
        }
        Step::Release { form, at, button } => {
            h.cursor.pressed = false;
            if !has(desk, form) {
                return true;
            }
            desk.mouse_up(store, form, *at, *button, Mods::NONE, Source::Script);
        }
        Step::Focus { form, comp } => {
            if let Some(f) = desk.form(form) {
                f.ui.sync(store);
                if f.ui.focused() != Some(comp.as_str()) {
                    f.ui.focus_id(store, comp);
                }
                f.ui.dirty = true;
            }
        }
        Step::Key { form, vk, text, mods } => {
            if !has(desk, form) {
                return false;
            }
            if let Some(f) = desk.form(form) {
                f.ui.sync(store);
            }
            desk.key_down(store, form, *vk, text, *mods, Source::Script);
            desk.key_up(form, *vk, *mods, Source::Script);
            h.cursor.used = Some(now);
        }
        Step::Commit { form, text } => {
            if !has(desk, form) {
                return false;
            }
            desk.ime_commit(store, form, text, Source::Script);
        }
        Step::Wheel { form, at, d } => {
            if !has(desk, form) {
                return false;
            }
            desk.mouse_wheel(store, form, *at, *d, Mods::NONE, Source::Script);
        }
        Step::Pause(_) | Step::Idle(_) => {}
    }
    true
}

/// The hands' turn (the runtime's loop calls it every turn, as the GUI
/// test script's): a job started (its target found, its steps planned),
/// its due steps played through the kernel and the program's events
/// dispatched, the cursor drawn where it is, and — once a job's last step
/// has had its handlers run — its end kept for [`take_done`].
pub fn step<R: Program + Windows>(rt: R) {
    if rt.in_host_callback() {
        return;
    }
    let now = rt.now();
    // (a job to start)
    let start = hands(|h| if h.running.is_none() { h.queue.pop_front() } else { None });
    if let Some((owner, act)) = start {
        let (speed, show, from) = hands(|h| (h.speed, h.show, (!h.cursor.form.is_empty() && h.cursor.shown).then(|| (h.cursor.form.clone(), h.cursor.at))));
        let mut planned: Option<Result<(Vec<Step>, String), String>> = None;
        let mut windows = Vec::new();
        let ok = rt.with_desk(&mut |desk, store| {
            windows = desk.stacking();
            planned = Some(plan(desk, store, &act, from.clone(), speed, show));
        });
        let planned = if ok { planned.unwrap_or_else(|| Err("The program's windows aren't ready yet: try again in a moment.".into())) } else { Err("The program's windows aren't ready yet (it shows none, or it is busy): try again in a moment.".into()) };
        hands(|h| match planned {
            Ok((steps, said)) => h.running = Some(Running { owner, steps: steps.into(), next: now, said, windows, until: None, quiet: 0, settling: 0 }),
            Err(text) => h.done.push(Done { owner, ok: false, text }),
        });
    }
    // (its steps due now)
    let mut fired = false;
    for _ in 0..64 {
        let due = hands(|h| h.running.as_ref().filter(|r| r.next <= now).and_then(|r| r.steps.front().cloned()));
        let Some(step) = due else { break };
        match &step {
            Step::Pause(ms) => {
                hands(|h| {
                    if let Some(r) = h.running.as_mut() {
                        r.steps.pop_front();
                        r.next = now + Duration::from_millis(*ms);
                    }
                });
                break;
            }
            Step::Idle(ms) => {
                let pending = rt.handlers_pending() || fired;
                let over = hands(|h| {
                    let Some(r) = h.running.as_mut() else { return true };
                    let until = *r.until.get_or_insert(now + Duration::from_millis(*ms));
                    r.quiet = if pending { 0 } else { r.quiet + 1 };
                    let over = r.quiet >= 3 || now >= until;
                    if over {
                        r.steps.pop_front();
                        r.until = None;
                    } else {
                        r.next = now + Duration::from_millis(30);
                    }
                    over
                });
                if !over {
                    break;
                }
                continue;
            }
            _ => {}
        }
        let mut alive = true;
        rt.with_desk(&mut |desk, store| {
            hands(|h| alive = play(desk, store, h, &step, now));
        });
        let wait = hands(|h| {
            let Some(r) = h.running.as_mut() else { return 0 };
            r.steps.pop_front();
            if !alive {
                // (its window went: what came of it so far is said)
                r.steps.clear();
                r.said.push_str(" (Its window closed meanwhile.)");
            }
            match &step {
                Step::Glide { ms, .. } => *ms,
                Step::Press { .. } | Step::Release { .. } => 40,
                _ => 0,
            }
        });
        if matches!(step, Step::Move { .. } | Step::Press { .. } | Step::Release { .. } | Step::Key { .. } | Step::Commit { .. } | Step::Wheel { .. } | Step::Focus { .. }) {
            fired = true;
            // (the program hears it now: its handlers run — at once in a
            // native build, queued for the interpreter's VM after this turn)
            rt.dispatch_pending();
        }
        if wait > 0 {
            hands(|h| {
                if let Some(r) = h.running.as_mut() {
                    r.next = now + Duration::from_millis(wait);
                }
            });
            break;
        }
        // (a step that made the program work: its handlers get their turn
        // before the next)
        if fired {
            hands(|h| {
                if let Some(r) = h.running.as_mut() {
                    r.next = now + Duration::from_millis(16);
                }
            });
            break;
        }
    }
    // (a job whose steps are all played: done once its handlers have run —
    // a turn after its last step, then for as long as the program has
    // handlers waiting, at most 3 s)
    let finished = hands(|h| h.running.as_ref().is_some_and(|r| r.steps.is_empty() && r.next <= now));
    if finished {
        let pending = rt.handlers_pending();
        let end = hands(|h| {
            let r = h.running.as_mut()?;
            if r.settling == 0 {
                r.settling = 1;
                r.next = now + Duration::from_millis(30);
                return None;
            }
            if pending && r.settling < 120 {
                r.settling += 1;
                r.next = now + Duration::from_millis(25);
                return None;
            }
            h.running.take()
        });
        if let Some(r) = end {
            let mut said = r.said;
            rt.with_desk(&mut |desk, _| {
                let after = desk.stacking();
                let opened: Vec<String> = after.iter().filter(|f| !r.windows.contains(f)).map(|f| format!("{f} (\"{}\")", desk.forms.get(f).map(|w| w.spec.title.as_str()).unwrap_or(""))).collect();
                let closed: Vec<&String> = r.windows.iter().filter(|f| !after.contains(f)).collect();
                if !opened.is_empty() {
                    said.push_str(&format!(" A new window opened: {}.", opened.join(", ")));
                }
                if !closed.is_empty() {
                    said.push_str(&format!(" Closed: {}.", closed.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
                }
            });
            hands(|h| {
                h.cursor.used = Some(now);
                h.done.push(Done { owner: r.owner, ok: true, text: said });
            });
        }
    }
    // (the cursor where it is now)
    rt.with_desk(&mut |desk, _| hands(|h| show_cursor(desk, h, now)));
}

/// Everything forgotten (the program's windows are gone, a test's end):
/// the jobs dropped (their owners told), the cursor hidden.
pub fn reset(reason: &str) {
    hands(|h| {
        let mut dropped: Vec<Owner> = h.queue.drain(..).map(|(o, _)| o).collect();
        if let Some(r) = h.running.take() {
            dropped.push(r.owner);
        }
        for owner in dropped {
            h.done.push(Done { owner, ok: false, text: reason.to_string() });
        }
        h.cursor = Cursor::default();
    });
    virtual_cursor::set(None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_parse_as_written() {
        assert_eq!(Target::parse("caption:7"), Target { caption: Some("7".into()), ..Target::default() });
        assert_eq!(Target::parse("Name: btnGo"), Target { name: Some("btnGo".into()), ..Target::default() });
        let any = Target::parse("OK");
        assert_eq!((any.name.as_deref(), any.caption.as_deref(), any.label.as_deref()), (Some("OK"), Some("OK"), Some("OK")));
    }

    #[test]
    fn keys_parse_with_modifiers() {
        let (vk, text, m) = parse_key("Ctrl+S").unwrap();
        assert_eq!((vk, text.as_str(), m.ctrl, m.command, m.shift), (83, "", true, true, false));
        let (vk, _, m) = parse_key("shift+tab").unwrap();
        assert_eq!((vk, m.shift), (9, true));
        assert_eq!(parse_key("Enter").unwrap().0, 13);
        assert_eq!(parse_key("F5").unwrap().0, 116);
        let (vk, text, m) = parse_key("A").unwrap();
        assert_eq!((vk, text.as_str(), m.shift), (65, "A", true));
        assert_eq!(parse_key("a").unwrap().1, "a");
        assert!(parse_key("Hyper+X").unwrap_err().contains("isn't a modifier"));
        assert!(parse_key("Blorp").unwrap_err().contains("isn't a key"));
    }

    #[test]
    fn typing_uses_keys_and_commits_the_rest() {
        let s = typing("f", "aB é\n");
        assert_eq!(s.len(), 5);
        assert!(matches!(&s[1], Step::Key { vk: 66, mods, .. } if mods.shift));
        assert!(matches!(&s[3], Step::Commit { text, .. } if text == "é"));
        assert!(matches!(&s[4], Step::Key { vk: 13, .. }));
    }

    #[test]
    fn glides_are_short_and_still_with_reduced_motion() {
        rapidr_value::theme::set_reduced_motion(false);
        let ms = glide_ms((0.0, 0.0), (300.0, 400.0), 1.0, true);
        assert!((250..=650).contains(&ms), "{ms}");
        assert_eq!(glide_ms((0.0, 0.0), (300.0, 400.0), 1.0, false), 0);
        assert_eq!(glide_ms((0.0, 0.0), (300.0, 400.0), 0.0, true), 0);
        rapidr_value::theme::set_reduced_motion(true);
        assert_eq!(glide_ms((0.0, 0.0), (300.0, 400.0), 1.0, true), 0);
        rapidr_value::theme::set_reduced_motion(false);
        assert!((ease(0.5) - 0.5).abs() < 1e-9 && ease(0.0) == 0.0 && ease(1.0) == 1.0);
    }
}
