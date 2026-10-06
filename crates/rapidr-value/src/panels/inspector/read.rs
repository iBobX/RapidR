//! Reading what an inspector shows from its subject: the registry says
//! which properties and events the inspected components' types have (the
//! ones they all have, for a multi-selection), the subject gives each its
//! value.

use super::model::{Custom, Ev, Prop, EXTENSIONS, MISC};
use super::values::{self, Kind, FONT_PARTS};
use crate::panels::subject::{default_value, Host, Subject};
use crate::Value;

/// What was read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    /// The inspected components: (name, type as the program writes it).
    pub objects: Vec<(String, String)>,
    pub props: Vec<Prop>,
    pub events: Vec<Ev>,
    /// Their type, when they are all one ("" otherwise).
    pub type_name: String,
}

/// The registry's properties of `c` the inspector shows: set at design
/// time, read and written, not indexed, answered by RapidR.
pub fn shown_properties(c: &'static rapidr_lang::Component) -> impl Iterator<Item = (&'static rapidr_lang::Property, Kind)> {
    c.properties
        .iter()
        .filter(|p| p.design && p.access == rapidr_lang::Access::ReadWrite && p.indexed == 0 && !p.missing)
        .filter_map(|p| Kind::of(p).map(|k| (p, k)))
}

/// Same kind of value (an enum's or set's constants may differ in order
/// between types: the same kind still).
fn same_kind(a: &Kind, b: &Kind) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

/// Reads `subject` (and the program's own properties `custom`) through
/// `host`; `live`: the subject is the program's components (a property
/// never set reads as the runtime's value).
pub fn read(host: &dyn Host, subject: Option<&dyn Subject>, custom: &[Custom]) -> Snapshot {
    let mut snap = Snapshot::default();
    if let Some(s) = subject {
        snap.objects = s.objects(host);
    }
    let comps: Vec<(&str, &'static rapidr_lang::Component)> = snap.objects.iter().filter_map(|(n, t)| Some((n.as_str(), rapidr_lang::component(t)?))).collect();
    if let (Some(s), Some((_, first))) = (subject, comps.first()) {
        let names: Vec<&str> = comps.iter().map(|(n, _)| *n).collect();
        let types: Vec<&str> = snap.objects.iter().map(|(_, t)| t.as_str()).collect();
        snap.type_name = if types.iter().all(|t| t.eq_ignore_ascii_case(types[0])) { types[0].to_string() } else { String::new() };
        for (p, kind) in shown_properties(first) {
            // (a multi-selection: the properties every one of them has)
            if !comps.iter().all(|(_, c)| shown_properties(c).any(|(q, k)| q.name.eq_ignore_ascii_case(p.name) && same_kind(&k, &kind))) {
                continue;
            }
            let ext = first.origin == rapidr_lang::Origin::RapidQ && p.origin == rapidr_lang::Origin::RapidR;
            let category = if ext {
                EXTENSIONS.to_string()
            } else if p.category.is_empty() {
                MISC.to_string()
            } else {
                p.category.to_string()
            };
            // (a Color without a registry default: what RapidQ reads until
            // it's set — clBtnFace, clWindow)
            let default = default_value(first.name, p.name)
                .or_else(|| (kind == Kind::Color && p.name.eq_ignore_ascii_case("color")).then(|| crate::component_defaults::color_read(first.name, &Value::Null, || None)).flatten())
                .or_else(|| (kind == Kind::Color && p.name.eq_ignore_ascii_case("fontcolor")).then_some(Value::Integer(crate::component_defaults::CL_WINDOW_TEXT)))
                .and_then(|d| values::normalize(&kind, &d));
            let mut prop = Prop { name: p.name.to_string(), kind: kind.clone(), category, ext, value: None, default, is_default: true, custom: false, doc: p.doc.to_string(), parts: Vec::new() };
            if kind == Kind::Font {
                prop.parts = FONT_PARTS
                    .iter()
                    .map(|(part_name, part)| {
                        let k = part.kind();
                        let read = |o: &str| s.get(host, o, &format!("{}.{part_name}", p.name)).or_else(|| s.get(host, o, part.flat()));
                        let mut q = Prop { name: part_name.to_string(), kind: k.clone(), category: String::new(), ext: false, value: None, default: Some(part.default()), is_default: true, custom: false, doc: String::new(), parts: Vec::new() };
                        fill(&mut q, names.iter().map(|o| read(o)).collect());
                        q
                    })
                    .collect();
                prop.is_default = prop.parts.iter().all(|q| q.is_default);
                prop.value = Some(Value::String(prop.text()));
            } else {
                fill(&mut prop, names.iter().map(|o| s.get(host, o, p.name)).collect());
            }
            snap.props.push(prop);
        }
        for e in first.events.iter().filter(|e| !e.missing) {
            if !comps.iter().all(|(_, c)| c.events.iter().any(|x| x.name.eq_ignore_ascii_case(e.name) && !x.missing)) {
                continue;
            }
            let bound: Vec<String> = names.iter().map(|o| s.handler(host, o, e.name).unwrap_or_default()).collect();
            let handler = if bound.iter().all(|b| b.eq_ignore_ascii_case(&bound[0])) { Some(bound[0].clone()) } else { None };
            snap.events.push(Ev { name: e.name.to_string(), params: e.params.iter().map(|p| p.name.to_string()).collect(), handler, doc: e.doc.to_string() });
        }
    }
    for c in custom {
        let mut prop = Prop {
            name: c.name.clone(),
            kind: c.kind.clone(),
            category: if c.category.is_empty() { MISC.to_string() } else { c.category.clone() },
            ext: false,
            value: None,
            default: values::normalize(&c.kind, &c.default),
            is_default: false,
            custom: true,
            doc: String::new(),
            parts: Vec::new(),
        };
        if c.kind == Kind::Font {
            // (a font of the program's: "Name, Size" as text)
            prop.kind = Kind::Text;
        }
        fill(&mut prop, vec![Some(c.value.clone())]);
        // (a name the subject's properties have too: the program's wins)
        snap.props.retain(|p| !p.name.eq_ignore_ascii_case(&c.name));
        snap.props.push(prop);
    }
    snap
}

/// Gives `prop` the components' values `read` (`None`: not set there):
/// its value when they agree, whether it's the default.
fn fill(prop: &mut Prop, read: Vec<Option<Value>>) {
    let kind = prop.kind.clone();
    let norm: Vec<Option<Value>> = read.iter().map(|v| v.as_ref().and_then(|v| values::normalize(&kind, v))).collect();
    let fallback = prop.default.clone().unwrap_or_else(|| kind.empty());
    let shown: Vec<Value> = norm.iter().map(|v| v.clone().unwrap_or_else(|| fallback.clone())).collect();
    prop.value = match shown.first() {
        Some(first) if shown.iter().all(|v| values::same(&kind, v, first)) => Some(first.clone()),
        Some(_) => None,
        None => None,
    };
    // (at its default: never set, or set to the registry's default — a
    // property without one: its type's empty value)
    prop.is_default = norm.iter().zip(&shown).all(|(set, v)| set.is_none() || values::same(&kind, v, &fallback));
}
