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
/// quotes (unless already a literal), Booleans as `True` / `False`, the
/// rest as typed (numbers, constants, `&H` colours).
pub fn format_value(ty: Type, typed: &str) -> String {
    let t = typed.trim();
    match ty {
        Type::String | Type::Picture | Type::Resource if !(t.starts_with('"') && t.ends_with('"') && t.len() >= 2) => super::value::write_str(typed),
        Type::Bool => match t.to_ascii_lowercase().as_str() {
            "1" | "-1" | "true" | "yes" => "True".into(),
            "0" | "false" | "no" => "False".into(),
            _ => t.into(),
        },
        _ => t.into(),
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
    for p in c0.properties.iter().filter(|p| p.design && p.access == rapidr_lang::Access::ReadWrite && p.indexed == 0 && shared(p.name)) {
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
        let value = typed.map(|t| reg.map_or_else(|| t.trim().to_string(), |p| format_value(p.ty, t)));
        if n.prop(prop).map(str::to_string) == value {
            continue;
        }
        cmds.push(Command::SetProp { node: id, name, value });
    }
    Command::Batch(cmds)
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
        assert_eq!(format_value(Type::Bool, "yes"), "True");
    }
}
