//! Where the kernel reads components from: the program's property store.
//! The kernel never copies properties into widgets (as FLTK's runtime
//! does); it reads them through [`Store`] when it builds a form's tree,
//! paints and routes input. runtime-core implements it over its component
//! registry (`rp_comp_get`, `rp_comp_type`, `get_children_of`); kernel-drawn
//! dialogs and tests use [`MemStore`].
//!
//! Property names are lowercase, ids are compared without case, type names
//! are RapidR's (`RBUTTON` for RapidQ's QBUTTON). The shared models
//! (TrackBar, TabControl, TextEdit …) are `rapidr_value::objects`', keyed
//! by the same ids.

use std::collections::HashMap;

use rapidr_value::objects::font::Font;
use rapidr_value::Value;

pub trait Store {
    /// A property (`Value::Null` when it has none: the kernel then uses
    /// RapidQ's default).
    fn get(&self, id: &str, prop: &str) -> Value;
    /// Its RapidR type name (`RFORM`, `RBUTTON` …), uppercase.
    fn type_of(&self, id: &str) -> String;
    /// Its children as (id, type name), in creation order.
    fn children(&self, id: &str) -> Vec<(String, String)>;
    /// The font its Font properties describe (`objects::font_from_props`).
    fn font(&self, id: &str) -> Font;
}

/// An integer property, `default` when it has none.
pub fn int(store: &dyn Store, id: &str, prop: &str, default: i64) -> i64 {
    match store.get(id, prop) {
        Value::Null => default,
        v => v.to_i64(),
    }
}

/// A true / false property as RapidQ's runtimes keep them (-1 / 0,
/// booleans, "0" / "False" strings), `default` when it has none.
pub fn flag(store: &dyn Store, id: &str, prop: &str, default: bool) -> bool {
    match store.get(id, prop) {
        Value::Null => default,
        Value::String(s) => {
            let s = s.trim();
            !(s.is_empty() || s == "0" || s.eq_ignore_ascii_case("false"))
        }
        v => v.to_bool(),
    }
}

pub fn string(store: &dyn Store, id: &str, prop: &str) -> String {
    match store.get(id, prop) {
        Value::Null => String::new(),
        v => v.to_string_val(),
    }
}

struct MemComp {
    type_name: String,
    parent: Option<String>,
    props: HashMap<String, Value>,
}

/// A store in memory: kernel-drawn dialogs' forms (a MessageBox's label and
/// buttons) and tests. Components are added in creation order; adding one
/// whose type has a shared model makes the model (`objects::create`), new.
#[derive(Default)]
pub struct MemStore {
    comps: HashMap<String, MemComp>,
    order: Vec<String>,
}

impl MemStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds component `id` of `type_name` (`RBUTTON` …) on `parent` (`None`
    /// for a form).
    pub fn add(&mut self, id: &str, type_name: &str, parent: Option<&str>) -> &mut Self {
        let key = id.to_lowercase();
        // (a model left from an earlier store with this id goes)
        rapidr_value::objects::remove(&key);
        rapidr_value::objects::create(&key, &type_name.to_ascii_uppercase());
        if !self.comps.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.comps.insert(key, MemComp { type_name: type_name.to_ascii_uppercase(), parent: parent.map(str::to_lowercase), props: HashMap::new() });
        self
    }

    /// Sets a property (a shared model's goes to the model, as the
    /// runtimes route it).
    pub fn set(&mut self, id: &str, prop: &str, value: Value) -> &mut Self {
        let (key, prop) = (id.to_lowercase(), prop.to_lowercase());
        if rapidr_value::objects::set(&key, &prop, &value).is_some_and(|r| r.is_ok()) {
            return self;
        }
        if let Some(c) = self.comps.get_mut(&key) {
            c.props.insert(prop, value);
        }
        self
    }

    /// Calls a shared model's method (`AddTabs` …).
    pub fn call(&mut self, id: &str, method: &str, args: &[Value]) -> Option<Value> {
        let reader = |_: &str, _: &str| Value::Null;
        rapidr_value::objects::call(&id.to_lowercase(), &method.to_lowercase(), args, &reader).and_then(Result::ok)
    }

    /// Every component's id, in creation order.
    pub fn ids(&self) -> &[String] {
        &self.order
    }

    /// Forgets the components and their models (a dialog closed).
    pub fn clear(&mut self) {
        for id in &self.order {
            rapidr_value::objects::remove(id);
        }
        self.comps.clear();
        self.order.clear();
    }
}

impl Store for MemStore {
    fn get(&self, id: &str, prop: &str) -> Value {
        let (key, prop) = (id.to_lowercase(), prop.to_lowercase());
        if let Some(v) = self.comps.get(&key).and_then(|c| c.props.get(&prop)) {
            return v.clone();
        }
        rapidr_value::objects::get(&key, &prop).unwrap_or(Value::Null)
    }

    fn type_of(&self, id: &str) -> String {
        self.comps.get(&id.to_lowercase()).map(|c| c.type_name.clone()).unwrap_or_default()
    }

    fn children(&self, id: &str) -> Vec<(String, String)> {
        let key = id.to_lowercase();
        self.order.iter().filter(|c| self.comps[*c].parent.as_deref() == Some(key.as_str())).map(|c| (c.clone(), self.comps[c].type_name.clone())).collect()
    }

    fn font(&self, id: &str) -> Font {
        rapidr_value::objects::font_from_props(id, &|i, p| self.get(i, p))
    }
}
