//! RapidR Studio's panels (docs/ide-plan.md I1, lane L-PANELS) drawn and
//! driven by the kernel — the same on the desktop and the web. Their models
//! are `rapidr_value::panels`; what the user does goes to the runtime as
//! `Container::Panel` ([`send`]), which the shared glue
//! (`rapidr_value::panels::runtime`) carries out with the program's events.
//!
//! [`common`] is their one look (every colour from the theme).

pub mod common;
pub mod console;
pub mod inspector;
pub mod palette;
pub mod project_tree;
pub mod toolbar;
pub mod toolbox;

use rapidr_value::panels::User;

use super::form::Container;
use super::list::InPlace;
use super::Cx;
use crate::input::KernelEvent;

/// What the user did to panel `cx`, for the runtime (after the pump).
pub fn send(cx: &mut Cx, action: User) {
    cx.events.push(KernelEvent::Container(Container::Panel { id: cx.id.to_string(), action }));
}

/// The focus left panel `id` (of `type_name`) while its in-place editor
/// (`list::begin_edit`: a search box, a value being typed) was open: what
/// the edit's end does — the events for the runtime.
pub fn focus_left(id: &str, type_name: &str, ed: InPlace) -> Vec<KernelEvent> {
    match type_name {
        "RPROPERTYINSPECTOR" => inspector::focus_left(id, ed),
        "RTOOLBOX" => toolbox::focus_left(id, ed),
        "RPROJECTTREE" => project_tree::focus_left(id, ed),
        "ROUTPUTCONSOLE" => console::focus_left(id, ed),
        "RCOMMANDPALETTE" => palette::focus_left(id, ed),
        _ => Vec::new(),
    }
}
