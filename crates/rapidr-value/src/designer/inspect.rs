//! "The inspected object": what the property inspector (RPropertyInspector,
//! lane L-PANELS) shows of the designer's selection and how it changes it.
//!
//! * [`inspect`] — the rows: every design-time property the registry
//!   (`rapidr_lang`) gives the selected components' type (for several
//!   components, the properties they all have), each with its type, enum /
//!   set values, category, origin (RapidQ's or a RapidR extension), docs,
//!   default, and its value as the CREATE block writes it — `None` when the
//!   block doesn't set it (the default applies), `mixed` when the selected
//!   components disagree, `in_code` when it's an expression the designer
//!   can't read (`Width = Screen.Width / 2`). Events likewise, with the
//!   handler each is bound to.
//! * [`set_value`] — a value typed in the inspector as the undoable
//!   command that writes it on every selected component (one undo step):
//!   pass it to `Designer::execute`. `None` removes the assignment (back
//!   to the default).
//! * [`format_value`] — how a value of a property type is written
//!   (strings quoted, Booleans `True` / `False`, sets as constant sums).

use rapidr_lang::{DefaultValue, Origin, Type};

use super::command::Command;
use super::model::{FormDesign, NodeId};

/// One property row.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyRow {
    /// The registry's spelling (`Caption`, `Anchors`).
    pub name: &'static str,
    pub ty: Type,
    /// An enum's or set's constants.
    pub values: &'static [&'static str],
    pub category: &'static str,
    pub origin: Origin,
    pub doc: &'static str,
    /// The registry's default, as source text.
    pub default: Option<String>,
    /// The value as written in the CREATE block (the first selected
    /// component's), `None` when not written.
    pub value: Option<String>,
    /// The selected components' values differ.
    pub mixed: bool,
    /// The value is an expression the designer can't read.
    pub in_code: bool,
}

/// One event row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRow {
    pub name: &'static str,
    /// Its parameter list (`Index AS INTEGER`), for a new handler.
    pub params: String,
    pub origin: Origin,
    pub doc: &'static str,
    /// The SUB it is bound to (`OnClick = Button1Click`).
    pub handler: Option<String>,
    pub mixed: bool,
}

/// The selection as the inspector shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inspected {
    pub components: Vec<NodeId>,
    /// The selected components' names.
    pub names: Vec<String>,
    /// Their type as written when they share one (else "").
    pub type_name: String,
    pub properties: Vec<PropertyRow>,
    pub events: Vec<EventRow>,
}

/// A registry default as source text.
pub fn default_text(d: &DefaultValue) -> String {
    match d {
        DefaultValue::Int(n) => n.to_string(),
        DefaultValue::Float(f) => f.to_string(),
        DefaultValue::Bool(b) => if *b { "True" } else { "False" }.into(),
        DefaultValue::Str(s) => super::value::write_str(s),
        DefaultValue::Expr(e) => e.to_string(),
    }
}

/// How a value typed for a property of type `ty` is written: strings in
/// quotes (unless already a literal), Booleans as `1` / `0` (RapidQ's
/// own spelling, and what works without RAPIDQ.INC: there `True` is an
/// undeclared variable, 0 — docs/rapidq-ground-truth.md), the rest as typed
/// (numbers, constants, `&H` colours).
pub fn format_value(ty: Type, typed: &str) -> String {
    format_value_as(ty, typed, None)
}

/// [`format_value`] for a line already written as `was`: a Boolean keeps
/// its words (`True` / `False`) when the line has them.
pub fn format_value_as(ty: Type, typed: &str, was: Option<&str>) -> String {
    let t = typed.trim();
    match ty {
        Type::String | Type::Picture | Type::Resource if !(t.starts_with('"') && t.ends_with('"') && t.len() >= 2) => super::value::write_str(typed),
        // (an INTEGER that is a Boolean — a form's Enabled: the inspector's
        // check box types True or False)
        Type::Int if matches!(t.to_ascii_lowercase().as_str(), "true" | "false") => format_value_as(Type::Bool, typed, was),
        Type::Bool => {
            let words = was.is_some_and(|w| matches!(w.trim().to_ascii_lowercase().as_str(), "true" | "false"));
            match t.to_ascii_lowercase().as_str() {
                "1" | "-1" | "true" | "yes" => if words { "True" } else { "1" }.into(),
                "0" | "false" | "no" => if words { "False" } else { "0" }.into(),
                _ => t.into(),
            }
        }
        _ => t.into(),
    }
}

/// A RapidQ property's constants (`clRed`, `alClient`, `fsBold + fsItalic`)
/// in a RapidQ program that doesn't define them (no RAPIDQ.INC) written as
/// their number (a colour as `&H` BGR), as RC.EXE needs them.
fn spelled_for(d: &FormDesign, p: &rapidr_lang::Property, v: String) -> String {
    if p.origin != Origin::RapidQ || !matches!(p.ty, Type::Enum | Type::Color | Type::Set) {
        return v;
    }
    let names: Vec<&str> = v.split(['+', '|']).map(str::trim).filter(|s| s.starts_with(|c: char| c.is_ascii_alphabetic())).collect();
    if names.is_empty() || names.iter().all(|n| d.writable_constant(n)) {
        return v;
    }
    match rapidr_lang::eval_constant(&v) {
        Some(n) if p.ty == Type::Color && n >= 0 => format!("&H{n:06X}"),
        Some(n) => n.to_string(),
        None => v,
    }
}

/// The inspector's rows for the components `sel` (the first is the one
/// whose values show).
pub fn inspect(d: &FormDesign, sel: &[NodeId]) -> Inspected {
    let nodes: Vec<_> = sel.iter().filter_map(|&id| d.node(id)).collect();
    let mut out = Inspected { components: nodes.iter().map(|n| n.id).collect(), names: nodes.iter().map(|n| n.name.clone()).collect(), ..Inspected::default() };
    let Some(first) = nodes.first() else { return out };
    if nodes.iter().all(|n| n.canonical == first.canonical) {
        out.type_name = first.type_written.clone();
    }
    let comps: Vec<_> = nodes.iter().filter_map(|n| n.component()).collect();
    let Some(c0) = comps.first() else { return out };
    let shared = |name: &str| comps.iter().all(|c| c.property(name).is_some());
    for p in c0.properties.iter().filter(|p| p.design && crate::designer::inspect::designable(p) && p.indexed == 0 && shared(p.name)) {
        let values: Vec<Option<&str>> = nodes.iter().map(|n| n.prop(p.name)).collect();
        let value = values[0].map(str::to_string);
        let in_code = value.as_deref().is_some_and(|v| super::value::read(v) == super::value::PropValue::Code);
        out.properties.push(PropertyRow {
            name: p.name,
            ty: p.ty,
            values: p.values,
            category: p.category,
            origin: p.origin,
            doc: p.doc,
            default: p.default.as_ref().map(default_text),
            mixed: values.iter().any(|v| *v != values[0]),
            value,
            in_code,
        });
    }
    let shared_event = |name: &str| comps.iter().all(|c| c.event(name).is_some());
    for e in c0.events.iter().filter(|e| shared_event(e.name)) {
        let handlers: Vec<Option<&str>> = nodes.iter().map(|n| n.prop(e.name)).collect();
        out.events.push(EventRow {
            name: e.name,
            params: e.params.iter().map(|p| p.text()).collect::<Vec<_>>().join(", "),
            origin: e.origin,
            doc: e.doc,
            handler: handlers[0].map(str::to_string),
            mixed: handlers.iter().any(|h| *h != handlers[0]),
        });
    }
    out
}

/// The command that sets property `prop` (any case; the registry's
/// spelling is written for a new line) to `typed` on every component in
/// `sel` (`None`: back to its default, the assignment removed). Values are
/// written as [`format_value`] says for the property's type.
pub fn set_value(d: &FormDesign, sel: &[NodeId], prop: &str, typed: Option<&str>) -> Command {
    let mut cmds = Vec::new();
    for &id in sel {
        let Some(n) = d.node(id) else { continue };
        let reg = n.component().and_then(|c| c.property(prop));
        // (an existing line keeps its spelling: the command finds it by key)
        let name = n.props().rev().find(|p| super::model::prop_key(&p.name) == super::model::prop_key(prop)).map(|p| p.name.clone()).or_else(|| reg.map(|p| p.name.to_string())).unwrap_or_else(|| prop.to_string());
        let value = typed.map(|t| match (reg, font_part(prop)) {
            (Some(p), _) => spelled_for(d, p, format_value_as(p.ty, t, n.prop(prop))),
            // (a font's part — `Font.Name = "Arial"`, `Font.Bold = 1` — as
            // its own type; a colour's constant as a Color's)
            (None, Some(ty)) => {
                let v = format_value_as(ty, t, n.prop(prop));
                match n.component().and_then(|c| c.property("Color")).filter(|_| ty == Type::Color) {
                    Some(color) => spelled_for(d, color, v),
                    None => v,
                }
            }
            (None, None) => t.trim().to_string(),
        });
        if n.prop(prop).map(str::to_string) == value {
            continue;
        }
        cmds.push(Command::SetProp { node: id, name, value });
    }
    Command::Batch(cmds)
}

/// The type of a font's part written in a CREATE block (`Font.Name`,
/// `Font.Size`, `Font.Color`, `Font.Bold` …), when `prop` is one.
fn font_part(prop: &str) -> Option<Type> {
    let (font, part) = prop.split_once('.')?;
    if !font.eq_ignore_ascii_case("font") {
        return None;
    }
    Some(match part.to_ascii_lowercase().as_str() {
        "name" => Type::String,
        "size" => Type::Int,
        "color" => Type::Color,
        "bold" | "italic" | "underline" | "strikeout" => Type::Bool,
        _ => return None,
    })
}

/// Whether the inspector edits property `p` of a component's CREATE block:
/// read-write, or a write-only Font — RapidQ's components take a font by
/// its parts there (`Font.Name = "Arial"`, `Font.Size = 12`, `Font.Color`,
/// `Font.Bold`: 47 programs of RapidQ's examples set them so).
pub fn designable(p: &rapidr_lang::Property) -> bool {
    p.access == rapidr_lang::Access::ReadWrite || (p.ty == rapidr_lang::Type::Font && p.access == rapidr_lang::Access::Write)
}

#[cfg(test)]
mod tests {
    use super::super::model::{Prop, SubItem, Subtree};
    use super::*;

    fn form() -> FormDesign {
        let b = |n: &str, cap: &str| Subtree { id: 0, name: n.into(), type_written: "QBUTTON".into(), body: vec![SubItem::Prop(Prop { name: "Caption".into(), value: cap.into() }), SubItem::Prop(Prop { name: "Width".into(), value: "Screen.Width / 4".into() })] };
        FormDesign::from_subtree(Subtree { id: 0, name: "Form".into(), type_written: "QFORM".into(), body: vec![SubItem::Child(b("B1", "\"One\"")), SubItem::Child(b("B2", "\"Two\""))] })
    }

    #[test]
    fn rows_from_the_registry_values_from_the_source() {
        let d = form();
        let (b1, b2) = (d.find("B1").unwrap(), d.find("B2").unwrap());
        let i = inspect(&d, &[b1]);
        assert_eq!(i.type_name, "QBUTTON");
        let cap = i.properties.iter().find(|p| p.name == "Caption").unwrap();
        assert_eq!((cap.value.as_deref(), cap.mixed, cap.ty), (Some("\"One\""), false, Type::String));
        let w = i.properties.iter().find(|p| p.name == "Width").unwrap();
        assert!(w.in_code);
        let anchors = i.properties.iter().find(|p| p.name == "Anchors").expect("RapidR's layout set");
        assert_eq!((anchors.origin, anchors.default.as_deref(), anchors.value.as_deref()), (Origin::RapidR, Some("akLeft + akTop"), None));
        assert!(i.events.iter().any(|e| e.name == "OnClick" && e.handler.is_none()));
        let both = inspect(&d, &[b1, b2]);
        assert!(both.properties.iter().find(|p| p.name == "Caption").unwrap().mixed);
    }

    #[test]
    fn setting_a_value_is_one_undoable_command() {
        let mut d = form();
        let (b1, b2) = (d.find("B1").unwrap(), d.find("B2").unwrap());
        let before = d.clone();
        let cmd = set_value(&d, &[b1, b2], "caption", Some("Go"));
        assert_eq!(cmd.flatten().len(), 2);
        let undo = cmd.apply(&mut d).unwrap();
        assert_eq!(d.node(b2).unwrap().prop("Caption"), Some("\"Go\""));
        undo.apply(&mut d).unwrap();
        assert_eq!(d, before);
        let cmd = set_value(&d, &[b1], "Anchors", Some("akLeft + akTop + akRight"));
        cmd.apply(&mut d).unwrap();
        assert_eq!(d.node(b1).unwrap().int("anchors"), Some(7));
        assert_eq!(format_value(Type::Bool, "yes"), "1");
        assert_eq!(format_value(Type::Bool, "False"), "0");
        assert_eq!(format_value(Type::Int, "True"), "1", "a Boolean kept as an INTEGER");
        assert_eq!(format_value(Type::Int, "42"), "42");
        assert_eq!(format_value_as(Type::Bool, "1", Some("False")), "True", "a line in words keeps them");
        // a font's parts, each as its type
        for (part, typed, written) in [("Font.Name", "Arial", "\"Arial\""), ("Font.Size", "12", "12"), ("Font.Bold", "True", "1"), ("Font.Color", "clBlue", "clBlue")] {
            set_value(&d, &[b1], part, Some(typed)).apply(&mut d).unwrap();
            assert_eq!(d.node(b1).unwrap().prop(part), Some(written), "{part}");
        }
    }
}
