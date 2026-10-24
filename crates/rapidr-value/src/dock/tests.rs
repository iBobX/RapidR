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
    // (inside its 1-pixel frame, over the header's bottom line)
    assert_eq!(explorer.content, (1, HEADER - 1, DEFAULT_SIDE - 2, 700 - HEADER));
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

#[test]
fn the_chrome_reads_in_rapidrs_looks() {
    // (DARK-FIX: a pane's title, its tabs shown and not, a document's
    // tabs and its view switch: text, WCAG AA on what each is drawn on)
    use crate::theme::{contrast, ALL};
    for t in ALL.into_iter().filter(|t| t.fluent()) {
        let p = look::palette(t);
        for (what, fg, bg) in [
            ("a title", p.title, p.header),
            ("the active title", p.title_active, p.header_active),
            ("a tab not shown", p.tab_text, p.header),
            ("a document tab not shown", p.tab_text, p.doc_strip),
            ("a view switch segment", p.tab_text, p.ground),
            ("a hovered segment", p.title_active, p.button_hover),
            ("the shown document tab", p.title_active, p.doc_tab),
            ("a strip tab", p.strip_text, p.strip_tab),
        ] {
            assert!(contrast(fg, bg) >= 4.5, "{}: {what} {fg:06X} on {bg:06X} {:.2}", t.name, contrast(fg, bg));
        }
    }
}

// ------------------------------------------------- splitters (SPLIT, S9) --

#[test]
fn a_splitter_shows_at_rest_against_both_its_neighbours() {
    // (at rest quiet: the band the panes' own gap, the ground, with a soft
    // line along it, 1.2 to 1.8:1 against the panes — a header, the active
    // one's, a pane's body, a document's strip —; high contrast's line
    // crisp. The grip carries the cue: 3:1 against its band and the line
    // it sits on, WCAG 1.4.11. Lit, the band 3:1 against the panes, the
    // grip 3:1 on it)
    use crate::theme::{contrast, ALL};
    for t in ALL.into_iter().filter(|t| t.fluent()) {
        let p = look::palette(t);
        for (beside, bg) in [("a header", p.header), ("the active header", p.header_active), ("a pane", p.body), ("the documents' strip", p.doc_strip)] {
            let (line, lit) = (contrast(p.splitter_line, bg), contrast(p.splitter_hot, bg));
            if p.contrast {
                assert!(line >= 7.0, "{}: the line beside {beside}", t.name);
            } else {
                assert!((1.2..=1.8).contains(&line), "{}: the line beside {beside} {bg:06X}: {line:.2}, quiet", t.name);
            }
            assert!(lit >= 3.0, "{}: the lit band beside {beside} {bg:06X}: {lit:.2}", t.name);
        }
        for (what, grip, band) in [("at rest", p.grip, p.splitter), ("under the mouse", p.grip_hot, p.splitter_hot)] {
            assert!(contrast(grip, band) >= 3.0, "{}: the grip {what} {grip:06X} on {band:06X}: {:.2}", t.name, contrast(grip, band));
        }
        // (on its soft line too; high contrast's grip is its crisp line
        // grown thick, the same white)
        if p.contrast {
            assert_eq!(p.grip, p.splitter_line, "{}", t.name);
        } else {
            assert!(contrast(p.grip, p.splitter_line) >= 3.0, "{}: the grip on its line", t.name);
        }
        // (from the theme's tokens, never a colour of its own)
        assert_eq!((p.grip, p.grip_hot), (t.splitter_grip, t.accent_text), "{}", t.name);
        if p.contrast {
            assert_eq!((p.splitter, p.splitter_line), (t.splitter, t.border), "{}", t.name);
        } else {
            assert_eq!((p.splitter, p.splitter_line), (p.ground, t.splitter), "{}", t.name);
        }
    }
}

#[test]
fn every_splitter_is_drawn_at_rest_with_its_grip_and_hot_in_the_accent() {
    use crate::objects::ops::Op;
    let mut m = ide();
    let g = m.geometry().clone();
    let t = &crate::theme::RAPIDR;
    let p = look::palette(t);
    let fills = |ops: &[Op], color: u32| ops.iter().filter(|o| matches!(o, Op::Fill { color: c, .. } if *c == color)).count();
    let grips = |ops: &[Op], color: u32| ops.iter().filter(|o| matches!(o, Op::Stroke { color: c, .. } if *c == color)).count();
    let band = |ops: &[Op], color: u32, r: (i64, i64, i64, i64)| ops.iter().any(|o| matches!(o, Op::Fill { color: c, rect } if *c == color && *rect == r));
    let rest = look::manager_ops(&m, &g, t, &Font::default());
    // (each: the gap the panes leave and the grip in it — nothing else,
    // nothing over the panes: their own edges show where they meet, no
    // third line between them)
    let line = |s: &geometry::Splitter| {
        let (x, y, w, h) = s.rect;
        if s.axis == Axis::Row { (x + w / 2, y, 1, h) } else { (x, y + h / 2, w, 1) }
    };
    assert!(g.splitters.iter().all(|s| band(&rest, p.splitter, s.rect)), "the gap under each splitter");
    assert_eq!(fills(&rest, p.splitter_line), 0, "no line between framed panes");
    assert_eq!(grips(&rest, p.grip), g.splitters.len(), "a grip on each");
    // (where nothing frames the panes — two views, two groups of
    // documents, RSPLITTER —: a soft line along the middle, the grip on it)
    for s in &g.splitters {
        let mut ops = Vec::new();
        look::splitter_ops(&mut ops, s.rect, s.axis == Axis::Row, false, &p, look::SplitterLook::Rest);
        assert!(band(&ops, p.splitter, s.rect) && band(&ops, p.splitter_line, line(s)), "{ops:?}");
        assert_eq!(grips(&ops, p.grip), 1);
    }
    // (under the mouse: lit only once the mouse has rested there)
    m.ui.hover = Some(manager::Part::Manager(Hit::Splitter(0)));
    assert_eq!(look::manager_ops(&m, &g, t, &Font::default()), rest, "not at once: no flicker passing over");
    m.ui.hot_ready = true;
    let hot = look::manager_ops(&m, &g, t, &Font::default());
    assert!(band(&hot, p.splitter_hot, g.splitters[0].rect), "the whole band in the accent");
    assert_eq!(grips(&hot, p.grip_hot) + grips(&hot, p.grip), g.splitters.len());
    assert!(grips(&hot, p.grip_hot) >= 1, "its grip in the accent's text colour");
    // (the grip: 20 to 28 long, 3 thick, centred along the splitter, on
    // its line across it — within half a pixel of the middle, its edges on
    // whole pixels: crisp at 1x —, inside its band, never over the panes)
    for s in &g.splitters {
        let stands = s.axis == Axis::Row;
        let (a, b, width) = look::grip(s.rect, stands, false).expect("a grip");
        let (x, y, w, h) = (s.rect.0 as f64, s.rect.1 as f64, s.rect.2 as f64, s.rect.3 as f64);
        let (cx, cy) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let long = if stands { b.1 - a.1 } else { b.0 - a.0 } + width;
        assert!((20.0..=28.0).contains(&long), "{long}");
        assert_eq!(width, look::GRIP_THICK);
        let (along, mid) = if stands { (cy, y + h / 2.0) } else { (cx, x + w / 2.0) };
        assert_eq!(along, mid, "centred along");
        let (across, start, thick) = if stands { (cx, x, w) } else { (cy, y, h) };
        assert!((across - (start + thick / 2.0)).abs() <= 0.5, "on the middle");
        let (lo, hi) = (across - width / 2.0, across + width / 2.0);
        assert!(lo >= start && hi <= start + thick, "inside its band: {lo}..{hi} of {start}+{thick}");
        assert_eq!((lo.fract(), hi.fract()), (0.0, 0.0), "on whole pixels");
        let l = line(s);
        let l = if stands { l.0 as f64 } else { l.1 as f64 };
        assert!(lo <= l && l + 1.0 <= hi, "over its line");
    }
    // (high contrast: the black gap between the panes' crisp white edges,
    // the grip white; unframed, a crisp line of its own)
    let hc = &crate::theme::RAPIDR_HIGH_CONTRAST;
    let ph = look::palette(hc);
    m.ui.hover = None;
    let hco = look::manager_ops(&m, &g, hc, &Font::default());
    assert!(g.splitters.iter().all(|s| band(&hco, ph.splitter, s.rect)), "the black gap");
    assert_eq!(grips(&hco, ph.grip), g.splitters.len(), "a white grip on each");
    let s = &g.splitters[0];
    let mut ops = Vec::new();
    look::splitter_ops(&mut ops, s.rect, s.axis == Axis::Row, false, &ph, look::SplitterLook::Rest);
    assert!(band(&ops, ph.splitter_line, line(s)), "a crisp line");
    // (the classic look: RapidQ's — the button-face gap, no grip)
    m.ui.hover = None;
    let classic = look::manager_ops(&m, &g, &crate::theme::CLASSIC, &Font::default());
    assert_eq!(classic.iter().filter(|o| matches!(o, Op::Stroke { .. })).count(), 0, "no grip");
}

#[test]
fn where_two_splitters_meet_a_corner_moves_both() {
    let mut m = ide();
    let g = m.geometry().clone();
    // (left | middle, and the middle's documents above Output: a T)
    let c = g.corners.first().expect("a corner where the splitters meet").clone();
    let (sv, sh) = (g.splitters[c.stands].clone(), g.splitters[c.lies].clone());
    assert_eq!((sv.axis, sh.axis), (Axis::Row, Axis::Column));
    assert_eq!((c.rect.2, c.rect.3), (geometry::CORNER, geometry::CORNER));
    assert_eq!((c.rect.0 + c.rect.2 / 2, c.rect.1 + c.rect.3 / 2), (sv.rect.0 + sv.rect.2 / 2, sh.rect.1 + sh.rect.3 / 2), "centred where they cross");
    assert_eq!(g.hit(c.rect.0 + 4, c.rect.1 + 4), Some(Hit::Corner(0)));
    // (under the mouse, rested: both bands lit)
    m.ui.hover = Some(manager::Part::Manager(Hit::Corner(0)));
    m.ui.hot_ready = true;
    let mut hot = look::hot_splitters(&m, &g);
    hot.sort();
    let mut both = vec![c.stands, c.lies];
    both.sort();
    assert_eq!(hot, both);
    // (the keyboard: Alt and an arrow across a splitter move the one it meets)
    m.ui.hover = None;
    m.ui.split_key = Some(manager::SplitKey { splitter: c.lies, back: None });
    let before = m.split_now(c.stands).unwrap();
    let step = m.split_key(39, false, true);
    let Some(User::Split { splitter, extents, .. }) = step.moved else { panic!("{step:?}") };
    assert_eq!(splitter, c.stands);
    assert_eq!(extents[sv.index], before[sv.index] + manager::SPLIT_STEP);
    // (Alt with an arrow along it: itself, as without Alt)
    let step = m.split_key(40, false, true);
    assert!(matches!(step.moved, Some(User::Split { splitter, .. }) if splitter == c.lies), "{step:?}");
}

#[test]
fn a_splitter_is_grabbed_a_little_past_its_gutter() {
    let mut m = ide();
    let g = m.geometry().clone();
    let s = g.splitters.iter().find(|s| s.axis == Axis::Row).unwrap().clone();
    let (x, y) = (s.rect.0, s.rect.1 + s.rect.3 / 2);
    // (8 pixels: the gutter's 4 and 2 each side)
    for dx in -geometry::GRAB..GAP + geometry::GRAB {
        assert!(matches!(g.hit(x + dx, y), Some(Hit::Splitter(_))), "{dx}");
    }
    assert_eq!(g.hit(x - geometry::GRAB - 1, y), None);
    assert_eq!(g.hit(x + GAP + geometry::GRAB, y), None);
}

#[test]
fn a_splitter_takes_the_keyboard() {
    let mut m = ide();
    // (beside Output: the splitter between the documents and it)
    assert!(m.focus_splitter(Some("output")));
    let k = m.ui.split_key.clone().unwrap();
    let s = m.geometry().splitters[k.splitter].clone();
    assert_eq!(s.axis, Axis::Column, "the one above Output");
    let at = |m: &mut Manager| m.split_now(k.splitter).unwrap()[s.index];
    let before = at(&mut m);
    // (an arrow: 10 pixels, carried out as the user's step)
    let step = m.split_key(40, false, false);
    assert!(step.took && !step.leave);
    let Some(u) = step.moved else { panic!() };
    m.user(u);
    assert_eq!(at(&mut m), before + manager::SPLIT_STEP);
    // (Shift: 50; up: back)
    let u = m.split_key(38, true, false).moved.unwrap();
    m.user(u);
    assert_eq!(at(&mut m), before + manager::SPLIT_STEP - manager::SPLIT_JUMP);
    // (Home: its least)
    let u = m.split_key(36, false, false).moved.unwrap();
    m.user(u);
    assert_eq!(at(&mut m), geometry::MIN_EXTENT);
    // (End: its most — the other side at its least)
    let u = m.split_key(35, false, false).moved.unwrap();
    m.user(u);
    let ext = m.split_now(k.splitter).unwrap();
    assert_eq!(ext[s.index + 1], geometry::MIN_EXTENT);
    // (Tab: the next splitter; Shift+Tab back)
    let n = m.geometry().splitters.len();
    assert!(m.split_key(9, false, false).took);
    assert_eq!(m.ui.split_key.as_ref().unwrap().splitter, (k.splitter + 1) % n);
    m.split_key(9, true, false);
    assert_eq!(m.ui.split_key.as_ref().unwrap().splitter, k.splitter);
    // (Enter / Escape: it leaves; another key leaves and goes on; Shift alone stays)
    assert_eq!(m.split_key(16, false, false), manager::SplitKeyStep { took: false, moved: None, leave: false });
    assert!(m.split_key(13, false, false).leave);
    assert!(m.split_key(27, false, false).leave);
    let other = m.split_key(65, false, false);
    assert!(!other.took && other.leave);
}

#[test]
fn a_double_click_puts_a_splitter_back_where_it_was() {
    let mut m = ide();
    let i = m.geometry().splitters.iter().position(|s| s.axis == Axis::Column).unwrap();
    let first = m.split_now(i).unwrap();
    let u = m.split_by(i, -120).unwrap();
    m.user(u);
    assert_ne!(m.split_now(i).unwrap(), first);
    let u = m.split_default(i).unwrap();
    m.user(u);
    assert_eq!(m.split_now(i).unwrap(), first);
}

#[test]
fn a_panes_width_and_height_read_and_set() {
    let mut m = ide();
    let h = m.pane_extent("output", Axis::Column).unwrap();
    assert_eq!(h, DEFAULT_BOTTOM);
    assert_eq!(m.pane_extent("explorer", Axis::Row), Some(DEFAULT_SIDE));
    let out = m.set_pane_extent("problems", Axis::Column, 260);
    assert!(out.changed);
    assert_eq!(m.pane_extent("output", Axis::Column), Some(260));
    m.set_pane_extent("explorer", Axis::Row, 300);
    assert_eq!(m.pane_extent("explorer", Axis::Row), Some(300));
    // (a floating or hidden pane: none)
    m.hide_pane("properties");
    assert_eq!(m.pane_extent("properties", Axis::Row), None);
}

#[test]
fn a_screen_reader_hears_where_a_splitter_is_as_a_percentage() {
    let mut m = ide();
    m.focus_splitter(Some("explorer"));
    let g = m.geometry().clone();
    let n = access::describe_manager("dock", &m, &g, (0, 0));
    let k = m.ui.split_key.as_ref().unwrap().splitter;
    let sp = &n.children[k];
    let num = sp.numeric.unwrap();
    assert_eq!((num.min, num.max), (0.0, 100.0));
    assert!(num.value > 0.0 && num.value < 100.0, "{}", num.value);
    assert_eq!(sp.value.as_deref(), Some(format!("{}%", num.value.round() as i64).as_str()));
    // (a whole number: the same on every host — a browser keeps it as a
    // 32-bit float, 32.7 read back as 32.70000076293945)
    assert_eq!(num.value.fract(), 0.0);
    assert_eq!(f64::from(num.value as f32), num.value);
    assert_eq!(access::share(&[327, 673], 0).1, "33%");
    assert!(sp.states.focused, "the splitter given the keyboard is focused");
    assert!(sp.actions.contains(&crate::objects::a11y::Action::Focus));
}

/// A runtime over a table of properties (the dock's runtime glue, without
/// a GUI): which components were laid out again, their OnResize.
mod relayout {
    use super::super::runtime::{self, Runtime};
    use crate::Value;
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static PROPS: RefCell<HashMap<(String, String), Value>> = RefCell::new(HashMap::new());
        static TYPES: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
        pub static LAID: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    #[derive(Clone, Copy)]
    pub struct Fake;

    impl Runtime for Fake {
        fn get(self, name: &str, prop: &str) -> Value {
            PROPS.with(|p| p.borrow().get(&(name.to_ascii_lowercase(), prop.to_ascii_lowercase())).cloned().unwrap_or(Value::Null))
        }
        fn set(self, name: &str, prop: &str, v: Value) {
            PROPS.with(|p| p.borrow_mut().insert((name.to_ascii_lowercase(), prop.to_ascii_lowercase()), v));
        }
        fn create(self, name: &str, type_name: &str) {
            TYPES.with(|t| t.borrow_mut().insert(name.to_ascii_lowercase(), type_name.to_string()));
        }
        fn exists(self, name: &str) -> bool {
            TYPES.with(|t| t.borrow().contains_key(&name.to_ascii_lowercase()))
        }
        fn type_of(self, name: &str) -> String {
            TYPES.with(|t| t.borrow().get(&name.to_ascii_lowercase()).cloned().unwrap_or_default())
        }
        fn fire(self, _: &str, _: &str, _: &[Value]) {}
        fn fire_then(self, _: &str, _: &str, args: &[Value], then: runtime::Then) {
            then(args)
        }
        fn children(self, _: &str) -> Vec<(String, String)> {
            Vec::new()
        }
        fn restructure(self) {}
        fn focus(self, _: &str) {}
        fn show(self, _: &str) {}
        fn relayout(self, name: &str) {
            LAID.with(|l| l.borrow_mut().push(name.to_ascii_lowercase()));
        }
    }

    pub fn component(name: &str, t: &str, w: i64, h: i64) {
        Fake.create(name, t);
        Fake.set(name, "width", Value::Integer(w));
        Fake.set(name, "height", Value::Integer(h));
    }
}

#[test]
fn a_pane_given_a_new_size_has_its_contents_laid_out_at_it() {
    // (S8: a pane placed while its dock manager is itself laid out — the
    // window restored or maximized at start-up — is laid out again at its
    // size; one whose size didn't change isn't)
    use relayout::{component, Fake, LAID};
    use runtime::Runtime;
    let dock = "s8dock";
    component(dock, "RDOCKMANAGER", 1000, 700);
    for p in ["s8left", "s8out", "s8vars"] {
        component(p, "RPANEL", 440, 190);
    }
    manager::with_mut(dock, |m| {
        m.add_pane("s8left", "Left", "left", "");
        m.add_pane("s8out", "Output", "bottom:documents", "");
        m.add_pane("s8vars", "Variables", "tab:s8out", "");
    });
    let laid = || LAID.with(|l| std::mem::take(&mut *l.borrow_mut()));
    runtime::apply(Fake, dock);
    let first = laid();
    assert!(first.contains(&"s8left".to_string()) && first.contains(&"s8vars".to_string()), "{first:?}");
    // (the same size again: nothing)
    runtime::apply(Fake, dock);
    assert_eq!(laid(), Vec::<String>::new());
    // (the window grows: the shown panes again, at their new size)
    Fake.set(dock, "width", crate::Value::Integer(1600));
    runtime::apply(Fake, dock);
    let grown = laid();
    assert!(grown.contains(&"s8vars".to_string()), "{grown:?}");
    assert_eq!(Fake.get("s8vars", "width").to_i64(), manager::with_mut(dock, |m| m.geometry().group_of("s8vars").unwrap().content.2));
    // (a tab shown at a size it hadn't: laid out too)
    manager::with_mut(dock, |m| m.show_pane("s8out"));
    Fake.set("s8out", "width", crate::Value::Integer(10));
    runtime::apply(Fake, dock);
    assert!(laid().contains(&"s8out".to_string()));
}

// ------------------------------------------- the panes' edges (SPLIT-3) --

/// What `ops` leave on a `w` × `h` grid — their fills (a frame, a header,
/// a strip are fills; text and glyphs are not) — with a pane's component
/// then over each of `panes` (it draws over everything it covers).
fn painted(ops: &[crate::objects::ops::Op], (w, h): (i64, i64), panes: &[Rect]) -> Vec<u32> {
    // (not a colour: a pane's own pixels)
    const PANE: u32 = 0x0100_0000;
    let mut px = vec![0u32; (w * h) as usize];
    let mut put = |r: Rect, c: u32| {
        for y in r.1.max(0)..(r.1 + r.3).min(h) {
            for x in r.0.max(0)..(r.0 + r.2).min(w) {
                px[(y * w + x) as usize] = c;
            }
        }
    };
    for o in ops {
        if let crate::objects::ops::Op::Fill { rect, color } = o {
            put(*rect, *color);
        }
    }
    for r in panes {
        put(*r, PANE);
    }
    px
}

#[test]
fn every_pane_keeps_its_edge_all_the_way_down() {
    // (SPLIT-3: a pane's component sits inside its group's 1-pixel frame —
    // left, right and bottom —, so the frame shows the whole height beside
    // it, not only beside the header or the tab strip; the same for each
    // group of documents. Every look lays them out alike; RapidR's looks
    // draw the frame, the classic look the components' own sunken edges)
    use crate::theme::ALL;
    // (one group of documents, then two side by side)
    let mut split = tabbed();
    split.user(User::MoveDocument("module2".into(), DocTarget::Split { anchor: "form1".into(), side: Side::Right }));
    for mut m in [tabbed(), split] {
        let g = m.geometry().clone();
        let d = g.documents.clone().unwrap();
        let active_doc = m.active_document();
        for t in ALL {
            let p = look::palette(t);
            for gr in &g.groups {
                let (w, h) = (gr.rect.2, gr.rect.3);
                let (cx, cy, cw, ch) = gr.content;
                assert_eq!((cx, cy, cx + cw, cy + ch), (1, HEADER - 1, w - 1, h - 1), "{}: {:?} inside its frame", t.name, gr.panes);
                if !t.fluent() {
                    continue;
                }
                for active in [false, true] {
                    let px = painted(&look::group_ops(&m, gr, t, &Font::default(), active), (w, h), &[gr.content]);
                    let edge = if p.contrast && active { p.accent } else { p.border };
                    let at = |x: i64, y: i64| px[(y * w + x) as usize];
                    // (under the active one's accent along the top)
                    for y in 3..h {
                        assert_eq!((at(0, y), at(w - 1, y)), (edge, edge), "{}: {:?} active {active}, its sides at y {y}", t.name, gr.panes);
                    }
                    for x in 0..w {
                        assert_eq!(at(x, h - 1), edge, "{}: {:?} active {active}, its bottom at x {x}", t.name, gr.panes);
                    }
                }
            }
            let (w, h) = (d.rect.2, d.rect.3);
            let places: Vec<Rect> = d.groups.iter().flat_map(|gr| gr.places.clone()).collect();
            let px = painted(&look::documents_ops(&m, &d, t, &Font::default()), (w, h), &places);
            for gr in &d.groups {
                let (gx, gy, gw, gh) = gr.rect;
                let (cx, cy, cw, ch) = gr.content;
                assert_eq!((cx, cy, cx + cw, cy + ch), (gx + 1, gy + geometry::DOC_TABS, gx + gw - 1, gy + gh - 1), "{}: the documents inside their frame", t.name);
                assert!(gr.places.iter().all(|r| r.0 >= cx && r.1 >= cy && r.0 + r.2 <= cx + cw && r.1 + r.3 <= cy + ch), "{}: each view inside it", t.name);
                if !t.fluent() {
                    continue;
                }
                let active = active_doc.as_ref().is_some_and(|a| gr.docs.contains(a));
                let edge = if p.contrast && active { p.accent } else { p.border };
                let at = |x: i64, y: i64| px[(y * w + x) as usize];
                // (from the strip's bottom line down; high contrast frames the
                // strip too)
                let top = if p.contrast { gy } else { gy + geometry::DOC_TABS - 1 };
                for y in top..gy + gh {
                    assert_eq!((at(gx, y), at(gx + gw - 1, y)), (edge, edge), "{}: the documents' sides at y {y}", t.name);
                }
                for x in gx..gx + gw {
                    assert_eq!(at(x, gy + gh - 1), edge, "{}: the documents' bottom at x {x}", t.name);
                }
            }
        }
        assert!(!d.groups.is_empty());
    }
}

#[test]
fn a_panes_edge_shows_at_rest_in_every_look() {
    // (SPLIT-3: between two panes the gap is their two edges and the gutter
    // between them. Each edge stands at least 1.3:1 off its pane — a
    // hairline, like the splitter token's 1.3 to 1.6 —, and at most 2:1, not
    // a wireframe. The light look's gutter is itself a greyer band beside the
    // white panes (1.17:1), so the edges only finish it; the dark look's
    // gutter is the panes' own colour (about 1:1), so its two edges alone
    // show the gap: they stand at least 1.6:1 off it and off the panes (now
    // 1.70:1; at 1.37 the gap hid until the mouse came). High contrast's
    // edges are its crisp white)
    use crate::theme::{contrast, ALL};
    for t in ALL.into_iter().filter(|t| t.fluent()) {
        let p = look::palette(t);
        let (edge, band) = (contrast(p.border, p.body), contrast(p.ground, p.body));
        if p.contrast {
            assert!(contrast(p.border, p.ground) >= 7.0 && edge >= 7.0, "{}: crisp edges", t.name);
            continue;
        }
        assert!((1.3..=2.0).contains(&edge), "{}: the edge {:06X} on a pane {:06X}: {edge:.2}", t.name, p.border, p.body);
        assert!((1.25..=2.0).contains(&contrast(p.border, p.header)), "{}: beside a header {:.2}", t.name, contrast(p.border, p.header));
        let on_gutter = contrast(p.border, p.ground);
        if band >= 1.15 {
            // (the gutter a band of its own: the light look)
            assert!(on_gutter >= 1.1, "{}: the edge on its gutter {on_gutter:.2}", t.name);
        } else {
            assert!(on_gutter >= 1.6 && edge >= 1.6, "{}: the gutter is the panes' colour ({band:.2}): the edges alone show the gap — {on_gutter:.2} on it, {edge:.2} on a pane", t.name);
        }
    }
    // (the light look as it was: Robert's; the dark one's edge 1.7:1)
    assert_eq!(look::palette(&crate::theme::RAPIDR).border, crate::theme::RAPIDR.border);
    let dark = look::palette(&crate::theme::RAPIDR_DARK);
    assert!((contrast(dark.border, dark.ground) - 1.70).abs() < 0.02, "{:.3}", contrast(dark.border, dark.ground));
}

/// (A11Y) Tabs that don't all fit give up their padding before their names
/// are cut, then the widest are trimmed alike — a short name is never cut
/// for a long one's sake.
#[test]
fn tabs_that_do_not_fit_give_up_padding_before_their_names() {
    // (they fit: untouched)
    assert_eq!(geometry::squeeze(&[100, 80], 200, 12, 40), vec![100, 80]);
    // (26 pixels over for four tabs: 7 each off the padding, every name whole)
    let w = geometry::squeeze(&[108, 82, 112, 147], 423, 12, 40);
    assert_eq!(w, vec![101, 75, 105, 140]);
    // (more than the padding can give: the padding all gone — 12 each — and the
    // widest trimmed alike to a common width, the short ones kept)
    let w = geometry::squeeze(&[300, 100, 100], 400, 12, 40);
    assert_eq!(w, vec![224, 88, 88]);
    assert!(w.iter().sum::<i64>() <= 400);
    // (the tabs trimmed alike down to what's left, a narrow one only by its padding)
    let w = geometry::squeeze(&[200, 200, 30], 100, 12, 40);
    assert_eq!(w, vec![41, 41, 18]);
    // (never under the floor when there's room for it)
    assert_eq!(geometry::squeeze(&[200, 200], 100, 12, 40), vec![50, 50]);
    assert_eq!(geometry::squeeze(&[200, 200], 60, 12, 40), vec![40, 40]);
}

/// …in the geometry: a crowded group's tabs have room for every title when
/// the padding can make it, and the title's place is what's drawn in.
#[test]
fn a_crowded_group_keeps_its_names_whole() {
    use geometry::Titles;
    struct T;
    impl Titles for T {
        fn title(&self, pane: &str) -> String {
            pane.to_string()
        }
        fn icon(&self, _pane: &str) -> String {
            "x".into()
        }
    }
    let font = Font::default();
    let panes: Vec<String> = ["Variables", "Watch", "Call Stack", "Breakpoints (12)"].iter().map(|s| s.to_string()).collect();
    let bold = Font { styles: font.styles | 1, ..font.clone() };
    let natural: i64 = panes.iter().map(|p| crate::objects::text::text_size(p, &bold).0 + 20 + 22).sum();
    // (a few pixels short of what the tabs want, the buttons' room taken off)
    let w = natural + 4 + 2 * 22 + 4 - 20;
    let gr = geometry::group(Slot::Docked(0), &panes, 3, (0, 0, w, 200), vec![], &T, &font);
    for (p, t) in panes.iter().zip(&gr.tabs) {
        let room = t.text.1 - t.text.0;
        let need = crate::objects::text::text_size(p, &bold).0;
        assert!(room >= need, "{p}: room {room} for {need}");
        assert!(t.text.0 > t.rect.0 + 22 && t.text.1 < t.rect.0 + t.rect.2, "{p}: the text is inside its tab");
    }
    // (the tabs still end where the buttons begin)
    let first_button = gr.buttons.iter().map(|b| b.1 .0).min().unwrap();
    assert!(gr.tabs.last().map(|t| t.rect.0 + t.rect.2).unwrap() <= first_button);
    // (and plenty of room: the paddings as they were, 10 and 8)
    let wide = geometry::group(Slot::Docked(0), &panes, 3, (0, 0, w + 400, 200), vec![], &T, &font);
    let t = &wide.tabs[0];
    assert_eq!((t.text.0 - t.rect.0, t.rect.0 + t.rect.2 - t.text.1), (10 + 22, 8));
}

/// Every pane of RapidR Studio has its icon on its tab (Preview 4's hotfix:
/// the Assistant and Help had none): each name `ide/*.inc` gives a pane
/// draws something, in the ink and in the accent.
#[test]
fn every_studio_pane_has_an_icon() {
    let ide = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ide");
    let mut names: Vec<String> = Vec::new();
    for f in std::fs::read_dir(&ide).unwrap() {
        let path = f.unwrap().path();
        if path.extension().is_some_and(|e| e == "inc") {
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                // (Dock.AddPane(Pane, "Title", "where", "icon"))
                if let Some(rest) = line.trim().strip_prefix("Dock.AddPane(").or_else(|| line.split("THEN Dock.AddPane(").nth(1)) {
                    let args: Vec<&str> = rest.trim_end_matches(')').split(',').map(str::trim).collect();
                    if args.len() == 4 && args[3].starts_with('"') {
                        names.push(args[3].trim_matches('"').to_string());
                    }
                }
            }
        }
    }
    names.sort();
    names.dedup();
    assert!(names.len() >= 14 && names.contains(&"ai".to_string()) && names.contains(&"help".to_string()), "{names:?}");
    for n in names {
        let mut ops = Vec::new();
        super::look::icon(&mut ops, &n, 0, 0, 0x202020, 0x2060d0);
        assert!(!ops.is_empty(), "the pane icon \"{n}\" draws nothing");
    }
}
