//! The inspector's model without a runtime: a host of plain property maps.

use std::cell::RefCell;
use std::collections::HashMap;

use super::model::{RowKind, EXTENSIONS};
use super::values::{self, Kind};
use super::*;
use crate::panels::subject::Host;

/// Components as maps of properties; the events fired, in order.
#[derive(Default)]
struct Fake {
    types: RefCell<Vec<(String, String)>>,
    props: RefCell<HashMap<(String, String), Value>>,
    fired: RefCell<Vec<String>>,
}

impl Fake {
    fn with(comps: &[(&str, &str)]) -> Fake {
        let f = Fake::default();
        for (n, t) in comps {
            f.types.borrow_mut().push((n.to_string(), t.to_string()));
        }
        f
    }

    fn prop(&self, name: &str, prop: &str) -> Value {
        self.props.borrow().get(&(name.to_ascii_lowercase(), prop.to_ascii_lowercase())).cloned().unwrap_or(Value::Null)
    }
}

impl Host for Fake {
    fn get(&self, name: &str, prop: &str) -> Value {
        self.prop(name, prop)
    }
    fn set(&self, name: &str, prop: &str, v: Value) {
        self.props.borrow_mut().insert((name.to_ascii_lowercase(), prop.to_ascii_lowercase()), v);
    }
    fn call(&self, _name: &str, _method: &str, _args: &[Value]) -> Value {
        Value::Null
    }
    fn type_of(&self, name: &str) -> String {
        self.types.borrow().iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, t)| t.clone()).unwrap_or_default()
    }
    fn components(&self) -> Vec<(String, String)> {
        self.types.borrow().clone()
    }
    fn fire(&self, name: &str, event: &str, args: &[Value]) {
        let args: Vec<String> = args.iter().map(Value::to_string_val).collect();
        self.fired.borrow_mut().push(format!("{name}.{event}({})", args.join(", ")));
    }
    fn invalidate(&self) {}
}

/// Inspector `id` inspecting `target` through `host`.
fn inspect(host: &Fake, id: &str, target: &str) {
    remove(id);
    with_mut(id, |m| m.target = target.into());
    refresh(host, id);
}

#[test]
fn values_are_spelled_three_ways() {
    let align = Kind::Enum(["alNone", "alTop", "alBottom", "alLeft", "alRight", "alClient"].map(String::from).to_vec());
    assert_eq!(values::display(&align, &Value::Integer(5)), "alClient");
    assert_eq!(values::parse(&align, "alclient"), Ok(Value::Integer(5)));
    assert!(values::parse(&align, "alMiddle").is_err());
    let anchors = Kind::Set(["akLeft", "akTop", "akRight", "akBottom"].map(String::from).to_vec());
    assert_eq!(values::display(&anchors, &Value::Integer(3)), "akLeft, akTop");
    assert_eq!(values::source(&anchors, &Value::Integer(7)), "akLeft + akTop + akRight");
    assert_eq!(values::parse(&anchors, "akLeft, akBottom"), Ok(Value::Integer(9)));
    assert_eq!(values::parse(&anchors, "[akTop]"), Ok(Value::Integer(2)));
    // (a designer's text normalizes as the runtime's number would)
    assert_eq!(values::normalize(&anchors, &v_str("akLeft + akTop")), Some(Value::Integer(3)));
    assert_eq!(values::display(&Kind::Color, &Value::Integer(255)), "clRed");
    assert_eq!(values::display(&Kind::Color, &Value::Integer(0x123456)), "&H123456");
    assert_eq!(values::display(&Kind::Color, &Value::Integer(-2147483633)), "clBtnFace");
    assert_eq!(values::parse(&Kind::Color, "#FF0000"), Ok(Value::Integer(255)));
    assert_eq!(values::parse(&Kind::Color, "&H00FF00"), Ok(Value::Integer(0xFF00)));
    assert_eq!(values::parse(&Kind::Bool, "yes"), Ok(Value::Integer(-1)));
    assert_eq!(values::display(&Kind::Bool, &v_str("False")), "False");
    assert!(values::parse(&Kind::Int, "12x").is_err());
    assert_eq!(values::parse(&Kind::Int, "&H10"), Ok(Value::Integer(16)));
    assert_eq!(values::display(&Kind::Float, &Value::Double(2.5)), "2.5");
    assert_eq!(values::display(&Kind::Strings, &v_str("a\r\nb")), "2 lines");
    assert_eq!(Kind::named("a|b|c"), Kind::Enum(vec!["a".into(), "b".into(), "c".into()]));
    assert_eq!(Kind::named("set:x|y"), Kind::Set(vec!["x".into(), "y".into()]));
}

#[test]
fn rows_come_from_the_registry() {
    let host = Fake::with(&[("Button1", "QBUTTON")]);
    host.set("Button1", "caption", v_str("OK"));
    inspect(&host, "insp", "Button1");
    let m = with("insp", Clone::clone).unwrap();
    assert_eq!(m.snap.type_name, "QBUTTON");
    // (design-time, read-write, not indexed: Caption yes; Handle, Parent no;
    // the write-only Font by its parts, as RapidQ's programs set it)
    let names: Vec<&str> = m.snap.props.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"Caption") && names.contains(&"Anchors") && names.contains(&"Align") && names.contains(&"Font"));
    assert!(!names.contains(&"Handle") && !names.contains(&"Parent"));
    // (categories A–Z, RapidR's own members last, with their badge)
    let rows = m.rows();
    let cats: Vec<&str> = rows.iter().filter(|r| r.kind == RowKind::Category).map(|r| r.name.as_str()).collect();
    assert_eq!(cats.last(), Some(&EXTENSIONS));
    let mut sorted = cats[..cats.len() - 1].to_vec();
    sorted.sort_by_key(|c| c.to_ascii_lowercase());
    assert_eq!(&cats[..cats.len() - 1], &sorted[..]);
    let anchors = m.snap.props.iter().find(|p| p.name == "Anchors").unwrap();
    assert!(anchors.ext && anchors.category == EXTENSIONS);
    assert_eq!(anchors.text(), "akLeft, akTop");
    assert!(anchors.is_default);
    let caption = m.snap.props.iter().find(|p| p.name == "Caption").unwrap();
    assert_eq!((caption.text(), caption.is_default), ("OK".to_string(), false));
    // (extensions hidden)
    with_mut("insp", |m| m.show_ext = false);
    assert!(!with("insp", |m| m.rows().iter().any(|r| r.name == "Anchors")).unwrap());
    // (A–Z: no headings; the search marks what it found)
    with_mut("insp", |m| {
        m.show_ext = true;
        m.alphabetic = true;
        m.filter = "cap".into();
    });
    let rows = with("insp", |m| m.rows()).unwrap();
    assert_eq!(rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["Caption"]);
    assert_eq!(rows[0].marks, vec![0, 1, 2]);
    assert_eq!(rows[0].describe(), "Caption=OK");
    // (events: OnClick, the SUB it runs)
    host.set("Button1", "onclick", v_str("Button1Click"));
    refresh(&host, "insp");
    with_mut("insp", |m| {
        m.events_page = true;
        m.filter.clear();
    });
    let rows = with("insp", |m| m.rows()).unwrap();
    assert!(rows.iter().any(|r| r.key == "@onclick" && r.text == "Button1Click"));
}

#[test]
fn a_multi_selection_shows_what_they_share() {
    let host = Fake::with(&[("B1", "QBUTTON"), ("B2", "QBUTTON"), ("E1", "QEDIT")]);
    host.set("B1", "caption", v_str("One"));
    host.set("B2", "caption", v_str("Two"));
    host.set("B1", "width", Value::Integer(75));
    host.set("B2", "width", Value::Integer(75));
    inspect(&host, "multi", "B1, B2");
    let m = with("multi", Clone::clone).unwrap();
    let caption = m.snap.props.iter().find(|p| p.name == "Caption").unwrap();
    assert_eq!(caption.value, None, "values differ: blank");
    assert_eq!(m.snap.props.iter().find(|p| p.name == "Width").unwrap().text(), "75");
    // (a change goes to every one)
    assert!(commit(&host, "multi", "caption", "Same").is_ok());
    assert_eq!((host.prop("B1", "caption"), host.prop("B2", "caption")), (v_str("Same"), v_str("Same")));
    assert_eq!(host.fired.borrow().last().unwrap(), "multi.onpropertychange(Caption, Same)");
    // (a button and an edit: only the properties both have; no one type)
    inspect(&host, "multi", "B1,E1");
    let m = with("multi", Clone::clone).unwrap();
    assert_eq!(m.snap.type_name, "");
    assert!(m.snap.props.iter().all(|p| p.name != "Caption" && p.name != "Text"));
    assert!(m.snap.props.iter().any(|p| p.name == "Left"));
}

#[test]
fn changes_defaults_and_reset() {
    let host = Fake::with(&[("Button1", "QBUTTON")]);
    inspect(&host, "rs", "Button1");
    // (a flag toggled: the set written as the program writes it)
    assert!(commit(&host, "rs", "anchors.akright", "True").is_ok());
    assert_eq!(host.prop("Button1", "anchors"), Value::Integer(7));
    assert_eq!(host.fired.borrow().last().unwrap(), "rs.onpropertychange(Anchors, akLeft + akTop + akRight)");
    assert!(!with("rs", |m| m.is_default("anchors")).unwrap());
    // (the pin editor's row commits the whole set)
    assert!(commit(&host, "rs", "anchors#pins", "akLeft, akBottom").is_ok());
    assert_eq!(host.prop("Button1", "anchors"), Value::Integer(9));
    reset(&host, "rs", "anchors");
    assert_eq!(host.prop("Button1", "anchors"), Value::Integer(3));
    assert!(with("rs", |m| m.is_default("anchors")).unwrap());
    assert_eq!(host.fired.borrow().last().unwrap(), "rs.onpropertychange(Anchors, akLeft + akTop)");
    // (an enum by its name; a wrong one refused, the row says why)
    assert!(commit(&host, "rs", "align", "alClient").is_ok());
    assert_eq!(host.prop("Button1", "align"), Value::Integer(5));
    assert!(commit(&host, "rs", "align", "alMiddle").is_err());
    assert!(with("rs", |m| m.ui.error.clone()).unwrap().is_some_and(|(k, _)| k == "align"));
    assert!(commit(&host, "rs", "width", "wide").is_err());
    // (a Boolean; a colour)
    assert!(commit(&host, "rs", "showhint", "True").is_ok());
    assert_eq!(host.prop("Button1", "showhint"), Value::Integer(-1));
    assert!(commit(&host, "rs", "color", "clRed").is_ok());
    assert_eq!(with("rs", |m| m.value_text("color")).flatten().as_deref(), Some("clRed"));
    // (a property without a registry default goes back to its empty value)
    assert!(commit(&host, "rs", "caption", "Go").is_ok());
    reset(&host, "rs", "caption");
    assert_eq!(host.prop("Button1", "caption"), v_str(""));
    // (read-only: nothing changes)
    with_mut("rs", |m| m.read_only = true);
    assert!(commit(&host, "rs", "caption", "No").is_err());
}

#[test]
fn events_bind_a_fitting_sub() {
    let host = Fake::with(&[("Button1", "QBUTTON")]);
    inspect(&host, "ev", "Button1");
    with_mut("ev", |m| m.handlers = "Button1Click(Sender AS QBUTTON)\nKeyed(Key AS WORD, Shift AS INTEGER)\nOther(a, b, c)".into());
    let fits = with("ev", |m| m.fitting_subs(None, 0)).unwrap();
    assert_eq!(fits, ["Button1Click"]);
    let fits = with("ev", |m| m.fitting_subs(None, 2)).unwrap();
    assert_eq!(fits, ["Keyed", "Other"]);
    assert!(commit(&host, "ev", "@onclick", "Button1Click").is_ok());
    assert_eq!(host.prop("Button1", "onclick"), v_str("Button1Click"));
    assert_eq!(host.fired.borrow().last().unwrap(), "ev.onpropertychange(OnClick, Button1Click)");
    assert!(commit(&host, "ev", "@onclick", "not a name").is_err());
}

#[test]
fn the_programs_own_properties() {
    let host = Fake::default();
    remove("own");
    with_mut("own", |_| ());
    let call = |m: &str, args: &[&str]| {
        let args: Vec<Value> = args.iter().map(|a| v_str(a)).collect();
        // (through the model directly: a Runtime isn't needed for these)
        match m {
            "addproperty" => {
                let kind = Kind::named(&args[1].to_string_val());
                let value = values::parse(&kind, &args[2].to_string_val()).unwrap();
                with_mut("own", |x| x.custom.push(Custom { name: args[0].to_string_val(), kind, default: value.clone(), value, category: args.get(3).map(|v| v.to_string_val()).unwrap_or_default() }));
                refresh(&host, "own");
            }
            _ => unreachable!(),
        }
    };
    call("addproperty", &["Speed", "int", "5", "Engine"]);
    call("addproperty", &["Mode", "fast|slow", "fast"]);
    let rows = with("own", |m| m.rows()).unwrap();
    assert!(rows.iter().any(|r| r.kind == RowKind::Category && r.name == "Engine"));
    assert!(commit(&host, "own", "speed", "9").is_ok());
    assert_eq!(host.fired.borrow().last().unwrap(), "own.onpropertychange(Speed, 9)");
    assert!(commit(&host, "own", "mode", "slow").is_ok());
    assert_eq!(with("own", |m| m.value_text("mode")).flatten().as_deref(), Some("slow"));
    assert!(!with("own", |m| m.is_default("mode")).unwrap());
    reset(&host, "own", "mode");
    assert_eq!(with("own", |m| m.value_text("mode")).flatten().as_deref(), Some("fast"));
}

#[test]
fn a_designers_selection_is_inspected_there() {
    let host = Fake::with(&[("Surface", "RDESIGNSURFACE")]);
    crate::objects::create("surface", "RDESIGNSURFACE");
    crate::objects::with_design_mut("surface", |d| {
        d.add("QBUTTON", "OkButton", (8, 8, 75, 25));
        d.add("QEDIT", "NameEdit", (8, 40, 120, 21));
        d.select(0);
    });
    remove("di");
    with_mut("di", |m| m.designer = "Surface".into());
    refresh(&host, "di");
    let m = with("di", Clone::clone).unwrap();
    assert_eq!(m.snap.objects, vec![("OkButton".to_string(), "QBUTTON".to_string())]);
    assert_eq!(m.value_text("caption").as_deref(), Some("OkButton"));
    assert_eq!(m.value_text("left").as_deref(), Some("8"));
    // (a change is the surface's: SetProp's text, as the program writes it)
    assert!(commit(&host, "di", "align", "alTop").is_ok());
    assert!(commit(&host, "di", "anchors.akright", "True").is_ok());
    assert!(commit(&host, "di", "width", "90").is_ok());
    crate::objects::with_design("surface", |d| {
        assert_eq!(d.components()[0].prop("align"), Some("alTop"));
        assert_eq!(d.components()[0].prop("anchors"), Some("akLeft + akTop + akRight"));
        assert_eq!(d.components()[0].prop("width"), Some("90"));
    });
    assert_eq!(with("di", |m| m.value_text("anchors")).flatten().as_deref(), Some("akLeft, akTop, akRight"));
    // (the selection changes: designer_changed reads it again)
    crate::objects::with_design_mut("surface", |d| d.select(1));
    designer_changed(&host, "surface");
    assert_eq!(with("di", |m| m.snap.type_name.clone()).unwrap(), "QEDIT");
    assert!(commit(&host, "di", "hint", "Your name").is_ok());
    crate::objects::with_design("surface", |d| assert_eq!(d.components()[1].prop("hint"), Some("Your name")));
    reset(&host, "di", "hint");
    crate::objects::with_design("surface", |d| assert_eq!(d.components()[1].prop("hint"), None));
}

#[test]
fn the_designer_models_selection_one_command_each() {
    use crate::designer::model::{FormDesign, Prop as DProp, SubItem, Subtree};
    use std::rc::Rc;
    let b = |n: &str, cap: &str| Subtree { id: 0, name: n.into(), type_written: "QBUTTON".into(), body: vec![SubItem::Child(Subtree { id: 0, name: String::new(), type_written: String::new(), body: Vec::new() }); 0].into_iter().chain([SubItem::Prop(DProp { name: "Caption".into(), value: cap.into() })]).collect() };
    let design = FormDesign::from_subtree(Subtree { id: 0, name: "Form".into(), type_written: "QFORM".into(), body: vec![SubItem::Child(b("B1", "\"One\"")), SubItem::Child(b("B2", "\"Two\""))] });
    let (b1, b2) = (design.find("B1").unwrap(), design.find("B2").unwrap());
    let d = Rc::new(RefCell::new(crate::designer::Designer::new(design)));
    d.borrow_mut().selection.set(b1);
    super::designer_model::attach("fd1", Rc::clone(&d));
    super::designer_model::register("RFORMDESIGNER");
    let host = Fake::with(&[("FD1", "RFORMDESIGNER")]);
    remove("dm");
    with_mut("dm", |m| m.designer = "FD1".into());
    refresh(&host, "dm");
    assert_eq!(with("dm", |m| m.value_text("caption")).flatten().as_deref(), Some("One"));
    // (a change: one command, the CREATE block's text as the program writes it)
    assert!(commit(&host, "dm", "caption", "Go").is_ok());
    assert!(commit(&host, "dm", "anchors.akright", "True").is_ok());
    let node = |id| d.borrow().design.node(id).map(|n| (n.prop("Caption").map(str::to_string), n.prop("Anchors").map(str::to_string)));
    assert_eq!(node(b1), Some((Some("\"Go\"".into()), Some("akLeft + akTop + akRight".into()))));
    assert_eq!(with("dm", |m| m.value_text("anchors")).flatten().as_deref(), Some("akLeft, akTop, akRight"));
    // (both selected: Caption differs, a change goes to both; undo is the designer's)
    d.borrow_mut().selection.toggle(b2);
    designer_changed(&host, "fd1");
    assert_eq!(with("dm", |m| m.snap.props.iter().find(|p| p.name == "Caption").unwrap().value.clone()).unwrap(), None);
    assert!(commit(&host, "dm", "caption", "Same").is_ok());
    assert_eq!(node(b2).unwrap().0.as_deref(), Some("\"Same\""));
    assert!(d.borrow_mut().undo());
    assert_eq!(node(b2).unwrap().0.as_deref(), Some("\"Two\""));
    // (reset: the line goes)
    reset(&host, "dm", "anchors");
    assert_eq!(node(b1).unwrap().1, None);
    super::designer_model::detach("fd1");
}
