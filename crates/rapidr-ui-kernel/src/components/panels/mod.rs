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
use super::Cx;
use crate::input::KernelEvent;

/// What the user did to panel `cx`, for the runtime (after the pump).
pub fn send(cx: &mut Cx, action: User) {
    cx.events.push(KernelEvent::Container(Container::Panel { id: cx.id.to_string(), action }));
}
