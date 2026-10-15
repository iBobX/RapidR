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
//! | `propose_edit`, `write_file` | proposal | a change shown to the user as a diff: nothing is written until they apply it |
//! | `create_project` | proposal | a new program's files, previewed before anything is created |
//!
//! What a tool returns from the project (a file, a property's value) goes
//! back to the model as data, marked untrusted with where it came from
//! (§5.5): never folded into the instructions.

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
    /// A new project's name ("" for changes to the open project).
    pub project: String,
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
    ];
    if proposals {
        out.push(t(
            "propose_edit",
            "Proposes changes to an existing file of the project, shown to the user as a diff they accept or reject (nothing changes until they do). Each edit replaces `old_text` — which must appear exactly once in the file, with enough lines around it to be unique — with `new_text`.",
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
            "Proposes a file's whole new text (a new file of the project, or one rewritten), shown to the user as a diff they accept or reject.",
            schema(json!({"path": {"type": "string"}, "text": {"type": "string"}, "summary": {"type": "string"}}), &["path", "text", "summary"]),
        ));
        out.push(t(
            "create_project",
            "Proposes a new program as a new project — its files (a main .rr file; more for forms or modules it $INCLUDEs) — which the user previews, then creates where they choose. Checked by the compiler: fix the errors it reports and call it again.",
            schema(
                json!({
                    "name": {"type": "string", "description": "the project's name: letters, digits, - and _ (its folder)"},
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
    Ok(parts.join("/"))
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
    match name {
        "list_files" => list_files(project),
        "read_file" => read_file(input, project),
        "rapidr_describe" => describe(input),
        "rapidr_list" | "rapidr_get" => design(name, input, project),
        "check_code" => check(input, project),
        "propose_edit" | "write_file" | "create_project" if !may_propose => err("the user allowed reading only (Tools > Settings > AI > Permission): describe the change in words instead", format!("{name}: not allowed (read-only)"), ""),
        "propose_edit" => propose_edit(input, project, proposals),
        "write_file" => write_file(input, project, proposals),
        "create_project" => create_project(input, proposals),
        other => err(format!("there is no tool `{other}`"), format!("{other}: no such tool"), ""),
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
    for f in checked {
        for d in a.diagnostics(&full(f)) {
            let text = a.text(&d.file).unwrap_or_default();
            let (line, col) = crate::langsvc::line_col(&text, d.start);
            let file = d.file.to_string_lossy().replace('\\', "/");
            let file = file.strip_prefix(&format!("{}/", root.trim_end_matches('/'))).unwrap_or(&file).to_string();
            let sev = format!("{:?}", d.severity).to_ascii_lowercase();
            let l = format!("{sev} {file}:{line}:{col}: {}", d.message);
            if !out.contains(&l) {
                out.push(l);
            }
        }
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

/// `old` with each edit applied; an edit whose old text isn't there exactly
/// once is refused, saying so.
fn apply_edits(old: &str, edits: &[Json]) -> Result<String, String> {
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
            0 => return Err(format!("edit {}: old_text isn't in the file (read it again: it must match exactly, spaces and line breaks too)", i + 1)),
            n => return Err(format!("edit {}: old_text is in the file {n} times: include more lines around it so it is once", i + 1)),
        }
    }
    Ok(text)
}

/// A proposal for `rel` replaces the turn's earlier one for that file.
fn propose(proposals: &mut Vec<Proposal>, p: Proposal) {
    if let Some(k) = proposals.iter().position(|q| q.path == p.path && q.project == p.project) {
        let old = proposals[k].old.clone();
        proposals[k] = Proposal { old, ..p };
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
    match apply_edits(&current, &edits) {
        Ok(new) => {
            let original = p.text(&rel).unwrap_or_default();
            let summary = s(input, "summary").to_string();
            propose(proposals, Proposal { kind: "edit".into(), path: rel.clone(), old: original, new, summary: summary.clone(), ..Proposal::default() });
            let mut o = ok(format!("Proposed to the user as a diff of {rel}; they decide whether to apply it. Don't propose it again: tell them in a sentence what it does."), format!("Proposed a change to {rel}: {summary}"), "state", &rel);
            o.risk = "state";
            o
        }
        Err(e) => err(e, format!("A change to {rel} didn't fit the file"), &rel),
    }
}

fn write_file(input: &Json, p: &Project, proposals: &mut Vec<Proposal>) -> Outcome {
    let rel = match project_path(p.folder, s(input, "path")) {
        Ok(r) => r,
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
    propose(proposals, Proposal { kind: kind.into(), path: rel.clone(), old: original, new: text, summary: summary.clone(), ..Proposal::default() });
    let what = if exists { "a new text for" } else { "the new file" };
    ok(format!("Proposed {what} {rel} to the user as a diff; they decide. Don't propose it again: tell them in a sentence what it does."), format!("Proposed {what} {rel}: {summary}"), "state", &rel)
}

/// A new project's name: its folder's.
fn project_name(name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() || n.len() > 60 || !n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') || n.starts_with('-') {
        return Err(format!("\"{name}\" can't be a project's name: use letters, digits, - and _ (a folder is made with it)"));
    }
    Ok(n.to_string())
}

fn create_project(input: &Json, proposals: &mut Vec<Proposal>) -> Outcome {
    let name = match project_name(s(input, "name")) {
        Ok(n) => n,
        Err(e) => return err(e, "create_project: a name that can't be a folder".into(), s(input, "name")),
    };
    let files = input.get("files").and_then(Json::as_array).cloned().unwrap_or_default();
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
        got.push((rel, s(f, "text").to_string()));
    }
    let main = project_path("", s(input, "main")).unwrap_or_default();
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
        proposals.push(Proposal { kind: "project".into(), path: rel.clone(), old: String::new(), new: text.clone(), summary: summary.clone(), project: name.clone(), main: *rel == main, decision: String::new() });
    }
    let n = got.len();
    if errors.is_empty() {
        let warn = if found.is_empty() { String::new() } else { format!(" Warnings:\n{}", found.join("\n")) };
        let files = if n == 1 { "1 file".to_string() } else { format!("{n} files") };
        ok(format!("The project {name} ({files}) compiles, and is shown to the user to preview and create.{warn} Tell them in a sentence what it does."), format!("Proposed a new project, {name} ({files}): it compiles"), "state", &name)
    } else {
        ok(format!("The project {name} doesn't compile yet — fix these and call create_project again with the whole project:\n{}", found.join("\n")), format!("Proposed a new project, {name}: {} errors to fix", errors.len()), "state", &name)
    }
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
    let mut pre = 0;
    while pre < a.len() && pre < b.len() && a[pre] == b[pre] {
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
        assert!(miss.is_error && miss.content.contains("isn't in the file"));
        let twice = run("propose_edit", &json!({"path": "main.rr", "summary": "x", "edits": [{"old_text": "Caption", "new_text": "y"}]}), &p, &mut props, true);
        assert!(twice.is_error && twice.content.contains("2 times"), "{}", twice.content);
        // (read-only: refused)
        assert!(run("write_file", &json!({"path": "a.rr", "text": "x", "summary": "s"}), &p, &mut props, false).is_error);
        // (a new project: checked; errors go back)
        let np = run("create_project", &json!({"name": "hello-form", "main": "main.rr", "summary": "Hi", "files": [{"path": "main.rr", "text": "CREATE F AS RForm\n  Caption = \"Hello\"\nEND CREATE\nF.ShowModal\n"}]}), &p, &mut props, true);
        assert!(np.content.contains("compiles"), "{}", np.content);
        assert_eq!(props.iter().filter(|q| q.project == "hello-form" && q.main).count(), 1);
        let broken = run("create_project", &json!({"name": "x", "main": "main.rr", "summary": "x", "files": [{"path": "main.rr", "text": "$INCLUDE \"form.inc\"\nF.ShowModal\n"}, {"path": "form.inc", "text": "CREATE F AS RForm\n  Caption = \"x\"\nEND CREATE\nNope x y\n"}]}), &p, &mut props, true);
        assert!(broken.content.contains("doesn't compile"), "{}", broken.content);
        // (its include was found among its own files: only the property is wrong)
        assert!(!broken.content.contains("form.inc: not found") && broken.content.contains("form.inc"), "{}", broken.content);
        assert!(run("create_project", &json!({"name": "../x", "main": "main.rr", "summary": "x", "files": [{"path": "main.rr", "text": "x"}]}), &p, &mut props, true).is_error);
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
    }
}
