//! A form's accessibility tree (the kernel's `AccessNode`s) as AccessKit's:
//! each window keeps the nodes it last sent and sends only those that
//! changed; screen readers' requests come back as the kernel's
//! (target, action, value), done as the user's input would be.

use std::collections::HashMap;

use accesskit::{Action as AAction, ActionData, ActionRequest, Affine, Node, NodeId, Orientation as AOrientation, Rect as ARect, Role as ARole, TextPosition, TextSelection, Toggled, TreeId, TreeInfo, TreeUpdate};
use rapidr_ui_kernel::AccessValue;
use rapidr_value::objects::a11y::{AccessNode, Action, Orientation, Role, TextInfo, TextPos};

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
        Role::SpinButton => ARole::SpinButton,
        Role::Splitter => ARole::Splitter,
        Role::Status => ARole::Status,
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
    // (a multi-line text box is its own role in AccessKit)
    let mut a = Node::new(if n.role == Role::TextInput && n.states.multiline { ARole::MultilineTextInput } else { role(n.role) });
    if !n.name.is_empty() {
        // (AccessKit reads a label's text from its value: UIA's Name,
        // AXValue)
        if a.role() == ARole::Label && n.value.is_none() {
            a.set_value(n.name.as_str());
        } else {
            a.set_label(n.name.as_str());
        }
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
    if let Some(l) = n.level {
        a.set_level(l);
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
    // (a text field's text runs: its first children, AccessKit's text)
    if let Some(t) = &n.text {
        for r in &t.runs {
            a.push_child(NodeId(r.id));
        }
        if let Some((anchor, focus)) = t.selection {
            let at = |p: TextPos| t.runs.get(p.run).map(|r| TextPosition { node: NodeId(r.id), character_index: p.char_index.min(r.char_lengths.len()) });
            if let (Some(anchor), Some(focus)) = (at(anchor), at(focus)) {
                a.set_text_selection(TextSelection { anchor, focus });
            }
        }
    }
    for c in &n.children {
        a.push_child(NodeId(c.id));
    }
    a
}

/// A text field's runs as AccessKit's text run nodes (a long line's pieces
/// linked as one line).
fn text_runs(t: &TextInfo, out: &mut Vec<(NodeId, Node)>) {
    for (i, r) in t.runs.iter().enumerate() {
        let mut a = Node::new(ARole::TextRun);
        a.set_value(r.text.as_str());
        a.set_bounds(bounds(r.bounds));
        a.set_character_lengths(r.char_lengths.clone());
        if r.char_positions.len() == r.char_lengths.len() && r.char_widths.len() == r.char_lengths.len() && !r.char_lengths.is_empty() {
            a.set_character_positions(r.char_positions.clone());
            a.set_character_widths(r.char_widths.clone());
        }
        if !r.word_starts.is_empty() {
            a.set_word_starts(r.word_starts.clone());
        }
        if r.continues {
            if let Some(next) = t.runs.get(i + 1) {
                a.set_next_on_line(NodeId(next.id));
            }
        }
        if let Some(prev) = i.checked_sub(1).and_then(|p| t.runs.get(p)).filter(|p| p.continues) {
            a.set_previous_on_line(NodeId(prev.id));
        }
        out.push((NodeId(r.id), a));
    }
}

fn flatten(n: &AccessNode, out: &mut Vec<(NodeId, Node)>, focus: &mut Option<NodeId>) {
    if n.states.focused {
        *focus = Some(NodeId(n.id));
    }
    out.push((NodeId(n.id), node(n)));
    if let Some(t) = &n.text {
        text_runs(t, out);
    }
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

/// A window's answer to a screen reader's first question (AccessKit's
/// activation handler, called from the platform's accessibility query, on
/// whatever thread the platform asks it): the form's tree as it was when
/// its window was made, at once — rather than an empty placeholder until
/// the next pump — and a request through the event loop for the tree as
/// it is now (`InitialTreeRequested`: a full update, as before).
pub struct FirstTree {
    tree: Option<TreeUpdate>,
    window: winit::window::WindowId,
    proxy: winit::event_loop::EventLoopProxy<crate::winit_host::UserEvent>,
}

impl FirstTree {
    pub fn new(tree: Option<TreeUpdate>, window: winit::window::WindowId, proxy: winit::event_loop::EventLoopProxy<crate::winit_host::UserEvent>) -> Self {
        FirstTree { tree, window, proxy }
    }
}

impl accesskit::ActivationHandler for FirstTree {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        let asked = accesskit_winit::Event { window_id: self.window, window_event: accesskit_winit::WindowEvent::InitialTreeRequested };
        self.proxy.send_event(asked.into()).ok();
        self.tree.take()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_says_its_text_as_its_value() {
        // (AccessKit names a label from its value: UIA's Name, AXValue)
        let label = node(&AccessNode { name: "Name:".into(), ..AccessNode::new(1, Role::Label) });
        assert_eq!(label.value(), Some("Name:"));
        assert_eq!(label.label(), None);
        let button = node(&AccessNode { name: "OK".into(), ..AccessNode::new(2, Role::Button) });
        assert_eq!(button.label(), Some("OK"));
        assert_eq!(button.value(), None);
    }

    /// A code editor's node: "Dim a\r\n\tb = a + 1\n" plus a 300-character
    /// last line, the caret selecting "a + 1" backwards.
    fn editor() -> AccessNode {
        use rapidr_value::objects::a11y::{TextRun, TextPos};
        fn run(id: u64, text: &str, brk: &str, continues: bool) -> TextRun {
            let mut lengths: Vec<u8> = text.chars().map(|c| c.len_utf8() as u8).collect();
            if !brk.is_empty() {
                lengths.push(brk.len() as u8);
            }
            let n = lengths.len();
            TextRun {
                id,
                text: format!("{text}{brk}"),
                bounds: (10, 20 + id as i64 * 18, 8 * n as i64, 18),
                char_positions: (0..n).map(|i| i as f32 * 8.0).collect(),
                char_widths: vec![8.0; n],
                char_lengths: lengths,
                word_starts: vec![0],
                continues,
            }
        }
        let long = "x".repeat(300);
        let mut n = AccessNode::new(1, Role::MultilineTextInput);
        n.states.focused = true;
        n.value = Some(format!("Dim a\r\n\tb = a + 1\n{long}"));
        n.text = Some(Box::new(TextInfo {
            runs: vec![run(10, "Dim a", "\r\n", false), run(11, "\tb = a + 1", "\n", false), run(12, &long[..250], "", true), run(13, &long[250..], "", false)],
            selection: Some((TextPos { run: 1, char_index: 10 }, TextPos { run: 1, char_index: 5 })),
        }));
        n.children.push(AccessNode::new(2, Role::Status));
        let mut root = AccessNode::new(0, Role::Window);
        root.children.push(n);
        root
    }

    #[test]
    fn a_text_fields_runs_are_its_text_by_character_and_line() {
        let mut sent = Sent::default();
        let up = update(&editor(), 1.0, &mut sent, true).unwrap();
        let by_id: HashMap<NodeId, &Node> = up.nodes.iter().map(|(id, n)| (*id, n)).collect();
        // the runs are the field's first children, before its other parts
        let field = by_id[&NodeId(1)];
        assert_eq!(field.children(), &[NodeId(10), NodeId(11), NodeId(12), NodeId(13), NodeId(2)]);
        let runs: Vec<&Node> = (10..14).map(|i| by_id[&NodeId(i)]).collect();
        assert!(runs.iter().all(|r| r.role() == ARole::TextRun));
        // each run's character lengths add up to its text; joined, the value
        for r in &runs {
            let sum: usize = r.character_lengths().iter().map(|&l| l as usize).sum();
            assert_eq!(sum, r.value().unwrap().len());
            assert_eq!(r.character_positions().map(<[f32]>::len), Some(r.character_lengths().len()));
        }
        assert_eq!(runs[0].character_lengths().last(), Some(&2), "a CR LF break is one character");
        let joined: String = runs.iter().map(|r| r.value().unwrap()).collect();
        assert_eq!(Some(joined.as_str()), field.value());
        // a long line's pieces are one line
        assert_eq!(runs[2].next_on_line(), Some(NodeId(13)));
        assert_eq!(runs[3].previous_on_line(), Some(NodeId(12)));
        assert_eq!(runs[1].next_on_line(), None);
        // the selection points into the second line's run
        let sel = field.text_selection().unwrap();
        assert_eq!((sel.anchor.node, sel.anchor.character_index), (NodeId(11), 10));
        assert_eq!((sel.focus.node, sel.focus.character_index), (NodeId(11), 5));
    }

    #[test]
    fn a_screen_reader_reads_the_runs_lines_and_selection() {
        // (the tree as the platform adapters read it: AccessKit's consumer)
        let mut sent = Sent::default();
        let up = update(&editor(), 1.0, &mut sent, true).unwrap();
        let tree = accesskit_consumer::Tree::new(up, true);
        let field = tree.state().focus().unwrap();
        assert!(field.supports_text_ranges());
        let long = "x".repeat(300);
        assert_eq!(field.document_range().text(), format!("Dim a\r\n\tb = a + 1\n{long}"));
        assert_eq!(field.line_range_from_index(0).unwrap().text(), "Dim a\r\n");
        assert_eq!(field.line_range_from_index(1).unwrap().text(), "\tb = a + 1\n");
        assert_eq!(field.line_range_from_index(2).unwrap().text(), long, "the pieces read as one line");
        assert_eq!(field.text_selection().unwrap().text(), "a + 1");
        let focus = field.text_selection_focus().unwrap();
        assert_eq!(focus.to_line_index(), 1);
        assert_eq!(focus.to_global_usv_index(), "Dim a\r\n\tb = ".chars().count());
    }
}
