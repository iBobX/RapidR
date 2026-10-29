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
    /// Since when an `__until_` step has waited, and whether its value was
    /// there at the last look (it must be at two looks running: the
    /// handlers the change queued run between them, and may change it back).
    waiting: Option<Instant>,
    seen: bool,
    /// An event's handlers are running (the next waits for them). A step
    /// that comes meanwhile comes from inside them: a native build's
    /// handler waiting for the user (ShowModal, a message box, INPUT$ …),
    /// whose loop steps — the next events go on then, to what it opened.
    running: bool,
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
    sc(|s| *s = Some(Script { events: capture.events.clone().into(), capture, next, started: false, finished: false, waiting: None, seen: false, running: false }));
}

/// When the test's next step may run, or the next keystroke SendKeys
/// queued is delivered (`None`: neither). Asked while an event's handlers
/// run — a native build's handler waiting for the user, its loop turning —:
/// now, so that turn comes to [`step`], which lets the next events go on.
pub fn next_step() -> Option<Instant> {
    let keys = (rapidr_value::send_keys::pending() > 0).then(rapidr_ui_kernel::tick::now);
    let next = sc(|s| s.as_ref().map(|s| if s.running { rapidr_ui_kernel::tick::now() } else { s.next }));
    [next, keys].into_iter().flatten().min()
}

/// SendKeys' next keystroke delivered (one per turn, each after the
/// handlers the one before fired have run); whether there was one.
fn send_key<R: Program + Windows>(rt: R) -> bool {
    if rt.in_host_callback() {
        return false;
    }
    let Some(k) = rapidr_value::send_keys::next() else { return false };
    rt.script_input(ScriptInput::Stroke(k));
    rt.dispatch_pending();
    true
}

/// A handler waits for the user's input in place — a native build's
/// StartDrag, moving its control until the mouse is let go: the script's
/// next steps (that input) come meanwhile, as the user's would, rather
/// than after the handler.
pub fn input_awaited(now: Instant) {
    sc(|s| {
        if let Some(sc) = s.as_mut() {
            sc.next = sc.next.min(now + Duration::from_millis(50));
        }
    });
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
        Action::Key(vk) => rt.script_input(ScriptInput::Key { comp, vk, state: 0 }),
        Action::KeyWith(vk, state) => rt.script_input(ScriptInput::Key { comp, vk, state }),
        Action::Mouse(kind, x, y) => rt.script_input(ScriptInput::Mouse { comp, kind, x, y }),
        Action::DblClick(x, y) => rt.script_input(ScriptInput::DblClick { comp, x, y }),
        Action::RightClick(x, y) => rt.script_input(ScriptInput::RightClick { comp, x, y }),
        Action::Close => forms::close(rt, &e.comp),
        // (RapidR's OnDropFiles: RAPIDR_TEST_DROP's files on the form)
        Action::Drop => forms::files_dropped(rt, &rt.form_of(&comp).unwrap_or(comp), &testhooks::drop_files()),
        Action::Ignored => {}
        // (timers during native menu tracking: the next pump held, as a menu
        // the user keeps open would hold it; the next event ms later: `step`)
        Action::Hold(ms) => rt.script_input(ScriptInput::Hold(ms)),
        // (waited for in `step`)
        Action::Wait(_) => {}
        // (the system tray: the form's icon clicked)
        Action::Tray(m) => crate::tray::test_message(rt, &e.comp, m),
        // (RapidR's OnThemeChange: the user switched the system to dark …
        // — a theme that follows it changes now, Application.OnThemeChange
        // fires as the events are dispatched)
        Action::Look(dark, contrast) => {
            testhooks::set_system_look(dark, contrast);
            rapidr_value::theme::system_answer(dark, contrast);
            crate::windows::invalidate();
        }
        // (the lists lane's: the component synthesizes the input)
        // (waited for in `step`)
        Action::Until(..) => {}
        Action::Item(_) | Action::Node(_) | Action::Toggle(_) | Action::Cell(..) | Action::Edit | Action::Enter | Action::Escape | Action::Step(_) => {
            let step = match e.action {
                Action::Step(ref s) => s.clone(),
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
    // (Application.SendKeys' keystrokes go first: a test's next event waits
    // for them)
    if send_key(rt) {
        return;
    }
    let now = rt.now();
    // (a step from inside an event's handlers: a native build's handler
    // waits for the user — its ShowModal, message box, INPUT$ — and its loop
    // steps. The user would answer it now: the next events come, to it,
    // as the interpreter's do once its handler has left the wait to the VM)
    sc(|s| {
        if let Some(sc) = s.as_mut().filter(|sc| sc.running) {
            sc.running = false;
            sc.next = now + Duration::from_millis(50);
        }
    });
    let due = sc(|s| s.as_ref().is_some_and(|sc| sc.next <= now));
    // (nor while the program waits for work in the background — a
    // QDOWNLOAD's transfer, waited for as the interpreter waits for a
    // dialog: a native build's handler is still running then, which the
    // script waits for anyway)
    if !due || rt.in_host_callback() || crate::dialogs::tasks_open() {
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
    // (`__until_`: the next step waits until the component's property reads
    // that value — work in the background done —, looked at every 50 ms)
    if let Some(TestEvent { comp, action: Action::Until(prop, value) }) = &next {
        let now = rt.now();
        let since = sc(|s| s.as_mut().map(|sc| *sc.waiting.get_or_insert(now))).unwrap_or(now);
        let got = rt.get(comp, prop).to_string_val();
        let late = now.saturating_duration_since(since) > Duration::from_secs(120);
        if late {
            eprintln!("[test] {comp}.{prop} never read {value} (it reads {got}): going on");
        }
        sc(|s| {
            if let Some(sc) = s.as_mut() {
                let there = got == *value;
                if late || (there && sc.seen) {
                    sc.waiting = None;
                    sc.seen = false;
                } else {
                    sc.seen = there;
                    sc.events.push_front(next.clone().expect("the step"));
                }
                sc.next = now + Duration::from_millis(50);
            }
        });
        return;
    }
    // (`__wait_N`: the program runs on its own for N ms — its timers, its
    // servers —, then the next event)
    if let Some(TestEvent { action: Action::Wait(ms), .. }) = &next {
        let at = now + Duration::from_millis(ms.max(&0).unsigned_abs());
        sc(|s| {
            if let Some(sc) = s.as_mut() {
                sc.next = at;
            }
        });
        return;
    }
    // (`__hold_ms`: the next event ms later, whether the host held its pump
    // — the headless one does — or can't: real windows, the web)
    let after = match &next {
        Some(TestEvent { action: Action::Hold(ms), .. }) => Duration::from_millis(ms.max(&0).unsigned_abs()).max(Duration::from_millis(50)),
        _ => Duration::from_millis(50),
    };
    match next {
        Some(e) => {
            // (the next step soon after, however this one's handlers end — a
            // native build's run-time error unwinds them: rapidr_value::catching)
            struct Next<R: Program>(R, Instant, Duration);
            impl<R: Program> Drop for Next<R> {
                fn drop(&mut self) {
                    let next = (self.0.now() + Duration::from_millis(50)).max(self.1 + self.2);
                    sc(|s| {
                        if let Some(sc) = s.as_mut() {
                            sc.next = next;
                            sc.running = false;
                        }
                    });
                }
            }
            sc(|s| {
                if let Some(sc) = s.as_mut() {
                    sc.running = true;
                }
            });
            let _next = Next(rt, now, after);
            run_event(rt, e);
            rt.dispatch_pending();
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

/// What `RAPIDR_TEST_DUMP`'s `comp.prop` prints: the property as the program
/// reads it; `__cursor_x_y`, the mouse pointer at (x, y) of the component
/// (the mouse moved there as the user's; the CSS name: `col-resize`,
/// `text`, `default` …).
pub fn dump_value<R: Program + Windows>(rt: R, comp: &str, prop: &str) -> String {
    // (`__popupat`: where the pop-up menu open on the component's window is,
    // from the component's top left — a right click at (x, y) of it must
    // open the menu at "x,y"; "none" when none is open)
    if prop.eq_ignore_ascii_case("__popupat") {
        return rt
            .place_of(&comp.to_lowercase())
            .and_then(|(form, (ox, oy))| rt.popup_rect(&form).map(|(x, y, _, _)| format!("{},{}", x - ox, y - oy)))
            .unwrap_or_else(|| "none".into());
    }
    let at = prop.strip_prefix("__cursor_").and_then(|n| n.split_once('_')).and_then(|(x, y)| Some((x.parse::<i64>().ok()?, y.parse::<i64>().ok()?)));
    match at {
        Some((x, y)) => {
            rt.script_input(ScriptInput::Cursor { comp: comp.to_lowercase(), x, y });
            crate::desktop::take_probe().to_string()
        }
        None => rt.get(comp, prop).to_string_val(),
    }
}

/// `RAPIDR_TEST_DUMP`'s lines, then the host's captures and the end.
fn capture_and_end<R: Program + Windows>(rt: R) {
    testhooks::print_dump(|c| shown_up(rt, c), |comp, prop| dump_value(rt, comp, prop));
    let prefix = sc(|s| s.as_ref().map(|sc| sc.capture.prefix.clone())).unwrap_or_default();
    // (the script is over: a host whose process can't exit — a page —
    // comes back here)
    sc(|s| *s = None);
    rt.capture_and_end(&prefix)
}
