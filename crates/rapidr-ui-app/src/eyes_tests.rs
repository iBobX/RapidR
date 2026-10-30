//! The eyes and the hands against a real kernel desktop in memory: a
//! calculator-like form in a `MemStore`, a `Desktop` with its kernel side,
//! a clock the test moves. What the layout report says, what it finds
//! wrong, what a click by caption, name, label or point reaches, typing,
//! keys, a modal window in the way, and that a window the program hasn't
//! got can't be captured.

use std::cell::{Cell, RefCell};
use std::time::Duration;

use rapidr_ui_kernel::components::form::Container;
use rapidr_ui_kernel::input::MemClipboard;
use rapidr_ui_kernel::tick::Instant;
use rapidr_ui_kernel::{KernelEvent, MemStore, Store};
use rapidr_value::layout::Constraints;
use rapidr_value::{v_int, v_str, Value};

use crate::desktop::{Desktop, HostEvent, WindowSpec};
use crate::eyes::{self, Scale};
use crate::hands::{self, Act, Done, Owner, Target};
use crate::{Program, ScriptInput, Windows};

thread_local! {
    static DESK: RefCell<Option<Desktop>> = const { RefCell::new(None) };
    static STORE: RefCell<Option<MemStore>> = const { RefCell::new(None) };
    static NOW: Cell<Option<Instant>> = const { Cell::new(None) };
    static HEARD: RefCell<Vec<KernelEvent>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Copy)]
struct K;

fn store<R>(f: impl FnOnce(&mut MemStore) -> R) -> R {
    STORE.with(|s| f(s.borrow_mut().as_mut().expect("a store")))
}

impl Program for K {
    fn get(self, id: &str, prop: &str) -> Value {
        store(|s| s.get(id, prop))
    }
    fn set(self, id: &str, prop: &str, value: Value) {
        store(|s| {
            s.set(id, prop, value);
        });
    }
    fn store(self, id: &str, prop: &str, value: Value) {
        self.set(id, prop, value);
    }
    fn type_of(self, id: &str) -> String {
        store(|s| s.type_of(id))
    }
    fn children(self, id: &str) -> Vec<(String, String)> {
        store(|s| s.children(id))
    }
    fn form_of(self, _id: &str) -> Option<String> {
        Some("calc".into())
    }
    fn flag(self, id: &str, prop: &str, default: bool) -> bool {
        store(|s| rapidr_ui_kernel::store::flag(s, id, prop, default))
    }
    fn fire(self, _id: &str, _event: &str) {}
    fn fire_args(self, _id: &str, _event: &str, _args: &[Value]) {}
    fn fire_then(self, _id: &str, _event: &str, args: &[Value], then: crate::program::Then) {
        then(args)
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
    fn container(self, _action: Container) {}
    fn now(self) -> Instant {
        NOW.with(|n| n.get().expect("a clock"))
    }
}

impl Windows for K {
    fn start(self) {}
    fn started(self) -> bool {
        true
    }
    fn flush(self) {}
    fn dispatch_pending(self) {
        let events = DESK.with(|d| std::mem::take(&mut d.borrow_mut().as_mut().expect("a desk").events));
        HEARD.with(|h| {
            h.borrow_mut().extend(events.into_iter().filter_map(|e| match e {
                HostEvent::Kernel(_, k) => Some(k),
                HostEvent::Wake => None,
            }))
        });
    }
    fn headless(self) -> bool {
        true
    }
    fn forced_scale(self) -> Option<f64> {
        Some(2.0)
    }
    fn window_scale(self, _form: &str) -> Option<f64> {
        Some(2.0)
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
        DESK.with(|d| d.borrow().as_ref().expect("a desk").stacking())
    }
    fn place_of(self, _comp: &str) -> Option<(String, (i64, i64))> {
        None
    }
    fn system_resized(self, _form: &str, _inside: (i64, i64), _at: (i64, i64)) {}
    fn open_popup(self, _form: &str, _menu: &str, _x: i64, _y: i64, _program: bool) {}
    fn open_dialog(self, _id: &str, _title: &str, _size: (i64, i64)) {}
    fn close_dialog(self, _id: &str) {}
    fn ask_files(self, _id: u64, _form: Option<&str>, _req: &crate::file_dialog::Request) {}
    fn files_answer(self, _id: u64) -> Option<Vec<String>> {
        None
    }
    fn script_input(self, _input: ScriptInput) {}
    fn capture_and_end(self, _prefix: &str) {}
    fn with_desk(self, f: &mut dyn FnMut(&mut Desktop, &dyn Store)) -> bool {
        DESK.with(|d| STORE.with(|s| f(d.borrow_mut().as_mut().expect("a desk"), s.borrow().as_ref().expect("a store"))));
        true
    }
}

/// A calculator-ish form `calc` ("Calculator", 200 × 150 inside): a
/// display, buttons 7 and 8, a label "Name:" with an edit beside it, a
/// button whose caption is too long, one on top of 7, one past the edge.
fn world() {
    let mut s = MemStore::new();
    s.add("calc", "RFORM", None).set("calc", "caption", v_str("Calculator")).set("calc", "clientwidth", v_int(200)).set("calc", "clientheight", v_int(150));
    let place = |s: &mut MemStore, id: &str, ty: &str, (x, y, w, h): (i64, i64, i64, i64), cap: &str| {
        s.add(id, ty, Some("calc")).set(id, "left", v_int(x)).set(id, "top", v_int(y)).set(id, "width", v_int(w)).set(id, "height", v_int(h));
        if !cap.is_empty() {
            s.set(id, "caption", v_str(cap));
        }
    };
    place(&mut s, "display", "REDIT", (10, 8, 180, 24), "");
    place(&mut s, "b7", "RBUTTON", (10, 40, 40, 30), "&7");
    place(&mut s, "b8", "RBUTTON", (60, 40, 40, 30), "8");
    place(&mut s, "lblname", "RLABEL", (10, 82, 40, 16), "Name:");
    place(&mut s, "edname", "REDIT", (60, 80, 100, 22), "");
    place(&mut s, "blong", "RBUTTON", (10, 110, 50, 24), "Save all the files now");
    place(&mut s, "bover", "RBUTTON", (110, 40, 40, 30), "Over");
    place(&mut s, "bunder", "RBUTTON", (120, 50, 40, 30), "Under");
    place(&mut s, "boff", "RBUTTON", (170, 120, 60, 24), "Off");
    // (a word-wrapped label sized by the program, too low for its lines)
    place(&mut s, "note", "RLABEL", (110, 82, 50, 14), "A note far too long for one line");
    s.set("note", "autosize", v_int(0)).set("note", "wordwrap", v_int(1));
    // (a panel too narrow for its button: the panel cuts it off)
    place(&mut s, "pnl", "RPANEL", (160, 80, 30, 30), "");
    s.add("bcut", "RBUTTON", Some("pnl")).set("bcut", "left", v_int(4)).set("bcut", "top", v_int(4)).set("bcut", "width", v_int(32)).set("bcut", "height", v_int(20));
    s.set("calc", "visible", v_int(1));
    let mut desk = Desktop::new(Box::new(MemClipboard::default()));
    desk.default_scale = 2.0;
    let spec = WindowSpec { title: "Calculator".into(), size: (200, 150), position: Some((100, 80)), border: true, ..WindowSpec::default() };
    desk.ensure_form(&s, "calc", true, spec);
    desk.show("calc");
    STORE.with(|st| *st.borrow_mut() = Some(s));
    DESK.with(|d| *d.borrow_mut() = Some(desk));
    NOW.with(|n| n.set(Some(rapidr_ui_kernel::tick::now())));
    HEARD.with(|h| h.borrow_mut().clear());
    hands::reset("a new test");
    hands::take_done();
    hands::set_speed(1.0);
    hands::set_cursor_shown(true);
}

fn desk<R>(f: impl FnOnce(&mut Desktop, &MemStore) -> R) -> R {
    DESK.with(|d| STORE.with(|s| f(d.borrow_mut().as_mut().expect("a desk"), s.borrow().as_ref().expect("a store"))))
}

/// Runs the hands' turns, the clock moving a frame at a time, until a job
/// ends (at most 10 s of the test's time).
fn run_until_done() -> Done {
    for _ in 0..700 {
        hands::step(K);
        if let Some(d) = hands::take_done().pop() {
            return d;
        }
        NOW.with(|n| n.set(n.get().map(|t| t + Duration::from_millis(16))));
    }
    panic!("the job never ended");
}

fn heard() -> Vec<KernelEvent> {
    HEARD.with(|h| std::mem::take(&mut *h.borrow_mut()))
}

#[test]
fn the_layout_report_lists_what_shows_and_what_looks_wrong() {
    world();
    let report = desk(|d, s| eyes::layout(d, s, None));
    let w = &report["windows"][0];
    assert_eq!((w["name"].as_str(), w["title"].as_str(), w["front"].as_bool()), (Some("calc"), Some("Calculator"), Some(true)));
    let comps = w["components"].as_array().unwrap();
    let b7 = comps.iter().find(|c| c["name"] == "b7").unwrap();
    assert_eq!((b7["type"].as_str(), b7["caption"].as_str(), b7["x"].as_i64(), b7["width"].as_i64()), (Some("RButton"), Some("7"), Some(10), Some(40)));
    let problems: Vec<String> = w["problems"].as_array().unwrap().iter().map(|p| p.as_str().unwrap().to_string()).collect();
    let all = problems.join("\n");
    assert!(all.contains("blong's caption \"Save all the files now\" is cut off"), "{all}");
    assert!(all.contains("bover and bunder are on top of each other"), "{all}");
    assert!(all.contains("boff (RButton) is cut off by the window's edge"), "{all}");
    assert!(all.contains("note's text is cut off: wrapped it takes"), "{all}");
    assert!(all.contains("bcut (RButton) is cut off by pnl's edge (it reaches 196,104; pnl ends at 190,110)"), "{all}");
    // (fine ones aren't judged)
    assert!(!all.contains("b7") && !all.contains("edname"), "{all}");
    let text = eyes::layout_text(&report);
    if std::env::var_os("RAPIDR_SHOW_LAYOUT").is_some() {
        eprintln!("{text}");
    }
    assert!(text.starts_with("Window calc \"Calculator\" (RForm), inside 200 × 150, in front"), "{text}");
    assert!(text.contains("  b7 (RButton) \"7\" at 10,40 size 40×30"), "{text}");
    assert!(text.contains("Problem: "), "{text}");
}

#[test]
fn pictures_are_the_programs_own_windows_only() {
    world();
    let (inside, framed, at1) = desk(|d, s| (eyes::capture(d, s, "calc", Scale::Screen, false, false).unwrap(), eyes::capture(d, s, "calc", Scale::Screen, true, false).unwrap(), eyes::capture(d, s, "CALC", Scale::Times(1.0), false, false).unwrap()));
    assert_eq!((inside.width, inside.height), (400, 300), "the screen's scale (2×)");
    assert_eq!((at1.width, at1.height), (200, 150), "1×");
    assert!(framed.width > inside.width && framed.height > inside.height + 40, "with its title bar: {}×{}", framed.width, framed.height);
    assert!(eyes::png(&at1).unwrap().starts_with(b"\x89PNG"));
    // (a window the program hasn't got: refused, the program's listed —
    // there is no way to ask for anything else: no screen, no other app)
    for other in ["Finder", "RapidR Studio", "desktop", "screen"] {
        let e = desk(|d, _| eyes::find_window(d, other)).unwrap_err();
        assert!(e.contains(&format!("No window named \"{other}\"")) && e.contains("calc (\"Calculator\")"), "{e}");
        assert!(desk(|d, s| eyes::capture(d, s, other, Scale::Screen, true, false)).is_err());
    }
    // (all of them: here one window, framed)
    let all = desk(|d, s| eyes::capture_all(d, s, Scale::Times(1.0), true, false)).unwrap();
    assert!(all.width >= 200 && all.height >= 150);
}

#[test]
fn a_click_by_caption_glides_and_reaches_the_button() {
    world();
    hands::submit(Owner::Link(7), Act::Click { target: Target { caption: Some("7".into()), ..Target::default() }, button: rapidr_value::input::Button::Left, double: false });
    // (gliding: the cursor shows in the window, between its start and the button)
    for _ in 0..6 {
        hands::step(K);
        NOW.with(|n| n.set(n.get().map(|t| t + Duration::from_millis(16))));
    }
    let mid = rapidr_ui_kernel::virtual_cursor::get().expect("the cursor shows while it glides");
    assert_eq!(mid.form, "calc");
    let d = run_until_done();
    assert!(d.ok && d.owner == Owner::Link(7), "{d:?}");
    assert!(d.text.starts_with("Clicked b7 (RButton \"7\") in calc (\"Calculator\") at 30,55."), "{}", d.text);
    let events = heard();
    assert!(events.contains(&KernelEvent::Click("b7".into())), "{events:?}");
    let c = rapidr_ui_kernel::virtual_cursor::get().expect("it stays a moment");
    assert_eq!(c.at, (30.0, 55.0));
}

#[test]
fn by_name_label_and_point_and_what_isnt_there() {
    world();
    let click = |t: Target| Act::Click { target: t, button: rapidr_value::input::Button::Left, double: false };
    hands::submit(Owner::Link(1), click(Target { name: Some("B8".into()), ..Target::default() }));
    assert!(run_until_done().ok);
    assert!(heard().contains(&KernelEvent::Click("b8".into())));
    // (a point: 7's middle)
    hands::submit(Owner::Link(2), click(Target { at: Some((25.0, 50.0)), ..Target::default() }));
    let d = run_until_done();
    assert!(d.ok && d.text.contains("25,50 of calc (b7 (RButton \"7\"))"), "{}", d.text);
    // (typing beside a label)
    hands::submit(Owner::Link(3), Act::Type { target: Target { label: Some("Name".into()), ..Target::default() }, text: "Ada".into() });
    let d = run_until_done();
    assert!(d.ok && d.text.contains("Typed \"Ada\" into edname (REdit) (beside the label \"Name:\")"), "{}", d.text);
    let typed: String = heard().iter().filter_map(|e| if let KernelEvent::KeyDown { text, .. } = e { Some(text.clone()) } else { None }).collect();
    assert_eq!(typed, "Ada");
    // (what isn't there: said with what there is)
    hands::submit(Owner::Link(4), click(Target { caption: Some("Go".into()), ..Target::default() }));
    let d = run_until_done();
    assert!(!d.ok && d.text.starts_with("No button (or other control) shows \"Go\" in calc. Buttons now: \"7\", \"8\""), "{}", d.text);
    assert!(d.text.contains("Labels: \"Name:\""), "{}", d.text);
    // (keys into what has the keyboard)
    hands::submit(Owner::Link(5), Act::Key { target: Target::default(), keys: "Ctrl+A Backspace".into() });
    let d = run_until_done();
    assert!(d.ok && d.text == "Pressed Ctrl+A Backspace in edname.", "{}", d.text);
    let keys: Vec<(i64, i64)> = heard().iter().filter_map(|e| if let KernelEvent::KeyDown { vk, shift, .. } = e { Some((*vk, *shift)) } else { None }).collect();
    assert_eq!(keys, vec![(65, 16), (8, 0)]);
}

#[test]
fn a_disabled_button_or_a_modal_window_in_the_way_says_so() {
    world();
    store(|s| {
        s.set("b8", "enabled", v_int(0));
    });
    hands::submit(Owner::Link(1), Act::Click { target: Target { caption: Some("8".into()), ..Target::default() }, button: rapidr_value::input::Button::Left, double: false });
    let d = run_until_done();
    assert!(!d.ok && d.text.contains("is disabled"), "{}", d.text);
    // (a modal window over it)
    store(|s| {
        s.add("ask", "RFORM", None).set("ask", "caption", v_str("Sure?")).set("ask", "clientwidth", v_int(120)).set("ask", "clientheight", v_int(60));
    });
    desk(|d, s| {
        d.ensure_form(s, "ask", true, WindowSpec { title: "Sure?".into(), size: (120, 60), border: true, ..WindowSpec::default() });
        d.show("ask");
        d.modal = vec!["ask".into()];
    });
    hands::submit(Owner::Link(2), Act::Click { target: Target { caption: Some("7".into()), ..Target::default() }, button: rapidr_value::input::Button::Left, double: false });
    let d = run_until_done();
    assert!(!d.ok && d.text.starts_with("ask (\"Sure?\") is open in front and waits for an answer"), "{}", d.text);
}

#[test]
fn reduced_motion_jumps_and_a_hidden_cursor_draws_nothing() {
    world();
    rapidr_value::theme::set_reduced_motion(true);
    hands::submit(Owner::Link(1), Act::Click { target: Target { caption: Some("7".into()), ..Target::default() }, button: rapidr_value::input::Button::Left, double: false });
    hands::step(K);
    // (no glide: at the button at once)
    let c = rapidr_ui_kernel::virtual_cursor::get().expect("shown");
    assert_eq!(c.at, (30.0, 55.0));
    assert!(run_until_done().ok);
    rapidr_value::theme::set_reduced_motion(false);
    hands::set_cursor_shown(false);
    hands::reset("again");
    hands::take_done();
    hands::submit(Owner::Link(2), Act::Click { target: Target { caption: Some("8".into()), ..Target::default() }, button: rapidr_value::input::Button::Left, double: false });
    assert!(run_until_done().ok);
    assert!(rapidr_ui_kernel::virtual_cursor::get().is_none());
    hands::set_cursor_shown(true);
}
