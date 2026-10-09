//! RapidR Studio's panels on the desktop (RPROPERTYINSPECTOR, RTOOLBOX,
//! RPROJECTTREE, ROUTPUTCONSOLE, RTOOLBAR, RCOMMANDPALETTE, RMARKDOWNVIEW):
//! `rapidr_value::panels` does the work (the same as on the web); this is
//! the desktop runtime it works through.

use rapidr_value::objects::ops::Rect;
use rapidr_value::panels::runtime::{self, Runtime, Then};
use rapidr_value::panels::User;

use crate::object::{all_components, form_of, get_children_of, rp_comp_call, rp_comp_get, rp_comp_set, rp_comp_type, rp_fire_event_args, rp_fire_event_then};
use crate::value::Value;

#[derive(Clone, Copy)]
pub struct Desktop;

impl Runtime for Desktop {
    fn get(self, name: &str, prop: &str) -> Value {
        rp_comp_get(name, prop)
    }
    fn set(self, name: &str, prop: &str, v: Value) {
        rp_comp_set(name, prop, v)
    }
    fn call(self, name: &str, method: &str, args: &[Value]) -> Value {
        rp_comp_call(name, method, args)
    }
    fn exists(self, name: &str) -> bool {
        !rp_comp_type(name).is_empty()
    }
    fn type_of(self, name: &str) -> String {
        rp_comp_type(name)
    }
    fn components(self) -> Vec<(String, String)> {
        all_components()
    }
    fn children(self, name: &str) -> Vec<(String, String)> {
        get_children_of(name)
    }
    fn form_of(self, name: &str) -> Option<String> {
        form_of(name)
    }
    fn fire(self, name: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(name, event, args)
    }
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Then) {
        rp_fire_event_then(name, event, args, move |a: &[Value]| then(a))
    }
    fn focus(self, name: &str) {
        rp_comp_call(name, "setfocus", &[]);
    }
    fn invalidate(self) {
        #[cfg(feature = "gui")]
        rapidr_ui_app::windows::invalidate();
    }
    fn restructure(self) {
        #[cfg(feature = "gui")]
        crate::ui::stack_widgets(&[]);
    }
    fn drop_list(self, form: &str, name: &str, items: Vec<String>, anchor: Rect) {
        #[cfg(feature = "gui")]
        {
            rapidr_ui_kernel::components::combo::open_list(form, name, items, anchor);
            rapidr_ui_app::windows::invalidate();
        }
        #[cfg(not(feature = "gui"))]
        let _ = (form, name, items, anchor);
    }
    fn open_url(self, url: &str) {
        open_in_browser(url);
    }
}

/// A web link the user clicked (an RMARKDOWNVIEW's: http, https or mailto,
/// checked by the model) opened in the system's browser — never through a
/// shell. Under the test hooks it is only reported.
pub fn open_in_browser(url: &str) {
    if !rapidr_value::panels::markdown::is_web_link(url) {
        return;
    }
    if std::env::var_os("RAPIDR_TEST_DUMP").is_some() || std::env::var_os("RAPIDR_CAPTURE").is_some() || std::env::var_os("RAPIDR_TEST_EVENTS").is_some() {
        eprintln!("[rapidr] (test) would open {url}");
        return;
    }
    let mut cmd = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(windows) {
        let mut c = std::process::Command::new("rundll32.exe");
        c.arg("url.dll,FileProtocolHandler");
        c
    } else {
        std::process::Command::new("xdg-open")
    };
    if let Err(e) = cmd.arg(url).spawn() {
        eprintln!("[rapidr] can't open {url}: {e}");
    }
}

/// A panel's method (`None`: not one of its own).
pub fn method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    runtime::rt_method(Desktop, name, method, args)
}

/// A panel's property its model answers.
pub fn get(name: &str, prop: &str) -> Option<Value> {
    runtime::rt_get(Desktop, name, prop)
}

/// Sets a panel's property in its model: whether it was the model's.
pub fn set(name: &str, prop: &str, val: &Value) -> bool {
    // (an inspector given a designer: the designer's changes and selections
    // reach every inspector following it — rapidr_value::objects::design's
    // change hook, told as the surface's events go out)
    if prop == "designer" {
        rapidr_value::objects::design::set_change_hook(designer_changed);
    }
    runtime::rt_set(Desktop, name, prop, val)
}

/// After any component's property was set.
pub fn after_set(name: &str, prop: &str) {
    runtime::rt_after_set(Desktop, name, prop)
}

/// A designer's selection, or a selected component's property, changed
/// (its methods): the inspectors following it read it again.
pub fn designer_changed(name: &str) {
    rapidr_value::panels::inspector::designer_changed(&Desktop, name)
}

/// What the user did to a panel (the kernel's `Container::Panel`).
pub fn user(name: &str, action: User) {
    runtime::rt_user(Desktop, name, action)
}
