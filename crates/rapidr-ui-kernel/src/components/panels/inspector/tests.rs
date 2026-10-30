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

/// The inspector in Studio's font (Inter, 10 pt), whose widths the column's default is made for.
fn in_studios_font(h: &H) {
    h.0.borrow_mut().set("insp", "fontname", v_str("Inter")).set("insp", "fontsize", v_int(10));
}

/// The tooltip a mouse resting at (x, y) of the form shows (the tips' clock
/// stepped past HintPause).
fn hover_tip(f: &mut FormUi, h: &H, ts: &mut TextSystem, x: i64, y: i64) -> Option<String> {
    let s = h.0.borrow();
    let t0 = crate::tick::now();
    crate::tick::set_test_now(Some(t0));
    f.mouse_move(&*s, ts, x as f64 + 0.5, y as f64 + 0.5, Mods::NONE);
    let later = t0 + std::time::Duration::from_millis(800);
    crate::tick::set_test_now(Some(later));
    f.tick(&*s, ts, later);
    crate::tick::set_test_now(None);
    f.hint_shown().map(str::to_string)
}

/// The texts a painting of the form draws.
fn drawn_words(f: &mut FormUi, ts: &mut TextSystem, h: &H) -> Vec<String> {
    f.paint(&*h.0.borrow(), ts, 1.0)
        .items
        .iter()
        .filter_map(|i| match i {
            crate::Item::Op { op: crate::Op::Text { text, .. }, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Where the first row's name column is (the rows by A to Z, the search
/// narrowed to `filter`): a point inside it, and one in the value column.
fn first_row(h: &H, filter: &str) -> ((i64, i64), (i64, i64)) {
    model::with_mut("insp", |m| {
        m.alphabetic = true;
        m.filter = filter.into();
    });
    model::refresh(h, "insp");
    let (g, nw) = model::with("insp", |m| (super::geo::geo(m, 300, 470, 0), m.name_width)).unwrap();
    let y = g.list.1 + 11;
    ((g.list.0 + 60, y), (g.list.0 + nw + 30, y))
}

/// (INSPECTOR-CATEGORIES) A property is under the heading of what it is for;
/// no heading names where it comes from: the three accessibility ones sit
/// together under Accessibility, Anchors with the layout ones, FontSize with
/// the font ones.
#[test]
fn every_row_is_under_a_heading_of_its_purpose() {
    let (_h, _f, _ts) = setup("b1");
    let rows = model::with("insp", |m| m.rows()).unwrap();
    let under = |name: &str| {
        let i = rows.iter().position(|r| r.name == name && r.kind == model::RowKind::Prop).unwrap_or_else(|| panic!("{name}'s row"));
        rows[..i].iter().rev().find(|r| r.kind == model::RowKind::Category).map(|r| r.name.clone()).unwrap()
    };
    for (name, heading) in [
        ("AccessibleName", "Accessibility"),
        ("AccessibleDescription", "Accessibility"),
        ("AccessibleHidden", "Accessibility"),
        ("Anchors", "Layout"),
        ("MinWidth", "Layout"),
        ("MaxHeight", "Layout"),
        ("Width", "Layout"),
        ("FontColor", "Font"),
        ("FontSize", "Font"),
        ("Hint", "Help"),
        ("Caption", "Appearance"),
        ("Default", "Behavior"),
        ("ModalResult", "Window"),
    ] {
        assert_eq!(under(name), heading, "{name}");
    }
    let headings: Vec<&str> = rows.iter().filter(|r| r.kind == model::RowKind::Category).map(|r| r.name.as_str()).collect();
    assert!(!headings.iter().any(|n| n.to_lowercase().contains("rapidr") || n.to_lowercase().contains("extension")), "{headings:?}");
    // (a heading carries no note of where its members come from, marking on or off)
    model::with_mut("insp", |m| m.mark_non_classic = true);
    assert!(model::with("insp", |m| m.rows()).unwrap().iter().all(|r| r.kind != model::RowKind::Category || !r.non_classic));
}

/// (INSPECTOR-CATEGORIES) A member older programs lack is marked only while
/// the inspector is told to (a project kept classic-compatible): a small
/// calm "new" after its name, its tooltip and its screen reader's
/// description saying "Not in classic programs", never the old "R". Unmarked:
/// nothing at all. In every look, and in Spanish.
#[test]
fn a_member_older_programs_lack_is_marked_only_in_a_classic_compatible_project() {
    let (h, mut f, mut ts) = setup("b1");
    in_studios_font(&h);
    let (name_at, _) = first_row(&h, "anchors");
    // (unmarked: no note drawn, no tip, no description)
    let drawn = drawn_words(&mut f, &mut ts, &h);
    assert!(drawn.iter().any(|t| t == "Anchors"), "{drawn:?}");
    assert!(!drawn.iter().any(|t| t == "new" || t == "R"), "{drawn:?}");
    assert_eq!(hover_tip(&mut f, &h, &mut ts, name_at.0, name_at.1), None);
    let tree = f.access_tree(&*h.0.borrow(), &mut ts);
    assert_eq!(find(&tree, &|n| n.role == Role::Row && n.name == "Anchors").expect("Anchors' row").description, "");
    // (marked: the note, the tip, the description)
    model::with_mut("insp", |m| m.mark_non_classic = true);
    for t in rapidr_value::theme::ALL {
        rapidr_value::theme::set(t);
        let drawn = drawn_words(&mut f, &mut ts, &h);
        assert!(drawn.iter().any(|w| w == "new"), "{}: the note, as the design guide's soft chip: {drawn:?}", t.name);
        assert!(!drawn.iter().any(|w| w == "R"), "{}: no letter badge", t.name);
    }
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
    assert_eq!(hover_tip(&mut f, &h, &mut ts, name_at.0, name_at.1).as_deref(), Some("Not in classic programs"));
    let tree = f.access_tree(&*h.0.borrow(), &mut ts);
    assert_eq!(find(&tree, &|n| n.role == Role::Row && n.name == "Anchors").unwrap().description, "Not in classic programs");
    // (Caption, which classic programs have, is never marked)
    let (cap_at, _) = first_row(&h, "caption");
    assert_eq!(hover_tip(&mut f, &h, &mut ts, cap_at.0, cap_at.1), None);
    let tree = f.access_tree(&*h.0.borrow(), &mut ts);
    assert_eq!(find(&tree, &|n| n.role == Role::Row && n.name == "Caption").expect("Caption's row").description, "changed");
    // (Spanish: "nuevo", and the words in the tip and the description)
    rapidr_value::i18n::set_language("es");
    let (name_at, _) = first_row(&h, "anchors");
    assert!(drawn_words(&mut f, &mut ts, &h).iter().any(|w| w == "nuevo"));
    assert_eq!(hover_tip(&mut f, &h, &mut ts, name_at.0, name_at.1).as_deref(), Some("No está en los programas clásicos"));
    rapidr_value::i18n::set_language("");
}

/// (INSPECTOR-CATEGORIES) A name the column cuts short — `AccessibleDescription`
/// at the default width — shows whole in its tooltip (a screen reader has it
/// whole in the row's name); a name that fits, none. A value cut short, likewise.
#[test]
fn a_name_the_column_cuts_short_shows_whole_in_its_tooltip() {
    let (h, mut f, mut ts) = setup("b1");
    in_studios_font(&h);
    let (name_at, _) = first_row(&h, "accessibled");
    let drawn = drawn_words(&mut f, &mut ts, &h);
    // (drawn in runs, the search's matched letters apart: never whole in one)
    assert!(!drawn.iter().any(|t| t == "AccessibleDescription"), "cut at the default width: {drawn:?}");
    assert_eq!(hover_tip(&mut f, &h, &mut ts, name_at.0, name_at.1).as_deref(), Some("AccessibleDescription"));
    let tree = f.access_tree(&*h.0.borrow(), &mut ts);
    assert!(find(&tree, &|n| n.role == Role::Row && n.name == "AccessibleDescription").is_some(), "the row's name is whole");
    // (the rest of the Accessibility trio fits the default width: no tip)
    for short in ["accessiblename", "accessiblehidden", "showhint"] {
        let (at, _) = first_row(&h, short);
        assert_eq!(hover_tip(&mut f, &h, &mut ts, at.0, at.1), None, "{short}");
    }
    // (a column dragged wider: the whole name shows, so no tip)
    model::with_mut("insp", |m| m.name_width = 200);
    let (at, _) = first_row(&h, "accessibled");
    assert_eq!(hover_tip(&mut f, &h, &mut ts, at.0, at.1), None);
    // (a value cut short: whole in its tip)
    model::with_mut("insp", |m| m.name_width = rapidr_value::panels::inspector::DEFAULT_NAME_WIDTH);
    h.0.borrow_mut().set("b1", "caption", v_str("A caption longer than the value column is wide"));
    model::refresh(&h, "insp");
    let (_, value_at) = first_row(&h, "caption");
    assert_eq!(hover_tip(&mut f, &h, &mut ts, value_at.0, value_at.1).as_deref(), Some("A caption longer than the value column is wide"));
    h.0.borrow_mut().set("b1", "caption", v_str("OK"));
    model::refresh(&h, "insp");
    let (_, value_at) = first_row(&h, "caption");
    assert_eq!(hover_tip(&mut f, &h, &mut ts, value_at.0, value_at.1), None);
}

/// (INSPECTOR-CATEGORIES) The names' column starts wide enough for the common
/// names — all of them but `AccessibleDescription` — and keeps where the user
/// dragged it while another component is inspected.
#[test]
fn the_default_names_column_fits_the_common_names() {
    let font = rapidr_value::objects::font::Font { name: "Inter".into(), size: 10, color: 0, styles: 0 };
    let room = rapidr_value::panels::inspector::DEFAULT_NAME_WIDTH - 20 - 4;
    let common = ["Caption", "BorderStyle", "KeyPreview", "WindowState", "AutoScroll", "ModalResult", "Alignment", "WordWrap", "ShowHint", "TabOrder", "AccessibleName", "AccessibleHidden", "MinHeight", "MaxWidth", "BrushColor"];
    for n in common {
        let w = rapidr_value::objects::text::text_size(n, &font).0;
        assert!(w <= room, "{n} is {w} pixels, the default column has room for {room}");
    }
    // (a pane as wide as Studio's Properties (272): the whole default; a narrower
    // one — a saved layout of 240 — gives the names 58 % so the values still read,
    // never under 120; where the user or the program put the divider is never cut)
    let mut m = model::Inspector::default();
    assert_eq!(super::geo::name_width(&m, 256), 148);
    assert_eq!(super::geo::name_width(&m, 224), 129);
    assert_eq!(super::geo::name_width(&m, 190), 120);
    m.name_set = true;
    assert_eq!(super::geo::name_width(&m, 224), 148);
    // (kept while the selection changes: it's the inspector's, not a row's)
    let (h, _f, _ts) = setup("b1");
    model::with_mut("insp", |m| m.name_width = 190);
    model::with_mut("insp", |m| m.target = "insp".into());
    model::refresh(&h, "insp");
    assert_eq!(model::with("insp", |m| m.name_width), Some(190));
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

/// (the final audit) The click that starts editing a value leaves it all
/// selected: typing replaces it (as in an inspector), not adds to
/// its end; F4 drops a value's list, as Alt+Down.
#[test]
fn a_click_on_a_value_then_typing_replaces_it() {
    let (h, mut f, mut ts) = setup("b1");
    let mut at = None;
    'scan: for y in (60..460).step_by(2) {
        for x in [200, 230] {
            f.mouse_down(&*h.0.borrow(), &mut ts, x as f64 + 0.5, y as f64 + 0.5, rapidr_value::input::Button::Left, Mods::NONE);
            f.mouse_up(&*h.0.borrow(), &mut ts, x as f64 + 0.5, y as f64 + 0.5, rapidr_value::input::Button::Left, Mods::NONE);
            let editing_caption = crate::components::list::editing("insp").is_some() && model::with("insp", |m| m.ui.selected.clone()).flatten().as_deref() == Some("caption");
            if editing_caption {
                at = Some((x, y));
                break 'scan;
            }
            key(&mut f, &h, &mut ts, 27, "", Mods::NONE);
        }
    }
    assert!(at.is_some(), "Caption's value cell somewhere");
    f.take_events();
    type_text(&mut f, &h, &mut ts, "Go");
    key(&mut f, &h, &mut ts, 13, "", Mods::NONE);
    let a = acts(f.take_events());
    assert!(a.contains(&User::Inspector(Act::Commit { key: "caption".into(), text: "Go".into() })), "typing replaced the caption: {a:?}");
}
