//! The design-time side (docs/ai-architecture.md §3.5): a program's files,
//! each a `rapidr-designer` [`Document`] — the source is the truth, its
//! top-level CREATE blocks are the forms — answered as the interface's
//! `list` and `get`. Read-only (phase 0).
//!
//! Files are confined: [`DesignModel::in_memory`] reads nothing from the
//! disk (`$INCLUDE`s resolve among the files given), [`DesignModel::open`]
//! reads only files inside its root (as the language server: SEC-18).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rapidr_designer::designer::inspect::designable;
use rapidr_designer::designer::model::{prop_key, Item};
use rapidr_designer::designer::value::PropValue;
use rapidr_designer::designer::{FormDesign, Node, NodeId};
use rapidr_designer::{Document, ItemSpan};
use rapidr_lang::{Component, Type};
use rapidr_preprocessor::PreprocessOptions;
use serde_json::{json, Value};

use crate::protocol::*;
use crate::value::{color, enumerated, number, set};
use crate::Introspect;

/// One file of the model.
struct DesignFile {
    /// As the project names it (`main.rr`, `forms/settings.inc`).
    path: String,
    doc: Document,
    /// Where each line after the first starts (byte offsets): lines by
    /// binary search, not by counting (`list` of 1,000 components).
    line_starts: Vec<usize>,
}

impl DesignFile {
    fn new(path: String, doc: Document) -> DesignFile {
        let line_starts = doc.text().match_indices('\n').map(|(i, _)| i + 1).collect();
        DesignFile { path, doc, line_starts }
    }
}

/// Where a component is: file, form, node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Loc {
    file: usize,
    form: usize,
    node: NodeId,
}

/// A program's source as the interface sees it at design time.
pub struct DesignModel {
    files: Vec<DesignFile>,
}

impl DesignModel {
    /// A model of files given as text (the web, tests, an editor's unsaved
    /// buffers): `$INCLUDE`s resolve among them, nothing is read from disk.
    pub fn in_memory(files: &[(&str, &str)]) -> DesignModel {
        let options = PreprocessOptions {
            virtual_files: files.iter().map(|(p, t)| (p.to_string(), t.to_string())).collect(),
            confine_to: Some(Vec::new()),
            ..PreprocessOptions::default()
        };
        DesignModel { files: files.iter().map(|(p, t)| DesignFile::new(p.to_string(), Document::open(t, Some(Path::new(p)), options.clone()))).collect() }
    }

    /// A model of files on disk, every one inside `root` (and their
    /// `$INCLUDE`s confined to it). Paths are shown relative to `root`.
    pub fn open(root: &Path, files: &[PathBuf]) -> Result<DesignModel, String> {
        let roots = vec![root.to_path_buf()];
        let options = PreprocessOptions { confine_to: Some(roots.clone()), ..PreprocessOptions::default() };
        let mut out = Vec::new();
        for f in files {
            let full = if f.is_absolute() { f.clone() } else { root.join(f) };
            if !rapidr_preprocessor::is_within(&full, &roots) {
                return Err(format!("{}: outside {}", f.display(), root.display()));
            }
            let bytes = std::fs::read(&full).map_err(|e| format!("{}: {e}", f.display()))?;
            let shown = full.strip_prefix(root).unwrap_or(&full).to_string_lossy().replace('\\', "/");
            out.push(DesignFile::new(shown, Document::open_bytes(&bytes, Some(&full), options.clone())));
        }
        Ok(DesignModel { files: out })
    }

    /// The files, as the model names them.
    pub fn paths(&self) -> Vec<&str> {
        self.files.iter().map(|f| f.path.as_str()).collect()
    }

    /// A file's document.
    pub fn document(&self, path: &str) -> Option<&Document> {
        self.files.iter().find(|f| f.path == path).map(|f| &f.doc)
    }

    fn design(&self, loc: Loc) -> &FormDesign {
        &self.files[loc.file].doc.forms()[loc.form].designer.design
    }

    fn node(&self, loc: Loc) -> &Node {
        self.design(loc).node(loc.node).expect("a located node")
    }

    /// Every component, in files' order, forms' order and creation order.
    fn all(&self) -> Vec<Loc> {
        let mut out = Vec::new();
        for (fi, f) in self.files.iter().enumerate() {
            for (k, form) in f.doc.forms().iter().enumerate() {
                for id in form.designer.design.ids() {
                    out.push(Loc { file: fi, form: k, node: id });
                }
            }
        }
        out
    }

    /// Each component's id: its name, qualified only when another
    /// component has the same one (`Form1@main.rr`, `Form1@main.rr#2`).
    fn ids(&self, all: &[Loc]) -> HashMap<Loc, String> {
        let mut by_name: HashMap<String, Vec<Loc>> = HashMap::new();
        for &l in all {
            by_name.entry(self.node(l).name.to_ascii_lowercase()).or_default().push(l);
        }
        let mut out = HashMap::new();
        for locs in by_name.values() {
            for &l in locs {
                let name = self.node(l).name.clone();
                if locs.len() == 1 {
                    out.insert(l, name);
                    continue;
                }
                let same_file: Vec<&Loc> = locs.iter().filter(|o| o.file == l.file).collect();
                let path = &self.files[l.file].path;
                let id = if same_file.len() == 1 {
                    format!("{name}@{path}")
                } else {
                    let n = same_file.iter().position(|o| **o == l).unwrap_or(0) + 1;
                    format!("{name}@{path}#{n}")
                };
                out.insert(l, id);
            }
        }
        out
    }

    /// The component an id names.
    fn resolve(&self, id: &str) -> Result<Loc, Error> {
        let id = id.trim();
        if id.contains('(') {
            return Err(Error::new(ErrorCode::NotFound, format!("`{id}`: component arrays and items exist only while the program runs")));
        }
        let (rest, nth) = match id.rsplit_once('#') {
            Some((r, n)) if id.contains('@') => match n.parse::<usize>() {
                Ok(n) if n >= 1 => (r, Some(n)),
                _ => return Err(Error::new(ErrorCode::BadRequest, format!("`{id}`: `#` takes a number from 1"))),
            },
            _ => (id, None),
        };
        let (name, file) = match rest.split_once('@') {
            Some((n, f)) => (n, Some(f)),
            None => (rest, None),
        };
        let all = self.all();
        let mut matches: Vec<Loc> = all.iter().copied().filter(|&l| self.node(l).name.eq_ignore_ascii_case(name)).collect();
        if let Some(f) = file {
            matches.retain(|l| self.files[l.file].path == f);
        }
        if let Some(n) = nth {
            return matches.get(n - 1).copied().ok_or_else(|| Error::new(ErrorCode::NotFound, format!("no `{id}`")));
        }
        match matches.as_slice() {
            [one] => Ok(*one),
            [] => Err(Error::new(ErrorCode::NotFound, format!("no component `{id}` (list shows them)"))),
            many => {
                let ids = self.ids(&all);
                let names: Vec<&str> = many.iter().filter_map(|l| ids.get(l).map(String::as_str)).collect();
                Err(Error::new(ErrorCode::Ambiguous, format!("`{id}` names {} components: {}", many.len(), names.join(", "))))
            }
        }
    }

    /// The 1-based line of a byte offset in a file.
    fn line(&self, file: usize, offset: usize) -> u32 {
        // (the lines starting at or before it: the first line, plus each
        // line start ≤ offset)
        (self.files[file].line_starts.partition_point(|&s| s <= offset) + 1) as u32
    }

    /// The line of a component's CREATE.
    fn create_line(&self, l: Loc) -> Option<u32> {
        let form = &self.files[l.file].doc.forms()[l.form];
        form.spans(l.node).map(|s| self.line(l.file, s.start))
    }

    /// The line of the body item `index` of a component (an assignment).
    fn item_line(&self, l: Loc, index: usize) -> Option<u32> {
        let form = &self.files[l.file].doc.forms()[l.form];
        let item = form.spans(l.node)?.items.get(index)?;
        let at = match item {
            ItemSpan::Prop { stmt, .. } | ItemSpan::Child { stmt } | ItemSpan::Code { stmt } => stmt.0,
        };
        Some(self.line(l.file, at))
    }

    fn info(&self, l: Loc, ids: &HashMap<Loc, String>) -> ObjectInfo {
        let n = self.node(l);
        let d = self.design(l);
        let comp = n.component();
        let type_name = comp.map_or_else(|| n.type_written.clone(), Component::spelling);
        ObjectInfo {
            id: ids.get(&l).cloned().unwrap_or_else(|| n.name.clone()),
            name: n.name.clone(),
            type_written: (!n.type_written.eq_ignore_ascii_case(&type_name)).then(|| n.type_written.clone()),
            type_name,
            parent: n.parent.map(|p| ids.get(&Loc { node: p, ..l }).cloned().unwrap_or_default()),
            form: ids.get(&Loc { node: d.root(), ..l }).cloned().unwrap_or_default(),
            file: Some(self.files[l.file].path.clone()),
            line: self.create_line(l),
            visual: n.is_visual(),
            container: n.is_container() || n.is_form(),
            children: n.children().count() as u32,
        }
    }

    /// A value written as `text` for a property of type `ty` (`prop`: the
    /// registry's entry), typed (§3.4).
    fn typed(d: &FormDesign, ty: Type, prop: Option<&rapidr_lang::Property>, text: &str) -> Value {
        if prop.is_some_and(|p| p.secret) {
            return json!({"redacted": true});
        }
        let code = || json!({"code": text.trim()});
        let ident = || {
            let t = text.trim();
            (t.starts_with(|c: char| c.is_ascii_alphabetic()) && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')).then_some(t)
        };
        let v = d.read_value(text);
        match (ty, v) {
            (Type::String | Type::Picture | Type::Resource, PropValue::Str(s)) => json!(s),
            (Type::Int | Type::Float, PropValue::Number(n)) => number(n),
            (Type::Bool, PropValue::Number(n)) => json!(n != 0.0),
            (Type::Enum, PropValue::Number(n)) => match prop {
                Some(p) => enumerated(p, n as i64, ident()),
                None => number(n),
            },
            (Type::Set, PropValue::Number(n)) => match prop {
                Some(p) => set(p, n as i64),
                None => number(n),
            },
            (Type::Color, PropValue::Number(n)) => color(n as i64, ident()),
            // (a constant the program doesn't define: RapidQ reads it as an
            // empty variable, so the value isn't known here)
            (Type::Enum | Type::Set | Type::Color | Type::Bool, PropValue::Name(c)) => json!({"const": c}),
            (Type::Component | Type::Font, PropValue::Name(c)) => json!({"ref": c}),
            (Type::Item | Type::Any, PropValue::Str(s)) => json!(s),
            (Type::Item | Type::Any, PropValue::Number(n)) => number(n),
            _ => code(),
        }
    }

    /// A part written as a property of its own (`Font.Name`,
    /// `Constraints.MinWidth`): its type.
    fn part_type(name: &str, comp: Option<&Component>) -> Option<Type> {
        let (whole, part) = name.split_once('.')?;
        if whole.eq_ignore_ascii_case("font") {
            return Some(match part.to_ascii_lowercase().as_str() {
                "name" => Type::String,
                "size" => Type::Int,
                "color" => Type::Color,
                "bold" | "italic" | "underline" | "strikeout" => Type::Bool,
                _ => Type::Any,
            });
        }
        // (Constraints.MinWidth is MinWidth)
        comp.and_then(|c| c.property(part)).map(|p| p.ty)
    }

    fn value_entry(&self, l: Loc, name: &str) -> Option<ValueEntry> {
        let n = self.node(l);
        let d = self.design(l);
        let comp = n.component();
        let reg = comp.and_then(|c| c.property(name));
        let index = n.prop_index(name);
        let written = index.and_then(|i| match &n.body[i] {
            Item::Prop(p) => Some((p.name.clone(), p.value.clone())),
            _ => None,
        });
        match (reg, written) {
            (Some(p), Some((_, text))) => Some(ValueEntry {
                name: p.name.to_string(),
                value: Self::typed(d, p.ty, Some(p), &text),
                written: (!p.secret).then_some(text),
                source: Source::Set,
                line: index.and_then(|i| self.item_line(l, i)),
                unknown: false,
            }),
            (Some(p), None) => {
                let value = if p.secret { json!({"redacted": true}) } else { crate::describe::default_value(p) };
                let source = if p.default.is_some() { Source::Default } else { Source::Unset };
                Some(ValueEntry { name: p.name.to_string(), value, written: None, source, line: None, unknown: false })
            }
            (None, Some((as_written, text))) => {
                let ty = Self::part_type(&as_written, comp);
                // (Constraints.MinWidth: MinWidth's entry types it)
                let constraint = as_written.split_once('.').filter(|(whole, _)| whole.eq_ignore_ascii_case("constraints")).and_then(|(_, part)| comp.and_then(|c| c.property(part)));
                Some(ValueEntry {
                    name: as_written,
                    value: Self::typed(d, ty.unwrap_or(Type::Any), constraint, &text),
                    written: Some(text),
                    source: Source::Set,
                    line: index.and_then(|i| self.item_line(l, i)),
                    unknown: ty.is_none(),
                })
            }
            (None, None) => None,
        }
    }
}

impl Introspect for DesignModel {
    /// A 64-bit FNV-1a hash of the files' paths and texts.
    fn rev(&self) -> String {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for f in &self.files {
            for b in f.path.bytes().chain([0]).chain(f.doc.text().bytes()).chain([0]) {
                h ^= b as u64;
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        format!("{h:016x}")
    }

    fn list(&self, q: &ListQuery) -> Result<ListResult, Error> {
        let rev = self.rev();
        let all = self.all();
        let ids = self.ids(&all);
        let scope = q.scope.as_deref().map(|s| self.resolve(s)).transpose()?;
        let canonical = match q.type_name.as_deref() {
            Some(t) => Some(rapidr_lang::resolve_component(t).map(|c| c.name).ok_or_else(|| Error::new(ErrorCode::UnknownType, format!("no component type `{t}`")))?),
            None => None,
        };
        let matching: Vec<Loc> = all
            .iter()
            .copied()
            .filter(|&l| match scope {
                Some(s) => l.file == s.file && l.form == s.form && l.node != s.node && self.design(l).is_within(l.node, s.node),
                None => true,
            })
            .filter(|&l| canonical.is_none_or(|c| self.node(l).canonical == c))
            .collect();
        let limit = match q.limit {
            Some(0) => return Err(Error::new(ErrorCode::BadRequest, "limit is at least 1")),
            Some(n) => n.min(LIST_MAX),
            None => LIST_LIMIT,
        } as usize;
        let start = match q.cursor.as_deref() {
            None => 0,
            Some(c) => {
                let (crev, at) = c.split_once(':').ok_or_else(|| Error::new(ErrorCode::BadRequest, format!("cursor `{c}`")))?;
                if crev != rev {
                    return Err(Error::new(ErrorCode::Stale, "the program changed since that page: list again"));
                }
                at.parse::<usize>().map_err(|_| Error::new(ErrorCode::BadRequest, format!("cursor `{c}`")))?
            }
        };
        let end = (start + limit).min(matching.len());
        let objects = matching.get(start..end).unwrap_or(&[]).iter().map(|&l| self.info(l, &ids)).collect();
        let next = (end < matching.len()).then(|| format!("{rev}:{end}"));
        Ok(ListResult { rev, total: matching.len() as u32, objects, next })
    }

    fn get(&self, id: &str, props: Option<&[String]>) -> Result<GetResult, Error> {
        let l = self.resolve(id)?;
        let all = self.all();
        let ids = self.ids(&all);
        let n = self.node(l);
        let comp = n.component();
        let type_name = comp.map_or_else(|| n.type_written.clone(), Component::spelling);
        let mut values = Vec::new();
        let mut events = Vec::new();
        let event_entry = |name: &str, handler: Option<String>| {
            let line = n.prop_index(name).and_then(|i| self.item_line(l, i));
            EventEntry { name: name.to_string(), handler, line }
        };
        match props {
            Some(names) => {
                for name in names {
                    if let Some(e) = comp.and_then(|c| c.event(name)).filter(|e| !e.missing) {
                        events.push(event_entry(e.name, n.prop(e.name).map(|h| h.trim().to_string())));
                        continue;
                    }
                    if comp.and_then(|c| c.property(name)).is_some_and(|p| p.missing) {
                        return Err(Error::new(ErrorCode::NotFound, format!("{type_name}.{name} isn't in RapidR yet")));
                    }
                    match self.value_entry(l, name) {
                        Some(v) => values.push(v),
                        None => return Err(Error::new(ErrorCode::NotFound, format!("{type_name} has no property or event `{name}` (describe {type_name} lists them)"))),
                    }
                }
            }
            None => {
                let mut covered: Vec<String> = Vec::new();
                if let Some(c) = comp {
                    for p in c.properties.iter().filter(|p| p.design && designable(p) && p.indexed == 0 && !p.missing) {
                        if let Some(v) = self.value_entry(l, p.name) {
                            covered.push(prop_key(p.name));
                            values.push(v);
                        }
                    }
                }
                // what the block writes that the rows above don't show: font
                // parts, a property the registry doesn't know; events bound
                for p in n.props() {
                    let key = prop_key(&p.name);
                    if covered.contains(&key) {
                        continue;
                    }
                    covered.push(key);
                    if let Some(e) = comp.and_then(|c| c.event(&p.name)) {
                        events.push(event_entry(e.name, n.prop(e.name).map(|h| h.trim().to_string())));
                    } else if let Some(v) = self.value_entry(l, &p.name) {
                        values.push(v);
                    }
                }
            }
        }
        Ok(GetResult { rev: self.rev(), id: ids.get(&l).cloned().unwrap_or_else(|| n.name.clone()), name: n.name.clone(), type_name, values, events })
    }
}
