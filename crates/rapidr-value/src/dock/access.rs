//! What a screen reader is told about a dock manager (the shared rules,
//! the web's too: the kernel's tree is mirrored there):
//!
//! - the dock manager: a pane; its splitters (separators, oriented, named
//!   by what they divide, their position a value) and its auto-hidden
//!   panes' strip tabs (buttons: "Explorer (auto-hidden)");
//! - each group: a group named by its shown pane; with more than one pane
//!   a tab list of tabs (selected or not); its buttons ("Close Explorer",
//!   "Auto Hide Explorer", "Dock Search"); its pane's component inside it;
//! - the document area: a group "Documents"; in tabbed mode its tabs and
//!   their close buttons.
//!
//! Every part can be clicked (as the mouse would): the ids say which
//! ([`decode`]).

use super::geometry::{Button, Documents, Geometry, Group, Slot, Titles};
use super::manager::Manager;
use super::{Axis, Node};
use crate::objects::a11y::{node_id, part_id, AccessNode, Action, Numeric, Orientation, Role, PART_ITEM, PART_TAB};

const BUTTONS: usize = 1000;
const SPLITTERS: usize = 2000;
const STRIPS: usize = 3000;
const CLOSES: usize = 4000;
const TABLIST: usize = 999;

/// A part of a dock manager's components, from its index (`a11y::part_id`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessPart {
    Tab(usize),
    Button(Button),
    Splitter(usize),
    Strip(usize),
    Close(usize),
}

pub fn decode(index: usize) -> Option<AccessPart> {
    Some(match index {
        i if i < TABLIST => AccessPart::Tab(i),
        i if (BUTTONS..BUTTONS + 3).contains(&i) => AccessPart::Button([Button::Pin, Button::Dock, Button::Close][i - BUTTONS]),
        i if (SPLITTERS..STRIPS).contains(&i) => AccessPart::Splitter(i - SPLITTERS),
        i if (STRIPS..CLOSES).contains(&i) => AccessPart::Strip(i - STRIPS),
        i if (CLOSES..CLOSES + 1000).contains(&i) => AccessPart::Close(i - CLOSES),
        _ => return None,
    })
}

fn button_index(b: Button) -> usize {
    BUTTONS
        + match b {
            Button::Pin => 0,
            Button::Dock => 1,
            Button::Close => 2,
        }
}

fn moved(r: super::Rect, by: (i64, i64)) -> super::Rect {
    (r.0 + by.0, r.1 + by.1, r.2, r.3)
}

/// The dock manager's node (bounds from its own corner, `at`: where it is
/// in its window).
pub fn describe_manager(id: &str, m: &Manager, g: &Geometry, at: (i64, i64)) -> AccessNode {
    let mut n = AccessNode::new(node_id(id), Role::Pane);
    n.bounds = (at.0, at.1, g.size.0, g.size.1);
    let titles = m.titles();
    let name_of = |path: &[usize]| -> String {
        match m.layout.root.at(path) {
            Some(Node::Tabs { panes, active }) => titles.title(&panes[*active]),
            Some(Node::Documents) => "Documents".into(),
            Some(split) => {
                let mut all = Vec::new();
                split.panes(&mut all);
                if split.has_documents() {
                    "Documents".into()
                } else {
                    all.first().map(|p| titles.title(p)).unwrap_or_default()
                }
            }
            None => String::new(),
        }
    };
    for (i, s) in g.splitters.iter().enumerate() {
        let mut sp = AccessNode::new(part_id(id, PART_ITEM, SPLITTERS + i), Role::Splitter);
        let mut a = s.path.clone();
        a.push(s.index);
        let mut b = s.path.clone();
        b.push(s.index + 1);
        sp.name = format!("Resize {} and {}", name_of(&a), name_of(&b));
        // (ARIA's sense: a bar between left and right panes stands up)
        sp.orientation = Some(if s.axis == Axis::Row { Orientation::Vertical } else { Orientation::Horizontal });
        if let Some((_, ext)) = g.extents.iter().find(|(p, _)| *p == s.path) {
            let total: i64 = ext.iter().sum();
            sp.numeric = Some(Numeric { value: ext[..=s.index].iter().sum::<i64>() as f64, min: 0.0, max: total as f64, step: 8.0, jump: 40.0 });
        }
        sp.actions = vec![Action::Increment, Action::Decrement];
        sp.bounds = moved(s.rect, at);
        n.children.push(sp);
    }
    for (i, st) in g.strip_tabs.iter().enumerate() {
        let mut b = AccessNode::new(part_id(id, PART_ITEM, STRIPS + i), Role::Button);
        b.name = format!("{} (auto-hidden)", titles.title(&st.pane));
        b.states.expanded = Some(m.flyout.as_deref() == Some(st.pane.as_str()));
        b.actions = vec![Action::Click];
        b.bounds = moved(st.rect, at);
        n.children.push(b);
    }
    n
}

/// A group's node (`at`: where it is in its window).
pub fn describe_group(id: &str, m: &Manager, gr: &Group, at: (i64, i64), active: bool) -> AccessNode {
    let titles = m.titles();
    let mut n = AccessNode::new(node_id(id), Role::Group);
    n.bounds = (at.0, at.1, gr.rect.2, gr.rect.3);
    let shown = gr.panes.get(gr.active).cloned().unwrap_or_default();
    n.name = titles.title(&shown);
    n.description = match gr.slot {
        Slot::Float(_) => "Floating".into(),
        Slot::Flyout => "Auto-hidden".into(),
        Slot::Docked(_) => String::new(),
    };
    n.states.focused = false;
    let _ = active;
    if !gr.single() {
        let mut list = AccessNode::new(part_id(id, PART_TAB, TABLIST), Role::TabList);
        list.name = n.name.clone();
        list.bounds = (at.0, at.1, gr.rect.2, super::geometry::HEADER);
        for (i, t) in gr.tabs.iter().enumerate() {
            let mut tab = AccessNode::new(part_id(id, PART_TAB, i), Role::Tab);
            tab.name = titles.title(&t.pane);
            tab.states.selected = Some(i == gr.active);
            tab.actions = vec![Action::Click];
            tab.bounds = moved(t.rect, at);
            list.children.push(tab);
        }
        n.children.push(list);
    }
    for (b, r) in &gr.buttons {
        let mut node = AccessNode::new(part_id(id, PART_ITEM, button_index(*b)), Role::Button);
        node.name = match (b, gr.slot) {
            (Button::Pin, Slot::Flyout) => format!("Dock {}", n.name),
            _ => format!("{} {}", b.name(), n.name),
        };
        node.actions = vec![Action::Click];
        node.bounds = moved(*r, at);
        n.children.push(node);
    }
    n
}

/// The document area's node.
pub fn describe_documents(id: &str, m: &Manager, d: &Documents, at: (i64, i64)) -> AccessNode {
    let titles = m.titles();
    let mut n = AccessNode::new(node_id(id), Role::Group);
    n.name = "Documents".into();
    n.bounds = (at.0, at.1, d.rect.2, d.rect.3);
    if !d.tabs.is_empty() {
        let mut list = AccessNode::new(part_id(id, PART_TAB, TABLIST), Role::TabList);
        list.name = "Documents".into();
        list.bounds = (at.0, at.1, d.rect.2, super::geometry::DOC_TABS);
        for (i, t) in d.tabs.iter().enumerate() {
            let mut tab = AccessNode::new(part_id(id, PART_TAB, i), Role::Tab);
            tab.name = titles.title(&t.pane);
            tab.states.selected = Some(m.layout.active_document == Some(i));
            tab.actions = vec![Action::Click];
            tab.bounds = moved(t.rect, at);
            if let Some(c) = t.close {
                let mut close = AccessNode::new(part_id(id, PART_ITEM, CLOSES + i), Role::Button);
                close.name = format!("Close {}", tab.name);
                close.actions = vec![Action::Click];
                close.bounds = moved(c, at);
                tab.children.push(close);
            }
            list.children.push(tab);
        }
        n.children.push(list);
    }
    n
}
