//! The kernel's input routing and accessibility description, without a
//! window or a GPU.

use accesskit::Role;

use crate::demo::{self, NoClipboard};
use crate::form::{Event, Form, Key, Kind, Mods};
use crate::text::TextSystem;
use crate::{a11y, render};

fn setup() -> (Form, TextSystem) {
    let mut ts = TextSystem::new();
    let mut f = demo::form();
    drop(render::scene(&mut f, &mut ts, 1.0));
    (f, ts)
}

fn name(f: &Form) -> &str {
    &f.components[f.focus.unwrap()].name
}

#[test]
fn tab_order_and_focus() {
    let (mut f, mut ts) = setup();
    let mut clip = NoClipboard(None);
    assert_eq!(name(&f), "edName");
    let mut seen = Vec::new();
    for _ in 0..4 {
        f.key(Key::Tab, Mods::default(), None, &mut ts, &mut clip);
        seen.push(name(&f).to_string());
    }
    assert_eq!(seen, ["btnOK", "tbLevel", "tcPages", "edName"], "labels are skipped, creation order");
    f.key(Key::Tab, Mods { shift: true, ..Mods::default() }, None, &mut ts, &mut clip);
    assert_eq!(name(&f), "tcPages");
}

#[test]
fn button_clicks_by_mouse_and_keyboard() {
    let (mut f, mut ts) = setup();
    let mut clip = NoClipboard(None);
    let ok = f.find("btnOK").unwrap();
    assert!(f.mouse_down(223.0, 20.0, Mods::default(), &mut ts).is_empty());
    assert_eq!(f.focus, Some(ok), "a button takes the focus when pressed");
    assert_eq!(f.mouse_up(223.0, 20.0), vec![Event::Click(ok)]);
    // let go elsewhere: no click
    f.mouse_down(223.0, 20.0, Mods::default(), &mut ts);
    assert!(f.mouse_up(10.0, 200.0).is_empty());
    assert_eq!(f.key(Key::Space, Mods::default(), Some(" "), &mut ts, &mut clip), vec![Event::Click(ok)]);
}

#[test]
fn track_bar_and_tab_control_models_get_the_input() {
    let (mut f, mut ts) = setup();
    let mut clip = NoClipboard(None);
    let tb = f.find("tbLevel").unwrap();
    f.mouse_down(58.0, 58.0, Mods::default(), &mut ts);
    let (_, ev) = f.mouse_move(108.0, 58.0, &mut ts);
    f.mouse_up(108.0, 58.0);
    assert_eq!(ev, vec![Event::Change(tb)]);
    let Kind::TrackBar(t) = &f.components[tb].kind else { panic!() };
    assert_eq!(t.position, 7);
    assert_eq!(f.key(Key::Right, Mods::default(), None, &mut ts, &mut clip), vec![Event::Change(tb)]);
    let tc = f.find("tcPages").unwrap();
    assert_eq!(f.mouse_down(220.0, 55.0, Mods::default(), &mut ts), vec![Event::Change(tc)]);
    let Kind::TabControl(t) = &f.components[tc].kind else { panic!() };
    assert_eq!(t.index, 1);
}

#[test]
fn edit_typing_selection_and_clipboard_reach_the_model() {
    let (mut f, mut ts) = setup();
    let mut clip = NoClipboard(None);
    let ed = f.find("edName").unwrap();
    let none = Mods::default();
    let shift = Mods { shift: true, ..none };
    let cmd = Mods { command: true, ..none };
    f.key(Key::End, none, None, &mut ts, &mut clip);
    assert_eq!(f.key(Key::Char('é'), none, Some("é"), &mut ts, &mut clip), vec![Event::Change(ed)]);
    f.key(Key::Left, shift, None, &mut ts, &mut clip);
    f.key(Key::Left, shift, None, &mut ts, &mut clip);
    let model = |f: &Form| match &f.components[ed].kind {
        Kind::Edit(e) => (e.model.text(), e.model.sel_start, e.model.sel_len, e.model.modified),
        _ => panic!(),
    };
    assert_eq!(model(&f), ("Grüße, ñandú ✓é".to_string(), 13, 2, true), "SelStart / SelLength in characters");
    f.key(Key::Char('x'), cmd, None, &mut ts, &mut clip);
    assert_eq!(clip.0.as_deref(), Some("✓é"));
    f.key(Key::Home, none, None, &mut ts, &mut clip);
    f.key(Key::Char('v'), cmd, None, &mut ts, &mut clip);
    assert_eq!(model(&f).0, "✓éGrüße, ñandú ");
    // Ime commits (dead keys, compositions) go in too
    f.commit_text("ß", &mut ts);
    assert_eq!(model(&f).0, "✓éßGrüße, ñandú ");
}

#[test]
fn accessibility_tree_describes_and_operates_the_form() {
    let (mut f, _) = setup();
    let tree = a11y::tree(&f, 2.0);
    let roles: Vec<Role> = tree.nodes.iter().map(|(_, n)| n.role()).collect();
    for r in [Role::Window, Role::Label, Role::Button, Role::TextInput, Role::Slider, Role::TabList, Role::Tab] {
        assert!(roles.contains(&r), "{r:?}");
    }
    let button = tree.nodes.iter().find(|(_, n)| n.role() == Role::Button).unwrap();
    assert_eq!(button.1.label(), Some("OK"));
    let edit = tree.nodes.iter().find(|(_, n)| n.role() == Role::TextInput).unwrap();
    assert_eq!(edit.1.value(), Some("Grüße, ñandú ✓"));
    assert_eq!(tree.focus, edit.0);
    // VoiceOver pressing the button: OnClick
    let ok = f.find("btnOK").unwrap();
    assert_eq!(a11y::apply(&mut f, a11y::Request::Click(ok)), vec![Event::Click(ok)]);
}

/// Spike A on the headless host (spike C): a ShowModal whose button
/// handler shows another form modally; the nested ShowModal returns its
/// ModalResult to the handler, the outer one to "main"; a timer ticks
/// while the nested modal is up; a click on a form under a modal is
/// dropped. All through `ui::step`, with no window and no event loop.
#[test]
fn nested_show_modal_and_timer_on_the_headless_host() {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use crate::form::Component;
    use crate::{host, script, ui};

    let s = script::Script::parse("wait 100\nclick f1 btn\nclick f2 btnNested\nwait 100\nclick f3 btnOK\nwait 20\nclick f2 btnDone\n").unwrap();
    ui::init(Box::new(host::HeadlessHost::new(1.0)), Some(s), None);
    let mk = |caption: &str, buttons: &[&str]| {
        let mut f = Form::new(caption);
        for (k, b) in buttons.iter().enumerate() {
            f.add(Component::new(b, "RBUTTON", 8 + 80 * k as i64, 8, Kind::Button { caption: b.to_string() }));
        }
        f
    };
    let f1 = ui::create_form("f1", mk("one", &["btn"]));
    let f2 = ui::create_form("f2", mk("two", &["btnNested", "btnDone"]));
    let f3 = ui::create_form("f3", mk("three", &["btnOK"]));
    ui::set_button_result(f2, "btnDone", 1);
    ui::set_button_result(f3, "btnOK", 7);
    let ticks = Rc::new(Cell::new(0));
    let t = ticks.clone();
    ui::add_timer(Duration::from_millis(10), move || t.set(t.get() + 1));
    let f1_clicks = Rc::new(Cell::new(0));
    let c = f1_clicks.clone();
    ui::on(f1, "btn", "click", move || c.set(c.get() + 1));
    let nested = Rc::new(Cell::new((0, 0)));
    let (n, t) = (nested.clone(), ticks.clone());
    ui::on(f2, "btnNested", "click", move || {
        let before = t.get();
        let r = ui::show_modal(f3);
        n.set((r, t.get() - before));
    });
    ui::show(f1);
    assert_eq!(ui::show_modal(f2), 1);
    let (r3, during) = nested.get();
    assert_eq!(r3, 7, "the nested ShowModal's result reaches its handler");
    assert!(during > 0, "the timer ticks while the nested modal is up");
    assert_eq!(f1_clicks.get(), 0, "input to a form under a modal one is dropped");
    assert!(!ui::shown(f2) && !ui::shown(f3) && ui::shown(f1));
}

/// Spike B: the CPU renderer is deterministic and sized like the GPU one.
#[test]
fn cpu_capture_is_deterministic() {
    let (mut f, mut ts) = setup();
    let a = crate::cpu::capture(&mut f, &mut ts, 2.0);
    let b = crate::cpu::capture(&mut f, &mut ts, 2.0);
    let (w, h) = render::device_size(&f, 2.0);
    assert_eq!((a.width, a.height), (w as usize, h as usize));
    assert_eq!(a.pixels, b.pixels);
    assert!(a.pixels.iter().any(|&p| p != a.pixels[0]), "something was drawn");
}
