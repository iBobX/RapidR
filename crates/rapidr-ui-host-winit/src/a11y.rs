//! A form's accessibility tree (the kernel's `AccessNode`s) as AccessKit's:
//! each window keeps the nodes it last sent and sends only those that
//! changed; screen readers' requests come back as the kernel's
//! (target, action, value), done as the user's input would be.

use std::collections::HashMap;

use accesskit::{Action as AAction, ActionData, ActionRequest, Affine, Live, Node, NodeId, Orientation as AOrientation, Rect as ARect, Role as ARole, TextDirection, TextPosition, TextSelection, Toggled, TreeId, TreeInfo, TreeUpdate};
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
        Role::Menu => ARole::Menu,
        Role::MenuItem => ARole::MenuItem,
        Role::MenuItemCheckBox => ARole::MenuItemCheckBox,
        Role::MenuItemRadio => ARole::MenuItemRadio,
        Role::Image => ARole::Image,
        Role::Canvas => ARole::Canvas,
        Role::SpinButton => ARole::SpinButton,
        Role::Splitter => ARole::Splitter,
        Role::Status => ARole::Status,
        Role::Document => ARole::Document,
        Role::Heading => ARole::Heading,
        Role::List => ARole::List,
        Role::ListItem => ARole::ListItem,
        Role::Table => ARole::Table,
        Role::Link => ARole::Link,
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
    // (a menu item's ShortCut, which a screen reader says after its name;
    // an item with a submenu says it has one)
    if let Some(k) = &n.accelerator {
        a.set_keyboard_shortcut(k.as_str());
    }
    if matches!(n.role, Role::MenuItem | Role::MenuItemCheckBox | Role::MenuItemRadio) && n.states.expanded.is_some() {
        a.set_has_popup(accesskit::HasPopup::Menu);
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
    // (a status node is a live region — polite, or assertive: what changes
    // in it is said without the focus moving, as the web's aria-live)
    if n.role == Role::Status {
        a.set_live(if s.assertive { Live::Assertive } else { Live::Polite });
        a.set_live_atomic();
    }
    for act in n.actions.iter().filter_map(|&x| action(x)) {
        a.add_action(act);
    }
    a.set_bounds(bounds(n.bounds));
    // (a text field's text runs: its first children, AccessKit's text)
    if let Some(t) = &n.text {
        // (the runs inherit it: without it, no character's bounds)
        a.set_text_direction(TextDirection::LeftToRight);
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
        // (a rich edit's run: its format, the text's attributes a screen
        // reader reports)
        if let Some(f) = &r.format {
            a.set_font_family(f.family.as_str());
            a.set_font_size(f.size);
            a.set_font_weight(if f.bold { 700.0 } else { 400.0 });
            if f.italic {
                a.set_italic();
            }
            let rgb = f.color.unwrap_or(0);
            let color = accesskit::Color { red: (rgb >> 16) as u8, green: (rgb >> 8) as u8, blue: rgb as u8, alpha: 255 };
            if f.underline {
                a.set_underline(accesskit::TextDecoration { style: accesskit::TextDecorationStyle::Solid, color });
            }
            if f.strike {
                a.set_strikethrough(accesskit::TextDecoration { style: accesskit::TextDecorationStyle::Solid, color });
            }
            if f.color.is_some() {
                a.set_foreground_color(color);
            }
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

/// The program's announcement (`Application.Announce`) as live regions in
/// a window's tree: the focused window's, so it is said once.
pub fn announce_into(tree: &mut AccessNode) {
    use rapidr_value::objects::a11y::{announcement, announcement_nodes};
    if let Some(a) = announcement() {
        let nodes = announcement_nodes(tree.id, &a);
        tree.children.extend(nodes);
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

    /// Application.Announce: the focused window's live regions get the text,
    /// and AccessKit's platform adapters announce it — they announce a live
    /// node added with a label, or whose label changed (accesskit_macos'
    /// event.rs; Windows' UIA_LiveRegionChanged, AT-SPI's Announcement
    /// alike), which is what is checked here, on AccessKit's own consumer:
    /// politely, at once when assertive, and the same text twice is said twice.
    #[test]
    fn an_announcement_is_a_live_region_a_screen_reader_says() {
        use accesskit_consumer::{common_filter, FilterResult, NodeRef, Tree, TreeChangeHandler};
        struct Said(Vec<(String, Live)>);
        impl TreeChangeHandler for Said {
            fn node_added(&mut self, n: &NodeRef) {
                if let (Some(l), true) = (n.label(), n.live() != Live::Off && common_filter(n) == FilterResult::Include) {
                    self.0.push((l, n.live()));
                }
            }
            fn node_updated(&mut self, old: &NodeRef, new: &NodeRef) {
                if let (Some(l), true) = (new.label(), new.live() != Live::Off && (new.label() != old.label() || new.live() != old.live())) {
                    self.0.push((l, new.live()));
                }
            }
            fn focus_moved(&mut self, _: Option<&NodeRef>, _: Option<&NodeRef>) {}
            fn node_removed(&mut self, _: &NodeRef) {}
        }
        let window = || {
            let mut w = AccessNode::new(100, Role::Window);
            w.bounds = (0, 0, 300, 200);
            w.children.push(AccessNode { name: "Run".into(), bounds: (10, 10, 80, 24), ..AccessNode::new(101, Role::Button) });
            w
        };
        let mut sent = Sent::default();
        let mut first = window();
        announce_into(&mut first);
        let mut tree = Tree::new(update(&first, 1.0, &mut sent, true).unwrap(), true);
        for (text, assertive) in [("3 rows found", false), ("3 rows found", false), ("The connection was lost", true)] {
            rapidr_value::objects::a11y::announce(text, assertive).expect("said");
            let mut w = window();
            announce_into(&mut w);
            assert_eq!(w.children.len(), 3, "the button and the two live regions");
            let mut said = Said(Vec::new());
            tree.update_and_process_changes(update(&w, 1.0, &mut sent, false).expect("changed"), &mut said);
            assert_eq!(said.0, vec![(text.to_string(), if assertive { Live::Assertive } else { Live::Polite })], "{text}");
        }
        // (nothing new: nothing said)
        let mut w = window();
        announce_into(&mut w);
        assert!(update(&w, 1.0, &mut sent, false).is_none());
    }

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

    #[test]
    fn menus_reach_accesskit_with_their_roles_states_and_keys() {
        // (an open menu: Menu > items; a check mark, a radio dot, a disabled
        // item, a submenu, the ShortCut)
        let mut menu = AccessNode::new(1, Role::Menu);
        menu.name = "File".into();
        let mut check = AccessNode::new(2, Role::MenuItemCheckBox);
        check.name = "Word wrap".into();
        check.states.checked = Some(true);
        check.accelerator = Some("Ctrl+W".into());
        let mut radio = AccessNode::new(3, Role::MenuItemRadio);
        radio.name = "Beginner".into();
        radio.states.checked = Some(false);
        radio.states.disabled = true;
        let mut sub = AccessNode::new(4, Role::MenuItem);
        sub.name = "Recent".into();
        sub.states.expanded = Some(false);
        menu.children = vec![check, radio, sub];
        let mut root = AccessNode::new(0, Role::Window);
        root.children.push(menu);
        let mut sent = Sent::default();
        let up = update(&root, 1.0, &mut sent, true).unwrap();
        let by_id: HashMap<NodeId, &Node> = up.nodes.iter().map(|(id, n)| (*id, n)).collect();
        let m = by_id[&NodeId(1)];
        assert_eq!((m.role(), m.label()), (ARole::Menu, Some("File")));
        assert_eq!(m.children(), &[NodeId(2), NodeId(3), NodeId(4)]);
        let (c, r, s) = (by_id[&NodeId(2)], by_id[&NodeId(3)], by_id[&NodeId(4)]);
        assert_eq!((c.role(), c.toggled(), c.keyboard_shortcut()), (ARole::MenuItemCheckBox, Some(Toggled::True), Some("Ctrl+W")));
        assert_eq!((r.role(), r.toggled(), r.is_disabled()), (ARole::MenuItemRadio, Some(Toggled::False), true));
        assert_eq!((s.role(), s.is_expanded(), s.has_popup()), (ARole::MenuItem, Some(false), Some(accesskit::HasPopup::Menu)));
        assert_eq!(c.has_popup(), None);
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
                format: None,
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

    /// A rich edit's run carries its format: the text's attributes a
    /// screen reader reports.
    #[test]
    fn a_rich_runs_format_is_its_texts_attributes() {
        use rapidr_value::objects::a11y::TextFormat;
        let mut root = editor();
        let f = TextFormat { family: "Courier New".into(), size: 12.0, bold: true, italic: true, underline: true, strike: false, color: Some(0xFF0000) };
        root.children[0].text.as_mut().unwrap().runs[0].format = Some(f);
        let mut sent = Sent::default();
        let up = update(&root, 1.0, &mut sent, true).unwrap();
        let r = up.nodes.iter().find(|(id, _)| *id == NodeId(10)).unwrap().1.clone();
        assert_eq!((r.font_family(), r.font_size(), r.font_weight(), r.is_italic()), (Some("Courier New"), Some(12.0), Some(700.0), true));
        assert_eq!(r.foreground_color(), Some(accesskit::Color { red: 255, green: 0, blue: 0, alpha: 255 }));
        assert!(r.underline().is_some() && r.strikethrough().is_none());
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
        let sel = field.text_selection().unwrap();
        assert_eq!(sel.text(), "a + 1");
        // where it's drawn: characters 5 to 10 of the run at (10, 218), 8 wide
        let boxes = sel.bounding_boxes();
        assert_eq!(boxes.len(), 1);
        assert_eq!((boxes[0].x0, boxes[0].x1, boxes[0].y0), (10.0 + 40.0, 10.0 + 80.0, 218.0));
        let focus = field.text_selection_focus().unwrap();
        assert_eq!(focus.to_line_index(), 1);
        assert_eq!(focus.to_global_usv_index(), "Dim a\r\n\tb = ".chars().count());
    }
}
