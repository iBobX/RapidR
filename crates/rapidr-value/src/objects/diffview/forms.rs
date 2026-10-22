//! RDIFFVIEW's form view (Kind = "form", docs/versions-plan.md §2: "diffs
//! that understand RapidR"): two versions of a program's CREATE blocks
//! compared as components and properties (`rapidr_diff::form`), shown as
//! the form before and after — drawn as the designer draws it, the
//! changed components outlined — above the list of what changed, in
//! words.
//!
//! The texts are read by the designer's source reader (the one
//! RDESIGNSURFACE uses: RapidR Studio's runtimes install it,
//! `objects::design::set_source_reader`); without it the view says so.
//! Every top-level CREATE block counts (a file can make several forms);
//! the form drawn is the first one with a change.

use rapidr_diff::form::{self, Component, ComponentChange, Kind};

use crate::designer::Designer;
use crate::objects::design::DesignSurface;

/// The gap around the two pictures, and above them for their labels.
pub const PAD: i64 = 12;
pub const LABEL_H: i64 = 22;
/// A change's own row, and each of its details' rows.
pub const ITEM_H: i64 = 26;
pub const DETAIL_H: i64 = 20;

/// What a row of the list shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    /// Change `c`'s heading: "+ Label3 (RLabel)".
    Change(usize),
    /// Change `c`'s property `p`.
    Prop(usize, usize),
    /// Change `c`'s moved / retyped / added or removed lines: its `k`-th
    /// other detail.
    Detail(usize, usize),
}

/// The two forms compared.
#[derive(Default)]
pub struct FormCompare {
    pub changes: Vec<ComponentChange>,
    /// The form drawn before and after (`None`: that side has no such form).
    pub before: Option<Box<DesignSurface>>,
    pub after: Option<Box<DesignSurface>>,
    /// The drawn form's name.
    pub form: String,
    /// Why something can't be shown ("" when all is well).
    pub note: String,
    /// The list's rows: where each starts (content y) and what it shows.
    pub lines: Vec<(i64, Line)>,
    /// The pictures' scale and their block's height, as last laid out.
    pub zoom: f64,
    pub pictures_h: i64,
}

impl std::fmt::Debug for FormCompare {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormCompare").field("changes", &self.changes).field("form", &self.form).field("note", &self.note).finish()
    }
}

impl Clone for FormCompare {
    fn clone(&self) -> Self {
        FormCompare {
            changes: self.changes.clone(),
            before: self.before.as_ref().map(|s| Box::new(DesignSurface::with_designer(s.designer.clone()))),
            after: self.after.as_ref().map(|s| Box::new(DesignSurface::with_designer(s.designer.clone()))),
            form: self.form.clone(),
            note: self.note.clone(),
            lines: self.lines.clone(),
            zoom: self.zoom,
            pictures_h: self.pictures_h,
        }
    }
}

/// A text's forms: each top-level CREATE block's name and designer.
fn read(text: &str) -> Result<(Vec<(String, Designer)>, Option<usize>), String> {
    let Some(reader) = crate::objects::design::source_reader() else {
        return Err("Form views need RapidR Studio's form designer, which isn't part of this program: compare the texts instead (Kind = \"text\").".into());
    };
    let doc = reader(text, "");
    let forms = doc.forms();
    let out = (0..forms.len()).filter_map(|k| Some((forms[k].0.clone(), doc.designer(k)?))).collect();
    Ok((out, doc.error_line()))
}

/// A form's components as `rapidr_diff::form` compares them.
fn components(d: &Designer) -> Vec<Component> {
    let design = &d.design;
    design
        .ids()
        .into_iter()
        .filter_map(|id| {
            let n = design.node(id)?;
            Some(Component {
                name: n.name.clone(),
                type_name: n.type_written.clone(),
                parent: n.parent.and_then(|p| design.node(p)).map(|p| p.name.clone()),
                props: n.props().map(|p| (p.name.clone(), p.value.clone())).collect(),
                code: n
                    .body
                    .iter()
                    .filter_map(|i| match i {
                        crate::designer::model::Item::Code(c) => Some(c.clone()),
                        _ => None,
                    })
                    .collect(),
            })
        })
        .collect()
}

impl FormCompare {
    /// Compares the forms of two program texts.
    pub fn new(old: &str, new: &str) -> FormCompare {
        let mut out = FormCompare { zoom: 1.0, ..FormCompare::default() };
        let (before, after) = match (read(old), read(new)) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) | (_, Err(e)) => {
                out.note = e;
                return out;
            }
        };
        let mut notes = Vec::new();
        for (side, (_, line)) in [("old", &before), ("new", &after)] {
            if let Some(l) = line {
                notes.push(format!("The {side} text has an error on line {l}: only the components before it are compared."));
            }
        }
        let all = |forms: &[(String, Designer)]| forms.iter().flat_map(|(_, d)| components(d)).collect::<Vec<_>>();
        out.changes = form::diff(&all(&before.0), &all(&after.0));
        if before.0.is_empty() && after.0.is_empty() {
            notes.push("Neither text makes a form (no CREATE … END CREATE block).".into());
        }
        // the form drawn: the first (of the new text's) with a change
        let changed = |d: &Designer| {
            let names: Vec<String> = d.design.ids().iter().filter_map(|&i| d.design.node(i)).map(|n| n.name.to_lowercase()).collect();
            out.changes.iter().any(|c| names.contains(&c.name.to_lowercase()))
        };
        let pick = after.0.iter().find(|(_, d)| changed(d)).or_else(|| before.0.iter().find(|(_, d)| changed(d))).or_else(|| after.0.first()).or_else(|| before.0.first()).map(|(n, _)| n.clone());
        if let Some(name) = pick {
            let find = |forms: &[(String, Designer)]| forms.iter().find(|(n, _)| n.eq_ignore_ascii_case(&name)).map(|(_, d)| Box::new(DesignSurface::with_designer(d.clone())));
            out.before = find(&before.0);
            out.after = find(&after.0);
            out.form = name;
        }
        out.note = notes.join(" ");
        out
    }

    /// The pictures' scale for a view `width` wide (the two side by side,
    /// at most their own size) and the list's rows below them; the
    /// content's height.
    pub fn layout(&mut self, width: i64) -> i64 {
        let size = |s: &Option<Box<DesignSurface>>| s.as_ref().map_or((0, 0), |s| s.layout().form_size());
        let (b, a) = (size(&self.before), size(&self.after));
        let half = ((width - 3 * PAD) / 2).max(40);
        let widest = b.0.max(a.0).max(1);
        self.zoom = (half as f64 / widest as f64).min(1.0);
        let tallest = b.1.max(a.1);
        self.pictures_h = if self.before.is_none() && self.after.is_none() { 0 } else { LABEL_H + (tallest as f64 * self.zoom).ceil() as i64 + PAD };
        let mut y = self.pictures_h + if self.note.is_empty() { 0 } else { DETAIL_H + 4 };
        self.lines.clear();
        for (c, ch) in self.changes.iter().enumerate() {
            self.lines.push((y, Line::Change(c)));
            y += ITEM_H;
            for p in 0..ch.props.len() {
                self.lines.push((y, Line::Prop(c, p)));
                y += DETAIL_H;
            }
            for k in 0..details(ch).len() {
                self.lines.push((y, Line::Detail(c, k)));
                y += DETAIL_H;
            }
        }
        y + PAD
    }

    /// Where each picture is drawn (content coordinates): before, after.
    pub fn picture_origins(&self, width: i64) -> ((i64, i64), (i64, i64)) {
        let half = (width - 3 * PAD) / 2;
        ((PAD, LABEL_H), (2 * PAD + half, LABEL_H))
    }

    /// The components to outline in a picture (`after`: the new form's):
    /// their client rectangles and how they changed.
    pub fn marked(&self, after: bool) -> Vec<((i64, i64, i64, i64), Kind)> {
        let Some(s) = (if after { &self.after } else { &self.before }) else { return Vec::new() };
        let comps = s.components();
        let root = s.designer.design.node(s.designer.design.root()).map(|n| n.name.to_lowercase());
        self.changes
            .iter()
            .filter(|c| match c.kind {
                Kind::Added => after,
                Kind::Removed => !after,
                Kind::Changed => true,
            })
            .filter(|c| Some(c.name.to_lowercase()) != root)
            .filter_map(|c| comps.iter().find(|k| k.name.eq_ignore_ascii_case(&c.name) && k.visual).map(|k| ((k.x, k.y, k.w, k.h), c.kind)))
            .collect()
    }

    /// Plain words, one line per change.
    pub fn summary(&self) -> Vec<String> {
        self.changes.iter().map(ComponentChange::summary).collect()
    }
}

/// A change's details other than its properties: its type, its parent,
/// the lines added and removed.
pub fn details(c: &ComponentChange) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(t) = &c.old_type {
        out.push(format!("Type: {t} \u{2192} {}", c.type_name));
    }
    if let Some((from, to)) = &c.parent {
        let n = |p: &Option<String>| p.clone().unwrap_or_else(|| "the form".into());
        out.push(format!("Moved from {} into {}", n(from), n(to)));
    }
    out.extend(c.lines_added.iter().map(|l| format!("+ {l}")));
    out.extend(c.lines_removed.iter().map(|l| format!("\u{2212} {l}")));
    out
}
