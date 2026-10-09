//! RTOOLBOX's model (docs/ide-components.md §3.6, docs/ide-plan.md I1 and
//! §6.5): the components a form designer places, under "RapidQ" (the
//! components RapidQ has, by their Q names) and "RapidR" (RapidR's own),
//! each in its group (`rapidr_icons::TOOLBOX_GROUPS`, design/icons/
//! inventory.toml's `[toolbox-groups]`), with the program's templates
//! under RapidR › Templates. Only the components the compilers create
//! today are shown (`rapidr_lang::is_component_type`): a planned one
//! (RFORMDESIGNER …) appears when it exists.
//!
//! The kernel (`rapidr-ui-kernel`'s `components/panels/toolbox.rs`) draws
//! [`Toolbox::rows`] and changes the UI state here (the filter typed, the
//! groups opened and closed, the keyboard's row, the hover, a drag); what
//! the program hears comes back as a [`User`] action ([`rt_user`]):
//! OnSelect, OnPick, OnDragStart, OnDragDrop.

use std::collections::BTreeSet;

use super::fuzzy;
use super::runtime::Runtime;
use crate::Value;

/// How the components are named ([`Toolbox::names`], the ShowNames
/// property).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Names {
    /// What the designer writes: RapidQ's components by their Q names
    /// (QBUTTON), the others by their R names (RPLOT).
    #[default]
    AsWritten,
    /// Every one by its R name (RBUTTON).
    RapidR,
    /// In words (Button).
    Titles,
}

impl Names {
    pub fn parse(s: &str) -> Option<Names> {
        match s.trim().to_ascii_lowercase().as_str() {
            "as-written" | "aswritten" | "written" | "" => Some(Names::AsWritten),
            "rapidr" | "r" => Some(Names::RapidR),
            "titles" | "title" | "words" => Some(Names::Titles),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Names::AsWritten => "as-written",
            Names::RapidR => "rapidr",
            Names::Titles => "titles",
        }
    }
}

/// A template: a component with preset properties (its CREATE block).
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub name: String,
    pub source: String,
    /// A built-in icon or a component type ("" : the template's own).
    pub icon: String,
}

/// What a row of the toolbox is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// A group, by its id (`rapidq`, `standard`, `templates` …).
    Group(String),
    /// A component, by its RapidR type (`RBUTTON`).
    Component(String),
    /// A template, by its name.
    Template(String),
}

impl Key {
    pub fn is_item(&self) -> bool {
        !matches!(self, Key::Group(_))
    }
}

/// A group as the toolbox shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    /// Its top group (`None`: it is one, "RapidQ" or "RapidR").
    pub parent: Option<&'static str>,
}

/// One row as drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub key: Key,
    /// 1: a top group, 2: a group in it, 3: an item (searching: 1, flat).
    pub level: usize,
    /// What the row shows (a group's title, an item's name as ShowNames
    /// says).
    pub text: String,
    /// The icon drawn before it.
    pub icon: String,
    /// A group: open or not; an item: `None`.
    pub open: Option<bool>,
    /// A group: how many items it holds.
    pub count: usize,
    /// Searching: the item's group's title, drawn dimmed beside it.
    pub group: String,
    /// Searching: the matched characters of `text`.
    pub marks: Vec<usize>,
}

/// A press on an item that may become a drag (the kernel's).
#[derive(Clone, Debug, PartialEq)]
pub struct Drag {
    pub key: Key,
    /// Where the press was and where the mouse is (the toolbox's pixels).
    pub from: (i64, i64),
    pub at: (i64, i64),
    /// The mouse went far enough: it is a drag (OnDragStart fired).
    pub started: bool,
}

/// RTOOLBOX's state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Toolbox {
    /// The search box's text.
    pub filter: String,
    pub names: Names,
    /// The groups closed (ids).
    pub closed: BTreeSet<String>,
    /// The selected item (a component or a template).
    pub selected: Option<Key>,
    /// The keyboard's row (a group or an item).
    pub cursor: Option<Key>,
    /// The row under the mouse.
    pub hover: Option<Key>,
    pub templates: Vec<Template>,
    /// A press on an item, maybe a drag.
    pub drag: Option<Drag>,
    /// What was typed for type-ahead (the list's keys).
    pub typed: String,
}

crate::panel_models!(Toolbox);

/// The groups in the toolbox's order: "RapidQ" and its groups, then
/// "RapidR" and its groups.
pub fn groups() -> Vec<Group> {
    let table = group_table();
    let mut out = Vec::new();
    for top in table.iter().filter(|g| g.3.is_empty()) {
        out.push(Group { id: top.0, title: top.1, icon: top.2, parent: None });
        for g in table.iter().filter(|g| g.3 == top.0) {
            out.push(Group { id: g.0, title: g.1, icon: g.2, parent: Some(top.0) });
        }
    }
    out
}

/// (id, title, icon, parent, members) of `rapidr_icons::TOOLBOX_GROUPS`.
type GroupEntry = (&'static str, &'static str, &'static str, &'static str, &'static [&'static str]);

#[cfg(feature = "icons")]
fn group_table() -> Vec<GroupEntry> {
    rapidr_icons::TOOLBOX_GROUPS.iter().map(|g| (g.id, g.title, g.icon, g.parent, g.members)).collect()
}

#[cfg(not(feature = "icons"))]
fn group_table() -> Vec<GroupEntry> {
    Vec::new()
}

/// A component's title in words (its icon's: "Button"), else its name as
/// written.
#[cfg(feature = "icons")]
fn title_of(type_name: &str) -> String {
    match rapidr_icons::component(type_name) {
        Some(i) => i.title.to_string(),
        None => written_name(type_name),
    }
}

#[cfg(not(feature = "icons"))]
fn title_of(type_name: &str) -> String {
    written_name(type_name)
}

/// The components a group shows: its members the compilers create, in the
/// table's order.
pub fn members(group: &str) -> Vec<&'static str> {
    group_table().into_iter().filter(|g| g.0 == group).flat_map(|g| g.4.iter().copied()).filter(|t| rapidr_lang::is_component_type(t)).collect()
}

/// A component as the designer writes it: QBUTTON for RapidQ's, RPLOT
/// for RapidR's own.
pub fn written_name(type_name: &str) -> String {
    match rapidr_lang::component(type_name) {
        Some(c) => c.written_name().to_string(),
        None => type_name.to_ascii_uppercase(),
    }
}

/// A component type the program names (QBUTTON, RBUTTON, any case): its
/// RapidR type, when the toolbox has it.
fn component_named(name: &str) -> Option<String> {
    let c = rapidr_lang::component(name).or_else(|| rapidr_lang::resolve_component(name))?;
    group_table().iter().any(|g| g.4.contains(&c.name)).then(|| c.name.to_string())
}

/// What a component's row says about it (the registry's doc, its first
/// sentence): a screen reader's description.
pub fn description(type_name: &str) -> String {
    let doc = rapidr_lang::component(type_name).map(|c| c.doc).unwrap_or("");
    match doc.find(". ") {
        Some(i) => doc[..=i].to_string(),
        None => doc.to_string(),
    }
}

/// What a match in a component's doc scores: below its names' matches.
const DOC_SCORE: i32 = -1000;

/// Whether every word of `filter` (three letters or more) starts a word of
/// component `type_name`'s doc or title — "chart" finds RPLOT, "timer"
/// QTIMER's neighbours, "grid" the grids.
pub fn doc_match(filter: &str, type_name: &str) -> bool {
    let q: Vec<String> = filter.split_whitespace().map(str::to_lowercase).collect();
    if q.is_empty() || q.iter().any(|w| w.chars().count() < 3) {
        return false;
    }
    let text = format!("{} {}", title_of(type_name), rapidr_lang::component(type_name).map_or("", |c| c.doc)).to_lowercase();
    let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    q.iter().all(|w| words.iter().any(|x| x.starts_with(w.as_str())))
}

/// An item's card (its tooltip): what it is in a sentence, where it comes
/// from, where it runs.
pub fn card(key: &Key) -> String {
    let Key::Component(t) = key else { return String::new() };
    let Some(c) = rapidr_lang::component(t) else { return String::new() };
    let from = match c.rapidq {
        Some(q) => format!("RapidQ's {q}"),
        None => "RapidR's own".to_string(),
    };
    let runs = match c.runtimes {
        rapidr_lang::Runtimes::Desktop => ", desktop only",
        rapidr_lang::Runtimes::Web => ", web only",
        _ => "",
    };
    // (a form is a document: the toolbox adds a form file for it)
    let does = match c.name {
        "RFORM" => " Click or double-click: Project > Add Form, a new form file of the program; dragged onto an RFormMDI: one of its child windows.",
        "RFORMMDI" => " Click or double-click: a new MDI main window file of the program.",
        _ => "",
    };
    format!("{} - {} ({from}{runs}){does}", c.pretty(c.name), description(t))
}

impl Toolbox {
    /// A component's name as ShowNames says.
    pub fn name_of(&self, type_name: &str) -> String {
        match self.names {
            Names::AsWritten => written_name(type_name),
            // (RapidR's own names as the registry spells them: RButton, RPlot)
            Names::RapidR => rapidr_lang::component(type_name).map_or_else(|| type_name.to_ascii_uppercase(), |c| c.pretty(c.name)),
            Names::Titles => title_of(type_name),
        }
    }

    /// What OnPick, OnSelect and Item give for an item: a component as the
    /// designer writes it, a template's name.
    pub fn written(key: &Key) -> String {
        match key {
            Key::Component(t) => written_name(t),
            Key::Template(n) => n.clone(),
            Key::Group(g) => g.clone(),
        }
    }

    /// The item a program names: a component (QBUTTON, RBUTTON …) or a
    /// template.
    pub fn key_of(&self, name: &str) -> Option<Key> {
        if let Some(t) = self.templates.iter().find(|t| t.name.eq_ignore_ascii_case(name)) {
            return Some(Key::Template(t.name.clone()));
        }
        component_named(name).map(Key::Component)
    }

    /// A group a program names, by its title ("Data Science") or its id.
    pub fn group_named(name: &str) -> Option<&'static str> {
        let n = name.trim();
        groups().into_iter().find(|g| g.title.eq_ignore_ascii_case(n) || g.id.eq_ignore_ascii_case(n)).map(|g| g.id)
    }

    /// The items of group `id` (a top group's: of all its groups), in order.
    fn group_items(&self, id: &str) -> Vec<Key> {
        if id == "templates" {
            return self.templates.iter().map(|t| Key::Template(t.name.clone())).collect();
        }
        let mut out: Vec<Key> = members(id).into_iter().map(|t| Key::Component(t.to_string())).collect();
        for g in groups().into_iter().filter(|g| g.parent == Some(id)) {
            out.extend(self.group_items(g.id));
        }
        out
    }

    /// Every item in the toolbox's order, with its group.
    pub fn catalog(&self) -> Vec<(Key, &'static Group)> {
        let gs: &'static [Group] = groups_static();
        let mut out = Vec::new();
        for g in gs.iter().filter(|g| g.parent.is_some()) {
            for k in self.group_items(g.id) {
                out.push((k, g));
            }
        }
        out
    }

    /// An item's icon: a component's type, a template's own (or its
    /// component's).
    pub fn icon_of(&self, key: &Key) -> String {
        match key {
            Key::Component(t) => t.clone(),
            Key::Template(n) => self.templates.iter().find(|t| &t.name == n).map(|t| if t.icon.is_empty() { "templates".to_string() } else { t.icon.clone() }).unwrap_or_default(),
            Key::Group(g) => groups_static().iter().find(|x| x.id == g).map(|x| x.icon.to_string()).unwrap_or_default(),
        }
    }

    /// An item's text as shown.
    pub fn text_of(&self, key: &Key) -> String {
        match key {
            Key::Component(t) => self.name_of(t),
            Key::Template(n) => n.clone(),
            Key::Group(g) => groups_static().iter().find(|x| x.id == g).map(|x| x.title.to_string()).unwrap_or_default(),
        }
    }

    /// How well the filter matches an item: its shown text first (with the
    /// marks), else its other names (QBUTTON, RBUTTON, Button: no marks).
    fn score(&self, key: &Key) -> Option<(i32, Vec<usize>)> {
        let shown = self.text_of(key);
        // (a type's name read without its Q or R too: "but" is QBUTTON's
        // word start)
        let bare = shown.len() > 1 && key.is_item() && shown.starts_with(['Q', 'R']) && shown[1..].starts_with(|c: char| c.is_ascii_uppercase());
        let whole = fuzzy::score(&self.filter, &shown);
        let rest = if bare { fuzzy::score(&self.filter, &shown[1..]).map(|(s, m)| (s, m.into_iter().map(|i| i + 1).collect())) } else { None };
        if let Some(m) = [whole, rest].into_iter().flatten().max_by_key(|(s, _)| *s) {
            return Some(m);
        }
        let Key::Component(t) = key else { return None };
        let named = [written_name(t), t.clone(), title_of(t)].iter().filter_map(|n| fuzzy::score(&self.filter, n)).map(|(s, _)| (s - 20, Vec::new())).max_by_key(|(s, _)| *s);
        named.or_else(|| doc_match(&self.filter, t).then(|| (DOC_SCORE, Vec::new())))
    }

    /// The rows shown now: the groups and their items (a closed group's
    /// hidden; a group with no items left out), or, while searching, the
    /// matches flat, best first.
    pub fn rows(&self) -> Vec<Row> {
        if !self.filter.trim().is_empty() {
            let mut found: Vec<(i32, usize, Row)> = Vec::new();
            for (n, (key, g)) in self.catalog().into_iter().enumerate() {
                if let Some((s, marks)) = self.score(&key) {
                    let row = Row { text: self.text_of(&key), icon: self.icon_of(&key), key, level: 1, open: None, count: 0, group: g.title.to_string(), marks };
                    found.push((s, n, row));
                }
            }
            found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            return found.into_iter().map(|(_, _, r)| r).collect();
        }
        let mut out = Vec::new();
        for top in groups_static().iter().filter(|g| g.parent.is_none()) {
            let all = self.group_items(top.id);
            if all.is_empty() {
                continue;
            }
            let open = !self.closed.contains(top.id);
            out.push(Row { key: Key::Group(top.id.to_string()), level: 1, text: top.title.to_string(), icon: top.icon.to_string(), open: Some(open), count: all.len(), group: String::new(), marks: Vec::new() });
            if !open {
                continue;
            }
            for g in groups_static().iter().filter(|g| g.parent == Some(top.id)) {
                let items = self.group_items(g.id);
                if items.is_empty() {
                    continue;
                }
                let gopen = !self.closed.contains(g.id);
                out.push(Row { key: Key::Group(g.id.to_string()), level: 2, text: g.title.to_string(), icon: g.icon.to_string(), open: Some(gopen), count: items.len(), group: String::new(), marks: Vec::new() });
                if !gopen {
                    continue;
                }
                for k in items {
                    out.push(Row { text: self.text_of(&k), icon: self.icon_of(&k), key: k, level: 3, open: None, count: 0, group: String::new(), marks: Vec::new() });
                }
            }
        }
        out
    }

    /// The items shown now (Count, Item): the rows less the groups.
    pub fn items(&self) -> Vec<Key> {
        self.rows().into_iter().map(|r| r.key).filter(Key::is_item).collect()
    }

    /// Opens or closes a group (`None`: turns it over).
    pub fn set_open(&mut self, group: &str, open: Option<bool>) {
        let now = !self.closed.contains(group);
        if open.unwrap_or(!now) {
            self.closed.remove(group);
        } else {
            self.closed.insert(group.to_string());
        }
        // (the keyboard's row inside a group closed: the group)
        if let Some(Key::Group(_)) | None = self.cursor {
            return;
        }
        if !self.rows().iter().any(|r| Some(&r.key) == self.cursor.as_ref()) {
            self.cursor = Some(Key::Group(group.to_string()));
        }
    }

    pub fn set_all(&mut self, open: bool) {
        self.closed.clear();
        if !open {
            self.closed.extend(groups_static().iter().map(|g| g.id.to_string()));
            self.cursor = self.cursor.take().map(|c| if c.is_item() { Key::Group("rapidq".into()) } else { c });
        }
    }

    /// The group row a row is in (`None`: a top group's, or searching).
    pub fn parent_of(&self, key: &Key) -> Option<Key> {
        if !self.filter.trim().is_empty() {
            return None;
        }
        match key {
            Key::Group(g) => groups_static().iter().find(|x| x.id == g).and_then(|x| x.parent).map(|p| Key::Group(p.to_string())),
            item => self.catalog().into_iter().find(|(k, _)| k == item).map(|(_, g)| Key::Group(g.id.to_string())),
        }
    }

    /// The filter changed: the keyboard's row goes to the best match.
    pub fn set_filter(&mut self, text: &str) {
        if self.filter == text {
            return;
        }
        self.filter = text.to_string();
        if !self.filter.trim().is_empty() {
            self.cursor = self.rows().first().map(|r| r.key.clone());
        } else if let Some(s) = self.selected.clone() {
            self.cursor = Some(s);
        }
    }

    /// Type-ahead: `ch` typed in the list goes to the next row whose text
    /// starts with what was typed (the letters so far, or `ch` alone from
    /// the row after the keyboard's when they match nothing); a name's Q or
    /// R may be left out ("la" finds QLABEL).
    pub fn type_ahead(&mut self, ch: &str) -> Option<Key> {
        let rows = self.rows();
        if rows.is_empty() {
            return None;
        }
        let n = rows.len();
        let starts = |text: &str, typed: &str| {
            let (t, p) = (text.to_lowercase(), typed.to_lowercase());
            t.starts_with(&p) || (text.len() > 1 && text.starts_with(['Q', 'R']) && text[1..].starts_with(|c: char| c.is_ascii_uppercase()) && t[1..].starts_with(&p))
        };
        let find = |from: usize, typed: &str| (0..n).map(|k| (from + k) % n).find(|&i| starts(&rows[i].text, typed));
        let at = self.cursor.as_ref().and_then(|c| rows.iter().position(|r| &r.key == c));
        let more = format!("{}{}", self.typed, ch);
        let found = find(at.unwrap_or(0), &more).map(|i| (i, more)).or_else(|| find(at.map_or(0, |a| a + 1), ch).map(|i| (i, ch.to_string())));
        let (i, typed) = found?;
        self.typed = typed;
        let key = rows[i].key.clone();
        self.cursor = Some(key.clone());
        Some(key)
    }

    pub fn add_template(&mut self, name: &str, source: &str, icon: &str) {
        let t = Template { name: name.to_string(), source: source.to_string(), icon: icon.to_string() };
        match self.templates.iter_mut().find(|x| x.name.eq_ignore_ascii_case(name)) {
            Some(x) => *x = t,
            None => self.templates.push(t),
        }
    }

    pub fn remove_template(&mut self, name: &str) {
        self.templates.retain(|t| !t.name.eq_ignore_ascii_case(name));
        let gone = |k: &Option<Key>| matches!(k, Some(Key::Template(n)) if n.eq_ignore_ascii_case(name));
        if gone(&self.selected) {
            self.selected = None;
        }
        if gone(&self.cursor) {
            self.cursor = Some(Key::Group("templates".into()));
        }
    }
}

/// The groups, made once per thread.
fn groups_static() -> &'static [Group] {
    thread_local! {
        static GROUPS: &'static [Group] = Box::leak(groups().into_boxed_slice());
    }
    GROUPS.with(|g| *g)
}

/// What the user did to it (from the kernel).
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// An item selected (a click, the keyboard): OnSelect.
    Select(Key),
    /// An item chosen to add (a double click, Enter): OnPick.
    Pick(Key),
    /// A press on an item went far enough: OnDragStart (and the toolbox is
    /// drawn over its neighbours while the drag lasts).
    DragStart(Key),
    /// The drag let go at (x, y) of the toolbox's own pixels: OnDragDrop
    /// over what is there on its form.
    Drop(Key, i64, i64),
    /// The drag ended over the toolbox itself, or Escape stopped it.
    DragEnd,
    /// Only its look changed (the row under the mouse): drawn again.
    Redraw,
}

/// Its methods; `None`: not one of its own.
pub fn rt_method<R: Runtime>(_rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let s = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    match method {
        "item" => {
            let i = args.first().map(Value::to_i64).unwrap_or(-1);
            let items = with_mut(name, |t| t.items().into_iter().map(|k| Toolbox::written(&k)).collect::<Vec<_>>());
            Some(Value::String(usize::try_from(i).ok().and_then(|i| items.get(i).cloned()).unwrap_or_default()))
        }
        "expand" | "collapse" => {
            if let Some(g) = Toolbox::group_named(&s(0)) {
                with_mut(name, |t| t.set_open(g, Some(method == "expand")));
            }
            Some(Value::Null)
        }
        "expandall" | "collapseall" => {
            with_mut(name, |t| t.set_all(method == "expandall"));
            Some(Value::Null)
        }
        "addtemplate" => {
            if !s(0).is_empty() {
                with_mut(name, |t| t.add_template(&s(0), &s(1), &s(2)));
            }
            Some(Value::Null)
        }
        "removetemplate" => {
            with_mut(name, |t| t.remove_template(&s(0)));
            Some(Value::Null)
        }
        "template" => Some(Value::String(with(name, |t| t.templates.iter().find(|x| x.name.eq_ignore_ascii_case(&s(0))).map(|x| x.source.clone())).flatten().unwrap_or_default())),
        _ => None,
    }
}

/// Its properties its model answers.
pub fn rt_get<R: Runtime>(_rt: R, name: &str, prop: &str) -> Option<Value> {
    match prop {
        "filter" => Some(Value::String(with(name, |t| t.filter.clone()).unwrap_or_default())),
        "shownames" => Some(Value::String(with(name, |t| t.names.as_str()).unwrap_or("as-written").to_string())),
        "selected" => Some(Value::String(with(name, |t| t.selected.as_ref().map(Toolbox::written)).flatten().unwrap_or_default())),
        "count" => Some(Value::Integer(with_mut(name, |t| t.items().len()) as i64)),
        _ => None,
    }
}

/// Its properties its model keeps: whether `prop` was one.
pub fn rt_set<R: Runtime>(_rt: R, name: &str, prop: &str, v: &Value) -> bool {
    match prop {
        "filter" => with_mut(name, |t| t.set_filter(&v.to_string_val())),
        "shownames" => {
            let n = Names::parse(&v.to_string_val()).unwrap_or_default();
            with_mut(name, |t| t.names = n);
        }
        "selected" => {
            let s = v.to_string_val();
            with_mut(name, |t| {
                t.selected = t.key_of(&s);
                if t.selected.is_some() {
                    t.cursor = t.selected.clone();
                }
            });
        }
        _ => return false,
    }
    true
}

/// What the user did.
pub fn rt_user<R: Runtime>(rt: R, name: &str, action: User) {
    match action {
        User::Select(key) => {
            let changed = with_mut(name, |t| {
                t.cursor = Some(key.clone());
                let changed = t.selected.as_ref() != Some(&key);
                t.selected = Some(key.clone());
                changed
            });
            if changed {
                rt.fire(name, "OnSelect", &[Value::String(Toolbox::written(&key))]);
            }
        }
        User::Pick(key) => {
            with_mut(name, |t| {
                t.selected = Some(key.clone());
                t.cursor = Some(key.clone());
            });
            rt.fire(name, "OnPick", &[Value::String(Toolbox::written(&key))]);
        }
        User::DragStart(key) => {
            // (drawn over its neighbours while it lasts: the kernel's z-order)
            rt.restructure();
            rt.fire(name, "OnDragStart", &[Value::String(Toolbox::written(&key))]);
        }
        User::Drop(key, x, y) => {
            with_mut(name, |t| t.drag = None);
            rt.restructure();
            if let Some((target, tx, ty)) = drop_target(rt, name, (x, y)) {
                rt.fire(name, "OnDragDrop", &[Value::String(Toolbox::written(&key)), Value::String(target), Value::Integer(tx), Value::Integer(ty)]);
            }
        }
        User::Redraw => {}
        User::DragEnd => {
            with_mut(name, |t| t.drag = None);
            rt.restructure();
        }
    }
}

// ------------------------------------------------- where a drop lands --

/// Where a component's children start inside it (the kernel's client
/// areas: a bsSingle panel's edge, a scroll box's border).
fn inset<R: Runtime>(rt: R, name: &str) -> i64 {
    match rt.type_of(name).to_ascii_uppercase().as_str() {
        "RPANEL" if rt.get(name, "borderstyle").to_i64() == 1 => 2,
        "RSCROLLBOX" => match rt.get(name, "borderstyle") {
            Value::Null => 2,
            v if v.to_i64() == 0 => 0,
            _ => 2,
        },
        _ => 0,
    }
}

/// Component `name`'s corner in its form's client area, and the form.
fn place<R: Runtime>(rt: R, name: &str) -> Option<(String, i64, i64)> {
    let (mut x, mut y) = (0, 0);
    let mut at = name.to_string();
    for _ in 0..64 {
        if rt.type_of(&at).eq_ignore_ascii_case("RFORM") {
            return Some((at, x, y));
        }
        x += rt.get(&at, "left").to_i64();
        y += rt.get(&at, "top").to_i64();
        let parent = rt.get(&at, "parent").to_string_val();
        if parent.is_empty() || parent.eq_ignore_ascii_case(&at) {
            return None;
        }
        x += inset(rt, &parent);
        y += inset(rt, &parent);
        at = parent;
    }
    None
}

/// Whether a component is drawn where it is (visible, a visual one, with
/// a size).
fn drawn<R: Runtime>(rt: R, name: &str, type_name: &str) -> bool {
    let visual = rapidr_lang::component(type_name).is_none_or(|c| c.visual);
    visual && super::runtime::truth(&rt.get(name, "visible")) && rt.get(name, "width").to_i64() > 0 && rt.get(name, "height").to_i64() > 0
}

/// The deepest shown component of `parent` under (x, y) of its client area
/// (the last made on top), but `skip` and what is in it: the component and
/// the point in its own pixels.
fn deepest<R: Runtime>(rt: R, parent: &str, (x, y): (i64, i64), skip: &str) -> Option<(String, i64, i64)> {
    for (c, t) in rt.children(parent).into_iter().rev() {
        if c.eq_ignore_ascii_case(skip) || !drawn(rt, &c, &t) {
            continue;
        }
        let (l, tp, w, h) = (rt.get(&c, "left").to_i64(), rt.get(&c, "top").to_i64(), rt.get(&c, "width").to_i64(), rt.get(&c, "height").to_i64());
        if x < l || y < tp || x >= l + w || y >= tp + h {
            continue;
        }
        let (lx, ly) = (x - l, y - tp);
        let i = inset(rt, &c);
        // (named in lower case, as every runtime keeps it)
        return deepest(rt, &c, (lx - i, ly - i), skip).or(Some((c.to_ascii_lowercase(), lx, ly)));
    }
    None
}

/// What a drag let go at (x, y) of toolbox `name`'s pixels lands on: the
/// deepest shown component there on its form (not the toolbox) and the
/// point in its pixels; the bare form: "" and the form's client
/// coordinates. `None`: off the form.
pub fn drop_target<R: Runtime>(rt: R, name: &str, (x, y): (i64, i64)) -> Option<(String, i64, i64)> {
    let (form, ox, oy) = place(rt, name)?;
    let (fx, fy) = (ox + x, oy + y);
    let (cw, ch) = (rt.get(&form, "clientwidth").to_i64(), rt.get(&form, "clientheight").to_i64());
    if fx < 0 || fy < 0 || (cw > 0 && fx >= cw) || (ch > 0 && fy >= ch) {
        return None;
    }
    Some(deepest(rt, &form, (fx, fy), name).unwrap_or((String::new(), fx, fy)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tb() -> Toolbox {
        Toolbox::default()
    }

    #[test]
    fn words_of_the_doc_find_a_component() {
        let mut t = Toolbox::default();
        t.filter = "chart".into();
        let rows = t.rows();
        assert!(!rows.is_empty(), "chart finds something");
        assert_eq!(rows[0].key, Key::Component("RPLOT".into()), "{rows:?}");
    }

    #[test]
    fn groups_come_from_the_icons_table() {
        let g = groups();
        assert_eq!(g[0].id, "rapidq");
        assert_eq!(g[0].parent, None);
        assert!(g.iter().any(|x| x.title == "Standard" && x.parent == Some("rapidq")));
        assert!(g.iter().any(|x| x.title == "Data Science" && x.parent == Some("rapidr")));
        // (RapidR's group after RapidQ's groups)
        let r = g.iter().position(|x| x.id == "rapidr").unwrap();
        assert!(g[..r].iter().all(|x| x.id == "rapidq" || x.parent == Some("rapidq")));
        assert!(members("standard").contains(&"RBUTTON"));
        // (only what the compilers create: the planned designer isn't shown)
        assert!(!members("ide").contains(&"RFORMDESIGNER") || rapidr_lang::is_component_type("RFORMDESIGNER"));
        assert!(members("ide").contains(&"RTOOLBOX"));
    }

    #[test]
    fn names_as_written_rapidr_and_titles() {
        let mut t = tb();
        assert_eq!(t.name_of("RBUTTON"), "QBUTTON");
        assert_eq!(t.name_of("RPLOT"), "RPLOT");
        t.names = Names::RapidR;
        assert_eq!(t.name_of("RBUTTON"), "RButton");
        assert_eq!(t.name_of("RSTRINGGRID"), "RStringGrid");
        t.names = Names::Titles;
        assert_eq!(t.name_of("RBUTTON"), "Button");
        // (events give what the designer writes, whatever is shown)
        assert_eq!(Toolbox::written(&Key::Component("RBUTTON".into())), "QBUTTON");
        assert_eq!(t.key_of("qbutton"), Some(Key::Component("RBUTTON".into())));
        assert_eq!(t.key_of("RBUTTON"), Some(Key::Component("RBUTTON".into())));
    }

    #[test]
    fn rows_are_groups_and_items() {
        let mut t = tb();
        let rows = t.rows();
        assert_eq!(rows[0].key, Key::Group("rapidq".into()));
        assert_eq!(rows[1].key, Key::Group("standard".into()));
        assert_eq!(rows[2].level, 3);
        assert_eq!(t.items()[0], Key::Component("RFORM".into()));
        let all = t.items().len();
        t.set_open("rapidq", Some(false));
        assert!(t.items().len() < all);
        assert_eq!(t.rows().iter().filter(|r| r.level == 1).count(), 2);
        t.set_all(false);
        assert_eq!(t.items().len(), 0);
        t.set_all(true);
        assert_eq!(t.items().len(), all);
        // (no templates: no Templates group)
        assert!(!t.rows().iter().any(|r| r.key == Key::Group("templates".into())));
        t.add_template("OK Button", "CREATE OK AS QBUTTON\nEND CREATE", "QBUTTON");
        assert!(t.rows().iter().any(|r| r.key == Key::Group("templates".into())));
        assert_eq!(t.items().last(), Some(&Key::Template("OK Button".into())));
        t.remove_template("ok button");
        assert!(t.templates.is_empty());
    }

    #[test]
    fn the_filter_ranks_the_matches() {
        let mut t = tb();
        t.set_filter("but");
        let rows = t.rows();
        assert!(rows.iter().all(|r| r.level == 1 && r.open.is_none()));
        assert_eq!(rows[0].key, Key::Component("RBUTTON".into()));
        assert_eq!(rows[0].group, "Standard");
        assert_eq!(rows[0].marks, vec![1, 2, 3]);
        assert_eq!(t.cursor, Some(Key::Component("RBUTTON".into())));
        // (titles shown: "Button" still found by "qbut")
        t.names = Names::Titles;
        t.filter.clear();
        t.set_filter("qbut");
        assert_eq!(t.rows()[0].key, Key::Component("RBUTTON".into()));
        t.set_filter("zzzz");
        assert!(t.rows().is_empty());
    }

    #[test]
    fn type_ahead_goes_to_a_name() {
        let mut t = tb();
        assert_eq!(t.type_ahead("q"), Some(Key::Component("RFORM".into())));
        // ("qb": the letters so far)
        assert_eq!(t.type_ahead("b"), Some(Key::Component("RBUTTON".into())));
        t.typed.clear();
        // (a name's Q left out)
        assert_eq!(t.type_ahead("l"), Some(Key::Component("RLABEL".into())));
        assert_eq!(t.type_ahead("i"), Some(Key::Component("RLISTBOX".into())));
        t.typed.clear();
        t.cursor = None;
        assert_eq!(t.type_ahead("s"), Some(Key::Group("standard".into())));
    }

    #[test]
    fn closing_a_group_keeps_the_keyboard_in_it() {
        let mut t = tb();
        t.cursor = Some(Key::Component("RLABEL".into()));
        t.set_open("standard", Some(false));
        assert_eq!(t.cursor, Some(Key::Group("standard".into())));
        assert_eq!(t.parent_of(&Key::Group("standard".into())), Some(Key::Group("rapidq".into())));
        assert_eq!(t.parent_of(&Key::Component("RPLOT".into())), Some(Key::Group("datascience".into())));
    }
}
