//! The assistant's tools (docs/ide-ai.md §4.2, docs/ai-architecture.md
//! §3–5): what a model may call while it answers, all on the project open
//! in Studio and nothing beyond it.
//!
//! | Tool | Risk | What |
//! |---|---|---|
//! | `list_files` | read | the project's files |
//! | `read_file` | read | a file's text (the editor's when it is open), inside the project only |
//! | `rapidr_describe` | read | a component type's or builtin's registry entry (the introspection interface) |
//! | `rapidr_list`, `rapidr_get` | read | the forms' components and their properties, from the source (design time) |
//! | `check_code` | read | RapidR's compiler on a file, or on text as that file would be |
//! | `propose_edit` | proposal | a change to a file — only the parts that change (search and replace, each old text once in the file) — shown to the user as a diff: nothing is written until they apply it |
//! | `write_file` | proposal | a new file's text (or a file rewritten almost entirely), shown the same way |
//! | `create_project` | proposal | a new program's files, previewed before anything is created |
//!
//! What a tool returns from the project (a file, a property's value) goes
//! back to the model as data, marked untrusted with where it came from
//! (§5.5): never folded into the instructions.
//!
//! A proposal is refused, nothing proposed, when its text has a syntax
//! error or is cut off at the end (a reply that ran out half-way: `END
//! FUNCTI`), in a module too; and a whole-file rewrite of a file over
//! [`WHOLE_FILE_LINES`] lines that keeps most of it (a 2-line fix sent as
//! `+382 −384`) is refused with a word to send the changed parts with
//! propose_edit (docs/preview4-plan.md R11). Every check sees the project
//! with all the proposals still waiting for the user applied together.

use std::collections::HashMap;
use std::path::PathBuf;

use rapidr_ai::Tool;
use rapidr_introspect::{Reply, Request};
use rapidr_langsvc::{Analysis, Options};
use serde_json::{json, Value as Json};

/// A change the assistant proposes: the user sees it as a diff (an edit),
/// or a new project's file, and decides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Proposal {
    /// `edit` (an existing file changed), `create` (a new file of the
    /// project), `project` (a file of a new project).
    pub kind: String,
    /// The file, relative to the project's folder (to the new project's).
    pub path: String,
    pub old: String,
    pub new: String,
    /// One sentence: what the change does.
    pub summary: String,
    /// A new project's name ("" for changes to the open project): its
    /// folder's, made from the title the model gave.
    pub project: String,
    /// A new project's title, as the model gave it ("Scientific
    /// Calculator"): its project file's name for it.
    pub title: String,
    /// The new project's main file.
    pub main: bool,
    /// What the user decided: "", "applied", "rejected", "partly".
    pub decision: String,
}

/// What the tools see of the project.
pub struct Project<'a> {
    /// The project's folder ("" — none open).
    pub folder: &'a str,
    /// Its files, relative to the folder.
    pub files: &'a [String],
    /// Its main file (relative).
    pub main: &'a str,
    /// "rapidq" in a RapidQ-compatible project.
    pub compat: &'a str,
    /// The open editors' texts (relative paths).
    pub buffers: &'a HashMap<String, String>,
}

/// A tool's answer: its text, whether it failed, and what the transcript
/// says it did (one line) — and the data source it read, for the turn's
/// provenance.
pub use rapidr_ai::rai::ToolOutcome as Outcome;

fn ok(content: String, note: String, risk: &'static str, target: &str) -> Outcome {
    Outcome { content, is_error: false, note, read: None, risk, target: target.to_string() }
}

fn err(content: impl Into<String>, note: String, target: &str) -> Outcome {
    Outcome { content: content.into(), is_error: true, note, read: None, risk: "read", target: target.to_string() }
}

/// The largest text a tool hands a model at once (characters).
pub const MAX_TEXT: usize = 120_000;
/// The most files a new project may have.
const MAX_FILES: usize = 24;

fn schema(props: Json, required: &[&str]) -> Json {
    json!({"type": "object", "properties": props, "required": required, "additionalProperties": false})
}

/// The tools a model is offered: the read tools always, the proposing ones
/// unless the permission is read-only.
pub fn tools(proposals: bool) -> Vec<Tool> {
    let t = |name: &str, description: &str, input_schema: Json| Tool { name: name.into(), description: description.into(), input_schema };
    let mut out = vec![
        t("list_files", "The open project's files (paths relative to its folder), its main file first.", schema(json!({}), &[])),
        t("read_file", "A file of the open project: its text as the editor has it (unsaved changes included). Only files inside the project.", schema(json!({"path": {"type": "string", "description": "the file's path, relative to the project's folder"}}), &["path"])),
        t(
            "rapidr_describe",
            "RapidR's registry entry for a component type (its properties with types, ranges and defaults, its methods, its events, an example) or for a builtin; or every type / builtin in one line each. Use it before using a component you haven't checked.",
            schema(json!({"type": {"type": "string", "description": "a component type: RButton, RForm, RStringGrid (RapidQ names too: QBUTTON)"}, "builtin": {"type": "string", "description": "a builtin function or statement: MID$, SHOWMESSAGE"}, "what": {"type": "string", "enum": ["types", "builtins"], "description": "list every type or every builtin"}}), &[]),
        ),
        t(
            "rapidr_list",
            "The components the project's source creates (its CREATE blocks), in creation order: their ids (names), types, parents and forms, with the file and line. Optionally only those inside `scope` (a component's id) or of one `type`.",
            schema(json!({"scope": {"type": "string"}, "type": {"type": "string"}}), &[]),
        ),
        t(
            "rapidr_get",
            "One component's design-time properties (as the CREATE block writes them, or their defaults) and its bound events, typed; `props` to ask for some only.",
            schema(json!({"id": {"type": "string", "description": "the component's name, as rapidr_list gives it"}, "props": {"type": "array", "items": {"type": "string"}}}), &["id"]),
        ),
        t(
            "check_code",
            "RapidR's compiler on a file of the project — or on `text` as that file would be with it — and the project's other files: its errors and warnings with line and column, or that there are none. Check every change before you propose it.",
            schema(json!({"path": {"type": "string", "description": "the file (relative); default: the main file"}, "text": {"type": "string", "description": "the file's text to check instead of what it has now"}}), &[]),
        ),
        t(
            "find_example",
            "A short working RapidR program close to what you describe (a calculator, a timer, a to-do list, a quiz, a drawing pad, a CSV grid, a list with Start/Stop …), from RapidR's recipes: start from it and adapt it.",
            schema(json!({"query": {"type": "string", "description": "what the program does, in a few words"}}), &["query"]),
        ),
        t(
            "project_outline",
            "The open project as the compiler reads it: its files in include order (which file $INCLUDEs which), and for each file its forms and their components, its SUBs and FUNCTIONs with their parameters, its globals (DIM) with their types, its CONSTs and TYPEs, each with its line. Call it before writing changes to a project of several files, and before adding a DIM, SUB or FUNCTION: a name is declared once in the whole program, and only SUBs and FUNCTIONs listed here (or builtins) can be called.",
            schema(json!({}), &[]),
        ),
        t(
            "search_manual",
            "RapidR's user manual (its guide pages), searched: the three sections that best answer a question (\"how do I add a menu\", \"read a CSV file\", \"breakpoints\", \"serial port\"), each trimmed, with its page and heading.",
            schema(json!({"query": {"type": "string", "description": "the question, or a few words of it"}}), &["query"]),
        ),
    ];
    if proposals {
        out.push(t(
            "layout",
            "A grid of buttons laid out for you (a keypad, a toolbar): give the captions row by row; it returns the CREATE blocks with exact Left / Top / Width / Height, one shared OnClick handler, and the ClientWidth / ClientHeight the form needs. Paste them into the form's CREATE block.",
            schema(
                json!({
                    "rows": {"type": "array", "items": {"type": "array", "items": {"type": "string"}}, "description": "the captions, row by row: [[\"7\",\"8\",\"9\",\"/\"],[\"4\",\"5\",\"6\",\"*\"]]"},
                    "left": {"type": "integer", "description": "the grid's left edge (default 8)"},
                    "top": {"type": "integer", "description": "the grid's top edge (default 8; below a display, its bottom + 8)"},
                    "width": {"type": "integer", "description": "each button's width (default 56)"},
                    "height": {"type": "integer", "description": "each button's height (default 36)"},
                    "gap": {"type": "integer", "description": "pixels between buttons (default 4)"},
                    "on_click": {"type": "string", "description": "the SUB every button calls (SUB Name (Sender AS RButton): Sender.Caption says which)"}
                }),
                &["rows"],
            ),
        ));
        out.push(t(
            "propose_edit",
            "The way to change an existing file of the project: send only the parts that change, shown to the user as a diff they accept or reject (nothing changes until they do). Each edit replaces `old_text` — a few whole lines copied exactly from the file, appearing exactly once in it — with `new_text`. Several edits in one call for changes in several places.",
            schema(
                json!({
                    "path": {"type": "string"},
                    "edits": {"type": "array", "items": {"type": "object", "properties": {"old_text": {"type": "string"}, "new_text": {"type": "string"}}, "required": ["old_text", "new_text"], "additionalProperties": false}},
                    "summary": {"type": "string", "description": "one sentence: what the change does"}
                }),
                &["path", "edits", "summary"],
            ),
        ));
        out.push(t(
            "write_file",
            "A new file's whole text (or, rarely, an existing file rewritten almost entirely), shown to the user as a diff they accept or reject. To change part of an existing file use propose_edit: a whole-file rewrite of a file over 40 lines that keeps most of it is refused.",
            schema(json!({"path": {"type": "string"}, "text": {"type": "string"}, "summary": {"type": "string"}}), &["path", "text", "summary"]),
        ));
        out.push(t(
            "create_project",
            "Makes a new program as a new project — its files (a main .rr file; more for forms or modules it $INCLUDEs) — which Studio opens for the user when your answer ends. Checked by the compiler: fix the errors it reports and call it again.",
            schema(
                json!({
                    "name": {"type": "string", "description": "the program's title, e.g. Scientific Calculator (its folder is made from it)"},
                    "main": {"type": "string", "description": "the main file's path, e.g. main.rr"},
                    "files": {"type": "array", "items": {"type": "object", "properties": {"path": {"type": "string"}, "text": {"type": "string"}}, "required": ["path", "text"], "additionalProperties": false}},
                    "summary": {"type": "string"}
                }),
                &["name", "main", "files", "summary"],
            ),
        ));
    }
    out
}

/// The lite set for a small context window (under 16K tokens; K8): the
/// tools a small model needs to read, check and change a program, each
/// said in a line — no form introspection, examples, outline or layout.
pub fn tools_lite(proposals: bool) -> Vec<Tool> {
    let t = |name: &str, description: &str, input_schema: Json| Tool { name: name.into(), description: description.into(), input_schema };
    let path = || json!({"type": "string"});
    let mut out = vec![
        t("list_files", "The project's files.", schema(json!({}), &[])),
        t("read_file", "A project file's text.", schema(json!({"path": path()}), &["path"])),
        t("rapidr_describe", "A component's or builtin's members and syntax.", schema(json!({"type": {"type": "string"}, "builtin": {"type": "string"}}), &[])),
        t("check_code", "Compiles a file (or `text` as that file): its errors.", schema(json!({"path": path(), "text": {"type": "string"}}), &[])),
        t("search_manual", "RapidR's manual: the sections that answer a question.", schema(json!({"query": {"type": "string"}}), &["query"])),
    ];
    if proposals {
        out.push(t(
            "propose_edit",
            "Changes a file: each edit's old_text (whole lines, once in the file) becomes new_text. The user reviews it.",
            schema(json!({"path": path(), "edits": {"type": "array", "items": {"type": "object", "properties": {"old_text": {"type": "string"}, "new_text": {"type": "string"}}, "required": ["old_text", "new_text"], "additionalProperties": false}}, "summary": {"type": "string"}}), &["path", "edits", "summary"]),
        ));
        out.push(t("write_file", "A new file's whole text.", schema(json!({"path": path(), "text": {"type": "string"}, "summary": {"type": "string"}}), &["path", "text", "summary"])));
        out.push(t(
            "create_project",
            "A new program (main.rr, and other files it $INCLUDEs), opened in Studio when you finish.",
            schema(json!({"name": {"type": "string"}, "main": {"type": "string"}, "files": {"type": "array", "items": {"type": "object", "properties": {"path": path(), "text": {"type": "string"}}, "required": ["path", "text"], "additionalProperties": false}}, "summary": {"type": "string"}}), &["name", "main", "files", "summary"]),
        ));
    }
    out
}

/// A tool call as the live status line says it, while the model writes it
/// or it runs ("Reading main.rr", "Searching the manual", "Writing
/// server.rr"); `input` may hold only the fields written so far.
pub fn activity(name: &str, input: &Json) -> Option<String> {
    let field = |k: &str| input.get(k).and_then(Json::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    Some(match name {
        "read_file" => field("path").map_or_else(|| "Reading a file".to_string(), |p| format!("Reading {p}")),
        "list_files" => "Listing the files".into(),
        "project_outline" => "Reading the project's outline".into(),
        "search_manual" => "Searching the manual".into(),
        "find_example" => "Finding an example".into(),
        "rapidr_describe" => field("type").or_else(|| field("builtin")).map_or_else(|| "Looking up RapidR's reference".to_string(), |t| format!("Looking up {t}")),
        "rapidr_list" | "rapidr_get" => "Looking at the form".into(),
        "check_code" => "Checking the code".into(),
        "layout" => "Laying out the buttons".into(),
        "propose_edit" | "write_file" => field("path").map_or_else(|| "Writing a change".to_string(), |p| format!("Writing {p}")),
        "create_project" => field("name").map_or_else(|| "Writing the new program".to_string(), |n| format!("Writing {n}")),
        _ => return None,
    })
}

/// A path the model gave, as a project's relative path: inside the
/// project's folder, never above it.
pub fn project_path(folder: &str, path: &str) -> Result<String, String> {
    let p = crate::slashes(path.trim());
    let folder = crate::slashes(folder.trim_end_matches(['/', '\\']));
    let rel = if !folder.is_empty() && (p == folder || p.starts_with(&format!("{folder}/"))) {
        p[folder.len()..].trim_start_matches('/').to_string()
    } else {
        p.trim_start_matches("./").to_string()
    };
    if rel.is_empty() {
        return Err("give a file's path, relative to the project's folder".into());
    }
    if rel.starts_with('/') || rel.get(1..3) == Some(":/") || rel.starts_with("//") {
        return Err(format!("{path}: outside the project (only its own files can be read or changed)"));
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in rel.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(format!("{path}: outside the project (only its own files can be read or changed)"));
                }
            }
            p => parts.push(p),
        }
    }
    if parts.is_empty() {
        return Err("give a file's path, relative to the project's folder".into());
    }
    let rel = parts.join("/");
    // (and as the disk has it: a link inside the project to a place outside
    // it is refused too, SEC-23)
    if !folder.is_empty() && !inside_for_real(&folder, &rel) {
        return Err(format!("{path}: outside the project, through a link (only the project's own files can be read or changed)"));
    }
    Ok(rel)
}

/// Whether `rel` under `folder` stays inside the folder once links are
/// followed: the deepest part of it that exists, resolved, is in the
/// folder resolved, and no part is a link to nowhere (a write would follow
/// it). A folder that isn't on this disk (the web's store) has no links.
fn inside_for_real(folder: &str, rel: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (folder, rel);
        true
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Ok(root) = std::fs::canonicalize(folder) else { return true };
        let mut at = std::path::PathBuf::from(crate::join(folder, rel));
        loop {
            match std::fs::canonicalize(&at) {
                Ok(real) => return real.starts_with(&root),
                // (a link whose target isn't there)
                Err(_) if std::fs::symlink_metadata(&at).is_ok() => return false,
                Err(_) => {
                    if !at.pop() {
                        return false;
                    }
                }
            }
        }
    }
}

impl Project<'_> {
    fn full(&self, rel: &str) -> String {
        crate::join(self.folder, rel)
    }

    /// A project file's text: the editor's, else the file's.
    pub fn text(&self, rel: &str) -> Result<String, String> {
        if let Some(t) = self.buffers.get(rel) {
            return Ok(t.clone());
        }
        if self.folder.is_empty() {
            return Err(format!("{rel}: no project is open"));
        }
        crate::read_text(&self.full(rel)).map_err(|e| format!("{rel}: {e}"))
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.buffers.contains_key(rel) || self.files.iter().any(|f| f == rel) || (!self.folder.is_empty() && rapidr_value::objects::read_file(&self.full(rel)).is_ok())
    }

    /// Every project file's text (relative path, text), the main file first.
    fn all(&self) -> Vec<(String, String)> {
        let mut names: Vec<String> = Vec::new();
        if !self.main.is_empty() {
            names.push(self.main.to_string());
        }
        for f in self.files.iter().chain(self.buffers.keys()) {
            if !names.contains(f) && is_source(f) {
                names.push(f.clone());
            }
        }
        names.into_iter().filter_map(|n| self.text(&n).ok().map(|t| (n, t))).collect()
    }
}

/// A BASIC source file (what the compiler and the designer read).
fn is_source(path: &str) -> bool {
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    matches!(ext.as_str(), "rr" | "bas" | "inc" | "rqw" | "rqb" | "rq")
}

/// Data from the project, marked as such for the model.
fn untrusted(source: &str, content: Json) -> String {
    json!({"untrusted": true, "source": source, "content": content}).to_string()
}

fn clip(text: &str) -> (String, bool) {
    if text.chars().count() <= MAX_TEXT {
        (text.to_string(), false)
    } else {
        (text.chars().take(MAX_TEXT).collect(), true)
    }
}

fn s<'a>(input: &'a Json, key: &str) -> &'a str {
    input.get(key).and_then(Json::as_str).unwrap_or("")
}

/// Runs a tool. `proposals` collects what the proposing tools propose (a
/// second proposal for a file in the same turn builds on the first).
pub fn run(name: &str, input: &Json, project: &Project, proposals: &mut Vec<Proposal>, may_propose: bool) -> Outcome {
    if let Some(raw) = input.get("_invalid_json").and_then(Json::as_str) {
        return err(format!("the arguments weren't valid JSON ({}…): call {name} again with a JSON object", raw.chars().take(60).collect::<String>()), format!("{name}: arguments not understood"), "");
    }
    if let Some(out) = on_pending(name, input, project, proposals, may_propose) {
        return out;
    }
    match name {
        "list_files" => list_files(project),
        "read_file" => read_file(input, project),
        "rapidr_describe" => describe(input),
        "rapidr_list" | "rapidr_get" => design(name, input, project),
        "check_code" => check(input, project),
        "find_example" => find_example(input),
        "project_outline" => project_outline(project),
        "search_manual" => search_manual(input),
        "layout" => layout(input),
        "propose_edit" | "write_file" | "create_project" if !may_propose => err("the user allowed reading only (Tools > Settings > AI > Permission): describe the change in words instead", format!("{name}: not allowed (read-only)"), ""),
        // (a proposing tool's risk is a proposal's, whether it fitted or not:
        // the audit log says what was tried)
        "propose_edit" => Outcome { risk: "state", ..propose_edit(input, project, proposals) },
        // (write_file called with create_project's arguments — a small model
        // mixes them up: what they say is done — a new program when none is
        // open, else each file written)
        "write_file" if input.get("path").is_none() && input.get("files").and_then(Json::as_array).is_some() => {
            if project.folder.is_empty() {
                return Outcome { risk: "state", ..create_project(input, proposals) };
            }
            let mut last = err("no files given", "write_file: no files".into(), "");
            for f in input["files"].as_array().into_iter().flatten() {
                let one = json!({"path": f.get("path").cloned().unwrap_or(Json::Null), "text": f.get("text").cloned().unwrap_or(Json::Null), "summary": input.get("summary").cloned().unwrap_or(json!(""))});
                last = write_file(&one, project, proposals);
                if last.is_error {
                    break;
                }
            }
            Outcome { risk: "state", ..last }
        }
        "write_file" => Outcome { risk: "state", ..write_file(input, project, proposals) },
        "create_project" => Outcome { risk: "state", ..create_project(input, proposals) },
        other => err(format!("there is no tool `{other}`: the tools are {}", tools(may_propose).iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ")), format!("{other}: no such tool"), ""),
    }
}

/// The new project the proposals hold that the user hasn't created yet
/// (create_project's): its name and main file.
pub fn pending(proposals: &[Proposal]) -> Option<(String, String)> {
    let files: Vec<&Proposal> = proposals.iter().filter(|p| p.kind == "project" && p.decision != "created").collect();
    let first = files.first()?;
    let main = files.iter().find(|p| p.main).map_or_else(|| first.path.clone(), |p| p.path.clone());
    Some((first.project.clone(), main))
}

/// A tool on the new project proposed and not created yet: with no project
/// open — or with a path that names it (`Calc/main.rr`) — reading,
/// checking and changing work on its files, as if it were open, and a
/// change updates the proposal (what the Preview shows). A small model
/// fixes its new program's errors with read_file and write_file before the
/// user presses Create; "no project open" there lost it.
fn on_pending(name: &str, input: &Json, project: &Project, proposals: &mut Vec<Proposal>, may_propose: bool) -> Option<Outcome> {
    if !matches!(name, "list_files" | "read_file" | "check_code" | "propose_edit" | "write_file" | "rapidr_list" | "rapidr_get" | "project_outline") {
        return None;
    }
    // (write_file with create_project's arguments: a new project again)
    if name == "write_file" && input.get("path").is_none() && input.get("files").is_some() {
        return None;
    }
    let (pname, main) = pending(proposals)?;
    let named = crate::slashes(s(input, "path").trim());
    let named = named.trim_start_matches("./");
    let stripped = named.get(..pname.len() + 1).filter(|p| p.eq_ignore_ascii_case(&format!("{pname}/"))).map(|_| named[pname.len() + 1..].to_string());
    if !project.folder.is_empty() && stripped.is_none() {
        return None;
    }
    let mut input = input.clone();
    if let (Some(rel), Some(o)) = (stripped, input.as_object_mut()) {
        o.insert("path".into(), json!(rel));
    }
    let files: Vec<String> = proposals.iter().filter(|p| p.kind == "project" && p.project == pname).map(|p| p.path.clone()).collect();
    let buffers: HashMap<String, String> = proposals.iter().filter(|p| p.kind == "project" && p.project == pname).map(|p| (p.path.clone(), p.new.clone())).collect();
    let folder = format!("/new/{pname}");
    let vp = Project { folder: &folder, files: &files, main: &main, compat: "", buffers: &buffers };
    let not_yet = format!("the new project {pname}, proposed and not created yet (the user creates it with Create; until then you can still change it with write_file or propose_edit)");
    match name {
        "list_files" => {
            let o = list_files(&vp);
            Some(Outcome { content: format!("No project is open: these are the files of {not_yet}:\n{}", o.content), note: format!("Listed the new project {pname}'s files"), ..o })
        }
        "propose_edit" | "write_file" if may_propose => {
            let mut changed = Vec::new();
            let out = run(name, &input, &vp, &mut changed, true);
            if out.is_error {
                return Some(Outcome { risk: "state", ..out });
            }
            let (summary, title) = proposals.iter().find(|p| p.kind == "project" && p.project == pname).map(|p| (p.summary.clone(), p.title.clone())).unwrap_or_default();
            for q in changed {
                match proposals.iter_mut().find(|p| p.kind == "project" && p.project == pname && p.path == q.path) {
                    Some(p) => p.new = q.new,
                    None => proposals.push(Proposal { kind: "project".into(), path: q.path, new: q.new, summary: summary.clone(), project: pname.clone(), title: title.clone(), ..Proposal::default() }),
                }
            }
            let got: Vec<(String, String)> = proposals.iter().filter(|p| p.kind == "project" && p.project == pname).map(|p| (p.path.clone(), p.new.clone())).collect();
            let found = diagnostics(&got, &folder, &main, &main, false);
            let rel = s(&input, "path").to_string();
            let errors = found.iter().filter(|l| l.starts_with("error")).count();
            Some(if errors == 0 {
                ok(format!("Changed {rel} of {not_yet}. It compiles. Tell the user in a sentence what changed."), format!("Changed the new project {pname}: {rel}"), "state", &rel)
            } else {
                let shown: Vec<&String> = found.iter().filter(|l| l.starts_with("error") || l.starts_with("hint")).take(24).collect();
                ok(
                    format!("Changed {rel} of {not_yet}, but it doesn't compile yet — fix these now (write_file with the file's whole corrected text):\n{}", shown.iter().map(|l| l.as_str()).collect::<Vec<_>>().join("\n")),
                    format!("Changed the new project {pname}: {rel} (errors to fix)"),
                    "state",
                    &rel,
                )
            })
        }
        "propose_edit" | "write_file" => None,
        _ => Some(run(name, &input, &vp, &mut Vec::new(), may_propose)),
    }
}

fn list_files(p: &Project) -> Outcome {
    if p.folder.is_empty() && p.buffers.is_empty() {
        return ok("No project is open. To make a new program, use create_project.".into(), "Looked at the project: none is open".into(), "read", "");
    }
    let mut names: Vec<String> = Vec::new();
    if !p.main.is_empty() {
        names.push(format!("{} (main)", p.main));
    }
    for f in p.files.iter().chain(p.buffers.keys()) {
        if f != p.main && !names.iter().any(|n| n == f) {
            names.push(f.clone());
        }
    }
    let n = names.len();
    ok(names.join("\n"), format!("Listed the project's files ({n})"), "read", "")
}

fn read_file(input: &Json, p: &Project) -> Outcome {
    let rel = match project_path(p.folder, s(input, "path")) {
        Ok(r) => r,
        Err(e) => return err(e, format!("Refused to read {}: outside the project", s(input, "path")), s(input, "path")),
    };
    match p.text(&rel) {
        Ok(text) => {
            // (a key a developer wrote into a file never reaches the model)
            let text = rapidr_ai::keys::redact(&text);
            let (text, cut) = clip(&text);
            let lines = text.lines().count();
            let mut out = untrusted(&format!("file:{rel}"), json!(text));
            if cut {
                out.push_str(&format!("\n(only its first {MAX_TEXT} characters)"));
            }
            Outcome { content: out, is_error: false, note: format!("Read {rel} ({lines} lines)"), read: Some(format!("file:{rel}")), risk: "read", target: rel }
        }
        Err(e) => err(e, format!("Couldn't read {rel}"), &rel),
    }
}

/// `describe`'s JSON as compact text: a model reads it in a fraction of
/// the tokens.
pub fn describe_text(v: &Json) -> String {
    let mut out = String::new();
    let line = |out: &mut String, t: String| {
        out.push_str(&t);
        out.push('\n');
    };
    let first = |d: &str, max: usize| -> String {
        let end = d.find(". ").map_or(d.len(), |i| i + 1);
        let t = d[..end].trim();
        if t.chars().count() > max { format!("{}…", t.chars().take(max).collect::<String>()) } else { t.to_string() }
    };
    if let Some(types) = v.get("types").and_then(Json::as_array) {
        for t in types {
            line(&mut out, format!("{} [{}]: {}", t["type"].as_str().unwrap_or(""), t["group"].as_str().unwrap_or(""), t["doc"].as_str().unwrap_or("")));
        }
        return out;
    }
    if let Some(bs) = v.get("builtins").and_then(Json::as_array) {
        for b in bs {
            line(&mut out, b["syntax"].as_str().unwrap_or("").to_string());
        }
        return out;
    }
    if let Some(b) = v.get("builtin").and_then(Json::as_str) {
        line(&mut out, format!("{b}: {}", v["syntax"].as_str().unwrap_or("")));
        line(&mut out, v["doc"].as_str().unwrap_or("").to_string());
        if let Some(e) = v["effects"].as_array() {
            line(&mut out, format!("Effects: {}", e.iter().filter_map(Json::as_str).collect::<Vec<_>>().join(", ")));
        }
        if let Some(x) = v["example"].as_str() {
            line(&mut out, format!("Example:\n{x}"));
        }
        return out;
    }
    let ty = v["type"].as_str().unwrap_or("");
    let q = v["rapidq"].as_str().map(|q| format!(" (RapidQ: {q})")).unwrap_or_default();
    let kind = if v["visual"].as_bool() == Some(true) { "visual component" } else { "non-visual component" };
    line(&mut out, format!("{ty}{q}: {kind}. {}", v["doc"].as_str().unwrap_or("")));
    if let Some(only) = v["only"].as_str() {
        line(&mut out, format!("Runs on: {only} only."));
    }
    if let Some(e) = v["defaultEvent"].as_str() {
        line(&mut out, format!("Default event: {e}"));
    }
    line(&mut out, "Properties (name: type = default; [read-only]; range, unit):".into());
    for p in v["properties"].as_array().into_iter().flatten() {
        let mut t = format!("- {}: {}", p["name"].as_str().unwrap_or(""), p["type"].as_str().unwrap_or(""));
        if let Some(vals) = p["values"].as_array() {
            t.push_str(&format!(" ({})", vals.iter().filter_map(Json::as_str).collect::<Vec<_>>().join("|")));
        }
        if let Some(k) = p["kinds"].as_array() {
            t.push_str(&format!(" (a {})", k.iter().filter_map(Json::as_str).collect::<Vec<_>>().join(" or ")));
        }
        match &p["default"] {
            Json::Null => {}
            Json::Object(o) => {
                if let Some(c) = o.get("const").or_else(|| o.get("set").map(|_| &o["value"])).or_else(|| o.get("color")) {
                    t.push_str(&format!(" = {}", c.as_str().map_or_else(|| c.to_string(), str::to_string)));
                }
            }
            d => t.push_str(&format!(" = {d}")),
        }
        match p["access"].as_str() {
            Some("read") => t.push_str(" [read-only]"),
            Some("write") => t.push_str(" [write-only]"),
            _ => {}
        }
        if let Some(r) = p["range"].as_str().filter(|r| *r != "any") {
            t.push_str(&format!(" {r}"));
        }
        if let Some(u) = p["unit"].as_str() {
            t.push_str(&format!(" {u}"));
        }
        if let Some(d) = p["doc"].as_str().filter(|d| !d.is_empty()) {
            t.push_str(&format!(" — {}", first(d, 100)));
        }
        line(&mut out, t);
    }
    line(&mut out, "Methods:".into());
    for m in v["methods"].as_array().into_iter().flatten() {
        let mut t = format!("- {}", m["signature"].as_str().or(m["name"].as_str()).unwrap_or(""));
        if let Some(e) = m["effects"].as_array() {
            t.push_str(&format!(" [{}]", e.iter().filter_map(Json::as_str).collect::<Vec<_>>().join(", ")));
        }
        if let Some(d) = m["doc"].as_str().filter(|d| !d.is_empty()) {
            t.push_str(&format!(" — {}", first(d, 100)));
        }
        line(&mut out, t);
    }
    line(&mut out, "Events (handler: SUB Name(params), bound with OnX = Name):".into());
    for e in v["events"].as_array().into_iter().flatten() {
        let params: Vec<String> = e["params"].as_array().into_iter().flatten().map(|p| format!("{} AS {}", p["name"].as_str().unwrap_or(""), p["type"].as_str().unwrap_or("INTEGER"))).collect();
        let mut t = format!("- {}({})", e["name"].as_str().unwrap_or(""), params.join(", "));
        if let Some(d) = e["doc"].as_str().filter(|d| !d.is_empty()) {
            t.push_str(&format!(" — {}", first(d, 90)));
        }
        line(&mut out, t);
    }
    if let Some(x) = v["example"].as_str() {
        line(&mut out, format!("Example:\n{x}"));
    }
    out
}

fn describe(input: &Json) -> Outcome {
    let opt = |k: &str| input.get(k).and_then(Json::as_str).filter(|v| !v.trim().is_empty()).map(|v| v.trim().to_string());
    let (type_name, builtin, what) = (opt("type"), opt("builtin"), opt("what"));
    let target = type_name.clone().or_else(|| builtin.clone()).or_else(|| what.clone()).unwrap_or_default();
    let req = Request::Describe { type_name, builtin, what };
    match rapidr_introspect::handle(None, &req) {
        Reply::Ok(v) => ok(describe_text(&v), format!("Looked up {target} in RapidR's registry"), "read", &target),
        Reply::Err(e) => err(e.message, format!("Looked up {target}: not found"), &target),
    }
}

/// A button's name from its caption: `Key7`, `KeyPlus`, `KeySqrt`.
fn key_name(prefix: &str, caption: &str) -> String {
    let words: String = caption
        .chars()
        .map(|c| match c {
            '+' => "Plus".to_string(),
            '-' => "Minus".to_string(),
            '*' | '×' => "Times".to_string(),
            '/' | '÷' => "Divide".to_string(),
            '=' => "Equals".to_string(),
            '.' => "Dot".to_string(),
            '%' => "Percent".to_string(),
            '^' => "Power".to_string(),
            '(' => "Open".to_string(),
            ')' => "Close".to_string(),
            '√' => "Sqrt".to_string(),
            '±' => "Sign".to_string(),
            c if c.is_ascii_alphanumeric() => c.to_string(),
            _ => String::new(),
        })
        .collect();
    let mut w = words;
    if let Some(f) = w.get(..1) {
        w = f.to_ascii_uppercase() + &w[1..];
    }
    format!("{prefix}{w}")
}

/// The `layout` tool: a grid of buttons as CREATE blocks with exact
/// coordinates, and the client size the form needs.
fn layout(input: &Json) -> Outcome {
    let rows: Vec<Vec<String>> = input.get("rows").and_then(Json::as_array).into_iter().flatten().map(|r| r.as_array().into_iter().flatten().map(|c| c.as_str().map_or_else(|| c.to_string(), str::to_string)).collect()).filter(|r: &Vec<String>| !r.is_empty()).collect();
    if rows.is_empty() || rows.len() > 20 || rows.iter().any(|r| r.len() > 12) {
        return err("give rows: the captions row by row (at most 20 rows of 12)", "layout: no rows".into(), "");
    }
    let n = |k: &str, d: i64| input.get(k).and_then(Json::as_i64).filter(|v| (0..=2000).contains(v)).unwrap_or(d);
    let (left, top, w, h, gap) = (n("left", 8), n("top", 8), n("width", 56).max(16), n("height", 36).max(16), n("gap", 4));
    let handler = s(input, "on_click").trim();
    let handler: String = handler.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    let mut names: Vec<String> = Vec::new();
    let mut lines = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, caption) in row.iter().enumerate() {
            if caption.is_empty() {
                continue;
            }
            let mut name = key_name("Key", caption);
            if name == "Key" || names.contains(&name) {
                name = format!("Key{}_{}", r + 1, c + 1);
            }
            names.push(name.clone());
            let on = if handler.is_empty() { String::new() } else { format!(": OnClick = {handler}") };
            lines.push(format!("    CREATE {name} AS RButton: Caption = \"{}\": Left = {}: Top = {}: Width = {w}: Height = {h}{on}: END CREATE", caption.replace('"', "\"\""), left + c as i64 * (w + gap), top + r as i64 * (h + gap)));
        }
    }
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0) as i64;
    let right = left + cols * w + (cols - 1).max(0) * gap + left;
    let bottom = top + rows.len() as i64 * h + (rows.len() as i64 - 1) * gap + left;
    let decl = if handler.is_empty() { String::new() } else { format!("\nThe handler: DECLARE SUB {handler} (Sender AS RButton) at the top, and SUB {handler} (Sender AS RButton) … END SUB — Sender.Caption is the key pressed.") };
    ok(format!("{}\n\nThe grid needs the form's client area to be at least ClientWidth = {right}: ClientHeight = {bottom} (more below or beside it for other controls).{decl}", lines.join("\n")), format!("Laid out {} buttons", names.len()), "read", "")
}

/// The recipe closest to `query`, whole, and the next ones' names.
fn find_example(input: &Json) -> Outcome {
    let query = s(input, "query");
    let found = crate::recipes::search(query);
    let Some((_, best)) = found.first() else {
        return ok(format!("No recipe is close to \"{query}\". The recipes: {}.", crate::recipes::all().iter().map(|r| r.title.as_str()).collect::<Vec<_>>().join("; ")), format!("Looked for an example of {query}: none close"), "read", query);
    };
    let others: Vec<&str> = found.iter().skip(1).take(2).map(|(_, r)| r.title.as_str()).collect();
    let more = if others.is_empty() { String::new() } else { format!("\nAlso close: {}.", others.join("; ")) };
    ok(format!("Recipe \"{}\", a working program — start from it and adapt it:\n```basic\n{}\n```{more}", best.title, best.code()), format!("Found an example: {}", best.title), "read", query)
}

/// `search_manual`: the manual's three sections that answer the question
/// best (crate::manual_search). The manual is RapidR's own text.
fn search_manual(input: &Json) -> Outcome {
    let query = s(input, "query").trim();
    if query.is_empty() {
        return err("give `query`: the question, or a few words of it", "search_manual: no question".into(), "");
    }
    let found = crate::manual_search::search(query, 3);
    if found.is_empty() {
        return ok(format!("The manual has nothing on \"{query}\". Try other words, or rapidr_describe for a component or builtin."), format!("Searched the manual for {query}: nothing"), "read", query);
    }
    let text = found.iter().map(|s| crate::manual_search::excerpt(s)).collect::<Vec<_>>().join("\n\n---\n\n");
    let where_ = found.iter().map(|s| s.page).collect::<Vec<_>>().join(", ");
    ok(text, format!("Searched the manual for {query}: {where_}"), "read", query)
}

/// One line of the outline: where it is (a file's line) and what.
struct Mark {
    line: usize,
    what: String,
}

/// `project_outline`: the program as the compiler reads it — its files in
/// include order, and what each declares (forms and their components,
/// SUBs and FUNCTIONs with their parameters, globals with their types,
/// CONSTs, TYPEs), from the language service's parse of the whole program
/// (every file the main file includes, waiting proposals applied). A file
/// the main file doesn't include is said, with its own names.
fn project_outline(p: &Project) -> Outcome {
    let files = p.all();
    if files.is_empty() {
        return ok("No project is open. To make a new program, use create_project.".into(), "Outlined the project: none is open".into(), "read", "");
    }
    let root = "/project";
    let full = |rel: &str| PathBuf::from(format!("{root}/{rel}"));
    let rel_of = |path: &std::path::Path| {
        let f = path.to_string_lossy().replace('\\', "/");
        f.strip_prefix(&format!("{root}/")).unwrap_or(&f).to_string()
    };
    let mut a = Analysis::new(Options { rapidq_compatible: p.compat.eq_ignore_ascii_case("rapidq"), roots: Some(Vec::new()), ..Options::default() });
    for (rel, text) in &files {
        a.update(full(rel), text.clone());
    }
    let main = if p.main.is_empty() { files[0].0.clone() } else { p.main.to_string() };
    let mut out = String::new();
    let mut outlined: Vec<String> = Vec::new();
    let program = |a: &mut Analysis, main: &str, out: &mut String, outlined: &mut Vec<String>| {
        let Some(snap) = a.snapshot(&full(main)) else { return };
        let pre = &snap.parsed.tools.preprocessed;
        // (the files in the order the compiler reads them; RapidR's own
        // libraries aren't the project's)
        let order: Vec<String> = pre.origins.files.iter().filter_map(|f| f.path.as_deref()).filter(|p| !p.to_string_lossy().starts_with("<RapidR>")).map(rel_of).collect();
        let mut included_by: HashMap<String, String> = HashMap::new();
        let mut notes: HashMap<String, Vec<String>> = HashMap::new();
        for u in &pre.includes {
            let from = pre.origins.files.get(u.file).and_then(|f| f.path.as_deref()).map(rel_of).unwrap_or_else(|| main.to_string());
            match &u.found {
                rapidr_preprocessor::IncludeFound::File(path) => {
                    included_by.entry(rel_of(path)).or_insert_with(|| format!("{from}:{}", u.line));
                }
                rapidr_preprocessor::IncludeFound::BuiltIn => notes.entry(from).or_default().push(format!("$INCLUDE \"{}\" (line {}): built into RapidR", u.written, u.line)),
                rapidr_preprocessor::IncludeFound::Missing => notes.entry(from).or_default().push(format!("$INCLUDE \"{}\" (line {}): NOT FOUND — the program doesn't compile", u.written, u.line)),
            }
        }
        let mut marks: HashMap<String, Vec<Mark>> = HashMap::new();
        let mut place = |span: rapidr_diagnostics::TextSpan, what: String, a: &Analysis| {
            let Some(l) = snap.locate(span) else { return };
            let text = a.text(&l.file).unwrap_or_default();
            let (line, _) = crate::langsvc::line_col(&text, l.start);
            marks.entry(rel_of(&l.file)).or_default().push(Mark { line, what });
        };
        let source = |span: rapidr_diagnostics::TextSpan| -> String {
            let s = pre.source.get(span.start..span.end.min(pre.source.len())).unwrap_or("");
            s.lines().next().unwrap_or("").split('\'').next().unwrap_or("").trim().to_string()
        };
        let params = |ps: &[rapidr_ast::Parameter]| -> String {
            if ps.is_empty() {
                return String::new();
            }
            let list: Vec<String> = ps.iter().map(|x| format!("{}{}{} AS {}", if x.by_ref { "BYREF " } else { "" }, x.name, if x.is_array { "()" } else { "" }, rapidr_lang::resolve_component(&x.type_name).map_or(x.type_name.clone(), |k| k.spelling()))).collect();
            format!(" ({})", list.join(", "))
        };
        fn globals(stmts: &[rapidr_ast::Statement], f: &mut dyn FnMut(&rapidr_ast::Statement)) {
            use rapidr_ast::Statement as S;
            for st in stmts {
                f(st);
                match st {
                    S::If(i) => {
                        globals(&i.then_body, f);
                        for b in &i.elseif_branches {
                            globals(&b.body, f);
                        }
                        globals(&i.else_body, f);
                    }
                    S::For(x) => globals(&x.body, f),
                    S::While(x) => globals(&x.body, f),
                    S::DoLoop(x) => globals(&x.body, f),
                    S::SelectCase(c) => {
                        for b in &c.cases {
                            globals(&b.body, f);
                        }
                        globals(&c.case_else, f);
                    }
                    _ => {}
                }
            }
        }
        fn components(stmts: &[rapidr_ast::Statement], into: &mut Vec<String>) {
            for st in stmts {
                if let rapidr_ast::Statement::Create(c) = st {
                    into.push(format!("{} AS {}", c.name, rapidr_lang::resolve_component(&c.type_name).map_or(c.type_name.clone(), |k| k.spelling())));
                    components(&c.body, into);
                }
            }
        }
        globals(&snap.parsed.program.statements, &mut |st| {
            use rapidr_ast::Statement as S;
            match st {
                S::Dim(d) if !d.is_redim => {
                    for v in &d.declarators {
                        let written = source(v.span);
                        let what = if written.to_ascii_uppercase().starts_with(&v.name.to_ascii_uppercase()) && !written.is_empty() { written } else { format!("{} AS {}", v.name, d.type_name) };
                        place(v.span, format!("DIM {what}"), a);
                    }
                }
                S::Const(c) => place(c.span, source(c.span), a),
                S::Create(c) => {
                    let ty = rapidr_lang::resolve_component(&c.type_name).map_or(c.type_name.clone(), |k| k.spelling());
                    let mut inner = Vec::new();
                    components(&c.body, &mut inner);
                    let shown = inner.len().min(40);
                    let more = if inner.len() > shown { format!(", … {} more", inner.len() - shown) } else { String::new() };
                    let kind = if ty.eq_ignore_ascii_case("RForm") { "form" } else { "component" };
                    let list = if inner.is_empty() { String::new() } else { format!(": {}{more}", inner[..shown].join(", ")) };
                    place(c.span, format!("{kind} {} AS {ty}{list}", c.name), a);
                }
                S::Subroutine(r) => place(r.span, format!("SUB {}{}", r.name, params(&r.params)), a),
                S::Function(f) => place(f.span, format!("FUNCTION {}{}{}", f.name, params(&f.params), f.return_type.as_deref().map(|t| format!(" AS {t}")).unwrap_or_default()), a),
                S::Declare(d) if d.lib.is_some() => place(d.span, format!("DECLARE {} {}{} (in {})", if d.is_function { "FUNCTION" } else { "SUB" }, d.name, params(&d.params), d.lib.as_deref().unwrap_or("")), a),
                S::Type(t) => {
                    let fields: Vec<String> = t.fields.iter().map(|f| format!("{} AS {}", f.name, f.type_name)).collect();
                    place(t.span, format!("TYPE {}: {}", t.name, fields.join(", ")), a);
                }
                _ => {}
            }
        });
        out.push_str(&format!("The program {main}, as the compiler reads it — files in include order:\n"));
        for f in &order {
            outlined.push(f.clone());
            let by = included_by.get(f).map(|b| format!(" (included by {b})")).unwrap_or_default();
            out.push_str(&format!("\n{f}{by}\n"));
            for n in notes.get(f).into_iter().flatten() {
                out.push_str(&format!("  {n}\n"));
            }
            let mut ms = marks.remove(f).unwrap_or_default();
            ms.sort_by_key(|m| m.line);
            if ms.is_empty() && notes.get(f).is_none() {
                out.push_str("  (nothing declared: code only)\n");
            }
            for m in ms {
                out.push_str(&format!("  {}: {}\n", m.line, m.what));
            }
        }
    };
    program(&mut a, &main, &mut out, &mut outlined);
    // (a source file the main file doesn't include: not part of the
    // program — said, with its own names)
    let loose: Vec<String> = files.iter().map(|(r, _)| r.clone()).filter(|r| is_source(r) && !outlined.contains(r)).collect();
    for f in &loose {
        out.push_str(&format!("\n{f} is NOT included by {main}: its names aren't part of the program (add $INCLUDE \"{f}\" to {main} to use them). On its own:\n"));
        let mut o = String::new();
        let mut seen = Vec::new();
        program(&mut a, f, &mut o, &mut seen);
        out.push_str(o.split_once('\n').map_or("", |(_, rest)| rest));
    }
    let (text, cut) = clip(&out);
    let mut content = untrusted("project:outline", json!(text));
    if cut {
        content.push_str(&format!("\n(only its first {MAX_TEXT} characters)"));
    }
    Outcome { content, is_error: false, note: format!("Outlined the project ({} files)", outlined.len() + loose.len()), read: Some("project:outline".into()), risk: "read", target: main }
}

fn design(name: &str, input: &Json, p: &Project) -> Outcome {
    let files = p.all();
    if files.is_empty() {
        return err("no project is open", format!("{name}: no project open"), "");
    }
    let refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let model = rapidr_introspect::design::DesignModel::in_memory(&refs);
    let opt = |k: &str| input.get(k).and_then(Json::as_str).filter(|v| !v.trim().is_empty()).map(str::to_string);
    let (req, target) = if name == "rapidr_list" {
        (Request::List { scope: opt("scope"), type_name: opt("type"), limit: Some(500), cursor: None }, opt("scope").unwrap_or_default())
    } else {
        let props = input.get("props").and_then(Json::as_array).map(|a| a.iter().filter_map(Json::as_str).map(str::to_string).collect::<Vec<_>>());
        (Request::Get { id: opt("id").unwrap_or_default(), props }, opt("id").unwrap_or_default())
    };
    match rapidr_introspect::handle(Some(&model), &req) {
        Reply::Ok(v) => {
            let note = if name == "rapidr_list" { format!("Listed the forms' components ({})", v["total"].as_u64().unwrap_or(0)) } else { format!("Read {target}'s properties") };
            let mut o = ok(untrusted("project:design", v), note, "read", &target);
            o.read = Some("project:design".into());
            o
        }
        Reply::Err(e) => {
            let note = format!("{name} {target}: {}", e.message);
            err(e.message, note, &target)
        }
    }
}

/// A source line's statements (split at `:` outside strings, the comment
/// gone), each with the byte column it starts at.
fn statements(line: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '\'' if !quoted => {
                out.push((start, &line[start..i]));
                return out;
            }
            ':' if !quoted => {
                out.push((start, &line[start..i]));
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push((start, &line[start..]));
    out
}

/// The member of `c` most like `name` (ColumnCount → ColCount), when one is
/// close: the same start and end, most of the name.
fn nearest_member(c: &rapidr_lang::Component, name: &str) -> Option<String> {
    let want = name.to_ascii_lowercase();
    let score = |m: &str| -> f64 {
        let m = m.to_ascii_lowercase();
        let pre = want.chars().zip(m.chars()).take_while(|(a, b)| a == b).count();
        let suf = want.chars().rev().zip(m.chars().rev()).take_while(|(a, b)| a == b).count();
        let (pre, suf) = (pre.min(m.len()), suf.min(m.len().saturating_sub(pre)));
        if pre < 2 { 0.0 } else { (pre + suf) as f64 / want.len().max(m.len()) as f64 }
    };
    let names = c.properties.iter().map(|p| p.name).chain(c.events.iter().map(|e| e.name));
    names.map(|n| (score(n), n)).filter(|(s, _)| *s >= 0.5).max_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, n)| n.to_string())
}

/// A statement's assignment target (`Caption`, `List1.ColumnWidths`,
/// `.Width`, `Font.Size`, `Cell(1, 2)`): the text before a top-level `=`,
/// when it is one.
fn assigned(stmt: &str) -> Option<&str> {
    let mut depth = 0i32;
    let mut quoted = false;
    for (i, c) in stmt.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            '=' | '<' | '>' if !quoted && depth == 0 => {
                if c != '=' {
                    return None;
                }
                let lhs = stmt[..i].trim();
                let head = lhs.split(['(', ' ']).next().unwrap_or("");
                let ok = !head.is_empty() && head.chars().all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.' || ch == '$') && !lhs.contains(' ');
                return ok.then_some(lhs);
            }
            _ => {}
        }
    }
    None
}

/// Assignments to a property a component doesn't have — `ColumnWidths =
/// 250` in an RListBox's CREATE block, `List1.ColumnWidths = 250` — which
/// the compiler lets pass, as RapidQ's does (the program only warns when it
/// runs), but which do nothing: for the assistant, errors saying where the
/// component's properties are. Only components RapidR's registry knows
/// (a TYPE's own objects aren't checked); `checked`: the files reported.
pub fn member_problems(files: &[(String, String)], checked: &[&str]) -> Vec<String> {
    use rapidr_lang::Component;
    let words = |s: &str| s.split_whitespace().map(str::to_string).collect::<Vec<_>>();
    // (the components the project makes or declares, by name: every type a
    // name has somewhere — a name used for two components is checked
    // against both)
    let mut named: HashMap<String, Vec<&'static Component>> = HashMap::new();
    for (_, text) in files {
        for line in text.lines() {
            for (_, st) in statements(line) {
                let w = words(st);
                let kw = w.first().map(|k| k.to_ascii_uppercase()).unwrap_or_default();
                if (kw == "CREATE" || kw == "DIM") && w.len() >= 4 && w[2].eq_ignore_ascii_case("AS") {
                    let name = w[1].trim_end_matches(',').to_ascii_lowercase();
                    match rapidr_lang::resolve_component(w[3].trim_end_matches(',')).filter(|c| c.kind == rapidr_lang::Kind::Component) {
                        Some(c) => named.entry(name).or_default().push(c),
                        // (a name that is also something else: not checked)
                        None => {
                            named.insert(name.clone(), Vec::new());
                        }
                    }
                }
            }
        }
    }
    // (what visual components take — Align, PopupMenu, Left, SelText … — is
    // taken from any (three or more have it): the kernel treats them alike, whatever one
    // registry entry lists)
    let visual: Vec<&'static Component> = rapidr_lang::COMPONENTS.iter().filter(|c| c.kind == rapidr_lang::Kind::Component && c.visual).collect();
    let common = |prop: &str| visual.iter().filter(|c| c.property(prop).is_some()).count() >= 3;
    let has = |c: &Component, prop: &str| c.has_member(prop) || (c.visual && common(prop));
    let mut out = Vec::new();
    for (path, text) in files.iter().filter(|(p, _)| checked.contains(&p.as_str())) {
        let mut creates: Vec<Option<&'static Component>> = Vec::new();
        let mut withs: Vec<Option<&'static Component>> = Vec::new();
        let mut in_type = false;
        for (ln, line) in text.lines().enumerate() {
            for (col, st) in statements(line) {
                let w = words(st);
                let Some(first) = w.first().map(|k| k.to_ascii_uppercase()) else { continue };
                let second = w.get(1).map(|k| k.to_ascii_uppercase()).unwrap_or_default();
                match (first.as_str(), second.as_str()) {
                    ("TYPE", _) => in_type = true,
                    ("END", "TYPE") => in_type = false,
                    _ if in_type => {}
                    ("CREATE", _) => creates.push(w.get(3).and_then(|t| rapidr_lang::resolve_component(t)).filter(|c| c.kind == rapidr_lang::Kind::Component)),
                    ("END", "CREATE") => {
                        creates.pop();
                    }
                    ("WITH", _) => withs.push(named.get(&second.to_ascii_lowercase()).filter(|t| t.len() == 1).map(|t| t[0])),
                    ("END", "WITH") => {
                        withs.pop();
                    }
                    _ => {
                        let Some(lhs) = assigned(st) else { continue };
                        let head = lhs.split('(').next().unwrap_or("");
                        let parts: Vec<&str> = head.split('.').collect();
                        let (owners, prop, owner_name): (Vec<&'static Component>, &str, String) = if let Some(rest) = head.strip_prefix('.') {
                            (withs.last().copied().flatten().into_iter().collect(), rest.split('.').next().unwrap_or(""), String::new())
                        } else if parts.len() == 1 {
                            (creates.last().copied().flatten().into_iter().collect(), parts[0], String::new())
                        } else if let Some(types) = named.get(&parts[0].to_ascii_lowercase()) {
                            (types.clone(), parts[1], format!("{}.", parts[0]))
                        } else {
                            (Vec::new(), "", String::new())
                        };
                        let Some(&c) = owners.first() else { continue };
                        if prop.is_empty() || owners.iter().any(|o| has(o, prop)) {
                            continue;
                        }
                        let at = st.find(prop).map_or(col, |k| col + k) + 1;
                        let ty = c.spelling();
                        let near = nearest_member(c, prop).map(|m| format!("; did you mean {m}?")).unwrap_or_default();
                        out.push(format!("error {path}:{}:{at}: {ty} has no property {owner_name}{prop}{near} (the compiler lets it pass, but it does nothing when the program runs; rapidr_describe {ty} lists its properties)", ln + 1));
                    }
                }
            }
        }
    }
    out
}

/// The compiler's diagnostics of `files` (path, text) for `target` (and,
/// when it is an include, for the main file that includes it).
fn diagnostics(files: &[(String, String)], root: &str, target: &str, main: &str, compat: bool) -> Vec<String> {
    let mut a = Analysis::new(Options { rapidq_compatible: compat, roots: Some(Vec::new()), ..Options::default() });
    let full = |rel: &str| PathBuf::from(format!("{}/{rel}", root.trim_end_matches('/')));
    for (rel, text) in files {
        a.update(full(rel), text.clone());
    }
    let mut out = Vec::new();
    let mut checked: Vec<&str> = vec![target];
    if !main.is_empty() && main != target {
        checked.push(main);
    }
    let rel = |p: &std::path::Path| {
        let file = p.to_string_lossy().replace('\\', "/");
        file.strip_prefix(&format!("{}/", root.trim_end_matches('/'))).unwrap_or(&file).to_string()
    };
    let push = |out: &mut Vec<String>, l: String| {
        if !out.contains(&l) {
            out.push(l);
        }
    };
    for f in &checked {
        for d in a.diagnostics(&full(f)) {
            let text = a.text(&d.file).unwrap_or_default();
            let (line, col) = crate::langsvc::line_col(&text, d.start);
            let file = rel(&d.file);
            // (a RapidQ include's constant without its include is 0 — alClient
            // is alNone, a layout that silently isn't: an error, for the
            // assistant)
            let sev = if d.code.as_deref() == Some("needs-include") { "error".to_string() } else { format!("{:?}", d.severity).to_ascii_lowercase() };
            let l = format!("{sev} {file}:{line}:{col}: {}", d.message);
            if out.contains(&l) {
                continue;
            }
            out.push(l);
            // (the habits from other languages on the line, and what the
            // message means: the exact fix, under the compiler's words)
            if sev == "error" {
                let src = text.lines().nth(line.saturating_sub(1)).unwrap_or("");
                for (_, h) in crate::friendly::for_message(&d.message).into_iter().chain(crate::friendly::for_line(src)) {
                    push(&mut out, format!("hint {file}:{line}: {h}"));
                }
            }
        }
    }
    // (the habits that never compile, on the lines the compiler didn't
    // reach: it stops at its first syntax error)
    for f in &checked {
        let Some((_, text)) = files.iter().find(|(p, _)| p.as_str() == *f) else { continue };
        for (k, line) in text.lines().enumerate() {
            for (id, h) in crate::friendly::for_line(line) {
                if crate::friendly::NEVER_BASIC.contains(&id) && !out.contains(&format!("hint {f}:{}: {h}", k + 1)) {
                    push(&mut out, format!("error {f}:{}:1: {h}", k + 1));
                }
            }
        }
    }
    // (properties no component has: the compiler takes them, they do
    // nothing — said once, in place of the compiler's own warning about the
    // same member when it gave one; with what the component has, once)
    let sources: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).filter(|p| is_source(p)).collect();
    let mut described: Vec<String> = Vec::new();
    for l in member_problems(files, &sources) {
        let place = l.split(": ").next().unwrap_or("").trim_start_matches("error ").rsplit_once(':').map_or("", |x| x.0).to_string();
        let prop = l.split(" has no property ").nth(1).and_then(|r| r.split([';', ' ']).next()).map(|p| p.rsplit('.').next().unwrap_or(p).to_ascii_uppercase()).unwrap_or_default();
        if let Some(k) = out.iter().position(|o| o.starts_with(&format!("warning {place}:")) && o.contains(&format!("Member {prop} not part of"))) {
            out[k] = l.clone();
        } else {
            push(&mut out, l.clone());
        }
        let ty = l.split(": ").nth(1).and_then(|r| r.split(" has no property").next()).unwrap_or("").to_string();
        if let Some(c) = rapidr_lang::resolve_component(&ty).filter(|_| !described.contains(&ty)) {
            described.push(ty);
            push(&mut out, format!("hint {place}: {}", crate::friendly::members(c)));
        }
    }
    // (a method the compiler doesn't know on a component: what it has)
    let unknown_members: Vec<(String, String)> = out
        .iter()
        .filter_map(|o| {
            let (head, msg) = o.split_once(": ")?;
            let ty = msg.strip_prefix("Member ")?.split(" not part of ").nth(1)?.split_whitespace().next()?.trim_end_matches(['.', ',']).to_string();
            Some((head.split_whitespace().nth(1)?.rsplit_once(':')?.0.to_string(), ty))
        })
        .collect();
    for (place, ty) in unknown_members {
        if let Some(c) = rapidr_lang::resolve_component(&ty).filter(|c| !described.contains(&c.spelling())) {
            described.push(c.spelling());
            push(&mut out, format!("hint {place}: {}", crate::friendly::members(c)));
        }
    }
    // (names never given a value: RapidQ makes each a new variable, 0 or
    // "" — `IF ButtonPressed = "7"` is never true)
    for f in &checked {
        for l in undefined_names(&mut a, &full(f), &rel) {
            push(&mut out, l);
        }
    }
    // (what only the whole file shows: the window never shown, handlers
    // that do nothing yet)
    for f in &checked {
        let Some((_, text)) = files.iter().find(|(p, _)| p.as_str() == *f) else { continue };
        for x in crate::friendly::program(text, *f == main || main.is_empty()) {
            push(&mut out, format!("{} {f}:{}:1: {}", if x.error { "error" } else { "warning" }, x.line, x.text));
        }
    }
    // (the forms as a person would see them: cut off, overlapping, a
    // caption too long — once the program compiles, it's what is left)
    if !out.iter().any(|l| l.starts_with("error")) {
        for l in layout_problems(files) {
            push(&mut out, l);
        }
    }
    out
}

/// What a person would see wrong in the forms the source makes, before
/// running it (docs/small-models-plan.md §2, `check_layout`): a control
/// past its form's client area (cut off), controls on top of each other, a
/// button's caption longer than the button. Warnings, with the numbers; a
/// control laid out by Align, and labels (which size to their text), aren't
/// judged.
pub fn layout_problems(files: &[(String, String)]) -> Vec<String> {
    let srcs: Vec<(&str, &str)> = files.iter().filter(|(p, _)| is_source(p)).map(|(a, b)| (a.as_str(), b.as_str())).collect();
    if srcs.is_empty() {
        return Vec::new();
    }
    let model = rapidr_introspect::design::DesignModel::in_memory(&srcs);
    let Reply::Ok(list) = rapidr_introspect::handle(Some(&model), &Request::List { scope: None, type_name: None, limit: Some(1000), cursor: None }) else { return Vec::new() };
    struct Box_ {
        id: String,
        ty: String,
        parent: String,
        x: i64,
        y: i64,
        w: i64,
        h: i64,
        aligned: bool,
        caption: String,
        client: Option<(i64, i64)>,
        visual: bool,
    }
    let mut boxes: Vec<Box_> = Vec::new();
    for o in list["objects"].as_array().into_iter().flatten() {
        let id = o["id"].as_str().unwrap_or("").to_string();
        let ty = o["type"].as_str().unwrap_or("").to_string();
        let comp = rapidr_lang::resolve_component(&ty);
        let Reply::Ok(g) = rapidr_introspect::handle(Some(&model), &Request::Get { id: id.clone(), props: None }) else { continue };
        let mut set: HashMap<String, Json> = HashMap::new();
        for v in g["values"].as_array().into_iter().flatten() {
            if v["source"] == "set" || v["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case("caption")) {
                set.insert(v["name"].as_str().unwrap_or("").to_ascii_lowercase(), v["value"].clone());
            }
        }
        let num = |k: &str| set.get(k).and_then(Json::as_i64);
        let (dw, dh) = comp.and_then(|c| c.size).map_or((0, 0), |(w, h)| (i64::from(w), i64::from(h)));
        let align = set.get("align").map(|a| a.as_str().map_or_else(|| a.to_string(), str::to_string)).unwrap_or_default();
        let client = match (num("clientwidth"), num("clientheight"), num("width"), num("height")) {
            (Some(w), Some(h), _, _) => Some((w, h)),
            (_, _, Some(w), Some(h)) => Some((w - 16, h - 39)),
            _ => None,
        };
        boxes.push(Box_ {
            id,
            ty: ty.to_ascii_uppercase(),
            parent: o["parent"].as_str().unwrap_or("").to_string(),
            x: num("left").unwrap_or(0),
            y: num("top").unwrap_or(0),
            w: num("width").unwrap_or(dw),
            h: num("height").unwrap_or(dh),
            aligned: !(align.is_empty() || align.eq_ignore_ascii_case("alNone") || align == "0"),
            caption: set.get("caption").and_then(Json::as_str).unwrap_or("").replace('&', ""),
            client,
            visual: comp.is_some_and(|c| c.visual),
        });
    }
    let mut out = Vec::new();
    let judged = |b: &Box_| b.visual && !b.aligned && !b.ty.ends_with("FORM") && b.w > 0 && b.h > 0;
    for f in boxes.iter().filter(|b| b.ty.ends_with("FORM")) {
        let Some((cw, ch)) = f.client else { continue };
        for b in boxes.iter().filter(|b| b.parent == f.id && judged(b)) {
            if b.x + b.w > cw + 2 || b.y + b.h > ch + 2 {
                out.push(format!("warning layout: {} (Left {} + Width {}, Top {} + Height {}) reaches past {}'s client area ({cw} × {ch}): it's cut off — make the form bigger (ClientWidth / ClientHeight) or move it", b.id, b.x, b.w, b.y, b.h, f.id));
            }
        }
    }
    let solid = |b: &Box_| judged(b) && !b.ty.ends_with("LABEL") && !b.ty.ends_with("PANEL") && !b.ty.ends_with("GROUPBOX") && !b.ty.ends_with("TIMER");
    let all: Vec<&Box_> = boxes.iter().filter(|b| solid(b)).collect();
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            if a.parent == b.parent && a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h {
                out.push(format!("warning layout: {} ({},{} {}×{}) and {} ({},{} {}×{}) overlap: move one", a.id, a.x, a.y, a.w, a.h, b.id, b.x, b.y, b.w, b.h));
            }
        }
    }
    for b in boxes.iter().filter(|b| judged(b) && b.ty.ends_with("BUTTON") && !b.caption.is_empty()) {
        let need = b.caption.chars().count() as i64 * 7 + 12;
        if need > b.w + 4 {
            out.push(format!("warning layout: {}'s caption \"{}\" needs about {need} pixels, its Width is {}: make it wider or the caption shorter", b.id, b.caption, b.w));
        }
    }
    out
}

/// The names the program reads but never gives a value, nor declares, in
/// `file`: errors with the nearest name it has (a variable, a component, a
/// routine, a builtin).
fn undefined_names(a: &mut Analysis, file: &std::path::Path, rel: &dyn Fn(&std::path::Path) -> String) -> Vec<String> {
    use rapidr_langsvc::model::{Access, SymbolKind};
    let Some(s) = a.snapshot(file) else { return Vec::new() };
    let m = &s.model;
    let written: std::collections::HashSet<usize> = m.references.iter().filter(|r| r.access != Access::Read).map(|r| r.symbol).collect();
    let known: Vec<&str> = m.symbols.iter().filter(|x| !x.implicit).map(|x| x.name.as_str()).collect();
    let mut out = Vec::new();
    let mut said: Vec<usize> = Vec::new();
    for r in &m.references {
        let sym = &m.symbols[r.symbol];
        if !sym.implicit || !matches!(sym.kind, SymbolKind::Global | SymbolKind::Local | SymbolKind::Static) || written.contains(&r.symbol) || said.contains(&r.symbol) {
            continue;
        }
        let base = sym.name.trim_end_matches(['$', '%', '#', '&', '!']);
        // (a RapidQ include's constant: the include is what's missing, said
        // by its own message)
        if rapidr_lang::constant(base).is_some() || base.eq_ignore_ascii_case("this") {
            continue;
        }
        let Some(at) = s.locate(r.span) else { continue };
        if at.file != file {
            continue;
        }
        said.push(r.symbol);
        let text = a.text(&at.file).unwrap_or_default();
        let (line, col) = crate::langsvc::line_col(&text, at.start);
        let near = crate::friendly::closest(base, known.iter().copied()).map(|n| format!("; did you mean {n}?")).unwrap_or_default();
        out.push(format!("error {}:{line}:{col}: {} is never given a value nor declared: RapidR makes it a new variable, always 0 or \"\" here{near} (declare it with DIM and set it, or use the right name)", rel(&at.file), sym.name));
    }
    out
}

fn check(input: &Json, p: &Project) -> Outcome {
    let mut files = p.all();
    let target = match s(input, "path") {
        "" => p.main.to_string(),
        path => match project_path(p.folder, path) {
            Ok(r) => r,
            Err(e) => return err(e, "check_code: outside the project".into(), path),
        },
    };
    if target.is_empty() {
        return err("no file to check: give `path` (and `text`)", "check_code: nothing to check".into(), "");
    }
    if let Some(text) = input.get("text").and_then(Json::as_str) {
        files.retain(|(f, _)| *f != target);
        files.push((target.clone(), text.to_string()));
    }
    if !files.iter().any(|(f, _)| *f == target) {
        return err(format!("{target}: no such file in the project"), format!("check_code: {target} not found"), &target);
    }
    let found = diagnostics(&files, "/project", &target, p.main, p.compat.eq_ignore_ascii_case("rapidq"));
    let errors = found.iter().filter(|l| l.starts_with("error")).count();
    if found.is_empty() {
        ok("No problems: it compiles.".into(), format!("Checked {target}: no problems"), "read", &target)
    } else {
        ok(found.join("\n"), format!("Checked {target}: {errors} errors, {} warnings", found.len() - errors), "read", &target)
    }
}

/// `old` (the file `path`) with each edit applied. An old text that is in
/// the file once but for its lines' indentation and trailing spaces fits
/// those lines (a small model rarely copies indentation exactly); one that
/// isn't there is refused with the file's lines closest to it, numbered,
/// for the model to copy from; one that is there more than once, saying so.
fn apply_edits(path: &str, old: &str, edits: &[Json]) -> Result<String, String> {
    let mut text = old.to_string();
    if edits.is_empty() {
        return Err("no edits given".into());
    }
    for (i, e) in edits.iter().enumerate() {
        let from = s(e, "old_text");
        let to = s(e, "new_text");
        if from.is_empty() {
            return Err(format!("edit {}: old_text is empty (to add a file, use write_file)", i + 1));
        }
        // (the file's line breaks: an edit written with \n fits a CR LF file)
        let crlf = text.contains("\r\n");
        let (from, to) = if crlf && !from.contains("\r\n") { (from.replace('\n', "\r\n"), to.replace("\r\n", "\n").replace('\n', "\r\n")) } else { (from.to_string(), to.to_string()) };
        match text.matches(&from).count() {
            1 => text = text.replacen(&from, &to, 1),
            0 => match loose_match(&text, &from) {
                Some((start, end, indent)) => {
                    // (the range ends before the last line's break, which stays)
                    let to = if from.ends_with('\n') { to.strip_suffix('\n').map_or(to.as_str(), |t| t.strip_suffix('\r').unwrap_or(t)) } else { to.as_str() };
                    text.replace_range(start..end, &reindent(to, &indent));
                }
                None => {
                    return Err(format!(
                        "edit {}: old_text isn't in {path} (it must match the file exactly, spaces and line breaks too). The lines of {path} closest to it (numbered here; the numbers aren't in the file):\n{}\nCopy old_text exactly from these lines (without the numbers) and call propose_edit again.",
                        i + 1,
                        closest_lines(&text, &from)
                    ))
                }
            },
            n => return Err(format!("edit {}: old_text is in the file {n} times: include more lines around it so it is once", i + 1)),
        }
    }
    Ok(text)
}

/// Each line's indentation: in the old text, in the file.
type Indents = Vec<(String, String)>;

/// Where `from`'s lines are in `text` once, each line compared without its
/// indentation and trailing spaces: the byte range of those whole lines
/// (without the last line break), and each line's indentation in `from`
/// and in the file.
fn loose_match(text: &str, from: &str) -> Option<(usize, usize, Indents)> {
    let want: Vec<&str> = from.lines().collect();
    let first = want.iter().position(|l| !l.trim().is_empty())?;
    let last = want.iter().rposition(|l| !l.trim().is_empty())?;
    let want = &want[first..=last];
    // (each line of the text: its start, its end without the break)
    let mut lines: Vec<(usize, usize)> = Vec::new();
    let mut at = 0;
    for piece in text.split_inclusive('\n') {
        let body = piece.trim_end_matches(['\n', '\r']);
        lines.push((at, at + body.len()));
        at += piece.len();
    }
    let line = |k: usize| &text[lines[k].0..lines[k].1];
    let mut found = None;
    for k in 0..lines.len() {
        if k + want.len() > lines.len() {
            break;
        }
        if want.iter().enumerate().all(|(j, w)| line(k + j).trim() == w.trim()) {
            if found.is_some() {
                return None;
            }
            found = Some(k);
        }
    }
    let k = found?;
    let lead = |s: &str| s[..s.len() - s.trim_start().len()].to_string();
    let indents = want.iter().enumerate().map(|(j, w)| (lead(w), lead(line(k + j)))).collect();
    Some((lines[k].0, lines[k + want.len() - 1].1, indents))
}

/// `to` at the file's indentation: shifted when the old text's lines were
/// all off by the same indentation (more or less); else its first line
/// given the file's, the rest as written.
fn reindent(to: &str, indents: &[(String, String)]) -> String {
    let more: Option<Vec<&str>> = indents.iter().map(|(t, f)| f.strip_suffix(t.as_str())).collect();
    let less: Option<Vec<&str>> = indents.iter().map(|(t, f)| t.strip_suffix(f.as_str())).collect();
    let same = |v: &[&str]| v.windows(2).all(|w| w[0] == w[1]);
    let lines: Vec<&str> = to.split('\n').collect();
    let shifted: Vec<String> = match (more, less) {
        (Some(m), _) if same(&m) => lines.iter().map(|l| if l.trim().is_empty() { l.to_string() } else { format!("{}{l}", m[0]) }).collect(),
        (_, Some(x)) if same(&x) => lines.iter().map(|l| l.strip_prefix(x[0]).unwrap_or(l).to_string()).collect(),
        _ => lines
            .iter()
            .enumerate()
            .map(|(j, l)| match indents.first() {
                Some((_, f)) if j == 0 && !l.starts_with([' ', '\t']) => format!("{f}{l}"),
                _ => l.to_string(),
            })
            .collect(),
    };
    shifted.join("\n")
}

/// The ≤ 30 lines of `text` most like `from`, numbered from 1.
fn closest_lines(text: &str, from: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return "(the file is empty)".into();
    }
    let words = |t: &str| -> Vec<String> { t.to_lowercase().split(|c: char| !(c.is_alphanumeric() || c == '$' || c == '_')).filter(|w| !w.is_empty()).map(str::to_string).collect() };
    let want: Vec<(String, Vec<String>)> = from.lines().map(str::trim).filter(|l| !l.is_empty()).map(|l| (l.to_string(), words(l))).collect();
    let have: Vec<(String, Vec<String>)> = lines.iter().map(|l| (l.trim().to_string(), words(l))).collect();
    let sim = |a: &(String, Vec<String>), b: &(String, Vec<String>)| -> f64 {
        if a.0 == b.0 {
            return 2.0;
        }
        if a.1.is_empty() || b.1.is_empty() {
            return 0.0;
        }
        b.1.iter().filter(|w| a.1.contains(w)).count() as f64 / a.1.len().max(b.1.len()) as f64
    };
    let mut best = (0usize, 0.0f64);
    for k in 0..have.len() {
        let score: f64 = want.iter().enumerate().map(|(j, w)| have.get(k + j).map_or(0.0, |h| sim(h, w))).sum();
        if score > best.1 {
            best = (k, score);
        }
    }
    let span = want.len().clamp(1, 24);
    let start = best.0.saturating_sub(3);
    let end = (best.0 + span + 3).min(lines.len()).min(start + 30);
    lines[start..end].iter().enumerate().map(|(k, l)| format!("{:>4}| {l}", start + k + 1)).collect::<Vec<_>>().join("\n")
}

/// A proposal for `rel` replaces the earlier one for that file (this
/// turn's, or one still waiting for the user): the file as it was, and
/// whether it is a new one, stay the earlier's.
fn propose(proposals: &mut Vec<Proposal>, p: Proposal) {
    if let Some(k) = proposals.iter().position(|q| q.path == p.path && q.project == p.project) {
        let old = proposals[k].old.clone();
        let kind = proposals[k].kind.clone();
        proposals[k] = Proposal { old, kind, ..p };
    } else {
        proposals.push(p);
    }
}

fn base_text(rel: &str, p: &Project, proposals: &[Proposal]) -> Option<String> {
    // (a second change to a file this turn builds on the first)
    proposals.iter().find(|q| q.path == rel && q.project.is_empty()).map(|q| q.new.clone()).or_else(|| p.text(rel).ok())
}

fn propose_edit(input: &Json, p: &Project, proposals: &mut Vec<Proposal>) -> Outcome {
    let rel = match project_path(p.folder, s(input, "path")) {
        Ok(r) => r,
        Err(e) => return err(e, format!("Refused a change to {}: outside the project", s(input, "path")), s(input, "path")),
    };
    let Some(current) = base_text(&rel, p, proposals) else {
        return err(format!("{rel}: no such file (to make a new file, use write_file)"), format!("propose_edit: {rel} not found"), &rel);
    };
    let edits = input.get("edits").and_then(Json::as_array).cloned().unwrap_or_default();
    match apply_edits(&rel, &current, &edits) {
        Ok(new) => {
            if let Some((why, short)) = refused(p, proposals, &rel, &current, &new) {
                return err(why, format!("Refused a change to {rel}: {short}"), &rel);
            }
            let original = p.text(&rel).unwrap_or_default();
            let summary = s(input, "summary").to_string();
            propose(proposals, Proposal { kind: "edit".into(), path: rel.clone(), old: original, new, summary: summary.clone(), ..Proposal::default() });
            match compile_errors(p, proposals) {
                None => ok(format!("Proposed to the user as a diff of {rel}; they decide whether to apply it. It compiles. Don't propose it again: tell them in a sentence what it does."), format!("Proposed a change to {rel}: {summary}"), "state", &rel),
                Some(errors) => ok(format!("Proposed to the user as a diff of {rel}, but the project doesn't compile with it yet — fix these now with propose_edit on the lines in error (it builds on this proposal):\n{errors}"), format!("Proposed a change to {rel}: {summary} (errors to fix)"), "state", &rel),
            }
        }
        Err(e) => err(e, format!("A change to {rel} didn't fit the file"), &rel),
    }
}

/// The compiler's errors for the project as the turn's proposals would
/// leave it (each changed file's proposed text), when it has some: what a
/// proposing tool tells the model at once, so it fixes them without having
/// to remember check_code.
fn compile_errors(p: &Project, proposals: &[Proposal]) -> Option<String> {
    let files = project_files(p, proposals);
    let changed: Vec<&Proposal> = proposals.iter().filter(|q| q.project.is_empty() && is_source(&q.path)).collect();
    let mut errors: Vec<String> = Vec::new();
    for q in &changed {
        // (each error with the hints that say its fix)
        for l in diagnostics(&files, "/project", &q.path, p.main, p.compat.eq_ignore_ascii_case("rapidq")) {
            if (l.starts_with("error") || l.starts_with("hint")) && !errors.contains(&l) {
                errors.push(l);
            }
        }
    }
    (errors.iter().any(|l| l.starts_with("error"))).then(|| errors.iter().take(24).cloned().collect::<Vec<_>>().join("\n"))
}

fn write_file(input: &Json, p: &Project, proposals: &mut Vec<Proposal>) -> Outcome {
    let rel = match project_path(p.folder, s(input, "path")) {
        Ok(r) => r,
        Err(e) if s(input, "path").trim().is_empty() => return err(format!("{e} (write_file takes path, text and summary)"), "write_file: no file named".into(), ""),
        Err(e) => return err(e, format!("Refused to write {}: outside the project", s(input, "path")), s(input, "path")),
    };
    if p.folder.is_empty() {
        return err("no project is open: use create_project for a new program", "write_file: no project open".into(), &rel);
    }
    let text = input.get("text").and_then(Json::as_str).unwrap_or("").to_string();
    let exists = p.exists(&rel);
    let original = if exists { p.text(&rel).unwrap_or_default() } else { String::new() };
    let summary = s(input, "summary").to_string();
    let kind = if exists { "edit" } else { "create" };
    // (cut off, or a syntax error: refused, said first)
    if let Some((why, short)) = refused(p, proposals, &rel, &original, &text) {
        return err(why, format!("Refused {rel}: {short}"), &rel);
    }
    // (a whole file sent for a few lines' change: refused — the changed
    // parts go with propose_edit; a model whose edits keep missing may
    // rewrite it: Studio says so in `_rewrite_ok`)
    if exists && !is_new_project(p) && input.get("_rewrite_ok").and_then(Json::as_bool) != Some(true) {
        if let Some(why) = needless_rewrite(&rel, &original, &text) {
            return err(why, format!("Refused a whole-file rewrite of {rel}: only a few lines change"), &rel);
        }
    }
    // (a file cut to a fraction of itself that doesn't compile is a
    // snippet sent for the whole: refused, the file left as proposed so far)
    let (old_lines, new_lines) = (original.lines().count(), text.lines().count());
    if exists && old_lines >= 20 && new_lines * 4 < old_lines {
        let mut trial = proposals.clone();
        propose(&mut trial, Proposal { kind: kind.into(), path: rel.clone(), old: original.clone(), new: text.clone(), ..Proposal::default() });
        if let Some(errors) = compile_errors(p, &trial) {
            return err(format!("write_file would replace {rel} ({old_lines} lines) with {new_lines} lines that don't compile: write_file takes the file's whole new text. Send the complete file, or change a part with propose_edit.\n{errors}"), format!("Refused a shortened {rel} that doesn't compile"), &rel);
        }
    }
    propose(proposals, Proposal { kind: kind.into(), path: rel.clone(), old: original, new: text, summary: summary.clone(), ..Proposal::default() });
    let what = if exists { "a new text for" } else { "the new file" };
    match compile_errors(p, proposals) {
        None => ok(format!("Proposed {what} {rel} to the user as a diff; they decide. It compiles. Don't propose it again: tell them in a sentence what it does."), format!("Proposed {what} {rel}: {summary}"), "state", &rel),
        Some(errors) => ok(format!("Proposed {what} {rel} to the user as a diff, but the project doesn't compile with it yet — fix these now with propose_edit on the lines in error (it builds on this proposal):\n{errors}"), format!("Proposed {what} {rel}: {summary} (errors to fix)"), "state", &rel),
    }
}

/// A new project's folder name, from the title the model gave: a name
/// that is one already (letters, digits, - and _) stays as it is; any
/// other title becomes one — accents dropped, its words joined, each
/// capitalised ("Scientific Calculator" → `ScientificCalculator`,
/// "calculadora científica" → `CalculadoraCientifica`). Never fails: a
/// title is never a reason to refuse a program (Robert's walk, 2026-10-18).
pub fn project_name(name: &str) -> String {
    let n = name.trim();
    if !n.is_empty() && n.len() <= 60 && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') && !n.starts_with(['-', '_']) {
        return n.to_string();
    }
    let plain: String = n.chars().flat_map(unaccent).collect();
    let mut out = String::new();
    for word in plain.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()) {
        let mut cs = word.chars();
        if let Some(first) = cs.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(cs);
        }
    }
    let out: String = out.chars().take(60).collect();
    if out.is_empty() { "NewProgram".into() } else { out }
}

/// A letter without its accent (Latin letters; others as they are).
fn unaccent(c: char) -> Vec<char> {
    let s = match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => "a",
        'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' | 'Ā' => "A",
        'é' | 'è' | 'ê' | 'ë' | 'ē' => "e",
        'É' | 'È' | 'Ê' | 'Ë' | 'Ē' => "E",
        'í' | 'ì' | 'î' | 'ï' | 'ī' => "i",
        'Í' | 'Ì' | 'Î' | 'Ï' | 'Ī' => "I",
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'ō' => "o",
        'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' | 'Ø' | 'Ō' => "O",
        'ú' | 'ù' | 'û' | 'ü' | 'ū' => "u",
        'Ú' | 'Ù' | 'Û' | 'Ü' | 'Ū' => "U",
        'ñ' => "n",
        'Ñ' => "N",
        'ç' => "c",
        'Ç' => "C",
        'ý' | 'ÿ' => "y",
        'Ý' => "Y",
        'ß' => "ss",
        'æ' => "ae",
        'Æ' => "Ae",
        'œ' => "oe",
        'Œ' => "Oe",
        _ => return vec![c],
    };
    s.chars().collect()
}

/// A name a folder can have, made from `text` (a file's name, a title):
/// letters, digits, - and _.
fn folder_name(text: &str) -> String {
    let stem = text.rsplit(['/', '\\']).next().unwrap_or(text);
    let stem = stem.rsplit_once('.').map_or(stem, |(a, _)| a);
    let n: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let n = n.trim_matches(['_', '-']).chars().take(60).collect::<String>();
    if n.is_empty() { "NewProgram".into() } else { n }
}

fn create_project(input: &Json, proposals: &mut Vec<Proposal>) -> Outcome {
    let files = input.get("files").and_then(Json::as_array).cloned().unwrap_or_default();
    // (a small model leaves out the name or the main file: what the files
    // say is taken — the main file is the first program, the name its own)
    let first_program = files.iter().map(|f| s(f, "path")).find(|p| is_source(p) && !p.to_ascii_lowercase().ends_with(".inc")).unwrap_or("").to_string();
    let main_given = s(input, "main").trim().to_string();
    let main_given = if main_given.is_empty() || !files.iter().any(|f| project_path("", s(f, "path")).ok() == project_path("", &main_given).ok()) { first_program.clone() } else { main_given };
    let name_given = s(input, "name").trim();
    let title = if name_given.is_empty() { folder_name(&main_given) } else { name_given.chars().take(80).collect::<String>() };
    // (any title: its folder's name made from it)
    let name = project_name(&title);
    if files.is_empty() || files.len() > MAX_FILES {
        return err(format!("a project has 1 to {MAX_FILES} files"), format!("create_project {name}: no files"), &name);
    }
    let mut got: Vec<(String, String)> = Vec::new();
    for f in &files {
        let rel = match project_path("", s(f, "path")) {
            Ok(r) => r,
            Err(e) => return err(e, format!("create_project {name}: a path outside it"), &name),
        };
        if got.iter().any(|(p, _)| *p == rel) {
            return err(format!("{rel} is given twice"), format!("create_project {name}: a file given twice"), &name);
        }
        // (a file cut off at the end: refused, nothing proposed)
        if let Some(at) = cut_off("", s(f, "text")) {
            return err(format!("{rel}'s text is cut off: {at}. Nothing was made. Call create_project again with every file complete."), format!("Refused the new program {title}: {rel} is cut off"), &name);
        }
        got.push((rel, s(f, "text").to_string()));
    }
    let main = project_path("", &main_given).unwrap_or_default();
    if !got.iter().any(|(p, _)| *p == main) {
        return err(format!("the main file \"{}\" isn't among the files", s(input, "main")), format!("create_project {name}: no main file"), &name);
    }
    // (checked by the compiler as it would build: the errors go back to
    // the model, which fixes them)
    let found = diagnostics(&got, &format!("/new/{name}"), &main, &main, false);
    let errors: Vec<&String> = found.iter().filter(|l| l.starts_with("error")).collect();
    proposals.retain(|p| p.project.is_empty());
    let summary = s(input, "summary").to_string();
    for (rel, text) in &got {
        proposals.push(Proposal { kind: "project".into(), path: rel.clone(), old: String::new(), new: text.clone(), summary: summary.clone(), project: name.clone(), title: title.clone(), main: *rel == main, decision: String::new() });
    }
    let n = got.len();
    if errors.is_empty() {
        let warn = if found.is_empty() { String::new() } else { format!(" Warnings:\n{}", found.join("\n")) };
        let files = if n == 1 { "1 file".to_string() } else { format!("{n} files") };
        ok(format!("The project {name} ({files}) compiles; Studio shows it to the user.{warn} Tell them in a sentence or two what it does and how to use it."), format!("Wrote the new program {title} ({files}): it compiles"), "state", &name)
    } else {
        let e = errors.len();
        ok(format!("The project {name} doesn't compile yet — fix these and call create_project again with the whole project:\n{}", found.join("\n")), format!("Wrote the new program {title}: {e} {} to fix", if e == 1 { "error" } else { "errors" }), "state", &name)
    }
}

/// How many errors the new project proposed and not created yet has.
pub fn project_errors(proposals: &[Proposal]) -> usize {
    let Some((name, main)) = pending(proposals) else { return 0 };
    let got: Vec<(String, String)> = proposals.iter().filter(|p| p.kind == "project" && p.project == name && p.decision != "created").map(|p| (p.path.clone(), p.new.clone())).collect();
    diagnostics(&got, &format!("/new/{name}"), &main, &main, false).iter().filter(|l| l.starts_with("error")).count()
}

/// How many errors the project has with the changes proposed.
pub fn edit_errors(p: &Project, proposals: &[Proposal]) -> usize {
    compile_errors(p, proposals).map_or(0, |e| e.lines().filter(|l| l.starts_with("error")).count())
}

/// The lines a change adds and removes: the lines of each text not in
/// their longest common run of lines (a change at the top and one at the
/// bottom of a long file are two lines, not the file).
pub fn line_delta(old: &str, new: &str) -> (usize, usize) {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let kept = common_lines(&a, &b, |x, y| x == y);
    (b.len() - kept, a.len() - kept)
}

/// How many lines `a` and `b` have in common, in order (the longest
/// common subsequence), lines compared with `same`. A pair of files too
/// large to compare line by line counts only the lines both start and end
/// with.
fn common_lines(a: &[&str], b: &[&str], same: impl Fn(&str, &str) -> bool) -> usize {
    let head = a.iter().zip(b).take_while(|(x, y)| same(x, y)).count();
    let tail = a[head..].iter().rev().zip(b[head..].iter().rev()).take_while(|(x, y)| same(x, y)).count();
    let (a, b) = (&a[head..a.len() - tail], &b[head..b.len() - tail]);
    if a.is_empty() || b.is_empty() || a.len().saturating_mul(b.len()) > 50_000_000 {
        return head + tail;
    }
    // (one row at a time: the lengths only)
    let mut prev = vec![0u32; b.len() + 1];
    let mut row = vec![0u32; b.len() + 1];
    for x in a {
        for (j, y) in b.iter().enumerate() {
            row[j + 1] = if same(x, y) { prev[j] + 1 } else { row[j].max(prev[j + 1]) };
        }
        std::mem::swap(&mut prev, &mut row);
    }
    head + tail + prev[b.len()] as usize
}

/// A file a whole-file rewrite is refused for when it keeps most of it:
/// more lines than this.
pub const WHOLE_FILE_LINES: usize = 40;

/// Why write_file's `text` for an existing file of more than
/// [`WHOLE_FILE_LINES`] lines is refused: it keeps more than half of the
/// file's lines (trailing spaces aside) — a few lines' change sent as the
/// whole file. `None`: it changes most of it, or the file is short.
fn needless_rewrite(rel: &str, old: &str, new: &str) -> Option<String> {
    let a: Vec<&str> = old.lines().collect();
    if a.len() <= WHOLE_FILE_LINES {
        return None;
    }
    let b: Vec<&str> = new.lines().collect();
    let kept = common_lines(&a, &b, |x, y| x.trim_end() == y.trim_end());
    let changed = (a.len() - kept).max(b.len() - kept);
    (changed * 2 < a.len()).then(|| {
        format!(
            "Not proposed: write_file would send all {} lines of {rel} to change {} of them. Send only the parts that change with propose_edit: edits, each with old_text (a few whole lines copied exactly from {rel}, appearing once) and new_text (what replaces them); several edits in one call for several places. write_file is for a new file, or for rewriting most of one.",
            b.len(),
            changed.max(1)
        )
    })
}

/// Whether `p` is a new program proposed and not created yet (its files
/// are the proposals: rewriting one whole costs nothing on disk).
fn is_new_project(p: &Project) -> bool {
    p.folder.starts_with("/new/")
}

/// The project's files as the proposals waiting for the user would leave
/// them (each changed file's proposed text, each new file): what every
/// check sees.
fn project_files(p: &Project, proposals: &[Proposal]) -> Vec<(String, String)> {
    let mut files = p.all();
    for q in proposals.iter().filter(|q| q.project.is_empty() && is_source(&q.path)) {
        files.retain(|(f, _)| *f != q.path);
        files.push((q.path.clone(), q.new.clone()));
    }
    files
}

/// Where `new` (a file's text; `old` what it replaces, "" for a new file)
/// is cut off: it ends in the middle of a closing keyword (`END FUNCTI`),
/// or with a SUB, FUNCTION, CREATE or TYPE still open that `old` closed.
/// What it ends with, in words; `None` when it ends whole.
pub fn cut_off(old: &str, new: &str) -> Option<String> {
    const ENDS: &[&str] = &["END SUB", "END FUNCTION", "END CREATE", "END TYPE", "END SUBI", "END FUNCTIONI", "END IF", "END SELECT", "END WITH"];
    let last = new.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()).last();
    if let Some((n, line)) = last {
        let words = line.trim().to_ascii_uppercase().split_whitespace().collect::<Vec<_>>().join(" ");
        if words.starts_with("END ") && !ENDS.contains(&words.as_str()) && ENDS.iter().any(|e| e.starts_with(&words)) {
            return Some(format!("it ends in the middle of a line (line {}: `{}`)", n + 1, line.trim()));
        }
    }
    let (open, kind) = open_blocks(new);
    if open > 0 && open_blocks(old).0 == 0 {
        let n = new.lines().count();
        return Some(format!("it ends at line {n} with a {kind} still open (no END {kind})"));
    }
    None
}

/// The SUBs, FUNCTIONs, CREATE and TYPE blocks still open at the text's
/// end, and the last one opened.
fn open_blocks(text: &str) -> (usize, &'static str) {
    let mut stack: Vec<&'static str> = Vec::new();
    for line in text.lines() {
        for (_, st) in statements(line) {
            let w: Vec<String> = st.split_whitespace().take(3).map(str::to_ascii_uppercase).collect();
            let first = w.first().map(String::as_str).unwrap_or("");
            let second = w.get(1).map(String::as_str).unwrap_or("");
            match first {
                "SUB" | "SUBI" if !second.is_empty() => stack.push("SUB"),
                "FUNCTION" | "FUNCTIONI" if !second.is_empty() && second != "=" => stack.push("FUNCTION"),
                "CREATE" if w.get(2).map(String::as_str) == Some("AS") => stack.push("CREATE"),
                "TYPE" if !second.is_empty() => stack.push("TYPE"),
                "END" => {
                    let kind = match second {
                        "SUB" | "SUBI" => "SUB",
                        "FUNCTION" | "FUNCTIONI" => "FUNCTION",
                        "CREATE" => "CREATE",
                        "TYPE" => "TYPE",
                        _ => continue,
                    };
                    if let Some(k) = stack.iter().rposition(|o| *o == kind) {
                        stack.truncate(k);
                    }
                }
                _ => {}
            }
        }
    }
    (stack.len(), stack.last().copied().unwrap_or(""))
}

/// The syntax errors of `target` (the lexer's and the parser's), in the
/// project `files` — the compiler's words, a line each, each with the
/// hints that say its fix (another language's habit on the line).
fn syntax_problems(files: &[(String, String)], root: &str, target: &str) -> Vec<String> {
    let mut a = Analysis::new(Options { roots: Some(Vec::new()), ..Options::default() });
    let full = |rel: &str| PathBuf::from(format!("{}/{rel}", root.trim_end_matches('/')));
    for (rel, text) in files {
        a.update(full(rel), text.clone());
    }
    let mut out: Vec<String> = Vec::new();
    for d in a.syntax_errors(&full(target)) {
        let text = a.text(&d.file).unwrap_or_default();
        let (line, col) = crate::langsvc::line_col(&text, d.start);
        out.push(format!("error {target}:{line}:{col}: {}", d.message));
        let src = text.lines().nth(line.saturating_sub(1)).unwrap_or("");
        for (_, h) in crate::friendly::for_message(&d.message).into_iter().chain(crate::friendly::for_line(src)) {
            let hint = format!("hint {target}:{line}: {h}");
            if !out.contains(&hint) {
                out.push(hint);
            }
        }
    }
    out
}

/// Why the proposed `new` text of `rel` (replacing `old`) is refused: cut
/// off at its end, or a syntax error — each always refused, in a module
/// too, the project checked with every proposal waiting for the user
/// applied. (The message for the model, a few words for the step.)
fn refused(p: &Project, proposals: &[Proposal], rel: &str, old: &str, new: &str) -> Option<(String, String)> {
    if !is_source(rel) {
        return None;
    }
    if let Some(at) = cut_off(old, new) {
        return Some((format!("Not proposed: the new text of {rel} is cut off — {at}. A reply that ran out of room ends like that. Send the change again, complete: only the parts that change, with propose_edit."), "it is cut off at the end".into()));
    }
    let mut files = project_files(p, proposals);
    files.retain(|(f, _)| f != rel);
    files.push((rel.to_string(), new.to_string()));
    let errors = syntax_problems(&files, "/project", rel);
    if errors.is_empty() {
        return None;
    }
    let shown: Vec<&str> = errors.iter().take(12).map(String::as_str).collect();
    Some((format!("Not proposed: the new text of {rel} has a syntax error, so it can't be part of the program — fix it and send the change again (propose_edit, with every line it needs):\n{}", shown.join("\n")), "a syntax error".into()))
}

/// One `RCODEEDITOR.ApplyPatches` line that turns `old` into `new`: the
/// lines that differ replaced whole (the file's own line breaks kept), so
/// applying it is one undo step; "" when they are the same.
pub fn patch(old: &str, new: &str) -> String {
    let new = if old.contains("\r\n") { new.replace("\r\n", "\n").replace('\n', "\r\n") } else { new.to_string() };
    if old == new {
        return String::new();
    }
    let a: Vec<&str> = old.split('\n').collect();
    let b: Vec<&str> = new.split('\n').collect();
    // (the old text's last line never counts as common start: a change that
    // adds a blank line and more at the end — a new SUB after the last —
    // would start past the old text's end, and the patch apply nothing)
    let mut pre = 0;
    while pre + 1 < a.len().min(b.len()) && a[pre] == b[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < a.len() - pre && suf < b.len() - pre && a[a.len() - 1 - suf] == b[b.len() - 1 - suf] {
        suf += 1;
    }
    let repl = &b[pre..b.len() - suf];
    let (end_line, end_col, text) = if suf > 0 {
        (a.len() - suf, 0, repl.iter().map(|l| format!("{l}\n")).collect::<String>())
    } else {
        let last = a.len() - 1;
        (last, a[last].chars().count(), repl.join("\n"))
    };
    let esc = text.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t");
    format!("{pre}\t0\t{end_line}\t{end_col}\t{esc}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project<'a>(files: &'a [String], buffers: &'a HashMap<String, String>) -> Project<'a> {
        Project { folder: "/nowhere-on-disk", files, main: "main.rr", compat: "", buffers }
    }

    #[test]
    fn the_outline_is_the_program_as_compiled() {
        // (Robert's DeepSeek evening, 2026-10-20, in miniature: globals in
        // one file, the forms in another, the routines in a third)
        let files: Vec<String> = ["main.rr", "globals.rr", "ui.rr", "server.rr", "old.rr"].iter().map(|s| s.to_string()).collect();
        let mut buffers = HashMap::new();
        buffers.insert("main.rr".to_string(), "' the server\n$INCLUDE \"globals.rr\"\n$INCLUDE \"ui.rr\"\n$INCLUDE \"server.rr\"\nStartServer 8080\nForm.ShowModal\n".to_string());
        buffers.insert("globals.rr".to_string(), "CONST MAX_SITES = 10\nDIM siteCount AS INTEGER\nDIM siteName(MAX_SITES) AS STRING ' the names\nTYPE Site\n    Name AS STRING\n    Port AS INTEGER\nEND TYPE\n".to_string());
        buffers.insert("ui.rr".to_string(), "DECLARE SUB GoClick (Sender AS RButton)\nCREATE Form AS RForm\n    CREATE Go AS RButton\n        OnClick = GoClick\n    END CREATE\n    CREATE Sites AS QLISTBOX\n    END CREATE\nEND CREATE\nSUB GoClick (Sender AS RButton)\n    PRINT Sender.Caption\nEND SUB\n".to_string());
        buffers.insert("server.rr".to_string(), "SUB StartServer (port AS INTEGER)\n    siteCount = 0\nEND SUB\nFUNCTION SiteURL (i AS INTEGER, BYREF host AS STRING) AS STRING\n    Result = host\nEND FUNCTION\n".to_string());
        buffers.insert("old.rr".to_string(), "SUB Leftover\nEND SUB\n".to_string());
        let p = project(&files, &buffers);
        let o = run("project_outline", &json!({}), &p, &mut Vec::new(), true);
        assert!(!o.is_error && o.content.starts_with("{\"untrusted\":true,\"source\":\"project:outline\""), "{}", o.content);
        let v: Json = serde_json::from_str(&o.content).unwrap();
        let text = v["content"].as_str().unwrap();
        let order: Vec<usize> = ["\nmain.rr\n", "\nglobals.rr (included by main.rr:2)", "\nui.rr (included by main.rr:3)", "\nserver.rr (included by main.rr:4)"].iter().map(|h| text.find(h).unwrap_or_else(|| panic!("{h} not in\n{text}"))).collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{text}");
        for want in [
            "  1: CONST MAX_SITES = 10",
            "  2: DIM siteCount AS INTEGER",
            "  3: DIM siteName(MAX_SITES) AS STRING\n",
            "  4: TYPE Site: Name AS STRING, Port AS INTEGER",
            "  2: form Form AS RForm: Go AS RButton, Sites AS RListBox",
            "  9: SUB GoClick (Sender AS RButton)",
            "  1: SUB StartServer (port AS INTEGER)",
            "  4: FUNCTION SiteURL (i AS INTEGER, BYREF host AS STRING) AS STRING",
            "old.rr is NOT included by main.rr",
            "  1: SUB Leftover",
        ] {
            assert!(text.contains(want), "{want:?} not in\n{text}");
        }
        assert_eq!(o.note, "Outlined the project (5 files)");
        // (no project: said)
        let none = Project { folder: "", files: &[], main: "", compat: "", buffers: &HashMap::new() };
        assert!(run("project_outline", &json!({}), &none, &mut Vec::new(), true).content.starts_with("No project is open"));
    }

    #[test]
    fn the_manual_is_searched() {
        let empty = HashMap::new();
        let p = project(&[], &empty);
        let r = run("search_manual", &json!({"query": "how do I set a breakpoint"}), &p, &mut Vec::new(), true);
        assert!(!r.is_error && r.content.contains("docs/manual/debugging.md — "), "{}", r.content);
        assert!(r.content.matches("docs/manual/").count() <= 3);
        assert!(run("search_manual", &json!({"query": "qqzzx"}), &p, &mut Vec::new(), true).content.contains("nothing on"));
        assert!(run("search_manual", &json!({}), &p, &mut Vec::new(), true).is_error);
        // (both tools are offered, the outline's description says when to call it)
        let tools = tools(false);
        let outline = tools.iter().find(|t| t.name == "project_outline").expect("project_outline");
        assert!(outline.description.contains("before writing changes to a project of several files"));
        assert!(tools.iter().any(|t| t.name == "search_manual"));
    }

    #[test]
    fn paths_stay_inside() {
        assert_eq!(project_path("/p", "main.rr").unwrap(), "main.rr");
        assert_eq!(project_path("/p", "/p/forms/a.inc").unwrap(), "forms/a.inc");
        assert_eq!(project_path("/p", "./forms/../b.rr").unwrap(), "b.rr");
        assert!(project_path("/p", "../secret.txt").is_err());
        assert!(project_path("/p", "/etc/passwd").is_err());
        assert!(project_path("/p", "C:\\Windows\\x.ini").is_err());
        assert!(project_path("/p", "a/../../x").is_err());
        assert!(project_path("/p", "").is_err());
    }

    #[test]
    fn reading_checking_and_proposing() {
        let files = vec!["main.rr".to_string()];
        let mut buffers = HashMap::new();
        buffers.insert("main.rr".to_string(), "CREATE Form AS RForm\n    Caption = \"Hi\"\n    CREATE B1 AS RButton\n        Caption = \"OK\"\n    END CREATE\nEND CREATE\nForm.ShowModal\n".to_string());
        let p = project(&files, &buffers);
        let mut props = Vec::new();
        let r = run("read_file", &json!({"path": "main.rr"}), &p, &mut props, true);
        assert!(!r.is_error && r.content.starts_with("{\"untrusted\":true,\"source\":\"file:main.rr\""), "{}", r.content);
        assert_eq!(r.read.as_deref(), Some("file:main.rr"));
        assert!(run("read_file", &json!({"path": "../../etc/passwd"}), &p, &mut props, true).is_error);
        let l = run("rapidr_list", &json!({}), &p, &mut props, true);
        assert!(l.content.contains("\"B1\"") && l.content.contains("RButton"), "{}", l.content);
        let g = run("rapidr_get", &json!({"id": "B1", "props": ["Caption"]}), &p, &mut props, true);
        assert!(g.content.contains("\"OK\""), "{}", g.content);
        let d = run("rapidr_describe", &json!({"type": "QBUTTON"}), &p, &mut props, true);
        assert!(d.content.starts_with("RButton (RapidQ: QButton): visual component.") && d.content.contains("- Caption: string") && d.content.contains("Events"), "{}", d.content);
        assert!(run("rapidr_describe", &json!({"type": "RNope"}), &p, &mut props, true).is_error);
        let c = run("check_code", &json!({}), &p, &mut props, true);
        assert_eq!(c.content, "No problems: it compiles.");
        let bad = run("check_code", &json!({"path": "main.rr", "text": "PRINT 1\nNope x y\n"}), &p, &mut props, true);
        assert!(bad.content.starts_with("error main.rr:2:"), "{}", bad.content);
        // (an edit: proposed, nothing written; a second one builds on it)
        let e = run("propose_edit", &json!({"path": "main.rr", "summary": "Caption", "edits": [{"old_text": "Caption = \"OK\"", "new_text": "Caption = \"Save\""}]}), &p, &mut props, true);
        assert!(!e.is_error, "{}", e.content);
        let e2 = run("propose_edit", &json!({"path": "main.rr", "summary": "Title", "edits": [{"old_text": "Caption = \"Hi\"", "new_text": "Caption = \"Notes\""}]}), &p, &mut props, true);
        assert!(!e2.is_error);
        assert_eq!(props.len(), 1);
        assert!(props[0].new.contains("\"Save\"") && props[0].new.contains("\"Notes\"") && props[0].old.contains("\"OK\""));
        let miss = run("propose_edit", &json!({"path": "main.rr", "summary": "x", "edits": [{"old_text": "nope", "new_text": "y"}]}), &p, &mut props, true);
        assert!(miss.is_error && miss.content.contains("isn't in main.rr") && miss.content.contains("   2|     Caption = \"Notes\""), "{}", miss.content);
        assert_eq!(miss.risk, "state", "a proposal that didn't fit is still a proposal's risk");
        let twice = run("propose_edit", &json!({"path": "main.rr", "summary": "x", "edits": [{"old_text": "Caption", "new_text": "y"}]}), &p, &mut props, true);
        assert!(twice.is_error && twice.content.contains("2 times"), "{}", twice.content);
        // (read-only: refused)
        assert!(run("write_file", &json!({"path": "a.rr", "text": "x", "summary": "s"}), &p, &mut props, false).is_error);
        // (write_file with create_project's arguments: each file written)
        let mixed = run("write_file", &json!({"name": "x", "main": "extra.rr", "summary": "more", "files": [{"path": "extra.rr", "text": "PRINT 2\n"}]}), &p, &mut props, true);
        assert!(!mixed.is_error && props.iter().any(|q| q.path == "extra.rr" && q.kind == "create"), "{}", mixed.content);
        props.retain(|q| q.path != "extra.rr");
        // (a new project: checked; errors go back)
        let np = run("create_project", &json!({"name": "hello-form", "main": "main.rr", "summary": "Hi", "files": [{"path": "main.rr", "text": "CREATE F AS RForm\n  Caption = \"Hello\"\nEND CREATE\nF.ShowModal\n"}]}), &p, &mut props, true);
        assert!(np.content.contains("compiles"), "{}", np.content);
        assert_eq!(props.iter().filter(|q| q.project == "hello-form" && q.main).count(), 1);
        let broken = run("create_project", &json!({"name": "x", "main": "main.rr", "summary": "x", "files": [{"path": "main.rr", "text": "$INCLUDE \"form.inc\"\nF.ShowModal\n"}, {"path": "form.inc", "text": "CREATE F AS RForm\n  Caption = \"x\"\nEND CREATE\nNope x y\n"}]}), &p, &mut props, true);
        assert!(broken.content.contains("doesn't compile"), "{}", broken.content);
        // (its include was found among its own files: only the property is wrong)
        assert!(!broken.content.contains("form.inc: not found") && broken.content.contains("form.inc"), "{}", broken.content);
        // (a name that is a path: only its words make the folder — never a
        // folder above the projects')
        let up = run("create_project", &json!({"name": "../x", "main": "main.rr", "summary": "x", "files": [{"path": "main.rr", "text": "x"}]}), &p, &mut props, true);
        assert!(!up.is_error && props.iter().any(|q| q.project == "X" && q.title == "../x"), "{}", up.content);
        // (Robert's walk, 2026-10-18: a title with spaces was refused — any
        // title makes a program now, its folder made from it)
        let titled = run("create_project", &json!({"name": "Scientific Calculator", "main": "main.rr", "summary": "x", "files": [{"path": "main.rr", "text": "CREATE F AS RForm\nEND CREATE\nF.ShowModal\n"}]}), &p, &mut props, true);
        assert!(!titled.is_error && titled.note == "Wrote the new program Scientific Calculator (1 file): it compiles", "{}", titled.note);
        assert!(props.iter().any(|q| q.project == "ScientificCalculator" && q.title == "Scientific Calculator" && q.main), "{props:#?}");
        // (gemma E4B, 2026-10-18: no name, no main — the files say them)
        let mut props = Vec::new();
        let none = Project { folder: "", files: &[], main: "", compat: "", buffers: &HashMap::new() };
        let bare = run("create_project", &json!({"summary": "Tips", "files": [{"path": "tip calculator.rr", "text": "CREATE F AS RForm\nEND CREATE\nF.ShowModal\n"}]}), &none, &mut props, true);
        assert!(!bare.is_error && bare.content.contains("The project tip_calculator (1 file) compiles"), "{}", bare.content);
        assert!(props.iter().any(|q| q.project == "tip_calculator" && q.main && q.path == "tip calculator.rr"), "{props:#?}");
        let wrong_main = run("create_project", &json!({"name": "T", "main": "app.rr", "summary": "T", "files": [{"path": "forms.inc", "text": "CREATE F AS RForm\nEND CREATE\n"}, {"path": "main.rr", "text": "$INCLUDE \"forms.inc\"\nF.ShowModal\n"}]}), &none, &mut props, true);
        assert!(!wrong_main.is_error && props.iter().any(|q| q.project == "T" && q.main && q.path == "main.rr"), "{}", wrong_main.content);
        // (write_file with create_project's files and no project open: a new program too)
        let mixed = run("write_file", &json!({"summary": "x", "files": [{"path": "main.rr", "text": "PRINT 1\n"}]}), &none, &mut props, true);
        assert!(!mixed.is_error && props.iter().any(|q| q.project == "main"), "{}", mixed.content);
    }

    #[test]
    fn folder_names_from_titles() {
        // (a name that is one already stays)
        assert_eq!(project_name("hello-form"), "hello-form");
        assert_eq!(project_name("Calc_2"), "Calc_2");
        // (words joined, each capitalised)
        assert_eq!(project_name("Scientific Calculator"), "ScientificCalculator");
        assert_eq!(project_name("  tip   calculator! "), "TipCalculator");
        assert_eq!(project_name("Celsius/Fahrenheit converter"), "CelsiusFahrenheitConverter");
        // (accents dropped)
        assert_eq!(project_name("calculadora científica"), "CalculadoraCientifica");
        assert_eq!(project_name("Ñandú Straße"), "NanduStrasse");
        // (nothing a folder can be named: a name all the same)
        assert_eq!(project_name("¿?"), "NewProgram");
        assert_eq!(project_name(""), "NewProgram");
        assert_eq!(project_name("../.."), "NewProgram");
        assert!(project_name(&"long words ".repeat(20)).len() <= 60);
        // (the lines a change adds and removes)
        assert_eq!(line_delta("a\nb\nc\n", "a\nB\nB2\nc\n"), (2, 1));
        assert_eq!(line_delta("", "a\nb\n"), (2, 0));
        assert_eq!(line_delta("a\n", "a\n"), (0, 0));
    }

    #[test]
    fn an_edit_that_misses_shows_the_closest_lines() {
        let text = (1..=60).map(|i| if i == 40 { "    SiteList.AddItems \"alpha\"".to_string() } else { format!("PRINT {i}") }).collect::<Vec<_>>().join("\n") + "\n";
        let e = apply_edits("main.rr", &text, &[json!({"old_text": "SiteList.AddItem \"alpha\"\nPRINT 41x", "new_text": "x"})]).unwrap_err();
        assert!(e.contains("isn't in main.rr") && e.contains("  40|     SiteList.AddItems \"alpha\"") && e.contains("call propose_edit again") && !e.contains("write_file"), "{e}");
        // (at most 30 lines, numbered from the file)
        assert!(e.lines().filter(|l| l.contains("| ")).count() <= 30, "{e}");
        assert!(!e.contains("   1| "), "{e}");
        // (an old text that is there but for its indentation fits, the
        // file's indentation kept)
        let src = "CREATE Form AS RForm\n    CREATE B1 AS RButton\n        Caption = \"OK\"\n    END CREATE\nEND CREATE\n";
        let fixed = apply_edits("main.rr", src, &[json!({"old_text": "CREATE B1 AS RButton\n    Caption = \"OK\"\n", "new_text": "CREATE B1 AS RButton\n    Caption = \"Save\"\n    Width = 90\n"})]).unwrap();
        assert_eq!(fixed, "CREATE Form AS RForm\n    CREATE B1 AS RButton\n        Caption = \"Save\"\n        Width = 90\n    END CREATE\nEND CREATE\n", "{fixed}");
        // (lines off by different amounts: the first line placed, the rest as written)
        let fixed = apply_edits("main.rr", src, &[json!({"old_text": "Caption = \"OK\"\nEND CREATE", "new_text": "Caption = \"Save\"\n    Width = 90\nEND CREATE"})]).unwrap();
        assert_eq!(fixed, "CREATE Form AS RForm\n    CREATE B1 AS RButton\n        Caption = \"Save\"\n    Width = 90\nEND CREATE\nEND CREATE\n", "{fixed}");
        let one = apply_edits("main.rr", src, &[json!({"old_text": "  caption = \"OK\"  ", "new_text": "Caption = \"Go\""})]);
        assert!(one.is_err(), "case still counts");
        let one = apply_edits("main.rr", src, &[json!({"old_text": "  Caption = \"OK\"  ", "new_text": "  Caption = \"Go\""})]).unwrap();
        assert!(one.contains("        Caption = \"Go\"\n"), "{one}");
        // (twice: refused, as before)
        let twice = "a\n  x = 1\nb\n  x = 1\n";
        assert!(apply_edits("m.rr", twice, &[json!({"old_text": "x = 1", "new_text": "y"})]).unwrap_err().contains("2 times"));
        assert!(apply_edits("m.rr", twice, &[json!({"old_text": " x = 1 ", "new_text": "y"})]).is_err());
    }

    #[test]
    fn properties_no_component_has() {
        let src = "DECLARE SUB Go\nCREATE Form AS RForm\n    Caption = \"Sites\"\n    CREATE SiteListView AS RListBox\n        Left = 8: Top = 8\n        ColumnWidths = 250, 100\n        OnClick = Go\n    END CREATE\nEND CREATE\nSUB Go\n    SiteListView.ColumnWidths = 3\n    SiteListView.Font.Size = 12\n    SiteListView.AddItems \"x\"\n    Form.Caption = \"x = 1\"\n    WITH SiteListView\n        .Nope = 1\n    END WITH\nEND SUB\nForm.ShowModal\n";
        let files = vec![("main.rr".to_string(), src.to_string())];
        let found = member_problems(&files, &["main.rr"]);
        assert_eq!(found.len(), 3, "{found:#?}");
        assert!(found[0].starts_with("error main.rr:6:9: RListBox has no property ColumnWidths") && found[0].contains("rapidr_describe RListBox"), "{}", found[0]);
        assert!(found[1].starts_with("error main.rr:11:18: RListBox has no property SiteListView.ColumnWidths"), "{}", found[1]);
        assert!(found[2].contains("main.rr:16:") && found[2].contains("Nope"), "{}", found[2]);
        // (the nearest member named; said once where the compiler warned too)
        let grid = vec![("g.rr".to_string(), "CREATE F AS RForm\n  CREATE G AS RStringGrid\n    ColumnCount = 2\n    ColCount = 2\n  END CREATE\nEND CREATE\nF.ShowModal\n".to_string())];
        let g = member_problems(&grid, &["g.rr"]);
        assert!(g.len() == 1 && g[0].contains("RStringGrid has no property ColumnCount; did you mean ColCount?"), "{g:#?}");
        let d = diagnostics(&grid, "/p", "g.rr", "g.rr", false);
        assert_eq!(d.iter().filter(|l| l.contains("COLUMNCOUNT") || l.contains("ColumnCount")).count(), 1, "{d:#?}");
        // (a TYPE's objects and unknown names: not checked)
        let t = vec![("a.rr".to_string(), "TYPE TBox EXTENDS QFORM\n  Extra AS INTEGER\nEND TYPE\nDIM B AS TBox\nB.Extra = 1\nCREATE F AS TBox\n  Extra = 2\nEND CREATE\nX.Y = 3\n".to_string())];
        assert!(member_problems(&t, &["a.rr"]).is_empty());
        // (check_code says it; the compiler and the runtime are unchanged:
        // it still compiles)
        let names = vec!["main.rr".to_string()];
        let mut buffers = HashMap::new();
        buffers.insert("main.rr".to_string(), src.to_string());
        let p = project(&names, &buffers);
        let c = run("check_code", &json!({}), &p, &mut Vec::new(), true);
        assert!(c.content.contains("RListBox has no property ColumnWidths"), "{}", c.content);
        let mut a = Analysis::new(Options { roots: Some(Vec::new()), ..Options::default() });
        a.update(PathBuf::from("/p/main.rr"), src.to_string());
        assert!(a.diagnostics(&PathBuf::from("/p/main.rr")).iter().all(|d| !d.message.contains("ColumnWidths")), "the compiler's own diagnostics don't change");
    }

    /// RapidR's own programs (the examples, RapidR Studio itself) set no
    /// property their components don't have: the check names none there.
    #[test]
    fn rapidrs_own_programs_have_no_unknown_properties() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |p: &std::path::Path| (p.strip_prefix(&root).unwrap().to_string_lossy().to_string(), std::fs::read_to_string(p).unwrap_or_default());
        let listed = |d: &std::path::Path, ext: &str| -> Vec<PathBuf> { std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == ext)).collect() };
        let mut found = Vec::new();
        let mut seen = 0;
        // (RapidR Studio: one program)
        let ide: Vec<(String, String)> = listed(&root.join("ide"), "inc").iter().chain(listed(&root.join("ide"), "rr").iter()).map(|p| read(p)).collect();
        let names: Vec<&str> = ide.iter().map(|(p, _)| p.as_str()).collect();
        found.extend(member_problems(&ide, &names));
        seen += ide.len();
        // (each example: its file and the includes beside it; RapidQ-style
        // examples keep RapidQ's own leniencies)
        let mut dirs = vec![root.join("examples")];
        while let Some(d) = dirs.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                if e.path().is_dir() && !e.path().ends_with("rapidq") {
                    dirs.push(e.path());
                }
            }
            let incs: Vec<(String, String)> = listed(&d, "inc").iter().map(|p| read(p)).collect();
            for f in listed(&d, "rr") {
                let mut files = vec![read(&f)];
                files.extend(incs.iter().cloned());
                let name = files[0].0.clone();
                found.extend(member_problems(&files, &[name.as_str()]));
                seen += 1;
            }
        }
        assert!(seen > 30, "{seen}");
        assert!(found.is_empty(), "{} found:\n{}", found.len(), found.join("\n"));
    }

    /// Robert's scientific calculator (2026-10-18, gemma E4B): the habits
    /// from other languages come back with their fixes, under the
    /// compiler's own errors.
    #[test]
    fn check_code_says_the_fix_for_other_languages_habits() {
        let src = "DECLARE SUB Press\nDIM waitingForOperand AS BOOLEAN AS BOOLEAN\nCREATE Form AS RForm\n    CREATE Display AS RLabel\n        Align = alTop\n    END CREATE\n    CREATE B7 AS RButton\n        Caption = \"7\": OnClick = Press\n    END CREATE\nEND CREATE\nSUB Press\n    IF ButtonPressed != \"7\" THEN Display.Caption = \"7\"\nEND SUB\n";
        let files = vec!["main.rr".to_string()];
        let mut buffers = HashMap::new();
        buffers.insert("main.rr".to_string(), src.to_string());
        let p = project(&files, &buffers);
        let c = run("check_code", &json!({}), &p, &mut Vec::new(), true).content;
        // (the compiler stops at its first error — the `!` — and the habit
        // on the line it never reached is said too)
        assert!(c.contains("hint main.rr:12: `!=` isn't BASIC: \"not equal\" is `<>`"), "{c}");
        assert!(c.contains("error main.rr:2:1: a name takes one `AS type`"), "{c}");
        assert!(c.contains("never shows its window: end the main file with Form.ShowModal"), "{c}");
        // (RAPIDQ.INC's constants are built into a RapidR program: nothing
        // to say; in a RapidQ program without its include, an error, for
        // the assistant)
        let aligned = "CREATE Form AS RForm\n    CREATE Display AS RLabel\n        Align = alTop\n    END CREATE\nEND CREATE\nForm.ShowModal\n";
        buffers.insert("main.rr".to_string(), aligned.to_string());
        let p = project(&files, &buffers);
        let c = run("check_code", &json!({}), &p, &mut Vec::new(), true).content;
        assert_eq!(c, "No problems: it compiles.");
        let bas = vec!["main.bas".to_string()];
        let mut bas_buffers = HashMap::new();
        bas_buffers.insert("main.bas".to_string(), aligned.to_string());
        let p = Project { main: "main.bas", ..project(&bas, &bas_buffers) };
        let c = run("check_code", &json!({}), &p, &mut Vec::new(), true).content;
        assert!(c.contains("error main.bas:3:17: alTop is a RAPIDQ.INC constant"), "{c}");
        // (a name never given a value, the nearest real one offered)
        let src2 = "DECLARE SUB Press\nDIM ButtonPress AS STRING\nCREATE Form AS RForm\n    CREATE B7 AS RButton\n        Caption = \"7\": OnClick = Press\n    END CREATE\nEND CREATE\nSUB Press\n    ButtonPress = \"7\"\n    IF ButtonPressed = \"7\" THEN B7.Caption = \"8\"\nEND SUB\nForm.ShowModal\n";
        buffers.insert("main.rr".to_string(), src2.to_string());
        let p = project(&files, &buffers);
        let c = run("check_code", &json!({}), &p, &mut Vec::new(), true).content;
        assert!(c.contains("error main.rr:10:8: ButtonPressed is never given a value nor declared") && c.contains("did you mean ButtonPress?"), "{c}");
        // (a data type from another language; a placeholder handler)
        let src3 = "DECLARE SUB Go\nDIM running AS BOOLEAN\nCREATE Form AS RForm\n    CREATE B AS RButton\n        OnClick = Go\n    END CREATE\nEND CREATE\nSUB Go\n    ' In a real implementation, this would start the server.\n    B.Caption = \"Started\"\nEND SUB\nForm.ShowModal\n";
        buffers.insert("main.rr".to_string(), src3.to_string());
        let p = project(&files, &buffers);
        let c = run("check_code", &json!({}), &p, &mut Vec::new(), true).content;
        assert!(c.contains("Unknown data type BOOLEAN") && c.contains("hint main.rr:2: RapidR has no BOOLEAN: use INTEGER"), "{c}");
        assert!(c.contains("error main.rr:9:1: SUB Go isn't finished: its comment says \"In a real implementation"), "{c}");
        // (a property a component doesn't have: what it has, once)
        let src4 = "CREATE Form AS RForm\n    CREATE G AS RStringGrid\n        ColumnCount = 2\n        RowHeight = 3\n    END CREATE\nEND CREATE\nForm.ShowModal\n";
        buffers.insert("main.rr".to_string(), src4.to_string());
        let p = project(&files, &buffers);
        let c = run("check_code", &json!({}), &p, &mut Vec::new(), true).content;
        assert!(c.contains("RStringGrid has no property ColumnCount; did you mean ColCount?"), "{c}");
        assert_eq!(c.matches("RStringGrid has — properties: ").count(), 1, "{c}");
        // (the proposing tools say the same)
        let mut props = Vec::new();
        buffers.insert("main.rr".to_string(), "CREATE Form AS RForm\nEND CREATE\nForm.ShowModal\n".to_string());
        let p = project(&files, &buffers);
        let w = run("write_file", &json!({"path": "main.rr", "summary": "x", "text": src}), &p, &mut props, true);
        // (syntax errors: refused, nothing proposed, the fix said)
        assert!(w.is_error && w.content.starts_with("Not proposed: the new text of main.rr has a syntax error") && w.content.contains("`!=` isn't BASIC") && props.is_empty(), "{}", w.content);
    }

    /// The `layout` tool's keypad compiles, fits the client size it says,
    /// and check_code finds nothing wrong with how it looks; a form with a
    /// button cut off, two on top of each other and a caption too long
    /// gets those three warnings — and the recipes none.
    #[test]
    fn a_laid_out_keypad_and_the_layout_check() {
        let o = run("layout", &json!({"rows": [["7", "8", "9", "/"], ["4", "5", "6", "*"], ["1", "2", "3", "-"], ["0", ".", "=", "+"]], "top": 60, "on_click": "KeyPress"}), &project(&[], &HashMap::new()), &mut Vec::new(), true);
        assert!(!o.is_error && o.content.starts_with("    CREATE Key7 AS RButton: Caption = \"7\": Left = 8: Top = 60: Width = 56: Height = 36: OnClick = KeyPress: END CREATE"), "{}", o.content);
        assert!(o.content.contains("CREATE KeyPlus AS RButton: Caption = \"+\": Left = 188: Top = 180") && o.content.contains("ClientWidth = 252: ClientHeight = 224"), "{}", o.content);
        let blocks: String = o.content.lines().filter(|l| l.contains("CREATE ")).map(|l| format!("{l}\n")).collect();
        let src = format!("DECLARE SUB KeyPress (Sender AS RButton)\nCREATE Form AS RForm\n    ClientWidth = 252: ClientHeight = 224\n    CREATE Display AS RLabel\n        Align = alTop: Height = 52: Caption = \"0\"\n    END CREATE\n{blocks}END CREATE\nSUB KeyPress (Sender AS RButton)\n    Display.Caption = Sender.Caption\nEND SUB\nForm.ShowModal\n");
        let files = vec!["main.rr".to_string()];
        let mut buffers = HashMap::new();
        buffers.insert("main.rr".to_string(), src);
        let c = run("check_code", &json!({}), &project(&files, &buffers), &mut Vec::new(), true).content;
        assert_eq!(c, "No problems: it compiles.");
        // (cut off, overlapping, a long caption)
        let bad = "CREATE Form AS RForm\n    ClientWidth = 200: ClientHeight = 100\n    CREATE A AS RButton\n        Caption = \"OK\": Left = 150: Top = 10: Width = 80: Height = 30\n    END CREATE\n    CREATE B AS RButton\n        Caption = \"Go\": Left = 10: Top = 10: Width = 80: Height = 30\n    END CREATE\n    CREATE C AS RButton\n        Caption = \"Calculate the total\": Left = 40: Top = 30: Width = 60: Height = 30\n    END CREATE\nEND CREATE\nForm.ShowModal\n";
        let found = layout_problems(&[("main.rr".to_string(), bad.to_string())]);
        assert!(found.iter().any(|l| l.contains("A (Left 150 + Width 80, Top 10 + Height 30) reaches past Form's client area (200 × 100)")), "{found:#?}");
        assert!(found.iter().any(|l| l.contains("B (10,10 80×30) and C (40,30 60×30) overlap")), "{found:#?}");
        assert!(found.iter().any(|l| l.contains("C's caption \"Calculate the total\" needs about 145 pixels, its Width is 60")), "{found:#?}");
        // (RapidR's recipes: nothing)
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/recipes");
        for e in std::fs::read_dir(&root).unwrap().flatten() {
            if e.path().extension().is_some_and(|x| x == "rr") {
                let text = std::fs::read_to_string(e.path()).unwrap();
                let found = layout_problems(&[(e.file_name().to_string_lossy().to_string(), text)]);
                assert!(found.is_empty(), "{}: {found:#?}", e.path().display());
            }
        }
    }

    /// The habits searched for in a whole file never compile: saying them
    /// where the compiler didn't reach can't contradict it.
    #[test]
    fn the_habits_searched_for_everywhere_never_compile() {
        let samples = [("ne", "IF a != 1 THEN PRINT 1"), ("or", "IF a = 1 || b = 2 THEN PRINT 1"), ("braces", "IF a = 1 THEN {"), ("brackets", "a = [1, 2]"), ("comment", "a = 1 // one"), ("as-as", "DIM b AS INTEGER AS INTEGER")];
        let mut compiles: Vec<String> = Vec::new();
        for (id, line) in samples {
            assert!(crate::friendly::for_line(line).iter().any(|(i, _)| *i == id), "{line}: {id}");
            let mut a = Analysis::new(Options { roots: Some(Vec::new()), ..Options::default() });
            a.update(PathBuf::from("/p/m.rr"), format!("DIM a AS INTEGER\n{line}\n"));
            if !a.diagnostics(&PathBuf::from("/p/m.rr")).iter().any(|d| d.severity == rapidr_langsvc::Severity::Error) {
                compiles.push(format!("{id}: {line}"));
            }
        }
        assert!(compiles.is_empty(), "{compiles:?}");
        assert!(crate::friendly::NEVER_BASIC.iter().all(|id| samples.iter().any(|(i, _)| i == id)));
    }

    /// The checks the assistant gets beyond the compiler's (names never
    /// given a value, handlers that do nothing, the window never shown) find
    /// nothing in RapidR's own programs: they don't cry wolf.
    #[test]
    fn rapidrs_own_programs_pass_the_assistants_extra_checks() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |p: &std::path::Path| (p.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read_to_string(p).unwrap_or_default());
        let listed = |d: &std::path::Path, ext: &str| -> Vec<PathBuf> { std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == ext)).collect() };
        let extra = |l: &&String| l.contains("is never given a value") || l.contains("does nothing yet") || l.contains("isn't finished") || l.contains("never shows its window");
        let mut found = Vec::new();
        let mut seen = 0;
        let mut dirs = vec![root.join("examples")];
        while let Some(d) = dirs.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                if e.path().is_dir() && !e.path().ends_with("rapidq") {
                    dirs.push(e.path());
                }
            }
            let incs: Vec<(String, String)> = listed(&d, "inc").iter().map(|p| read(p)).collect();
            for f in listed(&d, "rr") {
                let mut files = vec![read(&f)];
                files.extend(incs.iter().cloned());
                let name = files[0].0.clone();
                found.extend(diagnostics(&files, "/p", &name, &name, false).iter().filter(extra).cloned());
                seen += 1;
            }
        }
        assert!(seen > 30, "{seen}");
        assert!(found.is_empty(), "{} found:\n{}", found.len(), found.join("\n"));
    }

    #[test]
    fn one_patch_per_change() {
        assert_eq!(patch("a\nb\nc\n", "a\nb\nc\n"), "");
        assert_eq!(patch("a\nb\nc\n", "a\nB\nc\n"), "1\t0\t2\t0\tB\\n");
        assert_eq!(patch("a\nb\n", "a\nb\nc\n"), "2\t0\t2\t0\tc\\n");
        assert_eq!(patch("a\nb", "a\nB"), "1\t0\t1\t1\tB");
        assert_eq!(patch("a\nb\nc", "c"), "0\t0\t2\t0\t");
        // (a CR LF file keeps its breaks)
        assert_eq!(patch("a\r\nb\r\nc\r\n", "a\nX\tY\nc\n"), "1\t0\t2\t0\tX\\tY\\r\\n");
        // (a blank line and a new SUB after the last line: inserted before the
        // old text's end — it was a patch past it, applying nothing, while the
        // chat said "You applied the change")
        assert_eq!(patch("SUB A\nEND SUB\n", "SUB A\nEND SUB\n\nSUB B\nEND SUB\n"), "2\t0\t2\t0\t\\nSUB B\\nEND SUB\\n");
        assert_eq!(patch("x", "x\n\ny"), "0\t0\t0\t1\tx\\n\\ny");
    }

    /// R11: what is refused, and what isn't.
    #[test]
    fn syntax_errors_and_cut_off_texts_are_refused_even_in_modules() {
        let files = vec!["main.rr".to_string(), "server.rr".to_string()];
        let mut buffers = HashMap::new();
        buffers.insert("main.rr".to_string(), "$INCLUDE \"server.rr\"\nStartServer\n".to_string());
        buffers.insert("server.rr".to_string(), "SUB StartServer\n    PRINT 1\nEND SUB\n".to_string());
        let p = project(&files, &buffers);
        let mut props = Vec::new();
        // (a module the main file includes: a syntax error refused)
        let bad = run("propose_edit", &json!({"path": "server.rr", "summary": "x", "edits": [{"old_text": "PRINT 1", "new_text": "IF x != 1 THEN PRINT 1"}]}), &p, &mut props, true);
        assert!(bad.is_error && bad.content.starts_with("Not proposed: the new text of server.rr has a syntax error") && bad.content.contains("error server.rr:2:"), "{}", bad.content);
        assert!(props.is_empty());
        // (a new module nothing includes yet: refused all the same)
        let lone = run("write_file", &json!({"path": "other.rr", "summary": "x", "text": "SUB Other\n    PRINT (\nEND SUB\n"}), &p, &mut props, true);
        assert!(lone.is_error && lone.content.contains("has a syntax error"), "{}", lone.content);
        // (cut off: an END cut short, a SUB never closed)
        let cut = run("write_file", &json!({"path": "server.rr", "summary": "x", "text": "SUB StartServer\n    PRINT 1\nEND SU"}), &p, &mut props, true);
        assert!(cut.is_error && cut.content.contains("cut off — it ends in the middle of a line (line 3: `END SU`)"), "{}", cut.content);
        let open = run("propose_edit", &json!({"path": "server.rr", "summary": "x", "edits": [{"old_text": "    PRINT 1\nEND SUB\n", "new_text": "    PRINT 1\n"}]}), &p, &mut props, true);
        assert!(open.is_error && open.content.contains("with a SUB still open (no END SUB)"), "{}", open.content);
        assert!(props.is_empty());
        // (a type RapidR doesn't have isn't a syntax error: proposed, with
        // the error to fix, as before)
        let boolean = run("propose_edit", &json!({"path": "main.rr", "summary": "x", "edits": [{"old_text": "StartServer\n", "new_text": "DIM ready AS BOOLEAN\nStartServer\n"}]}), &p, &mut props, true);
        assert!(!boolean.is_error && boolean.content.contains("doesn't compile with it yet"), "{}", boolean.content);
        // (a new program with a file cut off: nothing made)
        let mut np = Vec::new();
        let made = run("create_project", &json!({"name": "Cut", "main": "main.rr", "summary": "x", "files": [{"path": "main.rr", "text": "CREATE F AS RForm\n    Caption = \"x\"\nEND CREATE\nF.ShowModal\nSUB Go\n    PRINT 1\nEND SU"}]}), &project(&[], &HashMap::new()), &mut np, true);
        assert!(made.is_error && made.content.contains("main.rr's text is cut off") && np.is_empty(), "{}", made.content);
        // (one-line blocks, a layout tool's buttons, close themselves)
        assert_eq!(cut_off("", "CREATE F AS RForm\n    CREATE K AS RButton: Caption = \"7\": END CREATE\nEND CREATE\n"), None);
        assert_eq!(cut_off("", "SUB A\nEND SUB\nFUNCTION B AS INTEGER\n    Result = 1\nEND FUNCTION\n"), None);
        // (a file that was never closed, in RapidQ's leniency, stays so)
        assert_eq!(cut_off("SUB A\n    PRINT 1\n", "SUB A\n    PRINT 2\n"), None);
    }

    /// R11: a few lines' change sent as the whole file of a long one is
    /// refused (unless Studio says the edits kept missing); a short file,
    /// a new one, or a rewrite of most of one are taken; the card counts
    /// the lines that change, wherever they are.
    #[test]
    fn a_whole_file_rewrite_needs_to_change_most_of_the_file() {
        let long: String = (1..=120).map(|i| format!("PRINT {i}\n")).collect();
        let files = vec!["main.rr".to_string()];
        let mut buffers = HashMap::new();
        buffers.insert("main.rr".to_string(), long.clone());
        let p = project(&files, &buffers);
        let mut props = Vec::new();
        let few = long.replace("PRINT 3\n", "PRINT 33\n").replace("PRINT 118\n", "PRINT 1188\n");
        let w = run("write_file", &json!({"path": "main.rr", "summary": "x", "text": few}), &p, &mut props, true);
        assert!(w.is_error && w.content.contains("to change 2 of them") && w.note == "Refused a whole-file rewrite of main.rr: only a few lines change", "{}", w.content);
        // (Studio's word: the edits kept missing)
        let w = run("write_file", &json!({"path": "main.rr", "summary": "x", "text": few, "_rewrite_ok": true}), &p, &mut props, true);
        assert!(!w.is_error, "{}", w.content);
        assert_eq!(line_delta(&props[0].old, &props[0].new), (2, 2));
        props.clear();
        // (most of it changes: taken)
        let most: String = (1..=120).map(|i| format!("PRINT {}\n", i * 2)).collect();
        assert!(!run("write_file", &json!({"path": "main.rr", "summary": "x", "text": most}), &p, &mut props, true).is_error);
        // (a short file, and a new one: taken)
        let mut short = HashMap::new();
        short.insert("main.rr".to_string(), "PRINT 1\nPRINT 2\n".to_string());
        assert!(!run("write_file", &json!({"path": "main.rr", "summary": "x", "text": "PRINT 1\nPRINT 3\n"}), &project(&files, &short), &mut Vec::new(), true).is_error);
        assert!(!run("write_file", &json!({"path": "new.rr", "summary": "x", "text": long}), &p, &mut Vec::new(), true).is_error);
        // (the lines that change, not the span between the first and the last)
        assert_eq!(line_delta(&long, &long.replace("PRINT 1\n", "PRINT 0\n").replace("PRINT 120\n", "PRINT 121\n")), (2, 2));
    }
}
