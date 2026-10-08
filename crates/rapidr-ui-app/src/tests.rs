//! The glue against a program and windows in memory (what a runtime
//! implements, at its smallest): a store, the events fired, the window
//! commands left for the host, a clock the test moves.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::time::Duration;

use rapidr_ui_kernel::components::form::Container;
use rapidr_ui_kernel::tick::Instant;
use rapidr_ui_kernel::KernelEvent;
use rapidr_value::layout::Constraints;
use rapidr_value::{v_int, v_str, Value};

use crate::waits::Wait;
use crate::windows::{take_notify, take_ops};
use crate::{dialogs, dispatch, forms, timers, waits, Program, ScriptInput, WindowOp, Windows};

#[derive(Default)]
struct World {
    types: HashMap<String, String>,
    props: HashMap<(String, String), Value>,
    /// (id, event, args) in the order fired.
    fired: Vec<(String, String, Vec<Value>)>,
    /// What a handler does to its arguments (RapidQ's by-reference
    /// parameters: OnClose's Action …).
    answers: HashMap<(String, String), Vec<Value>>,
    flushes: usize,
    containers: Vec<Container>,
    inputs: Vec<ScriptInput>,
    /// What the host queued for the program (a system resize's events).
    events: Vec<KernelEvent>,
    /// Handlers are queued, as for an interpreter (`fire_then`'s
    /// continuations wait in `queued` until the test runs them).
    queue: bool,
    queued: Vec<(Vec<Value>, crate::program::Then)>,
    /// The dialogs' windows the host made (id, title) and closed.
    dialogs: Vec<(String, String)>,
    closed: Vec<String>,
}

thread_local! {
    static WORLD: RefCell<World> = RefCell::new(World::default());
    static NOW: Cell<Option<Instant>> = const { Cell::new(None) };
}

fn world<R>(f: impl FnOnce(&mut World) -> R) -> R {
    WORLD.with(|w| f(&mut w.borrow_mut()))
}

fn make(id: &str, type_name: &str, parent: Option<&str>) {
    world(|w| {
        w.types.insert(id.into(), type_name.into());
        if let Some(p) = parent {
            w.props.insert((id.into(), "parent".into()), v_str(p));
        }
    });
}

fn fired() -> Vec<String> {
    world(|w| w.fired.iter().map(|(id, e, a)| if a.is_empty() { format!("{id}.{e}") } else { format!("{id}.{e}{:?}", a.iter().map(Value::to_i64).collect::<Vec<_>>()) }).collect())
}

fn advance(by: Duration) {
    NOW.with(|n| n.set(Some(n.get().expect("a clock") + by)));
}

#[derive(Clone, Copy)]
struct Mem;

impl Program for Mem {
    fn get(self, id: &str, prop: &str) -> Value {
        world(|w| w.props.get(&(id.to_lowercase(), prop.to_lowercase())).cloned().unwrap_or(Value::Null))
    }
    fn set(self, id: &str, prop: &str, value: Value) {
        self.store(id, prop, value);
    }
    fn store(self, id: &str, prop: &str, value: Value) {
        world(|w| w.props.insert((id.to_lowercase(), prop.to_lowercase()), value));
    }
    fn type_of(self, id: &str) -> String {
        world(|w| w.types.get(&id.to_lowercase()).cloned().unwrap_or_default())
    }
    fn children(self, id: &str) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = world(|w| {
            w.types.iter().filter(|(c, _)| w.props.get(&((*c).clone(), "parent".into())).is_some_and(|p| p.to_string_val() == id)).map(|(c, t)| (c.clone(), t.clone())).collect()
        });
        out.sort();
        out
    }
    fn form_of(self, id: &str) -> Option<String> {
        let mut cur = id.to_lowercase();
        loop {
            if self.type_of(&cur) == "RFORM" {
                return Some(cur);
            }
            cur = self.get(&cur, "parent").to_string_val();
            if cur.is_empty() {
                return None;
            }
        }
    }
    fn flag(self, id: &str, prop: &str, default: bool) -> bool {
        match self.get(id, prop) {
            Value::Null => default,
            v => v.to_bool(),
        }
    }
    fn fire(self, id: &str, event: &str) {
        self.fire_args(id, event, &[]);
    }
    fn fire_args(self, id: &str, event: &str, args: &[Value]) {
        world(|w| w.fired.push((id.into(), event.into(), args.to_vec())));
    }
    fn fire_then(self, id: &str, event: &str, args: &[Value], then: Box<dyn FnOnce(&[Value])>) {
        self.fire_args(id, event, args);
        let answer = world(|w| w.answers.get(&(id.to_string(), event.to_string())).cloned()).unwrap_or_else(|| args.to_vec());
        if world(|w| w.queue) {
            world(|w| w.queued.push((answer, then)));
        } else {
            then(&answer);
        }
    }
    fn has_handler(self, _id: &str, _event: &str) -> bool {
        true
    }
    fn in_host_callback(self) -> bool {
        false
    }
    fn quietly(self, f: &mut dyn FnMut()) {
        f()
    }
    fn constraints(self, form: &str) -> Constraints {
        Constraints::of(|p| self.get(form, p).to_i64())
    }
    fn client_changed(self, _form: &str) {}
    fn container(self, action: Container) {
        world(|w| w.containers.push(action));
    }
    fn now(self) -> Instant {
        NOW.with(|n| n.get()).unwrap_or_else(Instant::now)
    }
}

impl Windows for Mem {
    fn start(self) {}
    fn started(self) -> bool {
        true
    }
    fn flush(self) {
        world(|w| w.flushes += 1);
    }
    fn dispatch_pending(self) {
        for e in world(|w| std::mem::take(&mut w.events)) {
            dispatch::dispatch(Mem, e);
        }
    }
    fn headless(self) -> bool {
        true
    }
    fn forced_scale(self) -> Option<f64> {
        Some(1.0)
    }
    fn window_scale(self, _form: &str) -> Option<f64> {
        Some(1.0)
    }
    fn screen(self) -> (i64, i64) {
        (1920, 1080)
    }
    fn work_area(self) -> (i64, i64) {
        (1920, 1040)
    }
    fn menu_in_window(self) -> bool {
        true
    }
    fn stacking(self) -> Vec<String> {
        forms::shown_forms()
    }
    fn place_of(self, comp: &str) -> Option<(String, (i64, i64))> {
        Some((self.form_of(comp)?, (0, 0)))
    }
    fn system_resized(self, form: &str, (w, h): (i64, i64), (x, y): (i64, i64)) {
        // (what the desktop's kernel queues for a resize and a move)
        world(|w_| w_.events.extend([KernelEvent::Resized(form.into(), w, h), KernelEvent::Moved(form.into(), x, y)]));
    }
    fn open_popup(self, _form: &str, _menu: &str, _x: i64, _y: i64, _program: bool) {}
    fn open_dialog(self, id: &str, title: &str, _size: (i64, i64)) {
        world(|w| w.dialogs.push((id.into(), title.into())));
    }
    fn close_dialog(self, id: &str) {
        world(|w| w.closed.push(id.into()));
    }
    fn ask_files(self, _id: u64, _form: Option<&str>, _req: &crate::file_dialog::Request) {}
    fn files_answer(self, _id: u64) -> Option<Vec<String>> {
        None
    }
    fn script_input(self, input: ScriptInput) {
        world(|w| w.inputs.push(input));
    }
    fn capture_and_end(self, _prefix: &str) {
        panic!("the test's end")
    }
}

/// A form `f` (300 × 200, bsSizeable) with button `b` on it.
fn form_with_button() {
    make("f", "RFORM", None);
    make("b", "RBUTTON", Some("f"));
    for (p, v) in [("width", 300), ("height", 200), ("borderstyle", 2), ("left", 10), ("top", 20)] {
        Mem.store("f", p, v_int(v));
    }
    Mem.store("f", "visible", v_int(0));
}

#[test]
fn a_form_shows_once_built_and_paints_after_onshow() {
    form_with_button();
    forms::show(Mem, "F");
    // (the first Show: OnResize, OnShow, OnResize — RC.EXE)
    assert_eq!(fired(), ["f.onload", "F.onresize", "F.onshow", "F.onresize", "f.onpaint", "b.onpaint"]);
    assert_eq!(take_ops(), [WindowOp::Show("f".into())]);
    assert_eq!(world(|w| w.flushes), 1, "the window exists before OnShow");
    assert_eq!((forms::window_shown("f"), forms::form_scale(Mem, "f")), (Some(true), 1.0));
    // (shown again: on top, nothing fired)
    forms::show(Mem, "f");
    assert_eq!(fired().len(), 6);
    assert_eq!(take_ops(), [WindowOp::Show("f".into())]);
}

#[test]
fn onclose_action_decides() {
    form_with_button();
    forms::show(Mem, "f");
    take_ops();
    // caNone (0): it stays
    world(|w| w.answers.insert(("f".into(), "onclose".into()), vec![v_int(0)]));
    dispatch::dispatch(Mem, KernelEvent::Close("f".into()));
    assert!(forms::form_shown("f"));
    // caMinimize (3)
    world(|w| w.answers.insert(("f".into(), "onclose".into()), vec![v_int(3)]));
    forms::close(Mem, "f");
    assert_eq!(take_ops(), [WindowOp::Minimize("f".into())]);
    // caHide (the default)
    world(|w| w.answers.clear());
    forms::close(Mem, "f");
    assert!(!forms::form_shown("f"));
    assert_eq!(take_ops(), [WindowOp::Hide("f".into())]);
    assert_eq!(fired().iter().filter(|e| e.starts_with("f.onclose")).count(), 3);
}

#[test]
fn keys_go_to_the_form_first_with_keypreview() {
    form_with_button();
    let key = || KernelEvent::KeyDown { chain: vec!["b".into(), "f".into()], vk: 65, shift: 0, text: "a".into() };
    dispatch::dispatch(Mem, key());
    Mem.store("f", "keypreview", v_int(-1));
    dispatch::dispatch(Mem, key());
    assert_eq!(fired(), ["b.onkeydown[65, 0]", "f.onkeydown[65, 0]", "b.onkeydown[65, 0]"]);
}

#[test]
fn a_user_resize_follows_the_constraints() {
    form_with_button();
    forms::show(Mem, "f");
    take_ops();
    Mem.store("f", "maxwidth", v_int(400));
    // (the inside 500 × 300 asked: the frame added, Width held at MaxWidth)
    dispatch::dispatch(Mem, KernelEvent::Resized("f".into(), 500, 300));
    let (fw, fh) = rapidr_value::layout::form_frame(2);
    assert_eq!((Mem.get("f", "width").to_i64(), Mem.get("f", "height").to_i64()), (400, 300 + fh));
    assert_eq!(take_ops(), [WindowOp::Size("f".into(), (400 - fw, 300))]);
    assert_eq!(fired()[6..], ["f.onresize", "f.onpaint"]);
    // (a move is the runtime's, not the program's: no window command back)
    dispatch::dispatch(Mem, KernelEvent::Moved("f".into(), 5, 6));
    assert_eq!((Mem.get("f", "left").to_i64(), Mem.get("f", "top").to_i64()), (5, 6));
    assert!(take_ops().is_empty());
    // (Left set by the program: the window moves)
    forms::move_form(Mem, "f");
    assert_eq!(take_ops(), [WindowOp::Position("f".into(), (5, 6))]);
}

#[test]
fn set_and_container_events() {
    form_with_button();
    take_notify();
    dispatch::dispatch(Mem, KernelEvent::Set { id: "b".into(), prop: "down".into(), value: -1 });
    assert_eq!(Mem.get("b", "down").to_i64(), -1);
    assert_eq!(take_notify(), (true, false));
    dispatch::dispatch(Mem, KernelEvent::Container(Container::SplitEnd));
    dispatch::dispatch(Mem, KernelEvent::Container(Container::Resize { form: "f".into(), w: 1, h: 1 }));
    assert_eq!(world(|w| w.containers.clone()), [Container::SplitEnd], "a size grip's resize is the host's");
}

#[test]
fn timers_fire_when_due_and_rearm_from_their_handler() {
    NOW.with(|n| n.set(Some(Instant::now())));
    make("t", "RTIMER", None);
    Mem.store("t", "interval", v_int(125));
    Mem.store("t", "enabled", v_int(-1));
    timers::register("T");
    timers::start_all(Mem);
    timers::start_all(Mem);
    timers::fire_due(Mem);
    assert!(fired().is_empty());
    advance(Duration::from_millis(125));
    assert_eq!(timers::next_due(), NOW.with(|n| n.get()));
    timers::fire_due(Mem);
    assert_eq!(fired(), ["t.ontimer"], "armed once, however often started");
    // (Interval read again at each tick)
    Mem.store("t", "interval", v_int(250));
    advance(Duration::from_millis(125));
    timers::fire_due(Mem);
    assert_eq!(fired().len(), 2);
    advance(Duration::from_millis(249));
    timers::fire_due(Mem);
    assert_eq!(fired().len(), 2);
    // (disabled meanwhile: it stops; enabled again, it starts over)
    Mem.store("t", "enabled", v_int(0));
    advance(Duration::from_millis(1));
    timers::fire_due(Mem);
    assert_eq!((fired().len(), timers::next_due()), (2, None));
    Mem.store("t", "enabled", v_int(-1));
    timers::changed(Mem, "t");
    advance(Duration::from_millis(250));
    timers::fire_due(Mem);
    assert_eq!(fired().len(), 3);
}

#[test]
fn a_modal_form_is_a_wait_the_vm_serves() {
    form_with_button();
    waits::set_cooperative(true);
    forms::begin_modal(Mem, "f");
    waits::start(Wait::Form("f".into()));
    assert!(waits::take_started() && !waits::take_started());
    assert!(forms::is_modal("F") && forms::modal_forms() == ["f"]);
    assert!(!waits::over());
    // (a button's ModalResult closes it: the runtime hides it)
    Mem.store("f", "modalresult", v_int(1));
    forms::hide_window("f");
    assert!(waits::over());
    assert_eq!(waits::pop(), Some(Wait::Form("f".into())));
    assert_eq!(forms::modal_ended(Mem, "f"), 1);
    assert!(!forms::is_modal("f"));
    // (DOEVENTS: a turn, which the runtime ends)
    let now = Instant::now();
    waits::start(Wait::Once(now));
    assert_eq!(waits::turn(), Some(Wait::Once(now)));
    assert_eq!(waits::pop(), Some(Wait::Once(now)));
    assert_eq!(waits::turn(), None);
}

/// The handlers queued so far run, as an interpreter runs them after the
/// turn that queued them (each continuation with its handler's arguments).
fn run_queued() {
    for (args, then) in world(|w| std::mem::take(&mut w.queued)) {
        then(&args);
    }
}

#[test]
fn a_dialog_is_a_wait_the_vm_serves_and_ends_with_its_answer() {
    form_with_button();
    forms::show(Mem, "f");
    take_ops();
    waits::set_cooperative(true);
    // (MESSAGEDLG's Yes / No: mrYes 6, mrNo 7, Escape mrNo)
    let to_result = |b: Option<usize>| v_int(match b {
        Some(0) => 6,
        _ => 7,
    });
    let dialogs::Pending::Open(outer) = dialogs::message(Mem, "Confirm", "Sure?", &["Yes", "No"], None, false, to_result) else { panic!("a dialog shown") };
    let outer_form = world(|w| w.dialogs.last().cloned()).expect("its window made").0;
    assert!(forms::is_modal(&outer_form), "on the modal list");
    waits::start(Wait::Dialog(outer));
    assert!(waits::take_started());
    assert_eq!(waits::answered(Mem), None, "no answer yet");
    assert!(!waits::over());
    // (a timer's handler, run during it, opens a box of its own over it)
    let dialogs::Pending::Open(inner) = dialogs::message(Mem, "Inner", "Go on?", &["OK", "Cancel"], None, false, |b| v_int(if b == Some(0) { 1 } else { 2 })) else { panic!() };
    let inner_form = world(|w| w.dialogs.last().cloned()).expect("its window").0;
    waits::start(Wait::Dialog(inner));
    assert_eq!(forms::modal_forms(), [outer_form.clone(), inner_form.clone()]);
    // (the outer one's answer comes first: its wait isn't the innermost)
    dialogs::event(Mem, &outer_form, KernelEvent::Click(format!("{outer_form}:b1")));
    assert_eq!(waits::answered(Mem), None);
    // (the inner one answered OK: its wait ends with IDOK, then the outer's
    // with mrNo — each builtin's result, mapped when its wait ends)
    dialogs::event(Mem, &inner_form, KernelEvent::Click(format!("{inner_form}:b0")));
    assert_eq!(waits::answered(Mem), Some(v_int(1)));
    assert_eq!(waits::answered(Mem), Some(v_int(7)));
    assert_eq!(waits::pop(), None);
    assert_eq!(world(|w| w.closed.clone()), [inner_form, outer_form]);
    assert!(forms::modal_forms().is_empty());
    // (a native build waits in place: the same answer through `finished`)
    let dialogs::Pending::Open(native) = dialogs::message(Mem, "Box", "Hi", &["OK"], None, false, |_| v_int(0)) else { panic!() };
    let form = world(|w| w.dialogs.last().cloned()).expect("its window").0;
    assert_eq!(dialogs::finished(native), None);
    dialogs::event(Mem, &form, KernelEvent::KeyDown { chain: vec![form.clone()], vk: 27, shift: 0, text: String::new() });
    assert_eq!(dialogs::finished(native), Some(v_int(0)));
}

#[test]
fn a_timer_fires_once_the_handler_before_it_has_run() {
    NOW.with(|n| n.set(Some(Instant::now())));
    for t in ["t1", "t2"] {
        make(t, "RTIMER", None);
        Mem.store(t, "interval", v_int(125));
        Mem.store(t, "enabled", v_int(-1));
        timers::register(t);
    }
    timers::start_all(Mem);
    advance(Duration::from_millis(125));
    // (an interpreter's handlers are queued: the first timer's ends the
    // round, the second waits for it to have run)
    world(|w| w.queue = true);
    assert!(timers::fire_due(Mem));
    assert_eq!(fired(), ["t1.ontimer"]);
    assert!(timers::held_back() && timers::due_since(NOW.with(|n| n.get()).unwrap()));
    // (the first's handler hasn't run: it isn't armed again, however late)
    advance(Duration::from_millis(500));
    assert!(timers::take_held_back());
    assert!(timers::fire_due(Mem));
    assert_eq!(fired(), ["t1.ontimer", "t2.ontimer"]);
    assert!(!timers::fire_due(Mem), "nothing due: t1's handler hasn't run");
    // (the VM ran them: both armed again an Interval from then)
    run_queued();
    assert!(!timers::fire_due(Mem));
    advance(Duration::from_millis(125));
    assert!(timers::fire_due(Mem));
    assert_eq!(fired().len(), 3);
}

#[test]
fn the_headless_maximize_takes_the_work_area_and_comes_back() {
    form_with_button();
    forms::show(Mem, "f");
    take_ops();
    Mem.store("f", "windowstate", v_int(rapidr_value::window_state::WS_MAXIMIZED));
    forms::set_window_state(Mem, "f", rapidr_value::window_state::WS_NORMAL);
    let ops = take_ops();
    assert_eq!(ops[0], WindowOp::State("f".into(), rapidr_value::window_state::WS_MAXIMIZED));
    let (fw, fh) = rapidr_value::layout::form_frame(2);
    assert_eq!(ops[1], WindowOp::Size("f".into(), (1920 - fw, 1040 - fh)));
    let bounds = || ["left", "top", "width", "height"].map(|p| Mem.get("f", p).to_i64());
    assert_eq!(bounds(), [0, 0, 1920, 1040]);
    assert!(fired().contains(&"f.onresize".to_string()));
    // (restored: the bounds it had)
    Mem.store("f", "windowstate", v_int(rapidr_value::window_state::WS_NORMAL));
    forms::set_window_state(Mem, "f", rapidr_value::window_state::WS_MAXIMIZED);
    assert_eq!(bounds(), [10, 20, 300, 200]);
}

#[test]
fn timer_periods_are_windows_ticks() {
    // (RC.EXE's builds: Interval 0 never fires; 1 fires about every 16 ms)
    assert_eq!(timers::timer_period(0), None);
    assert_eq!(timers::timer_period(-5), None);
    assert_eq!(timers::timer_period(1), Some(Duration::from_micros(15_625)));
    assert_eq!(timers::timer_period(20), Some(Duration::from_micros(31_250)));
    assert_eq!(timers::timer_period(1000), Some(Duration::from_millis(1000)));
}
