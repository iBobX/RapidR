//! RDESIGNSURFACE as an inspector's subject (`Inspector.Designer =
//! Surface1`): its selected component, its properties as the surface keeps
//! them (SetProp's strings, lowercase names; Left / Top / Width / Height its
//! bounds), every change made in the surface's model — the same model its
//! program reads with GetProp, so the program sees what the user set.

use crate::objects::{with_design, with_design_mut};
use crate::panels::subject::{written_type, Host, Subject};
use crate::Value;

/// The selection of RDESIGNSURFACE `designer`.
pub struct DesignSubject {
    pub designer: String,
}

/// Registers RDESIGNSURFACE's subject (done once, at the inspector's first
/// use of a designer).
pub fn register() {
    crate::panels::subject::register_designer("RDESIGNSURFACE", |d| Box::new(DesignSubject { designer: d.to_string() }));
}

/// A bound's property: its place in (x, y, w, h).
fn bound(prop: &str) -> Option<usize> {
    ["left", "top", "width", "height"].iter().position(|b| b.eq_ignore_ascii_case(prop))
}

impl DesignSubject {
    fn index(&self, object: &str) -> Option<usize> {
        with_design(&self.designer, |d| d.components.iter().position(|c| c.name.eq_ignore_ascii_case(object))).flatten()
    }

    /// Sets `prop` of `objects` to `text` (as SetProp stores it).
    fn put(&self, objects: &[String], prop: &str, text: Option<&str>) {
        for o in objects {
            let Some(i) = self.index(o) else { continue };
            with_design_mut(&self.designer, |d| {
                let c = &mut d.components[i];
                match (bound(prop), text) {
                    (Some(b), Some(t)) => {
                        let v = super::values::parse_int(t).unwrap_or(0);
                        match b {
                            0 => c.x = v,
                            1 => c.y = v,
                            2 => c.w = v.max(1),
                            _ => c.h = v.max(1),
                        }
                    }
                    (Some(_), None) => {}
                    (None, Some(t)) => {
                        c.props.insert(prop.to_ascii_lowercase(), t.to_string());
                    }
                    (None, None) => {
                        c.props.remove(&prop.to_ascii_lowercase());
                    }
                }
            });
        }
    }
}

impl Subject for DesignSubject {
    fn objects(&self, _host: &dyn Host) -> Vec<(String, String)> {
        with_design(&self.designer, |d| d.selection().map(|i| d.components[i].clone()))
            .flatten()
            .map(|c| vec![(c.name.clone(), written_type(&c.type_name))])
            .unwrap_or_default()
    }

    fn get(&self, _host: &dyn Host, object: &str, prop: &str) -> Option<Value> {
        let i = self.index(object)?;
        with_design(&self.designer, |d| {
            let c = &d.components[i];
            match bound(prop) {
                Some(b) => Some(Value::Integer([c.x, c.y, c.w, c.h][b])),
                None => c.prop(&prop.to_ascii_lowercase()).map(|s| Value::String(s.to_string())),
            }
        })
        .flatten()
    }

    fn set(&self, _host: &dyn Host, objects: &[String], prop: &str, value: &Value) -> Result<(), String> {
        self.put(objects, prop, Some(&value.to_string_val()));
        Ok(())
    }

    fn set_source(&self, _host: &dyn Host, objects: &[String], prop: &str, _value: &Value, source: &str) -> Result<(), String> {
        self.put(objects, prop, Some(source));
        Ok(())
    }

    fn reset(&self, _host: &dyn Host, objects: &[String], prop: &str) {
        self.put(objects, prop, None);
    }

    fn handler(&self, host: &dyn Host, object: &str, event: &str) -> Option<String> {
        self.get(host, object, event).map(|v| v.to_string_val()).filter(|s| !s.is_empty())
    }

    fn set_handler(&self, _host: &dyn Host, objects: &[String], event: &str, sub: &str) {
        self.put(objects, event, if sub.is_empty() { None } else { Some(sub) });
    }

    fn components(&self, _host: &dyn Host) -> Vec<(String, String)> {
        with_design(&self.designer, |d| d.components.iter().map(|c| (c.name.clone(), written_type(&c.type_name))).collect()).unwrap_or_default()
    }
}
