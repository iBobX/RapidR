//! The kernel without a window, a GPU or an OS: forms in a [`MemStore`],
//! input in, events, display lists and accessibility trees out. The first
//! tests are the prototype's (`rapidr-ui-proto/src/tests.rs`) on the same
//! demo form, now built from a store.

use rapidr_value::input::{Button, Mouse};
use rapidr_value::objects::a11y::{node_id, part_id, Action, Role, PART_ITEM, PART_TAB};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::text::text_size;
use rapidr_value::objects::{with_tabcontrol, with_textedit, with_textedit_mut, with_trackbar};
use rapidr_value::{v_int, v_str, Value};

use crate::display::Item;
use crate::{AccessValue, FormUi, KernelEvent, MemClipboard, MemStore, Mods, Op, TextSystem};

const NONE: Mods = Mods::NONE;
const SHIFT: Mods = Mods::SHIFT;
const CMD: Mods = Mods { shift: false, ctrl: false, alt: false, command: true, word: false };
const ALT: Mods = Mods { shift: false, ctrl: false, alt: true, command: false, word: false };

/// The prototype's demo form (`rapidr-ui-proto/src/demo.rs`):
///
/// ```basic
/// CREATE Form AS QFORM
///     Caption = "RapidR UI kernel"
///     CREATE lblName AS QLABEL : Caption = "Name:" : Left = 8 : Top = 12 : END CREATE
///     CREATE edName AS QEDIT : Text = "Grüße, ñandú ✓" : Left = 56 : Top = 8 : END CREATE
///     CREATE btnOK AS QBUTTON : Caption = "OK" : Left = 186 : Top = 8 : END CREATE
///     CREATE tbLevel AS QTRACKBAR : Left = 8 : Top = 44 : Position = 3 : END CREATE
///     CREATE tcPages AS QTABCONTROL : Left = 164 : Top = 44
///         AddTabs "One", "Two", "Three"
///         CREATE lblPage AS QLABEL : Caption = "Page One" : END CREATE
///     END CREATE
///     CREATE lblStatus AS QLABEL : Left = 8 : Top = 156 : Width = 300 : Height = 48 : END CREATE
/// END CREATE
/// ```
fn demo_store() -> MemStore {
    let mut s = MemStore::new();
    s.add("form", "RFORM", None).set("form", "caption", v_str("RapidR UI kernel"));
    s.add("lblName", "RLABEL", Some("form")).set("lblName", "caption", v_str("Name:")).set("lblName", "left", v_int(8)).set("lblName", "top", v_int(12));
    s.add("edName", "REDIT", Some("form")).set("edName", "text", v_str("Grüße, ñandú ✓")).set("edName", "left", v_int(56)).set("edName", "top", v_int(8));
    s.add("btnOK", "RBUTTON", Some("form")).set("btnOK", "caption", v_str("OK")).set("btnOK", "left", v_int(186)).set("btnOK", "top", v_int(8));
    s.add("tbLevel", "RTRACKBAR", Some("form")).set("tbLevel", "left", v_int(8)).set("tbLevel", "top", v_int(44)).set("tbLevel", "position", v_int(3));
    s.add("tcPages", "RTABCONTROL", Some("form")).set("tcPages", "left", v_int(164)).set("tcPages", "top", v_int(44));
    s.set("tcPages", "width", v_int(150)).set("tcPages", "height", v_int(100));
    s.call("tcPages", "addtabs", &[v_str("One"), v_str("Two"), v_str("Three")]);
    // (the page's label in the area the components fill: TabControl::display)
    let (dx, dy, _, _) = with_tabcontrol("tcpages", |t| t.display(150, 100, &Font::default())).unwrap();
    s.add("lblPage", "RLABEL", Some("tcPages")).set("lblPage", "caption", v_str("Page One")).set("lblPage", "left", v_int(dx + 4)).set("lblPage", "top", v_int(dy + 4));
    s.add("lblStatus", "RLABEL", Some("form")).set("lblStatus", "caption", v_str("Ready.")).set("lblStatus", "left", v_int(8)).set("lblStatus", "top", v_int(156));
    s.set("lblStatus", "width", v_int(300)).set("lblStatus", "height", v_int(48));
    s
}

/// The demo form's kernel side, painted once at 1x (as a shown window is).
fn setup() -> (MemStore, FormUi, TextSystem) {
    let store = demo_store();
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&store, "form", false);
    drop(f.paint(&store, &mut ts, 1.0));
    (store, f, ts)
}

fn focused(f: &FormUi) -> String {
    f.focused().unwrap_or("").to_string()
}

/// The events that reach handlers other than the mouse's and keys'.
fn clicks_and_changes(events: Vec<KernelEvent>) -> Vec<KernelEvent> {
    events.into_iter().filter(|e| matches!(e, KernelEvent::Click(_) | KernelEvent::Change(_))).collect()
}

fn click(id: &str) -> KernelEvent {
    KernelEvent::Click(id.to_string())
}

fn change(id: &str) -> KernelEvent {
    KernelEvent::Change(id.to_string())
}

fn key(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, vk: i64, text: &str, mods: Mods) -> Vec<KernelEvent> {
    let mut clip = MemClipboard::default();
    f.key_down(s, ts, vk, text, mods, &mut clip);
    f.take_events()
}

fn edit_model(id: &str) -> (String, usize, usize, bool) {
    with_textedit(id, |t| (t.text(), t.sel_start, t.sel_len, t.modified)).unwrap()
}

// ------------------------------------------------- the prototype's tests --

#[test]
fn tab_order_and_focus() {
    let (s, mut f, mut ts) = setup();
    assert_eq!(focused(&f), "edname", "the first tab stop has the focus");
    let mut seen = Vec::new();
    for _ in 0..4 {
        key(&mut f, &s, &mut ts, 9, "\t", NONE);
        seen.push(focused(&f));
    }
    assert_eq!(seen, ["btnok", "tblevel", "tcpages", "edname"], "labels are skipped, creation order");
    key(&mut f, &s, &mut ts, 9, "\t", SHIFT);
    assert_eq!(focused(&f), "tcpages");
}

#[test]
fn button_clicks_by_mouse_and_keyboard() {
    let (s, mut f, mut ts) = setup();
    f.mouse_down(&s, &mut ts, 223.0, 20.0, Button::Left, NONE);
    assert!(clicks_and_changes(f.take_events()).is_empty());
    assert_eq!(focused(&f), "btnok", "a button takes the focus when pressed");
    f.mouse_up(&s, &mut ts, 223.0, 20.0, Button::Left, NONE);
    assert_eq!(clicks_and_changes(f.take_events()), vec![click("btnok")]);
    // let go elsewhere: no click
    f.mouse_down(&s, &mut ts, 223.0, 20.0, Button::Left, NONE);
    f.mouse_up(&s, &mut ts, 10.0, 200.0, Button::Left, NONE);
    assert!(clicks_and_changes(f.take_events()).is_empty());
    assert_eq!(clicks_and_changes(key(&mut f, &s, &mut ts, 32, " ", NONE)), vec![click("btnok")]);
    // a right click isn't a click
    f.mouse_down(&s, &mut ts, 223.0, 20.0, Button::Right, NONE);
    f.mouse_up(&s, &mut ts, 223.0, 20.0, Button::Right, NONE);
    assert!(clicks_and_changes(f.take_events()).is_empty());
}

#[test]
fn track_bar_and_tab_control_models_get_the_input() {
    let (s, mut f, mut ts) = setup();
    f.mouse_down(&s, &mut ts, 58.0, 58.0, Button::Left, NONE);
    f.mouse_move(&s, &mut ts, 108.0, 58.0, NONE);
    f.mouse_up(&s, &mut ts, 108.0, 58.0, Button::Left, NONE);
    assert_eq!(clicks_and_changes(f.take_events()), vec![change("tblevel")]);
    assert_eq!(with_trackbar("tblevel", |t| t.position), Some(7));
    assert_eq!(clicks_and_changes(key(&mut f, &s, &mut ts, 39, "", NONE)), vec![change("tblevel")]);
    f.mouse_down(&s, &mut ts, 220.0, 55.0, Button::Left, NONE);
    assert_eq!(clicks_and_changes(f.take_events()), vec![change("tcpages")]);
    assert_eq!(with_tabcontrol("tcpages", |t| t.index), Some(1));
    assert_eq!(focused(&f), "tcpages", "a tab clicked takes the focus");
}

#[test]
fn edit_typing_selection_and_clipboard_reach_the_model() {
    let (s, mut f, mut ts) = setup();
    let mut clip = MemClipboard::default();
    let press = |f: &mut FormUi, ts: &mut TextSystem, vk: i64, text: &str, m: Mods, clip: &mut MemClipboard| {
        f.key_down(&s, ts, vk, text, m, clip);
        f.take_events()
    };
    press(&mut f, &mut ts, 35, "", NONE, &mut clip);
    assert_eq!(clicks_and_changes(press(&mut f, &mut ts, 0, "é", NONE, &mut clip)), vec![change("edname")]);
    press(&mut f, &mut ts, 37, "", SHIFT, &mut clip);
    press(&mut f, &mut ts, 37, "", SHIFT, &mut clip);
    assert_eq!(edit_model("edname"), ("Grüße, ñandú ✓é".to_string(), 13, 2, true), "SelStart / SelLength in characters");
    press(&mut f, &mut ts, 88, "x", CMD, &mut clip);
    assert_eq!(clip.0.as_deref(), Some("✓é"));
    press(&mut f, &mut ts, 36, "", NONE, &mut clip);
    press(&mut f, &mut ts, 86, "v", CMD, &mut clip);
    assert_eq!(edit_model("edname").0, "✓éGrüße, ñandú ");
    // input methods' commits (dead keys, compositions) go in too
    f.ime_commit(&s, &mut ts, "ß");
    assert_eq!(edit_model("edname").0, "✓éßGrüße, ñandú ");
}

#[test]
fn accessibility_tree_describes_and_operates_the_form() {
    let (s, mut f, mut ts) = setup();
    let tree = f.access_tree(&s, &mut ts);
    let mut roles = Vec::new();
    tree.walk(&mut |n| roles.push(n.role));
    for r in [Role::Window, Role::Label, Role::Button, Role::TextInput, Role::Slider, Role::TabList, Role::Tab] {
        assert!(roles.contains(&r), "{r:?}");
    }
    assert_eq!(tree.name, "RapidR UI kernel");
    let button = tree.find(node_id("btnok")).unwrap();
    assert_eq!((button.role, button.name.as_str()), (Role::Button, "OK"));
    let edit = tree.find(node_id("edname")).unwrap();
    assert_eq!(edit.value.as_deref(), Some("Grüße, ñandú ✓"));
    assert_eq!(edit.name, "Name:", "named by the label to its left");
    assert_eq!(edit.labelled_by, Some(node_id("lblname")));
    assert_eq!(tree.find(node_id("tblevel")).unwrap().name, "", "the label above names the edit to its right already");
    assert!(edit.states.focused);
    // VoiceOver pressing the button: OnClick
    assert!(f.access_action(&s, &mut ts, node_id("btnok"), Action::Click, None));
    assert_eq!(f.take_events(), vec![click("btnok")]);
}

// ------------------------------------------------------------ routing --

#[test]
fn key_events_come_in_rapidq_order() {
    let (s, mut f, mut ts) = setup();
    // typing into the edit: OnKeyDown, the model (OnChange), OnKeyPress
    let chain = vec!["edname".to_string(), "form".to_string()];
    assert_eq!(
        key(&mut f, &s, &mut ts, 65, "a", NONE),
        vec![KernelEvent::KeyDown { chain: chain.clone(), vk: 65, shift: 0, text: "a".into() }, change("edname"), KernelEvent::KeyPress { chain: chain.clone(), key: 97 }]
    );
    f.key_up(65, NONE);
    assert_eq!(f.take_events(), vec![KernelEvent::KeyUp { chain, vk: 65, shift: 0 }]);
    // a key that types nothing: no OnKeyPress; Shift in RapidQ's terms
    f.focus_id(&s, "tbLevel");
    let chain = vec!["tblevel".to_string(), "form".to_string()];
    assert_eq!(key(&mut f, &s, &mut ts, 39, "", SHIFT), vec![KernelEvent::KeyDown { chain, vk: 39, shift: 256, text: String::new() }, change("tblevel")]);
    // a component inside a tab control: its chain goes through it
    let mut s2 = demo_store();
    s2.add("edPage", "REDIT", Some("tcPages")).set("edPage", "top", v_int(40));
    let mut g = FormUi::build(&s2, "form", false);
    g.focus_id(&s2, "edPage");
    assert_eq!(g.key_chain(), ["edpage", "tcpages", "form"]);
    // no focus: the form's own
    let mut empty = MemStore::new();
    empty.add("lonely", "RFORM", None);
    let mut h = FormUi::build(&empty, "lonely", false);
    assert_eq!(key(&mut h, &empty, &mut ts, 13, "\r", NONE), vec![
        KernelEvent::KeyDown { chain: vec!["lonely".into()], vk: 13, shift: 0, text: "\r".into() },
        KernelEvent::KeyPress { chain: vec!["lonely".into()], key: 13 }
    ]);
}

#[test]
fn enter_and_escape_click_the_default_and_cancel_buttons() {
    let (mut s, _, mut ts) = setup();
    s.add("btnCancel", "RBUTTON", Some("form")).set("btnCancel", "caption", v_str("Cancel")).set("btnCancel", "left", v_int(186)).set("btnCancel", "top", v_int(120));
    s.set("btnOK", "default", v_int(-1)).set("btnCancel", "cancel", Value::Boolean(true));
    let mut f = FormUi::build(&s, "form", false);
    assert_eq!(focused(&f), "edname");
    assert_eq!(clicks_and_changes(key(&mut f, &s, &mut ts, 13, "\r", NONE)), vec![click("btnok")], "Enter in an edit: the Default button");
    assert_eq!(clicks_and_changes(key(&mut f, &s, &mut ts, 27, "\u{1b}", NONE)), vec![click("btncancel")]);
    // Enter on another button clicks that one
    f.focus_id(&s, "btnCancel");
    assert_eq!(clicks_and_changes(key(&mut f, &s, &mut ts, 13, "\r", NONE)), vec![click("btncancel")]);
    // the Default button's frame shows while no other button has the focus
    f.focus_id(&s, "edName");
    let list = f.paint(&s, &mut ts, 1.0);
    let ok_frame = list.items.iter().any(|i| matches!(i, Item::Op { origin: (186, 8), op: Op::Edge { light, .. } } if light == &vec![rapidr_value::theme::CLASSIC.frame]));
    assert!(ok_frame, "{}", list.dump());
    // a disabled Default button isn't clicked, nor focused by Tab
    s.set("btnOK", "enabled", v_int(0));
    f.sync(&s);
    assert!(clicks_and_changes(key(&mut f, &s, &mut ts, 13, "\r", NONE)).is_empty());
    assert!(!f.tab_order(&s).contains(&f.index_of("btnok").unwrap()));
    // Kind / ModalResult are data: bkOK's caption and result
    s.add("btnKind", "RBUTTON", Some("form")).set("btnKind", "kind", v_int(1));
    let d = crate::components::button::ButtonData::of(&s, "btnKind");
    assert_eq!((d.caption.as_str(), d.modal_result, d.closes_form()), ("OK", 1, true));
}

#[test]
fn tab_order_follows_taborder_and_tabstop() {
    let (mut s, _, mut ts) = setup();
    s.set("btnOK", "taborder", v_int(0)).set("edName", "taborder", v_int(1)).set("tcPages", "tabstop", v_int(0));
    let mut f = FormUi::build(&s, "form", false);
    let names = |f: &FormUi| f.tab_order(&s).iter().map(|&i| f.nodes[i].id.clone()).collect::<Vec<_>>();
    // explicit TabOrders first among equals, creation order breaks ties
    assert_eq!(names(&f), ["btnok", "edname", "tblevel"]);
    assert_eq!(focused(&f), "btnok");
    // TabStop = False: still focused by a click
    f.mouse_down(&s, &mut ts, 220.0, 55.0, Button::Left, NONE);
    assert_eq!(focused(&f), "tcpages");
    // hidden components are skipped, and lose the focus
    s.set("tcPages", "visible", v_int(0));
    f.sync(&s);
    assert_eq!(f.focused(), None);
    key(&mut f, &s, &mut ts, 9, "\t", NONE);
    assert_eq!(focused(&f), "btnok");
}

#[test]
fn mnemonics_click_buttons_and_focus_after_labels() {
    let (mut s, _, mut ts) = setup();
    s.set("btnOK", "caption", v_str("&OK")).set("lblName", "caption", v_str("&Name:"));
    let mut f = FormUi::build(&s, "form", false);
    f.focus_id(&s, "tbLevel");
    let ev = key(&mut f, &s, &mut ts, 79, "o", ALT);
    assert_eq!(clicks_and_changes(ev.clone()), vec![click("btnok")]);
    assert!(!ev.iter().any(|e| matches!(e, KernelEvent::KeyPress { .. })), "Alt + a letter types nothing");
    key(&mut f, &s, &mut ts, 78, "n", ALT);
    assert_eq!(focused(&f), "edname", "a label's mnemonic focuses what follows it");
    let tree = f.access_tree(&s, &mut ts);
    assert_eq!(tree.find(node_id("btnok")).unwrap().shortcut, Some('o'));
}

#[test]
fn mouse_events_hover_and_capture() {
    let (s, mut f, mut ts) = setup();
    // OnMouseDown in the component's own pixels, after the model
    f.mouse_down(&s, &mut ts, 60.0, 50.0, Button::Left, SHIFT);
    let ev = f.take_events();
    assert_eq!(ev.last(), Some(&KernelEvent::Mouse { id: "tblevel".into(), kind: Mouse::Down, button: Button::Left, x: 52, y: 6, shift: 256 }));
    // captured: moves and the release go to it, wherever the mouse is
    f.mouse_move(&s, &mut ts, 300.0, 200.0, NONE);
    f.mouse_up(&s, &mut ts, 300.0, 200.0, Button::Left, NONE);
    let ev = f.take_events();
    assert!(ev.contains(&KernelEvent::Mouse { id: "tblevel".into(), kind: Mouse::Move, button: Button::Left, x: 292, y: 156, shift: 0 }));
    assert!(ev.contains(&KernelEvent::Mouse { id: "tblevel".into(), kind: Mouse::Up, button: Button::Left, x: 292, y: 156, shift: 0 }));
    // the form's own area: its client coordinates
    f.mouse_down(&s, &mut ts, 312.0, 200.0, Button::Right, NONE);
    assert_eq!(f.take_events(), vec![KernelEvent::Mouse { id: "form".into(), kind: Mouse::Down, button: Button::Right, x: 312, y: 200, shift: 0 }]);
    f.mouse_up(&s, &mut ts, 312.0, 200.0, Button::Right, NONE);
    f.take_events();
    // hover follows the mouse; a button pressed and dragged off isn't drawn pushed
    f.mouse_move(&s, &mut ts, 223.0, 20.0, NONE);
    assert_eq!(f.hover, f.index_of("btnok"));
    f.mouse_down(&s, &mut ts, 223.0, 20.0, Button::Left, NONE);
    assert_eq!(f.pressed, f.index_of("btnok"));
    f.mouse_move(&s, &mut ts, 223.0, 100.0, NONE);
    let list = f.paint(&s, &mut ts, 1.0);
    let pushed = list.items.iter().any(|i| matches!(i, Item::Op { origin: (186, 8), op: Op::Edge { light, dark, .. } } if light == &vec![0x808080] && dark == &vec![0x808080]));
    assert!(!pushed, "not pushed while the mouse is off it");
    f.mouse_up(&s, &mut ts, 223.0, 100.0, Button::Left, NONE);
    assert!(clicks_and_changes(f.take_events()).is_empty());
    // a disabled component gets no mouse
    let mut s2 = demo_store();
    s2.set("btnOK", "enabled", v_int(0));
    let mut g = FormUi::build(&s2, "form", false);
    g.mouse_down(&s2, &mut ts, 223.0, 20.0, Button::Left, NONE);
    g.mouse_up(&s2, &mut ts, 223.0, 20.0, Button::Left, NONE);
    assert!(g.take_events().is_empty());
}

#[test]
fn tree_places_children_and_the_menu_bar() {
    let (mut s, f, _) = setup();
    let tc = f.node("tcPages").unwrap().abs;
    let page = f.node("lblPage").unwrap();
    assert_eq!(page.parent, f.index_of("tcpages"));
    assert_eq!((page.abs.0 - tc.0, page.abs.1 - tc.1), (page.rect.0, page.rect.1), "Left / Top relative to the parent");
    assert_eq!(f.client, (318, 209), "QFORM's 320 × 240 less its frame");
    // a QMAINMENU in the window: everything below it
    s.add("mnu", "RMAINMENU", Some("form"));
    let mut ts = TextSystem::new();
    let mut g = FormUi::build(&s, "form", true);
    assert_eq!(g.menu_offset, rapidr_value::layout::MAIN_MENU_HEIGHT);
    assert_eq!(g.node("btnOK").unwrap().abs, (186, 8 + 28, 75, 25));
    assert!(g.node("mnu").is_none(), "non-visual: not placed");
    g.mouse_down(&s, &mut ts, 223.0, 20.0, Button::Left, NONE);
    assert!(g.take_events().is_empty(), "the menu bar's: not the form's");
    g.mouse_down(&s, &mut ts, 223.0, 48.0, Button::Left, NONE);
    g.mouse_up(&s, &mut ts, 223.0, 48.0, Button::Left, NONE);
    assert_eq!(clicks_and_changes(g.take_events()), vec![click("btnok")]);
    // the system's menu bar (macOS): no offset
    assert_eq!(FormUi::build(&s, "form", false).menu_offset, 0);
    // hidden parents hide their children; children are clipped to them
    s.set("tcPages", "visible", v_int(0));
    let g = FormUi::build(&s, "form", false);
    assert!(!g.node("lblPage").unwrap().shown);
    let tc = g.node("tcPages").unwrap().abs;
    assert_eq!(g.hit((tc.0 + 20) as f64, (tc.1 + 40) as f64), None);
}

#[test]
fn rebuild_keeps_ui_state() {
    let (mut s, mut f, mut ts) = setup();
    key(&mut f, &s, &mut ts, 35, "", NONE);
    let before = f.node("edName").unwrap().ui.edit.as_ref().unwrap().text().to_string();
    s.add("btnNew", "RBUTTON", Some("form")).set("btnNew", "top", v_int(120));
    f.rebuild(&s);
    assert_eq!(focused(&f), "edname");
    assert!(f.node("btnNew").is_some());
    assert_eq!(f.node("edName").unwrap().ui.edit.as_ref().unwrap().text(), before);
    assert_eq!(f.structure_rev, 2);
}

#[test]
fn the_program_changing_the_model_shows_in_the_edit() {
    let (mut s, mut f, mut ts) = setup();
    s.set("edName", "text", v_str("Changed"));
    let list = f.paint(&s, &mut ts, 1.0);
    assert_eq!(f.node("edName").unwrap().ui.edit.as_ref().unwrap().text(), "Changed");
    assert!(f.editor_layout("edName").is_some());
    // drawn: a text item for the editor, inside its sunken edge, with the caret
    let item = list.items.iter().find_map(|i| if let Item::Text(t) = i { Some(t.clone()) } else { None }).unwrap();
    assert_eq!(item.node, "edname");
    assert!(item.caret.is_some() && item.selection.is_empty());
    assert_eq!(item.origin.0, 59.0, "the text area: Left + 3");
    // a selection set by the program shows (focused)
    with_textedit_mut("edname", |t| {
        t.set("selstart", &v_int(1));
        t.set("sellength", &v_int(3));
    });
    let list = f.paint(&s, &mut ts, 2.0);
    let item = list.items.iter().find_map(|i| if let Item::Text(t) = i { Some(t.clone()) } else { None }).unwrap();
    assert_eq!(item.selection.len(), 1);
    assert!(item.caret.is_none());
    assert_eq!(item.origin.0, 118.0, "device pixels at 2x");
}

#[test]
fn input_method_composition_is_not_the_programs_text_until_committed() {
    let (s, mut f, mut ts) = setup();
    key(&mut f, &s, &mut ts, 35, "", NONE);
    assert!(f.wants_ime(&s));
    f.ime_preedit(&s, &mut ts, "ka", Some((2, 2)));
    assert!(f.take_events().is_empty());
    assert_eq!(edit_model("edname").0, "Grüße, ñandú ✓");
    assert_eq!(f.node("edName").unwrap().ui.edit.as_ref().unwrap().text(), "Grüße, ñandú ✓ka");
    let list = f.paint(&s, &mut ts, 1.0);
    let item = list.items.iter().find_map(|i| if let Item::Text(t) = i { Some(t.clone()) } else { None }).unwrap();
    assert_eq!(item.underlines.len(), 1, "the composition underlined");
    let area = f.ime_area(&s, &mut ts).unwrap();
    let ed = f.node("edName").unwrap().abs;
    assert!(area.0 > ed.0 && area.0 < ed.0 + ed.2 && area.1 >= ed.1, "{area:?} in {ed:?}");
    f.ime_commit(&s, &mut ts, "か");
    let chain = vec!["edname".to_string(), "form".to_string()];
    assert_eq!(f.take_events(), vec![change("edname"), KernelEvent::KeyPress { chain, key: 63 }]);
    assert_eq!(edit_model("edname").0, "Grüße, ñandú ✓か");
    // an empty preedit ends a composition without text
    f.ime_preedit(&s, &mut ts, "x", None);
    f.ime_preedit(&s, &mut ts, "", None);
    assert_eq!(f.node("edName").unwrap().ui.edit.as_ref().unwrap().text(), "Grüße, ñandú ✓か");
    // not for other components
    f.focus_id(&s, "btnOK");
    assert!(!f.wants_ime(&s));
}

#[test]
fn read_only_max_length_and_char_case_apply_to_typing() {
    let (mut s, mut f, mut ts) = setup();
    s.set("edName", "text", v_str("ab")).set("edName", "maxlength", v_int(3)).set("edName", "charcase", v_int(1));
    drop(f.paint(&s, &mut ts, 1.0));
    key(&mut f, &s, &mut ts, 35, "", NONE);
    key(&mut f, &s, &mut ts, 67, "c", NONE);
    key(&mut f, &s, &mut ts, 68, "d", NONE);
    assert_eq!(edit_model("edname").0, "ABC", "CharCase made the text upper case");
    s.set("edName", "readonly", v_int(-1));
    assert!(clicks_and_changes(key(&mut f, &s, &mut ts, 8, "\u{8}", NONE)).is_empty());
    assert_eq!(edit_model("edname").0, "ABC", "CharCase made the text upper case");
}

// ------------------------------------------------------------ drawing --

#[test]
fn display_list_of_a_simple_form() {
    let mut s = MemStore::new();
    s.add("f", "RFORM", None).set("f", "clientwidth", v_int(200)).set("f", "clientheight", v_int(80)).set("f", "color", v_int(0xFFFFFF));
    s.add("l", "RLABEL", Some("f")).set("l", "caption", v_str("&Hello")).set("l", "left", v_int(8)).set("l", "top", v_int(8));
    s.add("b", "RBUTTON", Some("f")).set("b", "caption", v_str("Go")).set("b", "left", v_int(100)).set("b", "top", v_int(40));
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "f", false);
    let list = f.paint(&s, &mut ts, 1.0);
    assert_eq!((list.size, list.scale), ((200, 80), 1.0));
    let hello_w = text_size("H", &Font::default()).0;
    let expected = format!(
        "fill 0,0 200x80 #ffffff @0,0
clip 0,0 65x17 @8,8
text 0,0 65x17 \"Hello\" MS Sans Serif 11px #000000 topleft @8,8
line 0.5,12.5-{}.5,12.5 #000000 @8,8
unclip @8,8
clip 0,0 75x25 @100,40
fill 0,0 75x25 #f0f0f0 @100,40
edge 0,0 75x25 #646464 #646464 @100,40
edge 1,1 73x23 #ffffff/#e3e3e3 #696969/#a0a0a0 @100,40
text 0,0 75x25 \"Go\" MS Sans Serif 11px #000000 center @100,40
focus 4,4 67x17 @100,40
unclip @100,40
",
        hello_w - 1
    );
    assert_eq!(list.dump(), expected);
    // hovered: the hot face; disabled: grey text
    f.mouse_move(&s, &mut ts, 110.0, 50.0, NONE);
    assert!(f.paint(&s, &mut ts, 1.0).dump().contains("fill 0,0 75x25 #e5f1fb @100,40"));
    s.set("b", "enabled", v_int(0));
    let dump = f.paint(&s, &mut ts, 1.0).dump();
    assert!(dump.contains("\"Go\" MS Sans Serif 11px #a0a0a0"), "{dump}");
    assert!(!dump.contains("focus "), "a disabled button loses the focus");
}

#[test]
fn shared_models_draw_their_own_ops() {
    let (s, mut f, mut ts) = setup();
    let list = f.paint(&s, &mut ts, 1.0);
    let at = |origin: (i64, i64)| list.items.iter().filter(move |i| matches!(i, Item::Op { origin: o, .. } if *o == origin)).collect::<Vec<_>>();
    // the track bar (classic: Windows' own look from the model's numbers): its
    // sunken channel 8 pixels in, the thumb where the model puts it
    let drawn: Vec<Op> = at((8, 44)).into_iter().filter_map(|i| if let Item::Op { op, .. } = i { Some(op.clone()) } else { None }).collect();
    assert!(drawn.iter().any(|o| matches!(o, Op::Edge { rect: (8, 4, 134, 18), .. })), "{drawn:?}");
    let thumb_left = 8 + with_trackbar("tblevel", |t| t.position).unwrap() * (150 - 27) / 10;
    assert!(drawn.iter().any(|o| matches!(o, Op::Fill { rect: (x, 2, 10, 1), .. } if (x - thumb_left).abs() <= 1)), "{drawn:?}");
    // the tab control: its ops, converted unchanged
    let ops = with_tabcontrol("tcpages", |t| t.ops(150, 100, &Font::default(), 0xF0F0F0, true, false)).unwrap();
    let drawn: Vec<_> = at((164, 44)).into_iter().filter_map(|i| if let Item::Op { op, .. } = i { Some(op.clone()) } else { None }).filter(|o| !matches!(o, Op::ClipPush { .. } | Op::ClipPop)).collect();
    assert_eq!(drawn, rapidr_value::objects::ops::lift(ops));
    // the page's label after (over) the tab control, inside its clip
    let dump = list.dump();
    let tabs_at = dump.find("@164,44").unwrap();
    let page_at = dump.find("\"Page One\"").unwrap();
    assert!(page_at > tabs_at);
    // clips balance
    let pushes = list.items.iter().filter(|i| matches!(i, Item::Op { op: Op::ClipPush { .. }, .. })).count();
    let pops = list.items.iter().filter(|i| matches!(i, Item::Op { op: Op::ClipPop, .. })).count();
    assert_eq!(pushes, pops);
}

// ------------------------------------------------------ accessibility --

#[test]
fn accessibility_ids_names_values_and_actions() {
    let (mut s, mut f, mut ts) = setup();
    let tree = f.access_tree(&s, &mut ts);
    // stable ids: adding a component changes none of them
    let mut ids = Vec::new();
    tree.walk(&mut |n| ids.push(n.id));
    s.add("btnFirst", "RBUTTON", Some("form")).set("btnFirst", "top", v_int(120));
    f.rebuild(&s);
    let tree2 = f.access_tree(&s, &mut ts);
    for id in &ids {
        assert!(tree2.find(*id).is_some(), "{id}");
    }
    // the slider's numbers; the tab list's tabs where they're drawn
    let slider = tree.find(node_id("tblevel")).unwrap();
    let n = slider.numeric.unwrap();
    assert_eq!((n.value, n.min, n.max, n.step, n.jump), (3.0, 0.0, 10.0, 1.0, 2.0));
    assert_eq!(slider.bounds, (8, 44, 150, 45));
    let tabs = tree.find(node_id("tcpages")).unwrap();
    assert_eq!(tabs.children.iter().filter(|c| c.role == Role::Tab).count(), 3);
    let two = tree.find(part_id("tcpages", PART_TAB, 1)).unwrap();
    let r = with_tabcontrol("tcpages", |t| t.tab_rect(1, 150, 100, &Font::default())).unwrap().unwrap();
    assert_eq!(two.bounds, (164 + r.0, 44 + r.1, r.2, r.3));
    assert_eq!((two.name.as_str(), two.states.selected), ("Two", Some(false)));
    // the page's label is the tab control's child
    assert!(tabs.children.iter().any(|c| c.id == node_id("lblpage")));
    // AccessibleName first, then Caption, Hint, the nearest label
    s.set("tbLevel", "hint", v_str("Level"));
    s.set("btnOK", "accessiblename", v_str("Confirm")).set("btnOK", "accessibledescription", v_str("Saves the name"));
    let tree = f.access_tree(&s, &mut ts);
    assert_eq!(tree.find(node_id("tblevel")).unwrap().name, "Level");
    let ok = tree.find(node_id("btnok")).unwrap();
    assert_eq!((ok.name.as_str(), ok.description.as_str()), ("Confirm", "Saves the name"));
    // disabled; the window a dialog while modal
    s.set("btnOK", "enabled", v_int(0));
    f.modal = true;
    let tree = f.access_tree(&s, &mut ts);
    assert!(tree.find(node_id("btnok")).unwrap().states.disabled);
    assert_eq!(tree.role, Role::Dialog);
    let json = tree.to_json();
    assert!(json.starts_with("{\"id\":") && json.contains("\"role\":\"dialog\"") && json.contains("\"disabled\""), "{json}");
}

#[test]
fn screen_reader_requests_are_user_input() {
    let (s, mut f, mut ts) = setup();
    // a tab picked: as a click at it (OnChange)
    assert!(f.access_action(&s, &mut ts, part_id("tcpages", PART_TAB, 2), Action::Click, None));
    assert_eq!(f.take_events(), vec![change("tcpages")]);
    assert_eq!(with_tabcontrol("tcpages", |t| t.index), Some(2));
    // the slider stepped and set
    assert!(f.access_action(&s, &mut ts, node_id("tblevel"), Action::Increment, None));
    assert!(f.access_action(&s, &mut ts, node_id("tblevel"), Action::SetValue, Some(AccessValue::Number(9.0))));
    assert_eq!(f.take_events(), vec![change("tblevel"), change("tblevel")]);
    assert_eq!(with_trackbar("tblevel", |t| t.position), Some(9));
    // the edit's text set
    assert!(f.access_action(&s, &mut ts, node_id("edname"), Action::SetValue, Some(AccessValue::Text("Ann".into()))));
    assert_eq!(f.take_events(), vec![change("edname")]);
    assert_eq!(edit_model("edname"), ("Ann".to_string(), 0, 0, true));
    // focus moved; unknown targets refused
    assert!(f.access_action(&s, &mut ts, node_id("btnok"), Action::Focus, None));
    assert_eq!(focused(&f), "btnok");
    assert!(!f.access_action(&s, &mut ts, 12345, Action::Click, None));
    assert!(!f.access_action(&s, &mut ts, node_id("lblname"), Action::Focus, None));
}

#[test]
fn screen_readers_reach_list_rows_and_tree_items() {
    let mut s = MemStore::new();
    s.add("lform", "RFORM", None);
    s.add("lbox", "RLISTBOX", Some("lform")).set("lbox", "width", v_int(120)).set("lbox", "height", v_int(80));
    s.call("lbox", "additems", &[v_str("Red"), v_str("Green")]);
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "lform", false);
    drop(f.paint(&s, &mut ts, 1.0));
    // a row's click: the row picked, OnClick
    assert!(f.access_action(&s, &mut ts, part_id("lbox", PART_ITEM, 1), Action::Click, None));
    assert_eq!(rapidr_value::objects::with_list("lbox", |l| l.item_index), Some(1));
    assert!(f.take_events().contains(&KernelEvent::Click("lbox".into())));
}

/// The kinds answer what the shared tables say (the web reads the tables).
#[test]
fn kinds_agree_with_the_shared_rules() {
    use rapidr_value::objects::a11y::{mnemonic_clicks, mnemonic_of, role_of, takes_focus};
    let mut s = MemStore::new();
    s.add("kform", "RFORM", None);
    for (k, (name, kind)) in crate::components::KINDS.iter().enumerate() {
        let id = format!("k{}", name.to_lowercase());
        s.add(&id, name, Some("kform")).set(&id, "caption", v_str("&Go")).set(&id, "top", v_int(k as i64 * 30));
        assert_eq!(kind.focusable(&s, &id), takes_focus(name), "{name}: focusable");
        assert_eq!(kind.mnemonic_clicks(), mnemonic_clicks(name), "{name}: mnemonic_clicks");
        assert_eq!(kind.mnemonic(&s, &id), mnemonic_of(name, &|p| crate::Store::get(&s, &id, p)), "{name}: mnemonic");
    }
    // and each describes itself with its shared role
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "kform", false);
    let tree = f.access_tree(&s, &mut ts);
    for (name, _) in crate::components::KINDS {
        let n = tree.find(node_id(&format!("k{}", name.to_lowercase()))).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(n.role, role_of(name), "{name}: role");
    }
}

// --------------------------------------------------------------- text --

/// The one size rule and no kerning: parley lays a caption out exactly as
/// wide as `TextWidth` (`text::text_size`) measures it.
#[test]
fn text_measurement_matches_text_width() {
    let mut ts = TextSystem::new();
    let strings = ["Hello, World", "AVAWAY To Ty", "Grüße, ñandú", "office fi fl", "1234567890", "Wally's iiii MMMM", "OK"];
    for (name, size, styles) in [("Arial", 10, 0), ("Times New Roman", 12, 0), ("Courier New", 9, 0), ("MS Sans Serif", 8, 0), ("Arial", 14, 2), ("Arial", -16, 0)] {
        let font = Font { name: name.into(), size, color: 0, styles };
        for s in strings {
            let (w, _) = ts.measure(s, &font);
            let expected = text_size(s, &font).0;
            assert!((f64::from(w) - expected as f64).abs() <= 0.5 + 1e-3, "{name} {size}: {s:?} parley {w} vs TextWidth {expected}");
        }
    }
    // bold (synthesized from the regular faces): MS Sans Serif's a pixel
    // wider a character in both (letter spacing in the layout); the others
    // keep the advances, TextWidth adding GDI's 1-pixel overhang
    let bold = Font { styles: 1, ..Font::default() };
    let arial_bold = Font { name: "Arial".into(), size: 10, styles: 1, color: 0 };
    for s in strings {
        let (w, _) = ts.measure(s, &bold);
        assert!((f64::from(w) - text_size(s, &bold).0 as f64).abs() <= 0.5 + 1e-3, "bold {s:?}: {w}");
        let (w, _) = ts.measure(s, &arial_bold);
        assert!((f64::from(w) + 1.0 - text_size(s, &arial_bold).0 as f64).abs() <= 0.5 + 1e-3, "Arial bold {s:?}: {w}");
    }
    // the setting matters: kerned, "AVAWAY" is narrower
    let font = Font::default();
    let (unkerned, _) = ts.measure("AVAWAY", &font);
    let mut b = ts.layout_cx.ranged_builder(&mut ts.font_cx, "AVAWAY", 1.0, true);
    for p in crate::text::styles(&font, 0).into_iter().filter(|p| !matches!(p, parley::StyleProperty::FontFeatures(_))) {
        b.push_default(p);
    }
    let mut kerned = b.build("AVAWAY");
    kerned.break_all_lines(None);
    assert!(kerned.full_width() < unkerned - 1.0, "kerned {} vs {unkerned}", kerned.full_width());
    // and the font size is Windows' MulDiv: 8 pt (RapidQ's default) = 11 px
    assert_eq!(crate::text::font_pixels(&font), 11.0);
}

// ------------------------------------------------- double clicks --
// (the input lane's)

/// A form with a panel, a label, an image, a canvas and a list box in a
/// row, painted once.
fn clicks_form() -> (MemStore, FormUi, TextSystem) {
    let mut s = MemStore::new();
    s.add("f", "RFORM", None).set("f", "width", v_int(500)).set("f", "height", v_int(300));
    for (k, (id, t)) in [("pn", "RPANEL"), ("lb", "RLABEL"), ("img", "RIMAGE"), ("cv", "RCANVAS"), ("lst", "RLISTBOX")].into_iter().enumerate() {
        s.add(id, t, Some("f")).set(id, "left", v_int(10 + 80 * k as i64)).set(id, "top", v_int(10)).set(id, "width", v_int(70)).set(id, "height", v_int(60));
    }
    s.call("lst", "additems", &[v_str("a"), v_str("b")]);
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "f", false);
    drop(f.paint(&s, &mut ts, 1.0));
    (s, f, ts)
}

/// Presses and releases at (x, y), `n` times, a fresh click count first:
/// the events as `down:id`, `click:id`, `up:id`, `dbl:id`, `<event>:id`.
fn press_times(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, (x, y): (f64, f64), n: usize) -> Vec<String> {
    f.forget_clicks();
    for _ in 0..n {
        f.mouse_down(s, ts, x, y, Button::Left, NONE);
        f.mouse_up(s, ts, x, y, Button::Left, NONE);
    }
    f.take_events()
        .into_iter()
        .filter_map(|e| match e {
            KernelEvent::Click(id) => Some(format!("click:{id}")),
            KernelEvent::DblClick(id) => Some(format!("dbl:{id}")),
            KernelEvent::Mouse { id, kind: Mouse::Down, .. } => Some(format!("down:{id}")),
            KernelEvent::Mouse { id, kind: Mouse::Up, .. } => Some(format!("up:{id}")),
            KernelEvent::List(id, crate::components::list::ListAction::Fire(ev, _)) => Some(format!("{ev}:{id}")),
            _ => None,
        })
        .collect()
}

fn named(id: &str, events: &[&str]) -> Vec<String> {
    events.iter().map(|e| format!("{e}:{id}")).collect()
}

#[test]
fn double_clicks_come_in_the_vcl_order() {
    let (s, mut f, mut ts) = clicks_form();
    // (WM_LBUTTONDOWN: OnMouseDown; up: OnClick, OnMouseUp; the double
    // click's press: OnDblClick, OnMouseDown; its release: OnMouseUp)
    let vcl = ["down", "click", "up", "dbl", "down", "up"];
    assert_eq!(press_times(&mut f, &s, &mut ts, (20.0, 20.0), 2), named("pn", &vcl));
    assert_eq!(press_times(&mut f, &s, &mut ts, (100.0, 20.0), 2), named("lb", &vcl));
    assert_eq!(press_times(&mut f, &s, &mut ts, (180.0, 20.0), 2), named("img", &vcl));
    // (the form's open area too)
    assert_eq!(press_times(&mut f, &s, &mut ts, (200.0, 200.0), 2), named("f", &vcl));
    // (QCANVAS: no OnDblClick in RapidQ — every click is a click)
    assert_eq!(press_times(&mut f, &s, &mut ts, (260.0, 20.0), 2), named("cv", &["down", "click", "up", "down", "click", "up"]));
    // (a third press starts over as a single click, a fourth is a double)
    assert_eq!(press_times(&mut f, &s, &mut ts, (20.0, 20.0), 4), named("pn", &[&vcl[..], &vcl[..]].concat()));
    // (a list box: OnClick, then OnDblClick in its second's place)
    assert_eq!(press_times(&mut f, &s, &mut ts, (340.0, 14.0), 2), named("lst", &["click", "down", "up", "ondblclick", "down", "up"]));
}

#[test]
fn a_double_click_needs_windows_time_and_distance() {
    let (s, mut f, mut ts) = clicks_form();
    let t0 = std::time::Instant::now();
    crate::tick::set_test_now(Some(t0));
    assert_eq!(press_times_keep(&mut f, &s, &mut ts, (20.0, 20.0)), named("pn", &["down", "click", "up"]));
    // (more than 500 ms later: a click again, not a double)
    crate::tick::set_test_now(Some(t0 + crate::tick::DOUBLE_CLICK + std::time::Duration::from_millis(1)));
    assert_eq!(press_times_keep(&mut f, &s, &mut ts, (20.0, 20.0)), named("pn", &["down", "click", "up"]));
    // (5 pixels off: a click again)
    assert_eq!(press_times_keep(&mut f, &s, &mut ts, (25.0, 20.0)), named("pn", &["down", "click", "up"]));
    // (on the same spot soon after: the double)
    assert_eq!(press_times_keep(&mut f, &s, &mut ts, (24.0, 21.0)), named("pn", &["dbl", "down", "up"]));
    // (a release off what was pressed: no click; a release without a press: none)
    f.forget_clicks();
    f.mouse_down(&s, &mut ts, 20.0, 20.0, Button::Left, NONE);
    f.mouse_up(&s, &mut ts, 100.0, 200.0, Button::Left, NONE);
    f.mouse_up(&s, &mut ts, 20.0, 20.0, Button::Left, NONE);
    assert!(!f.take_events().iter().any(|e| matches!(e, KernelEvent::Click(_))));
    crate::tick::set_test_now(None);
}

/// One press and release at (x, y), the click count going on.
fn press_times_keep(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, (x, y): (f64, f64)) -> Vec<String> {
    f.mouse_down(s, ts, x, y, Button::Left, NONE);
    f.mouse_up(s, ts, x, y, Button::Left, NONE);
    f.take_events()
        .into_iter()
        .filter_map(|e| match e {
            KernelEvent::Click(id) => Some(format!("click:{id}")),
            KernelEvent::DblClick(id) => Some(format!("dbl:{id}")),
            KernelEvent::Mouse { id, kind: Mouse::Down, .. } => Some(format!("down:{id}")),
            KernelEvent::Mouse { id, kind: Mouse::Up, .. } => Some(format!("up:{id}")),
            _ => None,
        })
        .collect()
}

#[test]
fn a_double_click_on_an_mdi_childs_title_bar_maximizes_it() {
    use crate::components::form::Container;
    let mut s = MemStore::new();
    s.add("f", "RFORM", None).set("f", "width", v_int(500)).set("f", "height", v_int(300));
    s.add("frame", "RMDICHILD", Some("f")).set("frame", "width", v_int(200)).set("frame", "height", v_int(120));
    s.set("frame", "__form", v_str("f")).set("frame", "__component", v_str("ed"));
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "f", false);
    drop(f.paint(&s, &mut ts, 1.0));
    let actions = |f: &mut FormUi| -> Vec<rapidr_value::mdi::Action> {
        f.take_events().into_iter().filter_map(|e| if let KernelEvent::Container(Container::Mdi { action, .. }) = e { Some(action) } else { None }).collect()
    };
    for _ in 0..2 {
        f.mouse_down(&s, &mut ts, 30.0, 10.0, Button::Left, NONE);
        f.mouse_up(&s, &mut ts, 30.0, 10.0, Button::Left, NONE);
    }
    assert_eq!(actions(&mut f), vec![rapidr_value::mdi::Action::Activate, rapidr_value::mdi::Action::ToggleMaximize]);
    // (inside the frame, below the title bar: no)
    f.forget_clicks();
    for _ in 0..2 {
        f.mouse_down(&s, &mut ts, 30.0, 60.0, Button::Left, NONE);
        f.mouse_up(&s, &mut ts, 30.0, 60.0, Button::Left, NONE);
    }
    assert_eq!(actions(&mut f), vec![rapidr_value::mdi::Action::Activate; 2]);
}

/// (kernel themes) A form with the main components.
fn themed_store() -> MemStore {
    let mut s = MemStore::new();
    let put = |s: &mut MemStore, id: &str, t: &str, (l, top, w, h): (i64, i64, i64, i64)| {
        s.add(id, t, Some("tf")).set(id, "left", v_int(l)).set(id, "top", v_int(top)).set(id, "width", v_int(w)).set(id, "height", v_int(h));
    };
    s.add("tf", "RFORM", None).set("tf", "clientwidth", v_int(420)).set("tf", "clientheight", v_int(300));
    put(&mut s, "tlbl", "RLABEL", (8, 8, 100, 16));
    s.set("tlbl", "caption", v_str("Hello"));
    put(&mut s, "tred", "RLABEL", (8, 28, 100, 16));
    s.set("tred", "caption", v_str("Mine")).set("tred", "fontcolor", v_int(0x0000FF)).set("tred", "font.color", v_int(0x0000FF)).set("tred", "color", v_int(0xC0FFFF));
    put(&mut s, "ted", "REDIT", (8, 48, 120, 22));
    s.set("ted", "text", v_str("text"));
    put(&mut s, "tok", "RBUTTON", (140, 48, 75, 25));
    s.set("tok", "caption", v_str("OK")).set("tok", "default", v_int(1));
    put(&mut s, "tck", "RCHECKBOX", (8, 80, 100, 20));
    s.set("tck", "caption", v_str("Check")).set("tck", "checked", v_int(1));
    put(&mut s, "trd", "RRADIOBUTTON", (120, 80, 100, 20));
    s.set("trd", "caption", v_str("Radio")).set("trd", "checked", v_int(1));
    put(&mut s, "tlst", "RLISTBOX", (8, 104, 120, 60));
    s.call("tlst", "additems", &[v_str("a"), v_str("b"), v_str("c"), v_str("d"), v_str("e"), v_str("f")]);
    s.set("tlst", "itemindex", v_int(1));
    put(&mut s, "tcb", "RCOMBOBOX", (140, 104, 120, 22));
    s.call("tcb", "additems", &[v_str("one"), v_str("two")]);
    put(&mut s, "ttb", "RTRACKBAR", (140, 130, 150, 40));
    put(&mut s, "tpg", "RPROGRESS", (8, 170, 120, 16));
    s.set("tpg", "position", v_int(50));
    put(&mut s, "ttc", "RTABCONTROL", (300, 8, 110, 90));
    s.call("ttc", "addtabs", &[v_str("A"), v_str("B")]);
    put(&mut s, "tgb", "RGROUPBOX", (300, 110, 110, 60));
    s.set("tgb", "caption", v_str("Group"));
    put(&mut s, "tsb", "RSTATUSBAR", (0, 276, 420, 24));
    s.set("tsb", "simplepanel", v_int(1)).set("tsb", "simpletext", v_str("Ready"));
    s
}

/// The form painted under `t` (the thread's theme put back after).
fn themed_dump(s: &MemStore, t: &'static rapidr_value::theme::Theme) -> String {
    let was = rapidr_value::theme::current();
    rapidr_value::theme::set(t);
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(s, "tf", false);
    let dump = f.paint(s, &mut ts, 1.0).dump();
    rapidr_value::theme::set(was);
    dump
}

#[test]
fn every_theme_draws_the_main_components_in_its_own_colours() {
    use rapidr_value::theme::{ALL, CLASSIC};
    let s = themed_store();
    let dumps: Vec<String> = ALL.iter().map(|t| themed_dump(&s, t)).collect();
    for (t, dump) in ALL.iter().zip(&dumps) {
        let hex = |c: u32| format!("#{c:06x}");
        // (the form's background, the label's text: the theme's)
        assert!(dump.starts_with(&format!("fill 0,0 420x300 {}", hex(t.face))), "{}: {dump}", t.name);
        assert!(dump.lines().any(|l| l.contains("\"Hello\"") && l.contains(&hex(t.text))), "{}: {dump}", t.name);
        // (the program's colours win in every theme)
        assert!(dump.lines().any(|l| l.contains("\"Mine\"") && l.contains("#ff0000")), "{}: {dump}", t.name);
        assert!(dump.lines().any(|l| l.starts_with("fill 0,0 100x16 #ffffc0")), "{}: {dump}", t.name);
        // (the classic look is all bevels and pixels; a fluent one rounds)
        let smooth = dump.lines().filter(|l| l.starts_with("round ") || l.starts_with("stroke ")).count();
        if **t == CLASSIC {
            assert_eq!(smooth, 0, "{dump}");
        } else {
            assert!(smooth >= 8, "{}: {smooth} smooth shapes\n{dump}", t.name);
            assert!(dump.contains(&hex(t.accent)), "{}: the accent\n{dump}", t.name);
        }
    }
    // (four looks, not one under four names)
    for i in 0..dumps.len() {
        for j in i + 1..dumps.len() {
            assert_ne!(dumps[i], dumps[j], "{} = {}", ALL[i].name, ALL[j].name);
        }
    }
}

#[test]
fn a_theme_changes_no_geometry() {
    use rapidr_value::theme::ALL;
    // (where each component is drawn, clipped to it: the same in every
    // theme, as the mouse and the program find it)
    let s = themed_store();
    let clips = |dump: &str| dump.lines().filter(|l| l.starts_with("clip ")).map(str::to_string).collect::<Vec<_>>();
    let first = clips(&themed_dump(&s, ALL[0]));
    assert!(first.len() > 10);
    for t in &ALL[1..] {
        assert_eq!(clips(&themed_dump(&s, t)), first, "{}", t.name);
    }
}

#[test]
fn switching_the_theme_repaints_in_the_new_one() {
    use rapidr_value::theme::{self, CLASSIC, RAPIDR_DARK as DARK};
    let s = themed_store();
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "tf", false);
    let before = f.paint(&s, &mut ts, 1.0).dump();
    theme::set(&DARK);
    let after = f.paint(&s, &mut ts, 1.0).dump();
    theme::set(&CLASSIC);
    // (the edit's box: white, then the dark theme's window, rounded)
    assert!(before.contains("fill 0,0 120x22 #ffffff @8,48"), "{before}");
    assert!(after.contains(&format!("round 0,0 120x22 r4 fill #{:06x}", DARK.window)), "{after}");
}
