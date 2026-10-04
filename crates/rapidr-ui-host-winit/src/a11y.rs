//! A form's accessibility tree (the kernel's `AccessNode`s) as AccessKit's:
//! each window keeps the nodes it last sent and sends only those that
//! changed; screen readers' requests come back as the kernel's
//! (target, action, value), done as the user's input would be.

use std::collections::HashMap;

use accesskit::{Action as AAction, ActionData, ActionRequest, Affine, Node, NodeId, Orientation as AOrientation, Rect as ARect, Role as ARole, Toggled, TreeId, TreeInfo, TreeUpdate};
use rapidr_ui_kernel::AccessValue;
use rapidr_value::objects::a11y::{AccessNode, Action, Orientation, Role};

fn role(r: Role) -> ARole {
    match r {
        Role::Window => ARole::Window,
        Role::Dialog => ARole::Dialog,
        Role::Pane => ARole::Pane,
        Role::Group => ARole::Group,
        Role::Label => ARole::Label,
        Role::Button => ARole::Button,
        Role::CheckBox => ARole::CheckBox,
        Role::RadioButton => ARole::RadioButton,
        Role::TextInput => ARole::TextInput,
        Role::MultilineTextInput => ARole::MultilineTextInput,
        Role::Slider => ARole::Slider,
        Role::ProgressIndicator => ARole::ProgressIndicator,
        Role::TabList => ARole::TabList,
        Role::Tab => ARole::Tab,
        Role::ListBox => ARole::ListBox,
        Role::ListBoxOption => ARole::ListBoxOption,
        Role::ComboBox => ARole::ComboBox,
        Role::Tree => ARole::Tree,
        Role::TreeItem => ARole::TreeItem,
        Role::Grid => ARole::Grid,
        Role::Row => ARole::Row,
        Role::Cell => ARole::Cell,
        Role::MenuBar => ARole::MenuBar,
        Role::MenuItem => ARole::MenuItem,
        Role::Image => ARole::Image,
        Role::Canvas => ARole::Canvas,
        Role::Unknown => ARole::Unknown,
    }
}

fn action(a: Action) -> Option<AAction> {
    Some(match a {
        Action::Click => AAction::Click,
        Action::Focus => AAction::Focus,
        Action::SetValue => AAction::SetValue,
        Action::Increment => AAction::Increment,
        Action::Decrement => AAction::Decrement,
        Action::Expand => AAction::Expand,
        Action::Collapse => AAction::Collapse,
        Action::ScrollIntoView => AAction::ScrollIntoView,
    })
}

fn bounds((x, y, w, h): (i64, i64, i64, i64)) -> ARect {
    ARect::new(x as f64, y as f64, (x + w) as f64, (y + h) as f64)
}

fn node(n: &AccessNode) -> Node {
    let mut a = Node::new(role(n.role));
    if !n.name.is_empty() {
        a.set_label(n.name.as_str());
    }
    if !n.description.is_empty() {
        a.set_description(n.description.as_str());
    }
    if let Some(v) = &n.value {
        a.set_value(v.as_str());
    }
    if let Some(num) = &n.numeric {
        a.set_numeric_value(num.value);
        a.set_min_numeric_value(num.min);
        a.set_max_numeric_value(num.max);
        a.set_numeric_value_step(num.step);
        a.set_numeric_value_jump(num.jump);
    }
    if let Some(o) = n.orientation {
        a.set_orientation(if o == Orientation::Vertical { AOrientation::Vertical } else { AOrientation::Horizontal });
    }
    if let Some(c) = n.shortcut {
        a.set_access_key(format!("Alt+{c}"));
    }
    if let Some(l) = n.labelled_by {
        a.push_labelled_by(NodeId(l));
    }
    let s = &n.states;
    if s.disabled {
        a.set_disabled();
    }
    if s.read_only {
        a.set_read_only();
    }
    if s.modal {
        a.set_modal();
    }
    if let Some(c) = s.checked {
        a.set_toggled(if c { Toggled::True } else { Toggled::False });
    }
    if let Some(v) = s.selected {
        a.set_selected(v);
    }
    if let Some(v) = s.expanded {
        a.set_expanded(v);
    }
    for act in n.actions.iter().filter_map(|&x| action(x)) {
        a.add_action(act);
    }
    a.set_bounds(bounds(n.bounds));
    for c in &n.children {
        a.push_child(NodeId(c.id));
    }
    a
}

fn flatten(n: &AccessNode, out: &mut Vec<(NodeId, Node)>, focus: &mut Option<NodeId>) {
    if n.states.focused {
        *focus = Some(NodeId(n.id));
    }
    out.push((NodeId(n.id), node(n)));
    for c in &n.children {
        flatten(c, out, focus);
    }
}

/// What a window last told AccessKit.
#[derive(Default)]
pub struct Sent {
    nodes: HashMap<NodeId, Node>,
    focus: Option<NodeId>,
    scale: f64,
}

/// The update bringing AccessKit from what `sent` holds to `tree` (all of
/// it the first time; then the nodes that changed). `None`: nothing did.
pub fn update(tree: &AccessNode, scale: f64, sent: &mut Sent, full: bool) -> Option<TreeUpdate> {
    let mut nodes = Vec::new();
    let mut focus = None;
    flatten(tree, &mut nodes, &mut focus);
    let root = NodeId(tree.id);
    // (bounds are logical: the window's node scales them to device pixels)
    if let Some((_, w)) = nodes.iter_mut().find(|(id, _)| *id == root) {
        w.set_transform(Affine::scale(scale));
    }
    let focus = focus.unwrap_or(root);
    let first = full || sent.nodes.is_empty() || sent.scale != scale;
    let changed: Vec<(NodeId, Node)> = if first { nodes.clone() } else { nodes.iter().filter(|(id, n)| sent.nodes.get(id) != Some(n)).cloned().collect() };
    if changed.is_empty() && sent.focus == Some(focus) && !first {
        return None;
    }
    sent.nodes = nodes.into_iter().collect();
    sent.focus = Some(focus);
    sent.scale = scale;
    Some(TreeUpdate { nodes: changed, tree: first.then(|| TreeInfo::new(root)), tree_id: TreeId::ROOT, focus })
}

/// An update changing nothing (what a window sends when asked and nothing
/// changed).
pub fn unchanged(sent: &Sent) -> TreeUpdate {
    TreeUpdate { nodes: Vec::new(), tree: None, tree_id: TreeId::ROOT, focus: sent.focus.unwrap_or(NodeId(0)) }
}

/// A screen reader's request as the kernel's: (target node, action, value).
pub fn request(req: &ActionRequest) -> Option<(u64, Action, Option<AccessValue>)> {
    let target = req.target_node.0;
    Some(match (req.action, &req.data) {
        (AAction::Focus, _) => (target, Action::Focus, None),
        (AAction::Click, _) => (target, Action::Click, None),
        (AAction::Increment, _) => (target, Action::Increment, None),
        (AAction::Decrement, _) => (target, Action::Decrement, None),
        (AAction::Expand, _) => (target, Action::Expand, None),
        (AAction::Collapse, _) => (target, Action::Collapse, None),
        (AAction::ScrollIntoView, _) => (target, Action::ScrollIntoView, None),
        (AAction::SetValue, Some(ActionData::NumericValue(v))) => (target, Action::SetValue, Some(AccessValue::Number(*v))),
        (AAction::SetValue | AAction::ReplaceSelectedText, Some(ActionData::Value(s))) => (target, Action::SetValue, Some(AccessValue::Text(s.to_string()))),
        _ => return None,
    })
}
