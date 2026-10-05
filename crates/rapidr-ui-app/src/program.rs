//! The program as the glue sees it: its components (the property store),
//! its events, its layout and the clock — what [`Program`] asks of a
//! runtime. runtime-core implements it over its component registry
//! (`object.rs`) and its layout (`layout.rs`, `scroll.rs`, `mdi.rs`); the
//! web runtime will over `object_web.rs` and its `*_web.rs`
//! (docs/web-host-plan.md, "W2 results").
//!
//! As `rapidr_value::mdi::Runtime`, a runtime passes a unit struct by value:
//! what runs after a handler ([`Program::fire_then`]'s continuation) keeps
//! it.

use std::time::Duration;

use rapidr_ui_kernel::components::form::Container;
use rapidr_ui_kernel::tick::Instant;
use rapidr_value::layout::Constraints;
use rapidr_value::Value;

/// What runs once a handler has run, with the arguments as it left them
/// (RapidQ's event parameters are by reference: OnClose's Action …).
pub type Then = Box<dyn FnOnce(&[Value])>;

/// A runtime's program, for the glue.
pub trait Program: Copy + 'static {
    // ---- its components ----

    /// A property as the program reads it (`rp_comp_get`).
    fn get(self, id: &str, prop: &str) -> Value;
    /// A property set as by the program: what follows a set runs (layout,
    /// the window, a model, a store hook; `rp_comp_set`).
    fn set(self, id: &str, prop: &str, value: Value);
    /// A property stored without anything following (`store_prop`).
    fn store(self, id: &str, prop: &str, value: Value);
    /// Its type name as the registry keeps it (`RFORM` …; empty for no
    /// such component).
    fn type_of(self, id: &str) -> String;
    /// Its children (id, type name), in creation order.
    fn children(self, id: &str) -> Vec<(String, String)>;
    /// The form it is on (itself for a form).
    fn form_of(self, id: &str) -> Option<String>;
    /// A true / false property as the kernel reads it (its `Store`'s rules),
    /// `default` when it has none.
    fn flag(self, id: &str, prop: &str, default: bool) -> bool;

    // ---- its events ----

    /// Fires `event` without arguments (the handler gets the Sender); a
    /// click's also gives a button's ModalResult to its form
    /// (`rp_fire_event`).
    fn fire(self, id: &str, event: &str);
    /// Fires `event` with `args` (`rp_fire_event_args`).
    fn fire_args(self, id: &str, event: &str, args: &[Value]);
    /// Fires `event`, then runs `then` with the arguments as the handler
    /// left them, once it has run — at once in a native build, queued
    /// behind it in the interpreter (`rp_fire_event_then`).
    fn fire_then(self, id: &str, event: &str, args: &[Value], then: Then);
    /// Whether the program handles it (the work to fire it is done only
    /// then: OnDrawCell for every cell …).
    fn has_handler(self, id: &str, event: &str) -> bool;
    /// A host callback is on the stack: program code fired now would wait
    /// (the desktop's safety net, `rp_host_callback_active`).
    fn in_host_callback(self) -> bool;

    // ---- its layout ----

    /// Runs `f` without the geometry it stores laying anything out
    /// (`layout::quietly`).
    fn quietly(self, f: &mut dyn FnMut());
    /// Form `form`'s MinWidth … MaxHeight.
    fn constraints(self, form: &str) -> Constraints;
    /// Form `form`'s client area changed size: its aligned and anchored
    /// children laid out again, its scroll bars brought up to date
    /// (`layout::client_changed`, `scroll::update`).
    fn client_changed(self, form: &str);
    /// A container's action the user took, into the layout models (scrolled,
    /// a splitter dragged, an MDI child's frame used).
    fn container(self, action: Container);

    // ---- the clock ----

    /// Now, for the timers and the test script (the kernel's `Instant`: the
    /// OS's, `performance.now()` in a browser).
    fn now(self) -> Instant {
        Instant::now()
    }

    // ---- the DirectX objects (docs/directx-plan.md) ----

    /// Timer `id`'s own period, when its type has one (a QDXTIMER's
    /// Interval 0 is a screen refresh); `None`: its Interval.
    fn timer_period(self, _id: &str) -> Option<Duration> {
        None
    }

    /// Timer `id` is about to fire (a QDXTIMER counts its frames).
    fn timer_firing(self, _id: &str) {}

    /// Form `id`'s kernel side was just made, after its OnLoad (its
    /// QDXSCREENs set up: OnInitialize).
    fn form_built(self, _id: &str) {}
}
