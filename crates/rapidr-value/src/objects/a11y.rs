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
//!
//! The rules are here too, so both hosts apply the same ones (plan §6,
//! Stage 12): what a component of each type is ([`role_of`]), what it says
//! about itself from its properties and its model ([`describe`]), the name
//! rule ([`apply_name_rule`], [`label_for`]), which components take the
//! focus and in what order Tab visits them ([`takes_focus`],
//! [`tab_order`]) and what Alt + a caption's `&` letter does
//! ([`mnemonic_of`], [`mnemonic_clicks`]). The kernel adds what only it
//! knows (where its rows are drawn, a combo box's open list); the web maps
//! the nodes onto its elements' ARIA (`rapidr-runtime-web`'s a11y_web.rs).

use super::font::Font;
use super::ops::Rect;
use super::tabcontrol::TabControl;
use super::trackbar::TrackBar;
use crate::Value;

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
    /// A QUPDOWN (a number with up / down arrows).
    SpinButton,
    /// A QSPLITTER's bar.
    Splitter,
    /// A QSTATUSBAR (a polite live region: what it says is announced).
    Status,
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
            Role::SpinButton => "spinbutton",
            Role::Splitter => "separator",
            Role::Status => "status",
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
    /// A tree item's depth (1: a top-level node).
    pub level: Option<usize>,
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
            level: None,
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
        if let Some(l) = self.level {
            s.push_str(&format!(",\"level\":{l}"));
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
/// A grid's in-place editor.
pub const PART_EDITOR: u64 = 6;
/// A dropped-down list (a grid's gcsList column) and its items.
pub const PART_POPUP: u64 = 7;

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

/// A component's property as the rules read it (`Value::Null` when it has
/// none: RapidQ's default then). The kernel reads its store, the web its
/// component registry.
pub type Props<'a> = &'a dyn Fn(&str) -> Value;

/// A true / false property as RapidQ's runtimes keep them (-1 / 0,
/// booleans, "0" / "False" strings).
pub fn flag(get: Props, prop: &str, default: bool) -> bool {
    match get(prop) {
        Value::Null => default,
        Value::String(s) => {
            let s = s.trim();
            !(s.is_empty() || s == "0" || s.eq_ignore_ascii_case("false"))
        }
        v => v.to_bool(),
    }
}

fn int(get: Props, prop: &str, default: i64) -> i64 {
    match get(prop) {
        Value::Null => default,
        v => v.to_i64(),
    }
}

fn text(get: Props, prop: &str) -> String {
    match get(prop) {
        Value::Null => String::new(),
        v => v.to_string_val(),
    }
}

/// What a component of RapidR type `type_name` (`RBUTTON` …) is.
pub fn role_of(type_name: &str) -> Role {
    match type_name.to_ascii_uppercase().as_str() {
        "RFORM" | "RMDICHILD" => Role::Window,
        "RBUTTON" | "RCOOLBTN" | "ROVALBTN" => Role::Button,
        "RLABEL" => Role::Label,
        "REDIT" | "RMEMO" | "RRICHEDIT" | "RCODEEDITOR" => Role::TextInput,
        "RCHECKBOX" => Role::CheckBox,
        "RRADIOBUTTON" => Role::RadioButton,
        "RCOMBOBOX" => Role::ComboBox,
        // (the IDE's designer: its designed components as options)
        "RLISTBOX" | "RFILELISTBOX" | "RLISTVIEW" | "RDESIGNSURFACE" => Role::ListBox,
        "RTREEVIEW" | "RDIRTREE" => Role::Tree,
        "RSTRINGGRID" => Role::Grid,
        "RTABCONTROL" => Role::TabList,
        // (a QSCROLLBAR too: a value between Min and Max the user moves)
        "RTRACKBAR" | "RSCROLLBAR" => Role::Slider,
        "RUPDOWN" => Role::SpinButton,
        "RPROGRESS" | "RPROGRESSBAR" => Role::ProgressIndicator,
        "RGROUPBOX" | "RHEADER" => Role::Group,
        // (I1: a dock manager's groups and its document area — rapidr_value::dock)
        "RDOCKGROUP" | "RDOCKDOCS" => Role::Group,
        // (I1 / L-PANELS: RapidR Studio's panels — rapidr_value::panels; the
        // kernel describes their rows, tabs and buttons)
        "RPROPERTYINSPECTOR" => Role::Grid,
        "RTOOLBOX" | "RPROJECTTREE" => Role::Tree,
        "ROUTPUTCONSOLE" => Role::MultilineTextInput,
        "RTOOLBAR" => Role::Group,
        "RCOMMANDPALETTE" => Role::Dialog,
        "RSTATUSBAR" => Role::Status,
        "RSPLITTER" => Role::Splitter,
        // (and a kernel-drawn message box's icon)
        "RIMAGE" | "RDLGPART" => Role::Image,
        // (QDIGDISPLAY: a picture of its Display, named by it)
        "RDIGDISPLAY" => Role::Image,
        "RCANVAS" | "RDXSCREEN" => Role::Canvas,
        "RMAINMENU" => Role::MenuBar,
        // (QPANEL, QSCROLLBOX and what the hosts only place)
        _ => Role::Pane,
    }
}

/// Whether a component of `type_name` takes the keyboard focus when it's
/// shown and enabled (Windows': labels, panels, pictures, a cool button
/// don't; a click on one leaves the focus where it was).
pub fn takes_focus(type_name: &str) -> bool {
    matches!(
        type_name.to_ascii_uppercase().as_str(),
        "RBUTTON" | "REDIT" | "RMEMO" | "RRICHEDIT" | "RCODEEDITOR" | "RCHECKBOX" | "RRADIOBUTTON" | "RCOMBOBOX" | "RLISTBOX" | "RFILELISTBOX" | "RLISTVIEW" | "RTREEVIEW" | "RDIRTREE" | "RSTRINGGRID" | "RTABCONTROL" | "RTRACKBAR" | "RUPDOWN"
            | "RSCROLLBAR"
        // (I1 / L-PANELS: RapidR Studio's panels)
        | "RPROPERTYINSPECTOR" | "RTOOLBOX" | "RPROJECTTREE" | "ROUTPUTCONSOLE" | "RCOMMANDPALETTE"
    )
}

/// The letter a component's caption marks with `&` (Alt + it), for the
/// kinds that have one: buttons, check boxes, radio buttons, labels.
pub fn mnemonic_of(type_name: &str, get: Props) -> Option<char> {
    let t = type_name.to_ascii_uppercase();
    if !(mnemonic_clicks(&t) || t == "RLABEL") {
        return None;
    }
    mnemonic(&caption_of(&t, get)).1.map(|m| m.1)
}

/// What Alt + a component's letter does: `true` clicks it (a button, a
/// check box …); a label's focuses the component after it in Tab order.
pub fn mnemonic_clicks(type_name: &str) -> bool {
    matches!(type_name.to_ascii_uppercase().as_str(), "RBUTTON" | "RCHECKBOX" | "RRADIOBUTTON" | "RCOOLBTN" | "ROVALBTN")
}

/// The caption a component shows: a QBUTTON without one shows its Kind's.
pub fn caption_of(type_name: &str, get: Props) -> String {
    let caption = text(get, "caption");
    if caption.is_empty() && type_name.eq_ignore_ascii_case("RBUTTON") {
        return crate::events::button_kind(int(get, "kind", 0)).map_or(String::new(), |k| k.0.to_string());
    }
    caption
}

/// The QSTATUSBAR's texts: a panel's each, or its SimpleText alone with
/// SimplePanel; no panels and SimplePanel False (the default) is one empty
/// box — VCL's TStatusBar shows SimpleText only with SimplePanel (RC.EXE:
/// `SimpleText = "Ready"` leaves SimplePanel 0, and the bar shows nothing).
pub fn status_texts(get: Props) -> Vec<String> {
    let count = int(get, "panelcount", 0).clamp(0, 256);
    if flag(get, "simplepanel", false) {
        return vec![text(get, "simpletext")];
    }
    if count == 0 {
        return vec![String::new()];
    }
    (0..count).map(|i| text(get, &format!("panel({i}).caption"))).collect()
}

/// A status bar's panel `k`'s node id.
pub fn status_panel_id(component: &str, k: usize) -> u64 {
    node_id(&format!("{component}.panel({k})"))
}

/// The most rows × columns a grid describes (a screen reader reads a
/// grid's cells; one this big is cut).
pub const MOST_CELLS: usize = 10_000;
/// The most items a list describes.
pub const MOST_ITEMS: usize = 1_000;

/// A component's own node, from its properties (`get`) and its shared model
/// (keyed by `id`): its role, name (its caption, `&` dropped), shortcut,
/// value and states, and its parts — a list's rows, a tab control's tabs, a
/// tree's items (every node with a row: its parents expanded), a grid's
/// rows and cells, a status bar's panels — with ids as `part_id` gives them.
/// Bounds are its own (`size`) and its parts' where the model knows them
/// (tabs, header sections); each host places the rest. The name rule's
/// other steps come after ([`apply_name_rule`]).
pub fn describe(id: &str, type_name: &str, get: Props, size: (i64, i64), font: &Font) -> AccessNode {
    let t = type_name.to_ascii_uppercase();
    let (w, h) = size;
    let mut n = AccessNode::new(node_id(id), role_of(&t));
    n.bounds = (0, 0, w, h);
    let own_caption = |n: &mut AccessNode, shortcut: bool| {
        let (shown, mark) = mnemonic(&caption_of(&t, get));
        n.name = shown;
        if shortcut {
            n.shortcut = mark.map(|m| m.1);
        }
    };
    match t.as_str() {
        "RBUTTON" => {
            own_caption(&mut n, true);
            n.actions = vec![Action::Click, Action::Focus];
        }
        // (a toggle button: pressed or not, once it's in a group)
        "RCOOLBTN" | "ROVALBTN" => {
            own_caption(&mut n, true);
            if int(get, "groupindex", 0) != 0 {
                n.states.checked = Some(flag(get, "down", false));
            }
            n.actions = vec![Action::Click];
        }
        "RCHECKBOX" | "RRADIOBUTTON" => {
            own_caption(&mut n, true);
            n.states.checked = Some(flag(get, "checked", false));
            n.actions = vec![Action::Click, Action::Focus];
        }
        "RLABEL" | "RGROUPBOX" | "RPANEL" | "RBEVEL" => own_caption(&mut n, false),
        // (QDIGDISPLAY: the text it shows)
        "RDIGDISPLAY" => n.name = super::digdisplay_text(id).unwrap_or_else(|| text(get, "display")),
        // (its text: a PasswordChar's characters for a password)
        "REDIT" | "RMEMO" | "RRICHEDIT" | "RCODEEDITOR" => {
            let multi = t != "REDIT";
            let (value, read_only) = super::with_textedit(id, |e| (e.text(), e.read_only)).unwrap_or_default();
            n.value = Some(match text(get, "passwordchar").chars().next() {
                Some(m) if !multi => std::iter::repeat_n(m, value.chars().count()).collect(),
                _ => value,
            });
            n.states.read_only = read_only;
            n.states.multiline = multi;
            n.actions = vec![Action::SetValue, Action::Focus];
        }
        "RPROGRESS" | "RPROGRESSBAR" => {
            let (min, max, pos) = progress_range(get);
            n.numeric = Some(Numeric { value: pos as f64, min: min as f64, max: max as f64, step: 1.0, jump: 10.0 });
            n.value = Some(format!("{}%", progress_percent(get)));
        }
        "RUPDOWN" => {
            let (min, max, pos) = progress_range(get);
            n.numeric = Some(Numeric { value: pos as f64, min: min as f64, max: max as f64, step: int(get, "increment", 1).max(1) as f64, jump: 10.0 });
            n.actions = vec![Action::Increment, Action::Decrement, Action::Focus];
        }
        // (Windows' scroll bar: its Position between Min and Max less a
        // page but one, SmallChange a step, LargeChange a page)
        "RSCROLLBAR" => {
            let min = int(get, "min", 0);
            let max = int(get, "max", 100).max(min);
            let last = (max - (int(get, "pagesize", 1) - 1).max(0)).max(min);
            n.numeric = Some(Numeric {
                value: int(get, "position", 0).clamp(min, last) as f64,
                min: min as f64,
                max: last as f64,
                step: int(get, "smallchange", 1) as f64,
                jump: int(get, "largechange", 1) as f64,
            });
            n.orientation = Some(if int(get, "kind", 0) == 1 { Orientation::Vertical } else { Orientation::Horizontal });
            n.actions = vec![Action::Increment, Action::Decrement, Action::SetValue, Action::Focus];
        }
        // (ARIA's sense: a bar between left and right panes stands up)
        "RSPLITTER" => n.orientation = Some(if matches!(int(get, "align", 3), 1 | 2) { Orientation::Horizontal } else { Orientation::Vertical }),
        "RMDICHILD" => {
            n.name = text(get, "caption");
            n.states.focused = flag(get, "active", false);
        }
        "RTRACKBAR" => {
            if let Some(d) = super::with_trackbar(id, |tb| tb.describe(n.id, w, h)) {
                n = d;
            }
            n.actions.push(Action::Focus);
        }
        "RTABCONTROL" => {
            if let Some(d) = super::with_tabcontrol(id, |tc| tc.describe(id, w, h, font)) {
                n = d;
            }
            n.actions.push(Action::Focus);
        }
        "RLISTBOX" | "RFILELISTBOX" => {
            n.actions = vec![Action::Focus];
            let items = super::with_list(id, |l| l.items.iter().enumerate().take(MOST_ITEMS).map(|(i, s)| (s.clone(), l.is_selected(i))).collect::<Vec<_>>()).unwrap_or_default();
            n.children = items.into_iter().enumerate().map(|(i, (s, sel))| option(id, i, s, sel)).collect();
        }
        // (its list's state, dropped or not, is the host's)
        "RCOMBOBOX" => {
            let (value, items, index) = super::with_list(id, |l| (l.text.clone(), l.items.iter().take(MOST_ITEMS).cloned().collect::<Vec<_>>(), l.item_index)).unwrap_or_default();
            n.value = Some(value);
            n.states.expanded = Some(false);
            n.actions = vec![Action::Focus, Action::Expand, Action::Collapse];
            n.children = items.into_iter().enumerate().map(|(i, s)| option(id, i, s, index == i as i64)).collect();
        }
        "RLISTVIEW" => {
            n.actions = vec![Action::Focus];
            let (items, index) = super::with_listview(id, |lv| (lv.items.iter().take(MOST_ITEMS).map(|it| it.caption.clone()).collect::<Vec<_>>(), lv.item_index)).unwrap_or_default();
            n.children = items.into_iter().enumerate().map(|(i, s)| option(id, i, s, index == i as i64)).collect();
        }
        "RTREEVIEW" => {
            n.actions = vec![Action::Focus];
            let rows = super::with_tree(id, |tv| {
                tv.visible_rows()
                    .into_iter()
                    .take(MOST_ITEMS)
                    .map(|i| {
                        let node = &tv.nodes[i];
                        (i, node.text.clone(), node.level, tv.item_index == i as i64, tv.has_children(i).then_some(node.expanded))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
            for (i, name, level, selected, expanded) in rows {
                let mut item = AccessNode::new(part_id(id, PART_ITEM, i), Role::TreeItem);
                item.name = name;
                item.level = Some(level + 1);
                item.states.selected = Some(selected);
                item.states.expanded = expanded;
                item.actions = vec![Action::Click];
                if let Some(e) = expanded {
                    item.actions.push(if e { Action::Collapse } else { Action::Expand });
                }
                n.children.push(item);
            }
        }
        "RSTRINGGRID" => {
            n.actions = vec![Action::Focus];
            let rows = super::with_grid(id, |g| {
                let cols = g.col_count().max(1);
                (0..g.row_count().min(MOST_CELLS / cols)).map(|r| (0..g.col_count()).map(|c| (g.cell(c, r).to_string(), g.is_selected(c, r))).collect::<Vec<_>>()).collect::<Vec<_>>()
            })
            .unwrap_or_default();
            for (r, cells) in rows.into_iter().enumerate() {
                let mut row = AccessNode::new(part_id(id, PART_ROW, r), Role::Row);
                for (c, (name, selected)) in cells.into_iter().enumerate() {
                    let mut cell = AccessNode::new(grid_cell_id(id, c, r), Role::Cell);
                    cell.name = name;
                    cell.states.selected = Some(selected);
                    row.children.push(cell);
                }
                n.children.push(row);
            }
        }
        "RHEADER" => {
            let sections = super::with_header(id, |hd| hd.sections.iter().map(|s| s.caption.clone()).zip(hd.spans()).collect::<Vec<_>>()).unwrap_or_default();
            for (i, (caption, (l, r))) in sections.into_iter().enumerate() {
                let mut b = AccessNode::new(part_id(id, PART_ITEM, i), Role::Button);
                b.name = caption;
                b.actions = vec![Action::Click];
                b.bounds = (l, 0, r - l, h);
                n.children.push(b);
            }
        }
        "RSTATUSBAR" => {
            for (k, s) in status_texts(get).into_iter().enumerate() {
                let mut l = AccessNode::new(status_panel_id(id, k), Role::Label);
                l.name = s;
                n.children.push(l);
            }
        }
        _ => {}
    }
    n
}

/// A list's row `i` (a list box's, a combo box's, a list view's item).
fn option(component: &str, i: usize, name: String, selected: bool) -> AccessNode {
    let mut o = AccessNode::new(part_id(component, PART_ITEM, i), Role::ListBoxOption);
    o.name = name;
    o.states.selected = Some(selected);
    o.actions = vec![Action::Click];
    o
}

/// A grid's cell (c, r)'s node id.
pub fn grid_cell_id(component: &str, c: usize, r: usize) -> u64 {
    part_id(component, PART_CELL, r * 10_000 + c)
}

/// A QPROGRESSBAR's (a QUPDOWN's) Min, Max and Position (clamped).
pub fn progress_range(get: Props) -> (i64, i64, i64) {
    let min = int(get, "min", 0);
    let max = int(get, "max", 100).max(min);
    (min, max, int(get, "position", 0).clamp(min, max))
}

/// How much is done, in percent (TGauge's PercentDone: whole numbers).
pub fn progress_percent(get: Props) -> i64 {
    let (min, max, pos) = progress_range(get);
    if max == min {
        return 0;
    }
    (pos - min) * 100 / (max - min)
}

/// Where a node's name came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameFrom {
    /// Its own: its caption (`describe`), or none.
    Own,
    /// Its AccessibleName (RapidR's).
    Given,
    Hint,
    /// The QLABEL naming it (`labelled_by`).
    Label,
}

/// The name rule (plan §6), after a component's own name (`describe`'s:
/// its Caption …): its AccessibleName replaces it; without either, its
/// Hint, then the QLABEL naming it (`label`: that label's node id and
/// caption, asked only then, and only for a control: a label, a panel, a
/// status bar, a picture are never named by one). Its description is its
/// AccessibleDescription, else its Hint when the Hint isn't its name (a
/// tooltip, as a browser reads a title).
pub fn apply_name_rule(n: &mut AccessNode, get: Props, label: impl FnOnce() -> Option<(u64, String)>) -> NameFrom {
    let hint = text(get, "hint");
    let given = text(get, "accessiblename");
    let from = if !given.is_empty() {
        n.name = given;
        NameFrom::Given
    } else if !n.name.is_empty() {
        NameFrom::Own
    } else if !hint.is_empty() {
        n.name = hint.clone();
        NameFrom::Hint
    } else if matches!(n.role, Role::Label | Role::Pane | Role::Group | Role::Status | Role::Splitter | Role::Image | Role::Canvas | Role::Window | Role::Dialog | Role::MenuBar | Role::Unknown) {
        NameFrom::Own
    } else {
        match label() {
            Some((by, caption)) => {
                n.labelled_by = Some(by);
                n.name = mnemonic(&caption).0;
                NameFrom::Label
            }
            None => NameFrom::Own,
        }
    };
    n.description = text(get, "accessibledescription");
    if n.description.is_empty() && from != NameFrom::Hint {
        n.description = hint;
    }
    from
}

/// Which of a parent's shown components names component `target` without a
/// name of its own (indexes into `siblings`: each one's rectangle, and
/// whether it's a QLABEL): the nearest label starting to its left on its
/// line, else the nearest above it that doesn't already name a control to
/// its right.
pub fn label_for(target: usize, siblings: &[(Rect, bool)]) -> Option<usize> {
    let labels: Vec<usize> = (0..siblings.len()).filter(|&l| l != target && siblings[l].1).collect();
    let rects: Vec<Rect> = labels.iter().map(|&l| siblings[l].0).collect();
    if let Some(k) = label_left_of(siblings[target].0, &rects) {
        return Some(labels[k]);
    }
    let claimed: Vec<usize> = (0..siblings.len()).filter(|&o| o != target && !siblings[o].1).filter_map(|o| label_left_of(siblings[o].0, &rects)).collect();
    let free: Vec<usize> = (0..labels.len()).filter(|k| !claimed.contains(k)).collect();
    let free_rects: Vec<Rect> = free.iter().map(|&k| rects[k]).collect();
    label_above(siblings[target].0, &free_rects).map(|k| labels[free[k]])
}

/// A form as Tab walks it ([`tab_order`]).
pub trait TabTree {
    type Id: Clone;
    /// A component's children (`None`: the form's), in creation order.
    fn children(&self, parent: Option<&Self::Id>) -> Vec<Self::Id>;
    /// Its TabOrder (`None`: unset).
    fn tab_order_of(&self, c: &Self::Id) -> Option<i64>;
    /// Shown and enabled (else neither it nor its children are visited).
    fn active(&self, c: &Self::Id) -> bool;
    /// Tab stops on it: it takes the focus and its TabStop is on.
    fn stops(&self, c: &Self::Id) -> bool;
}

/// The components Tab visits, in order, as Windows' dialog navigation
/// walks them: depth first through the form, each parent's children by
/// TabOrder where they have one (else in creation order; equal ones keep
/// it), skipping what's hidden or disabled (and its children) and what
/// Tab doesn't stop on.
pub fn tab_order<T: TabTree>(tree: &T) -> Vec<T::Id> {
    tab_walk(tree).into_iter().filter(|(_, stops)| *stops).map(|(c, _)| c).collect()
}

/// Every shown and enabled component in the order Tab walks them, with
/// whether Tab stops on it (a label is walked past: [`next_stop_after`]).
pub fn tab_walk<T: TabTree>(tree: &T) -> Vec<(T::Id, bool)> {
    fn walk<T: TabTree>(tree: &T, parent: Option<&T::Id>, out: &mut Vec<(T::Id, bool)>) {
        let mut sorted: Vec<(i64, T::Id)> = tree.children(parent).into_iter().enumerate().map(|(k, c)| (tree.tab_order_of(&c).unwrap_or(k as i64), c)).collect();
        sorted.sort_by_key(|(o, _)| *o);
        for (_, c) in sorted {
            if !tree.active(&c) {
                continue;
            }
            out.push((c.clone(), tree.stops(&c)));
            walk(tree, Some(&c), out);
        }
    }
    let mut out = Vec::new();
    walk(tree, None, &mut out);
    out
}

/// What Alt + a label's letter focuses: the first Tab stop after the
/// label in Tab's walk (Windows' dialogs: the control after the static
/// text).
pub fn next_stop_after<T: TabTree>(tree: &T, label: &T::Id) -> Option<T::Id>
where
    T::Id: PartialEq,
{
    tab_walk(tree).into_iter().skip_while(|(c, _)| c != label).skip(1).find(|(_, stops)| *stops).map(|(c, _)| c)
}

/// A menu bar's items (`main`: the QMAINMENU), separators left out: each
/// a menu item named by its caption, its `&` letter its shortcut, a submenu
/// collapsed (its host says when it's open), bounds the host's.
pub fn menu_bar(main: &str) -> Vec<AccessNode> {
    super::menu::children(main)
        .into_iter()
        .filter_map(|item| {
            let submenu = !super::menu::children(&item).is_empty();
            super::menu::with(&item, |m| (m.caption.clone(), m.enabled, submenu)).filter(|(c, ..)| c != "-")
        })
        .enumerate()
        .map(|(k, (caption, enabled, submenu))| {
            let mut c = AccessNode::new(part_id(main, PART_MENU, k), Role::MenuItem);
            let (shown, mark) = mnemonic(&caption);
            c.name = shown;
            c.shortcut = mark.map(|m| m.1);
            c.states.disabled = !enabled;
            c.states.expanded = submenu.then_some(false);
            c.actions = vec![Action::Click];
            c
        })
        .collect()
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

    /// Properties as a host keeps them (lowercase names).
    fn props(pairs: &[(&str, Value)]) -> impl Fn(&str) -> Value {
        let map: std::collections::HashMap<String, Value> = pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        move |p: &str| map.get(p).cloned().unwrap_or(Value::Null)
    }

    #[test]
    fn components_describe_themselves_from_their_properties() {
        let font = Font::default();
        let get = props(&[("caption", v_str("&Remember me")), ("checked", v_int(-1))]);
        let n = describe("chk", "RCHECKBOX", &get, (120, 20), &font);
        assert_eq!((n.role, n.name.as_str(), n.shortcut, n.states.checked), (Role::CheckBox, "Remember me", Some('r'), Some(true)));
        // a button without a caption shows its Kind's
        let n = describe("ok", "RBUTTON", &props(&[("kind", v_int(1))]), (75, 25), &font);
        assert_eq!(n.name, "OK");
        // a cool button in a group is a toggle (pressed: checked)
        let n = describe("cb", "RCOOLBTN", &props(&[("groupindex", v_int(1)), ("down", v_int(0))]), (25, 25), &font);
        assert_eq!((n.role, n.states.checked), (Role::Button, Some(false)));
        let n = describe("pb", "RPROGRESSBAR", &props(&[("position", v_int(30)), ("max", v_int(60))]), (100, 16), &font);
        assert_eq!((n.role, n.value.as_deref(), n.numeric.unwrap().max), (Role::ProgressIndicator, Some("50%"), 60.0));
        let n = describe("ud", "RUPDOWN", &props(&[("position", v_int(4)), ("increment", v_int(2))]), (16, 25), &font);
        assert_eq!((n.role, n.numeric.unwrap().value, n.numeric.unwrap().step), (Role::SpinButton, 4.0, 2.0));
        // a splitter between left and right panes stands up (ARIA's sense)
        let n = describe("sp", "RSPLITTER", &props(&[("align", v_int(3))]), (3, 100), &font);
        assert_eq!((n.role, n.orientation), (Role::Splitter, Some(Orientation::Vertical)));
        let n = describe("sb", "RSTATUSBAR", &props(&[("panelcount", v_int(2)), ("panel(0).caption", v_str("Ready")), ("panel(1).caption", v_str("INS"))]), (300, 20), &font);
        assert_eq!((n.role, n.children.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()), (Role::Status, vec!["Ready", "INS"]));
        assert_eq!(n.children[1].id, status_panel_id("sb", 1));
        assert_eq!(role_of("RPANEL"), Role::Pane);
        assert!(takes_focus("REDIT") && !takes_focus("RLABEL") && !takes_focus("rcoolbtn"));
    }

    #[test]
    fn models_parts_are_described() {
        let font = Font::default();
        let get = props(&[]);
        crate::objects::create("a11ytree", "RTREEVIEW");
        crate::objects::call("a11ytree", "additems", &[v_str("Root"), v_str("Other")], &|_, _| Value::Null);
        crate::objects::call("a11ytree", "addchilditems", &[v_int(0), v_str("Leaf")], &|_, _| Value::Null);
        let n = describe("a11ytree", "RTREEVIEW", &get, (100, 100), &font);
        // (Root collapsed: its leaf has no row)
        let items: Vec<_> = n.children.iter().map(|c| (c.name.as_str(), c.level, c.states.expanded)).collect();
        assert_eq!(items[0], ("Root", Some(1), Some(false)), "{items:?}");
        assert!(n.children.iter().all(|c| c.role == Role::TreeItem && c.name != "Leaf"), "{items:?}");
        assert!(n.to_json().contains("\"level\":1"));
        crate::objects::create("a11ygrid", "RSTRINGGRID");
        crate::objects::set("a11ygrid", "colcount", &v_int(2));
        crate::objects::set("a11ygrid", "rowcount", &v_int(3));
        let n = describe("a11ygrid", "RSTRINGGRID", &get, (100, 100), &font);
        assert_eq!((n.children.len(), n.children[0].children.len()), (3, 2));
        assert_eq!((n.children[2].role, n.children[2].children[1].role, n.children[2].children[1].id), (Role::Row, Role::Cell, grid_cell_id("a11ygrid", 1, 2)));
        crate::objects::remove("a11ytree");
        crate::objects::remove("a11ygrid");
    }

    #[test]
    fn the_name_rule() {
        let font = Font::default();
        // AccessibleName wins over the caption
        let get = props(&[("caption", v_str("&Go")), ("accessiblename", v_str("Start the search")), ("accessibledescription", v_str("F5"))]);
        let mut n = describe("b", "RBUTTON", &get, (75, 25), &font);
        assert_eq!(apply_name_rule(&mut n, &get, || None), NameFrom::Given);
        assert_eq!((n.name.as_str(), n.description.as_str()), ("Start the search", "F5"));
        // no caption: the Hint, then the label naming it
        let get = props(&[("hint", v_str("Your name"))]);
        let mut n = describe("e", "REDIT", &get, (100, 21), &font);
        assert_eq!((apply_name_rule(&mut n, &get, || Some((7, "&Name:".into()))), n.name.as_str()), (NameFrom::Hint, "Your name"));
        let get = props(&[]);
        let mut n = describe("e", "REDIT", &get, (100, 21), &font);
        assert_eq!((apply_name_rule(&mut n, &get, || Some((7, "&Name:".into()))), n.name.as_str(), n.labelled_by), (NameFrom::Label, "Name:", Some(7)));
        // a label is never named by another, nor a status bar
        let mut n = describe("l", "RLABEL", &get, (100, 21), &font);
        assert_eq!((apply_name_rule(&mut n, &get, || Some((7, "x".into()))), n.labelled_by), (NameFrom::Own, None));
        let mut n = describe("sb", "RSTATUSBAR", &get, (300, 20), &font);
        assert_eq!((apply_name_rule(&mut n, &get, || Some((7, "x".into()))), n.labelled_by), (NameFrom::Own, None));
        // a Hint its caption doesn't need is its description (a tooltip)
        let get = props(&[("caption", v_str("OK")), ("hint", v_str("Signs you in"))]);
        let mut n = describe("ok", "RBUTTON", &get, (75, 25), &font);
        apply_name_rule(&mut n, &get, || None);
        assert_eq!((n.name.as_str(), n.description.as_str()), ("OK", "Signs you in"));
    }

    #[test]
    fn labels_name_the_controls_beside_or_under_them() {
        // "Name:" [edit]   "Notes" over a memo, a button with no label
        let siblings = [((8, 12, 40, 16), true), ((56, 8, 120, 21), false), ((8, 40, 60, 16), true), ((8, 60, 170, 50), false), ((200, 8, 75, 25), false)];
        assert_eq!(label_for(1, &siblings), Some(0));
        assert_eq!(label_for(3, &siblings), Some(2));
        // the label left of the edit doesn't also name what's under it
        let siblings = [((8, 12, 40, 16), true), ((56, 8, 120, 21), false), ((56, 40, 120, 21), false)];
        assert_eq!(label_for(2, &siblings), None);
    }

    /// A form of (name, TabOrder, takes the focus, children).
    struct Form(Vec<(&'static str, Option<i64>, bool, Vec<usize>)>, Vec<usize>);

    impl TabTree for Form {
        type Id = usize;
        fn children(&self, parent: Option<&usize>) -> Vec<usize> {
            parent.map_or_else(|| self.1.clone(), |&p| self.0[p].3.clone())
        }
        fn tab_order_of(&self, &c: &usize) -> Option<i64> {
            self.0[c].1
        }
        fn active(&self, _: &usize) -> bool {
            true
        }
        fn stops(&self, &c: &usize) -> bool {
            self.0[c].2
        }
    }

    #[test]
    fn tab_follows_tab_order_depth_first() {
        // edit (TabOrder 2), a panel (TabOrder 0) holding two buttons, a check box (TabOrder 1)
        let f = Form(vec![("edit", Some(2), true, vec![]), ("panel", Some(0), false, vec![2, 3]), ("b1", None, true, vec![]), ("b2", None, true, vec![]), ("chk", Some(1), true, vec![])], vec![0, 1, 4]);
        let names: Vec<_> = tab_order(&f).into_iter().map(|i| f.0[i].0).collect();
        assert_eq!(names, ["b1", "b2", "chk", "edit"]);
    }

    #[test]
    fn a_labels_letter_focuses_the_next_stop_in_tabs_walk() {
        // "Name:" [edit], "Pass:" [edit2], a button TabOrder 1 (before "Pass:")
        let f = Form(vec![("name", None, false, vec![]), ("edit", None, true, vec![]), ("pass", None, false, vec![]), ("edit2", None, true, vec![]), ("btn", Some(1), true, vec![])], vec![0, 1, 2, 3, 4]);
        assert_eq!(next_stop_after(&f, &0).map(|i| f.0[i].0), Some("edit"));
        // (the button comes before "Pass:" in Tab's walk: not after it)
        assert_eq!(next_stop_after(&f, &2).map(|i| f.0[i].0), Some("edit2"));
        assert_eq!(next_stop_after(&f, &3), None);
    }
}
