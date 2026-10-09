//! The panels through a runtime — the same glue for the desktop
//! (runtime-core's `panels.rs`) and the web (runtime-web's `panels_web.rs`),
//! as `crate::dock::runtime`: a panel's methods and properties, what the
//! user did ([`super::User`], from the kernel), and the program's events,
//! each panel's in its own module (`rt_method`, `rt_get`, `rt_set`,
//! `rt_user`, `rt_after_set`), dispatched here by type.

use super::User;
use crate::objects::ops::Rect;
use crate::Value;

/// What runs once a handler has run, with its arguments as it left them
/// (OnRename's Cancel …).
pub type Then = Box<dyn FnOnce(&[Value])>;

/// What a runtime provides to the panels.
pub trait Runtime: Copy + 'static {
    /// A property as the program reads it.
    fn get(self, name: &str, prop: &str) -> Value;
    /// A property set as by the program (what follows a set runs: layout,
    /// the window, a model).
    fn set(self, name: &str, prop: &str, v: Value);
    /// A method called as by the program (`Designer.SetProp …`).
    fn call(self, name: &str, method: &str, args: &[Value]) -> Value;
    fn exists(self, name: &str) -> bool;
    /// Its type as the runtime keeps it (`RBUTTON`; "" for none).
    fn type_of(self, name: &str) -> String;
    /// Every component: (name, type), in creation order.
    fn components(self) -> Vec<(String, String)>;
    /// The components parented to `name`: (name, type), in creation order.
    fn children(self, name: &str) -> Vec<(String, String)>;
    /// The form `name` is on (itself for a form).
    fn form_of(self, name: &str) -> Option<String>;
    fn fire(self, name: &str, event: &str, args: &[Value]);
    /// Fires, then runs `then` with the arguments as the handler left them.
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Then);
    /// `name.SetFocus`.
    fn focus(self, name: &str);
    /// The panel's look changed: its window is drawn again.
    fn invalidate(self);
    /// Components' stacking changed (a palette shown over the others): the
    /// windows' trees are built again.
    fn restructure(self);
    /// Drops a list of `items` under `anchor` (logical, in form `form`'s
    /// client area) for panel `name`, as a combo box drops its list; a pick
    /// comes back as [`User::Picked`].
    fn drop_list(self, form: &str, name: &str, items: Vec<String>, anchor: Rect);
    /// A file's text, through the objects' file hooks (the disk on the
    /// desktop, the page's own files on the web).
    fn read_file(self, path: &str) -> Option<String> {
        crate::objects::read_file(path).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
    }
    /// Writes a file through the same hooks: whether it was written.
    fn write_file(self, path: &str, text: &str) -> bool {
        crate::objects::write_file(path, text.as_bytes()).is_ok()
    }
    /// Opens a web link (http, https, mailto) the user clicked in the
    /// system's browser (a new tab on the web); nothing by default.
    fn open_url(self, _url: &str) {}
}

/// The type `name` is, if it is a panel (`RTOOLBOX` …).
fn panel_type<R: Runtime>(rt: R, name: &str) -> Option<String> {
    let t = rt.type_of(name).to_ascii_uppercase();
    super::is_panel(&t).then_some(t)
}

/// A panel's method; `None` if `method` isn't one of its own (the
/// runtime's generic ones then: SetFocus, Repaint …).
pub fn rt_method<R: Runtime>(rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let method = method.to_ascii_lowercase();
    let out = match panel_type(rt, name)?.as_str() {
        "RPROPERTYINSPECTOR" => super::inspector::rt_method(rt, name, &method, args),
        "RTOOLBOX" => super::toolbox::rt_method(rt, name, &method, args),
        "RPROJECTTREE" => super::project_tree::rt_method(rt, name, &method, args),
        "ROUTPUTCONSOLE" => super::console::rt_method(rt, name, &method, args),
        "RTOOLBAR" => super::toolbar::rt_method(rt, name, &method, args),
        "RCOMMANDPALETTE" => super::palette::rt_method(rt, name, &method, args),
        "RMARKDOWNVIEW" => super::markdown::rt_method(rt, name, &method, args),
        _ => None,
    };
    if out.is_some() {
        rt.invalidate();
    }
    out
}

/// A panel's property as its model has it; `None`: the runtime's store
/// answers (Left, Visible …).
pub fn rt_get<R: Runtime>(rt: R, name: &str, prop: &str) -> Option<Value> {
    let prop = prop.to_ascii_lowercase();
    match panel_type(rt, name)?.as_str() {
        "RPROPERTYINSPECTOR" => super::inspector::rt_get(rt, name, &prop),
        "RTOOLBOX" => super::toolbox::rt_get(rt, name, &prop),
        "RPROJECTTREE" => super::project_tree::rt_get(rt, name, &prop),
        "ROUTPUTCONSOLE" => super::console::rt_get(rt, name, &prop),
        "RTOOLBAR" => super::toolbar::rt_get(rt, name, &prop),
        "RCOMMANDPALETTE" => super::palette::rt_get(rt, name, &prop),
        "RMARKDOWNVIEW" => super::markdown::rt_get(rt, name, &prop),
        _ => None,
    }
}

/// Sets a panel's property in its model: whether it was the model's (the
/// runtime's store keeps the rest, and keeps a copy of these too).
pub fn rt_set<R: Runtime>(rt: R, name: &str, prop: &str, v: &Value) -> bool {
    let prop = prop.to_ascii_lowercase();
    let Some(t) = panel_type(rt, name) else { return false };
    let done = match t.as_str() {
        "RPROPERTYINSPECTOR" => super::inspector::rt_set(rt, name, &prop, v),
        "RTOOLBOX" => super::toolbox::rt_set(rt, name, &prop, v),
        "RPROJECTTREE" => super::project_tree::rt_set(rt, name, &prop, v),
        "ROUTPUTCONSOLE" => super::console::rt_set(rt, name, &prop, v),
        "RTOOLBAR" => super::toolbar::rt_set(rt, name, &prop, v),
        "RCOMMANDPALETTE" => super::palette::rt_set(rt, name, &prop, v),
        "RMARKDOWNVIEW" => super::markdown::rt_set(rt, name, &prop, v),
        _ => false,
    };
    if done {
        rt.invalidate();
    }
    done
}

/// After any component's property was set (the program's or a user's):
/// panels that show it follow (an inspector whose target it is).
pub fn rt_after_set<R: Runtime>(rt: R, name: &str, prop: &str) {
    super::inspector::rt_after_set(rt, name, prop);
}

/// What the user did to panel `name` (the kernel's `Container::Panel`).
pub fn rt_user<R: Runtime>(rt: R, name: &str, action: User) {
    match action {
        User::Inspector(a) => super::inspector::rt_user(rt, name, a),
        User::Toolbox(a) => super::toolbox::rt_user(rt, name, a),
        User::ProjectTree(a) => super::project_tree::rt_user(rt, name, a),
        User::Console(a) => super::console::rt_user(rt, name, a),
        User::ToolBar(a) => super::toolbar::rt_user(rt, name, a),
        User::Palette(a) => super::palette::rt_user(rt, name, a),
        User::Markdown(a) => super::markdown::rt_user(rt, name, a),
        User::Picked(item) => match panel_type(rt, name).as_deref() {
            Some("RPROPERTYINSPECTOR") => super::inspector::rt_picked(rt, name, item),
            Some("RTOOLBAR") => super::toolbar::rt_picked(rt, name, item),
            _ => {}
        },
    }
    rt.invalidate();
}

/// A component argument: its name, or its Handle's component.
pub fn component_arg(v: &Value) -> String {
    match v {
        Value::Integer(h) => crate::handles::name_of(*h).unwrap_or_else(|| h.to_string()),
        Value::Double(h) => crate::handles::name_of(*h as i64).unwrap_or_else(|| h.to_string()),
        v => v.to_string_val(),
    }
}

/// A True / False argument or value as RapidQ's runtimes keep them (-1 /
/// 0, booleans, "True" / "False").
pub fn truth(v: &Value) -> bool {
    match v {
        Value::String(s) => {
            let s = s.trim();
            !(s.is_empty() || s == "0" || s.eq_ignore_ascii_case("false"))
        }
        v => v.to_bool(),
    }
}

/// RapidQ's True as the runtimes give it back.
pub fn basic_bool(b: bool) -> Value {
    Value::Integer(if b { -1 } else { 0 })
}
