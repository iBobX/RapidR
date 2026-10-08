//! What a property inspector inspects: the small interface between
//! RPROPERTYINSPECTOR and whatever owns the objects it shows — the
//! program's live components ([`Live`]) or a form designer's selection (a
//! designer registers a [`Subject`] maker for its type with
//! [`register_designer`]; RDESIGNSURFACE's is in the inspector's module,
//! RFORMDESIGNER's comes with the designer model, docs/ide-plan.md I4).
//!
//! The inspector never holds the objects: it asks the subject for their
//! names and types, reads each property it shows, and hands every change
//! back — so a designer stays the one owner of its form (its source, its
//! undo), and the inspector works the same on every runtime. A subject
//! is a description (which objects), resolved again on each use; it reaches
//! the runtime through [`Host`] (properties, methods, events).
//!
//! **For a designer** (L-DMODEL / L-DVIEW):
//! 1. `register_designer("RFORMDESIGNER", |designer| Box::new(MySubject { designer: designer.into() }))`
//!    once (the designer's runtime glue, before programs run — its module's
//!    first use is fine);
//! 2. implement [`Subject`] over the designer's model: `objects` is the
//!    selection (name, type as the source writes it); `get` a property's
//!    value as the CREATE block sets it (`None`: not set there — the
//!    registry's default applies; the text as written is fine: `alClient`,
//!    `True`, `&HFF`, `akLeft + akTop`); `set_source` (the value as the
//!    program writes it, and as a runtime keeps it) / `reset` /
//!    `set_handler` become the designer's commands (one undo step each, the
//!    smallest text edit); a font's parts come as `Font.Name`, `Font.Size`,
//!    `Font.Color`, `Font.Bold`, `Font.Italic`, `Font.Underline`,
//!    `Font.StrikeOut`;
//! 3. when the selection or a selected object's property changes, call
//!    [`super::inspector::designer_changed`]`(host, designer)`: every
//!    inspector following that designer reads it again.
//!
//! The program then writes `Inspector1.Designer = Designer1` (or the IDE's
//! shell does), and the inspector follows that designer's selection.

use std::cell::RefCell;

use super::runtime::Runtime;
use crate::Value;

/// The runtime as a subject reaches it (object-safe: every
/// [`Runtime`] is one).
pub trait Host {
    /// A property as the program reads it.
    fn get(&self, name: &str, prop: &str) -> Value;
    /// A property set as by the program.
    fn set(&self, name: &str, prop: &str, v: Value);
    /// A method called as by the program.
    fn call(&self, name: &str, method: &str, args: &[Value]) -> Value;
    /// Its type as the runtime keeps it (`RBUTTON`; "" for none).
    fn type_of(&self, name: &str) -> String;
    /// Every component: (name, type), in creation order.
    fn components(&self) -> Vec<(String, String)>;
    fn fire(&self, name: &str, event: &str, args: &[Value]);
    /// Something shown changed: the windows are drawn again.
    fn invalidate(&self);
}

impl<R: Runtime> Host for R {
    fn get(&self, name: &str, prop: &str) -> Value {
        Runtime::get(*self, name, prop)
    }
    fn set(&self, name: &str, prop: &str, v: Value) {
        Runtime::set(*self, name, prop, v)
    }
    fn call(&self, name: &str, method: &str, args: &[Value]) -> Value {
        Runtime::call(*self, name, method, args)
    }
    fn type_of(&self, name: &str) -> String {
        Runtime::type_of(*self, name)
    }
    fn components(&self) -> Vec<(String, String)> {
        Runtime::components(*self)
    }
    fn fire(&self, name: &str, event: &str, args: &[Value]) {
        Runtime::fire(*self, name, event, args)
    }
    fn invalidate(&self) {
        Runtime::invalidate(*self)
    }
}

/// Objects an inspector shows, and how it reads and changes them.
pub trait Subject {
    /// The objects inspected: (name, type as the program writes it —
    /// `QBUTTON` for a RapidQ component written so, `RPLOT`), the first
    /// the "primary" one. Several: a multi-selection (the inspector shows
    /// the properties they all have).
    fn objects(&self, host: &dyn Host) -> Vec<(String, String)>;

    /// Property `prop` (any case) of `object`: `None` when it isn't set
    /// (the registry's default applies, shown dimmed).
    fn get(&self, host: &dyn Host, object: &str, prop: &str) -> Option<Value>;

    /// Sets `prop` on each of `objects` (one change, one undo step); `Err`
    /// says why it was refused (shown to the user, the old value kept).
    fn set(&self, host: &dyn Host, objects: &[String], prop: &str, value: &Value) -> Result<(), String>;

    /// [`set`](Subject::set) with the value also as the program writes it
    /// (`source`: `alClient`, `akLeft + akTop`, `True`, `clRed`,
    /// `&H0000FF`, a text as it is — what OnPropertyChange says): what the
    /// inspector calls. A designer writes `source` into the CREATE block;
    /// by default, `set` with the runtime's value.
    fn set_source(&self, host: &dyn Host, objects: &[String], prop: &str, value: &Value, source: &str) -> Result<(), String> {
        let _ = source;
        self.set(host, objects, prop, value)
    }

    /// Puts `prop` back to its default on each of `objects` (a designer
    /// removes the assignment from the CREATE block).
    fn reset(&self, host: &dyn Host, objects: &[String], prop: &str);

    /// The SUB event `event` of `object` runs (`None`: none).
    fn handler(&self, host: &dyn Host, object: &str, event: &str) -> Option<String>;

    /// Binds event `event` of each of `objects` to SUB `sub` ("" unbinds).
    fn set_handler(&self, host: &dyn Host, objects: &[String], event: &str, sub: &str);

    /// The components a component-reference property may name: (name,
    /// type) — the form's (a designer's), or the program's.
    fn components(&self, host: &dyn Host) -> Vec<(String, String)>;

    /// What property `prop` of `object` is while it isn't set (`get` gave
    /// `None`), when the subject knows better than the registry's default:
    /// a designer's laid-out Left / Top / Width / Height (a component's
    /// size by its type, an aligned one's place). Shown dimmed, as a
    /// default.
    fn fallback(&self, _host: &dyn Host, _object: &str, _prop: &str) -> Option<Value> {
        None
    }

    /// The SUBs an event may run, as `Name(parameters)` (a designer reads
    /// its source; `None`: the inspector's Handlers property says).
    fn subs(&self, _host: &dyn Host) -> Option<Vec<String>> {
        None
    }
}

/// Makes the subject of designer `name` (its selection).
pub type DesignerSubject = fn(designer: &str) -> Box<dyn Subject>;

thread_local! {
    static DESIGNERS: RefCell<Vec<(&'static str, DesignerSubject)>> = const { RefCell::new(Vec::new()) };
}

/// Designer components of type `type_name` (RapidR's: `RFORMDESIGNER`)
/// give their selection to inspectors through `make`.
pub fn register_designer(type_name: &'static str, make: DesignerSubject) {
    DESIGNERS.with(|d| {
        let mut d = d.borrow_mut();
        d.retain(|(t, _)| !t.eq_ignore_ascii_case(type_name));
        d.push((type_name, make));
    });
}

/// The subject of designer `name` of type `type_name`, when that type
/// registered one.
pub fn designer_subject(type_name: &str, name: &str) -> Option<Box<dyn Subject>> {
    let make = DESIGNERS.with(|d| d.borrow().iter().find(|(t, _)| t.eq_ignore_ascii_case(type_name)).map(|(_, m)| *m))?;
    Some(make(name))
}

/// The program's own components, live: what the program reads and sets.
#[derive(Clone, Debug, PartialEq)]
pub struct Live {
    pub names: Vec<String>,
}

impl Subject for Live {
    fn objects(&self, host: &dyn Host) -> Vec<(String, String)> {
        self.names
            .iter()
            .filter_map(|n| {
                let t = host.type_of(n);
                (!t.is_empty()).then(|| (n.clone(), written_type(&t)))
            })
            .collect()
    }

    fn get(&self, host: &dyn Host, object: &str, prop: &str) -> Option<Value> {
        match host.get(object, prop) {
            Value::Null => None,
            v => Some(v),
        }
    }

    fn set(&self, host: &dyn Host, objects: &[String], prop: &str, value: &Value) -> Result<(), String> {
        for o in objects {
            host.set(o, prop, value.clone());
        }
        Ok(())
    }

    fn reset(&self, host: &dyn Host, objects: &[String], prop: &str) {
        for (o, t) in self.objects(host).into_iter().filter(|(o, _)| objects.iter().any(|x| x.eq_ignore_ascii_case(o))) {
            if let Some(v) = default_value(&t, prop) {
                host.set(&o, prop, v);
            }
        }
    }

    fn handler(&self, host: &dyn Host, object: &str, event: &str) -> Option<String> {
        let v = host.get(object, event).to_string_val();
        (!v.is_empty()).then_some(v)
    }

    fn set_handler(&self, host: &dyn Host, objects: &[String], event: &str, sub: &str) {
        for o in objects {
            host.set(o, event, Value::String(sub.to_string()));
        }
    }

    fn components(&self, host: &dyn Host) -> Vec<(String, String)> {
        host.components()
    }
}

/// A running component's type as RapidR names it (`RBUTTON` →
/// `RButton`: R-NAMES; the runtime keeps no other name).
pub fn written_type(type_name: &str) -> String {
    match rapidr_lang::component(type_name) {
        Some(c) => c.spelling(),
        None => type_name.to_ascii_uppercase(),
    }
}

/// Property `prop`'s default for a component of `type_name`, from the
/// registry, as a value a runtime takes (a constant's number, a Boolean as
/// -1 / 0).
pub fn default_value(type_name: &str, prop: &str) -> Option<Value> {
    use rapidr_lang::DefaultValue as D;
    let p = rapidr_lang::component(type_name)?.property(prop)?;
    Some(match p.default? {
        D::Int(i) => Value::Integer(i),
        D::Float(f) => Value::Double(f),
        D::Bool(b) => Value::Integer(if b { -1 } else { 0 }),
        D::Str(s) => Value::String(s.to_string()),
        D::Expr(e) => Value::Integer(rapidr_lang::eval_constant(e)?),
    })
}
