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
