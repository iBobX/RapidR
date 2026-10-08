//! The program as `rapidr_ui_app` sees it on the desktop: [`Rt`], over the
//! component registry and its events (`object.rs`) and the layout
//! (`layout.rs`, `scroll.rs`, `mdi.rs`). The web runtime will have its own
//! over `object_web.rs` (docs/web-host-plan.md, "W2 results").

use std::time::Duration;

use rapidr_ui_app::program::{Program, Then};
use rapidr_ui_kernel::components::form::Container;
use rapidr_value::layout::Constraints;

use crate::object::{form_of, get_children_of, rp_comp_get, rp_comp_set, rp_comp_type, rp_fire_event, rp_fire_event_args, rp_fire_event_then, rp_has_handler, rp_host_callback_active, store_prop};
use crate::value::Value;

/// runtime-core's program (and, in `kernel.rs`, its windows).
#[derive(Clone, Copy)]
pub struct Rt;

impl Program for Rt {
    fn get(self, id: &str, prop: &str) -> Value {
        rp_comp_get(id, prop)
    }
    fn set(self, id: &str, prop: &str, value: Value) {
        rp_comp_set(id, prop, value)
    }
    fn store(self, id: &str, prop: &str, value: Value) {
        store_prop(id, prop, value)
    }
    fn type_of(self, id: &str) -> String {
        rp_comp_type(id)
    }
    fn children(self, id: &str) -> Vec<(String, String)> {
        get_children_of(id)
    }
    fn form_of(self, id: &str) -> Option<String> {
        form_of(id)
    }
    fn flag(self, id: &str, prop: &str, default: bool) -> bool {
        super::kernel_store::flag(id, prop, default)
    }

    fn fire(self, id: &str, event: &str) {
        rp_fire_event(id, event)
    }
    fn fire_args(self, id: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(id, event, args)
    }
    fn fire_then(self, id: &str, event: &str, args: &[Value], then: Then) {
        rp_fire_event_then(id, event, args, then)
    }
    fn has_handler(self, id: &str, event: &str) -> bool {
        rp_has_handler(id, event)
    }
    fn in_host_callback(self) -> bool {
        rp_host_callback_active()
    }

    fn quietly(self, f: &mut dyn FnMut()) {
        crate::layout::quietly(f)
    }
    fn constraints(self, form: &str) -> Constraints {
        crate::layout::constraints_of(form)
    }
    fn client_changed(self, form: &str) {
        crate::layout::client_changed(form);
        crate::scroll::update(form);
    }
    fn container(self, action: Container) {
        match action {
            Container::Scrolled { id, dx, dy } => crate::scroll::user_scrolled(&id, (dx, dy)),
            Container::SplitBegin(id) => {
                crate::layout::splitter_begin(&id);
            }
            Container::SplitMove(delta) => crate::layout::splitter_move(delta),
            Container::SplitEnd => crate::layout::splitter_end(),
            Container::Mdi { form, component, action } => crate::mdi::user(&form, &component, action),
            // (I1: RDOCKMANAGER — dock.rs)
            Container::Dock { id, action } => crate::dock::user(&id, action),
            // (I1 / L-PANELS — panels.rs)
            Container::Panel { id, action } => crate::panels::user(&id, action),
            // (the input lane's: the host's — `Desktop` makes it a window command)
            Container::Resize { .. } => {}
        }
    }

    fn timer_period(self, id: &str) -> Option<Duration> {
        crate::directx::timer_interval(id)
    }

    fn timer_firing(self, id: &str) -> bool {
        crate::directx::timer_fired(id)
    }

    fn form_built(self, id: &str) {
        crate::directx::form_built(id);
    }

    fn form_fullscreen(self, id: &str) -> bool {
        crate::directx::form_fullscreen(id)
    }
}
