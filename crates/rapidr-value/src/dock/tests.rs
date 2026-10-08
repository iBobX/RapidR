//! The dock layout model: every operation, the text round trip, the
//! geometry, the compass, the manager's methods and the keyboard's move.

use super::geometry::{self, Button, Guide, Hit, Slot, GAP, HEADER, STRIP};
use super::manager::{parse_where, Manager, User};
use super::*;
use crate::objects::font::Font;

fn no_extent(_: &[usize], _: Axis) -> i64 {
    600
}

fn tabs(panes: &[&str], active: usize) -> Node {
    Node::Tabs { panes: panes.iter().map(|s| s.to_string()).collect(), active }
}

/// An IDE's layout: explorer | (documents / output) | properties.
fn ide() -> Manager {
    let mut m = Manager::default();
    m.resize((1000, 700), &Font::default());
    assert_eq!(m.add_pane("Explorer", "Explorer", "left", "explorer").value, Some(crate::Value::Integer(-1)));
    m.add_pane("Properties", "Properties", "right", "properties");
    m.add_pane("Output", "Output", "bottom:documents", "output");
    m.add_pane("Problems", "Problems", "tab:output", "problems");
    m.add_pane("Form1", "Form1.rr", "documents", "form");
    m.add_pane("Module1", "Module1.rr", "documents", "code");
    m
}

#[test]
fn edges_wrap_the_root_and_reuse_a_split_on_the_same_axis() {
    let mut l = Layout::default();
    assert!(l.insert("a", &Target::Edge(Side::Left), 200, &no_extent));
    assert!(l.insert("b", &Target::Edge(Side::Right), 0, &no_extent));
    assert_eq!(l.root, Node::Split { axis: Axis::Row, children: vec![tabs(&["a"], 0), Node::Documents, tabs(&["b"], 0)], sizes: vec![200, 0, DEFAULT_SIDE] });
    assert!(l.insert("c", &Target::Edge(Side::Bottom), 0, &no_extent));
    let Node::Split { axis: Axis::Column, children, sizes } = &l.root else { panic!("{:?}", l.root) };
    assert_eq!((children.len(), sizes[1]), (2, DEFAULT_BOTTOM));
    // (a pane already placed isn't placed twice)
    assert!(!l.insert("a", &Target::Edge(Side::Top), 0, &no_extent));
}

#[test]
fn beside_splits_a_group_or_joins_its_split_and_into_tabs() {
    let mut l = Layout::default();
    l.insert("a", &Target::Edge(Side::Left), 300, &no_extent);
    // (below a: a's group becomes a column of two)
    assert!(l.insert("b", &Target::Beside(Anchor::Pane("a".into()), Side::Bottom), 0, &|_, _| 600));
    let Node::Split { children, .. } = &l.root else { panic!() };
    assert_eq!(children[0], Node::Split { axis: Axis::Column, children: vec![tabs(&["a"], 0), tabs(&["b"], 0)], sizes: vec![600 - DEFAULT_BOTTOM - GAP, DEFAULT_BOTTOM] });
    // (right of the documents: a sibling in the root row)
    assert!(l.insert("c", &Target::Beside(Anchor::Documents, Side::Right), 0, &no_extent));
    let Node::Split { children, .. } = &l.root else { panic!() };
    assert_eq!(children.len(), 3);
    assert_eq!(children[2], tabs(&["c"], 0));
    // (into a's group as a tab: it's shown)
    assert!(l.insert("d", &Target::Into(Anchor::Pane("a".into())), 0, &no_extent));
    assert_eq!(l.find("d"), Some(Where::Docked(vec![0, 0], 1)));
    assert!(l.is_active("d") && !l.is_active("a"));
    // (beside a missing pane: nothing)
    assert!(!l.insert("e", &Target::Beside(Anchor::Pane("zz".into()), Side::Left), 0, &no_extent));
}

#[test]
fn remove_remembers_the_place_and_normalizes() {
    let mut l = Layout::default();
    l.insert("a", &Target::Edge(Side::Left), 0, &no_extent);
    l.insert("b", &Target::Into(Anchor::Pane("a".into())), 0, &no_extent);
    l.insert("c", &Target::Edge(Side::Bottom), 150, &no_extent);
    // (from a group of two: back as its tab)
    assert_eq!(l.remove("b", &no_extent), Some(Place::Tab("a".into(), 1)));
    // (a lone group at the top level: its edge, its extent)
    let at_bottom = l.remove("c", &|_, _| 150);
    assert_eq!(at_bottom, Some(Place::Edge(Side::Bottom, 150)));
    // (the split of one child is gone)
    assert_eq!(l.root, Node::Split { axis: Axis::Row, children: vec![tabs(&["a"], 0), Node::Documents], sizes: vec![DEFAULT_SIDE, 0] });
    assert_eq!(l.remove("a", &|_, _| 240), Some(Place::Beside(Anchor::Documents, Side::Left, 240)));
    assert_eq!(l.root, Node::Documents);
    assert_eq!(l.remove("a", &no_extent), None);
    // (the places lead back)
    let docked = |_: &str| true;
    assert_eq!(Place::Tab("a".into(), 0).target(&docked), Some((Target::Into(Anchor::Pane("a".into())), 0)));
    assert_eq!(Place::Tab("a".into(), 0).target(&|_| false), None);
}

#[test]
fn removing_the_shown_tab_shows_its_neighbour() {
    let mut l = Layout { root: tabs(&["a", "b", "c"], 2), ..Default::default() };
    l.remove("c", &no_extent);
    assert_eq!(l.root, tabs(&["a", "b"], 1));
    let mut l = Layout { root: tabs(&["a", "b", "c"], 1), ..Default::default() };
    l.remove("b", &no_extent);
    assert_eq!(l.root, tabs(&["a", "c"], 1));
}

#[test]
fn autohide_float_and_documents_are_places_too() {
    let mut l = Layout::default();
    l.insert("a", &Target::AutoHide(Side::Right), 0, &no_extent);
    l.insert("b", &Target::Float((10, 20, 300, 200)), 0, &no_extent);
    l.insert("c", &Target::Into(Anchor::Pane("b".into())), 0, &no_extent);
    l.insert("d", &Target::Into(Anchor::Documents), 0, &no_extent);
    l.insert("e", &Target::Into(Anchor::Documents), 0, &no_extent);
    assert_eq!(l.find("a"), Some(Where::AutoHide(Side::Right, 0)));
    assert_eq!(l.find("c"), Some(Where::Floating(0, 1)));
    assert_eq!((l.find("e"), l.active_document), (Some(Where::Document(1)), Some(1)));
    assert_eq!(l.remove("a", &no_extent), Some(Place::AutoHide(Side::Right)));
    assert_eq!(l.remove("b", &no_extent), Some(Place::Float((10, 20, 300, 200))));
    assert_eq!(l.floating[0].panes, vec!["c".to_string()]);
    assert_eq!(l.remove("c", &no_extent), Some(Place::Float((10, 20, 300, 200))));
    assert!(l.floating.is_empty());
    assert_eq!(l.remove("e", &no_extent), Some(Place::Document));
    assert_eq!(l.active_document, Some(0));
}

#[test]
fn the_text_round_trips() {
    let m = ide();
    let mut m = m;
    m.auto_hide("problems", true);
    m.hide_pane("properties");
    let text = m.save();
    assert!(text.starts_with("rapidr-dock 1\nmode mdi\nroot split row\n"), "{text}");
    assert!(text.contains("autohide bottom problems") && text.contains("hidden properties") && text.contains("documents 1 form1 module1"), "{text}");
    let known = |p: &str| ["explorer", "properties", "output", "problems", "form1", "module1"].contains(&p);
    let (back, hidden) = Layout::load(&text, &known).unwrap();
    assert_eq!(back, m.layout);
    assert_eq!(hidden, vec!["properties".to_string()]);
    assert_eq!(back.save(&hidden), text);
    // (unknown panes are dropped; a group left empty goes)
    let (some, _) = Layout::load(&text, &|p| p == "explorer").unwrap();
    assert_eq!(some.root, Node::Split { axis: Axis::Row, children: vec![tabs(&["explorer"], 0), Node::Documents], sizes: vec![DEFAULT_SIDE, 0] });
    assert!(Layout::load("hello", &known).is_err());
    assert!(Layout::load("rapidr-dock 1\nroot split diagonal\n", &known).is_err());
}

#[test]
fn extents_give_the_flex_child_the_rest_and_shrink_alike() {
    let ch = vec![tabs(&["a"], 0), Node::Documents, tabs(&["b"], 0)];
    assert_eq!(geometry::split_extents(&ch, &[200, 0, 300], 1000), vec![200, 500, 300]);
    let e = geometry::split_extents(&ch, &[200, 0, 300], 300);
    assert_eq!(e.iter().sum::<i64>(), 300);
    assert!(e[1] >= 60, "{e:?}");
    // (no documents: the last takes the rest)
    let ch = vec![tabs(&["a"], 0), tabs(&["b"], 0)];
    assert_eq!(geometry::split_extents(&ch, &[100, 999], 400), vec![100, 300]);
}

#[test]
fn geometry_places_groups_splitters_strips_and_documents() {
    let mut m = ide();
    let g = m.geometry().clone();
    assert_eq!(g.inner, (0, 0, 1000, 700));
    assert_eq!(g.groups.len(), 3);
    let explorer = g.group_of("explorer").unwrap();
    assert_eq!(explorer.rect, (0, 0, DEFAULT_SIDE, 700));
    assert_eq!(explorer.content, (0, HEADER - 1, DEFAULT_SIDE, 700 - HEADER + 1));
    assert!(explorer.single() && explorer.buttons.iter().map(|b| b.0).eq([Button::Pin, Button::Close]));
    let output = g.group_of("output").unwrap();
    assert_eq!((output.tabs.len(), output.active), (2, 1));
    let d = g.documents.as_ref().unwrap();
    assert_eq!(d.rect, (DEFAULT_SIDE + GAP, 0, 1000 - 2 * DEFAULT_SIDE - 2 * GAP, 700 - DEFAULT_BOTTOM - GAP));
    assert!(d.tabs.is_empty(), "MDI: no tabs");
    // (a splitter between explorer and the middle, hit a pixel either side)
    assert_eq!(g.hit(DEFAULT_SIDE - 1, 300), Some(Hit::Splitter(0)));
    assert_eq!(g.hit(DEFAULT_SIDE + 2, 300), Some(Hit::Splitter(0)));
    assert_eq!(g.hit(500, 300), None);
    // (auto-hidden: a strip on its edge, the layout inside it)
    m.auto_hide("explorer", true);
    let g = m.geometry().clone();
    assert_eq!(g.inner, (STRIP, 0, 1000 - STRIP, 700));
    assert_eq!(g.strip_tabs[0].pane, "explorer");
    assert_eq!(g.hit(g.strip_tabs[0].rect.0 + 2, g.strip_tabs[0].rect.1 + 2), Some(Hit::Strip("explorer".into())));
    // (its flyout over the layout, its extent)
    m.user(User::Flyout(Some("explorer".into())));
    let g = m.geometry().clone();
    assert_eq!(g.flyout.as_ref().map(|f| f.rect), Some((STRIP, 0, DEFAULT_SIDE, 700)));
    // (tabbed documents: a strip of tabs with close buttons)
    m.layout.mode = DocumentMode::Tabs;
    m.touch();
    let g = m.geometry().clone();
    let d = g.documents.as_ref().unwrap();
    assert_eq!((d.tabs.len(), d.content.1), (2, geometry::DOC_TABS));
    assert!(d.tabs.iter().all(|t| t.close.is_some()));
    assert_eq!(d.hit(d.tabs[1].rect.0 + 3, 10), Some(geometry::DocHit::Tab(1)));
}

#[test]
fn the_compass_offers_edges_and_the_area_under_the_mouse() {
    let mut m = ide();
    let g = m.geometry().clone();
    let d = g.documents.as_ref().unwrap().rect;
    let mid = (d.0 + d.2 / 2, d.1 + d.3 / 2);
    let guides = g.compass("explorer", Some(mid), None);
    assert_eq!(guides.len(), 9, "4 edges, the centre, 4 arms");
    let center = guides.iter().find(|(gd, _, _)| *gd == Guide::Center).unwrap();
    assert_eq!(center.2, Target::Into(Anchor::Documents));
    assert!(center.1 .0 <= mid.0 && mid.0 < center.1 .0 + center.1 .2);
    // (over its own lone group: only the edges)
    let e = g.group_of("explorer").unwrap().rect;
    assert_eq!(g.compass("explorer", Some((e.0 + 50, e.1 + 50)), None).len(), 4);
    // (over its own group of two: the arms, no centre)
    let o = g.group_of("output").unwrap().rect;
    let own = g.compass("output", Some((o.0 + o.2 / 2, o.1 + o.3 / 2)), None);
    assert_eq!(own.len(), 8);
    assert!(own.iter().all(|(gd, _, _)| *gd != Guide::Center));
    // (previews)
    assert_eq!(g.preview(&Target::Into(Anchor::Documents), 200), Some(d));
    assert_eq!(g.preview(&Target::Edge(Side::Left), 200), Some((0, 0, 200, 700)));
    assert_eq!(g.preview(&Target::Beside(Anchor::Documents, Side::Top), 200), Some((d.0, d.1, d.2, 200)));
}

#[test]
fn where_strings() {
    assert_eq!(parse_where("left"), Some(Target::Edge(Side::Left)));
    assert_eq!(parse_where("Documents"), Some(Target::Into(Anchor::Documents)));
    assert_eq!(parse_where("tab:Explorer"), Some(Target::Into(Anchor::Pane("explorer".into()))));
    assert_eq!(parse_where("bottom:explorer"), Some(Target::Beside(Anchor::Pane("explorer".into()), Side::Bottom)));
    assert_eq!(parse_where("right:documents"), Some(Target::Beside(Anchor::Documents, Side::Right)));
    assert_eq!(parse_where("autohide:top"), Some(Target::AutoHide(Side::Top)));
    assert!(matches!(parse_where("float"), Some(Target::Float(_))));
    assert_eq!(parse_where("sideways"), None);
}

#[test]
fn hide_and_show_go_back_to_the_same_place() {
    let mut m = ide();
    let before = m.layout.clone();
    for p in ["explorer", "properties", "output", "problems"] {
        let o = m.hide_pane(p);
        assert!(o.changed && o.events[0].name == "onpanechange");
        assert_eq!(m.state_of(p), "hidden");
        m.show_pane(p);
        // (shown: the tab its group shows now)
        assert!(m.layout.is_active(p));
        m.layout.select(if p == "output" { "problems" } else { p });
        assert_eq!(m.layout.root, before.root, "{p}");
    }
    assert_eq!(m.state_of("output"), "tabbed");
    assert_eq!(m.state_of("explorer"), "docked");
    assert_eq!(m.state_of("form1"), "document");
    assert_eq!(m.state_of("nothing"), "");
}

#[test]
fn autohide_on_and_off_go_back_to_the_same_place() {
    let mut m = ide();
    let before = m.layout.root.clone();
    m.auto_hide("properties", true);
    assert_eq!(m.layout.autohide[Side::Right.index()], vec!["properties".to_string()]);
    assert_eq!(m.state_of("properties"), "autohide");
    assert_eq!(m.pane("properties").unwrap().extent, DEFAULT_SIDE);
    m.auto_hide("properties", false);
    assert_eq!(m.layout.root, before);
    // (the group's pin: every pane of the group to the strip)
    let out = g_button(&mut m, "output", Button::Pin);
    assert_eq!(out.events.len(), 2);
    assert_eq!(m.layout.autohide[Side::Bottom.index()], vec!["output".to_string(), "problems".to_string()]);
    // (the flyout's pin docks it again)
    m.user(User::Flyout(Some("problems".into())));
    m.user(User::Button(Slot::Flyout, Button::Pin));
    assert_eq!(m.state_of("problems"), "docked");
    assert_eq!(m.flyout, None);
}

fn g_button(m: &mut Manager, pane: &str, b: Button) -> manager::Outcome {
    let slot = m.geometry().group_of(pane).unwrap().slot;
    m.user(User::Button(slot, b))
}

#[test]
fn float_and_dock_back() {
    let mut m = ide();
    let before = m.layout.root.clone();
    let out = m.float_pane("properties", None);
    assert_eq!(out.floating, vec![("properties".to_string(), None)]);
    m.floated("properties", (500, 200, 300, 360));
    assert_eq!(m.state_of("properties"), "floating");
    m.float_moved(0, (520, 210, 320, 380));
    assert_eq!(m.layout.floating[0].rect, (520, 210, 320, 380));
    m.user(User::Button(Slot::Float(0), Button::Dock));
    assert_eq!(m.layout.root, before);
    assert!(m.layout.floating.is_empty());
    // (closed by its window's close box: hidden, and back to its window)
    m.float_pane("explorer", None);
    m.floated("explorer", (1, 2, 300, 360));
    m.float_closed(0);
    assert_eq!(m.state_of("explorer"), "hidden");
    let o = m.show_pane("explorer");
    assert_eq!(o.floating, vec![("explorer".to_string(), None)]);
}

#[test]
fn drops_move_panes_and_never_onto_themselves() {
    let mut m = ide();
    let o = m.user(User::Drop("explorer".into(), Target::Beside(Anchor::Pane("properties".into()), Side::Top)));
    assert!(o.changed);
    let g = m.geometry().clone();
    let (e, p) = (g.group_of("explorer").unwrap().rect, g.group_of("properties").unwrap().rect);
    assert_eq!((e.0, e.2), (p.0, p.2));
    assert!(e.1 + e.3 < p.1);
    // (onto its own lone group: nothing)
    let before = m.layout.clone();
    assert!(!m.user(User::Drop("explorer".into(), Target::Into(Anchor::Pane("explorer".into())))).changed);
    assert_eq!(m.layout, before);
    // (a tab beside its own group: split off)
    m.user(User::Drop("problems".into(), Target::Beside(Anchor::Pane("problems".into()), Side::Right)));
    assert_eq!(m.state_of("problems"), "docked");
    assert_eq!(m.state_of("output"), "docked");
    // (a tool pane as a document, a document stays one)
    m.user(User::Drop("output".into(), Target::Into(Anchor::Documents)));
    assert_eq!(m.state_of("output"), "document");
    assert_eq!(m.active_document().as_deref(), Some("output"));
}

#[test]
fn documents_activate_cycle_and_close() {
    let mut m = ide();
    assert_eq!(m.active_document().as_deref(), Some("module1"));
    let o = m.next_document(false);
    assert_eq!((m.active_document().as_deref(), o.focus.as_deref()), (Some("form1"), Some("form1")));
    assert_eq!(o.events[0].name, "ondocumentactivate");
    let o = m.close_pane("form1");
    assert_eq!(o.closing, vec!["form1".to_string()]);
    let o = m.close_document("form1");
    assert_eq!(m.active_document().as_deref(), Some("module1"));
    assert!(o.events.iter().any(|e| e.name == "ondocumentactivate"));
    assert!(m.pane("form1").is_none(), "a closed document is gone");
}

#[test]
fn splitters_resize_within_their_neighbours() {
    let mut m = ide();
    let g = m.geometry().clone();
    let start = g.extents[0].1.clone();
    let ext = m.split_extents(0, &start, 60).unwrap();
    assert_eq!(ext[0], start[0] + 60);
    assert_eq!(ext[1], start[1] - 60);
    let ext = m.split_extents(0, &start, -10_000).unwrap();
    assert_eq!(ext[0], geometry::MIN_EXTENT);
    let extents = m.split_extents(0, &start, 60).unwrap();
    m.user(User::Split { splitter: 0, extents, done: true });
    assert_eq!(m.geometry().group_of("explorer").unwrap().rect.2, DEFAULT_SIDE + 60);
}

#[test]
fn reset_and_load_put_panes_back() {
    let mut m = ide();
    let fresh = m.save();
    m.hide_pane("explorer");
    m.auto_hide("output", true);
    m.user(User::Drop("properties".into(), Target::Edge(Side::Top)));
    assert_ne!(m.save(), fresh);
    m.reset();
    assert_eq!(m.save(), fresh);
    m.hide_pane("explorer");
    let changed = m.save();
    assert_eq!(m.load(&fresh).value, Some(crate::Value::Integer(-1)));
    assert_eq!(m.save(), fresh);
    assert_eq!(m.load(&changed).value, Some(crate::Value::Integer(-1)));
    assert_eq!(m.save(), changed);
    assert_eq!(m.load("nonsense").value, Some(crate::Value::Integer(0)));
    assert_eq!(m.save(), changed);
}

#[test]
fn the_keyboards_move() {
    let mut m = ide();
    assert!(m.begin_move("explorer"));
    // (its lone group: the documents' compass, the centre)
    assert_eq!(m.move_target(), Some(Target::Into(Anchor::Documents)));
    assert_eq!(m.move_key(39, false), (true, None));
    assert_eq!(m.move_target(), Some(Target::Beside(Anchor::Documents, Side::Right)));
    // (the same arrow again: the outer edge)
    m.move_key(39, false);
    assert_eq!(m.move_target(), Some(Target::Edge(Side::Right)));
    // (Tab: the next area's compass)
    m.move_key(37, false);
    m.move_key(9, false);
    assert!(m.move_target().is_some());
    let (took, action) = m.move_key(13, false);
    assert!(took);
    let Some(User::Drop(p, t)) = action else { panic!("{action:?}") };
    assert_eq!(p, "explorer");
    assert!(matches!(t, Target::Beside(_, Side::Left)), "{t:?}");
    assert!(m.ui.moving.is_none());
    // (Escape stops it; F floats)
    m.begin_move("output");
    assert_eq!(m.move_key(27, false), (true, None));
    m.begin_move("output");
    assert!(matches!(m.move_key(70, false).1, Some(User::Float(..))));
    assert!(!m.begin_move("form1"), "documents don't move");
}

#[test]
fn f6_visits_the_groups_then_the_document() {
    let mut m = ide();
    assert_eq!(m.areas_for_keys(), vec!["explorer", "problems", "properties", "module1"]);
}

#[test]
fn every_theme_draws_it() {
    let mut m = ide();
    m.auto_hide("explorer", true);
    m.layout.mode = DocumentMode::Tabs;
    m.touch();
    let g = m.geometry().clone();
    for t in crate::theme::ALL {
        assert!(!look::manager_ops(&m, &g, t, &Font::default()).is_empty());
        for gr in &g.groups {
            assert!(look::group_ops(&m, gr, t, &Font::default(), true).len() > 4, "{}", t.name);
        }
        assert!(!look::documents_ops(&m, g.documents.as_ref().unwrap(), t, &Font::default()).is_empty());
    }
    // (the compass while dragging)
    m.ui.drag = Some(manager::Drag { pane: "output".into(), from: (0, 0), at: (500, 200), started: true, target: Some(Target::Edge(Side::Left)) });
    let over = look::overlay_ops(&m, &g, &crate::theme::RAPIDR, &Font::default());
    assert!(over.len() > 20, "{}", over.len());
}

#[test]
fn a_screen_reader_hears_splitters_tabs_and_buttons() {
    let mut m = ide();
    let g = m.geometry().clone();
    let n = access::describe_manager("dock", &m, &g, (0, 0));
    assert_eq!(n.children.len(), g.splitters.len());
    assert_eq!(n.children[0].role, crate::objects::a11y::Role::Splitter);
    assert_eq!(n.children[0].name, "Resize Explorer and Documents");
    let out = g.group_of("output").unwrap();
    let gn = access::describe_group("dock__g1", &m, out, (0, 0), false);
    assert_eq!(gn.name, "Problems");
    assert_eq!(gn.children[0].role, crate::objects::a11y::Role::TabList);
    assert_eq!(gn.children[0].children.len(), 2);
    assert_eq!(gn.children[0].children[1].states.selected, Some(true));
    assert!(gn.children.iter().any(|c| c.name == "Close Problems"));
    assert_eq!(access::decode(1002), Some(access::AccessPart::Button(Button::Close)));
    assert_eq!(access::decode(1), Some(access::AccessPart::Tab(1)));
}
