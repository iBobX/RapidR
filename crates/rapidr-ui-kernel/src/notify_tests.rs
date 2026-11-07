//! RNOTIFY and RNOTIFYBAR in the kernel (lane NOTIFY): a transparent
//! overlay that takes the mouse only where a toast is, toasts as groups
//! with their buttons for a screen reader, a strip that docks its buttons.

use rapidr_value::input::Button;
use rapidr_value::objects::a11y::{AccessNode, Role};
use rapidr_value::panels::{notify, notifybar};
use rapidr_value::{v_int, v_str};

use crate::{FormUi, KernelEvent, MemStore, Mods, TextSystem};

/// A 640 x 400 form with a button at its top left and an RNotify over all of
/// it (as `Align = alClient` makes it).
fn setup() -> (MemStore, FormUi, TextSystem) {
    rapidr_value::theme::set(&rapidr_value::theme::RAPIDR);
    let mut s = MemStore::new();
    s.add("form", "RFORM", None).set("form", "width", v_int(640)).set("form", "height", v_int(400)).set("form", "clientwidth", v_int(640)).set("form", "clientheight", v_int(400));
    s.add("btn", "RBUTTON", Some("form")).set("btn", "caption", v_str("Press")).set("btn", "left", v_int(10)).set("btn", "top", v_int(10));
    s.add("notify", "RNOTIFY", Some("form")).set("notify", "left", v_int(0)).set("notify", "top", v_int(0)).set("notify", "width", v_int(640)).set("notify", "height", v_int(400));
    notify::with_mut("notify", |m| *m = notify::Notify::default());
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "form", false);
    drop(f.paint(&s, &mut ts, 1.0));
    f.take_events();
    (s, f, ts)
}

fn find<'a>(n: &'a AccessNode, role: Role, name: &str) -> Option<&'a AccessNode> {
    if n.role == role && n.name == name {
        return Some(n);
    }
    n.children.iter().find_map(|c| find(c, role, name))
}

#[test]
fn a_toast_takes_the_mouse_only_where_it_is() {
    let (s, mut f, mut ts) = setup();
    // (nothing said: the overlay is nowhere, the button under it is hit)
    assert_eq!(f.hit(20.0, 20.0).map(|i| f.nodes[i].id.clone()).as_deref(), Some("btn"));
    notify::with_mut("notify", |m| {
        m.say("Saved", notify::Kind::Success, notify::parse_actions("Show"), None, "", "10:42");
    });
    drop(f.paint(&s, &mut ts, 1.0));
    let card = notify::with("notify", |m| m.shown[0].rect).unwrap();
    // (over the toast: the overlay; anywhere else on the form: what is under it)
    assert_eq!(f.hit((card.0 + 20) as f64, (card.1 + 8) as f64).map(|i| f.nodes[i].id.clone()).as_deref(), Some("notify"));
    assert_eq!(f.hit(20.0, 20.0).map(|i| f.nodes[i].id.clone()).as_deref(), Some("btn"), "a click beside the toast goes to the button");
}

#[test]
fn a_toasts_action_and_close_button_are_pressed_with_the_mouse() {
    let (s, mut f, mut ts) = setup();
    notify::with_mut("notify", |m| {
        m.say("Saved", notify::Kind::Info, notify::parse_actions("Show|Undo"), None, "k", "10:42");
    });
    drop(f.paint(&s, &mut ts, 1.0));
    let g = notify::with("notify", |m| m.shown[0].clone()).unwrap();
    let (ax, ay, aw, ah) = g.actions[1].1;
    let (px, py) = ((ax + aw / 2) as f64, (ay + ah / 2) as f64);
    f.mouse_move(&s, &mut ts, px, py, Mods::NONE);
    f.mouse_down(&s, &mut ts, px, py, Button::Left, Mods::NONE);
    f.mouse_up(&s, &mut ts, px, py, Button::Left, Mods::NONE);
    let events = f.take_events();
    let pressed = events.iter().find_map(|e| match e {
        KernelEvent::Container(crate::components::form::Container::Panel { action: rapidr_value::panels::User::Notify(notify::User::Action { index, key, .. }), .. }) => Some((*index, key.clone())),
        _ => None,
    });
    assert_eq!(pressed, Some((1, "k".to_string())), "the second action, with its key: {events:?}");
    // (the close button: the toast starts going, nothing else)
    let (cx, cy) = ((g.close.0 + g.close.2 / 2) as f64, (g.close.1 + g.close.3 / 2) as f64);
    f.mouse_down(&s, &mut ts, cx, cy, Button::Left, Mods::NONE);
    f.mouse_up(&s, &mut ts, cx, cy, Button::Left, Mods::NONE);
    f.take_events();
    assert!(notify::with("notify", |m| m.toasts[0].leaving.is_some()).unwrap());
}

#[test]
fn a_toast_is_a_group_with_its_buttons_for_a_screen_reader() {
    let (s, mut f, mut ts) = setup();
    notify::with_mut("notify", |m| {
        m.say("5 files were reloaded: main.rr, form1.rr, utils.inc +2", notify::Kind::Info, notify::parse_actions("&Show|&Undo"), None, "", "10:42");
    });
    drop(f.paint(&s, &mut ts, 1.0));
    let tree = f.access_tree(&s, &mut ts);
    let card = find(&tree, Role::Group, "5 files were reloaded: main.rr, form1.rr, utils.inc +2 (Show, Undo)").expect("the toast, its words and its actions");
    let names: Vec<&str> = card.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Show", "Undo", "Close"]);
    assert!(card.children.iter().all(|c| c.role == Role::Button));
}

#[test]
fn the_history_closes_with_a_click_outside_it() {
    let (s, mut f, mut ts) = setup();
    notify::with_mut("notify", |m| {
        m.say("One", notify::Kind::Info, Vec::new(), None, "", "10:42");
        m.history_open = true;
    });
    drop(f.paint(&s, &mut ts, 1.0));
    let card = notify::with("notify", |m| m.shown_history.clone().unwrap().rect).unwrap();
    // (while it is open the overlay takes the whole form: a click outside is its)
    assert_eq!(f.hit(20.0, 20.0).map(|i| f.nodes[i].id.clone()).as_deref(), Some("notify"));
    f.mouse_down(&s, &mut ts, 20.0, 20.0, Button::Left, Mods::NONE);
    f.mouse_up(&s, &mut ts, 20.0, 20.0, Button::Left, Mods::NONE);
    f.take_events();
    assert!(!notify::with("notify", |m| m.history_open).unwrap(), "closed; {card:?}");
}

#[test]
fn a_strip_lays_out_its_buttons_and_answers_a_click() {
    rapidr_value::theme::set(&rapidr_value::theme::RAPIDR);
    let mut s = MemStore::new();
    s.add("form", "RFORM", None).set("form", "width", v_int(800)).set("form", "height", v_int(300)).set("form", "clientwidth", v_int(800)).set("form", "clientheight", v_int(300));
    s.add("bar", "RNOTIFYBAR", Some("form")).set("bar", "left", v_int(0)).set("bar", "top", v_int(0)).set("bar", "width", v_int(800)).set("bar", "height", v_int(36));
    notifybar::with_mut("bar", |b| {
        *b = notifybar::Bar { text: "main.rr changed on disk while you were editing it.".into(), buttons: notifybar::parse_buttons("&Compare,&Use Disk Version,&Keep Mine"), ..Default::default() };
    });
    let mut ts = TextSystem::new();
    let mut f = FormUi::build_unfocused(&s, "form", false);
    drop(f.paint(&s, &mut ts, 1.0));
    let tree = f.access_tree(&s, &mut ts);
    let bar = find(&tree, Role::Group, "main.rr changed on disk while you were editing it. Compare, Use Disk Version, Keep Mine").expect("the strip");
    assert_eq!(bar.children.len(), 3);
    let b = &bar.children[2];
    assert_eq!(b.name, "Keep Mine");
    let (x, y) = ((b.bounds.0 + b.bounds.2 / 2) as f64, (b.bounds.1 + b.bounds.3 / 2) as f64);
    f.mouse_move(&s, &mut ts, x, y, Mods::NONE);
    f.mouse_down(&s, &mut ts, x, y, Button::Left, Mods::NONE);
    f.mouse_up(&s, &mut ts, x, y, Button::Left, Mods::NONE);
    let chose = f.take_events().into_iter().any(|e| matches!(e, KernelEvent::Container(crate::components::form::Container::Panel { action: rapidr_value::panels::User::NotifyBar(notifybar::User::Choose(2)), .. })));
    assert!(chose, "the third button was chosen");
}

/// Tab, Enter and Escape with the keyboard on the toasts: the form's own Tab
/// walk used to carry the focus past them.
#[test]
fn tab_steps_through_a_toasts_buttons_and_enter_presses_one() {
    use crate::input::MemClipboard;
    let (s, mut f, mut ts) = setup();
    notify::with_mut("notify", |m| {
        m.say("Made it", notify::Kind::Success, notify::parse_actions("Run It|Undo"), None, "k", "10:42");
    });
    drop(f.paint(&s, &mut ts, 1.0));
    let press = |f: &mut FormUi, ts: &mut TextSystem, vk: i64, shift: bool| {
        f.key_down(&s, ts, vk, "", Mods { shift, ..Mods::NONE }, &mut MemClipboard::default());
        f.take_events()
    };
    let kept = || notify::with("notify", |m| m.focus).flatten();
    // (the keyboard arrives — F6, SetFocus — on the first action)
    assert!(f.focus_id(&s, "notify"));
    assert_eq!(kept(), Some(notify::Hit::Action(1, 0)));
    // Tab: the second action; Enter presses it (1-based for the program: its index 1 here is 0-based)
    press(&mut f, &mut ts, 9, false);
    assert_eq!(kept(), Some(notify::Hit::Action(1, 1)));
    let events = press(&mut f, &mut ts, 13, false);
    let pressed = events.iter().find_map(|e| match e {
        KernelEvent::Container(crate::components::form::Container::Panel { action: rapidr_value::panels::User::Notify(notify::User::Action { index, .. }), .. }) => Some(*index),
        _ => None,
    });
    assert_eq!(pressed, Some(1), "{events:?}");
    // Shift+Tab goes back; Tab twice more goes to the close button and then leaves
    drop(f.paint(&s, &mut ts, 1.0));
    notify::with_mut("notify", |m| m.focus = Some(notify::Hit::Action(1, 0)));
    press(&mut f, &mut ts, 9, true);
    assert_eq!(f.focused(), Some("btn"), "Shift+Tab past the first button leaves the toasts");
    assert_eq!(kept(), None);
}
