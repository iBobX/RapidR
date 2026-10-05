//! The accessibility mirror: a form's `AccessNode` tree (the kernel's,
//! made by the Stage 12 rules in `rapidr_value::objects::a11y` — the very
//! tree the desktop host hands AccessKit) as invisible DOM elements over
//! the canvas, with ARIA roles, names, values and states, so a screen
//! reader in the browser gets the tree a desktop screen reader gets.
//!
//! This module only *describes* the elements ([`mirror`]): one per node,
//! nested as the nodes are, placed at the node's bounds (logical pixels =
//! CSS pixels, relative to the parent's element). The page creates and
//! patches them (`tests/web_host_spike.js`). Elements are chosen so the
//! browser's own semantics say what the node says:
//!
//! - a text box is a real `<input>` / `<textarea>` (its value the text) —
//!   it is also where the keyboard, input methods and mobile keyboards type
//!   while it has the focus;
//! - a label is its text (Chrome's StaticText), a canvas a `<canvas>`;
//! - everything else a `<div>` with the ARIA role the node's role names
//!   (a form, which ARIA has no window for, is a dialog; a pane a group)
//!   and aria-checked / pressed / selected / expanded / disabled /
//!   readonly / level / valuenow … from its states.
//!
//! Focusable nodes get `tabindex=-1`: the page moves the DOM focus to the
//! kernel's focused node, so screen readers announce what the kernel
//! focused (Tab itself is the kernel's).

use rapidr_value::objects::a11y::{AccessNode, Orientation, Role};

/// The ARIA role for a kernel role (`None`: the element's own).
pub fn aria_role(role: Role) -> Option<&'static str> {
    Some(match role {
        // (ARIA has no window: a form is a dialog, as the DOM runtime's)
        Role::Window | Role::Dialog => "dialog",
        Role::Pane | Role::Group => "group",
        Role::Label | Role::TextInput | Role::MultilineTextInput | Role::Canvas | Role::Unknown => return None,
        Role::Button => "button",
        Role::CheckBox => "checkbox",
        Role::RadioButton => "radio",
        Role::Slider => "slider",
        Role::ProgressIndicator => "progressbar",
        Role::TabList => "tablist",
        Role::Tab => "tab",
        Role::ListBox => "listbox",
        Role::ListBoxOption => "option",
        Role::ComboBox => "combobox",
        Role::Tree => "tree",
        Role::TreeItem => "treeitem",
        Role::Grid => "grid",
        Role::Row => "row",
        Role::Cell => "gridcell",
        Role::MenuBar => "menubar",
        Role::MenuItem => "menuitem",
        Role::Image => "img",
        Role::SpinButton => "spinbutton",
        Role::Splitter => "separator",
        Role::Status => "status",
    })
}

/// Whether the node takes the DOM focus when the kernel focuses it.
fn focusable(n: &AccessNode) -> bool {
    !matches!(n.role, Role::Window | Role::Dialog | Role::Pane | Role::Group | Role::Label | Role::Canvas | Role::Image | Role::Status | Role::Unknown)
}

fn tag(n: &AccessNode) -> &'static str {
    match n.role {
        Role::MultilineTextInput => "textarea",
        Role::TextInput if n.states.multiline => "textarea",
        Role::TextInput => "input",
        // (a combo box's value is its text: a text field's, as the browser
        // reads one; its list's elements beside it — an input holds none)
        Role::ComboBox => "input",
        Role::Canvas => "canvas",
        _ => "div",
    }
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

/// The ARIA attributes node `n` needs (its role's, its name, its states).
pub fn attributes(n: &AccessNode) -> Vec<(&'static str, String)> {
    let mut a = Vec::new();
    if let Some(r) = aria_role(n.role) {
        a.push(("role", r.to_string()));
    }
    let st = &n.states;
    if n.role != Role::Label && !n.name.is_empty() {
        a.push(("aria-label", n.name.clone()));
    }
    if !n.description.is_empty() {
        a.push(("aria-description", n.description.clone()));
    }
    if let Some(c) = n.shortcut {
        a.push(("aria-keyshortcuts", format!("Alt+{}", c.to_ascii_uppercase())));
    }
    if st.disabled {
        a.push(("aria-disabled", "true".into()));
    }
    if st.read_only && matches!(n.role, Role::TextInput | Role::MultilineTextInput | Role::Grid | Role::ComboBox) {
        a.push(("aria-readonly", "true".into()));
    }
    if st.modal {
        a.push(("aria-modal", "true".into()));
    }
    if let Some(c) = st.checked {
        // (a toggle button is pressed, not checked)
        a.push((if n.role == Role::Button { "aria-pressed" } else { "aria-checked" }, yes_no(c).into()));
    }
    if let Some(s) = st.selected {
        a.push(("aria-selected", yes_no(s).into()));
    }
    if let Some(e) = st.expanded {
        a.push(("aria-expanded", yes_no(e).into()));
    }
    if let Some(l) = n.level {
        a.push(("aria-level", l.to_string()));
    }
    if let Some(num) = &n.numeric {
        a.push(("aria-valuenow", num.value.to_string()));
        a.push(("aria-valuemin", num.min.to_string()));
        a.push(("aria-valuemax", num.max.to_string()));
    }
    if n.role == Role::ProgressIndicator {
        if let Some(v) = &n.value {
            a.push(("aria-valuetext", v.clone()));
        }
    }
    if let Some(o) = n.orientation.filter(|_| matches!(n.role, Role::Slider | Role::Splitter | Role::ListBox | Role::TabList | Role::MenuBar)) {
        a.push(("aria-orientation", if o == Orientation::Vertical { "vertical" } else { "horizontal" }.into()));
    }
    if n.role == Role::Status {
        a.push(("aria-live", "polite".into()));
        a.push(("aria-atomic", "true".into()));
    }
    a
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One element of the mirror (a node of the kernel's tree).
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    /// The node's id (stable: a hash of the component's id).
    pub id: u64,
    pub parent: Option<u64>,
    /// `div`, `input`, `textarea` or `canvas`.
    pub tag: &'static str,
    pub attrs: Vec<(&'static str, String)>,
    /// A label's text.
    pub text: String,
    /// A text box's value.
    pub value: String,
    /// Relative to the parent's element (logical = CSS pixels).
    pub bounds: (i64, i64, i64, i64),
    /// It takes the DOM focus when the kernel focuses it.
    pub focusable: bool,
    pub focused: bool,
}

/// The mirror's elements for a form's tree, parents first.
pub fn specs(root: &AccessNode) -> Vec<Spec> {
    let mut out = Vec::new();
    walk(root, None, (0, 0), &mut out);
    out
}

fn walk(n: &AccessNode, parent: Option<u64>, origin: (i64, i64), out: &mut Vec<Spec>) {
    let (x, y, w, h) = n.bounds;
    // (a label is its text; a text box its value)
    let text = if n.role == Role::Label { n.name.clone() } else { String::new() };
    let value = match n.role {
        Role::TextInput | Role::MultilineTextInput | Role::ComboBox => n.value.clone().unwrap_or_default(),
        _ => String::new(),
    };
    out.push(Spec {
        id: n.id,
        parent,
        tag: tag(n),
        attrs: attributes(n),
        text,
        value,
        bounds: (x - origin.0, y - origin.1, w.max(0), h.max(0)),
        focusable: focusable(n) && !n.states.disabled,
        focused: n.states.focused,
    });
    for c in &n.children {
        walk(c, Some(n.id), (x, y), out);
    }
}

/// The mirror's elements for a form's tree, parents first, as JSON: an
/// array of `{id, parent, tag, attrs: [[name, value] …], text, value,
/// bounds: [x, y, w, h], focusable, focused}` (the spike's page reads it).
pub fn mirror(root: &AccessNode) -> String {
    let items: Vec<String> = specs(root)
        .iter()
        .map(|s| {
            let attrs: Vec<String> = s.attrs.iter().map(|(k, v)| format!("[{},{}]", json_str(k), json_str(v))).collect();
            let (x, y, w, h) = s.bounds;
            format!(
                "{{\"id\":\"{}\",\"parent\":{},\"tag\":\"{}\",\"attrs\":[{}],\"text\":{},\"value\":{},\"bounds\":[{x},{y},{w},{h}],\"focusable\":{},\"focused\":{}}}",
                s.id,
                s.parent.map_or("null".to_string(), |p| format!("\"{p}\"")),
                s.tag,
                attrs.join(","),
                json_str(&s.text),
                json_str(&s.value),
                s.focusable,
                s.focused,
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

#[cfg(test)]
mod tests {
    use rapidr_value::objects::a11y::{AccessNode, Numeric, Role};

    use super::*;

    #[test]
    fn a_slider_says_its_numbers() {
        let mut n = AccessNode::new(7, Role::Slider);
        n.name = "Level".into();
        n.numeric = Some(Numeric { value: 3.0, min: 0.0, max: 10.0, step: 1.0, jump: 2.0 });
        n.orientation = Some(Orientation::Horizontal);
        let a = attributes(&n);
        assert!(a.contains(&("role", "slider".into())));
        assert!(a.contains(&("aria-valuenow", "3".into())));
        assert!(a.contains(&("aria-valuemax", "10".into())));
        assert!(a.contains(&("aria-label", "Level".into())));
    }

    #[test]
    fn nested_bounds_are_the_parents() {
        let mut root = AccessNode::new(1, Role::Window);
        root.bounds = (0, 0, 100, 100);
        let mut tabs = AccessNode::new(2, Role::TabList);
        tabs.bounds = (10, 20, 50, 30);
        let mut tab = AccessNode::new(3, Role::Tab);
        tab.bounds = (12, 22, 20, 10);
        tabs.children.push(tab);
        root.children.push(tabs);
        let json = mirror(&root);
        assert!(json.contains("\"id\":\"3\",\"parent\":\"2\",\"tag\":\"div\""), "{json}");
        assert!(json.contains("\"bounds\":[2,2,20,10]"), "{json}");
    }
}
