//! A GUI test's run (`RAPIDR_CAPTURE`; the environment is [`crate::testhooks`]),
//! played one step per turn of the runtime's loop (moved from runtime-core's
//! `ui/kernel.rs`, docs/desktop-host-plan.md §2.3): after the delay, the
//! splitter drag and the resize, then one event per step — a handler
//! fired, or input through the kernel's routing as the user's
//! ([`ScriptInput`], the host's) — each when the handlers fired so far
//! have run; a moment after the last, the dump, and the host's captures and
//! the end.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;

use rapidr_ui_kernel::components::form::Container;
use rapidr_ui_kernel::tick::Instant;

use crate::testhooks::{self, Action, Capture, TestEvent};
use crate::windows::ScriptInput;
use crate::{forms, Program, Windows};

/// A GUI test's run.
struct Script {
    capture: Capture,
    events: VecDeque<TestEvent>,
    /// When the next step may run.
    next: Instant,
    /// The resize and splitter drag done (they come before the events).
    started: bool,
    /// The events done: the next step captures.
    finished: bool,
}

thread_local! {
    static SCRIPT: RefCell<Option<Script>> = const { RefCell::new(None) };
}

fn sc<R>(f: impl FnOnce(&mut Option<Script>) -> R) -> R {
    SCRIPT.with(|s| f(&mut s.borrow_mut()))
}

/// The test's run starts (the host started under `RAPIDR_CAPTURE`): its
/// first step at `next`.
pub fn start(capture: Capture, next: Instant) {
    sc(|s| *s = Some(Script { events: capture.events.clone().into(), capture, next, started: false, finished: false }));
}

/// When the test's next step may run (`None`: no test).
pub fn next_step() -> Option<Instant> {
    sc(|s| s.as_ref().map(|s| s.next))
}

/// Whether `name` shows: visible up to its form, whose window shows.
pub fn shown_up<P: Program>(p: P, name: &str) -> bool {
    let mut cur = name.to_lowercase();
    for _ in 0..64 {
        if forms::is_form(p, &cur) {
            return forms::form_shown(&cur);
        }
        if !p.flag(&cur, "visible", true) {
            return false;
        }
        let parent = p.get(&cur, "parent").to_string_val();
        if parent.is_empty() {
            return false;
        }
        cur = parent.to_lowercase();
    }
    false
}

fn run_event<R: Program + Windows>(rt: R, e: TestEvent) {
    let comp = e.comp_lower();
    match e.action {
        Action::Fire(ref event) => {
            // (a toggle button's click goes through its group, as its press does)
            if event == "onclick" && forms::is_toggle_button(rt, &comp) {
                forms::toggle_press(rt, &comp);
            }
            rt.fire(&e.comp, event)
        }
        Action::Key(vk) => rt.script_input(ScriptInput::Key { comp, vk }),
        Action::Mouse(kind, x, y) => rt.script_input(ScriptInput::Mouse { comp, kind, x, y }),
        Action::DblClick(x, y) => rt.script_input(ScriptInput::DblClick { comp, x, y }),
        Action::Close => forms::close(rt, &e.comp),
        Action::Ignored => {}
        // (timers during native menu tracking: the next pump held, as a menu
        // the user keeps open would hold it)
        Action::Hold(ms) => rt.script_input(ScriptInput::Hold(ms)),
        // (the lists lane's: the component synthesizes the input)
        Action::Item(_) | Action::Node(_) | Action::Toggle(_) | Action::Cell(..) | Action::Edit | Action::Enter | Action::Escape => {
            let step = match e.action {
                Action::Item(i) => format!("__item_{i}"),
                Action::Node(i) => format!("__node_{i}"),
                Action::Toggle(i) => format!("__toggle_{i}"),
                Action::Cell(c, r) => format!("__cell_{c}_{r}"),
                Action::Edit => "__edit".into(),
                Action::Enter => "__enter".into(),
                _ => "__escape".into(),
            };
            if let Some(form) = rt.form_of(&comp) {
                rt.script_input(ScriptInput::Step { form, comp, step });
            }
        }
    }
}

/// The test's next step, when it's due and the handlers fired so far have
/// run: the resize and splitter drag, then one event per step, then (a
/// moment after the last) the dump, the accessibility trees, the captures,
/// and the end.
pub fn step<R: Program + Windows>(rt: R) {
    let now = rt.now();
    let due = sc(|s| s.as_ref().is_some_and(|sc| sc.next <= now));
    if !due || rt.in_host_callback() {
        return;
    }
    let (started, finished) = sc(|s| s.as_ref().map(|sc| (sc.started, sc.finished))).unwrap_or((true, true));
    if finished {
        capture_and_end(rt);
        return;
    }
    // (busy until this step's handlers have run: a handler's ShowModal steps
    // again, and the next event waits for it)
    sc(|s| {
        if let Some(sc) = s.as_mut() {
            sc.next = now + Duration::from_secs(86_400);
        }
    });
    if !started {
        let (split, resize) = sc(|s| {
            let sc = s.as_mut().expect("a script");
            sc.started = true;
            (sc.capture.split.clone(), sc.capture.resize)
        });
        // (a splitter dragged as the user's press, moves and release; the
        // moves and release do nothing when it has no neighbour to resize)
        if let Some((name, delta)) = split {
            rt.container(Container::SplitBegin(name));
            rt.container(Container::SplitMove(delta / 2));
            rt.container(Container::SplitMove(delta));
            rt.container(Container::SplitEnd);
        }
        if let Some((w, h)) = resize {
            rt.script_input(ScriptInput::Resize { w: i64::from(w), h: i64::from(h) });
            rt.dispatch_pending();
        }
    }
    let next = sc(|s| s.as_mut().and_then(|sc| sc.events.pop_front()));
    match next {
        Some(e) => {
            run_event(rt, e);
            rt.dispatch_pending();
            let next = rt.now() + Duration::from_millis(50);
            sc(|s| {
                if let Some(sc) = s.as_mut() {
                    sc.next = next;
                }
            });
        }
        None => {
            let next = rt.now() + Duration::from_millis(300);
            sc(|s| {
                if let Some(sc) = s.as_mut() {
                    sc.finished = true;
                    sc.next = next;
                }
            })
        }
    }
}

/// `RAPIDR_TEST_DUMP`'s lines, then the host's captures and the end.
fn capture_and_end<R: Program + Windows>(rt: R) {
    testhooks::print_dump(|c| shown_up(rt, c), |comp, prop| rt.get(comp, prop).to_string_val());
    let prefix = sc(|s| s.as_ref().map(|sc| sc.capture.prefix.clone())).unwrap_or_default();
    // (the script is over: a host whose process can't exit — a page —
    // comes back here)
    sc(|s| *s = None);
    rt.capture_and_end(&prefix)
}
