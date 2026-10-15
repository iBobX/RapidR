//! Compiles data/*.toml into static tables ($OUT_DIR/registry.rs), checking
//! everything a table can't: names unique, types known, enum values and
//! constant defaults real constants, parameter lists well formed, default
//! events real events, sets defined, the AI facets' vocabularies (effects,
//! risk levels, units, the range grammar and the properties a range names:
//! docs/ai-architecture.md §2.2). A mistake in the data fails the build
//! with the file and the entry.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

type Res<T> = Result<T, String>;

// The AI facets' vocabularies (EFFECT_NAMES, EFFECT_RISK, RISK_NAMES, UNITS),
// shared with the library's src/ai.rs.
include!("src/ai_vocab.rs");

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("data");
    println!("cargo:rerun-if-changed={}", dir.display());
    let code = match build(&dir) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("rapidr-lang data: {e}");
            panic!("rapidr-lang data: {e}");
        }
    };
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("registry.rs");
    fs::write(out, code).unwrap();
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for p in entries {
        println!("cargo:rerun-if-changed={}", p.display());
        if p.is_dir() {
            files(&p, out);
        } else if p.extension().is_some_and(|e| e == "toml") {
            out.push(p);
        }
    }
}

fn load(path: &Path) -> Res<Table> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.parse::<Table>().map_err(|e| format!("{}: {e}", path.display()))
}

// --- small readers -----------------------------------------------------------

fn s<'a>(t: &'a Table, k: &str) -> Option<&'a str> {
    t.get(k).and_then(Value::as_str)
}

fn req<'a>(t: &'a Table, k: &str, at: &str) -> Res<&'a str> {
    s(t, k).ok_or_else(|| format!("{at}: `{k}` missing"))
}

fn b(t: &Table, k: &str) -> Option<bool> {
    t.get(k).and_then(Value::as_bool)
}

fn arr<'a>(t: &'a Table, k: &str) -> &'a [Value] {
    t.get(k).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn strs(t: &Table, k: &str, at: &str) -> Res<Vec<String>> {
    arr(t, k).iter().map(|v| v.as_str().map(str::to_string).ok_or_else(|| format!("{at}: `{k}` must be strings"))).collect()
}

fn keys_allowed(t: &Table, allowed: &[&str], at: &str) -> Res<()> {
    for k in t.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(format!("{at}: unknown key `{k}` (one of: {})", allowed.join(", ")));
        }
    }
    Ok(())
}

fn lit(s: &str) -> String {
    format!("{s:?}")
}

fn opt(s: Option<&str>) -> String {
    match s {
        Some(s) => format!("Some({s:?})"),
        None => "None".into(),
    }
}

fn origin(t: &Table, default: &str, at: &str) -> Res<String> {
    match s(t, "origin").unwrap_or(default) {
        "rapidq" => Ok("Origin::RapidQ".into()),
        "rapidr" => Ok("Origin::RapidR".into()),
        o => Err(format!("{at}: origin `{o}` (rapidq or rapidr)")),
    }
}

fn runtimes(t: &Table, at: &str) -> Res<String> {
    match s(t, "only") {
        None => Ok("Runtimes::All".into()),
        Some("desktop") => Ok("Runtimes::Desktop".into()),
        Some("web") => Ok("Runtimes::Web".into()),
        Some(o) => Err(format!("{at}: only `{o}` (desktop or web)")),
    }
}

// --- the AI facets (docs/ai-architecture.md §2.2) ------------------------------------

/// A vocabulary word as its enum variant: `file-read` → `FileRead`.
fn variant(word: &str) -> String {
    word.split('-')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
        })
        .collect()
}

/// `effects = [...]`: the code of the list (`&[Effect::FileWrite]`) and the
/// highest risk among them (an index in RISK_NAMES), `None` when not given.
fn effects(t: &Table, at: &str) -> Res<(String, Option<u8>)> {
    let names = strs(t, "effects", at)?;
    if names.is_empty() {
        if t.contains_key("effects") {
            return Err(format!("{at}: `effects` is empty (say [\"none\"] for no effect)"));
        }
        return Ok(("&[]".into(), None));
    }
    if names.len() > 1 && names.iter().any(|n| n == "none") {
        return Err(format!("{at}: effect `none` stands alone"));
    }
    let mut code = String::from("&[");
    let mut seen = Vec::new();
    let mut max = 0u8;
    for n in &names {
        let i = EFFECT_NAMES.iter().position(|e| e == n).ok_or_else(|| format!("{at}: effect `{n}` (one of {})", EFFECT_NAMES.join(", ")))?;
        if seen.contains(&i) {
            return Err(format!("{at}: effect `{n}` twice"));
        }
        seen.push(i);
        max = max.max(EFFECT_RISK[i]);
        write!(code, "Effect::{}, ", variant(n)).unwrap();
    }
    code.push(']');
    Ok((code, Some(max)))
}

/// `risk = "…"`: only to raise the effects' risk, so the effects are said too.
fn risk(t: &Table, effects_risk: Option<u8>, at: &str) -> Res<String> {
    let Some(r) = s(t, "risk") else { return Ok("None".into()) };
    let i = RISK_NAMES.iter().position(|x| *x == r).ok_or_else(|| format!("{at}: risk `{r}` (one of {})", RISK_NAMES.join(", ")))? as u8;
    match effects_risk {
        None => Err(format!("{at}: `risk` raises the effects' risk: say the `effects` too")),
        Some(m) if i < m => Err(format!("{at}: risk `{r}` is lower than its effects' (`{}`)", RISK_NAMES[m as usize])),
        Some(_) => Ok(format!("Some(Risk::{})", variant(r))),
    }
}

/// A `range`: `any`, or `lo..hi` with either bound left out, each a number or
/// the name of another property of the component. The names it uses.
fn range_refs(r: &str, at: &str) -> Res<Vec<String>> {
    if r == "any" {
        return Ok(Vec::new());
    }
    let bad = || format!("{at}: range `{r}` (\"lo..hi\", either bound optional, a number or a property's name; or \"any\")");
    let (lo, hi) = r.split_once("..").ok_or_else(bad)?;
    let (lo, hi) = (lo.trim(), hi.trim());
    if lo.is_empty() && hi.is_empty() {
        return Err(bad());
    }
    let mut refs = Vec::new();
    for b in [lo, hi] {
        if b.is_empty() || b.parse::<f64>().is_ok() {
            continue;
        }
        if !b.starts_with(|c: char| c.is_ascii_alphabetic()) || !b.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(bad());
        }
        refs.push(b.to_string());
    }
    if let (Ok(l), Ok(h)) = (lo.parse::<f64>(), hi.parse::<f64>()) {
        if l > h {
            return Err(format!("{at}: range `{r}`: its low end is above its high end"));
        }
    }
    Ok(refs)
}

fn unit(u: &str, at: &str) -> Res<()> {
    if UNITS.contains(&u) {
        Ok(())
    } else {
        Err(format!("{at}: unit `{u}` (one of {})", UNITS.join(", ")))
    }
}

// --- parameters ----------------------------------------------------------------

/// `[BYREF] Name [AS Type] [= default]`, optional ones in `[…]`, a trailing
/// `...` for any number more (SUBI).
fn params(text: &str, at: &str) -> Res<String> {
    let mut out = String::from("&[");
    if text.trim().is_empty() {
        return Ok("&[]".into());
    }
    for part in text.split(',') {
        let mut p = part.trim();
        let variadic = p.ends_with("...");
        if variadic {
            p = p.trim_end_matches("...").trim();
        }
        let optional = p.starts_with('[');
        if optional {
            p = p.trim_start_matches('[').trim_end_matches(']').trim();
        }
        let (p, default) = match p.split_once('=') {
            Some((a, d)) => (a.trim(), Some(d.trim().to_string())),
            None => (p, None),
        };
        let words: Vec<&str> = p.split_whitespace().collect();
        let (byref, words) = match words.first() {
            Some(w) if w.eq_ignore_ascii_case("BYREF") => (true, &words[1..]),
            Some(w) if w.eq_ignore_ascii_case("BYVAL") => (false, &words[1..]),
            _ => (false, &words[..]),
        };
        let (name, ty) = match words {
            [n] => (*n, ""),
            [n, a, t] if a.eq_ignore_ascii_case("AS") => (*n, *t),
            _ => return Err(format!("{at}: parameter `{}` isn't `[BYREF] Name [AS Type]`", part.trim())),
        };
        if !name.trim_end_matches("()").chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || "$%&#!".contains(c)) {
            return Err(format!("{at}: parameter name `{name}`"));
        }
        write!(
            out,
            "Param {{ name: {}, ty: {}, optional: {optional}, byref: {byref}, variadic: {variadic}, default: {} }}, ",
            lit(name),
            lit(&ty.to_ascii_uppercase()),
            opt(default.as_deref())
        )
        .unwrap();
    }
    out.push(']');
    Ok(out)
}

// --- the registry being built ------------------------------------------------------

/// A glossary entry: what every member of that name means (its own `doc` and
/// `category` win), and for a property its `range` and `unit`.
#[derive(Clone, Default)]
struct GlossaryEntry {
    doc: String,
    category: String,
    range: Option<String>,
    unit: Option<String>,
}

struct Glossary {
    // (kind, lower name) → the entry
    entries: HashMap<(String, String), GlossaryEntry>,
}

impl Glossary {
    fn doc(&self, kind: &str, name: &str) -> (String, String) {
        let e = self.entry(kind, name);
        (e.doc, e.category)
    }

    fn entry(&self, kind: &str, name: &str) -> GlossaryEntry {
        self.entries.get(&(kind.to_string(), name.to_ascii_lowercase())).cloned().unwrap_or_default()
    }
}

struct Ctx {
    constants: HashMap<String, i64>,
    glossary: Glossary,
    sets: HashMap<String, Table>,
    /// items.toml's objects (a type = "item" property's kinds).
    items: Vec<String>,
}

const TYPES: &[(&str, &str)] = &[
    ("int", "Int"),
    ("float", "Float"),
    ("string", "String"),
    ("bool", "Bool"),
    ("color", "Color"),
    ("enum", "Enum"),
    ("set", "Set"),
    ("font", "Font"),
    ("component", "Component"),
    ("picture", "Picture"),
    ("resource", "Resource"),
    ("item", "Item"),
    ("any", "Any"),
];

/// One property: its code, and the property names its range uses (checked
/// against the component's once all its properties are known).
fn property(ctx: &Ctx, p: &Table, family: &str, set: Option<&str>, at: &str) -> Res<(String, Vec<String>)> {
    keys_allowed(
        p,
        &[
            "name", "type", "values", "kinds", "default", "access", "design", "indexed", "category", "origin", "from", "only", "missing", "editor", "doc", "range",
            "unit", "effects", "secret",
        ],
        at,
    )?;
    let name = req(p, "name", at)?;
    let at = &format!("{at}, property {name}");
    let ty_s = s(p, "type").unwrap_or("any");
    let ty = TYPES.iter().find(|(k, _)| *k == ty_s).ok_or_else(|| format!("{at}: type `{ty_s}`"))?.1;
    let values = strs(p, "values", at)?;
    // (a string's values are the words it is offered — RAI's Provider —, an
    // enum's or a set's the constants they are)
    for v in values.iter().filter(|_| ty != "String") {
        if !ctx.constants.contains_key(&v.to_ascii_lowercase()) {
            return Err(format!("{at}: value `{v}` isn't a constant (constants.toml)"));
        }
    }
    if matches!(ty, "Enum" | "Set") && values.is_empty() {
        return Err(format!("{at}: an {ty_s} needs `values`"));
    }
    let kinds = strs(p, "kinds", at)?;
    if ty == "Item" {
        for k in &kinds {
            if !ctx.items.contains(k) {
                return Err(format!("{at}: item kind `{k}` isn't in items.toml"));
            }
        }
        if kinds.is_empty() {
            return Err(format!("{at}: an item property names its kinds (items.toml)"));
        }
    }
    let default = match p.get("default") {
        None => "None".to_string(),
        Some(Value::Integer(i)) => format!("Some(DefaultValue::Int({i}))"),
        Some(Value::Float(f)) => format!("Some(DefaultValue::Float({f:?}))"),
        Some(Value::Boolean(v)) => format!("Some(DefaultValue::Bool({v}))"),
        Some(Value::String(v)) if matches!(ty, "String" | "Picture" | "Any") => format!("Some(DefaultValue::Str({}))", lit(v)),
        Some(Value::String(v)) => {
            // (a constant, or constants added: `akLeft + akTop`)
            for part in v.split(['+', '|']).map(str::trim) {
                if !ctx.constants.contains_key(&part.to_ascii_lowercase()) && part.parse::<i64>().is_err() {
                    return Err(format!("{at}: default `{v}`: `{part}` isn't a constant"));
                }
            }
            if ty == "Enum" && !values.iter().any(|x| x.eq_ignore_ascii_case(v)) {
                return Err(format!("{at}: default `{v}` isn't one of its values"));
            }
            format!("Some(DefaultValue::Expr({}))", lit(v))
        }
        Some(other) => return Err(format!("{at}: default {other}")),
    };
    let access = match s(p, "access") {
        None => "Access::ReadWrite",
        Some("read") => "Access::Read",
        Some("write") => "Access::Write",
        Some(a) => return Err(format!("{at}: access `{a}` (read or write)")),
    };
    let indexed = p.get("indexed").and_then(Value::as_integer).unwrap_or(0);
    if !(0..=2).contains(&indexed) {
        return Err(format!("{at}: indexed {indexed}"));
    }
    let design = b(p, "design").unwrap_or(access == "Access::ReadWrite" && indexed == 0);
    let (gdoc, gcat) = ctx.glossary.doc("property", name);
    let doc = s(p, "doc").map(str::to_string).filter(|d| !d.is_empty()).unwrap_or(gdoc);
    let category = s(p, "category").map(str::to_string).unwrap_or(gcat);
    let editor = s(p, "editor");
    if let Some(e) = editor {
        if !EDITORS.contains(&e) {
            return Err(format!("{at}: editor `{e}` (one of {})", EDITORS.join(", ")));
        }
    }
    // The AI facets: a numeric property's range and unit (its own, else the
    // glossary's), what setting it does when that reaches outside, secrets.
    let numeric = matches!(ty, "Int" | "Float");
    let gloss = ctx.glossary.entry("property", name);
    let own_range = s(p, "range");
    let own_unit = s(p, "unit");
    if !numeric && (own_range.is_some() || own_unit.is_some()) {
        return Err(format!("{at}: `range` and `unit` are for int and float properties"));
    }
    let range = own_range.map(str::to_string).or(if numeric { gloss.range } else { None });
    let range_names = match &range {
        Some(r) => range_refs(r, at)?,
        None => Vec::new(),
    };
    let unit_v = own_unit.map(str::to_string).or(if numeric { gloss.unit } else { None });
    if let Some(u) = &unit_v {
        unit(u, at)?;
    }
    let (effects_code, _) = effects(p, at)?;
    let code = format!(
        "Property {{ name: {}, ty: Type::{ty}, values: &[{}], kinds: &[{}], default: {default}, access: {access}, design: {design}, indexed: {indexed}, category: {}, origin: {}, from: {}, runtimes: {}, missing: {}, set: {}, editor: {}, doc: {}, range: {}, unit: {}, effects: {effects_code}, secret: {} }}",
        lit(name),
        values.iter().map(|v| lit(v)).collect::<Vec<_>>().join(", "),
        kinds.iter().map(|v| lit(v)).collect::<Vec<_>>().join(", "),
        lit(&category),
        origin(p, family, at)?,
        opt(s(p, "from")),
        runtimes(p, at)?,
        b(p, "missing").unwrap_or(false),
        opt(set),
        opt(editor),
        lit(&doc),
        opt(range.as_deref()),
        opt(unit_v.as_deref()),
        b(p, "secret").unwrap_or(false),
    );
    Ok((code, range_names))
}

/// The inspector's editors a property may ask for beyond its type's
/// (RPropertyInspector, docs/ide-components.md §3.4).
const EDITORS: &[&str] = &["strings", "columns", "file", "picture", "multiline", "sql", "expression", "secret"];

fn method(ctx: &Ctx, m: &Table, family: &str, set: Option<&str>, at: &str) -> Res<String> {
    keys_allowed(m, &["name", "params", "returns", "value", "origin", "from", "only", "missing", "test", "doc", "effects", "risk", "example"], at)?;
    let name = req(m, "name", at)?;
    let at = &format!("{at}, method {name}");
    let (effects_code, effects_risk) = effects(m, at)?;
    let risk_code = risk(m, effects_risk, at)?;
    let test = match s(m, "test") {
        None => "Test::Call".to_string(),
        Some(t) if t.starts_with("skip") => format!("Test::Skip({})", lit(t.trim_start_matches("skip").trim_start_matches(':').trim())),
        Some(t) if t.starts_with("args:") => format!("Test::Args({})", lit(t["args:".len()..].trim())),
        Some(t) => return Err(format!("{at}: test `{t}` (skip: reason, or args: …)")),
    };
    let (gdoc, _) = ctx.glossary.doc("method", name);
    let doc = s(m, "doc").map(str::to_string).filter(|d| !d.is_empty()).unwrap_or(gdoc);
    Ok(format!(
        "Method {{ name: {}, params: {}, returns: {}, value: {}, origin: {}, from: {}, runtimes: {}, missing: {}, test: {test}, set: {}, doc: {}, effects: {effects_code}, risk: {risk_code}, example: {} }}",
        lit(name),
        params(s(m, "params").unwrap_or(""), at)?,
        opt(s(m, "returns")),
        b(m, "value").unwrap_or(false),
        origin(m, family, at)?,
        opt(s(m, "from")),
        runtimes(m, at)?,
        b(m, "missing").unwrap_or(false),
        opt(set),
        lit(&doc),
        lit(s(m, "example").unwrap_or("")),
    ))
}

fn event(ctx: &Ctx, e: &Table, family: &str, set: Option<&str>, at: &str) -> Res<String> {
    keys_allowed(e, &["name", "params", "origin", "from", "only", "missing", "doc"], at)?;
    let name = req(e, "name", at)?;
    let at = &format!("{at}, event {name}");
    let (gdoc, _) = ctx.glossary.doc("event", name);
    let doc = s(e, "doc").map(str::to_string).filter(|d| !d.is_empty()).unwrap_or(gdoc);
    Ok(format!(
        "Event {{ name: {}, params: {}, origin: {}, from: {}, runtimes: {}, missing: {}, set: {}, doc: {} }}",
        lit(name),
        params(s(e, "params").unwrap_or(""), at)?,
        origin(e, family, at)?,
        opt(s(e, "from")),
        runtimes(e, at)?,
        b(e, "missing").unwrap_or(false),
        opt(set),
        lit(&doc),
    ))
}

fn tables<'a>(t: &'a Table, k: &str, at: &str) -> Res<Vec<&'a Table>> {
    arr(t, k).iter().map(|v| v.as_table().ok_or_else(|| format!("{at}: `{k}` entries must be tables"))).collect()
}

/// One component or global object.
fn component(ctx: &Ctx, c: &Table, group: &str, global: bool, at: &str) -> Res<(String, Vec<String>, String, bool)> {
    keys_allowed(
        c,
        &[
            "name", "display", "rapidq", "from", "aliases", "kind", "group", "visual", "container", "size", "only", "where", "default_event", "sets", "origin",
            "instance_of", "doc", "properties", "methods", "events", "example",
        ],
        at,
    )?;
    let name = req(c, "name", at)?;
    let at = &format!("{at}: {name}");
    let rapidq = s(c, "rapidq");
    // (a component's name in mixed case without its Q / R: `CheckBox`;
    // a global object's is its name)
    let display = match s(c, "display") {
        Some(d) if global || d.eq_ignore_ascii_case(&name[1..]) => d,
        Some(d) => return Err(format!("{at}: display `{d}` isn't its name without the Q / R")),
        None if global => name,
        None => return Err(format!("{at}: `display` (its name in mixed case without the Q / R) is required")),
    };
    let family = if global { s(c, "origin").unwrap_or("rapidq") } else if rapidq.is_some() { "rapidq" } else { "rapidr" };
    let kind = match (global, s(c, "kind").unwrap_or("component")) {
        (true, "global") => "Kind::Global",
        (true, "item") => "Kind::Item",
        (false, "component") => "Kind::Component",
        (false, "library") => "Kind::Library",
        (false, "planned") => "Kind::Planned",
        (_, k) => return Err(format!("{at}: kind `{k}`")),
    };
    let aliases = strs(c, "aliases", at)?;
    let mut names = vec![name.to_ascii_uppercase()];
    names.extend(rapidq.map(str::to_ascii_uppercase));
    names.extend(aliases.iter().map(|a| a.to_ascii_uppercase()));
    names.sort();
    names.dedup();
    let mut seen: BTreeMap<(&str, String), ()> = BTreeMap::new();
    let mut props = Vec::new();
    let mut methods = Vec::new();
    let mut events = Vec::new();
    let mut member_names: HashSet<String> = HashSet::new();
    let mut check = |kind: &'static str, n: &str, at: &str| -> Res<()> {
        if !member_names.insert(n.to_ascii_lowercase()) {
            return Err(format!("{at}: {kind} `{n}`: a member of that name already"));
        }
        seen.insert((kind, n.to_ascii_lowercase()), ());
        Ok(())
    };
    // (each property's name and the names its range uses)
    let mut prop_names: Vec<String> = Vec::new();
    let mut range_uses: Vec<(String, Vec<String>)> = Vec::new();
    for p in tables(c, "properties", at)? {
        let pname = req(p, "name", at)?;
        check("property", pname, at)?;
        let (code, refs) = property(ctx, p, family, None, at)?;
        props.push(code);
        prop_names.push(pname.to_ascii_lowercase());
        range_uses.push((pname.to_string(), refs));
    }
    for m in tables(c, "methods", at)? {
        check("method", req(m, "name", at)?, at)?;
        methods.push(method(ctx, m, family, None, at)?);
    }
    let mut event_names = Vec::new();
    for e in tables(c, "events", at)? {
        let n = req(e, "name", at)?;
        check("event", n, at)?;
        event_names.push(n.to_ascii_lowercase());
        events.push(event(ctx, e, family, None, at)?);
    }
    for set in strs(c, "sets", at)? {
        let st = ctx.sets.get(&set).ok_or_else(|| format!("{at}: set `{set}` isn't in sets.toml"))?;
        let sf = s(st, "origin").unwrap_or("rapidr");
        for p in tables(st, "properties", at)? {
            let pname = req(p, "name", at)?;
            check("property", pname, at)?;
            let (code, refs) = property(ctx, p, sf, Some(&set), &format!("{at} (set {set})"))?;
            props.push(code);
            prop_names.push(pname.to_ascii_lowercase());
            range_uses.push((pname.to_string(), refs));
        }
        for m in tables(st, "methods", at)? {
            check("method", req(m, "name", at)?, at)?;
            methods.push(method(ctx, m, sf, Some(&set), &format!("{at} (set {set})"))?);
        }
        for e in tables(st, "events", at)? {
            check("event", req(e, "name", at)?, at)?;
            events.push(event(ctx, e, sf, Some(&set), &format!("{at} (set {set})"))?);
        }
    }
    // (a range naming a property names one of this component's)
    for (pname, refs) in &range_uses {
        if let Some(r) = refs.iter().find(|r| !prop_names.contains(&r.to_ascii_lowercase())) {
            return Err(format!("{at}, property {pname}: its range names `{r}`, which isn't one of its properties"));
        }
    }
    let default_event = s(c, "default_event");
    if let Some(d) = default_event {
        if !event_names.contains(&d.to_ascii_lowercase()) {
            return Err(format!("{at}: default_event `{d}` isn't one of its events"));
        }
    }
    let size = match c.get("size").and_then(Value::as_array) {
        Some(v) if v.len() == 2 => format!("Some(({}, {}))", v[0].as_integer().unwrap_or(0), v[1].as_integer().unwrap_or(0)),
        Some(_) => return Err(format!("{at}: size is [width, height]")),
        None => "None".into(),
    };
    let doc = s(c, "doc").unwrap_or("");
    let code = format!(
        "Component {{ name: {}, display: {}, rapidq: {}, from: {}, aliases: &[{}], kind: {kind}, group: {}, origin: {}, visual: {}, container: {}, size: {size}, default_event: {}, runtimes: {}, where_note: {}, instance_of: {}, doc: {}, example: {}, properties: &[{}], methods: &[{}], events: &[{}] }}",
        lit(name),
        lit(display),
        opt(rapidq),
        opt(s(c, "from")),
        aliases.iter().map(|a| lit(a)).collect::<Vec<_>>().join(", "),
        lit(s(c, "group").unwrap_or(group)),
        if family == "rapidq" { "Origin::RapidQ" } else { "Origin::RapidR" },
        b(c, "visual").unwrap_or(false),
        b(c, "container").unwrap_or(false),
        opt(default_event),
        runtimes(c, at)?,
        opt(s(c, "where")),
        opt(s(c, "instance_of")),
        lit(doc),
        lit(s(c, "example").unwrap_or("")),
        props.join(",\n        "),
        methods.join(",\n        "),
        events.join(",\n        "),
    );
    let is_component_type = kind == "Kind::Component";
    Ok((code, names, name.to_string(), is_component_type))
}

/// The component files in the manual's (and the toolbox's) order: what a
/// component is for, the everyday ones first.
const FAMILIES: &[&str] =
    &["forms", "input", "display", "lists", "menus", "dialogs", "objects", "databases", "network", "media", "directx", "d3d", "datascience", "web", "ide", "libraries"];

fn build(dir: &Path) -> Res<String> {
    let mut paths = Vec::new();
    files(dir, &mut paths);
    let rank = |p: &PathBuf| p.file_stem().and_then(|s| s.to_str()).and_then(|s| FAMILIES.iter().position(|f| *f == s)).unwrap_or(FAMILIES.len());
    paths.sort_by_key(|p| (p.parent().map(Path::to_path_buf), rank(p)));
    let by_name = |n: &str| paths.iter().find(|p| p.file_name().is_some_and(|f| f == n)).cloned();
    // Constants first: everything else names them.
    let consts_t = load(&by_name("constants.toml").ok_or("constants.toml missing")?)?;
    let mut constants = HashMap::new();
    let mut groups_code = Vec::new();
    for g in tables(&consts_t, "group", "constants.toml")? {
        keys_allowed(g, &["name", "source", "origin", "doc", "constants"], "constants.toml")?;
        let gname = req(g, "name", "constants.toml")?;
        let at = format!("constants.toml: {gname}");
        let mut items = Vec::new();
        for c in arr(g, "constants") {
            let pair = c.as_array().filter(|a| a.len() == 2).ok_or_else(|| format!("{at}: constants are [name, value]"))?;
            let n = pair[0].as_str().ok_or_else(|| format!("{at}: a constant's name"))?;
            let v = pair[1].as_integer().ok_or_else(|| format!("{at}: {n}'s value"))?;
            if constants.insert(n.to_ascii_lowercase(), v).is_some_and(|old| old != v) {
                return Err(format!("{at}: {n} defined twice with different values"));
            }
            items.push(format!("({}, {v})", lit(n)));
        }
        groups_code.push(format!(
            "ConstantGroup {{ name: {}, source: {}, origin: {}, doc: {}, constants: &[{}] }}",
            lit(gname),
            lit(req(g, "source", &at)?),
            origin(g, "rapidq", &at)?,
            lit(s(g, "doc").unwrap_or("")),
            items.join(", ")
        ));
    }
    // The glossary: docs and categories by member name.
    let mut glossary = Glossary { entries: HashMap::new() };
    for p in paths.iter().filter(|p| p.parent().is_some_and(|d| d.ends_with("glossary"))) {
        let g = load(p)?;
        let gfile = p.strip_prefix(dir).unwrap().display().to_string();
        for (kind, entries) in &g {
            if !["property", "method", "event"].contains(&kind.as_str()) {
                return Err(format!("{gfile}: [{kind}] (property, method or event)"));
            }
            let entries = entries.as_table().ok_or_else(|| format!("{gfile}: tables of entries"))?;
            for (name, e) in entries {
                let at = format!("{gfile}: {kind}.{name}");
                let e = e.as_table().ok_or_else(|| format!("{at}: a table"))?;
                // (a property's range and unit by name: numbers only — a
                // range naming another property belongs on the component)
                keys_allowed(e, if kind == "property" { &["doc", "category", "range", "unit"] } else { &["doc", "category"] }, &at)?;
                let range = s(e, "range").map(str::to_string);
                if let Some(r) = &range {
                    if !range_refs(r, &at)?.is_empty() {
                        return Err(format!("{at}: a glossary range has numbers only (one naming a property goes on the component)"));
                    }
                }
                let unit_s = s(e, "unit").map(str::to_string);
                if let Some(u) = &unit_s {
                    unit(u, &at)?;
                }
                let entry = GlossaryEntry { doc: s(e, "doc").unwrap_or("").to_string(), category: s(e, "category").unwrap_or("").to_string(), range, unit: unit_s };
                if glossary.entries.insert((kind.clone(), name.to_ascii_lowercase()), entry).is_some() {
                    return Err(format!("{at}: twice"));
                }
            }
        }
    }
    let mut sets = HashMap::new();
    if let Some(p) = by_name("sets.toml") {
        for st in tables(&load(&p)?, "set", "sets.toml")? {
            sets.insert(req(st, "name", "sets.toml")?.to_string(), st.clone());
        }
    }
    let mut items_names = Vec::new();
    if let Some(p) = by_name("items.toml") {
        for o in tables(&load(&p)?, "object", "items.toml")? {
            items_names.push(req(o, "name", "items.toml")?.to_string());
        }
    }
    let ctx = Ctx { constants, glossary, sets, items: items_names };

    let mut comps = Vec::new();
    let mut component_types = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut globals = Vec::new();
    let mut items = Vec::new();
    let mut builtins = Vec::new();
    let mut internal = Vec::new();
    let mut statements = Vec::new();
    let mut directives = Vec::new();
    let mut keywords = Vec::new();
    let mut types = Vec::new();
    for path in &paths {
        let file = path.strip_prefix(dir).unwrap().display().to_string();
        let t = load(path)?;
        if file.starts_with("components") {
            keys_allowed(&t, &["group", "component"], &file)?;
            let group = s(&t, "group").unwrap_or("");
            for c in tables(&t, "component", &file)? {
                let (code, names, name, is_type) = component(&ctx, c, group, false, &file)?;
                for n in names {
                    if index.insert(n.clone(), comps.len()).is_some() {
                        return Err(format!("{file}: the name {n} is another component's too"));
                    }
                }
                if is_type {
                    component_types.push(name.to_ascii_uppercase());
                }
                comps.push(code);
            }
        } else if file == "items.toml" {
            for c in tables(&t, "object", &file)? {
                let (code, _, _, _) = component(&ctx, c, "Items", true, &file)?;
                items.push(code);
            }
        } else if file == "globals.toml" {
            for c in tables(&t, "object", &file)? {
                let (code, _, _, _) = component(&ctx, c, "Global objects", true, &file)?;
                globals.push(code);
            }
        } else if file == "builtins.toml" {
            keys_allowed(&t, &["builtin", "internal"], &file)?;
            let mut keys = HashSet::new();
            for x in tables(&t, "builtin", &file)? {
                keys_allowed(x, &["name", "syntax", "params", "returns", "group", "bare", "origin", "only", "missing", "doc", "effects", "risk", "example"], &file)?;
                let name = req(x, "name", &file)?;
                let at = format!("{file}: {name}");
                let key = builtin_key(name);
                if !keys.insert(key.clone()) {
                    return Err(format!("{at}: another builtin has the key `{key}`"));
                }
                let (effects_code, effects_risk) = effects(x, &at)?;
                let risk_code = risk(x, effects_risk, &at)?;
                builtins.push(format!(
                    "Builtin {{ name: {}, key: {}, syntax: {}, params: {}, returns: {}, group: {}, bare: {}, origin: {}, runtimes: {}, missing: {}, doc: {}, effects: {effects_code}, risk: {risk_code}, example: {} }}",
                    lit(name),
                    lit(&key),
                    lit(s(x, "syntax").unwrap_or("")),
                    params(s(x, "params").unwrap_or(""), &at)?,
                    opt(s(x, "returns")),
                    lit(s(x, "group").unwrap_or("Other")),
                    b(x, "bare").unwrap_or(false),
                    origin(x, "rapidq", &at)?,
                    runtimes(x, &at)?,
                    b(x, "missing").unwrap_or(false),
                    lit(s(x, "doc").unwrap_or("")),
                    lit(s(x, "example").unwrap_or("")),
                ));
            }
            internal = strs(&t, "internal", &file)?;
        } else if file == "language.toml" {
            keys_allowed(&t, &["statement", "directive", "keyword", "type"], &file)?;
            for x in tables(&t, "statement", &file)? {
                keys_allowed(x, &["name", "syntax", "group", "origin", "doc", "effects"], &file)?;
                let name = req(x, "name", &file)?;
                let at = format!("{file}: {name}");
                let (effects_code, _) = effects(x, &at)?;
                statements.push(format!(
                    "Statement {{ name: {}, syntax: {}, group: {}, origin: {}, doc: {}, effects: {effects_code} }}",
                    lit(name),
                    lit(req(x, "syntax", &at)?),
                    lit(s(x, "group").unwrap_or("Other")),
                    origin(x, "rapidq", &at)?,
                    lit(s(x, "doc").unwrap_or(""))
                ));
            }
            for x in tables(&t, "directive", &file)? {
                keys_allowed(x, &["name", "syntax", "origin", "doc"], &file)?;
                let name = req(x, "name", &file)?;
                let at = format!("{file}: {name}");
                if !name.starts_with('$') {
                    return Err(format!("{at}: a directive starts with $"));
                }
                directives.push(format!(
                    "Directive {{ name: {}, syntax: {}, origin: {}, doc: {} }}",
                    lit(name),
                    lit(req(x, "syntax", &at)?),
                    origin(x, "rapidq", &at)?,
                    lit(s(x, "doc").unwrap_or(""))
                ));
            }
            for x in tables(&t, "keyword", &file)? {
                keys_allowed(x, &["name", "kind", "origin", "doc"], &file)?;
                let name = req(x, "name", &file)?;
                let at = format!("{file}: {name}");
                keywords.push(format!(
                    "Keyword {{ name: {}, kind: {}, origin: {}, doc: {} }}",
                    lit(name),
                    lit(req(x, "kind", &at)?),
                    origin(x, "rapidq", &at)?,
                    lit(s(x, "doc").unwrap_or(""))
                ));
            }
            for x in tables(&t, "type", &file)? {
                keys_allowed(x, &["name", "origin", "doc"], &file)?;
                let name = req(x, "name", &file)?;
                let at = format!("{file}: {name}");
                types.push(format!("TypeName {{ name: {}, origin: {}, doc: {} }}", lit(name), origin(x, "rapidq", &at)?, lit(s(x, "doc").unwrap_or(""))));
            }
        } else if !["constants.toml", "sets.toml"].contains(&file.as_str()) && !file.starts_with("glossary") {
            return Err(format!("{file}: not a registry file (components/*.toml, globals, builtins, language, constants, glossary, sets)"));
        }
    }
    let mut out = String::from("// Generated by build.rs from data/*.toml. Do not edit.\n\n");
    writeln!(out, "pub static COMPONENTS: &[Component] = &[\n    {},\n];\n", comps.join(",\n    ")).unwrap();
    writeln!(out, "pub static GLOBALS: &[Component] = &[\n    {},\n];\n", globals.join(",\n    ")).unwrap();
    writeln!(out, "pub static ITEMS: &[Component] = &[\n    {},\n];\n", items.join(",\n    ")).unwrap();
    writeln!(
        out,
        "/// Every component the compilers create (RapidR's upper-case names), in the data's order.\npub const COMPONENT_TYPES: &[&str] = &[{}];\n",
        component_types.iter().map(|n| lit(n)).collect::<Vec<_>>().join(", ")
    )
    .unwrap();
    writeln!(
        out,
        "/// Every name of a component (RapidR's, RapidQ's, aliases), upper case, sorted: its index in COMPONENTS.\nstatic BY_NAME: &[(&str, usize)] = &[{}];\n",
        index.iter().map(|(n, i)| format!("({}, {i})", lit(n))).collect::<Vec<_>>().join(", ")
    )
    .unwrap();
    writeln!(out, "pub static BUILTINS: &[Builtin] = &[\n    {},\n];\n", builtins.join(",\n    ")).unwrap();
    writeln!(out, "pub const INTERNAL_BUILTINS: &[&str] = &[{}];\n", internal.iter().map(|n| lit(n)).collect::<Vec<_>>().join(", ")).unwrap();
    writeln!(out, "pub static STATEMENTS: &[Statement] = &[\n    {},\n];\n", statements.join(",\n    ")).unwrap();
    writeln!(out, "pub static DIRECTIVES: &[Directive] = &[\n    {},\n];\n", directives.join(",\n    ")).unwrap();
    writeln!(out, "pub static KEYWORDS: &[Keyword] = &[\n    {},\n];\n", keywords.join(",\n    ")).unwrap();
    writeln!(out, "pub static TYPE_NAMES: &[TypeName] = &[\n    {},\n];\n", types.join(",\n    ")).unwrap();
    writeln!(out, "pub static CONSTANT_GROUPS: &[ConstantGroup] = &[\n    {},\n];\n", groups_code.join(",\n    ")).unwrap();
    Ok(out)
}

/// The runtimes' dispatch key: lower case, one type suffix dropped (`MID$` → `mid`).
fn builtin_key(name: &str) -> String {
    let mut k = name.to_ascii_lowercase();
    if k.len() > 1 && matches!(k.chars().last(), Some('$' | '%' | '#' | '&' | '!')) {
        k.pop();
    }
    k
}
