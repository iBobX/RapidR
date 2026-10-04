//! The form's accessibility tree for AccessKit (ROADMAP principle 8): every
//! component described by its role, name, value, state, bounds and the
//! actions it allows, so screen readers (VoiceOver, Narrator, Orca) read
//! and operate the program without its author doing anything. The same
//! description is what RAI will operate a running program by (Phase 5).
//!
//! Bounds are logical pixels; the window node's transform scales them to
//! the device pixels AccessKit expects.

use accesskit::{Action, ActionData, ActionRequest, Affine, Node, NodeId, Orientation, Rect as ARect, Role, TreeId, TreeInfo, TreeUpdate};

use rapidr_value::v_int;

use crate::form::{Event, Form, Kind};

const WINDOW: NodeId = NodeId(1);

/// A component's node.
fn component_id(i: usize) -> NodeId {
    NodeId(100 + i as u64)
}

/// Tab `k` of the tab control `i`.
fn tab_id(i: usize, k: usize) -> NodeId {
    NodeId(((i as u64 + 1) << 32) | k as u64)
}

fn bounds((x, y, w, h): (i64, i64, i64, i64)) -> ARect {
    ARect::new(x as f64, y as f64, (x + w) as f64, (y + h) as f64)
}

/// The whole tree (AccessKit's adapters take a full tree each time here:
/// the form is small, and it keeps the update simple).
pub fn tree(form: &Form, scale: f64) -> TreeUpdate {
    let mut nodes = Vec::new();
    let mut window = Node::new(Role::Window);
    window.set_label(form.caption.as_str());
    window.set_transform(Affine::scale(scale));
    window.set_bounds(bounds((0, 0, form.width, form.height)));
    // (an edit is labelled by the QLABEL just before it, as "Name:")
    let mut last_label: Option<NodeId> = None;
    for (i, c) in form.components.iter().enumerate() {
        let id = component_id(i);
        window.push_child(id);
        let mut node = match &c.kind {
            Kind::Label { caption } => {
                let mut n = Node::new(Role::Label);
                n.set_value(caption.as_str());
                last_label = Some(id);
                n
            }
            Kind::Button { caption } => {
                let mut n = Node::new(Role::Button);
                n.set_label(caption.as_str());
                n.add_action(Action::Click);
                n
            }
            Kind::Edit(e) => {
                let mut n = Node::new(Role::TextInput);
                n.set_value(e.model.text());
                if let Some(l) = last_label {
                    n.push_labelled_by(l);
                }
                if e.model.read_only {
                    n.set_read_only();
                }
                n.add_action(Action::SetValue);
                n.add_action(Action::ReplaceSelectedText);
                n
            }
            Kind::TrackBar(t) => {
                let mut n = Node::new(Role::Slider);
                n.set_label(c.name.as_str());
                n.set_numeric_value(t.get("position").map_or(0, |v| v.to_i64()) as f64);
                n.set_min_numeric_value(t.min.min(t.max) as f64);
                n.set_max_numeric_value(t.max.max(t.min) as f64);
                n.set_numeric_value_step(t.line_size as f64);
                n.set_numeric_value_jump(t.page_size as f64);
                n.set_orientation(if t.vertical() { Orientation::Vertical } else { Orientation::Horizontal });
                for a in [Action::Increment, Action::Decrement, Action::SetValue] {
                    n.add_action(a);
                }
                n
            }
            Kind::TabControl(t) => {
                let mut n = Node::new(Role::TabList);
                n.set_label(c.name.as_str());
                // (the tabs' own rectangles aren't public in the shared model
                // yet: each tab gets the strip above the area for now)
                let (_, dy, _, _) = t.display(c.width, c.height, &c.font);
                for (k, caption) in t.tabs.iter().enumerate() {
                    let mut tab = Node::new(Role::Tab);
                    tab.set_label(caption.as_str());
                    tab.set_selected(t.index == k as i64);
                    tab.set_bounds(bounds((c.left, c.top, c.width, dy.max(1))));
                    tab.add_action(Action::Click);
                    n.push_child(tab_id(i, k));
                    nodes.push((tab_id(i, k), tab));
                }
                n
            }
        };
        if c.focusable() {
            node.add_action(Action::Focus);
        }
        if !c.enabled {
            node.set_disabled();
        }
        node.set_bounds(bounds(c.rect()));
        nodes.push((id, node));
    }
    nodes.push((WINDOW, window));
    let focus = form.focus.map_or(WINDOW, component_id);
    TreeUpdate { nodes, tree: Some(TreeInfo::new(WINDOW)), tree_id: TreeId::ROOT, focus }
}

/// What a screen reader asked for, as the kernel's input: which component
/// and what to do.
pub enum Request {
    Focus(usize),
    Click(usize),
    /// A tab of a tab control picked.
    SelectTab(usize, usize),
    Increment(usize, bool),
    SetNumber(usize, f64),
    SetText(usize, String),
}

/// Does what a screen reader asked, as if the user had (so the program's
/// OnClick / OnChange fire as for a click or a key).
pub fn apply(form: &mut Form, req: Request) -> Vec<Event> {
    let mut events = Vec::new();
    match req {
        Request::Focus(i) => {
            if form.components[i].focusable() {
                form.set_focus(Some(i));
            }
        }
        Request::Click(i) => {
            if matches!(form.components[i].kind, Kind::Button { .. }) && form.components[i].enabled {
                events.push(Event::Click(i));
            }
        }
        Request::SelectTab(i, k) => {
            if let Kind::TabControl(t) = &mut form.components[i].kind {
                if k < t.tabs.len() && t.index != k as i64 {
                    t.set("tabindex", &v_int(k as i64));
                    events.push(Event::Change(i));
                }
            }
        }
        Request::Increment(i, up) => {
            if let Kind::TrackBar(t) = &mut form.components[i].kind {
                if t.key(if up { 39 } else { 37 }) {
                    events.push(Event::Change(i));
                }
            }
        }
        Request::SetNumber(i, v) => {
            if let Kind::TrackBar(t) = &mut form.components[i].kind {
                let before = t.position;
                t.set("position", &v_int(v.round() as i64));
                if t.position != before {
                    events.push(Event::Change(i));
                }
            }
        }
        Request::SetText(i, s) => {
            if let Kind::Edit(e) = &mut form.components[i].kind {
                if !e.model.read_only {
                    e.model.set_text(&s);
                    e.model.modified = true;
                    events.push(Event::Change(i));
                }
            }
        }
    }
    events
}

pub fn request(form: &Form, req: &ActionRequest) -> Option<Request> {
    let raw = req.target_node.0;
    if raw >> 32 != 0 {
        let (i, k) = ((raw >> 32) as usize - 1, (raw & 0xFFFF_FFFF) as usize);
        return (i < form.components.len()).then_some(Request::SelectTab(i, k));
    }
    let i = usize::try_from(raw.checked_sub(100)?).ok().filter(|i| *i < form.components.len())?;
    Some(match (req.action, &req.data) {
        (Action::Focus, _) => Request::Focus(i),
        (Action::Click, _) => Request::Click(i),
        (Action::Increment, _) => Request::Increment(i, true),
        (Action::Decrement, _) => Request::Increment(i, false),
        (Action::SetValue, Some(ActionData::NumericValue(v))) => Request::SetNumber(i, *v),
        (Action::SetValue | Action::ReplaceSelectedText, Some(ActionData::Value(s))) => Request::SetText(i, s.to_string()),
        _ => return None,
    })
}
