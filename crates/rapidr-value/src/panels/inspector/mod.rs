//! RPROPERTYINSPECTOR's model (docs/ide-components.md §3.4): Delphi's object
//! inspector as a public component — the data the kernel draws
//! (`rapidr-ui-kernel`'s `components/panels/inspector/`) and the program's
//! members and events through the runtime glue.
//!
//! - What it shows comes from the language registry (which properties and
//!   events a type has, their kinds, constants, defaults, categories,
//!   origin) and from its subject ([`super::subject`]: the program's live
//!   components named by `Target`, or a designer's selection through
//!   `Designer`), read into a snapshot ([`read`]) whenever the subject
//!   changes; [`model`] makes the rows (categories or A–Z, the search, the
//!   parts of opened rows); [`values`] spells values three ways (as a row
//!   shows them, as the program writes them, as a runtime keeps them);
//!   [`design`] is RDESIGNSURFACE's subject.
//! - A change the user makes ([`User::Commit`], [`User::Reset`], a pick
//!   from a dropped list) goes to the subject, then OnPropertyChange (Prop,
//!   Value) with the value as the program writes it.

pub mod design;
pub mod designer_model;
pub mod model;
pub mod read;
pub mod values;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

pub use model::{Custom, Ev, Prop, Row, RowKind};
use read::Snapshot;
pub use values::Kind;

use super::runtime::{basic_bool, component_arg, truth, Runtime};
use super::subject::{default_value, designer_subject, Host, Live, Subject};
use crate::objects::ops::Rect;
use crate::{v_int, v_str, Value};

/// RPROPERTYINSPECTOR's state.
#[derive(Clone, Debug, PartialEq)]
pub struct Inspector {
    /// `Target`: the inspected components' names (commas between).
    pub target: String,
    /// `Designer`: the designer whose selection it follows ("" none).
    pub designer: String,
    /// `View = "alphabetic"` (else by categories).
    pub alphabetic: bool,
    /// `Page = "events"`.
    pub events_page: bool,
    /// `Filter`: the search box's text.
    pub filter: String,
    pub show_events: bool,
    pub show_ext: bool,
    pub read_only: bool,
    /// `Handlers`: the SUBs events can run, a line each (`Name(params)`).
    pub handlers: String,
    pub name_width: i64,
    /// AddProperty's.
    pub custom: Vec<Custom>,
    /// What the subject had when last read.
    pub snap: Snapshot,
    /// The user's (the kernel's): selection, what's open, the mouse.
    pub ui: Ui,
}

impl Default for Inspector {
    fn default() -> Self {
        Inspector {
            target: String::new(),
            designer: String::new(),
            alphabetic: false,
            events_page: false,
            filter: String::new(),
            show_events: true,
            show_ext: true,
            read_only: false,
            handlers: String::new(),
            name_width: 120,
            custom: Vec::new(),
            snap: Snapshot::default(),
            ui: Ui::default(),
        }
    }
}

/// What the mouse is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hover {
    Row(usize),
    /// Row i's reset glyph.
    Reset(usize),
    /// Row i's drop / "…" button.
    Button(usize),
    /// Row i's check box.
    Check(usize),
    /// The line between the columns.
    Divider,
    Tab(usize),
    Search,
    /// The view's button (categories / A–Z).
    ViewButton,
    /// An anchor's pin (0 left, 1 top, 2 right, 3 bottom).
    Pin(usize),
    /// A colour picker's swatch.
    Swatch(usize),
}

/// The user's state of an inspector (the kernel changes it directly).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ui {
    /// The selected row's key.
    pub selected: Option<String>,
    /// Closed categories' keys (`[layout]`).
    pub collapsed: BTreeSet<String>,
    /// Opened rows' keys (`font`, `anchors`).
    pub expanded: BTreeSet<String>,
    pub hover: Option<Hover>,
    /// The anchors editor's focused pin (0 left, 1 top, 2 right, 3 bottom).
    pub pin: usize,
    /// The colour picker's focused swatch.
    pub swatch: usize,
    /// The divider dragged: where it was pressed, NameWidth then.
    pub drag: Option<(i64, i64)>,
    /// A value refused: its row's key and why.
    pub error: Option<(String, String)>,
    /// The row whose list is dropped, its items.
    pub dropped: Option<String>,
    pub drop_items: Vec<String>,
    /// The item to light when the list shows (the current value).
    pub drop_hot: Option<usize>,
    /// EditValue: the row whose editor the kernel opens.
    pub edit_request: Option<String>,
    /// The selected row is to be scrolled into view.
    pub reveal: bool,
}

crate::panel_models!(Inspector);

/// What the user did to it (from the kernel).
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// A value typed, picked or toggled for row `key` (`caption`,
    /// `font.bold`, `@onclick`), `text` as the program writes it.
    Commit { key: String, text: String },
    /// Row `key` back to its default (Delete, the reset glyph).
    Reset(String),
    /// The selected row changed: OnSelect.
    Select(String),
    /// An event's row double-clicked (Enter on an empty one):
    /// OnEventDblClick.
    EventDblClick(String),
    /// A row's "…" button: OnEditorRequest.
    EditorRequest(String),
    /// Row `key`'s list asked for (its button, Alt+Down), under `anchor`
    /// (absolute in the form's client area).
    Drop { key: String, anchor: Rect },
    /// The designer this was sent for (its name the panel's) changed its
    /// selection or a selected component's property (runtime glue's hook).
    DesignerChanged,
}

/// A change a commit makes.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    /// As the program names it: `Caption`, `Font.Bold`, `OnClick`.
    pub prop: String,
    pub value: Value,
    /// As the program writes it.
    pub source: String,
    /// An event's SUB.
    pub event: bool,
    /// AddProperty's.
    pub custom: bool,
}

impl Inspector {
    /// The rows shown now.
    pub fn rows(&self) -> Vec<Row> {
        let view = model::View {
            alphabetic: self.alphabetic,
            events: self.events_page && self.show_events,
            filter: &self.filter,
            show_ext: self.show_ext,
            collapsed: Some(&self.ui.collapsed),
            expanded: Some(&self.ui.expanded),
        };
        model::rows(&self.snap.props, &self.snap.events, &view)
    }

    /// The Events page is shown.
    pub fn on_events(&self) -> bool {
        self.events_page && self.show_events
    }

    /// The property named `name` (`Caption`): its index.
    pub fn prop_index(&self, name: &str) -> Option<usize> {
        self.snap.props.iter().position(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// The row key a program's name means: `Caption` → `caption`,
    /// `Font.Bold` → `font.bold`, `OnClick` → `@onclick` (an event of the
    /// type), a category's name → `[layout]`.
    pub fn key_of(&self, name: &str) -> Option<String> {
        let low = name.trim().to_ascii_lowercase();
        if self.prop_index(&low).is_some() {
            return Some(low);
        }
        if let Some((p, part)) = low.split_once('.') {
            let i = self.prop_index(p)?;
            let prop = &self.snap.props[i];
            let known = match &prop.kind {
                Kind::Font => prop.parts.iter().any(|q| q.name.eq_ignore_ascii_case(part)),
                Kind::Set(values) => values.iter().any(|v| v.eq_ignore_ascii_case(part)),
                _ => false,
            };
            return known.then_some(low);
        }
        if self.snap.events.iter().any(|e| e.name.eq_ignore_ascii_case(&low)) {
            return Some(format!("@{low}"));
        }
        let cat = format!("[{low}]");
        self.snap.props.iter().any(|p| p.category.eq_ignore_ascii_case(&low)).then_some(cat)
    }

    /// The selected row's index among `rows`.
    pub fn selected_in(&self, rows: &[Row]) -> Option<usize> {
        let key = self.ui.selected.as_deref()?;
        rows.iter().position(|r| r.key == key)
    }

    /// The property row `key` (or its part, line, editor) belongs to.
    pub fn prop_of_key(&self, key: &str) -> Option<usize> {
        let base = key.split(['.', '#']).next()?;
        self.prop_index(base).filter(|_| !key.starts_with('@') && !key.starts_with('['))
    }

    /// Opens / closes row (or category) `key`.
    pub fn set_open(&mut self, key: &str, open: bool) {
        if key.starts_with('[') {
            if open {
                self.ui.collapsed.remove(key);
            } else {
                self.ui.collapsed.insert(key.to_string());
            }
        } else if open {
            self.ui.expanded.insert(key.to_string());
        } else {
            self.ui.expanded.remove(key);
        }
    }

    /// Opens (or closes) every category and every row with parts.
    pub fn set_all_open(&mut self, open: bool) {
        let cats: Vec<String> = self.snap.props.iter().map(|p| format!("[{}]", p.category.to_ascii_lowercase())).collect();
        let opens: Vec<String> = self.snap.props.iter().filter(|p| p.kind.has_parts()).map(Prop::key).collect();
        for k in cats.iter().chain(&opens) {
            self.set_open(k, open);
        }
    }

    /// What committing `text` (typed, picked or toggled) to row `key`
    /// changes; `Err`: why it can't.
    pub fn interpret(&self, key: &str, text: &str) -> Result<Change, String> {
        if self.read_only {
            return Err("The inspector is read-only".into());
        }
        if let Some(ev) = key.strip_prefix('@') {
            let e = self.snap.events.iter().find(|e| e.name.eq_ignore_ascii_case(ev)).ok_or("No such event")?;
            let sub = text.trim();
            if !sub.is_empty() && !sub.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                return Err(format!("\"{sub}\" is not a SUB's name"));
            }
            return Ok(Change { prop: e.name.clone(), value: v_str(sub), source: sub.to_string(), event: true, custom: false });
        }
        let i = self.prop_of_key(key).ok_or("No such property")?;
        let p = &self.snap.props[i];
        let current = p.value.clone().unwrap_or_else(|| p.kind.empty());
        let whole = |value: Value| Change { prop: p.name.clone(), source: values::source(&p.kind, &value), value, event: false, custom: p.custom };
        let rest = &key[p.key().len()..];
        if let Some(part) = rest.strip_prefix('.') {
            match &p.kind {
                Kind::Font => {
                    let q = p.parts.iter().find(|q| q.name.eq_ignore_ascii_case(part)).ok_or("No such part")?;
                    let value = values::parse(&q.kind, text)?;
                    return Ok(Change { prop: format!("{}.{}", p.name, q.name), source: values::source(&q.kind, &value), value, event: false, custom: p.custom });
                }
                Kind::Set(flags) => {
                    let f = flags.iter().position(|v| v.eq_ignore_ascii_case(part)).ok_or("No such flag")?;
                    let on = values::parse_bool(text).ok_or_else(|| format!("\"{text}\" is not True or False"))?;
                    let bit = values::flag_bit(flags, f);
                    let mask = current.to_i64();
                    return Ok(whole(Value::Integer(if on { mask | bit } else { mask & !bit })));
                }
                _ => return Err("No such part".into()),
            }
        }
        if let Some(line) = rest.strip_prefix('#') {
            let mut lines = values::lines(&current.to_string_val());
            match line {
                "+" => lines.push(text.to_string()),
                "pins" | "picker" => return Ok(whole(values::parse(&p.kind, text)?)),
                n => {
                    let n: usize = n.parse().map_err(|_| "No such line")?;
                    *lines.get_mut(n).ok_or("No such line")? = text.to_string();
                }
            }
            return Ok(whole(Value::String(lines.join("\r\n"))));
        }
        if matches!(p.kind, Kind::Font) {
            return Err("A font is changed by its parts".into());
        }
        Ok(whole(values::parse(&p.kind, text)?))
    }

    /// Row `key`'s value as shown ("" none).
    pub fn value_text(&self, key: &str) -> Option<String> {
        if let Some(ev) = key.strip_prefix('@') {
            return self.snap.events.iter().find(|e| e.name.eq_ignore_ascii_case(ev)).map(|e| e.handler.clone().unwrap_or_default());
        }
        let p = &self.snap.props[self.prop_of_key(key)?];
        let rest = &key[p.key().len()..];
        match rest.strip_prefix('.') {
            Some(part) => match &p.kind {
                Kind::Font => p.parts.iter().find(|q| q.name.eq_ignore_ascii_case(part)).map(Prop::text),
                Kind::Set(flags) => {
                    let f = flags.iter().position(|v| v.eq_ignore_ascii_case(part))?;
                    let on = p.value.as_ref()?.to_i64() & values::flag_bit(flags, f) != 0;
                    Some(if on { "True" } else { "False" }.into())
                }
                _ => None,
            },
            None => Some(p.text()),
        }
    }

    /// Row `key`'s value as the program writes it.
    pub fn source_text(&self, key: &str) -> Option<String> {
        let p = &self.snap.props[self.prop_of_key(key)?];
        match key[p.key().len()..].strip_prefix('.') {
            Some(part) if p.kind == Kind::Font => p.parts.iter().find(|q| q.name.eq_ignore_ascii_case(part)).map(Prop::source),
            Some(_) => self.value_text(key),
            None => Some(p.source()),
        }
    }

    /// Whether row `key`'s property is at its default.
    pub fn is_default(&self, key: &str) -> bool {
        if let Some(ev) = key.strip_prefix('@') {
            return self.snap.events.iter().find(|e| e.name.eq_ignore_ascii_case(ev)).is_none_or(|e| e.handler.as_deref() == Some(""));
        }
        let Some(i) = self.prop_of_key(key) else { return true };
        let p = &self.snap.props[i];
        match key[p.key().len()..].strip_prefix('.') {
            Some(part) if p.kind == Kind::Font => p.parts.iter().find(|q| q.name.eq_ignore_ascii_case(part)).is_none_or(|q| q.is_default),
            _ => p.is_default,
        }
    }

    /// The SUBs an event with `params` parameters can run: `Handlers`' or
    /// the subject's, those whose parameters fit (as many, or one more
    /// first: Sender).
    pub fn fitting_subs(&self, subs: Option<Vec<String>>, params: usize) -> Vec<String> {
        let lines = subs.unwrap_or_else(|| values::lines(&self.handlers));
        lines
            .iter()
            .filter_map(|l| {
                let l = l.trim();
                let (name, args) = match l.split_once('(') {
                    Some((n, a)) => (n.trim(), a.trim_end_matches(')').trim()),
                    None => (l, ""),
                };
                let n = if args.is_empty() { 0 } else { args.split(',').count() };
                (!name.is_empty() && (n == params || n == params + 1)).then(|| name.to_string())
            })
            .collect()
    }

    /// The names its Target lists.
    pub fn target_names(&self) -> Vec<String> {
        self.target.split(',').map(str::trim).filter(|n| !n.is_empty()).map(str::to_string).collect()
    }
}

// --------------------------------------------------------- the subject --

thread_local! {
    static DESIGN_REGISTERED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The subject inspector `m` shows: its designer's selection, else its
/// Target's components, live.
fn subject_of(host: &dyn Host, designer: &str, target: &[String]) -> Option<Box<dyn Subject>> {
    if !designer.is_empty() {
        if !DESIGN_REGISTERED.with(|r| r.replace(true)) {
            design::register();
        }
        return designer_subject(&host.type_of(designer), designer);
    }
    (!target.is_empty()).then(|| Box::new(Live { names: target.to_vec() }) as Box<dyn Subject>)
}

/// Reads inspector `name`'s subject again (its rows follow).
pub fn refresh(host: &dyn Host, name: &str) {
    let Some((designer, target, custom)) = with(name, |m| (m.designer.clone(), m.target_names(), m.custom.clone())) else { return };
    let subject = subject_of(host, &designer, &target);
    let snap = read::read(host, subject.as_deref(), &custom);
    with_mut(name, |m| {
        m.snap = snap;
        // (a row gone: nothing selected)
        if let Some(sel) = m.ui.selected.clone() {
            let rows = m.rows();
            if !rows.iter().any(|r| r.key == sel) && m.prop_of_key(&sel).is_none() && m.key_of(sel.trim_start_matches('@')).is_none() {
                m.ui.selected = None;
            }
        }
    });
    host.invalidate();
}

/// Commits `text` to row `key` of inspector `name` (typed, picked,
/// toggled, SetValue): the subject changes, OnPropertyChange. `Err`: why
/// it was refused (the row shows it).
pub fn commit(host: &dyn Host, name: &str, key: &str, text: &str) -> Result<(), String> {
    let Some(prep) = with(name, |m| (m.interpret(key, text), m.designer.clone(), m.target_names(), m.snap.objects.iter().map(|o| o.0.clone()).collect::<Vec<_>>())) else { return Err("No inspector".into()) };
    let (change, designer, target, objects) = prep;
    let change = match change {
        Ok(c) => c,
        Err(e) => {
            with_mut(name, |m| m.ui.error = Some((key.to_string(), e.clone())));
            host.invalidate();
            return Err(e);
        }
    };
    if change.custom {
        with_mut(name, |m| {
            if let Some(c) = m.custom.iter_mut().find(|c| c.name.eq_ignore_ascii_case(&change.prop)) {
                c.value = change.value.clone();
            }
        });
    } else if let Some(s) = subject_of(host, &designer, &target) {
        if change.event {
            s.set_handler(host, &objects, &change.prop, &change.source);
        } else if let Err(e) = s.set_source(host, &objects, &change.prop, &change.value, &change.source) {
            with_mut(name, |m| m.ui.error = Some((key.to_string(), e.clone())));
            host.invalidate();
            return Err(e);
        }
    }
    with_mut(name, |m| m.ui.error = None);
    refresh(host, name);
    host.fire(name, "onpropertychange", &[v_str(&change.prop), v_str(&change.source)]);
    Ok(())
}

/// Puts row `key`'s property back to its default; OnPropertyChange with
/// the value it has then.
pub fn reset(host: &dyn Host, name: &str, key: &str) {
    if let Some(ev) = key.strip_prefix('@') {
        let ev = ev.to_string();
        let _ = commit(host, name, &format!("@{ev}"), "");
        return;
    }
    let Some(prep) = with(name, |m| {
        if m.read_only {
            return None;
        }
        let i = m.prop_of_key(key)?;
        let p = &m.snap.props[i];
        let part = key[p.key().len()..].strip_prefix('.').map(str::to_string);
        let (prop, fallback, kind) = match (&part, &p.kind) {
            (Some(part), Kind::Font) => {
                let q = p.parts.iter().find(|q| q.name.eq_ignore_ascii_case(part))?;
                (format!("{}.{}", p.name, q.name), q.default.clone().unwrap_or_else(|| q.kind.empty()), q.kind.clone())
            }
            _ => (p.name.clone(), p.default.clone().unwrap_or_else(|| p.kind.empty()), p.kind.clone()),
        };
        Some((prop, fallback, kind, p.custom, m.designer.clone(), m.target_names(), m.snap.objects.clone()))
    })
    .flatten() else {
        return;
    };
    let (prop, fallback, kind, custom, designer, target, objects) = prep;
    let names: Vec<String> = objects.iter().map(|o| o.0.clone()).collect();
    if custom {
        with_mut(name, |m| {
            if let Some(c) = m.custom.iter_mut().find(|c| c.name.eq_ignore_ascii_case(&prop)) {
                c.value = c.default.clone();
            }
        });
    } else if let Some(s) = subject_of(host, &designer, &target) {
        // (the program's components: a property without a registry default
        // goes back to its type's empty value; a font's part to RapidQ's)
        let registry = objects.first().and_then(|(_, t)| default_value(t, &prop));
        if designer.is_empty() && registry.is_none() {
            let _ = s.set_source(host, &names, &prop, &fallback, &values::source(&kind, &fallback));
        } else {
            s.reset(host, &names, &prop);
        }
    }
    refresh(host, name);
    let source = with(name, |m| m.key_of(&prop).and_then(|k| m.source_text(&k))).flatten().unwrap_or_default();
    host.fire(name, "onpropertychange", &[v_str(&prop), v_str(&source)]);
}

/// Every inspector following designer `designer` reads it again (the
/// designer's selection, or a selected component's property, changed:
/// super::subject).
pub fn designer_changed(host: &dyn Host, designer: &str) {
    let names: Vec<String> = MODELS.with(|m| m.borrow().iter().filter(|(_, i)| i.designer.eq_ignore_ascii_case(designer)).map(|(n, _)| n.clone()).collect());
    for n in names {
        refresh(host, &n);
    }
}

// ------------------------------------------------- the runtime's glue --

/// Its methods; `None`: not one of its own.
pub fn rt_method<R: Runtime>(rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let text = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    with_mut(name, |_| ());
    Some(match method {
        "refresh" => {
            refresh(&rt, name);
            Value::Null
        }
        "expandall" | "collapseall" => {
            with_mut(name, |m| m.set_all_open(method == "expandall"));
            Value::Null
        }
        "expand" | "collapse" => {
            with_mut(name, |m| {
                if let Some(k) = m.key_of(&text(0)) {
                    m.set_open(&k, method == "expand");
                }
            });
            Value::Null
        }
        "value" => v_str(&with(name, |m| m.key_of(&text(0)).and_then(|k| m.value_text(&k))).flatten().unwrap_or_default()),
        "setvalue" => {
            let key = with(name, |m| m.key_of(&text(0))).flatten();
            basic_bool(key.is_some_and(|k| commit(&rt, name, &k, &text(1)).is_ok()))
        }
        "resetvalue" => {
            if let Some(k) = with(name, |m| m.key_of(&text(0))).flatten() {
                reset(&rt, name, &k);
            }
            Value::Null
        }
        "isdefault" => basic_bool(with(name, |m| m.key_of(&text(0)).map(|k| m.is_default(&k))).flatten().unwrap_or(true)),
        "editvalue" => {
            with_mut(name, |m| {
                let Some(k) = m.key_of(&text(0)) else { return };
                m.events_page = k.starts_with('@');
                reveal(m, &k);
                m.ui.edit_request = Some(k);
            });
            rt.focus(name);
            Value::Null
        }
        "addproperty" => {
            let kind = Kind::named(&text(1));
            let value = values::parse(&kind, &text(2)).unwrap_or_else(|_| v_str(&text(2)));
            with_mut(name, |m| {
                m.custom.retain(|c| !c.name.eq_ignore_ascii_case(&text(0)));
                m.custom.push(Custom { name: text(0), kind, default: value.clone(), value, category: text(3) });
            });
            refresh(&rt, name);
            Value::Null
        }
        "clearproperties" => {
            with_mut(name, |m| m.custom.clear());
            refresh(&rt, name);
            Value::Null
        }
        "row" => {
            let i = args.first().map_or(-1, Value::to_i64);
            v_str(&with(name, |m| usize::try_from(i).ok().and_then(|i| m.rows().get(i).map(Row::describe))).flatten().unwrap_or_default())
        }
        _ => return None,
    })
}

/// Selects row `key` of `m`, opening what hides it, scrolled into view.
pub fn reveal(m: &mut Inspector, key: &str) {
    if let Some(i) = m.prop_of_key(key) {
        let p = m.snap.props[i].clone();
        if !m.alphabetic {
            m.ui.collapsed.remove(&format!("[{}]", p.category.to_ascii_lowercase()));
        }
        if key != p.key() {
            m.ui.expanded.insert(p.key());
        }
        if !m.show_ext && p.ext {
            m.show_ext = true;
        }
    }
    m.ui.selected = Some(key.to_string());
    m.ui.reveal = true;
}

/// What row `row` of `m` is, in words: a property's registry doc, an
/// event's doc and parameters (the footer under the rows; `Doc`).
pub fn row_doc(m: &Inspector, row: &Row) -> String {
    match row.kind {
        RowKind::Event => m.snap.events.get(row.prop).map(|e| {
            let params = if e.params.is_empty() { "no parameters".to_string() } else { e.params.join(", ") };
            if e.doc.is_empty() {
                format!("Runs a SUB with {params}.")
            } else {
                format!("{} ({params})", e.doc)
            }
        }),
        RowKind::Category => Some(format!("{} properties", row.count)),
        _ => m.snap.props.get(row.prop).map(|p| p.doc.clone()),
    }
    .unwrap_or_default()
    // (the registry's docs mark code with backquotes)
    .replace('`', "")
}

/// Its properties its model answers.
pub fn rt_get<R: Runtime>(_rt: R, name: &str, prop: &str) -> Option<Value> {
    let m = with(name, Clone::clone).unwrap_or_default();
    Some(match prop {
        // (following a designer: the components it has selected)
        "target" if !m.designer.is_empty() => v_str(&m.snap.objects.iter().map(|o| o.0.as_str()).collect::<Vec<_>>().join(",")),
        "target" => v_str(&m.target),
        "designer" => v_str(&m.designer),
        "rows" => v_str(&m.rows().iter().map(Row::describe).collect::<Vec<_>>().join("\n")),
        "doc" => {
            let rows = m.rows();
            v_str(&m.selected_in(&rows).map(|i| row_doc(&m, &rows[i])).unwrap_or_default())
        }
        "view" => v_str(if m.alphabetic { "alphabetic" } else { "categories" }),
        "page" => v_str(if m.events_page { "events" } else { "properties" }),
        "filter" => v_str(&m.filter),
        "showevents" => basic_bool(m.show_events),
        "showrapidrextensions" => basic_bool(m.show_ext),
        "readonly" => basic_bool(m.read_only),
        "handlers" => v_str(&m.handlers),
        "namewidth" => v_int(m.name_width),
        "selected" => {
            let rows = m.rows();
            v_str(&m.selected_in(&rows).map(|i| rows[i].prop_name(&m.snap.props, &m.snap.events)).unwrap_or_default())
        }
        "rowcount" => v_int(m.rows().len() as i64),
        "targettype" => v_str(&m.snap.type_name),
        _ => return None,
    })
}

/// Its properties its model keeps: whether `prop` was one.
pub fn rt_set<R: Runtime>(rt: R, name: &str, prop: &str, v: &Value) -> bool {
    let text = v.to_string_val();
    let again = match prop {
        "target" => {
            let names: Vec<String> = match v {
                Value::Integer(_) | Value::Double(_) => vec![component_arg(v)],
                _ => text.split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect(),
            };
            with_mut(name, |m| {
                m.target = names.join(",");
                m.designer.clear();
            });
            true
        }
        "designer" => {
            let d = if text.is_empty() { String::new() } else { component_arg(v) };
            with_mut(name, |m| m.designer = d);
            true
        }
        "view" => {
            with_mut(name, |m| m.alphabetic = text.trim().eq_ignore_ascii_case("alphabetic"));
            false
        }
        "page" => {
            with_mut(name, |m| m.events_page = text.trim().eq_ignore_ascii_case("events"));
            false
        }
        "filter" => {
            with_mut(name, |m| m.filter = text);
            false
        }
        "showevents" => {
            with_mut(name, |m| m.show_events = truth(v));
            false
        }
        "showrapidrextensions" => {
            with_mut(name, |m| m.show_ext = truth(v));
            false
        }
        "readonly" => {
            with_mut(name, |m| m.read_only = truth(v));
            false
        }
        "handlers" => {
            with_mut(name, |m| m.handlers = text);
            false
        }
        "namewidth" => {
            with_mut(name, |m| m.name_width = v.to_i64().max(24));
            false
        }
        "selected" => {
            with_mut(name, |m| match m.key_of(&text) {
                Some(k) => reveal(m, &k),
                None => m.ui.selected = None,
            });
            false
        }
        _ => return false,
    };
    if again {
        refresh(&rt, name);
    }
    true
}

/// What the user did.
pub fn rt_user<R: Runtime>(rt: R, name: &str, action: User) {
    match action {
        User::Commit { key, text } => {
            let _ = commit(&rt, name, &key, &text);
        }
        User::Reset(key) => reset(&rt, name, &key),
        User::Select(key) => {
            let prop = with(name, |m| {
                let rows = m.rows();
                rows.iter().find(|r| r.key == key).map(|r| r.prop_name(&m.snap.props, &m.snap.events))
            })
            .flatten();
            if let Some(p) = prop {
                rt.fire(name, "onselect", &[v_str(&p)]);
            }
        }
        User::EventDblClick(ev) => rt.fire(name, "oneventdblclick", &[v_str(&ev)]),
        User::EditorRequest(prop) => rt.fire(name, "oneditorrequest", &[v_str(&prop)]),
        User::Drop { key, anchor } => drop_list(rt, name, &key, anchor),
        User::DesignerChanged => designer_changed(&rt, name),
    }
}

/// Drops row `key`'s list: an enum's constants, True / False, the
/// components a reference may name, the SUBs an event may run.
fn drop_list<R: Runtime>(rt: R, name: &str, key: &str, anchor: Rect) {
    let Some(m) = with(name, Clone::clone) else { return };
    if m.read_only {
        return;
    }
    let current = m.value_text(key).unwrap_or_default();
    let items: Vec<String> = if let Some(ev) = key.strip_prefix('@') {
        let Some(e) = m.snap.events.iter().find(|e| e.name.eq_ignore_ascii_case(ev)) else { return };
        let subject = subject_of(&rt, &m.designer, &m.target_names());
        let subs = subject.as_ref().and_then(|s| s.subs(&rt));
        let mut items = m.fitting_subs(subs, e.params.len());
        if !items.is_empty() && !current.is_empty() {
            items.insert(0, "(none)".into());
        }
        items
    } else {
        let Some(i) = m.prop_of_key(key) else { return };
        let p = &m.snap.props[i];
        let part = &key[p.key().len()..];
        let kind = match (&p.kind, part.strip_prefix('.')) {
            (Kind::Font, Some(q)) => p.parts.iter().find(|x| x.name.eq_ignore_ascii_case(q)).map(|x| x.kind.clone()).unwrap_or(Kind::Text),
            (Kind::Set(_), Some(_)) => Kind::Bool,
            (k, _) => k.clone(),
        };
        match kind {
            Kind::Enum(values) => values,
            Kind::Bool => vec!["False".into(), "True".into()],
            Kind::Component(kinds) => {
                let subject = subject_of(&rt, &m.designer, &m.target_names());
                let comps = subject.map(|s| s.components(&rt)).unwrap_or_default();
                let fits = |t: &str| kinds.is_empty() || kinds.iter().any(|k| rapidr_lang::component(k).zip(rapidr_lang::component(t)).is_some_and(|(a, b)| a.name == b.name));
                let inspected: Vec<&str> = m.snap.objects.iter().map(|o| o.0.as_str()).collect();
                let mut items = vec!["(none)".to_string()];
                items.extend(comps.into_iter().filter(|(n, t)| fits(t) && !inspected.iter().any(|o| o.eq_ignore_ascii_case(n))).map(|(n, _)| n));
                items
            }
            Kind::Color => values::STANDARD_COLORS.iter().chain(values::SYSTEM_COLORS.iter()).map(|s| s.to_string()).collect(),
            _ => return,
        }
    };
    if items.is_empty() {
        return;
    }
    let hot = items.iter().position(|i| i.eq_ignore_ascii_case(&current));
    with_mut(name, |m| {
        m.ui.dropped = Some(key.to_string());
        m.ui.drop_items = items.clone();
        m.ui.drop_hot = hot;
    });
    let form = rt.form_of(name).unwrap_or_default();
    rt.drop_list(&form, name, items, anchor);
}

/// Any component's property was set: an inspector showing it follows.
pub fn rt_after_set<R: Runtime>(rt: R, name: &str, _prop: &str) {
    let names: Vec<String> = MODELS.with(|m| {
        m.borrow()
            .iter()
            .filter(|(_, i)| i.designer.is_empty() && i.target_names().iter().any(|t| t.eq_ignore_ascii_case(name)))
            .map(|(n, _)| n.clone())
            .collect()
    });
    for n in names {
        refresh(&rt, &n);
    }
}

/// An item picked from a list it dropped.
pub fn rt_picked<R: Runtime>(rt: R, name: &str, item: String) {
    let Some(key) = with_mut(name, |m| {
        m.ui.drop_items.clear();
        m.ui.drop_hot = None;
        m.ui.dropped.take()
    }) else {
        return;
    };
    let text = if item == "(none)" { String::new() } else { item };
    let _ = commit(&rt, name, &key, &text);
}
