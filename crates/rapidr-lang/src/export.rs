//! Everything generated from the registry: `rapidr lang export`.
//!
//! - [`json`]: the whole registry as JSON (the IDE's completion data);
//! - [`vscode_js`]: the VS Code extension's `languageData.js`;
//! - [`web_ide_js`]: the web IDE's `lang-data.js` (until web-ide/ goes);
//! - [`manual`]: the user manual's reference pages (docs/manual/reference);
//! - [`prompt`]: the language section of the AI system prompt.

use crate::*;
use std::fmt::Write as _;

// --- JSON ------------------------------------------------------------------------

/// A JSON string literal.
pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A JSON object's fields, the ones with a value only.
struct Obj(Vec<(&'static str, String)>);

impl Obj {
    fn new() -> Self {
        Obj(Vec::new())
    }
    fn s(mut self, k: &'static str, v: &str) -> Self {
        self.0.push((k, json_str(v)));
        self
    }
    fn s_nonempty(self, k: &'static str, v: &str) -> Self {
        if v.is_empty() { self } else { self.s(k, v) }
    }
    fn opt(self, k: &'static str, v: Option<&str>) -> Self {
        match v {
            Some(v) => self.s(k, v),
            None => self,
        }
    }
    fn b(mut self, k: &'static str, v: bool) -> Self {
        self.0.push((k, v.to_string()));
        self
    }
    fn b_true(self, k: &'static str, v: bool) -> Self {
        if v { self.b(k, true) } else { self }
    }
    fn raw(mut self, k: &'static str, v: String) -> Self {
        self.0.push((k, v));
        self
    }
    fn strs(self, k: &'static str, v: &[&str]) -> Self {
        if v.is_empty() {
            return self;
        }
        let list = v.iter().map(|x| json_str(x)).collect::<Vec<_>>().join(",");
        self.raw(k, format!("[{list}]"))
    }
    fn done(self) -> String {
        format!("{{{}}}", self.0.iter().map(|(k, v)| format!("{}:{v}", json_str(k))).collect::<Vec<_>>().join(","))
    }
}

fn list(items: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(","))
}

fn origin_fields(o: Obj, origin: Origin, from: Option<&str>, runtimes: Runtimes, missing: bool) -> Obj {
    let o = o.s("origin", origin.as_str()).opt("from", from);
    let o = if runtimes != Runtimes::All { o.s("only", runtimes.as_str()) } else { o };
    o.b_true("missing", missing)
}

fn params_json(params: &[Param]) -> String {
    list(params.iter().map(|p| {
        Obj::new()
            .s("name", p.name)
            .s_nonempty("type", p.ty)
            .b_true("optional", p.optional)
            .b_true("byref", p.byref)
            .b_true("variadic", p.variadic)
            .opt("default", p.default)
            .done()
    }))
}

/// A default as the JSON value programs would read (numbers, text,
/// Booleans), with the constant expression kept for the IDE.
fn default_json(d: &DefaultValue) -> String {
    match d {
        DefaultValue::Int(i) => i.to_string(),
        DefaultValue::Float(f) => format!("{f}"),
        DefaultValue::Bool(b) => b.to_string(),
        DefaultValue::Str(s) => json_str(s),
        DefaultValue::Expr(e) => Obj::new().s("expr", e).raw("value", eval_constant(e).map_or("null".into(), |v| v.to_string())).done(),
    }
}

fn property_json(p: &Property) -> String {
    let o = Obj::new().s("name", p.name).s("type", p.ty.as_str()).strs("values", p.values).strs("kinds", p.kinds);
    let o = match &p.default {
        Some(d) => o.raw("default", default_json(d)),
        None => o,
    };
    let o = match p.access {
        Access::ReadWrite => o,
        Access::Read => o.s("access", "read"),
        Access::Write => o.s("access", "write"),
    };
    let o = o.b("design", p.design);
    let o = if p.indexed > 0 { o.raw("indexed", p.indexed.to_string()) } else { o };
    let o = o.s_nonempty("category", p.category);
    let o = origin_fields(o, p.origin, p.from, p.runtimes, p.missing).opt("set", p.set);
    o.s("doc", p.doc).done()
}

fn method_json(m: &Method) -> String {
    let o = Obj::new().s("name", m.name).s("signature", &m.signature()).raw("params", params_json(m.params)).opt("returns", m.returns).b_true("value", m.value);
    origin_fields(o, m.origin, m.from, m.runtimes, m.missing).opt("set", m.set).s("doc", m.doc).done()
}

fn event_json(e: &Event) -> String {
    let o = Obj::new().s("name", e.name).s("signature", &e.signature()).raw("params", params_json(e.params));
    origin_fields(o, e.origin, e.from, e.runtimes, e.missing).opt("set", e.set).s("doc", e.doc).done()
}

fn component_json(c: &Component) -> String {
    let o = Obj::new()
        .s("name", c.name)
        .opt("rapidq", c.rapidq)
        .opt("from", c.from)
        .strs("aliases", c.aliases)
        .s("kind", c.kind.as_str())
        .s("group", c.group)
        .s("origin", c.origin.as_str())
        .s("written", c.written_name())
        .b("visual", c.visual)
        .b("container", c.container);
    let o = match c.size {
        Some((w, h)) => o.raw("size", format!("[{w},{h}]")),
        None => o,
    };
    let o = o.opt("default_event", c.default_event);
    let o = if c.runtimes != Runtimes::All { o.s("only", c.runtimes.as_str()) } else { o };
    o.opt("where", c.where_note)
        .opt("instance_of", c.instance_of)
        .s("doc", c.doc)
        .raw("properties", list(c.properties.iter().map(property_json)))
        .raw("methods", list(c.methods.iter().map(method_json)))
        .raw("events", list(c.events.iter().map(event_json)))
        .done()
}

/// The whole registry as JSON: the IDE's completion data and VS Code's
/// source. Stable field order; fields with no value left out.
pub fn json() -> String {
    let builtins = list(BUILTINS.iter().map(|b| {
        let o = Obj::new()
            .s("name", b.name)
            .s("key", b.key)
            .s_nonempty("syntax", b.syntax)
            .raw("params", params_json(b.params))
            .opt("returns", b.returns)
            .s("group", b.group)
            .b_true("bare", b.bare);
        origin_fields(o, b.origin, None, b.runtimes, b.missing).s("doc", b.doc).done()
    }));
    let statements = list(STATEMENTS.iter().map(|s| {
        Obj::new().s("name", s.name).s("syntax", s.syntax).s("group", s.group).s("origin", s.origin.as_str()).s("doc", s.doc).done()
    }));
    let directives = list(DIRECTIVES.iter().map(|d| Obj::new().s("name", d.name).s("syntax", d.syntax).s("origin", d.origin.as_str()).s("doc", d.doc).done()));
    let keywords = list(KEYWORDS.iter().map(|k| Obj::new().s("name", k.name).s("kind", k.kind).s("origin", k.origin.as_str()).s("doc", k.doc).done()));
    let types = list(TYPE_NAMES.iter().map(|t| Obj::new().s("name", t.name).s("origin", t.origin.as_str()).s("doc", t.doc).done()));
    let constants = list(CONSTANT_GROUPS.iter().map(|g| {
        Obj::new()
            .s("name", g.name)
            .s("source", g.source)
            .s("origin", g.origin.as_str())
            .s("doc", g.doc)
            .raw("constants", list(g.constants.iter().map(|(n, v)| format!("[{},{v}]", json_str(n)))))
            .done()
    }));
    let mut out = String::new();
    out.push_str("{\"registry\":1,\n");
    let _ = writeln!(out, "\"components\":[\n{}\n],", COMPONENTS.iter().map(component_json).collect::<Vec<_>>().join(",\n"));
    let _ = writeln!(out, "\"globals\":[\n{}\n],", GLOBALS.iter().map(component_json).collect::<Vec<_>>().join(",\n"));
    let _ = writeln!(out, "\"items\":[\n{}\n],", ITEMS.iter().map(component_json).collect::<Vec<_>>().join(",\n"));
    let _ = writeln!(out, "\"builtins\":{builtins},");
    let _ = writeln!(out, "\"statements\":{statements},");
    let _ = writeln!(out, "\"directives\":{directives},");
    let _ = writeln!(out, "\"keywords\":{keywords},");
    let _ = writeln!(out, "\"types\":{types},");
    let _ = writeln!(out, "\"constants\":{constants}");
    out.push_str("}\n");
    out
}

// --- the JS language data (VS Code, the web IDE) ---------------------------------

/// A JS single-quoted string.
fn js_str(s: &str) -> String {
    let mut out = String::from("'");
    for c in s.chars() {
        match c {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

/// A snippet's placeholder text for a parameter (VS Code / Monaco syntax).
fn snippet(name: &str, params: &[Param]) -> String {
    let escaped = name.replace('$', "\\\\$");
    let required: Vec<&Param> = params.iter().filter(|p| !p.optional).collect();
    if required.is_empty() {
        return escaped;
    }
    let args = required.iter().enumerate().map(|(i, p)| format!("${{{}:{}}}", i + 1, p.name.trim_end_matches("()"))).collect::<Vec<_>>().join(", ");
    format!("{escaped}({args})")
}

/// Components keyed by every name programs write (RapidR's and RapidQ's),
/// as the old hand-written files had them: lowercase member names, a
/// signature and description per method.
fn component_registry_js() -> String {
    let mut out = String::from("const COMPONENT_REGISTRY = {\n");
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Component) {
        let lower = |n: &str| js_str(&n.to_ascii_lowercase());
        let live = |missing: bool| !missing;
        let props: Vec<String> = c.properties.iter().filter(|p| live(p.missing)).map(|p| lower(p.name)).collect();
        let methods: Vec<String> = c.methods.iter().filter(|m| live(m.missing)).map(|m| lower(m.name)).collect();
        let events: Vec<String> = c.events.iter().filter(|e| live(e.missing)).map(|e| lower(e.name)).collect();
        let _ = writeln!(out, "    {}: {{", c.name);
        let _ = writeln!(out, "        description: {},", js_str(c.doc));
        let _ = writeln!(out, "        rapidq: {},", c.rapidq.map_or("null".into(), js_str));
        let _ = writeln!(out, "        props: [{}],", props.join(", "));
        let _ = writeln!(out, "        methods: [{}],", methods.join(", "));
        let _ = writeln!(out, "        events: [{}],", events.join(", "));
        let _ = writeln!(out, "        methodSignatures: {{");
        for m in c.methods.iter().filter(|m| live(m.missing)) {
            let _ = writeln!(out, "            {}: {{ sig: {}, desc: {} }},", lower(m.name), js_str(&m.signature()), js_str(m.doc));
        }
        let _ = writeln!(out, "        }},");
        let _ = writeln!(out, "        propDocs: {{");
        for p in c.properties.iter().filter(|p| live(p.missing)) {
            let _ = writeln!(out, "            {}: {},", lower(p.name), js_str(p.doc));
        }
        let _ = writeln!(out, "        }},");
        let _ = writeln!(out, "        eventSignatures: {{");
        for e in c.events.iter().filter(|e| live(e.missing)) {
            let _ = writeln!(out, "            {}: {{ sig: {}, desc: {} }},", lower(e.name), js_str(&e.signature()), js_str(e.doc));
        }
        let _ = writeln!(out, "        }},");
        let _ = writeln!(out, "    }},");
    }
    out.push_str("};\n");
    out.push_str("// RapidQ's names for the same components (QBUTTON is RBUTTON).\n");
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Component) {
        for q in c.rapidq.iter().chain(c.aliases.iter()) {
            if !q.eq_ignore_ascii_case(c.name) {
                let _ = writeln!(out, "COMPONENT_REGISTRY[{}] = COMPONENT_REGISTRY.{};", js_str(q), c.name);
            }
        }
    }
    out
}

fn builtins_js() -> String {
    let mut out = String::from("const BUILTIN_FUNCTIONS = [\n");
    for b in BUILTINS.iter().filter(|b| !b.missing) {
        let sig = if b.syntax.is_empty() { format!("{}{}", b.name, params_text(b.params)) } else { b.syntax.to_string() };
        let _ = writeln!(
            out,
            "    {{ name: {}, description: {}, signature: {}, snippet: {} }},",
            js_str(b.name),
            js_str(b.doc),
            js_str(&sig),
            js_str(&snippet(b.name, b.params))
        );
    }
    out.push_str("];\n");
    out
}

fn keywords_js() -> String {
    let mut words: Vec<String> = STATEMENTS.iter().map(|s| s.name.to_string()).collect();
    words.extend(KEYWORDS.iter().map(|k| k.name.to_string()));
    for w in ["END IF", "END SUB", "END FUNCTION", "END SELECT", "END TYPE", "END CREATE", "END WITH", "CASE ELSE", "EXIT FOR", "EXIT DO", "EXIT WHILE", "EXIT SUB", "EXIT FUNCTION", "TRUE", "FALSE"] {
        words.push(w.to_string());
    }
    words.sort();
    words.dedup();
    let mut out = String::from("const KEYWORDS = [\n");
    for chunk in words.chunks(10) {
        let _ = writeln!(out, "    {},", chunk.iter().map(|w| js_str(w)).collect::<Vec<_>>().join(", "));
    }
    out.push_str("];\n\nconst TYPE_KEYWORDS = [\n");
    for t in TYPE_NAMES {
        let _ = writeln!(out, "    {{ name: {}, description: {} }},", js_str(t.name), js_str(t.doc));
    }
    out.push_str("];\n\nconst DIRECTIVES = [\n");
    for d in DIRECTIVES {
        let bare = d.name.trim_start_matches('$');
        let _ = writeln!(out, "    {{ name: {}, description: {}, snippet: {} }},", js_str(bare), js_str(d.doc), js_str(bare));
    }
    out.push_str("];\n");
    out
}

const JS_HEADER: &str = "// Generated by `rapidr lang export` from the language registry (crates/rapidr-lang/data).\n// Do not edit: change the registry and run `rapidr lang export --all`.\n\n";

/// The VS Code extension's language data (utilities/vscodeext/rapidr/src/languageData.js).
pub fn vscode_js() -> String {
    format!(
        "{JS_HEADER}{}\n{}\n{}\nmodule.exports = {{ COMPONENT_REGISTRY, BUILTIN_FUNCTIONS, KEYWORDS, TYPE_KEYWORDS, DIRECTIVES }};\n",
        component_registry_js(),
        builtins_js(),
        keywords_js()
    )
}

/// The web IDE's language data (web-ide/lang-data.js), with its helpers.
pub fn web_ide_js() -> String {
    let names = COMPONENTS
        .iter()
        .filter(|c| c.kind == Kind::Component)
        .flat_map(|c| std::iter::once(c.name).chain(c.rapidq).chain(c.aliases.iter().copied()))
        .map(|n| format!("{n}:{}", js_str(&pretty_name(n))))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{JS_HEADER}{}\n{}\n{}\nexport {{ COMPONENT_REGISTRY, BUILTIN_FUNCTIONS, KEYWORDS, TYPE_KEYWORDS, DIRECTIVES }};\n\n{}",
        component_registry_js(),
        builtins_js(),
        keywords_js(),
        WEB_IDE_HELPERS.replace("@NAMES@", &names)
    )
}

/// `RBUTTON` → `RButton`, `QBUTTON` → `QButton`: the prefix letter and
/// the words of the name.
pub fn pretty_name(upper: &str) -> String {
    if !upper.starts_with(['R', 'Q']) || upper.len() < 2 {
        return upper.to_string();
    }
    const WORDS: &[&str] = &[
        "Form", "MDI", "Button", "Label", "Edit", "Panel", "Check", "Box", "Radio", "Combo", "List", "File", "Dir", "Tree", "Timer", "Image", "Canvas",
        "Header", "Rect", "String", "Grid", "Tab", "Control", "View", "Main", "Menu", "Item", "Popup", "Open", "Save", "Dialog", "Color", "Font", "Tool",
        "Bar", "Status", "Progress", "Rich", "Memo", "Scroll", "Up", "Down", "Date", "Time", "Picker", "Stream", "Track", "Printer", "Registry",
        "Splitter", "SQLite", "MySQL", "Socket", "Server", "Http", "Num", "Data", "Frame", "Plot", "Design", "Surface", "Code", "Editor", "Group",
        "Cool", "Btn", "Oval", "Json", "DX", "Screen", "Sound", "Joystick", "D3D", "Mesh", "Builder", "Face", "Light", "Texture", "Visual", "Wrap",
        "Vector", "Memory", "Bitmap", "Notify", "Icon", "Bevel", "Dig", "Display", "Glass", "CGI", "Com", "Port", "Download", "MIDI", "Wave", "Video",
        "CD", "Audio", "Gauge", "Outline", "Web", "DOM", "JavaScript", "Storage", "Notification", "Geolocation", "Router",
    ];
    let rest = &upper[1..];
    let mut out = upper[..1].to_string();
    let mut i = 0;
    'outer: while i < rest.len() {
        if let Some(w) = WORDS.iter().filter(|w| rest[i..].to_ascii_uppercase().starts_with(&w.to_ascii_uppercase())).max_by_key(|w| w.len()) {
            out.push_str(w);
            i += w.len();
            continue 'outer;
        }
        out.push_str(&rest[i..].to_ascii_lowercase());
        break;
    }
    out
}

const WEB_IDE_HELPERS: &str = r#"// Pretty display name for an upper-case component key (RBUTTON -> RButton).
const _NAME_MAP = { @NAMES@ };
export function prettyComponentName(upper) {
  if (!upper) return upper;
  const u = String(upper).toUpperCase();
  return _NAME_MAP[u] || (u[0] + u.slice(1).toLowerCase());
}

// Heuristic variable→type resolver. Scans DIM/CREATE/AS lines.
export function resolveVariableType(text, varName) {
  if (!text || !varName) return null;
  const v = varName.replace(/[.\[\]()$#%&!]+$/, "");
  // CREATE Foo AS RButton
  let re = new RegExp("\\bCREATE\\s+" + v + "\\s+AS\\s+(\\w+)", "i");
  let m = re.exec(text);
  if (m) return m[1].toUpperCase();
  // DIM Foo AS RButton
  re = new RegExp("\\bDIM\\s+" + v + "\\s+AS\\s+(\\w+)", "i");
  m = re.exec(text);
  if (m) return m[1].toUpperCase();
  // GLOBAL Foo AS RButton
  re = new RegExp("\\bGLOBAL\\s+" + v + "\\s+AS\\s+(\\w+)", "i");
  m = re.exec(text);
  if (m) return m[1].toUpperCase();
  return null;
}
"#;

// --- the user manual's reference pages ---------------------------------------------

const MD_HEADER: &str = "<!-- Generated by `rapidr lang export --manual` from the language registry (crates/rapidr-lang/data). Do not edit: change the registry and run it again. -->\n\n";

fn md(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

fn origin_mark(o: Origin) -> &'static str {
    match o {
        Origin::RapidQ => "",
        Origin::RapidR => " *(RapidR)*",
    }
}

fn where_text(c: &Component) -> String {
    if let Some(w) = c.where_note {
        return w.to_string();
    }
    match c.runtimes {
        Runtimes::All => "everywhere".into(),
        Runtimes::Desktop => "desktop".into(),
        Runtimes::Web => "web".into(),
    }
}

fn groups() -> Vec<&'static str> {
    let mut g: Vec<&str> = Vec::new();
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Component) {
        if !g.contains(&c.group) {
            g.push(c.group);
        }
    }
    g
}

fn components_page() -> String {
    let mut out = String::from(MD_HEADER);
    out.push_str("# Components: RapidQ's and RapidR's names\n\n");
    out.push_str(
        "Every component RapidR creates, by what it is for. A RapidQ name and its R name are the same component (same properties, methods, events and behaviour); a program may use either and mix them freely. A name with no RapidQ name is RapidR's own. Unless the last column says otherwise, a component works in native builds, interpreted programs and the browser. Each component's members: [members.md](members.md).\n\n",
    );
    let mut total = 0;
    let mut rq = 0;
    for g in groups() {
        let _ = write!(out, "## {g}\n\n| RapidR name | RapidQ name | From | Where |\n|---|---|---|---|\n");
        for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Component && c.group == g) {
            total += 1;
            let mut names: Vec<String> = c.rapidq.iter().chain(c.aliases.iter()).map(|n| format!("`{n}`")).collect();
            names.dedup();
            if c.rapidq.is_some() {
                rq += 1;
            }
            let from = if c.rapidq.is_some() { c.from.unwrap_or("RapidQ") } else { "RapidR" };
            let _ = writeln!(out, "| [`{}`](members.md#{}) | {} | {from} | {} |", c.name, c.name.to_ascii_lowercase(), if names.is_empty() { "—".into() } else { names.join(", ") }, where_text(c));
        }
        out.push('\n');
    }
    out.push_str("## RapidQ's include libraries built in\n\nA program that names these (or includes the library) gets RapidR's own implementation, written in BASIC on RapidR's components, on every runtime:\n\n");
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Library) {
        let _ = writeln!(out, "- `{}` (RapidQ's `{}`)", c.name, c.from.unwrap_or(""));
    }
    out.push_str("\n## Not implemented\n\nRapidQ has these objects (RC.EXE knows them); RapidR doesn't yet. Variables of the OLE types compile as generic objects whose methods print a warning:\n\n");
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Planned) {
        let _ = writeln!(out, "- `{}`: {}", c.name, md(c.doc));
    }
    let _ = write!(out, "\n{total} components; {rq} of them have a RapidQ name.\n");
    out
}

fn members_page() -> String {
    let mut out = String::from(MD_HEADER);
    out.push_str("# Components' members\n\n");
    out.push_str(
        "Every property, method and event of every component, from the language registry. *(RapidR)* marks a RapidR extension of a RapidQ component (RapidQ's compiler doesn't know it); *not yet* marks a RapidQ member RapidR doesn't implement yet; *desktop* / *web* one that only that runtime has. A Q name and its R name have the same members.\n\n",
    );
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Component).chain(GLOBALS.iter()).chain(ITEMS.iter()) {
        let title = match c.rapidq {
            Some(q) if q != c.name => format!("{} ({q})", c.name),
            _ => c.name.to_string(),
        };
        let _ = write!(out, "<a id=\"{}\"></a>\n## {title}\n\n{}\n\n", c.name.to_ascii_lowercase(), md(c.doc));
        let flags = |origin: Origin, missing: bool, rt: Runtimes| -> String {
            let mut f = origin_mark(origin).to_string();
            if missing {
                f.push_str(" *(not yet)*");
            }
            match rt {
                Runtimes::Desktop => f.push_str(" *(desktop)*"),
                Runtimes::Web => f.push_str(" *(web)*"),
                Runtimes::All => {}
            }
            if c.origin == Origin::RapidR {
                f = f.replace(" *(RapidR)*", "");
            }
            f
        };
        if !c.properties.is_empty() {
            out.push_str("| Property | Type | Default | |\n|---|---|---|---|\n");
            for p in c.properties {
                let default = match &p.default {
                    Some(DefaultValue::Int(i)) => i.to_string(),
                    Some(DefaultValue::Float(f)) => f.to_string(),
                    Some(DefaultValue::Bool(b)) => (if *b { "True" } else { "False" }).into(),
                    Some(DefaultValue::Str(s)) => format!("`\"{s}\"`"),
                    Some(DefaultValue::Expr(e)) => format!("`{e}`"),
                    None => String::new(),
                };
                let access = match p.access {
                    Access::Read => " (read-only)",
                    Access::Write => " (write-only)",
                    Access::ReadWrite => "",
                };
                let _ = writeln!(out, "| `{}`{}{access} | {} | {default} | {} |", p.name, flags(p.origin, p.missing, p.runtimes), p.ty.as_str(), md(p.doc));
            }
            out.push('\n');
        }
        if !c.methods.is_empty() {
            out.push_str("| Method | |\n|---|---|\n");
            for m in c.methods {
                let _ = writeln!(out, "| `{}`{} | {} |", md(&m.signature()), flags(m.origin, m.missing, m.runtimes), md(m.doc));
            }
            out.push('\n');
        }
        if !c.events.is_empty() {
            out.push_str("| Event | |\n|---|---|\n");
            for e in c.events {
                let _ = writeln!(out, "| `{}`{} | {} |", md(&e.signature()), flags(e.origin, e.missing, e.runtimes), md(e.doc));
            }
            out.push('\n');
        }
    }
    out
}

fn builtins_page() -> String {
    let mut out = String::from(MD_HEADER);
    out.push_str("# Built-in functions and procedures\n\n");
    out.push_str(
        "The builtins every runtime implements (native builds, the interpreter and the web share one list, unit-tested against each runtime's dispatch). Names are not case-sensitive, and a builtin may be written with or without its type suffix: `MID$`, `Mid` and `mid` are the same. *bare*: may be written without parentheses or arguments (`t = TIMER`); *(RapidR)*: RapidQ doesn't have it; *not yet*: RapidQ's, not in RapidR yet. Statements are in [statements.md](statements.md).\n\n",
    );
    let mut gs: Vec<&str> = BUILTINS.iter().map(|b| b.group).collect();
    gs.sort();
    gs.dedup();
    for g in gs {
        let _ = write!(out, "## {g}\n\n");
        if g == "Library functions" {
            out.push_str("RapidQ programs get these from include files or DECLAREs (RapidQ's compiler doesn't know them itself); RapidR doesn't have them yet.\n\n");
        }
        out.push_str("| Builtin | |\n|---|---|\n");
        for b in BUILTINS.iter().filter(|b| b.group == g) {
            let mut flags = origin_mark(b.origin).to_string();
            if b.bare {
                flags.push_str(" *bare*");
            }
            if b.missing {
                flags.push_str(" *(not yet)*");
            }
            if b.runtimes == Runtimes::Desktop {
                flags.push_str(" *(desktop)*");
            }
            let sig = if b.syntax.is_empty() { b.name.to_string() } else { b.syntax.to_string() };
            let _ = writeln!(out, "| `{}`{flags} | {} |", md(&sig), md(b.doc));
        }
        out.push('\n');
    }
    let _ = write!(out, "{} builtins.\n", BUILTINS.len());
    out
}

fn statements_page() -> String {
    let mut out = String::from(MD_HEADER);
    out.push_str("# Statements, directives, keywords and types\n\n*(RapidR)* marks what RapidQ's compiler doesn't accept.\n\n");
    let mut gs: Vec<&str> = STATEMENTS.iter().map(|s| s.group).collect();
    gs.dedup();
    let mut seen = Vec::new();
    for g in gs {
        if seen.contains(&g) {
            continue;
        }
        seen.push(g);
        let _ = write!(out, "## {g}\n\n| Statement | |\n|---|---|\n");
        for s in STATEMENTS.iter().filter(|s| s.group == g) {
            let _ = writeln!(out, "| `{}`{} | {} |", md(s.syntax), origin_mark(s.origin), md(s.doc));
        }
        out.push('\n');
    }
    out.push_str("## Directives\n\n| Directive | |\n|---|---|\n");
    for d in DIRECTIVES {
        let _ = writeln!(out, "| `{}`{} | {} |", md(d.syntax), origin_mark(d.origin), md(d.doc));
    }
    out.push_str("\n## Keywords and operators\n\n| Keyword | |\n|---|---|\n");
    for k in KEYWORDS {
        let _ = writeln!(out, "| `{}`{} | {} |", k.name, origin_mark(k.origin), md(k.doc));
    }
    out.push_str("\n## Types\n\n| Type | |\n|---|---|\n");
    for t in TYPE_NAMES {
        let _ = writeln!(out, "| `{}`{} | {} |", t.name, origin_mark(t.origin), md(t.doc));
    }
    out
}

fn constants_page() -> String {
    let mut out = String::from(MD_HEADER);
    out.push_str("# Constants\n\nRapidQ's RAPIDQ.INC (built in: `$INCLUDE \"RAPIDQ.INC\"` needs no file), its library includes' and RapidR's own. Colours are RapidQ's: `&HBBGGRR`.\n\n");
    for g in CONSTANT_GROUPS {
        let _ = write!(out, "## {}\n\n{}{}\n\n", g.name, md(g.doc), if g.source == "RAPIDQ.INC" { String::new() } else { format!(" (`{}`)", g.source) });
        let items = g.constants.iter().map(|(n, v)| format!("`{n}` = {v}")).collect::<Vec<_>>().join(", ");
        let _ = write!(out, "{items}\n\n");
    }
    out
}

fn data_science_page() -> String {
    let mut out = String::from(MD_HEADER);
    out.push_str("# Data-science members: RNUM, RDATAFRAME, RPLOT\n\n");
    out.push_str(
        "Every method and property the runtimes answer, with where: *desktop* is native builds and interpreted programs (one implementation: ndarray, polars, plotters), *web* the browser's runtime. Names are not case-sensitive, and a method that returns a value can be read like a property on every runtime (`PRINT arr.Sum`). How to use them: [Data science](../data-science.md).\n\n",
    );
    for name in ["RNUM", "RDATAFRAME", "RPLOT"] {
        let c = component(name).expect("data-science component");
        let _ = write!(out, "## {name}\n\n{}\n\n### Properties\n\n| Name | Desktop | Web | |\n|---|:-:|:-:|---|\n", md(c.doc));
        let marks = |rt: Runtimes| match rt {
            Runtimes::All => ("✓", "✓"),
            Runtimes::Desktop => ("✓", "—"),
            Runtimes::Web => ("—", "✓"),
        };
        for p in c.properties.iter().filter(|p| p.set.is_none()) {
            let (d, w) = marks(p.runtimes);
            let _ = writeln!(out, "| `{}` | {d} | {w} | {} |", p.name, md(p.doc));
        }
        out.push_str("\n### Methods\n\n| Name | Desktop | Web | |\n|---|:-:|:-:|---|\n");
        for m in c.methods {
            let (d, w) = marks(m.runtimes);
            let _ = writeln!(out, "| `{}` | {d} | {w} | {} |", md(&m.signature()), md(m.doc));
        }
        out.push('\n');
    }
    out
}

/// Every file the repository keeps generated from the registry: (path from
/// the repository's root, text). `rapidr lang export --all` writes them;
/// tests/generated.rs fails when one is out of date.
pub fn generated_files() -> Vec<(String, String)> {
    let mut files = vec![
        ("utilities/vscodeext/rapidr/src/languageData.js".to_string(), vscode_js()),
        ("web-ide/lang-data.js".to_string(), web_ide_js()),
    ];
    files.extend(manual().into_iter().map(|(name, text)| (format!("docs/manual/reference/{name}"), text)));
    files
}

/// The user manual's reference pages: (file name in docs/manual/reference, text).
pub fn manual() -> Vec<(&'static str, String)> {
    vec![
        ("components.md", components_page()),
        ("members.md", members_page()),
        ("builtins.md", builtins_page()),
        ("statements.md", statements_page()),
        ("constants.md", constants_page()),
        ("data-science.md", data_science_page()),
    ]
}

// --- the AI system prompt's language section -------------------------------------

/// What a model needs to write correct RapidR: compact, every name, no
/// prose beyond one line each.
pub fn prompt() -> String {
    let mut out = String::new();
    out.push_str("# The RapidR language (generated from its registry)\n\n");
    out.push_str("RapidR runs RapidQ BASIC programs and extends them. Names are case-insensitive. A component has a RapidQ name (QBUTTON) and a RapidR name (RBUTTON): the same component; write RapidQ's name for RapidQ's components and the R name for RapidR-only ones. Members marked [R] are RapidR extensions (RapidQ's own compiler refuses them); [missing] ones aren't implemented yet: don't use them.\n\n");
    out.push_str("## Statements\n\n");
    for s in STATEMENTS {
        let _ = writeln!(out, "- {}{}", s.syntax, if s.origin == Origin::RapidR { " [R]" } else { "" });
    }
    out.push_str("\n## Directives\n\n");
    for d in DIRECTIVES {
        let _ = writeln!(out, "- {}{}", d.syntax, if d.origin == Origin::RapidR { " [R]" } else { "" });
    }
    out.push_str("\n## Builtins\n\n");
    for b in BUILTINS.iter().filter(|b| !b.missing) {
        let sig = if b.syntax.is_empty() { b.name } else { b.syntax };
        let _ = writeln!(out, "- {sig}{}: {}", if b.origin == Origin::RapidR { " [R]" } else { "" }, b.doc);
    }
    out.push_str("\n## Components\n\n");
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Component) {
        let names = match c.rapidq {
            Some(q) => format!("{q} / {}", c.name),
            None => c.name.to_string(),
        };
        let _ = writeln!(out, "### {names}{}\n{}", if c.runtimes == Runtimes::Web { " (web only)" } else { "" }, c.doc);
        let tag = |o: Origin| if o == Origin::RapidR && c.origin == Origin::RapidQ { "[R]" } else { "" };
        let props: Vec<String> = c.properties.iter().filter(|p| !p.missing).map(|p| format!("{}{}", p.name, tag(p.origin))).collect();
        let methods: Vec<String> = c.methods.iter().filter(|m| !m.missing).map(|m| format!("{}{}", m.signature(), tag(m.origin))).collect();
        let events: Vec<String> = c.events.iter().filter(|e| !e.missing).map(|e| format!("{}{}", e.signature(), tag(e.origin))).collect();
        if !props.is_empty() {
            let _ = writeln!(out, "Properties: {}", props.join(", "));
        }
        if !methods.is_empty() {
            let _ = writeln!(out, "Methods: {}", methods.join(", "));
        }
        if !events.is_empty() {
            let _ = writeln!(out, "Events: {}", events.join(", "));
        }
        out.push('\n');
    }
    out.push_str("## Global objects\n\n");
    for g in GLOBALS {
        let props: Vec<&str> = g.properties.iter().filter(|p| !p.missing).map(|p| p.name).collect();
        let methods: Vec<String> = g.methods.iter().filter(|m| !m.missing).map(|m| m.signature()).collect();
        let _ = writeln!(out, "- {}: {} {}", g.name, props.join(", "), methods.join(", "));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escapes() {
        assert_eq!(json_str("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
    }

    #[test]
    fn pretty_names() {
        assert_eq!(pretty_name("RBUTTON"), "RButton");
        assert_eq!(pretty_name("RSTRINGGRID"), "RStringGrid");
        assert_eq!(pretty_name("RDXSCREEN"), "RDXScreen");
        assert_eq!(pretty_name("QBUTTON"), "QButton");
        assert_eq!(pretty_name("COMPORT"), "COMPORT");
    }

    #[test]
    fn exports_are_whole() {
        let j = json();
        assert!(j.starts_with("{\"registry\":1"));
        assert!(j.contains("\"name\":\"RBUTTON\""));
        assert!(vscode_js().contains("module.exports"));
        assert!(web_ide_js().contains("export function resolveVariableType"));
        assert!(manual().iter().any(|(n, t)| *n == "components.md" && t.contains("`RFORM`")));
        assert!(prompt().contains("QBUTTON / RBUTTON"));
    }
}
