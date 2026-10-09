//! The inspector headless: a form in a [`MemStore`] (a host over it reads
//! and sets the inspected button), input in, the program's actions out.

use std::cell::RefCell;

use rapidr_value::objects::a11y::{AccessNode, Action, Role};
use rapidr_value::panels::inspector::{self as model, User as Act};
use rapidr_value::panels::subject::Host;
use rapidr_value::panels::User;
use rapidr_value::{v_int, v_str, Value};

use crate::components::form::Container;
use crate::{FormUi, KernelEvent, MemClipboard, MemStore, Mods, TextSystem};

/// The runtime as the inspector reaches it: the store's properties.
struct H(RefCell<MemStore>);

impl Host for H {
    fn get(&self, name: &str, prop: &str) -> Value {
        crate::store::Store::get(&*self.0.borrow(), name, prop)
    }
    fn set(&self, name: &str, prop: &str, v: Value) {
        self.0.borrow_mut().set(name, prop, v);
    }
    fn call(&self, _: &str, _: &str, _: &[Value]) -> Value {
        Value::Null
    }
    fn type_of(&self, name: &str) -> String {
        crate::store::Store::type_of(&*self.0.borrow(), name)
    }
    fn components(&self) -> Vec<(String, String)> {
        let s = self.0.borrow();
        s.ids().iter().map(|i| (i.clone(), crate::store::Store::type_of(&*s, i))).collect()
    }
    fn fire(&self, _: &str, _: &str, _: &[Value]) {}
    fn invalidate(&self) {}
}

fn setup(target: &str) -> (H, FormUi, TextSystem) {
    let mut s = MemStore::new();
    s.add("f", "RFORM", None).set("f", "width", v_int(640)).set("f", "height", v_int(520));
    s.add("insp", "RPROPERTYINSPECTOR", Some("f")).set("insp", "width", v_int(300)).set("insp", "height", v_int(470));
    s.add("b1", "RBUTTON", Some("f")).set("b1", "left", v_int(320)).set("b1", "caption", v_str("OK"));
    let h = H(RefCell::new(s));
    model::remove("insp");
    model::with_mut("insp", |m| m.target = target.into());
    model::refresh(&h, "insp");
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&*h.0.borrow(), "f", false);
    drop(f.paint(&*h.0.borrow(), &mut ts, 1.0));
    f.focus_id(&*h.0.borrow(), "insp");
    (h, f, ts)
}

/// The inspector's actions for the program among `events`.
fn acts(events: Vec<KernelEvent>) -> Vec<User> {
    events
        .into_iter()
        .filter_map(|e| match e {
            KernelEvent::Container(Container::Panel { action, .. }) => Some(action),
            _ => None,
        })
        .collect()
}

fn key(f: &mut FormUi, h: &H, ts: &mut TextSystem, vk: i64, text: &str, mods: Mods) {
    f.key_down(&*h.0.borrow(), ts, vk, text, mods, &mut MemClipboard::default());
}

fn type_text(f: &mut FormUi, h: &H, ts: &mut TextSystem, text: &str) {
    for c in text.chars() {
        let vk = rapidr_value::input::vk_of_char(c).unwrap_or(0);
        key(f, h, ts, vk, &c.to_string(), Mods::NONE);
    }
}

fn find<'a>(n: &'a AccessNode, f: &dyn Fn(&AccessNode) -> bool) -> Option<&'a AccessNode> {
    if f(n) {
        return Some(n);
    }
    n.children.iter().find_map(|c| find(c, f))
}

#[test]
fn typing_a_value_and_the_keyboard() {
    let (h, mut f, mut ts) = setup("b1");
    // (type-ahead to Caption, F2, a new caption, Enter: the program hears it)
    type_text(&mut f, &h, &mut ts, "cap");
    assert_eq!(model::with("insp", |m| m.ui.selected.clone()).flatten().as_deref(), Some("caption"));
    f.take_events();
    key(&mut f, &h, &mut ts, 113, "", Mods::NONE);
    type_text(&mut f, &h, &mut ts, "Go");
    key(&mut f, &h, &mut ts, 13, "", Mods::NONE);
    let a = acts(f.take_events());
    assert!(a.contains(&User::Inspector(Act::Commit { key: "caption".into(), text: "Go".into() })), "{a:?}");
    // (a wrong number stays in its editor, the row says why)
    model::with_mut("insp", |m| m.ui.selected = Some("width".into()));
    key(&mut f, &h, &mut ts, 113, "", Mods::NONE);
    type_text(&mut f, &h, &mut ts, "x");
    key(&mut f, &h, &mut ts, 13, "", Mods::NONE);
    assert!(model::with("insp", |m| m.ui.error.clone()).flatten().is_some_and(|(k, _)| k == "width"));
    assert!(crate::components::list::editing("insp").is_some());
    key(&mut f, &h, &mut ts, 27, "", Mods::NONE);
    assert!(crate::components::list::editing("insp").is_none());
    // (Space ticks a Boolean; Alt+Down drops an enum's list; Delete resets)
    model::with_mut("insp", |m| m.ui.selected = Some("default".into()));
    f.take_events();
    key(&mut f, &h, &mut ts, 32, " ", Mods::NONE);
    assert!(acts(f.take_events()).contains(&User::Inspector(Act::Commit { key: "default".into(), text: "True".into() })));
    model::with_mut("insp", |m| m.ui.selected = Some("cursor".into()));
    key(&mut f, &h, &mut ts, 40, "", Mods { alt: true, ..Mods::NONE });
    assert!(acts(f.take_events()).iter().any(|a| matches!(a, User::Inspector(Act::Drop { key, .. }) if key == "cursor")));
    key(&mut f, &h, &mut ts, 39, "", Mods::NONE);
    assert!(acts(f.take_events()).contains(&User::Inspector(Act::Commit { key: "cursor".into(), text: "crNone".into() })));
}

#[test]
fn the_anchors_pins_and_the_tree_for_screen_readers() {
    let (h, mut f, mut ts) = setup("b1");
    model::with_mut("insp", |m| {
        m.set_open("anchors", true);
        m.ui.selected = Some("anchors#pins".into());
        m.ui.pin = 1;
    });
    // (Space on the focused pin: the top one turned off)
    key(&mut f, &h, &mut ts, 32, " ", Mods::NONE);
    assert!(acts(f.take_events()).contains(&User::Inspector(Act::Commit { key: "anchors.aktop".into(), text: "False".into() })));
    // (the arrows go from pin to pin)
    key(&mut f, &h, &mut ts, 39, "", Mods::NONE);
    assert_eq!(model::with("insp", |m| m.ui.pin), Some(0));
    // (the tree: a grid of rows, the pins check boxes; a click on one)
    let tree = f.access_tree(&*h.0.borrow(), &mut ts);
    let grid = find(&tree, &|n| n.role == Role::Grid).expect("a grid");
    assert!(grid.name.contains("b1"), "{}", grid.name);
    let caption = find(grid, &|n| n.role == Role::Row && n.name == "Caption").expect("Caption's row");
    assert_eq!((caption.value.as_deref(), caption.level), (Some("OK"), Some(2)));
    let right = find(grid, &|n| n.role == Role::CheckBox && n.name == "Anchor right").expect("the right pin");
    assert_eq!(right.states.checked, Some(false));
    let id = right.id;
    assert!(f.access_action(&*h.0.borrow(), &mut ts, id, Action::Click, None));
    assert!(acts(f.take_events()).contains(&User::Inspector(Act::Commit { key: "anchors.akright".into(), text: "True".into() })));
    // (SetValue on a row: as if typed)
    let id = find(&f.access_tree(&*h.0.borrow(), &mut ts), &|n| n.role == Role::Row && n.name == "Hint").unwrap().id;
    assert!(f.access_action(&*h.0.borrow(), &mut ts, id, Action::SetValue, Some(crate::a11y::AccessValue::Text("Press me".into()))));
    assert!(acts(f.take_events()).contains(&User::Inspector(Act::Commit { key: "hint".into(), text: "Press me".into() })));
}

#[test]
fn every_row_kind_draws_in_every_theme() {
    // (an inspector inspecting another: a font, a list of lines, colours)
    let (h, mut f, mut ts) = setup("insp2");
    h.0.borrow_mut().add("insp2", "RPROPERTYINSPECTOR", Some("f")).set("insp2", "left", v_int(320));
    model::with_mut("insp2", |m| m.handlers = "A()\nB(x)".into());
    model::refresh(&h, "insp");
    model::with_mut("insp", |m| {
        m.set_all_open(true);
        m.ui.selected = Some("font.bold".into());
    });
    let rows = model::with("insp", |m| m.rows()).unwrap();
    assert!(rows.iter().any(|r| r.key == "font.name"), "a font's parts");
    for t in rapidr_value::theme::ALL {
        rapidr_value::theme::set(t);
        for scale in [1.0, 2.0] {
            let list = f.paint(&*h.0.borrow(), &mut ts, scale);
            assert!(!list.items.is_empty());
        }
    }
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
    // (A–Z, the Events page, a search that finds nothing)
    model::with_mut("insp", |m| {
        m.alphabetic = true;
        m.events_page = true;
        m.filter = "zzz".into();
    });
    drop(f.paint(&*h.0.borrow(), &mut ts, 1.0));
    key(&mut f, &h, &mut ts, 70, "f", Mods { ctrl: true, ..Mods::NONE });
    assert!(crate::components::list::editing("insp").is_some(), "Ctrl+F: the search box");
}

/// The line between the name and value columns is a divider that drags:
/// the column resize pointer over it (kept while it is held), the accent
/// line under the mouse, the I-beam in the search box and an open editor,
/// the arrow elsewhere.
#[test]
fn the_divider_between_the_columns_shows_the_resize_pointer() {
    use model::Hover;
    use rapidr_value::input::{Button, Cursor};
    let (h, mut f, mut ts) = setup("b1");
    let s = h.0.borrow();
    let probe = |f: &mut FormUi, ts: &mut TextSystem, x: i64, y: i64| {
        f.mouse_move(&*s, ts, x as f64 + 0.5, y as f64 + 0.5, Mods::NONE);
        f.pointer_at(&*s, ts, x as f64 + 0.5, y as f64 + 0.5)
    };
    // (scan the panel for where the pointer is the divider's)
    let mut found = None;
    'scan: for y in 60..460 {
        for x in 20..280 {
            if probe(&mut f, &mut ts, x, y) == Cursor::ColResize {
                found = Some((x, y));
                break 'scan;
            }
        }
    }
    let (x, y) = found.expect("a divider somewhere on a row that has two columns");
    assert_eq!(model::with("insp", |m| m.ui.hover.clone()).flatten(), Some(Hover::Divider), "the line is lit");
    // (a band of a few pixels, not the whole row)
    assert_ne!(probe(&mut f, &mut ts, x + 12, y), Cursor::ColResize);
    assert_ne!(probe(&mut f, &mut ts, x - 12, y), Cursor::ColResize);
    // (held: the pointer follows the mouse, the column the drag; let go: the arrow)
    let before = model::with("insp", |m| m.name_width).unwrap();
    probe(&mut f, &mut ts, x, y);
    f.mouse_down(&*s, &mut ts, x as f64 + 0.5, y as f64 + 0.5, Button::Left, Mods::NONE);
    assert_eq!(probe(&mut f, &mut ts, x + 30, y + 90), Cursor::ColResize);
    assert_ne!(model::with("insp", |m| m.name_width).unwrap(), before);
    f.mouse_up(&*s, &mut ts, x as f64 + 30.5, y as f64 + 90.5, Button::Left, Mods::NONE);
    // (the divider is where the mouse let go; elsewhere the arrow)
    assert_eq!(probe(&mut f, &mut ts, x + 30, y + 90), Cursor::ColResize);
    assert_ne!(probe(&mut f, &mut ts, x + 60, y + 90), Cursor::ColResize);
    // (the search box above the rows)
    assert!((0..120).step_by(3).any(|yy| (0..300).step_by(6).any(|xx| probe(&mut f, &mut ts, xx, yy) == Cursor::IBeam)), "the search box takes the I-beam");
}
