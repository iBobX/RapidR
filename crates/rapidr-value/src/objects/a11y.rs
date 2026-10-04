//! What a screen reader is told about a form (ROADMAP principle 8): each
//! component described as an [`AccessNode`] — its role, name, value,
//! states, the actions it allows and its bounds. The UI kernel assembles a
//! form's tree from its components and the shared models' `describe`s
//! (here, next to the models, so the web runtime can map the same nodes to
//! ARIA and RAI can read them as JSON); the desktop host turns it into
//! AccessKit's tree.
//!
//! Node ids are stable: a hash of the component's (lowercase) id, and for
//! its parts (a tab of a tab control) that hash mixed with the part's kind
//! and index — so they don't change when other components are added.
//! Bounds are logical pixels; the kernel makes them the form's.

use super::font::Font;
use super::ops::Rect;
use super::tabcontrol::TabControl;
use super::trackbar::TrackBar;

/// What a node is (AccessKit's / ARIA's roles, the ones RapidQ's
/// components need).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Window,
    Dialog,
    Pane,
    Group,
    Label,
    Button,
    CheckBox,
    RadioButton,
    TextInput,
    MultilineTextInput,
    Slider,
    ProgressIndicator,
    TabList,
    Tab,
    ListBox,
    ListBoxOption,
    ComboBox,
    Tree,
    TreeItem,
    Grid,
    Row,
    Cell,
    MenuBar,
    MenuItem,
    Image,
    Canvas,
    Unknown,
}

impl Role {
    /// Its name in the JSON dump (`RAPIDR_TEST_A11Y`), as ARIA spells it.
    pub fn name(self) -> &'static str {
        match self {
            Role::Window => "window",
            Role::Dialog => "dialog",
            Role::Pane => "pane",
            Role::Group => "group",
            Role::Label => "label",
            Role::Button => "button",
            Role::CheckBox => "checkbox",
            Role::RadioButton => "radio",
            Role::TextInput => "textbox",
            Role::MultilineTextInput => "textbox-multiline",
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
            Role::Canvas => "canvas",
            Role::Unknown => "generic",
        }
    }
}

/// What a screen reader (or RAI) may do to a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Click,
    Focus,
    SetValue,
    Increment,
    Decrement,
    Expand,
    Collapse,
    ScrollIntoView,
}

impl Action {
    pub fn name(self) -> &'static str {
        match self {
            Action::Click => "click",
            Action::Focus => "focus",
            Action::SetValue => "setValue",
            Action::Increment => "increment",
            Action::Decrement => "decrement",
            Action::Expand => "expand",
            Action::Collapse => "collapse",
            Action::ScrollIntoView => "scrollIntoView",
        }
    }
}

/// A slider's (a progress bar's) number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Numeric {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    /// An arrow key's step.
    pub step: f64,
    /// Page Up / Page Down's.
    pub jump: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct States {
    pub focused: bool,
    pub disabled: bool,
    /// A check box's / radio button's (`None`: not checkable).
    pub checked: Option<bool>,
    /// A tab's, a list row's (`None`: not selectable).
    pub selected: Option<bool>,
    /// A tree item's, a combo box's (`None`: not expandable).
    pub expanded: Option<bool>,
    pub read_only: bool,
    pub multiline: bool,
    pub modal: bool,
}

/// A node of a form's accessibility tree.
#[derive(Clone, Debug, PartialEq)]
pub struct AccessNode {
    pub id: u64,
    pub role: Role,
    /// What it's called (AccessibleName, Caption, …; see the kernel's rule).
    pub name: String,
    pub description: String,
    /// Its text (an edit's, a label's caption).
    pub value: Option<String>,
    pub numeric: Option<Numeric>,
    pub orientation: Option<Orientation>,
    /// Alt + this letter activates it (a caption's `&` mnemonic).
    pub shortcut: Option<char>,
    /// The node naming it (a QLABEL), when its name comes from one.
    pub labelled_by: Option<u64>,
    pub states: States,
    pub actions: Vec<Action>,
    /// Logical pixels (the form's client area once the kernel places it).
    pub bounds: Rect,
    pub children: Vec<AccessNode>,
}

impl AccessNode {
    pub fn new(id: u64, role: Role) -> AccessNode {
        AccessNode {
            id,
            role,
            name: String::new(),
            description: String::new(),
            value: None,
            numeric: None,
            orientation: None,
            shortcut: None,
            labelled_by: None,
            states: States::default(),
            actions: Vec::new(),
            bounds: (0, 0, 0, 0),
            children: Vec::new(),
        }
    }

    /// Moves it and its children by (dx, dy) (a component's own bounds
    /// into its form's).
    pub fn offset(&mut self, dx: i64, dy: i64) {
        self.bounds.0 += dx;
        self.bounds.1 += dy;
        for c in &mut self.children {
            c.offset(dx, dy);
        }
    }

    /// The node `id` in this tree.
    pub fn find(&self, id: u64) -> Option<&AccessNode> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find(id))
    }

    /// Every node, depth first (this one first).
    pub fn walk(&self, f: &mut dyn FnMut(&AccessNode)) {
        f(self);
        for c in &self.children {
            c.walk(f);
        }
    }

    /// The tree as JSON (what `RAPIDR_TEST_A11Y` writes): roles, names,
    /// values and states, deterministic.
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        self.json_into(&mut s);
        s
    }

    fn json_into(&self, s: &mut String) {
        s.push_str(&format!("{{\"id\":{},\"role\":\"{}\",\"name\":{}", self.id, self.role.name(), json_str(&self.name)));
        if !self.description.is_empty() {
            s.push_str(&format!(",\"description\":{}", json_str(&self.description)));
        }
        if let Some(v) = &self.value {
            s.push_str(&format!(",\"value\":{}", json_str(v)));
        }
        if let Some(n) = &self.numeric {
            s.push_str(&format!(",\"numeric\":{{\"value\":{},\"min\":{},\"max\":{},\"step\":{},\"jump\":{}}}", n.value, n.min, n.max, n.step, n.jump));
        }
        if let Some(o) = self.orientation {
            s.push_str(if o == Orientation::Vertical { ",\"orientation\":\"vertical\"" } else { ",\"orientation\":\"horizontal\"" });
        }
        if let Some(c) = self.shortcut {
            s.push_str(&format!(",\"shortcut\":{}", json_str(&format!("Alt+{c}"))));
        }
        if let Some(l) = self.labelled_by {
            s.push_str(&format!(",\"labelledBy\":{l}"));
        }
        let st = &self.states;
        let mut states = Vec::new();
        for (on, name) in [(st.focused, "focused"), (st.disabled, "disabled"), (st.read_only, "readonly"), (st.multiline, "multiline"), (st.modal, "modal")] {
            if on {
                states.push(format!("\"{name}\""));
            }
        }
        for (v, name) in [(st.checked, "checked"), (st.selected, "selected"), (st.expanded, "expanded")] {
            if let Some(v) = v {
                states.push(format!("\"{}{name}\"", if v { "" } else { "not-" }));
            }
        }
        s.push_str(&format!(",\"states\":[{}]", states.join(",")));
        let actions: Vec<String> = self.actions.iter().map(|a| format!("\"{}\"", a.name())).collect();
        s.push_str(&format!(",\"actions\":[{}]", actions.join(",")));
        let (x, y, w, h) = self.bounds;
        s.push_str(&format!(",\"bounds\":[{x},{y},{w},{h}]"));
        if !self.children.is_empty() {
            s.push_str(",\"children\":[");
            for (i, c) in self.children.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                c.json_into(s);
            }
            s.push(']');
        }
        s.push('}');
    }
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

/// The parts of a component that are nodes of their own.
pub const PART_TAB: u64 = 1;
pub const PART_ITEM: u64 = 2;
pub const PART_ROW: u64 = 3;
pub const PART_CELL: u64 = 4;
pub const PART_MENU: u64 = 5;

/// A component's stable node id: FNV-1a of its lowercase id, kept off the
/// top byte (parts mix their kind there) and never 0.
pub fn node_id(component: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in component.to_lowercase().bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (h & 0x00FF_FFFF_FFFF_FFFF).max(1)
}

/// Part `index` of kind `kind` (`PART_TAB` …) of a component.
pub fn part_id(component: &str, kind: u64, index: usize) -> u64 {
    node_id(component) ^ (kind << 56 | index as u64 & 0x00FF_FFFF_FFFF_FFFF)
}

/// The QLABEL that names a control without a name of its own, as people
/// read a form: the nearest label to its left on the same line, else the
/// nearest above it overlapping its columns (`labels`: the same parent's
/// QLABELs' rectangles). Its index in `labels`.
pub fn nearest_label(target: Rect, labels: &[Rect]) -> Option<usize> {
    label_left_of(target, labels).or_else(|| label_above(target, labels))
}

/// The nearest label starting to the left of `target`, its vertical span
/// overlapping the target's. (A label's box is often wider than its
/// caption — QLABEL's default is 75 pixels — so where it starts counts,
/// not where it ends.)
pub fn label_left_of(target: Rect, labels: &[Rect]) -> Option<usize> {
    let (tx, ty, _, th) = target;
    labels.iter().enumerate().filter(|(_, &(x, y, _, h))| x < tx && y < ty + th && y + h > ty).max_by_key(|(_, &(x, ..))| x).map(|(i, _)| i)
}

/// The nearest label starting above `target`, its horizontal span
/// overlapping the target's.
pub fn label_above(target: Rect, labels: &[Rect]) -> Option<usize> {
    let (tx, ty, tw, _) = target;
    labels.iter().enumerate().filter(|(_, &(x, y, w, _))| y < ty && x < tx + tw && x + w > tx).max_by_key(|(_, &(_, y, ..))| y).map(|(i, _)| i)
}

/// A caption's mnemonic: the text shown (`&&` an ampersand, a lone `&`
/// dropped), and the character index and letter a `&` marks (underlined;
/// Alt + it activates).
pub fn mnemonic(caption: &str) -> (String, Option<(usize, char)>) {
    let mut shown = String::new();
    let mut mark = None;
    let mut chars = caption.chars().peekable();
    let mut n = 0;
    while let Some(c) = chars.next() {
        if c == '&' {
            match chars.next() {
                Some('&') => {
                    shown.push('&');
                    n += 1;
                }
                Some(next) => {
                    if mark.is_none() {
                        mark = Some((n, next.to_lowercase().next().unwrap_or(next)));
                    }
                    shown.push(next);
                    n += 1;
                }
                None => {}
            }
        } else {
            shown.push(c);
            n += 1;
        }
    }
    (shown, mark)
}

impl TrackBar {
    /// A slider for a `w` × `h` QTRACKBAR (bounds the control's own).
    pub fn describe(&self, id: u64, w: i64, h: i64) -> AccessNode {
        let mut n = AccessNode::new(id, Role::Slider);
        n.numeric = Some(Numeric {
            value: self.get("position").map_or(0, |v| v.to_i64()) as f64,
            min: self.min.min(self.max) as f64,
            max: self.max.max(self.min) as f64,
            step: self.line_size as f64,
            jump: self.page_size as f64,
        });
        n.orientation = Some(if self.vertical() { Orientation::Vertical } else { Orientation::Horizontal });
        n.actions = vec![Action::Increment, Action::Decrement, Action::SetValue];
        n.bounds = (0, 0, w, h);
        n
    }
}

impl TabControl {
    /// A tab list for a `w` × `h` QTABCONTROL in `font`, a tab node per
    /// tab (ids: `part_id(component, PART_TAB, i)`), each where it's drawn.
    pub fn describe(&self, component: &str, w: i64, h: i64, font: &Font) -> AccessNode {
        let mut n = AccessNode::new(node_id(component), Role::TabList);
        n.bounds = (0, 0, w, h);
        for (i, caption) in self.tabs.iter().enumerate() {
            let mut tab = AccessNode::new(part_id(component, PART_TAB, i), Role::Tab);
            let (shown, mark) = mnemonic(caption);
            tab.name = shown;
            tab.shortcut = mark.map(|m| m.1);
            tab.states.selected = Some(self.index == i as i64);
            tab.actions = vec![Action::Click];
            match self.tab_rect(i, w, h, font) {
                Some(r) => tab.bounds = r,
                None => tab.actions.push(Action::ScrollIntoView),
            }
            n.children.push(tab);
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_int, v_str};

    #[test]
    fn ids_are_stable_and_distinct() {
        assert_eq!(node_id("Button1"), node_id("button1"));
        assert_ne!(node_id("button1"), node_id("button2"));
        assert_ne!(part_id("tabs", PART_TAB, 0), part_id("tabs", PART_TAB, 1));
        assert_ne!(part_id("tabs", PART_TAB, 0), node_id("tabs"));
        assert_eq!(node_id("x") >> 56, 0);
    }

    #[test]
    fn the_nearest_label_names_a_control() {
        // "Name:" left of the edit, "Title" above it, "Far" further left
        let labels = [(8, 12, 40, 16), (56, 0, 60, 14), (0, 10, 4, 16)];
        assert_eq!(nearest_label((56, 8, 120, 25), &labels), Some(0));
        // nothing on the left: the one above
        assert_eq!(nearest_label((56, 20, 120, 25), &labels[1..2]), Some(0));
        assert_eq!(nearest_label((200, 200, 10, 10), &labels), None);
    }

    #[test]
    fn mnemonics() {
        assert_eq!(mnemonic("&File"), ("File".to_string(), Some((0, 'f'))));
        assert_eq!(mnemonic("Save && E&xit"), ("Save & Exit".to_string(), Some((8, 'x'))));
        assert_eq!(mnemonic("plain"), ("plain".to_string(), None));
    }

    #[test]
    fn models_describe_themselves() {
        let mut t = TrackBar::default();
        t.set("position", &v_int(3));
        let n = t.describe(9, 150, 45);
        assert_eq!((n.role, n.numeric.unwrap().value, n.numeric.unwrap().max), (Role::Slider, 3.0, 10.0));
        let mut tc = TabControl::default();
        tc.call("addtabs", &[v_str("&One"), v_str("Two")]);
        let n = tc.describe("tc", 150, 100, &Font::default());
        assert_eq!(n.children.len(), 2);
        assert_eq!((n.children[0].name.as_str(), n.children[0].states.selected, n.children[0].shortcut), ("One", Some(true), Some('o')));
        assert!(n.children[1].bounds.0 > n.children[0].bounds.0);
        let json = n.to_json();
        assert!(json.contains("\"role\":\"tab\",\"name\":\"Two\"") && json.contains("\"not-selected\""), "{json}");
    }
}
