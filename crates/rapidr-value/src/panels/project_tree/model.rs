//! RPROJECTTREE's model: the project (`rapidr_project`'s, `.rrproj` format
//! 2), the files' texts it knows, each form's components, and the tree's
//! state — what is open, selected, hovered, being renamed, deleted or
//! dragged. GUI-free: the kernel lays its [`ProjectTree::rows`] out and
//! paints them; the runtime glue changes it with the program's events.
//!
//! Nodes have keys: [`KEY_PROJECT`], `@group/<kind>`, `@folder/<kind>/<path>/`,
//! a file's path, `path#Component`; the program sees a node by its
//! [`ProjectTree::public`] path (a file's path, `path#Component`, a
//! folder's `path/`, "" for the project and its groups).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use rapidr_project::{implicit_from_sources, import_v1, is_v1_json, kind_for_path, kind_for_source, normalize_path, FileKind, Project, ProjectFile};

use super::scan::{self, Comp};

/// The project's own node.
pub const KEY_PROJECT: &str = "@project";

/// The groups, in the order they show.
pub const KINDS: [FileKind; 6] = [FileKind::Form, FileKind::Module, FileKind::Include, FileKind::Resource, FileKind::Asset, FileKind::Data];

/// A group's title.
pub fn kind_title(k: FileKind) -> &'static str {
    match k {
        FileKind::Form => "Forms",
        FileKind::Module => "Modules",
        FileKind::Include => "Includes",
        FileKind::Resource => "Resources",
        FileKind::Asset => "Assets",
        FileKind::Data => "Data",
    }
}

/// A kind from its name ("form", "module" …; any case).
pub fn parse_kind(s: &str) -> Option<FileKind> {
    KINDS.into_iter().find(|k| k.as_str().eq_ignore_ascii_case(s.trim()))
}

fn group_key(k: FileKind) -> String {
    format!("@group/{}", k.as_str())
}

fn folder_key(k: FileKind, path: &str) -> String {
    format!("@folder/{}/{}", k.as_str(), path)
}

/// The folder part of a project path, with its `/` ("" at the top).
pub fn folder_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..=i],
        None => "",
    }
}

/// The name part of a path.
pub fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// A name's extension (without the dot; "" for none).
fn ext_of(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 => &name[i + 1..],
        _ => "",
    }
}

/// What a node is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Project,
    Group(FileKind),
    /// A folder inside a group: its path, ending in `/`.
    Folder(FileKind, String),
    /// A file: its path.
    File(String),
    /// A form's component.
    Component { file: String, name: String, ty: String },
}

/// A row of the tree as shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub key: String,
    pub label: String,
    /// 0: the project; its groups 1 …
    pub level: usize,
    pub expandable: bool,
    pub expanded: bool,
    /// The project's main file.
    pub main: bool,
    /// A group's number of files.
    pub count: Option<usize>,
    /// The row above it at the level above (`None`: the project).
    pub parent: Option<usize>,
}

/// Where a dragged file would go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropAt {
    /// Before file `path` (in its folder).
    Before(String),
    /// After file `path`.
    After(String),
    /// Into a folder (`path/`; "" the group's top).
    Into(String),
}

/// A file's removal being confirmed: the strip at the tree's bottom.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Confirm {
    pub path: String,
    /// The strip's button with the keyboard: 0 Remove, 1 Cancel.
    pub focus: usize,
    pub hover: Option<usize>,
    /// Pressed with the mouse (drawn pushed while over it).
    pub pressed: Option<usize>,
}

/// A press on a file that may become a drag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drag {
    /// The file.
    pub path: String,
    pub from: (i64, i64),
    pub at: (i64, i64),
    /// It moved far enough to be a drag.
    pub active: bool,
    pub target: Option<DropAt>,
}

/// RPROJECTTREE's state.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectTree {
    pub project: Project,
    pub loaded: bool,
    /// The project's file as given (Project, LoadText's Path).
    pub path: String,
    /// Files' texts known (SetFileText's, the files read), by lowercase path.
    pub texts: HashMap<String, String>,
    /// The ones SetFileText gave (a Refresh keeps them).
    pub given: HashSet<String>,
    /// Each form's components, by its lowercase path.
    pub comps: HashMap<String, Vec<Comp>>,
    pub show_files: bool,
    pub show_forms: bool,
    pub show_components: bool,
    /// Nodes opened or closed (others: their default).
    pub open: HashMap<String, bool>,
    /// The selected node's key ("" none).
    pub selected: String,
    pub modified: bool,
    // ---- what the kernel keeps here (pure UI state) ----
    pub hover: Option<String>,
    pub confirm: Option<Confirm>,
    pub drag: Option<Drag>,
    /// A file whose name the kernel is to edit (NewFile, Rename without a
    /// name): it opens its editor when it next draws.
    pub edit_request: Option<String>,
    /// The file whose name is being edited.
    pub editing: Option<String>,
    /// The file NewFile added, being named (its edit ends with OnNewFile).
    pub new_file: Option<String>,
    /// Why the name being typed can't be (shown under the editor).
    pub rename_error: Option<String>,
    /// The selected node is to be scrolled into view.
    pub reveal: bool,
}

impl Default for ProjectTree {
    fn default() -> Self {
        ProjectTree {
            project: Project::default(),
            loaded: false,
            path: String::new(),
            texts: HashMap::new(),
            given: HashSet::new(),
            comps: HashMap::new(),
            show_files: true,
            show_forms: true,
            show_components: true,
            open: HashMap::new(),
            selected: String::new(),
            modified: false,
            hover: None,
            confirm: None,
            drag: None,
            edit_request: None,
            editing: None,
            new_file: None,
            rename_error: None,
            reveal: false,
        }
    }
}

impl ProjectTree {
    // ---------------------------------------------------------- loading --

    /// The project's folder (its file's, with `/`; "" here).
    pub fn folder(&self) -> String {
        folder_of(&self.path.replace('\\', "/")).to_string()
    }

    /// Reads a project from its file `path`: a `.rrproj` (format 2 TOML, or
    /// the web IDE's v1 JSON), or a source file and what it `$INCLUDE`s.
    /// `main_text`: the file's text when the caller has it (LoadText); else
    /// `read` reads it. `read` takes a path as the program gives one (the
    /// project's folder before a project path). Whether it was read; when
    /// not, nothing changed.
    pub fn load(&mut self, path: &str, main_text: Option<String>, read: &dyn Fn(&str) -> Option<String>) -> bool {
        let unified = path.trim().replace('\\', "/");
        let folder = folder_of(&unified).to_string();
        let name = name_of(&unified).to_string();
        let ext = ext_of(&name).to_ascii_lowercase();
        let same = self.loaded && self.path.eq_ignore_ascii_case(path.trim());
        let from_text = main_text.is_some();
        let mut texts: HashMap<String, String> = if same { self.given_texts() } else { HashMap::new() };
        let mut given: HashSet<String> = if same { self.given.clone() } else { HashSet::new() };
        let project = if ext == "rrproj" || ext == "json" {
            let Some(text) = main_text.or_else(|| read(&unified)) else { return false };
            if is_v1_json(&text) {
                let Ok(imported) = import_v1(&text) else { return false };
                for (p, bytes) in &imported.files {
                    let key = normalize_path(p).to_lowercase();
                    texts.insert(key.clone(), String::from_utf8_lossy(bytes).into_owned());
                    given.insert(key);
                }
                imported.project
            } else {
                match Project::from_toml(&text) {
                    Ok(p) => p,
                    Err(_) => return false,
                }
            }
        } else {
            if name.is_empty() {
                return false;
            }
            let found: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
            let main_key = normalize_path(&name).to_lowercase();
            let main_text = RefCell::new(main_text);
            let lookup = |rel: &str| -> Option<String> {
                let key = normalize_path(rel).to_lowercase();
                if key == main_key {
                    if let Some(t) = main_text.borrow_mut().take() {
                        found.borrow_mut().insert(key, t.clone());
                        return Some(t);
                    }
                }
                if let Some(t) = texts.get(&key).cloned().or_else(|| found.borrow().get(&key).cloned()) {
                    return Some(t);
                }
                let t = read(&format!("{folder}{rel}"))?;
                found.borrow_mut().insert(key, t.clone());
                Some(t)
            };
            let p = implicit_from_sources(&name, &lookup);
            // (a main file that isn't there: no project)
            if !found.borrow().contains_key(&main_key) && !texts.contains_key(&main_key) {
                return false;
            }
            texts.extend(found.into_inner());
            // (LoadText's text: kept over a Refresh, as SetFileText's)
            if from_text {
                given.insert(main_key);
            }
            p
        };
        self.project = project;
        self.path = path.trim().to_string();
        self.texts = texts;
        self.given = given;
        self.loaded = true;
        self.modified = false;
        if !same {
            self.open.clear();
            self.selected.clear();
            self.hover = None;
            self.confirm = None;
            self.drag = None;
            self.edit_request = None;
            self.editing = None;
            self.new_file = None;
            self.rename_error = None;
        }
        self.scan_all(read);
        self.keep_selection();
        true
    }

    fn given_texts(&self) -> HashMap<String, String> {
        self.texts.iter().filter(|(k, _)| self.given.contains(*k)).map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Reads the forms' components again (their texts read through `read`
    /// unless known).
    pub fn scan_all(&mut self, read: &dyn Fn(&str) -> Option<String>) {
        self.comps.clear();
        let folder = self.folder();
        let forms: Vec<String> = self.project.files.iter().filter(|f| f.kind == FileKind::Form).map(|f| f.path.clone()).collect();
        for path in forms {
            let key = path.to_lowercase();
            if !self.texts.contains_key(&key) {
                if let Some(t) = read(&format!("{folder}{path}")) {
                    self.texts.insert(key.clone(), t);
                }
            }
            if let Some(t) = self.texts.get(&key) {
                self.comps.insert(key, scan::creates(t));
            }
        }
    }

    /// A file's text given (SetFileText): its components listed from it.
    pub fn set_text(&mut self, path: &str, text: &str) {
        let key = normalize_path(path).to_lowercase();
        self.texts.insert(key.clone(), text.to_string());
        self.given.insert(key.clone());
        if self.file(&key).is_some_and(|f| f.kind == FileKind::Form) {
            self.comps.insert(key, scan::creates(text));
            self.keep_selection();
        }
    }

    /// The selection forgotten if its node went.
    fn keep_selection(&mut self) {
        if !self.selected.is_empty() && !self.has_node(&self.selected.clone()) {
            self.selected.clear();
        }
    }

    // ------------------------------------------------------------ files --

    /// The project's file `path` (any case).
    pub fn file(&self, path: &str) -> Option<&ProjectFile> {
        let p = normalize_path(path);
        self.project.files.iter().find(|f| f.path.eq_ignore_ascii_case(&p))
    }

    fn index_of(&self, path: &str) -> Option<usize> {
        let p = normalize_path(path);
        self.project.files.iter().position(|f| f.path.eq_ignore_ascii_case(&p))
    }

    /// Adds a file (its kind from `kind`, else its extension and text).
    pub fn add_file(&mut self, path: &str, kind: Option<FileKind>, read: &dyn Fn(&str) -> Option<String>) -> bool {
        let p = normalize_path(path);
        if p.is_empty() || self.file(&p).is_some() {
            return false;
        }
        let key = p.to_lowercase();
        let kind = kind.unwrap_or_else(|| {
            let k = kind_for_path(&p);
            if k != FileKind::Module {
                return k;
            }
            if !self.texts.contains_key(&key) {
                if let Some(t) = read(&format!("{}{p}", self.folder())) {
                    self.texts.insert(key.clone(), t);
                }
            }
            self.texts.get(&key).map_or(k, |t| kind_for_source(&p, t))
        });
        self.project.add_file(&p, kind);
        if kind == FileKind::Form {
            if let Some(t) = self.texts.get(&key) {
                self.comps.insert(key, scan::creates(t));
            }
        }
        if !self.loaded {
            self.loaded = true;
            if self.project.name.is_empty() {
                self.project.name = "Project".into();
            }
        }
        self.modified = true;
        true
    }

    /// Takes a file out of the project (the selection goes to the row now
    /// where it was): whether it was there.
    pub fn remove_file(&mut self, path: &str) -> bool {
        let Some(i) = self.index_of(path) else { return false };
        let actual = self.project.files[i].path.clone();
        let rows = self.rows();
        let was = rows.iter().position(|r| r.key == self.selected);
        let sel_gone = self.selected == actual || self.selected.starts_with(&format!("{actual}#"));
        self.project.remove_file(&actual);
        let key = actual.to_lowercase();
        self.comps.remove(&key);
        self.forget_keys(&actual);
        if self.new_file.as_deref() == Some(actual.as_str()) {
            self.new_file = None;
        }
        if sel_gone {
            let rows = self.rows();
            self.selected = match was {
                Some(i) if !rows.is_empty() => rows[i.min(rows.len() - 1)].key.clone(),
                _ => String::new(),
            };
        }
        self.modified = true;
        true
    }

    /// Open states and the hover of file `path` and its components forgotten.
    fn forget_keys(&mut self, path: &str) {
        let comp = format!("{path}#");
        self.open.retain(|k, _| k != path && !k.starts_with(&comp));
        if self.hover.as_deref().is_some_and(|h| h == path || h.starts_with(&comp)) {
            self.hover = None;
        }
    }

    /// Why `typed` can't be file `old`'s new name, or the new path it makes
    /// (the old extension kept unless one is typed; `old` itself: no change).
    pub fn validate(&self, old: &str, typed: &str) -> Result<String, String> {
        let t = typed.trim();
        if t.is_empty() {
            return Err("A file name is needed.".into());
        }
        if t.contains(['/', '\\']) {
            return Err("A name can't hold / or \\.".into());
        }
        if t.chars().any(|c| c.is_control() || "<>:\"|?*".contains(c)) {
            return Err("A name can't hold < > : \" | ? *.".into());
        }
        if t.chars().all(|c| c == '.') {
            return Err("That isn't a file name.".into());
        }
        let old_ext = ext_of(name_of(old));
        let name = if ext_of(t).is_empty() && !old_ext.is_empty() { format!("{}.{old_ext}", t.trim_end_matches('.')) } else { t.to_string() };
        let new = format!("{}{name}", folder_of(old));
        if new == old {
            return Ok(new);
        }
        if self.project.files.iter().any(|f| f.path.eq_ignore_ascii_case(&new) && !f.path.eq_ignore_ascii_case(old)) {
            return Err(format!("{name} is already in the project."));
        }
        Ok(new)
    }

    /// File `old` is now `new` (in the project, its texts, the tree's state).
    pub fn apply_rename(&mut self, old: &str, new: &str) {
        let Some(i) = self.index_of(old) else { return };
        let old = self.project.files[i].path.clone();
        let new = normalize_path(new);
        if old == new {
            return;
        }
        self.project.files[i].path = new.clone();
        if self.project.main == old {
            self.project.main = new.clone();
        }
        let (ok, nk) = (old.to_lowercase(), new.to_lowercase());
        if let Some(t) = self.texts.remove(&ok) {
            self.texts.insert(nk.clone(), t);
        }
        if self.given.remove(&ok) {
            self.given.insert(nk.clone());
        }
        if let Some(c) = self.comps.remove(&ok) {
            self.comps.insert(nk, c);
        }
        let move_key = |k: &str| -> Option<String> {
            if k == old {
                Some(new.clone())
            } else {
                k.strip_prefix(&format!("{old}#")).map(|rest| format!("{new}#{rest}"))
            }
        };
        self.open = self.open.drain().map(|(k, v)| (move_key(&k).unwrap_or(k), v)).collect();
        if let Some(k) = move_key(&self.selected) {
            self.selected = k;
        }
        for slot in [&mut self.hover, &mut self.new_file, &mut self.edit_request, &mut self.editing] {
            if let Some(k) = slot.as_deref().and_then(move_key) {
                *slot = Some(k);
            }
        }
        self.modified = true;
    }

    /// A free name for a new file of `kind` at the project's top (`name`
    /// wanted: it, or it numbered): `Form2.rr`, `Module1.rr` …
    pub fn free_name(&self, kind: FileKind, name: Option<&str>) -> String {
        let (stem, ext) = match kind {
            FileKind::Form => ("Form", "rr"),
            FileKind::Module => ("Module", "rr"),
            FileKind::Include => ("Include", "inc"),
            FileKind::Resource => ("Resource", "ico"),
            FileKind::Asset => ("Asset", "png"),
            FileKind::Data => ("Data", "csv"),
        };
        let taken = |p: &str| self.project.files.iter().any(|f| f.path.eq_ignore_ascii_case(p));
        if let Some(n) = name.map(str::trim).filter(|n| !n.is_empty()) {
            let n = name_of(&n.replace('\\', "/")).to_string();
            let (s, e) = match ext_of(&n) {
                "" => (n.trim_end_matches('.').to_string(), ext.to_string()),
                e => (n[..n.len() - e.len() - 1].to_string(), e.to_string()),
            };
            let first = format!("{s}.{e}");
            if !taken(&first) {
                return first;
            }
            return (2..).map(|i| format!("{s}{i}.{e}")).find(|p| !taken(p)).unwrap_or(first);
        }
        (1..).map(|i| format!("{stem}{i}.{ext}")).find(|p| !taken(p)).unwrap_or_default()
    }

    /// Where file `path` goes dropped at `at`: its new path and its index
    /// in the project's order (`None`: it can't go there, or stays as is).
    pub fn drop_result(&self, path: &str, at: &DropAt) -> Option<(String, usize)> {
        let i = self.index_of(path)?;
        let f = &self.project.files[i];
        let name = name_of(&f.path);
        let mut rest: Vec<&ProjectFile> = self.project.files.iter().collect();
        rest.remove(i);
        let (folder, index) = match at {
            DropAt::Before(a) | DropAt::After(a) => {
                let anchor = self.file(a)?;
                if anchor.kind != f.kind || anchor.path == f.path {
                    return None;
                }
                // (already beside it in its folder: no change)
                let sibs: Vec<&str> = self.project.files.iter().filter(|o| o.kind == f.kind && folder_of(&o.path) == folder_of(&anchor.path)).map(|o| o.path.as_str()).collect();
                let (pf, pa) = (sibs.iter().position(|p| *p == f.path), sibs.iter().position(|p| *p == anchor.path));
                if let (Some(pf), Some(pa)) = (pf, pa) {
                    if (matches!(at, DropAt::Before(_)) && pf + 1 == pa) || (matches!(at, DropAt::After(_)) && pa + 1 == pf) {
                        return None;
                    }
                }
                let pos = rest.iter().position(|r| r.path == anchor.path)?;
                (folder_of(&anchor.path).to_string(), if matches!(at, DropAt::After(_)) { pos + 1 } else { pos })
            }
            DropAt::Into(folder) => (folder.clone(), i),
        };
        let new = format!("{folder}{name}");
        if new != f.path && self.project.files.iter().any(|o| o.path.eq_ignore_ascii_case(&new)) {
            return None;
        }
        if new == f.path && index == i {
            return None;
        }
        Some((new, index))
    }

    /// File `path` moved to `new` at `index` of the project's order.
    pub fn apply_move(&mut self, path: &str, new: &str, index: usize) {
        let Some(i) = self.index_of(path) else { return };
        let actual = self.project.files[i].path.clone();
        self.apply_rename(&actual, new);
        let f = self.project.files.remove(i);
        let at = index.min(self.project.files.len());
        self.project.files.insert(at, f);
        self.modified = true;
    }

    /// Where Alt+Up / Alt+Down moves file `path`: before the file above it
    /// in its folder, after the one below.
    pub fn step(&self, path: &str, down: bool) -> Option<(String, usize)> {
        let f = self.file(path)?;
        let folder = folder_of(&f.path);
        let sibs: Vec<&ProjectFile> = self.project.files.iter().filter(|o| o.kind == f.kind && folder_of(&o.path) == folder).collect();
        let at = sibs.iter().position(|o| o.path == f.path)?;
        if down {
            self.drop_result(&f.path, &DropAt::After(sibs.get(at + 1)?.path.clone()))
        } else {
            self.drop_result(&f.path, &DropAt::Before(sibs.get(at.checked_sub(1)?)?.path.clone()))
        }
    }

    // ------------------------------------------------------------- rows --

    /// Whether node `key` is open (`default`: unless opened or closed).
    fn is_open(&self, key: &str, default: bool) -> bool {
        self.open.get(key).copied().unwrap_or(default)
    }

    /// A node's default: the project, its groups, folders and components
    /// open; files closed.
    fn default_open(key: &str) -> bool {
        key.starts_with('@') || key.contains('#')
    }

    pub fn expanded(&self, key: &str) -> bool {
        self.is_open(key, Self::default_open(key))
    }

    pub fn set_open(&mut self, key: &str, open: bool) {
        self.open.insert(key.to_string(), open);
    }

    /// The tree's rows as shown (open nodes' children under them).
    pub fn rows(&self) -> Vec<Node> {
        let mut rows = Vec::new();
        if !self.loaded {
            return rows;
        }
        let open = self.expanded(KEY_PROJECT);
        rows.push(Node {
            kind: NodeKind::Project,
            key: KEY_PROJECT.into(),
            label: if self.project.name.is_empty() { "Project".into() } else { self.project.name.clone() },
            level: 0,
            expandable: true,
            expanded: open,
            main: false,
            count: None,
            parent: None,
        });
        if !open {
            return rows;
        }
        for k in KINDS {
            if (k == FileKind::Form && !self.show_forms) || (k != FileKind::Form && !self.show_files) {
                continue;
            }
            let files: Vec<&ProjectFile> = self.project.files.iter().filter(|f| f.kind == k).collect();
            if files.is_empty() {
                continue;
            }
            let key = group_key(k);
            let open = self.expanded(&key);
            let at = rows.len();
            rows.push(Node { kind: NodeKind::Group(k), key, label: kind_title(k).into(), level: 1, expandable: true, expanded: open, main: false, count: Some(files.len()), parent: Some(0) });
            if open {
                self.folder_rows(&mut rows, k, "", &files, 2, at);
            }
        }
        rows
    }

    /// Folder `prefix`'s rows in group `k`: its folders, then its files.
    fn folder_rows(&self, rows: &mut Vec<Node>, k: FileKind, prefix: &str, files: &[&ProjectFile], level: usize, parent: usize) {
        let mut folders: Vec<&str> = Vec::new();
        for f in files {
            if let Some((dir, _)) = f.path[prefix.len()..].split_once('/') {
                if !folders.contains(&dir) {
                    folders.push(dir);
                }
            }
        }
        for dir in folders {
            let path = format!("{prefix}{dir}/");
            let key = folder_key(k, &path);
            let open = self.expanded(&key);
            let at = rows.len();
            rows.push(Node { kind: NodeKind::Folder(k, path.clone()), key, label: dir.to_string(), level, expandable: true, expanded: open, main: false, count: None, parent: Some(parent) });
            if open {
                let inside: Vec<&ProjectFile> = files.iter().copied().filter(|f| f.path.starts_with(&path)).collect();
                self.folder_rows(rows, k, &path, &inside, level + 1, at);
            }
        }
        for f in files.iter().filter(|f| !f.path[prefix.len()..].contains('/')) {
            let comps = if k == FileKind::Form && self.show_components { self.comps.get(&f.path.to_lowercase()).map(Vec::as_slice).unwrap_or(&[]) } else { &[] };
            let open = !comps.is_empty() && self.expanded(&f.path);
            let at = rows.len();
            rows.push(Node {
                kind: NodeKind::File(f.path.clone()),
                key: f.path.clone(),
                label: name_of(&f.path).to_string(),
                level,
                expandable: !comps.is_empty(),
                expanded: open,
                main: f.path == self.project.main,
                count: None,
                parent: Some(parent),
            });
            if open {
                self.comp_rows(rows, &f.path, comps, level + 1, at);
            }
        }
    }

    fn comp_rows(&self, rows: &mut Vec<Node>, file: &str, comps: &[Comp], level: usize, parent: usize) {
        for c in comps {
            let key = format!("{file}#{}", c.name);
            let open = !c.children.is_empty() && self.expanded(&key);
            let at = rows.len();
            rows.push(Node {
                kind: NodeKind::Component { file: file.to_string(), name: c.name.clone(), ty: c.ty.clone() },
                key,
                label: c.name.clone(),
                level,
                expandable: !c.children.is_empty(),
                expanded: open,
                main: false,
                count: None,
                parent: Some(parent),
            });
            if open {
                self.comp_rows(rows, file, &c.children, level + 1, at);
            }
        }
    }

    /// Whether node `key` exists (shown or inside a closed one).
    pub fn has_node(&self, key: &str) -> bool {
        if key == KEY_PROJECT {
            return self.loaded;
        }
        if let Some(k) = key.strip_prefix("@group/") {
            return parse_kind(k).is_some_and(|k| self.project.files.iter().any(|f| f.kind == k));
        }
        if let Some(rest) = key.strip_prefix("@folder/") {
            let Some((k, path)) = rest.split_once('/') else { return false };
            return parse_kind(k).is_some_and(|k| self.project.files.iter().any(|f| f.kind == k && f.path.starts_with(path)));
        }
        match key.split_once('#') {
            Some((file, name)) => self.comp_path(file, name).is_some(),
            None => self.project.files.iter().any(|f| f.path == key),
        }
    }

    /// The names of the components around component `name` of `file`, from
    /// the form down (`None`: no such component).
    fn comp_path(&self, file: &str, name: &str) -> Option<Vec<String>> {
        fn find(cs: &[Comp], name: &str, trail: &mut Vec<String>) -> bool {
            for c in cs {
                if c.name.eq_ignore_ascii_case(name) {
                    return true;
                }
                trail.push(c.name.clone());
                if find(&c.children, name, trail) {
                    return true;
                }
                trail.pop();
            }
            false
        }
        let cs = self.comps.get(&file.to_lowercase())?;
        let mut trail = Vec::new();
        find(cs, name, &mut trail).then_some(trail)
    }

    /// The node the program names (a file's path, `path#Component`, a
    /// folder's `path/`): its key.
    pub fn key_of(&self, public: &str) -> Option<String> {
        let p = public.trim().replace('\\', "/");
        if let Some((file, name)) = p.split_once('#') {
            let f = self.file(file)?;
            let cs = self.comps.get(&f.path.to_lowercase())?;
            let exact = find_name(cs, name)?;
            return Some(format!("{}#{exact}", f.path));
        }
        if p.ends_with('/') {
            let f = self.project.files.iter().find(|f| f.path.to_lowercase().starts_with(&p.to_lowercase()))?;
            return Some(folder_key(f.kind, &f.path[..p.len()]));
        }
        self.file(&p).map(|f| f.path.clone())
    }

    /// Node `key` as the program sees it.
    pub fn public(key: &str) -> String {
        if let Some(rest) = key.strip_prefix("@folder/") {
            return rest.split_once('/').map(|(_, p)| p.to_string()).unwrap_or_default();
        }
        if key.starts_with('@') {
            return String::new();
        }
        key.to_string()
    }

    /// Opens the nodes above `key` (its group, folders, form, components).
    pub fn open_above(&mut self, key: &str) {
        self.set_open(KEY_PROJECT, true);
        let (file, comp) = match key.split_once('#') {
            Some((f, c)) => (f.to_string(), Some(c.to_string())),
            None if key.starts_with("@folder/") => {
                let rest = &key["@folder/".len()..];
                if let Some((k, path)) = rest.split_once('/') {
                    if let Some(k) = parse_kind(k) {
                        self.set_open(&group_key(k), true);
                        self.open_folders(k, path.trim_end_matches('/'));
                    }
                }
                return;
            }
            None => (key.to_string(), None),
        };
        let Some(f) = self.file(&file).cloned() else { return };
        self.set_open(&group_key(f.kind), true);
        self.open_folders(f.kind, folder_of(&f.path).trim_end_matches('/'));
        if let Some(c) = comp {
            self.set_open(&f.path, true);
            for above in self.comp_path(&f.path, &c).unwrap_or_default() {
                self.set_open(&format!("{}#{above}", f.path), true);
            }
        }
    }

    /// Group `k`'s folders down to `dir` ("a/b") opened.
    fn open_folders(&mut self, k: FileKind, dir: &str) {
        if dir.is_empty() {
            return;
        }
        let mut path = String::new();
        for part in dir.split('/') {
            path.push_str(part);
            path.push('/');
            self.set_open(&folder_key(k, &path), true);
        }
    }

    /// Every expandable node opened (`true`) or closed (the project stays open).
    pub fn open_all(&mut self, open: bool) {
        self.set_open(KEY_PROJECT, true);
        let mut keys: Vec<String> = KINDS.iter().map(|k| group_key(*k)).collect();
        for f in &self.project.files {
            let mut path = String::new();
            let dir = folder_of(&f.path).trim_end_matches('/');
            if !dir.is_empty() {
                for part in dir.split('/') {
                    path.push_str(part);
                    path.push('/');
                    keys.push(folder_key(f.kind, &path));
                }
            }
            if f.kind == FileKind::Form {
                keys.push(f.path.clone());
                fn comps(file: &str, cs: &[Comp], keys: &mut Vec<String>) {
                    for c in cs {
                        if !c.children.is_empty() {
                            keys.push(format!("{file}#{}", c.name));
                            comps(file, &c.children, keys);
                        }
                    }
                }
                comps(&f.path, self.comps.get(&f.path.to_lowercase()).map(Vec::as_slice).unwrap_or(&[]), &mut keys);
            }
        }
        for k in keys {
            self.set_open(&k, open);
        }
        // (the selection inside what closed: its shown ancestor)
        if !open {
            let rows = self.rows();
            if !self.selected.is_empty() && !rows.iter().any(|r| r.key == self.selected) {
                self.selected.clear();
            }
        }
    }
}

/// The written name of component `name` (any case) in `cs`.
fn find_name(cs: &[Comp], name: &str) -> Option<String> {
    cs.iter().find_map(|c| if c.name.eq_ignore_ascii_case(name) { Some(c.name.clone()) } else { find_name(&c.children, name) })
}
