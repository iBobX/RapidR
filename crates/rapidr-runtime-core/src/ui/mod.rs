//! The desktop UI facade: everything the runtime asks of the window system
//! goes through here, to the UI kernel and its winit or headless host
//! (`kernel.rs`, docs/desktop-host-plan.md). The names below are the ones
//! the runtime may use; the rest of `kernel` is the host's own.
//!
//! What doesn't depend on the host is `rapidr_ui_app`'s, shared with the
//! web runtime (docs/web-host-plan.md, Stage W2): the program it works
//! through is [`program::Rt`].

// (the test hooks' environment; the dialogs lane's Open / Save, colour and
// font dialogs' requests and answers: rapidr-ui-app's)
pub use rapidr_ui_app::{choose_dialogs, file_dialog, testhooks};

pub mod program;
pub mod kernel;
pub mod kernel_store;

use crate::value::Value;

pub use kernel::{
    // Something to repaint.
    redraw_widget, gui_redraw, canvas_redraw, picture_refresh, tab_control_changed, list_refresh, listview_refresh,
    grid_refresh, tree_refresh, header_refresh, dirtree_refresh, gui_set_caption, gui_apply_font, toggle_down_set,
    schedule_menu_sync, gui_timer_changed,
    // The component tree changed.
    attach_late, gui_set_visible, gui_set_parent, gui_widget_add_items, gui_widget_clear, stack_widgets,
    ensure_menu_widget,
    // Left / Top / Width / Height (and a form's client size).
    gui_apply_geometry,
    // Windows (`gui_set_window_state`: the WindowState lane's, Form.WindowState
    // set; it was `from`).
    gui_show, gui_show_visible, gui_hide, gui_close, gui_center, gui_move_form, gui_set_form_border, gui_apply_icon,
    gui_apply_icons, gui_set_window_state, gui_menu_popup,
    // Text between the store and the host's editors.
    text_push, gui_set_text, gui_set_input_value, text_pull, gui_get_text, gui_get_input_value,
    // Waits (DOEVENTS and the VM's pump: [`gui_doevents`], [`gui_pump_wait`]).
    // (`gui_choice`, the dialogs lane's: the box's icon, and whether it
    // beeps as Windows' MessageBox does — MESSAGEBOX; Delphi's MessageDlg
    // doesn't)
    gui_showmodal, gui_wait_key, gui_begin_app_wait, gui_take_wait_started, gui_set_cooperative_waits,
    run_gui_event_loop, gui_choice, gui_dialog_execute,
    // (the I/O lane's: a method waiting for work done in the background)
    gui_wait_task,
    // What only the host knows (`app_active`: the DirectX lane's).
    window_shown, form_window_exists, form_scale, menu_offset, is_modal, mouse_in_form, app_active,
    // Methods drawn by the host.
    canvas_method, image_method, tree_method,
    // The IDE's components: RDESIGNSURFACE's Show / Hide (its model, and
    // RCODEEDITOR's, are rapidr_value::objects' design and textedit).
    design_surface_method,
    // `$THEME name`, and a QTIMER the program made (generated programs call
    // both through the prelude).
    set_theme, gui_register_timer,
    // The screen (`Screen.Width` / `Height`, the work area less the task
    // bar / menu bar), the mouse on it, `Application.Minimize` (every
    // window), MSGBOX (the text and OK).
    screen_size, work_area, mouse, monitors, minimize, message_box,
    // The program ends (END, Application.Terminate): the windows' pending
    // commands run.
    before_exit,
};

/// `DOEVENTS`: pending events, timers and redraws get their turn, then the
/// handlers the host deferred (`object::rp_run_deferred`).
pub fn gui_doevents() {
    kernel::gui_doevents();
    crate::object::rp_run_deferred();
}

/// One step of the innermost wait (the VM's pump): `None` while it goes
/// on, `Some` when over; the handlers the host deferred run before it
/// returns.
pub fn gui_pump_wait() -> Option<Value> {
    let step = kernel::gui_pump_wait();
    crate::object::rp_run_deferred();
    step
}
