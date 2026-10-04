//! The text lane's editing behaviours, headless (a `MemStore` form, input
//! in; models, events and display lists out): QEDIT's PasswordChar,
//! Alignment, HideSelection, ReadOnly caret, clicks, context menu,
//! clipboard and undo; the caret's blink (the kernel's deadlines); QMEMO /
//! QRICHEDIT lines, keys, word wrap, scroll bars and the wheel; an editable
//! QCOMBOBOX's box; the wheel on lists.

use std::time::{Duration, Instant};

use rapidr_value::input::Button;
use rapidr_value::objects::{with_list, with_textedit, with_textedit_mut};
use rapidr_value::{v_int, v_str};

use crate::display::{DisplayList, Item, TextItem};
use crate::tick::{set_test_now, BLINK};
use crate::{FormUi, KernelEvent, MemClipboard, MemStore, Mods, TextSystem};

const NONE: Mods = Mods::NONE;
const SHIFT: Mods = Mods::SHIFT;
const CMD: Mods = Mods { shift: false, ctrl: false, alt: false, command: true, word: false };
/// Ctrl on Windows (the shortcut key and words).
const CTRL: Mods = Mods { shift: false, ctrl: true, alt: false, command: true, word: true };

fn form(add: impl FnOnce(&mut MemStore)) -> (MemStore, FormUi, TextSystem) {
    let mut s = MemStore::new();
    s.add("tf", "RFORM", None).set("tf", "clientwidth", v_int(400)).set("tf", "clientheight", v_int(300));
    add(&mut s);
    let mut ts = TextSystem::new();
    let mut f = FormUi::build(&s, "tf", false);
    drop(f.paint(&s, &mut ts, 1.0));
    (s, f, ts)
}

fn model(id: &str) -> (String, usize, usize) {
    with_textedit(id, |t| (t.text(), t.sel_start, t.sel_len)).unwrap()
}

fn key(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, clip: &mut MemClipboard, vk: i64, text: &str, mods: Mods) -> Vec<KernelEvent> {
    f.key_down(s, ts, vk, text, mods, clip);
    f.key_up(vk, mods);
    f.take_events()
}

fn typed(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, clip: &mut MemClipboard, text: &str) {
    for c in text.chars() {
        let vk = if c == '\n' { 13 } else { rapidr_value::input::vk_of_char(c).unwrap_or(0) };
        key(f, s, ts, clip, vk, if c == '\n' { "" } else { &text[text.find(c).unwrap()..][..c.len_utf8()] }, NONE);
    }
}

fn texts(list: &DisplayList, node: &str) -> Vec<TextItem> {
    list.items.iter().filter_map(|i| if let Item::Text(t) = i { (t.node == node).then(|| t.clone()) } else { None }).collect()
}

/// A click (press and release) at (x, y) of the client area.
fn click_at(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, x: f64, y: f64, button: Button) {
    f.mouse_down(s, ts, x, y, button, NONE);
    f.mouse_up(s, ts, x, y, button, NONE);
}

fn edit_form(text: &str) -> (MemStore, FormUi, TextSystem) {
    form(|s| {
        s.add("te", "REDIT", Some("tf")).set("te", "text", v_str(text)).set("te", "left", v_int(10)).set("te", "top", v_int(10)).set("te", "width", v_int(150));
        s.add("tb", "RBUTTON", Some("tf")).set("tb", "left", v_int(10)).set("tb", "top", v_int(200));
    })
}

#[test]
fn a_password_edit_shows_its_character_and_keeps_the_text() {
    let (mut s, mut f, mut ts) = edit_form("");
    s.set("te", "passwordchar", v_str("*"));
    let mut clip = MemClipboard::default();
    f.focus_id(&s, "te");
    typed(&mut f, &s, &mut ts, &mut clip, "pw1");
    assert_eq!(model("te").0, "pw1");
    drop(f.paint(&s, &mut ts, 1.0));
    let e = f.node("te").unwrap().ui.edit.as_ref().unwrap();
    let shown: String = e.layout().unwrap().lines().flat_map(|l| l.runs().collect::<Vec<_>>()).map(|r| r.text_range().len()).sum::<usize>().to_string();
    assert_eq!(shown, "3", "three characters laid out");
    assert_eq!(e.text(), "pw1", "the editor keeps the text");
    // no copying a password out; no input methods
    key(&mut f, &s, &mut ts, &mut clip, 65, "", CMD);
    key(&mut f, &s, &mut ts, &mut clip, 67, "", CMD);
    assert_eq!(clip.0, None);
    assert!(!f.wants_ime(&s));
    // the screen reader hears the mask
    let tree = f.access_tree(&s, &mut ts);
    let node = tree.children.iter().find(|n| n.value.as_deref() == Some("***"));
    assert!(node.is_some(), "{tree:?}");
}

#[test]
fn alignment_hide_selection_and_the_read_only_caret() {
    let (mut s, mut f, mut ts) = edit_form("ab");
    s.set("te", "alignment", v_int(1));
    f.focus_id(&s, "tb");
    with_textedit_mut("te", |t| {
        t.set("selstart", &v_int(0));
        t.set("sellength", &v_int(1));
    });
    let list = f.paint(&s, &mut ts, 1.0);
    let item = &texts(&list, "te")[0];
    assert!(item.origin.0 > 10.0 + 100.0, "right-aligned: {:?}", item.origin);
    assert!(item.selection.is_empty(), "HideSelection: not focused, no selection shown");
    s.set("te", "hideselection", v_int(0));
    let list = f.paint(&s, &mut ts, 1.0);
    assert_eq!(texts(&list, "te")[0].selection.len(), 1, "HideSelection False: shown");
    // ReadOnly: the caret shows and moves, nothing types
    s.set("te", "readonly", v_int(-1));
    f.focus_id(&s, "te");
    let mut clip = MemClipboard::default();
    let ev = key(&mut f, &s, &mut ts, &mut clip, 35, "", NONE);
    assert!(!ev.contains(&KernelEvent::Change("te".into())));
    assert_eq!(model("te"), ("ab".into(), 2, 0));
    key(&mut f, &s, &mut ts, &mut clip, 88, "x", NONE);
    assert_eq!(model("te").0, "ab");
    let list = f.paint(&s, &mut ts, 1.0);
    assert!(texts(&list, "te")[0].caret.is_some(), "a read-only edit's caret");
}

#[test]
fn double_click_selects_a_word_triple_click_everything() {
    let (s, mut f, mut ts) = edit_form("alpha beta gamma");
    let t0 = Instant::now();
    set_test_now(Some(t0));
    let y = 20.0;
    // (in "beta": the text starts at 10 + 3)
    let x = 13.0 + rapidr_value::objects::text::text_size("alpha be", &rapidr_value::objects::font::Font::default()).0 as f64;
    click_at(&mut f, &s, &mut ts, x, y, Button::Left);
    click_at(&mut f, &s, &mut ts, x + 1.0, y, Button::Left);
    let (_, start, len) = model("te");
    assert_eq!((start, len), (6, 4), "the word under the mouse (\"beta\")");
    click_at(&mut f, &s, &mut ts, x + 1.0, y, Button::Left);
    assert_eq!(model("te").2, 16, "a third click: everything");
    // too late for a double click: a plain click
    set_test_now(Some(t0 + Duration::from_secs(2)));
    click_at(&mut f, &s, &mut ts, x, y, Button::Left);
    click_at(&mut f, &s, &mut ts, x, y, Button::Left);
    assert_eq!(model("te").2, 4, "again a double click");
    set_test_now(Some(t0 + Duration::from_secs(4)));
    click_at(&mut f, &s, &mut ts, x, y, Button::Left);
    assert_eq!(model("te").2, 0);
    set_test_now(None);
}

#[test]
fn the_context_menu_cuts_copies_pastes_and_undoes() {
    let (s, mut f, mut ts) = edit_form("hello world");
    let mut clip = MemClipboard::default();
    f.focus_id(&s, "te");
    // (select "world", then a right click on the edit: the kernel's menu)
    with_textedit_mut("te", |t| {
        t.set("selstart", &v_int(6));
        t.set("sellength", &v_int(5));
    });
    drop(f.paint(&s, &mut ts, 1.0));
    f.mouse_down(&s, &mut ts, 40.0, 20.0, Button::Right, NONE);
    f.mouse_up(&s, &mut ts, 40.0, 20.0, Button::Right, NONE);
    f.edit_commands(&s, &mut ts, &mut clip);
    assert_eq!(f.popup_open(), Some(crate::components::edit::EDIT_MENU));
    let events = f.take_events();
    assert!(events.iter().all(|e| matches!(e, KernelEvent::Mouse { .. })), "{events:?}");
    // Cut: the third row (Undo, a separator, Cut …)
    let panel = f.menus.panels[0].rect;
    let row = |k: usize| {
        let rows = crate::components::menubar::rows(&crate::components::menubar::items(crate::components::edit::EDIT_MENU));
        (panel.0 as f64 + 20.0, (panel.1 + rows[k].0 + rows[k].1 / 2) as f64)
    };
    let (x, y) = row(2);
    click_at(&mut f, &s, &mut ts, x, y, Button::Left);
    f.edit_commands(&s, &mut ts, &mut clip);
    assert_eq!(clip.0.as_deref(), Some("world"));
    assert_eq!(model("te").0, "hello ");
    assert!(f.take_events().contains(&KernelEvent::Change("te".into())), "a cut is the user's change");
    assert_eq!(f.popup_open(), None);
    // Paste (Ctrl+V) twice, then Ctrl+Z takes the last paste back
    key(&mut f, &s, &mut ts, &mut clip, 86, "", CMD);
    key(&mut f, &s, &mut ts, &mut clip, 86, "", CMD);
    assert_eq!(model("te").0, "hello worldworld");
    key(&mut f, &s, &mut ts, &mut clip, 90, "", CMD);
    assert_eq!(model("te").0, "hello world");
    key(&mut f, &s, &mut ts, &mut clip, 90, "", CMD);
    assert_eq!(model("te").0, "hello worldworld", "an undo undoes itself");
    // the program's PopupMenu wins: no kernel menu
    let mut s2 = s;
    s2.set("te", "popupmenu", v_str("pm"));
    f.mouse_down(&s2, &mut ts, 40.0, 20.0, Button::Right, NONE);
    f.mouse_up(&s2, &mut ts, 40.0, 20.0, Button::Right, NONE);
    assert_eq!(f.popup_open(), None);
    // the menu key opens it at the caret
    s2.set("te", "popupmenu", v_str(""));
    key(&mut f, &s2, &mut ts, &mut clip, 93, "", NONE);
    assert_eq!(f.popup_open(), Some(crate::components::edit::EDIT_MENU));
}

#[test]
fn the_caret_blinks_on_the_kernels_deadline() {
    let (s, mut f, mut ts) = edit_form("x");
    assert!(f.next_wake().is_some(), "the focused edit's blink armed when first drawn");
    let t0 = Instant::now();
    set_test_now(Some(t0));
    f.focus_id(&s, "te");
    assert_eq!(f.next_wake(), Some(t0 + BLINK));
    f.tick(&s, &mut ts, t0 + BLINK);
    assert!(!f.caret_on, "blinked off");
    assert_eq!(f.next_wake(), Some(t0 + BLINK + BLINK));
    let list = f.paint(&s, &mut ts, 1.0);
    assert!(texts(&list, "te")[0].caret.is_none());
    // a key shows it again, the blink starting over
    let mut clip = MemClipboard::default();
    set_test_now(Some(t0 + Duration::from_millis(600)));
    key(&mut f, &s, &mut ts, &mut clip, 36, "", NONE);
    assert!(f.caret_on);
    assert_eq!(f.next_wake(), Some(t0 + Duration::from_millis(600) + BLINK));
    // nothing blinks without an editor focused, nor when blinking is off
    f.focus_id(&s, "tb");
    assert_eq!(f.next_wake(), None);
    f.focus_id(&s, "te");
    f.blinks = false;
    f.reset_caret();
    assert_eq!(f.next_wake(), None);
    set_test_now(None);
}

fn memo_form(type_name: &str) -> (MemStore, FormUi, TextSystem) {
    form(|s| {
        s.add("tm", type_name, Some("tf")).set("tm", "left", v_int(10)).set("tm", "top", v_int(10)).set("tm", "width", v_int(150)).set("tm", "height", v_int(80));
        s.add("tb", "RBUTTON", Some("tf")).set("tb", "left", v_int(10)).set("tb", "top", v_int(200)).set("tb", "default", v_int(-1));
    })
}

#[test]
fn a_memo_types_lines_and_the_model_counts_them() {
    for type_name in ["RMEMO", "RRICHEDIT"] {
        let (s, mut f, mut ts) = memo_form(type_name);
        let mut clip = MemClipboard::default();
        f.focus_id(&s, "tm");
        typed(&mut f, &s, &mut ts, &mut clip, "ab\ncd");
        assert_eq!(model("tm").0, "ab\r\ncd", "Enter breaks the line (no default button click)");
        let get = |p: &str| with_textedit("tm", |t| t.get(p).unwrap().to_i64()).unwrap();
        assert_eq!((get("linecount"), get("wherey"), get("wherex")), (2, 1, 2));
        // Up keeps the column; Home / Ctrl+End; Shift+Up selects across the break
        key(&mut f, &s, &mut ts, &mut clip, 38, "", NONE);
        assert_eq!((get("wherey"), get("wherex")), (0, 2));
        key(&mut f, &s, &mut ts, &mut clip, 35, "", CTRL);
        key(&mut f, &s, &mut ts, &mut clip, 38, "", SHIFT);
        assert_eq!(model("tm").2, 3, "\"\\ncd\" — a break counts one");
        // Backspace at a line's start joins the lines
        key(&mut f, &s, &mut ts, &mut clip, 39, "", NONE);
        key(&mut f, &s, &mut ts, &mut clip, 36, "", NONE);
        let ev = key(&mut f, &s, &mut ts, &mut clip, 8, "", NONE);
        assert_eq!(model("tm").0, "abcd");
        assert!(ev.contains(&KernelEvent::Change("tm".into())));
        assert!(!ev.contains(&KernelEvent::Click("tb".into())));
        // drawn: one text item per paragraph shown, the multi-line node
        with_textedit_mut("tm", |t| t.set("text", &v_str("1\r\n2\r\n3")));
        let list = f.paint(&s, &mut ts, 1.0);
        let items = texts(&list, "tm");
        assert_eq!(items.iter().map(|t| t.para).collect::<Vec<_>>(), vec![0, 1, 2]);
        assert!(items[1].origin.1 > items[0].origin.1);
        assert!(f.editor_layout_at("tm", 2).is_some());
        let tree = f.access_tree(&s, &mut ts);
        assert!(tree.children.iter().any(|n| n.states.multiline), "multi-line text input");
    }
}

#[test]
fn a_memo_wraps_scrolls_and_shows_only_what_fits() {
    let (mut s, mut f, mut ts) = memo_form("RMEMO");
    let lines: Vec<String> = (0..40).map(|i| format!("line {i}")).collect();
    with_textedit_mut("tm", |t| {
        t.call("addstrings", &lines.iter().map(|l| v_str(l)).collect::<Vec<_>>());
    });
    s.set("tm", "scrollbars", v_int(2));
    let list = f.paint(&s, &mut ts, 1.0);
    let shown = texts(&list, "tm").len();
    assert!(shown < 10, "only the paragraphs in view are drawn ({shown})");
    let e = f.node("tm").unwrap().ui.edit.as_ref().unwrap();
    assert!(e.bars.vert.shown, "the vertical bar: the text doesn't fit");
    assert!(!e.bars.horz.shown);
    // the wheel scrolls it (three lines a notch)
    f.mouse_wheel(&s, &mut ts, (50.0, 40.0), (0.0, 1.0), NONE);
    let e = f.node("tm").unwrap().ui.edit.as_ref().unwrap();
    let (_, sy) = e.scroll();
    assert!(sy > 30.0, "{sy}");
    assert_eq!(e.bars.vert.position as f64, sy.round());
    let list = f.paint(&s, &mut ts, 1.0);
    assert!(texts(&list, "tm")[0].para >= 3);
    // a press on the bar's down arrow scrolls, repeats on the tick, and doesn't focus
    let t0 = Instant::now();
    set_test_now(Some(t0));
    let (bx, by) = (10.0 + 2.0 + 148.0 - 4.0 - 8.0, 10.0 + 2.0 + 76.0 - 8.0);
    f.focus_id(&s, "tb");
    f.mouse_down(&s, &mut ts, bx, by, Button::Left, NONE);
    let after_press = f.node("tm").unwrap().ui.edit.as_ref().unwrap().scroll().1;
    assert!(after_press > sy);
    assert_ne!(f.focused(), Some("tm"));
    let wake = f.next_wake().unwrap();
    f.tick(&s, &mut ts, wake);
    let after_tick = f.node("tm").unwrap().ui.edit.as_ref().unwrap().scroll().1;
    assert!(after_tick > after_press, "the held arrow repeated");
    f.mouse_up(&s, &mut ts, bx, by, Button::Left, NONE);
    assert_eq!(f.nodes[f.index_of("tm").unwrap()].ui.wake, None);
    set_test_now(None);
    // WordWrap off: one line a paragraph, and a horizontal bar when asked
    with_textedit_mut("tm", |t| t.set("text", &v_str("a very long line that does not fit in a memo this narrow at all")));
    s.set("tm", "scrollbars", v_int(3)).set("tm", "wordwrap", v_int(0));
    drop(f.paint(&s, &mut ts, 1.0));
    let e = f.node("tm").unwrap().ui.edit.as_ref().unwrap();
    assert_eq!(e.para_layout(0).unwrap().len(), 1);
    assert!(e.bars.horz.shown);
    s.set("tm", "wordwrap", v_int(-1));
    drop(f.paint(&s, &mut ts, 1.0));
    let e = f.node("tm").unwrap().ui.edit.as_ref().unwrap();
    assert!(e.para_layout(0).unwrap().len() > 2, "WordWrap: broken at the view");
    assert!(!e.bars.horz.shown, "no horizontal bar with WordWrap");
}

#[test]
fn an_editable_combo_box_types_into_its_text() {
    let (s, mut f, mut ts) = form(|s| {
        s.add("tc", "RCOMBOBOX", Some("tf")).set("tc", "left", v_int(10)).set("tc", "top", v_int(10));
        s.call("tc", "additems", &[v_str("red"), v_str("green")]);
    });
    let mut clip = MemClipboard::default();
    f.focus_id(&s, "tc");
    let ev = key(&mut f, &s, &mut ts, &mut clip, 66, "b", NONE);
    assert_eq!(with_list("tc", |l| l.text.clone()).unwrap(), "b");
    assert!(ev.contains(&KernelEvent::Change("tc".into())));
    typed(&mut f, &s, &mut ts, &mut clip, "lue");
    assert_eq!(with_list("tc", |l| l.text.clone()).unwrap(), "blue");
    // Down still picks the next item (the box shows it)
    key(&mut f, &s, &mut ts, &mut clip, 40, "", NONE);
    assert_eq!(with_list("tc", |l| (l.item_index, l.text.clone())).unwrap(), (0, "red".into()));
    let list = f.paint(&s, &mut ts, 1.0);
    assert_eq!(f.node("tc").unwrap().ui.edit.as_ref().unwrap().text(), "red");
    assert_eq!(texts(&list, "tc").len(), 1);
    assert!(f.wants_ime(&s));
}

#[test]
fn the_wheel_scrolls_the_list_under_the_mouse() {
    let (s, mut f, mut ts) = form(|s| {
        s.add("tl", "RLISTBOX", Some("tf")).set("tl", "left", v_int(10)).set("tl", "top", v_int(10)).set("tl", "height", v_int(60));
        s.call("tl", "additems", &(0..30).map(|i| v_str(&format!("item {i}"))).collect::<Vec<_>>());
    });
    drop(f.paint(&s, &mut ts, 1.0));
    f.mouse_wheel(&s, &mut ts, (30.0, 30.0), (0.0, 1.0), NONE);
    let (pos, shown, _) = crate::components::list::vscroll_state("tl");
    assert!(shown && pos > 0, "{pos}");
    // half notches add up
    f.mouse_wheel(&s, &mut ts, (30.0, 30.0), (0.0, 0.5), NONE);
    assert_eq!(crate::components::list::vscroll_state("tl").0, pos);
    f.mouse_wheel(&s, &mut ts, (30.0, 30.0), (0.0, 0.5), NONE);
    assert!(crate::components::list::vscroll_state("tl").0 > pos);
}
