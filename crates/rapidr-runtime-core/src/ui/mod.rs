//! The desktop UI facade: everything the runtime asks of the window system
//! goes through here, to the host [`select`] chose — FLTK (`gui.rs`) or,
//! later, the UI kernel (docs/desktop-host-plan.md). The functions keep
//! `gui.rs`'s names and meaning.
//!
//! Until the kernel host lands, a build with only `kernel` answers as a
//! program without windows would (and says so once).

pub mod select;
pub mod testhooks;

#[cfg(feature = "gui")]
mod fltk_platform;
#[cfg(feature = "gui")]
use crate::gui;

use crate::value::Value;
use select::Backend;

/// The kernel's answer until it has one: `Default`, or what the function
/// gives after `=`.
#[cfg(feature = "kernel")]
macro_rules! or_default {
    () => {
        Default::default()
    };
    ($e:expr) => {
        $e
    };
}

/// One facade function per host function: `fn name(args) -> Ret = kernel
/// default;` forwards to `host::name`.
macro_rules! forward {
    ($host:ident: $( $(#[$m:meta])* fn $name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? $(= $def:expr)?; )*) => { $(
        $(#[$m])*
        pub fn $name($($arg: $ty),*) $(-> $ret)? {
            match select::backend() {
                #[cfg(feature = "gui")]
                Backend::Fltk => $host::$name($($arg),*),
                #[cfg(feature = "kernel")]
                Backend::Kernel => {
                    let _ = ($($arg,)*);
                    pending(stringify!($name));
                    or_default!($($def)?)
                }
            }
        }
    )* };
}

/// A facade function the kernel host doesn't serve yet (warned once).
#[cfg(feature = "kernel")]
fn pending(name: &str) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!("[rapidr] the UI kernel host isn't built yet ({name}): no windows");
    }
}

forward! { gui:
    // Something to repaint.
    fn redraw_widget(name: &str);
    fn gui_redraw(name: &str);
    fn canvas_redraw(name: &str);
    fn picture_refresh(name: &str);
    fn tab_control_changed(name: &str);
    fn list_refresh(name: &str);
    fn listview_refresh(name: &str);
    fn grid_refresh(name: &str);
    fn tree_refresh(name: &str);
    fn header_refresh(name: &str);
    fn dirtree_refresh(name: &str);
    fn gui_set_caption(name: &str, text: &str);
    fn gui_apply_font(name: &str);
    fn toggle_down_set(name: &str);
    fn schedule_menu_sync();
    fn gui_timer_changed(name: &str);

    // The component tree changed.
    fn attach_late(name: &str);
    fn gui_set_visible(name: &str, visible: bool);
    fn gui_set_parent(child_name: &str, parent_name: &str);
    fn gui_widget_add_items(name: &str, items_text: &str);
    fn gui_widget_clear(name: &str);
    fn stack_widgets(names: &[String]);
    fn ensure_menu_widget(name: &str);

    // Left / Top / Width / Height (and a form's client size).
    fn gui_apply_geometry(name: &str);

    // Windows.
    fn gui_show(name: &str);
    fn gui_show_visible(name: &str);
    fn gui_hide(name: &str);
    fn gui_close(name: &str);
    fn gui_center(name: &str);
    fn gui_move_form(name: &str);
    fn gui_set_form_border(name: &str);
    fn gui_apply_icon(name: &str);
    fn gui_apply_icons();
    fn gui_menu_popup(name: &str, x: i32, y: i32);

    // Text between the store and the host's editors.
    fn text_push(name: &str);
    fn gui_set_text(name: &str, text: &str);
    fn gui_set_input_value(name: &str, text: &str);
    fn text_pull(name: &str);
    fn gui_get_text(name: &str) -> String;
    fn gui_get_input_value(name: &str) -> Option<String>;

    // Waits (DOEVENTS and the VM's pump: [`gui_doevents`], [`gui_pump_wait`]).
    fn gui_showmodal(name: &str) -> i64;
    fn gui_wait_key() -> Option<bool>;
    fn gui_begin_app_wait();
    fn gui_take_wait_started() -> bool;
    fn gui_set_cooperative_waits(on: bool);
    fn run_gui_event_loop();
    fn gui_choice(title: &str, text: &str, labels: &[&str]) -> Option<usize>;
    fn gui_dialog_execute(name: &str, comp_type: &str) -> Value = crate::value::v_int(0);

    // What only the host knows.
    fn window_shown(name: &str) -> Option<bool>;
    fn form_window_exists(name: &str) -> bool;
    fn form_scale(name: &str) -> f64 = 1.0;
    fn menu_offset(form: &str) -> i32;
    fn is_modal(name: &str) -> bool;
    fn mouse_in_form() -> (i64, i64);

    // Methods drawn by the host.
    fn canvas_method(name: &str, method: &str, args: &[Value]) -> Value = crate::value::v_null();
    fn image_method(name: &str, method: &str, args: &[Value]) -> Value = crate::value::v_null();
    fn tree_method(name: &str, method: &str, args: &[Value]) -> Value = crate::value::v_null();

    // The IDE's components.
    fn design_surface_get(name: &str, prop: &str) -> Option<Value>;
    fn design_surface_set(name: &str, prop: &str, val: &Value) -> bool;
    fn design_surface_method(name: &str, method: &str, args: &[Value]) -> Value = crate::value::v_null();
    fn code_editor_method(name: &str, method: &str, args: &[Value]) -> Value = crate::value::v_null();

    /// `$THEME name` (generated programs call it through the prelude).
    fn set_theme(theme: &str);
    /// A QTIMER the program made (generated programs call it through the
    /// prelude).
    fn gui_register_timer(name: &str);
}

forward! { fltk_platform:
    /// `Screen.Width` / `Height`.
    fn screen_size() -> (i64, i64);
    /// The screen less the task bar / menu bar.
    fn work_area() -> (i64, i64);
    /// The mouse on the screen.
    fn mouse() -> (i64, i64);
    fn monitors() -> i64 = 1;
    /// `Application.Minimize`: every window.
    fn minimize();
    /// MSGBOX: the text and OK.
    fn message_box(text: &str);
}

/// `DOEVENTS`: pending events, timers and redraws get their turn, then the
/// handlers the host deferred (`object::rp_run_deferred`).
pub fn gui_doevents() {
    match select::backend() {
        #[cfg(feature = "gui")]
        Backend::Fltk => gui::gui_doevents(),
        #[cfg(feature = "kernel")]
        Backend::Kernel => pending("gui_doevents"),
    }
    crate::object::rp_run_deferred();
}

/// One step of the innermost wait (the VM's pump): `None` while it goes
/// on, `Some` when over; the handlers the host deferred run before it
/// returns.
pub fn gui_pump_wait() -> Option<Value> {
    let step = match select::backend() {
        #[cfg(feature = "gui")]
        Backend::Fltk => gui::gui_pump_wait(),
        #[cfg(feature = "kernel")]
        Backend::Kernel => {
            pending("gui_pump_wait");
            Some(crate::value::v_null())
        }
    };
    crate::object::rp_run_deferred();
    step
}
