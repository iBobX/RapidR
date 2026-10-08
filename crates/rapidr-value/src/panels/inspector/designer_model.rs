//! The designer model (`crate::designer::Designer`, lane L-DMODEL) as an
//! inspector's subject: its selection's components and their values as the
//! CREATE block writes them, each change one undoable command over the
//! whole selection ([`Designer::set_property`]; `None` puts the default
//! back by removing the line).
//!
//! Two places hold a designer model: an RDESIGNSURFACE (its own,
//! [`Source::Surface`], registered by [`super::design`]), and any other
//! designer component's owner (L-SHELL / L-DVIEW: RFORMDESIGNER), which
//! makes its model reachable by the component's name and registers the type
//! once:
//!
//! ```ignore
//! designer_model::attach("FormDesigner1", Rc::clone(&designer));
//! designer_model::register("RFORMDESIGNER");
//! // … after each selection change or command:
//! panels::inspector::designer_changed(host, "FormDesigner1");
//! ```
//!
//! Then `Inspector.Designer = FormDesigner1` inspects its selection.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::designer::value::{read, PropValue};
use crate::designer::Designer;
use crate::panels::subject::{Host, Subject};
use crate::Value;

thread_local! {
    static DESIGNERS: RefCell<HashMap<String, Rc<RefCell<Designer>>>> = RefCell::new(HashMap::new());
}

/// Makes `designer` the model of designer component `name`.
pub fn attach(name: &str, designer: Rc<RefCell<Designer>>) {
    DESIGNERS.with(|d| d.borrow_mut().insert(name.to_ascii_lowercase(), designer));
}

/// Forgets designer component `name`'s model.
pub fn detach(name: &str) {
    DESIGNERS.with(|d| d.borrow_mut().remove(&name.to_ascii_lowercase()));
}

/// Designer components of `type_name` give their selection to inspectors
/// through their attached model.
pub fn register(type_name: &'static str) {
    crate::panels::subject::register_designer(type_name, |d| Box::new(DesignerModelSubject { source: Source::Attached(d.to_string()) }));
}

/// Where a subject's designer model is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// Attached by its owner ([`attach`]) under this name.
    Attached(String),
    /// RDESIGNSURFACE `name`'s own (`objects::design`).
    Surface(String),
}

/// The selection of a designer model.
pub struct DesignerModelSubject {
    pub source: Source,
}

impl DesignerModelSubject {
    /// Runs `f` on the designer model (`None`: there is none).
    fn with<R>(&self, f: impl FnOnce(&mut Designer) -> R) -> Option<R> {
        match &self.source {
            Source::Attached(name) => {
                let m = DESIGNERS.with(|d| d.borrow().get(&name.to_ascii_lowercase()).cloned())?;
                let r = f(&mut m.borrow_mut());
                Some(r)
            }
            // (a change to an RDESIGNSURFACE's model is written into its
            // code at once: `commit`, its edits waiting in its outbox)
            Source::Surface(name) => crate::objects::with_design_mut(name, |s| {
                let r = f(&mut s.designer);
                s.commit();
                r
            }),
        }
    }

    /// A property's (or event's) text as the CREATE block writes it, on
    /// `object`.
    fn text(&self, object: &str, prop: &str) -> Option<String> {
        self.with(|d| {
            let id = d.design.find(object)?;
            d.design.node(id)?.prop(prop).map(str::to_string)
        })
        .flatten()
    }

    fn put(&self, host: &dyn Host, prop: &str, typed: Option<&str>) -> Result<(), String> {
        let r = self.with(|d| d.set_property(prop, typed).map_err(|e| format!("{e:?}"))).unwrap_or_else(|| Err("No designer".into()));
        // (an RDESIGNSURFACE's edits heard by the program — OnSourceEdit
        // for the code editor, then OnChange — as its own changes are)
        if let Source::Surface(name) = &self.source {
            for e in crate::objects::take_design_events(name) {
                host.fire(name, e.event(), &e.args());
            }
        }
        r
    }
}

impl Subject for DesignerModelSubject {
    fn objects(&self, _host: &dyn Host) -> Vec<(String, String)> {
        self.with(|d| d.selection.ids().iter().filter_map(|&id| d.design.node(id)).map(|n| (n.name.clone(), n.type_written.clone())).collect()).unwrap_or_default()
    }

    /// The CREATE block's text read as a value: a string's text, a number,
    /// a constant's name as written (`alClient`, `akLeft + akTop`); an
    /// expression the designer can't read, as written.
    fn get(&self, _host: &dyn Host, object: &str, prop: &str) -> Option<Value> {
        let text = self.text(object, prop)?;
        Some(match read(&text) {
            PropValue::Str(s) => Value::String(s),
            PropValue::Number(n) if text.trim().chars().next().is_some_and(|c| c.is_ascii_digit() || c == '-' || c == '.') => {
                if n.fract() == 0.0 {
                    Value::Integer(n as i64)
                } else {
                    Value::Double(n)
                }
            }
            _ => Value::String(text.trim().to_string()),
        })
    }

    fn set(&self, _host: &dyn Host, _objects: &[String], prop: &str, value: &Value) -> Result<(), String> {
        self.put(_host, prop, Some(&value.to_string_val()))
    }

    /// The selection is the designer's: the value as the program writes it
    /// becomes one command over all of it.
    fn set_source(&self, _host: &dyn Host, _objects: &[String], prop: &str, _value: &Value, source: &str) -> Result<(), String> {
        self.put(_host, prop, Some(source))
    }

    fn reset(&self, _host: &dyn Host, _objects: &[String], prop: &str) {
        let _ = self.put(_host, prop, None);
    }

    fn handler(&self, _host: &dyn Host, object: &str, event: &str) -> Option<String> {
        self.text(object, event).filter(|s| !s.trim().is_empty())
    }

    fn set_handler(&self, _host: &dyn Host, _objects: &[String], event: &str, sub: &str) {
        let _ = self.put(_host, event, if sub.is_empty() { None } else { Some(sub) });
    }

    /// Left, Top, Width, Height not written in the CREATE block: as the
    /// designer lays the component out (its type's size, its Align).
    fn fallback(&self, _host: &dyn Host, object: &str, prop: &str) -> Option<Value> {
        let p = prop.to_ascii_lowercase();
        if !matches!(p.as_str(), "left" | "top" | "width" | "height") {
            return None;
        }
        self.with(|d| {
            let id = d.design.find(object)?;
            let r = d.layout().rect(id)?;
            Some(Value::Integer(match p.as_str() {
                "left" => r.left,
                "top" => r.top,
                "width" => r.width,
                _ => r.height,
            }))
        })
        .flatten()
    }

    /// The SUBs of the code an RDESIGNSURFACE designs, as
    /// `Name(parameters)` (the Events page offers those that fit).
    fn subs(&self, _host: &dyn Host) -> Option<Vec<String>> {
        let Source::Surface(name) = &self.source else { return None };
        let text = crate::objects::with_design_mut(name, |s| s.get("source").map(|v| v.to_string_val())).flatten()?;
        Some(sub_headers(&text))
    }

    fn components(&self, _host: &dyn Host) -> Vec<(String, String)> {
        self.with(|d| {
            let root = d.design.root();
            d.design.ids().into_iter().filter(|&id| id != root).filter_map(|id| d.design.node(id)).map(|n| (n.name.clone(), n.type_written.clone())).collect()
        })
        .unwrap_or_default()
    }
}

/// The SUBs a program's text defines, as `Name(parameters)` (the
/// parameters' names: `Key, Shift`).
pub fn sub_headers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        if t.len() < 4 || !t[..4].eq_ignore_ascii_case("sub ") {
            continue;
        }
        let rest = t[4..].trim();
        let (name, params) = match rest.find('(') {
            Some(i) => (rest[..i].trim(), rest[i + 1..].rsplit_once(')').map_or("", |p| p.0)),
            None => (rest.split_whitespace().next().unwrap_or(""), ""),
        };
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let names: Vec<String> = params
            .split(',')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| {
                let p = p.strip_prefix("BYREF ").or_else(|| p.strip_prefix("byref ")).unwrap_or(p);
                p.split_whitespace().next().unwrap_or("").to_string()
            })
            .collect();
        out.push(format!("{name}({})", names.join(", ")));
    }
    out
}

#[cfg(test)]
mod sub_tests {
    #[test]
    fn sub_headers_of_a_program() {
        let t = "DECLARE SUB A\nSUB Greet\nEND SUB\n  sub Key (BYREF K AS WORD, Shift AS INTEGER)\nFUNCTION F(x)\n";
        assert_eq!(super::sub_headers(t), ["Greet()", "Key(K, Shift)"]);
    }
}
