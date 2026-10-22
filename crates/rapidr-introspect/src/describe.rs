//! `describe`: the registry's view of a component type or a builtin, with
//! every AI facet (docs/ai-architecture.md §2) and a JSON Schema for each
//! value — the same on every side (it reads only `rapidr_lang`).
//!
//! RapidR's names throughout (`RButton`; a type's RapidQ name is given as
//! `rapidq`); a RapidQ name is accepted. Members RapidR doesn't answer yet
//! (`missing`) are left out: an AI must not use them.

use rapidr_lang::ai::{event_example, method_example, property_example};
use rapidr_lang::{Access, Builtin, Component, DefaultValue, Effect, Kind, Param, Property, Runtimes, Type};
use serde_json::{json, Map, Value};

use crate::protocol::{Error, ErrorCode, PROTOCOL_VERSION};

/// The component a name means for `describe`: a component (any of its
/// names, Q-prefixed ones read as the compilers read them), a global or an
/// item object.
pub fn find_type(name: &str) -> Result<&'static Component, Error> {
    let c = rapidr_lang::resolve_component(name).or_else(|| rapidr_lang::global(name)).or_else(|| rapidr_lang::item(name));
    match c {
        Some(c) if c.kind == Kind::Planned => Err(Error::new(ErrorCode::UnknownType, format!("{} is RapidQ's; RapidR doesn't have it yet", c.spelling()))),
        Some(c) => Ok(c),
        None => Err(Error::new(ErrorCode::UnknownType, format!("no component type `{name}` (describe {{\"what\":\"types\"}} lists them)"))),
    }
}

/// A type name as RapidR writes it: `QFORM` → `RForm` (a kind of a
/// component property, a parameter's type).
fn spelled(name: &str) -> String {
    rapidr_lang::resolve_component(name).map_or_else(|| name.to_string(), |c| c.spelling())
}

fn effects(e: &[Effect]) -> Value {
    Value::Array(e.iter().map(|x| json!(x.as_str())).collect())
}

fn only(rt: Runtimes) -> Option<&'static str> {
    match rt {
        Runtimes::All => None,
        r => Some(r.as_str()),
    }
}

/// A JSON object without its null fields.
fn obj(fields: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in fields {
        if !v.is_null() {
            m.insert(k.to_string(), v);
        }
    }
    Value::Object(m)
}

/// `0..`, `Min..Max`: its numeric bounds.
fn bounds(range: Option<&str>) -> (Option<f64>, Option<f64>) {
    let Some((lo, hi)) = range.and_then(|r| r.split_once("..")) else { return (None, None) };
    (lo.trim().parse().ok(), hi.trim().parse().ok())
}

/// The JSON Schema a value of property `p` satisfies (§2.3; the subset
/// every provider accepts: no `$ref`, no `anyOf`).
pub fn property_schema(p: &Property) -> Value {
    let (lo, hi) = bounds(p.range);
    let num = |ty: &str| {
        let mut m = Map::new();
        m.insert("type".into(), json!(ty));
        if let Some(l) = lo {
            m.insert("minimum".into(), if ty == "integer" { json!(l as i64) } else { json!(l) });
        }
        if let Some(h) = hi {
            m.insert("maximum".into(), if ty == "integer" { json!(h as i64) } else { json!(h) });
        }
        Value::Object(m)
    };
    match p.ty {
        Type::Int => num("integer"),
        Type::Float => num("number"),
        Type::String | Type::Picture | Type::Resource => json!({"type": "string"}),
        Type::Bool => json!({"type": "boolean"}),
        Type::Enum => json!({"type": "string", "enum": p.values}),
        Type::Set => json!({"type": "array", "items": {"type": "string", "enum": p.values}, "uniqueItems": true}),
        Type::Color => json!({"type": "string", "description": "#RRGGBB, or a colour constant (clRed)"}),
        Type::Component => {
            let kinds: Vec<String> = p.kinds.iter().map(|k| spelled(k)).collect();
            json!({"type": "string", "description": format!("the name of a {}", kinds.join(" or "))})
        }
        Type::Font => json!({"type": "string", "description": "the name of an RFont"}),
        Type::Item | Type::Any => json!({}),
    }
}

/// A parameter's JSON Schema, from its BASIC type.
pub fn param_schema(p: &Param) -> Value {
    match p.ty.to_ascii_uppercase().as_str() {
        "INTEGER" | "LONG" | "WORD" | "BYTE" | "SHORT" | "DWORD" | "INT64" => json!({"type": "integer"}),
        "DOUBLE" | "SINGLE" | "FLOAT" => json!({"type": "number"}),
        "STRING" => json!({"type": "string"}),
        "BOOLEAN" => json!({"type": "boolean"}),
        "" => json!({}),
        t => match rapidr_lang::resolve_component(t) {
            Some(c) => json!({"type": "string", "description": format!("the name of a {}", c.spelling())}),
            None => json!({"description": format!("a {t}")}),
        },
    }
}

fn params(ps: &[Param]) -> Value {
    Value::Array(
        ps.iter()
            .map(|p| {
                obj(vec![
                    ("name", json!(p.name)),
                    ("type", if p.ty.is_empty() { Value::Null } else { json!(spelled(p.ty)) }),
                    ("optional", if p.optional { json!(true) } else { Value::Null }),
                    ("byref", if p.byref { json!(true) } else { Value::Null }),
                    ("variadic", if p.variadic { json!(true) } else { Value::Null }),
                    ("default", p.default.map_or(Value::Null, |d| json!(d))),
                    ("schema", param_schema(p)),
                ])
            })
            .collect(),
    )
}

/// A registry default as a typed value (§3.4).
pub fn default_value(p: &Property) -> Value {
    match &p.default {
        None => Value::Null,
        Some(DefaultValue::Int(i)) => match p.ty {
            Type::Bool => json!(*i != 0),
            Type::Color => crate::value::color(*i, None),
            Type::Enum => crate::value::enumerated(p, *i, None),
            Type::Set => crate::value::set(p, *i),
            _ => json!(i),
        },
        Some(DefaultValue::Float(f)) => json!(f),
        Some(DefaultValue::Bool(b)) => json!(b),
        Some(DefaultValue::Str(s)) => json!(s),
        Some(DefaultValue::Expr(e)) => {
            let value = rapidr_lang::eval_constant(e);
            match p.ty {
                Type::Set => json!({"set": e.split(['+', '|']).map(str::trim).collect::<Vec<_>>(), "value": value}),
                Type::Color => crate::value::color(value.unwrap_or(0), Some(e)),
                _ => json!({"const": e, "value": value}),
            }
        }
    }
}

fn property(c: &Component, p: &Property) -> Value {
    let access = match p.access {
        Access::ReadWrite => "rw",
        Access::Read => "read",
        Access::Write => "write",
    };
    let set_effects = if p.writable() { effects(p.set_effects(c)) } else { Value::Null };
    obj(vec![
        ("name", json!(p.name)),
        ("type", json!(p.ty.as_str())),
        ("values", if p.values.is_empty() { Value::Null } else { json!(p.values) }),
        ("kinds", if p.kinds.is_empty() { Value::Null } else { json!(p.kinds.iter().map(|k| spelled(k)).collect::<Vec<_>>()) }),
        ("default", default_value(p)),
        ("access", json!(access)),
        ("design", json!(p.design)),
        ("indexed", if p.indexed > 0 { json!(p.indexed) } else { Value::Null }),
        ("range", p.range.map_or(Value::Null, |r| json!(r))),
        ("unit", p.unit.map_or(Value::Null, |u| json!(u))),
        ("schema", property_schema(p)),
        ("setEffects", set_effects),
        ("risk", if p.writable() { json!(p.set_risk(c).as_str()) } else { json!("read") }),
        ("secret", if p.secret { json!(true) } else { Value::Null }),
        ("category", if p.category.is_empty() { Value::Null } else { json!(p.category) }),
        ("origin", json!(p.origin.as_str())),
        ("only", only(p.runtimes).map_or(Value::Null, |o| json!(o))),
        ("doc", json!(p.doc)),
        ("example", json!(property_example(c, p))),
    ])
}

fn method(c: &Component, m: &rapidr_lang::Method) -> Value {
    obj(vec![
        ("name", json!(m.name)),
        ("signature", json!(m.signature())),
        ("params", params(m.params)),
        ("returns", m.returns.map_or(Value::Null, |r| json!(spelled(r)))),
        ("effects", if m.effects.is_empty() { Value::Null } else { effects(m.effects) }),
        ("risk", m.risk().map_or(Value::Null, |r| json!(r.as_str()))),
        ("needsPerson", if m.effects.contains(&Effect::Wait) { json!(true) } else { Value::Null }),
        // (its result or its value is a credential: never given to a model)
        ("secret", if m.secret { json!(true) } else { Value::Null }),
        ("origin", json!(m.origin.as_str())),
        ("only", only(m.runtimes).map_or(Value::Null, |o| json!(o))),
        ("doc", json!(m.doc)),
        ("example", json!(method_example(c, m))),
    ])
}

fn event(c: &Component, e: &rapidr_lang::Event) -> Value {
    obj(vec![
        ("name", json!(e.name)),
        ("params", params(e.params)),
        ("origin", json!(e.origin.as_str())),
        ("only", only(e.runtimes).map_or(Value::Null, |o| json!(o))),
        ("doc", json!(e.doc)),
        ("example", json!(event_example(c, e))),
    ])
}

/// A component type's full view.
pub fn describe_type(c: &Component) -> Value {
    let caps: Vec<&str> = c.capabilities().iter().map(|e| e.as_str()).collect();
    obj(vec![
        ("protocol", json!(PROTOCOL_VERSION)),
        ("type", json!(c.spelling())),
        ("rapidq", c.rapidq_spelling().map_or(Value::Null, |q| json!(q))),
        ("kind", json!(c.kind.as_str())),
        ("group", json!(c.group)),
        ("origin", json!(c.origin.as_str())),
        ("visual", json!(c.visual)),
        ("container", json!(c.container)),
        ("size", c.size.map_or(Value::Null, |(w, h)| json!([w, h]))),
        ("defaultEvent", c.default_event.map_or(Value::Null, |e| json!(e))),
        ("only", only(c.runtimes).map_or(Value::Null, |o| json!(o))),
        ("where", c.where_note.map_or(Value::Null, |w| json!(w))),
        ("risk", c.risk().map_or(Value::Null, |r| json!(r.as_str()))),
        ("capabilities", json!(caps)),
        ("doc", json!(c.doc)),
        ("example", if c.example.is_empty() { Value::Null } else { json!(c.example) }),
        ("properties", Value::Array(c.properties.iter().filter(|p| !p.missing).map(|p| property(c, p)).collect())),
        ("methods", Value::Array(c.methods.iter().filter(|m| !m.missing).map(|m| method(c, m)).collect())),
        ("events", Value::Array(c.events.iter().filter(|e| !e.missing).map(|e| event(c, e)).collect())),
    ])
}

/// A builtin's view.
pub fn describe_builtin(b: &Builtin) -> Value {
    obj(vec![
        ("protocol", json!(PROTOCOL_VERSION)),
        ("builtin", json!(b.name)),
        ("syntax", json!(b.syntax)),
        ("group", json!(b.group)),
        ("params", if b.params.is_empty() { Value::Null } else { params(b.params) }),
        ("returns", b.returns.map_or(Value::Null, |r| json!(r))),
        ("effects", if b.effects.is_empty() { Value::Null } else { effects(b.effects) }),
        ("risk", b.risk().map_or(Value::Null, |r| json!(r.as_str()))),
        ("bare", if b.bare { json!(true) } else { Value::Null }),
        ("origin", json!(b.origin.as_str())),
        ("only", only(b.runtimes).map_or(Value::Null, |o| json!(o))),
        ("doc", json!(b.doc)),
        ("example", if b.example.is_empty() { Value::Null } else { json!(b.example) }),
    ])
}

/// The first sentence of a doc (for the lists).
fn first_sentence(doc: &str) -> &str {
    match doc.find(". ") {
        Some(i) => &doc[..=i],
        None => doc,
    }
}

/// Every component type the compilers create, the BASIC libraries and the
/// global objects, one line each, by group.
pub fn list_types() -> Value {
    let types: Vec<Value> = rapidr_lang::COMPONENTS
        .iter()
        .filter(|c| matches!(c.kind, Kind::Component | Kind::Library))
        .chain(rapidr_lang::GLOBALS)
        .map(|c| {
            obj(vec![
                ("type", json!(c.spelling())),
                ("rapidq", c.rapidq_spelling().map_or(Value::Null, |q| json!(q))),
                ("kind", json!(c.kind.as_str())),
                ("group", json!(c.group)),
                ("visual", json!(c.visual)),
                ("only", only(c.runtimes).map_or(Value::Null, |o| json!(o))),
                ("doc", json!(first_sentence(c.doc))),
            ])
        })
        .collect();
    json!({"protocol": PROTOCOL_VERSION, "types": types})
}

/// Every builtin RapidR has, one line each.
pub fn list_builtins() -> Value {
    let builtins: Vec<Value> = rapidr_lang::BUILTINS
        .iter()
        .filter(|b| !b.missing)
        .map(|b| {
            obj(vec![
                ("builtin", json!(b.name)),
                ("syntax", json!(if b.syntax.is_empty() { b.name } else { b.syntax })),
                ("group", json!(b.group)),
                ("risk", b.risk().map_or(Value::Null, |r| json!(r.as_str()))),
                ("only", only(b.runtimes).map_or(Value::Null, |o| json!(o))),
            ])
        })
        .collect();
    json!({"protocol": PROTOCOL_VERSION, "builtins": builtins})
}

/// The `describe` request.
pub fn describe(type_name: Option<&str>, builtin: Option<&str>, what: Option<&str>) -> Result<Value, Error> {
    match (type_name, builtin, what) {
        (Some(t), None, None) => find_type(t).map(describe_type),
        (None, Some(b), None) => match rapidr_lang::builtin(b) {
            Some(b) if !b.missing => Ok(describe_builtin(b)),
            Some(b) => Err(Error::new(ErrorCode::UnknownType, format!("{} is RapidQ's; RapidR doesn't have it yet", b.name))),
            None => Err(Error::new(ErrorCode::UnknownType, format!("no builtin `{b}` (describe {{\"what\":\"builtins\"}} lists them)"))),
        },
        (None, None, Some("types")) => Ok(list_types()),
        (None, None, Some("builtins")) => Ok(list_builtins()),
        (None, None, Some(w)) => Err(Error::new(ErrorCode::BadRequest, format!("describe what `{w}`: types or builtins"))),
        _ => Err(Error::new(ErrorCode::BadRequest, "describe takes one of `type`, `builtin` or `what`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_button_described() {
        let v = describe(Some("QBUTTON"), None, None).unwrap();
        assert_eq!(v["type"], "RButton");
        assert_eq!(v["rapidq"], "QButton");
        assert_eq!(v["risk"], "ui");
        assert_eq!(v["capabilities"], json!([]));
        assert!(v["example"].as_str().unwrap().contains("CREATE Save AS RButton"));
        let props = v["properties"].as_array().unwrap();
        let left = props.iter().find(|p| p["name"] == "Left").unwrap();
        assert_eq!(left["range"], "any");
        assert_eq!(left["unit"], "px");
        assert_eq!(left["schema"], json!({"type": "integer"}));
        let width = props.iter().find(|p| p["name"] == "Width").unwrap();
        assert_eq!(width["schema"], json!({"type": "integer", "minimum": 0}));
        let align = props.iter().find(|p| p["name"] == "Align").unwrap();
        assert_eq!(align["default"], json!({"const": "alNone", "value": 0}));
        assert_eq!(align["schema"]["enum"][5], "alClient");
        let parent = props.iter().find(|p| p["name"] == "Parent").unwrap();
        assert_eq!(parent["kinds"], json!(["RForm", "RPanel", "RTabControl"]));
        assert_eq!(parent["access"], "write");
        let anchors = props.iter().find(|p| p["name"] == "Anchors").unwrap();
        assert_eq!(anchors["default"], json!({"set": ["akLeft", "akTop"], "value": 3}));
        assert_eq!(anchors["origin"], "rapidr");
        let caption = props.iter().find(|p| p["name"] == "Caption").unwrap();
        assert_eq!((caption["setEffects"].clone(), caption["risk"].clone()), (json!(["ui"]), json!("ui")));
        let handle = props.iter().find(|p| p["name"] == "Handle").unwrap();
        assert_eq!((handle["risk"].clone(), handle.get("setEffects")), (json!("read"), None));
        let m = v["methods"].as_array().unwrap().iter().find(|m| m["name"] == "SetFocus").unwrap();
        assert_eq!((m["effects"].clone(), m["risk"].clone(), m["example"].clone()), (json!(["ui"]), json!("ui"), json!("Button1.SetFocus")));
        let e = v["events"].as_array().unwrap().iter().find(|e| e["name"] == "OnKeyDown").unwrap();
        assert_eq!(e["params"][0], json!({"name": "Key", "type": "WORD", "schema": {"type": "integer"}}));
        // RapidQ members RapidR doesn't answer yet aren't offered
        let (c, m) = rapidr_lang::COMPONENTS
            .iter()
            .chain(rapidr_lang::GLOBALS)
            .filter(|c| c.kind != Kind::Planned)
            .find_map(|c| c.methods.iter().find(|m| m.missing).map(|m| (c, m)))
            .expect("a member RapidR lacks (the registry's `missing`)");
        let v = describe(Some(c.name), None, None).unwrap();
        assert!(v["methods"].as_array().unwrap().iter().all(|x| x["name"] != m.name), "{}.{}", c.name, m.name);
    }

    #[test]
    fn builtins_and_lists() {
        let k = describe(None, Some("kill"), None).unwrap();
        assert_eq!((k["builtin"].clone(), k["risk"].clone(), k["effects"].clone()), (json!("KILL"), json!("irreversible"), json!(["file-write"])));
        assert_eq!(k["params"][0]["schema"], json!({"type": "string"}));
        let mid = describe(None, Some("Mid"), None).unwrap();
        assert_eq!((mid["risk"].clone(), mid["params"][2]["optional"].clone()), (json!("read"), json!(true)));
        let types = describe(None, None, Some("types")).unwrap();
        let types = types["types"].as_array().unwrap();
        assert!(types.iter().any(|t| t["type"] == "RButton" && t["rapidq"] == "QButton"));
        assert!(types.iter().any(|t| t["type"] == "Screen"));
        assert!(types.iter().all(|t| t["kind"] != "planned"));
        assert!(describe(None, None, Some("builtins")).unwrap()["builtins"].as_array().unwrap().len() > 100);
    }

    #[test]
    fn bad_describes() {
        assert_eq!(describe(Some("QNOTHING"), None, None).unwrap_err().code, ErrorCode::UnknownType);
        assert_eq!(describe(Some("QOLEOBJECT"), None, None).unwrap_err().code, ErrorCode::UnknownType, "planned");
        assert_eq!(describe(None, Some("NOPE$"), None).unwrap_err().code, ErrorCode::UnknownType);
        assert_eq!(describe(Some("RButton"), Some("MID$"), None).unwrap_err().code, ErrorCode::BadRequest);
        assert_eq!(describe(None, None, None).unwrap_err().code, ErrorCode::BadRequest);
        assert_eq!(describe(None, None, Some("everything")).unwrap_err().code, ErrorCode::BadRequest);
    }
}
