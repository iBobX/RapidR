//! Reading the web IDE's JSON projects (`"rapidr_project": 1`).
//!
//! A v1 project is one JSON object (see `web-ide/host.js`,
//! `serializeProjectModel`):
//!
//! ```json
//! { "rapidr_project": 1, "name": "Demo",
//!   "modules": [{ "id": "m1", "name": "util", "source": "SUB Hi ..." }],
//!   "assets":  [{ "name": "logo.png", "mime": "image/png", "dataUrl": "data:image/png;base64,..." }],
//!   "forms":   [{ "id": "f1", "name": "Form1", "props": { "caption": "Form1", ... },
//!                 "children": [{ "name": "Button1", "type": "RButton", "props": {...},
//!                                "code": { "handlers": { "OnClick": "Button1_Click" } } }],
//!                 "code": { "handlers": {}, "source": "SUB Button1_Click ..." } }],
//!   "startupForm": "f1" }
//! ```
//!
//! The web IDE turned that into one program (`model.js`, `serializeProject`).
//! The import writes it as a format 2 project instead: one `.rr` file per
//! form (its CREATE block exactly as `serializeForm` wrote it, the event
//! handlers inside the blocks, then the form's code), one per module, the
//! assets as files under `assets/`, and a main file that `$INCLUDE`s them
//! and shows the startup form.

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::error::ProjectError;
use crate::project::{basic_string, FileKind, Project};

/// The result of importing a v1 project: the format 2 project, the files to
/// write (project path, bytes) and what was not carried over (or changed).
#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    pub project: Project,
    pub files: Vec<(String, Vec<u8>)>,
    pub notes: Vec<String>,
}

impl Imported {
    /// The text of one of the imported files, by project path.
    pub fn file_text(&self, path: &str) -> Option<String> {
        self.files
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
    }
}

// ─── An order-keeping JSON value ────────────────────────────────────────
//
// serde_json's `Map` keeps key order only with its `preserve_order`
// feature, and a form's property order is its CREATE block's line order, so
// the import reads JSON into this small value type instead.

#[derive(Debug, Clone, PartialEq)]
enum JVal {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(String),
    Arr(Vec<JVal>),
    Obj(Vec<(String, JVal)>),
}

impl JVal {
    fn get(&self, key: &str) -> Option<&JVal> {
        match self {
            JVal::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            JVal::Str(s) => Some(s),
            _ => None,
        }
    }

    fn entries(&self) -> &[(String, JVal)] {
        match self {
            JVal::Obj(entries) => entries,
            _ => &[],
        }
    }

    fn items(&self) -> &[JVal] {
        match self {
            JVal::Arr(items) => items,
            _ => &[],
        }
    }

    fn is_one(&self) -> bool {
        match self {
            JVal::Int(n) => *n == 1,
            JVal::UInt(n) => *n == 1,
            JVal::Float(f) => *f == 1.0,
            _ => false,
        }
    }

    /// A string field, or "" when missing / null / not a string.
    fn str_field(&self, key: &str) -> &str {
        self.get(key).and_then(JVal::as_str).unwrap_or("")
    }

    /// What JavaScript's `String(v)` gives.
    fn js_string(&self) -> String {
        match self {
            JVal::Null => "null".into(),
            JVal::Bool(b) => b.to_string(),
            JVal::Int(n) => n.to_string(),
            JVal::UInt(n) => n.to_string(),
            JVal::Float(f) => js_number(*f),
            JVal::Str(s) => s.clone(),
            JVal::Arr(items) => items
                .iter()
                .map(|v| match v {
                    JVal::Null => String::new(),
                    v => v.js_string(),
                })
                .collect::<Vec<_>>()
                .join(","),
            JVal::Obj(_) => "[object Object]".into(),
        }
    }
}

/// JavaScript's `String(number)` for a float.
fn js_number(f: f64) -> String {
    if f == 0.0 {
        return "0".into();
    }
    let a = f.abs();
    if (1e-6..1e21).contains(&a) {
        return format!("{f}");
    }
    let s = format!("{f:e}");
    match s.split_once('e') {
        Some((m, e)) if !e.starts_with('-') => format!("{m}e+{e}"),
        _ => s,
    }
}

struct JValVisitor;

impl<'de> Visitor<'de> for JValVisitor {
    type Value = JVal;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }
    fn visit_bool<E>(self, v: bool) -> Result<JVal, E> {
        Ok(JVal::Bool(v))
    }
    fn visit_i64<E>(self, v: i64) -> Result<JVal, E> {
        Ok(JVal::Int(v))
    }
    fn visit_u64<E>(self, v: u64) -> Result<JVal, E> {
        Ok(JVal::UInt(v))
    }
    fn visit_f64<E>(self, v: f64) -> Result<JVal, E> {
        Ok(JVal::Float(v))
    }
    fn visit_str<E>(self, v: &str) -> Result<JVal, E> {
        Ok(JVal::Str(v.to_string()))
    }
    fn visit_string<E>(self, v: String) -> Result<JVal, E> {
        Ok(JVal::Str(v))
    }
    fn visit_unit<E>(self) -> Result<JVal, E> {
        Ok(JVal::Null)
    }
    fn visit_none<E>(self) -> Result<JVal, E> {
        Ok(JVal::Null)
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<JVal, D::Error> {
        JVal::deserialize(d)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<JVal, A::Error> {
        let mut items = Vec::new();
        while let Some(v) = seq.next_element()? {
            items.push(v);
        }
        Ok(JVal::Arr(items))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<JVal, A::Error> {
        let mut entries: Vec<(String, JVal)> = Vec::new();
        while let Some((k, v)) = map.next_entry::<String, JVal>()? {
            // As in JavaScript, a repeated key keeps its first place and
            // takes the last value.
            match entries.iter_mut().find(|(ek, _)| *ek == k) {
                Some(slot) => slot.1 = v,
                None => entries.push((k, v)),
            }
        }
        Ok(JVal::Obj(entries))
    }
}

impl<'de> Deserialize<'de> for JVal {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<JVal, D::Error> {
        d.deserialize_any(JValVisitor)
    }
}

fn parse_json(text: &str) -> Result<JVal, ProjectError> {
    serde_json::from_str(text).map_err(|e| ProjectError::Json(e.to_string()))
}

// ─── model.js's serializer ──────────────────────────────────────────────

/// Properties model.js always writes as strings.
const STR_PROPS: &[&str] = &[
    "caption",
    "text",
    "items",
    "picture",
    "fontname",
    "tooltip",
    "hint",
    "url",
    "alignment",
];

/// model.js's `capitalize`: the first character upper-cased.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// JavaScript's `/^-?\d+(\.\d+)?$/`.
fn looks_numeric(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s, None),
    };
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    digits(int) && frac.is_none_or(digits)
}

/// model.js's `emitVal`.
fn emit_val(key: &str, v: &JVal) -> String {
    match v {
        JVal::Int(_) | JVal::UInt(_) | JVal::Float(_) => v.js_string(),
        JVal::Bool(b) => if *b { "1" } else { "0" }.into(),
        _ if STR_PROPS.contains(&key.to_lowercase().as_str()) => basic_string(&v.js_string()),
        JVal::Str(s) if looks_numeric(s) => s.clone(),
        _ => basic_string(&v.js_string()),
    }
}

/// The lines of a CREATE block's body: model.js's `emitProps`, then the
/// event handlers.
fn body_lines(
    owner: &str,
    props: &JVal,
    handlers: &[(String, String)],
    indent: &str,
    notes: &mut Vec<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (k, v) in props.entries() {
        if matches!(v, JVal::Null) || k == "isStartup" {
            continue;
        }
        if matches!(v, JVal::Arr(_) | JVal::Obj(_)) {
            notes.push(format!(
                "{owner}.{k}: a structured value, written as the web IDE did ({})",
                emit_val(k, v)
            ));
        }
        out.push(format!("{indent}{} = {}", capitalize(k), emit_val(k, v)));
    }
    for (event, sub) in handlers {
        out.push(format!("{indent}{event} = {sub}"));
    }
    out
}

/// The `code.handlers` of a form or widget, in order.
fn handlers_of(owner: &str, obj: &JVal, notes: &mut Vec<String>) -> Vec<(String, String)> {
    let Some(handlers) = obj.get("code").and_then(|c| c.get("handlers")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (event, sub) in handlers.entries() {
        match sub {
            JVal::Str(s) if !s.trim().is_empty() => out.push((event.clone(), s.trim().to_string())),
            JVal::Null => {}
            JVal::Str(_) => {}
            other => notes.push(format!(
                "{owner}.{event}: the handler is not a SUB name ({}); not carried over",
                other.js_string()
            )),
        }
    }
    out
}

/// A form's CREATE block as model.js's `serializeForm` writes it, with the
/// event handlers of the form and its widgets inside their blocks.
fn form_block(form: &JVal, form_name: &str, notes: &mut Vec<String>) -> String {
    let handlers = handlers_of(form_name, form, notes);
    let empty = JVal::Obj(Vec::new());
    let props = form.get("props").unwrap_or(&empty);
    let inner = body_lines(form_name, props, &handlers, "    ", notes).join("\n");
    let mut kids = Vec::new();
    for (i, w) in form
        .get("children")
        .map(JVal::items)
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let (name, ty) = (w.str_field("name"), w.str_field("type"));
        if name.is_empty() || ty.is_empty() {
            notes.push(format!(
                "{form_name}: widget {} has no name or type; not carried over",
                i + 1
            ));
            continue;
        }
        let owner = format!("{form_name}.{name}");
        let handlers = handlers_of(&owner, w, notes);
        let props = w.get("props").unwrap_or(&empty);
        let inner = body_lines(&owner, props, &handlers, "        ", notes).join("\n");
        kids.push(format!(
            "    CREATE {name} AS {ty}\n{inner}\n    END CREATE"
        ));
    }
    let kids = kids.join("\n");
    let kids = if kids.is_empty() {
        String::new()
    } else {
        format!("\n{kids}")
    };
    format!("CREATE {form_name} AS RForm\n{inner}{kids}\nEND CREATE")
}

// ─── Data URLs ──────────────────────────────────────────────────────────

/// Decodes base64 (standard or URL-safe alphabet; whitespace ignored,
/// padding optional).
pub fn decode_base64(text: &str) -> Option<Vec<u8>> {
    fn value(c: u8) -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32)
    }
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits, mut count) = (0u32, 0u32, 0usize);
    let mut padding = false;
    for &c in text.as_bytes() {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'=' {
            padding = true;
            continue;
        }
        if padding {
            return None; // data after the padding
        }
        acc = (acc << 6) | value(c)?;
        bits += 6;
        count += 1;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    if count % 4 == 1 {
        return None; // a lone 6-bit group cannot end a byte
    }
    Some(out)
}

/// Decodes `%XX` escapes (a `%` not followed by two hex digits stays).
fn percent_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = |b: u8| (b as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// The bytes of a `data:` URL (base64 or percent-encoded), or why not.
pub fn decode_data_url(url: &str) -> Result<Vec<u8>, String> {
    let rest = url
        .get(..5)
        .filter(|p| p.eq_ignore_ascii_case("data:"))
        .map(|_| &url[5..])
        .ok_or_else(|| "not a data: URL".to_string())?;
    let (meta, data) = rest
        .split_once(',')
        .ok_or_else(|| "a data: URL without a comma".to_string())?;
    let is_base64 = meta
        .rsplit(';')
        .next()
        .is_some_and(|p| p.trim().eq_ignore_ascii_case("base64"));
    if is_base64 {
        decode_base64(&String::from_utf8_lossy(&percent_decode(data)))
            .ok_or_else(|| "invalid base64 data".to_string())
    } else {
        Ok(percent_decode(data))
    }
}

// ─── File names ─────────────────────────────────────────────────────────

/// A safe file name: letters, digits, `_`, `-`, `.` and space kept, any
/// other character (path separators included) becomes `_`, no `..`, no
/// leading or trailing dots or spaces, no Windows device names; `fallback`
/// when nothing is left.
pub fn sanitize_file_name(name: &str, fallback: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    while s.contains("..") {
        s = s.replace("..", ".");
    }
    let s = s.trim_matches(|c| c == '.' || c == ' ').to_string();
    if s.is_empty() {
        return fallback.to_string();
    }
    let stem = s
        .split('.')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        format!("_{s}")
    } else {
        s
    }
}

/// `name` with `.rr` added when it has no extension.
fn with_rr(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => name.to_string(),
        _ => format!("{name}.rr"),
    }
}

/// Hands out project paths, unique without regard to case.
#[derive(Default)]
struct Names {
    used: HashSet<String>,
}

impl Names {
    /// `dir` + `file`, renamed `stem_2.ext`, `stem_3.ext`... when taken.
    fn claim(&mut self, dir: &str, file: &str, notes: &mut Vec<String>) -> String {
        let (stem, ext) = match file.rfind('.') {
            Some(i) if i > 0 => (&file[..i], &file[i..]),
            _ => (file, ""),
        };
        let mut candidate = format!("{dir}{file}");
        let mut n = 2;
        while !self.used.insert(candidate.to_lowercase()) {
            candidate = format!("{dir}{stem}_{n}{ext}");
            n += 1;
        }
        if candidate != format!("{dir}{file}") {
            notes.push(format!(
                "`{dir}{file}` was taken by another file; written as `{candidate}`"
            ));
        }
        candidate
    }
}

// ─── The import ─────────────────────────────────────────────────────────

/// Whether `text` is a web IDE v1 project (`"rapidr_project": 1`).
pub fn is_v1_json(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with('{')
        && t.contains("rapidr_project")
        && parse_json(t).is_ok_and(|v| v.get("rapidr_project").is_some_and(JVal::is_one))
}

const KNOWN_KEYS: &[&str] = &[
    "rapidr_project",
    "name",
    "modules",
    "assets",
    "forms",
    "startupForm",
];

/// Imports a web IDE v1 project (no file system access): the format 2
/// project, its files and notes about what was not carried over.
pub fn import_v1(json: &str) -> Result<Imported, ProjectError> {
    let root = parse_json(json)?;
    if !matches!(root, JVal::Obj(_)) {
        return Err(ProjectError::InvalidV1("the JSON is not an object".into()));
    }
    match root.get("rapidr_project") {
        Some(v) if v.is_one() => {}
        Some(v) => {
            return Err(ProjectError::InvalidV1(format!(
                "`rapidr_project` is {}, expected 1",
                v.js_string()
            )))
        }
        None => return Err(ProjectError::InvalidV1("no `rapidr_project` key".into())),
    }
    for key in ["modules", "assets", "forms"] {
        if let Some(v) = root.get(key) {
            if !matches!(v, JVal::Arr(_) | JVal::Null) {
                return Err(ProjectError::InvalidV1(format!("`{key}` is not a list")));
            }
        }
    }

    let mut notes = Vec::new();
    for (k, _) in root.entries() {
        if !KNOWN_KEYS.contains(&k.as_str()) {
            notes.push(format!("the project's `{k}` entry is not carried over"));
        }
    }

    let name = match root.str_field("name") {
        "" => "untitled",
        n => n,
    }
    .to_string();
    let mut names = Names::default();
    let main = names.claim(
        "",
        &format!("{}.rr", sanitize_file_name(&name, "untitled")),
        &mut notes,
    );
    let mut project = Project {
        name: name.clone(),
        main: main.clone(),
        ..Project::default()
    };
    project.add_file(&main, FileKind::Module);
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut includes: Vec<String> = Vec::new();

    // Modules.
    for (i, m) in root
        .get("modules")
        .map(JVal::items)
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let fallback = format!("Module{}", i + 1);
        let mname = match m.str_field("name") {
            "" => fallback.as_str(),
            n => n,
        };
        let clean = sanitize_file_name(mname, &fallback);
        if clean != mname {
            notes.push(format!(
                "module `{mname}` is written as `{}`",
                with_rr(&clean)
            ));
        }
        let path = names.claim("", &with_rr(&clean), &mut notes);
        let mut source = m.str_field("source").to_string();
        if !source.is_empty() && !source.ends_with('\n') {
            source.push('\n');
        }
        project.add_file(&path, FileKind::Module);
        includes.push(path.clone());
        files.push((path, source.into_bytes()));
    }

    // Forms.
    let forms = root.get("forms").map(JVal::items).unwrap_or(&[]);
    let mut form_names = Vec::new();
    for (i, f) in forms.iter().enumerate() {
        let fallback = format!("Form{}", i + 1);
        let fname = match f.str_field("name") {
            "" => {
                notes.push(format!("form {} has no name; named `{fallback}`", i + 1));
                fallback.clone()
            }
            n => n.to_string(),
        };
        let clean = sanitize_file_name(&fname, &fallback);
        let path = names.claim("", &format!("{clean}.rr"), &mut notes);
        let mut text = form_block(f, &fname, &mut notes);
        text.push('\n');
        let source = f
            .get("code")
            .map(|c| c.str_field("source"))
            .unwrap_or("")
            .trim();
        if !source.is_empty() {
            text.push('\n');
            text.push_str(source);
            text.push('\n');
        }
        project.add_file(&path, FileKind::Form);
        includes.push(path.clone());
        files.push((path, text.into_bytes()));
        form_names.push(fname);
    }

    // Assets.
    for (i, a) in root
        .get("assets")
        .map(JVal::items)
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let fallback = format!("asset{}", i + 1);
        let aname = match a.str_field("name") {
            "" => fallback.as_str(),
            n => n,
        };
        let url = a.str_field("dataUrl");
        match decode_data_url(url) {
            Ok(bytes) => {
                let clean = sanitize_file_name(aname, &fallback);
                if clean != aname {
                    notes.push(format!("asset `{aname}` is written as `assets/{clean}`"));
                }
                let path = names.claim("assets/", &clean, &mut notes);
                project.add_file(&path, FileKind::Asset);
                files.push((path, bytes));
            }
            Err(why) => notes.push(format!("asset `{aname}` is not carried over: {why}")),
        }
    }

    // The main file: what the web IDE built, as includes.
    let mut main_text = String::from("$APPTYPE WEB\n\n");
    for inc in &includes {
        main_text.push_str(&format!("$INCLUDE \"{inc}\"\n"));
    }
    let startup = root
        .get("startupForm")
        .filter(|v| !matches!(v, JVal::Null))
        .and_then(|id| {
            let id = id.js_string();
            forms
                .iter()
                .position(|f| f.get("id").is_some_and(|fid| fid.js_string() == id))
        })
        .or(if forms.is_empty() { None } else { Some(0) });
    if let Some(i) = startup {
        main_text.push_str(&format!("{}.ShowModal\n", form_names[i]));
    }
    files.insert(0, (main, main_text.into_bytes()));

    Ok(Imported {
        project,
        files,
        notes,
    })
}

/// Imports the v1 project at `path` and writes it beside it: its files and
/// `<name>.rrproj`. Nothing is replaced, except the v1 file itself when it
/// is named `<name>.rrproj`: it is kept as `<name>.v1.json` first (noted).
/// Returns the path of the format 2 project and the import.
pub fn import_v1_file(path: &Path) -> Result<(PathBuf, Imported), ProjectError> {
    let text = std::fs::read_to_string(path).map_err(|e| ProjectError::io(path, e))?;
    let mut imported = import_v1(&text)?;
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let stem = sanitize_file_name(&imported.project.name, "untitled");
    let project_path = dir.join(format!("{stem}.rrproj"));

    // Check everything before writing anything.
    for (rel, _) in &imported.files {
        let target = dir.join(rel);
        if target.exists() {
            return Err(ProjectError::AlreadyExists(target.display().to_string()));
        }
    }
    let mut backup = None;
    if project_path.exists() {
        let same = match (
            std::fs::canonicalize(&project_path),
            std::fs::canonicalize(path),
        ) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        };
        if !same {
            return Err(ProjectError::AlreadyExists(
                project_path.display().to_string(),
            ));
        }
        let kept = dir.join(format!("{stem}.v1.json"));
        if kept.exists() {
            return Err(ProjectError::AlreadyExists(kept.display().to_string()));
        }
        backup = Some(kept);
    }

    if let Some(kept) = &backup {
        std::fs::write(kept, &text).map_err(|e| ProjectError::io(kept, e))?;
        imported.notes.push(format!(
            "the v1 project file was kept as `{}`",
            kept.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    for (rel, bytes) in &imported.files {
        let target = dir.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ProjectError::io(parent, e))?;
        }
        std::fs::write(&target, bytes).map_err(|e| ProjectError::io(&target, e))?;
    }
    imported.project.save(&project_path)?;
    Ok((project_path, imported))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"{
      "rapidr_project": 1,
      "name": "Demo",
      "modules": [
        { "id": "m1", "name": "util", "source": "SUB Hello\n  PRINT \"hi\"\nEND SUB" },
        { "id": "m2", "name": "../evil/x.inc", "source": "" }
      ],
      "assets": [
        { "name": "hello.txt", "mime": "text/plain", "dataUrl": "data:text/plain;base64,SGVsbG8sIHdvcmxkIQ==" },
        { "name": "note.txt", "mime": "text/plain", "dataUrl": "data:,a%20b%2Cc" },
        { "name": "remote.png", "mime": "image/png", "dataUrl": "https://example.com/x.png" }
      ],
      "forms": [
        { "id": "f1", "name": "Form1",
          "props": { "caption": "Say \"hi\"", "width": 480, "height": 320, "startX": 100,
                     "startY": 100, "borderstyle": 2, "isStartup": true, "center": true,
                     "hint": null, "zoom": 1.5, "tag": "12", "color": "&HFF00FF" },
          "children": [
            { "name": "Button1", "type": "RButton",
              "props": { "caption": "OK", "left": 10, "top": 20, "text": 5, "enabled": false },
              "code": { "handlers": { "OnClick": "Button1_Click", "OnMouseDown": "" } } },
            { "name": "Label1", "type": "RLabel",
              "props": { "caption": "1234", "parent": "Panel1" },
              "code": { "handlers": {} } }
          ],
          "code": { "handlers": { "OnClose": "Form1_Close" },
                    "source": "\nSUB Button1_Click\n  ShowMessage \"clicked\"\nEND SUB\n" } },
        { "id": "f2", "name": "About",
          "props": { "caption": "About", "width": 200 },
          "children": [],
          "code": { "handlers": {}, "source": "" } }
      ],
      "startupForm": "f2",
      "theme": "dark"
    }"##;

    #[test]
    fn detects_v1() {
        assert!(is_v1_json(SAMPLE));
        assert!(is_v1_json("  {\"rapidr_project\": 1}"));
        assert!(!is_v1_json("{\"rapidr_project\": 2}"));
        assert!(!is_v1_json("format = 2\n"));
        assert!(!is_v1_json("{\"name\": \"rapidr_project\"}"));
    }

    #[test]
    fn imports_two_forms() {
        let imp = import_v1(SAMPLE).unwrap();
        let p = &imp.project;
        assert_eq!(p.name, "Demo");
        assert_eq!(p.main, "Demo.rr");
        let paths: Vec<&str> = imp.files.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            paths,
            [
                "Demo.rr",
                "util.rr",
                "_evil_x.inc",
                "Form1.rr",
                "About.rr",
                "assets/hello.txt",
                "assets/note.txt"
            ]
        );
        let project_paths: Vec<&str> = p.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(project_paths, paths);
        assert_eq!(p.files_of_kind(FileKind::Form).len(), 2);
        assert_eq!(p.files_of_kind(FileKind::Module).len(), 3);
        assert_eq!(p.files_of_kind(FileKind::Asset).len(), 2);

        assert_eq!(
            imp.file_text("Demo.rr").unwrap(),
            "$APPTYPE WEB\n\n\
             $INCLUDE \"util.rr\"\n\
             $INCLUDE \"_evil_x.inc\"\n\
             $INCLUDE \"Form1.rr\"\n\
             $INCLUDE \"About.rr\"\n\
             About.ShowModal\n"
        );
        assert_eq!(
            imp.file_text("util.rr").unwrap(),
            "SUB Hello\n  PRINT \"hi\"\nEND SUB\n"
        );
        assert_eq!(
            imp.file_text("Form1.rr").unwrap(),
            "CREATE Form1 AS RForm\n\
             \x20   Caption = \"Say \" + CHR$(34) + \"hi\" + CHR$(34) + \"\"\n\
             \x20   Width = 480\n\
             \x20   Height = 320\n\
             \x20   StartX = 100\n\
             \x20   StartY = 100\n\
             \x20   Borderstyle = 2\n\
             \x20   Center = 1\n\
             \x20   Zoom = 1.5\n\
             \x20   Tag = 12\n\
             \x20   Color = \"&HFF00FF\"\n\
             \x20   OnClose = Form1_Close\n\
             \x20   CREATE Button1 AS RButton\n\
             \x20       Caption = \"OK\"\n\
             \x20       Left = 10\n\
             \x20       Top = 20\n\
             \x20       Text = 5\n\
             \x20       Enabled = 0\n\
             \x20       OnClick = Button1_Click\n\
             \x20   END CREATE\n\
             \x20   CREATE Label1 AS RLabel\n\
             \x20       Caption = \"1234\"\n\
             \x20       Parent = \"Panel1\"\n\
             \x20   END CREATE\n\
             END CREATE\n\
             \n\
             SUB Button1_Click\n  ShowMessage \"clicked\"\nEND SUB\n"
        );
        assert_eq!(
            imp.file_text("About.rr").unwrap(),
            "CREATE About AS RForm\n    Caption = \"About\"\n    Width = 200\nEND CREATE\n"
        );
        assert_eq!(imp.file_text("assets/hello.txt").unwrap(), "Hello, world!");
        assert_eq!(imp.file_text("assets/note.txt").unwrap(), "a b,c");

        let notes = imp.notes.join("\n");
        assert!(notes.contains("`theme`"), "{notes}");
        assert!(notes.contains("remote.png"), "{notes}");
        assert!(notes.contains("evil_x.inc"), "{notes}");
    }

    #[test]
    fn startup_defaults_to_first_form_and_names_are_unique() {
        let json = r#"{"rapidr_project":1,"name":"Form1",
            "modules":[{"name":"form1","source":"x = 1"}],
            "forms":[{"id":"a","name":"Form1","props":{},"children":[]}]}"#;
        let imp = import_v1(json).unwrap();
        let paths: Vec<&str> = imp.files.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(paths, ["Form1.rr", "form1_2.rr", "Form1_3.rr"]);
        let main = imp.file_text("Form1.rr").unwrap();
        assert!(
            main.ends_with("$INCLUDE \"Form1_3.rr\"\nForm1.ShowModal\n"),
            "{main}"
        );
        // An empty props object: serializeForm's empty body line.
        assert_eq!(
            imp.file_text("Form1_3.rr").unwrap(),
            "CREATE Form1 AS RForm\n\nEND CREATE\n"
        );
        assert_eq!(imp.notes.len(), 2, "{:?}", imp.notes);
    }

    #[test]
    fn rejects_non_v1() {
        assert!(matches!(import_v1("[1]"), Err(ProjectError::InvalidV1(_))));
        assert!(matches!(import_v1("{}"), Err(ProjectError::InvalidV1(_))));
        assert!(matches!(
            import_v1("{\"rapidr_project\": 2}"),
            Err(ProjectError::InvalidV1(_))
        ));
        assert!(matches!(import_v1("{"), Err(ProjectError::Json(_))));
        assert!(matches!(
            import_v1("{\"rapidr_project\": 1, \"forms\": 3}"),
            Err(ProjectError::InvalidV1(_))
        ));
        let empty = import_v1("{\"rapidr_project\": 1}").unwrap();
        assert_eq!(empty.project.name, "untitled");
        assert_eq!(empty.file_text("untitled.rr").unwrap(), "$APPTYPE WEB\n\n");
    }

    #[test]
    fn values_like_model_js() {
        assert_eq!(emit_val("width", &JVal::Int(-3)), "-3");
        assert_eq!(emit_val("x", &JVal::Float(100.0)), "100");
        assert_eq!(emit_val("x", &JVal::Float(1e21)), "1e+21");
        assert_eq!(emit_val("x", &JVal::Float(1e-7)), "1e-7");
        assert_eq!(emit_val("visible", &JVal::Bool(true)), "1");
        assert_eq!(emit_val("Caption", &JVal::Str("12".into())), "\"12\"");
        assert_eq!(emit_val("caption", &JVal::Bool(false)), "0");
        assert_eq!(emit_val("tag", &JVal::Str("-1.25".into())), "-1.25");
        assert_eq!(emit_val("tag", &JVal::Str("1.".into())), "\"1.\"");
        assert_eq!(
            emit_val("tag", &JVal::Str("\"".into())),
            "\"\" + CHR$(34) + \"\""
        );
        assert_eq!(
            emit_val(
                "items",
                &JVal::Arr(vec![JVal::Str("a".into()), JVal::Null, JVal::Int(2)])
            ),
            "\"a,,2\""
        );
        assert_eq!(capitalize("startX"), "StartX");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn base64_and_data_urls() {
        assert_eq!(decode_base64("TWFu").unwrap(), b"Man");
        assert_eq!(decode_base64("TWE=").unwrap(), b"Ma");
        assert_eq!(decode_base64("TQ==").unwrap(), b"M");
        assert_eq!(decode_base64("TQ").unwrap(), b"M");
        assert_eq!(decode_base64("").unwrap(), b"");
        assert_eq!(decode_base64("SGVs\nbG8=").unwrap(), b"Hello");
        assert_eq!(decode_base64("-_8=").unwrap(), [0xfb, 0xff]);
        assert_eq!(decode_base64("+/8=").unwrap(), [0xfb, 0xff]);
        assert!(decode_base64("T").is_none());
        assert!(decode_base64("TQ==TQ").is_none());
        assert!(decode_base64("T*Q=").is_none());
        assert_eq!(decode_data_url("DATA:;BASE64,AAEC").unwrap(), [0, 1, 2]);
        assert_eq!(
            decode_data_url("data:text/plain;charset=utf-8,%E2%82%AC%").unwrap(),
            "€%".as_bytes()
        );
        assert!(decode_data_url("data:abc").is_err());
        assert!(decode_data_url("blob:x").is_err());
    }

    #[test]
    fn file_names() {
        assert_eq!(sanitize_file_name("../../etc/passwd", "x"), "_._etc_passwd");
        assert_eq!(sanitize_file_name("a\\b:c", "x"), "a_b_c");
        assert_eq!(sanitize_file_name("..", "x"), "x");
        assert_eq!(sanitize_file_name("  My Form . ", "x"), "My Form");
        assert_eq!(sanitize_file_name("Año-1_b.rr", "x"), "Año-1_b.rr");
        assert_eq!(sanitize_file_name("con.rr", "x"), "_con.rr");
        assert_eq!(sanitize_file_name("COM1", "x"), "_COM1");
        assert_eq!(sanitize_file_name("Console", "x"), "Console");
        assert_eq!(with_rr("util"), "util.rr");
        assert_eq!(with_rr("util.bas"), "util.bas");
    }

    #[test]
    fn import_to_disk() {
        let dir = std::env::temp_dir().join(format!("rapidr-project-v1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // A v1 file named like the project it becomes.
        let v1 = dir.join("Demo.rrproj");
        std::fs::write(&v1, SAMPLE).unwrap();
        let (proj, imp) = import_v1_file(&v1).unwrap();
        assert_eq!(proj, v1);
        assert_eq!(
            std::fs::read_to_string(dir.join("Demo.v1.json")).unwrap(),
            SAMPLE
        );
        assert!(imp.notes.iter().any(|n| n.contains("Demo.v1.json")));
        let loaded = Project::load(&proj).unwrap();
        assert_eq!(loaded, imp.project);
        assert_eq!(
            std::fs::read(dir.join("assets/hello.txt")).unwrap(),
            b"Hello, world!"
        );
        assert!(dir.join("Form1.rr").exists());
        // A second import would replace files: refused, nothing written.
        let again = dir.join("again.json");
        std::fs::write(&again, SAMPLE).unwrap();
        assert!(matches!(
            import_v1_file(&again),
            Err(ProjectError::AlreadyExists(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
