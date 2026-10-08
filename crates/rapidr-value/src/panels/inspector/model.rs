//! What an inspector shows: the properties and events read from its
//! subject ([`Prop`], [`Ev`]: a snapshot, read again whenever the subject
//! changes), and the rows they make now ([`Row`]: categories or A–Z, the
//! search applied, opened rows' parts).

use std::collections::BTreeSet;

use super::values::{self, Kind, FONT_PARTS};
use crate::panels::fuzzy;
use crate::panels::rows::ROW;
use crate::Value;

/// The category of RapidR's own members of a RapidQ component.
pub const EXTENSIONS: &str = "RapidR extensions";
/// The category of members without one.
pub const MISC: &str = "Misc";

/// The anchors' pin editor row's height.
pub const PINS_H: i64 = 112;
/// The colour picker row's height.
pub const PICKER_H: i64 = 184;

/// A property the inspector shows (the selection's, when several are
/// inspected: only the ones they all have).
#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    /// As the registry writes it (`Caption`; a font's part: `Bold`).
    pub name: String,
    pub kind: Kind,
    pub category: String,
    /// RapidR's own member of a RapidQ component (the "R" badge).
    pub ext: bool,
    /// Its value as a runtime keeps it; `None` when the inspected
    /// components' values differ.
    pub value: Option<Value>,
    /// The registry's default (a font's part: RapidQ's).
    pub default: Option<Value>,
    /// At its default: shown dimmed, nothing to reset.
    pub is_default: bool,
    /// The program's own (AddProperty): its changes only reach
    /// OnPropertyChange.
    pub custom: bool,
    pub doc: String,
    /// A font's parts (Name, Size, Color, Bold …).
    pub parts: Vec<Prop>,
}

impl Prop {
    /// Its row's key (lowercase name).
    pub fn key(&self) -> String {
        self.name.to_ascii_lowercase()
    }

    /// Its value as the row shows it ("" for differing values).
    pub fn text(&self) -> String {
        match (&self.kind, &self.value) {
            (Kind::Font, _) => {
                let part = |i: usize| self.parts.get(i).and_then(|p| p.value.clone());
                if self.parts.iter().any(|p| p.value.is_none()) {
                    return String::new();
                }
                let b = |i| part(i).is_some_and(|v| v.to_i64() != 0);
                values::font_text(&part(0).map(|v| v.to_string_val()).unwrap_or_default(), part(1).map_or(0, |v| v.to_i64()), b(3), b(4), b(5), b(6))
            }
            (k, Some(v)) => values::display(k, v),
            (_, None) => String::new(),
        }
    }

    /// Its value as the program writes it.
    pub fn source(&self) -> String {
        self.value.as_ref().map(|v| values::source(&self.kind, v)).unwrap_or_default()
    }
}

/// An event the inspector's Events page lists.
#[derive(Clone, Debug, PartialEq)]
pub struct Ev {
    pub name: String,
    /// Its parameters (the registry's, `Sender` not counted).
    pub params: Vec<String>,
    /// The SUB it runs ("" none); `None` when the inspected components'
    /// differ.
    pub handler: Option<String>,
    pub doc: String,
}

impl Ev {
    pub fn key(&self) -> String {
        format!("@{}", self.name.to_ascii_lowercase())
    }
}

/// A property the program added (AddProperty).
#[derive(Clone, Debug, PartialEq)]
pub struct Custom {
    pub name: String,
    pub kind: Kind,
    pub value: Value,
    /// What it was given (its default: dimmed while it has it).
    pub default: Value,
    pub category: String,
}

/// What a row is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// A category's heading (its open / closed state, its count).
    Category,
    /// A property.
    Prop,
    /// A font's part `i` ([`FONT_PARTS`]).
    FontPart(usize),
    /// A set's flag `i`.
    Flag(usize),
    /// A list's line `i`.
    Line(usize),
    /// A list's "add a line" row.
    AddLine,
    /// The anchors' pin editor.
    Pins,
    /// A colour's picker.
    Picker,
    /// An event (Events page).
    Event,
}

/// A row as shown now.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Its stable key: `caption`, `font.bold`, `anchors#pins`,
    /// `items#2`, `[layout]`, `@onclick`.
    pub key: String,
    pub kind: RowKind,
    /// Its property's (or event's) index.
    pub prop: usize,
    /// How deep (0: a heading or a top property in A–Z; parts one more).
    pub level: usize,
    pub name: String,
    /// Its value as shown ("" for a heading).
    pub text: String,
    /// The search's matched letters in `name`.
    pub marks: Vec<usize>,
    pub height: i64,
    /// It opens (a heading, a set, a font, a list, a colour).
    pub expandable: bool,
    pub expanded: bool,
    /// At its default (drawn dimmed).
    pub is_default: bool,
    /// Its values differ in the selection (drawn "—").
    pub mixed: bool,
    /// How many rows a heading holds.
    pub count: usize,
    /// RapidR's own member of a RapidQ component (A–Z: its badge).
    pub ext: bool,
}

impl Row {
    /// "Name=Value", a heading's "[Name]" (`Row(i)`).
    pub fn describe(&self) -> String {
        match self.kind {
            RowKind::Category => format!("[{}]", self.name),
            RowKind::Pins | RowKind::Picker => format!("({})", self.name),
            _ => format!("{}={}", self.name, self.text),
        }
    }

    /// The property name the program knows it by: `Caption`,
    /// `Font.Bold`, `OnClick`.
    pub fn prop_name(&self, props: &[Prop], events: &[Ev]) -> String {
        match self.kind {
            RowKind::Event => events.get(self.prop).map(|e| e.name.clone()).unwrap_or_default(),
            RowKind::FontPart(i) => format!("{}.{}", props.get(self.prop).map_or("Font", |p| p.name.as_str()), FONT_PARTS[i].0),
            RowKind::Category => self.name.clone(),
            _ => props.get(self.prop).map(|p| p.name.clone()).unwrap_or_default(),
        }
    }
}

/// The rows' view: categories or A–Z, which page, what's open.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct View<'a> {
    pub alphabetic: bool,
    pub events: bool,
    pub filter: &'a str,
    pub show_ext: bool,
    pub collapsed: Option<&'a BTreeSet<String>>,
    pub expanded: Option<&'a BTreeSet<String>>,
}

/// The rows `props` / `events` make in `view`.
pub fn rows(props: &[Prop], events: &[Ev], view: &View) -> Vec<Row> {
    let empty = BTreeSet::new();
    let collapsed = view.collapsed.unwrap_or(&empty);
    let expanded = view.expanded.unwrap_or(&empty);
    let filtering = !view.filter.trim().is_empty();
    let found = |name: &str| fuzzy::score(view.filter, name).map(|(_, m)| m);
    if view.events {
        return events
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let marks = found(&e.name)?;
                Some(Row {
                    key: e.key(),
                    kind: RowKind::Event,
                    prop: i,
                    level: 0,
                    name: e.name.clone(),
                    text: e.handler.clone().unwrap_or_default(),
                    marks,
                    height: ROW,
                    expandable: false,
                    expanded: false,
                    is_default: e.handler.as_deref() == Some(""),
                    mixed: e.handler.is_none(),
                    count: 0,
                    ext: false,
                })
            })
            .collect();
    }
    let shown: Vec<(usize, Vec<usize>)> = props.iter().enumerate().filter(|(_, p)| view.show_ext || !p.ext).filter_map(|(i, p)| Some((i, found(&p.name)?))).collect();
    let mut out = Vec::new();
    let push_prop = |out: &mut Vec<Row>, i: usize, marks: Vec<usize>, level: usize| {
        let p = &props[i];
        let key = p.key();
        let open = p.kind.has_parts() && expanded.contains(&key);
        out.push(Row {
            key: key.clone(),
            kind: RowKind::Prop,
            prop: i,
            level,
            name: p.name.clone(),
            text: p.text(),
            marks,
            height: ROW,
            expandable: p.kind.has_parts(),
            expanded: open,
            is_default: p.is_default,
            mixed: if p.kind == Kind::Font { p.parts.iter().any(|q| q.value.is_none()) } else { p.value.is_none() },
            count: 0,
            ext: p.ext,
        });
        if open {
            parts(out, p, i, level + 1);
        }
    };
    if view.alphabetic {
        let mut order = shown;
        order.sort_by_key(|(i, _)| props[*i].name.to_ascii_lowercase());
        for (i, marks) in order {
            push_prop(&mut out, i, marks, 0);
        }
        return out;
    }
    // (categories A–Z, RapidR's extensions last; each one's rows A–Z)
    let mut cats: Vec<&str> = shown.iter().map(|(i, _)| props[*i].category.as_str()).collect();
    cats.sort_by_key(|c| (*c == EXTENSIONS, c.to_ascii_lowercase()));
    cats.dedup();
    for cat in cats {
        let mut members: Vec<&(usize, Vec<usize>)> = shown.iter().filter(|(i, _)| props[*i].category == cat).collect();
        members.sort_by_key(|(i, _)| props[*i].name.to_ascii_lowercase());
        let key = format!("[{}]", cat.to_ascii_lowercase());
        // (a search opens every category it finds something in)
        let open = filtering || !collapsed.contains(&key);
        out.push(Row {
            key,
            kind: RowKind::Category,
            prop: members.first().map_or(0, |m| m.0),
            level: 0,
            name: cat.to_string(),
            text: String::new(),
            marks: Vec::new(),
            height: ROW,
            expandable: true,
            expanded: open,
            is_default: true,
            mixed: false,
            count: members.len(),
            ext: cat == EXTENSIONS,
        });
        if open {
            for (i, marks) in members {
                push_prop(&mut out, *i, marks.clone(), 1);
            }
        }
    }
    out
}

/// Prop `p`'s (index `i`) parts, opened, at `level`.
fn parts(out: &mut Vec<Row>, p: &Prop, i: usize, level: usize) {
    let key = p.key();
    let row = |key: String, kind: RowKind, name: String, text: String, is_default: bool, height: i64| Row {
        key,
        kind,
        prop: i,
        level,
        name,
        text,
        marks: Vec::new(),
        height,
        expandable: false,
        expanded: false,
        is_default,
        mixed: false,
        count: 0,
        ext: false,
    };
    match &p.kind {
        Kind::Set(values) if key == "anchors" && *values == ["akLeft", "akTop", "akRight", "akBottom"] => {
            out.push(row(format!("{key}#pins"), RowKind::Pins, "Anchors editor".into(), p.text(), p.is_default, PINS_H));
        }
        Kind::Set(values) => {
            let mask = p.value.as_ref().map(|v| v.to_i64());
            for (f, flag) in values.iter().enumerate() {
                let on = mask.map(|m| m & values::flag_bit(values, f) != 0);
                let text = on.map(|b| if b { "True" } else { "False" }).unwrap_or_default().to_string();
                let dflt = p.default.as_ref().map(|d| d.to_i64() & values::flag_bit(values, f) != 0);
                let mut r = row(format!("{key}.{}", flag.to_ascii_lowercase()), RowKind::Flag(f), flag.clone(), text, on == dflt || on.is_none(), ROW);
                r.mixed = on.is_none();
                out.push(r);
            }
        }
        Kind::Font => {
            for (f, part) in p.parts.iter().enumerate() {
                let mut r = row(format!("{key}.{}", part.name.to_ascii_lowercase()), RowKind::FontPart(f), part.name.clone(), part.text(), part.is_default, ROW);
                r.mixed = part.value.is_none();
                out.push(r);
            }
        }
        Kind::Strings | Kind::Columns => {
            let text = p.value.as_ref().map(|v| v.to_string_val()).unwrap_or_default();
            for (n, line) in values::lines(&text).into_iter().enumerate() {
                out.push(row(format!("{key}#{n}"), RowKind::Line(n), format!("[{n}]"), line, false, ROW));
            }
            let add = if p.kind == Kind::Columns { "Add a column" } else { "Add a line" };
            out.push(row(format!("{key}#+"), RowKind::AddLine, add.into(), String::new(), true, ROW));
        }
        Kind::Color => {
            out.push(row(format!("{key}#picker"), RowKind::Picker, "Colour picker".into(), p.text(), p.is_default, PICKER_H));
        }
        _ => {}
    }
}
