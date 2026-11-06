//! Document tabs keep their names (lane NOTIFY; docs/design-guidelines.md,
//! "Tabs"): never narrower than 112 pixels, a long name cut in its middle,
//! a strip that scrolls with arrows, the shown tab always in view, a list
//! of every open tab.

use super::geometry::{DocHit, MIN_DOC_TAB, TAB_BTN};
use super::manager::{Manager, User};
use super::*;
use crate::objects::font::Font;

/// An IDE with `n` tabbed documents `Document<i>.rr` (the first the shown).
fn many(n: usize) -> Manager {
    let mut m = Manager::default();
    m.resize((1280, 720), &Font::default());
    m.layout.mode = DocumentMode::Tabs;
    m.add_pane("Explorer", "Explorer", "left", "explorer");
    m.add_pane("Properties", "Properties", "right", "properties");
    for i in 0..n {
        m.add_pane(&format!("Doc{i}"), &format!("Document{i}.rr"), "documents", "code");
    }
    // (the first shown: the last added is active when added)
    m.layout.select("doc0");
    m.touch();
    m
}

#[test]
fn a_few_tabs_fit_with_their_names_and_the_list_button() {
    let mut m = many(4);
    let g = m.geometry().clone();
    let gr = &g.documents.as_ref().unwrap().groups[0];
    assert!(gr.arrows.is_none() && gr.max_scroll == 0 && gr.scroll == 0);
    assert!(gr.tabs.iter().all(|t| t.rect.2 >= MIN_DOC_TAB));
    // (the list at the strip's right end, 24 square; every tab left of it)
    let list = gr.list.expect("with two tabs or more");
    assert_eq!((list.2, list.3), (TAB_BTN, TAB_BTN));
    assert!(gr.tabs.iter().all(|t| t.rect.0 + t.rect.2 < list.0));
    // (one tab: nothing to list)
    let mut one = many(1);
    assert!(one.geometry().documents.as_ref().unwrap().groups[0].list.is_none());
}

#[test]
fn seventeen_tabs_scroll_and_none_is_narrower_than_112() {
    let mut m = many(17);
    let g = m.geometry().clone();
    let gr = &g.documents.as_ref().unwrap().groups[0];
    assert!(gr.tabs.iter().all(|t| t.rect.2 >= MIN_DOC_TAB), "{:?}", gr.tabs.iter().map(|t| t.rect.2).collect::<Vec<_>>());
    let (l, r) = gr.arrows.expect("the tabs don't fit: arrows");
    assert_eq!((l.2, l.3, r.2, r.3), (TAB_BTN, TAB_BTN, TAB_BTN, TAB_BTN));
    assert!(gr.max_scroll > 0);
    // (the viewport ends before the arrows, the arrows before the list)
    let list = gr.list.unwrap();
    assert!(gr.viewport.0 + gr.viewport.2 < l.0 && l.0 + l.2 <= r.0 && r.0 + r.2 <= list.0);
    // (a tab scrolled out of the viewport is not under the mouse there)
    let d = g.documents.as_ref().unwrap();
    assert!(!matches!(d.hit(gr.viewport.0 + gr.viewport.2 + 1, 10), Some(DocHit::Tab(..))));
}

#[test]
fn the_shown_tab_is_always_scrolled_into_view() {
    let mut m = many(17);
    // (the last document made active: its tab revealed, the strip scrolled to the end)
    m.user(User::Select("doc16".into()));
    let g = m.geometry().clone();
    let gr = &g.documents.as_ref().unwrap().groups[0];
    assert_eq!(gr.scroll, gr.max_scroll);
    let shown = &gr.tabs[16];
    assert!(shown.rect.0 >= gr.viewport.0 && shown.rect.0 + shown.rect.2 <= gr.viewport.0 + gr.viewport.2 + 1, "{shown:?} in {:?}", gr.viewport);
    // (back to the first: scrolled to the start)
    m.user(User::Select("doc0".into()));
    let g = m.geometry().clone();
    assert_eq!(g.documents.as_ref().unwrap().groups[0].scroll, 0);
}

#[test]
fn the_arrows_move_the_strip_a_tab_at_a_time_and_the_wheel_by_pixels() {
    let mut m = many(17);
    let g = m.geometry().clone();
    let gr = &g.documents.as_ref().unwrap().groups[0];
    let first_cut = gr.offsets.iter().find(|(l, w)| l + w > gr.viewport.2).copied().unwrap();
    m.user(User::DocScroll { group: 0, tabs: 1, pixels: 0 });
    let after = m.geometry().clone();
    let gr2 = &after.documents.as_ref().unwrap().groups[0];
    // (the first tab cut at the right now ends at the viewport's edge)
    assert_eq!(gr2.scroll, first_cut.0 + first_cut.1 - gr2.viewport.2);
    // (and back: the arrow left puts the last cut tab's left edge at the start)
    m.user(User::DocScroll { group: 0, tabs: -1, pixels: 0 });
    assert_eq!(m.geometry().documents.as_ref().unwrap().groups[0].scroll, 0);
    // (the wheel: pixels, never past either end)
    m.user(User::DocScroll { group: 0, tabs: 0, pixels: 100_000 });
    let end = m.geometry().documents.as_ref().unwrap().groups[0].clone();
    assert_eq!(end.scroll, end.max_scroll);
    m.user(User::DocScroll { group: 0, tabs: 0, pixels: -100_000 });
    assert_eq!(m.geometry().documents.as_ref().unwrap().groups[0].scroll, 0);
}

#[test]
fn the_list_button_tells_the_program_where_it_is() {
    let mut m = many(6);
    let out = m.user(User::DocList(0));
    assert_eq!(out.events.len(), 1);
    assert_eq!(out.events[0].name, "ondocumentlist");
    let g = m.geometry().clone();
    let d = g.documents.as_ref().unwrap();
    let list = d.groups[0].list.unwrap();
    assert_eq!(out.events[0].args, vec![crate::Value::Integer(d.rect.0 + list.0 + list.2), crate::Value::Integer(d.rect.1 + list.1 + list.3)]);
    // (a click on it is the list, a click on an arrow its own)
    assert_eq!(d.hit(list.0 + 3, list.1 + 3), Some(DocHit::List(0)));
}

#[test]
fn a_long_name_is_cut_in_the_middle_and_keeps_its_extension() {
    let f = Font::default();
    let cut = crate::panels::notify::middle("form_invoices_and_orders_2026.rr", &f, 60);
    assert!(cut.contains('…') && cut.ends_with(".rr") && cut.starts_with("form"), "{cut}");
    assert_eq!(crate::panels::notify::middle("main.rr", &f, 200), "main.rr");
}
