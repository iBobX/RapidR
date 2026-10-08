//! The dock layout model: every operation, the text round trip, the
//! geometry, the compass, the manager's methods and the keyboard's move.

use super::geometry::{self, Button, Guide, Hit, Slot, GAP, HEADER, STRIP};
use super::manager::{parse_where, DocView, Manager, User};
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
    assert!(d.groups.is_empty(), "MDI: no tabs");
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
    assert_eq!(d.groups.len(), 1);
    let gr = &d.groups[0];
    assert_eq!((gr.tabs.len(), gr.content.1), (2, geometry::DOC_TABS));
    assert!(gr.tabs.iter().all(|t| t.close.is_some()));
    assert_eq!(d.hit(gr.tabs[1].rect.0 + 3, 10), Some(geometry::DocHit::Tab(0, 1)));
}

/// Tabbed documents in an IDE's layout (four documents).
fn tabbed() -> Manager {
    let mut m = ide();
    m.layout.mode = DocumentMode::Tabs;
    m.add_pane("Form2", "Form2.rr", "documents", "form");
    m.add_pane("Module2", "Module2.rr", "documents", "code");
    m.touch();
    m
}

#[test]
fn documents_split_into_groups_and_join_again() {
    let mut m = tabbed();
    assert_eq!(m.layout.group_count(), 1);
    // (dragged to the right of its group: a new group there, it active)
    m.user(User::MoveDocument("module2".into(), DocTarget::Split { anchor: "form1".into(), side: Side::Right }));
    assert_eq!(m.layout.group_count(), 2);
    assert_eq!(m.layout.documents, ["form1", "module1", "form2", "module2"]);
    assert_eq!(m.active_document().as_deref(), Some("module2"));
    let g = m.geometry().clone();
    let d = g.documents.as_ref().unwrap();
    assert_eq!(d.groups.len(), 2);
    assert_eq!(d.splitters.len(), 1);
    // (side by side, the room shared alike)
    let (a, b) = (d.groups[0].rect, d.groups[1].rect);
    assert_eq!((a.1, b.1, a.3, b.3), (0, 0, d.rect.3, d.rect.3));
    assert!((a.2 - b.2).abs() <= 1 && a.2 + b.2 + GAP == d.rect.2, "{a:?} {b:?}");
    // (a new document goes to the active group)
    m.add_pane("Module3", "Module3.rr", "documents", "code");
    assert_eq!(m.layout.groups.groups()[1].1, ["module2", "module3"]);
    // (below the second group: a column in the row's second place)
    m.user(User::MoveDocument("module3".into(), DocTarget::Split { anchor: "module2".into(), side: Side::Bottom }));
    assert_eq!(m.layout.group_count(), 3);
    assert!(matches!(&m.layout.groups, DocNode::Split { axis: Axis::Row, children, .. } if matches!(children[1], DocNode::Split { axis: Axis::Column, .. })));
    // (into the first group's strip at tab 1)
    m.user(User::MoveDocument("module3".into(), DocTarget::Into { anchor: "form1".into(), index: 1 }));
    assert_eq!(m.layout.group_count(), 2);
    assert_eq!(m.layout.groups.groups()[0].1, ["form1", "module3", "module1", "form2"]);
    // (the last of a group closed: the group goes, the other is active)
    m.layout.select("module2");
    let out = m.close_document("module2");
    assert_eq!(m.layout.group_count(), 1);
    assert!(out.events.iter().any(|e| e.name == "ondocumentactivate"));
    // (beside its own lone group: nothing)
    assert!(!m.layout.move_document("form1", &DocTarget::Split { anchor: "form1".into(), side: Side::Left }) || m.layout.group_count() == 2);
}

#[test]
fn a_tab_reorders_in_its_strip() {
    let mut m = tabbed();
    m.user(User::MoveDocument("form1".into(), DocTarget::Into { anchor: "form1".into(), index: 3 }));
    assert_eq!(m.layout.documents, ["module1", "form2", "form1", "module2"]);
    assert_eq!(m.active_document().as_deref(), Some("form1"));
    m.user(User::MoveDocument("module2".into(), DocTarget::Into { anchor: "form1".into(), index: 0 }));
    assert_eq!(m.layout.documents, ["module2", "module1", "form2", "form1"]);
}

#[test]
fn drops_over_a_group_split_it_or_join_it() {
    let mut m = tabbed();
    let g = m.geometry().clone();
    let d = g.documents.clone().unwrap();
    let gr = &d.groups[0];
    let (cx, cy, cw, ch) = gr.content;
    // (the right quarter: a new group on the right, the outline its half)
    let (t, preview, bar) = d.drop_at("form1", cx + cw - 10, cy + ch / 2).unwrap();
    assert_eq!(t, DocTarget::Split { anchor: "form1".into(), side: Side::Right });
    assert_eq!(preview, Some((cx + cw - cw / 2, cy, cw / 2, ch)));
    assert!(bar.is_none());
    // (the bottom quarter)
    assert_eq!(d.drop_at("form1", cx + cw / 2, cy + ch - 5).unwrap().0, DocTarget::Split { anchor: "form1".into(), side: Side::Bottom });
    // (the middle of its own group: nowhere)
    assert!(d.drop_at("form1", cx + cw / 2, cy + ch / 2).is_none());
    // (the strip: between tabs, the bar there)
    let t2 = gr.tabs[2].rect;
    let (t, _, bar) = d.drop_at("form1", t2.0 + 3, t2.1 + 5).unwrap();
    assert_eq!(t, DocTarget::Into { anchor: "form1".into(), index: 2 });
    assert_eq!(bar.map(|b| b.0), Some(t2.0 - 2));
    // (two groups: the middle of the other joins it)
    m.user(User::MoveDocument("module2".into(), DocTarget::Split { anchor: "form1".into(), side: Side::Right }));
    let g = m.geometry().clone();
    let d = g.documents.clone().unwrap();
    let (cx, cy, cw, ch) = d.groups[1].content;
    assert_eq!(d.drop_at("form1", cx + cw / 2, cy + ch / 2).unwrap().0, DocTarget::Into { anchor: "module2".into(), index: 1 });
    // (its lone document's own group: no split beside itself)
    assert!(d.drop_at("module2", cx + cw - 5, cy + ch / 2).is_none());
}

#[test]
fn group_splitters_share_the_room_by_weight() {
    let mut m = tabbed();
    m.user(User::MoveDocument("module2".into(), DocTarget::Split { anchor: "form1".into(), side: Side::Right }));
    let g = m.geometry().clone();
    let d = g.documents.clone().unwrap();
    let sp = d.splitters[0].clone();
    let hit = d.hit(sp.rect.0 + 1, sp.rect.1 + 100);
    assert_eq!(hit, Some(geometry::DocHit::Splitter(0)));
    let start = d.extents[0].1.clone();
    m.user(User::DocSplit { path: sp.path.clone(), extents: vec![start[0] + 100, start[1] - 100], done: true });
    let g = m.geometry().clone();
    let d2 = g.documents.clone().unwrap();
    assert_eq!(d2.groups[0].rect.2, start[0] + 100);
    // (resized: the same shares)
    m.resize((1500, 700), &Font::default());
    let g = m.geometry().clone();
    let d3 = g.documents.clone().unwrap();
    let r = d3.groups[0].rect.2 as f64 / (d3.groups[0].rect.2 + d3.groups[1].rect.2) as f64;
    let r0 = (start[0] + 100) as f64 / (start[0] + start[1]) as f64;
    assert!((r - r0).abs() < 0.01, "{r} {r0}");
    assert_eq!(geometry::weighted(&[1, 1, 2], 400), vec![100, 100, 200]);
    assert_eq!(geometry::weighted(&[1, 1000], 1000).iter().min(), Some(&geometry::MIN_EXTENT));
}

#[test]
fn views_switch_and_sit_side_by_side() {
    let mut m = tabbed();
    m.add_view("form1", "form1design", "Design");
    m.add_view("form1", "form1", "Code");
    m.layout.select("form1");
    m.touch();
    let g = m.geometry().clone();
    let d = g.documents.clone().unwrap();
    let gr = &d.groups[0];
    // (Design | Code | side by side at the strip's right)
    assert_eq!(gr.switch.iter().map(|s| s.1.as_str()).collect::<Vec<_>>(), ["Design", "Code", "Side by Side"]);
    assert!(gr.switch.last().unwrap().2 .0 + gr.switch.last().unwrap().2 .2 <= gr.rect.0 + gr.rect.2);
    assert!(gr.tabs.iter().all(|t| t.rect.0 + t.rect.2 < gr.switch[0].2 .0));
    assert_eq!(m.pane("form1").unwrap().shown_components(), ["form1design"]);
    assert_eq!(d.hit(gr.switch[1].2 .0 + 4, gr.switch[1].2 .1 + 4), Some(geometry::DocHit::View(0, 1)));
    // (Code: OnDocumentView, the editor shown)
    let out = m.user(User::View("form1".into(), DocView::One(1)));
    assert_eq!(out.events[0].name, "ondocumentview");
    assert_eq!(out.events[0].args[1], crate::Value::String("Code".into()));
    assert_eq!(out.focus.as_deref(), Some("form1"));
    assert_eq!(m.pane("form1").unwrap().view_caption(), "Code");
    // (side by side: two places, the splitter between)
    m.set_view("form1", m.view_named("form1", "split").unwrap());
    let g = m.geometry().clone();
    let gr = g.documents.clone().unwrap().groups[0].clone();
    assert_eq!(gr.places.len(), 2);
    let vs = gr.view_splitter.unwrap();
    assert_eq!(gr.places[0].0 + gr.places[0].2, vs.0);
    assert_eq!(vs.0 + GAP, gr.places[1].0);
    m.user(User::ViewRatio("form1".into(), 300, true));
    let g = m.geometry().clone();
    let gr = g.documents.clone().unwrap().groups[0].clone();
    assert_eq!(gr.places[0].2, (gr.content.2 - GAP) * 300 / 1000);
    assert_eq!(m.document_of_view("form1design").as_deref(), Some("form1"));
    // (the view and its share kept in the layout's text, back with OnDocumentView)
    let text = m.save();
    assert!(text.ends_with("view form1 split 300\n"), "{text}");
    let mut n = tabbed();
    n.add_view("form1", "form1design", "Design");
    n.add_view("form1", "form1", "Code");
    let out = n.load(&text);
    assert_eq!(out.events.iter().map(|e| e.name).collect::<Vec<_>>(), ["ondocumentview"]);
    assert_eq!((n.pane("form1").unwrap().view, n.pane("form1").unwrap().view_ratio), (DocView::Split, 300));
    assert_eq!(n.save(), text);
    // (FocusPane on a view shows it; side by side, it stays so)
    let out = n.focus_pane("form1design");
    assert_eq!((out.focus.as_deref(), n.pane("form1").unwrap().view), (Some("form1design"), DocView::Split));
    n.set_view("form1", DocView::One(1));
    let out = n.focus_pane("form1design");
    assert_eq!((out.focus.as_deref(), n.pane("form1").unwrap().view), (Some("form1design"), DocView::First));
    assert_eq!(out.events[0].name, "ondocumentview");
    // (a document without views has no switch)
    m.layout.select("module1");
    m.touch();
    assert!(m.geometry().documents.clone().unwrap().groups[0].switch.is_empty());
}

#[test]
fn split_groups_round_trip_as_text() {
    let mut m = tabbed();
    m.user(User::MoveDocument("module2".into(), DocTarget::Split { anchor: "form1".into(), side: Side::Right }));
    m.user(User::MoveDocument("form2".into(), DocTarget::Split { anchor: "module2".into(), side: Side::Bottom }));
    let text = m.save();
    assert!(text.contains("groups split row\n  1000 group "), "{text}");
    let mut n = tabbed();
    assert_eq!(n.load(&text).value, Some(crate::Value::Integer(-1)));
    assert_eq!(n.layout.groups, m.layout.groups);
    assert_eq!(n.layout.documents, m.layout.documents);
    assert_eq!(n.active_document(), m.active_document());
    assert_eq!(n.save(), text);
    // (a layout without groups: one group, as before)
    let plain = tabbed().save();
    assert!(!plain.contains("groups"), "{plain}");
    // (documents the text doesn't know join the active group)
    let mut k = tabbed();
    k.add_pane("Extra", "Extra.rr", "documents", "code");
    k.load(&text);
    assert!(k.layout.documents.contains(&"extra".to_string()) && k.layout.group_count() == 3, "{:?}", k.layout.groups);
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
