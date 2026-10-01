//! Component object system — global registry for RapidP components.
//!
//! Components (RForm, RButton, RSQLite, RSocket, etc.) are stored in a
//! global thread-local registry indexed by variable name. Generated code
//! uses `rp_create_component`, `rp_comp_set`, `rp_comp_get`, `rp_comp_method`,
//! and `rp_bind_event` to interact with components.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::value::{v_bool, v_dbl, v_int, v_null, v_str, Value};

// ---------------------------------------------------------------------------
// Component representation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RpComponent {
    pub type_name: String,
    pub properties: HashMap<String, Value>,
    pub creation_order: u32,
}

impl RpComponent {
    pub fn new(type_name: &str) -> Self {
        let mut props = HashMap::new();
        // Set default properties based on type
        let tn = type_name.to_uppercase();
        match tn.as_str() {
            "RFORM" => {
                props.insert("caption".into(), v_str(""));
                props.insert("width".into(), v_int(640));
                props.insert("height".into(), v_int(480));
                props.insert("left".into(), v_int(100));
                props.insert("top".into(), v_int(100));
                props.insert("visible".into(), v_bool(true));
                props.insert("color".into(), v_int(0xFFFFFF));
                props.insert("borderstyle".into(), v_int(2));
            }
            "RBUTTON" => {
                props.insert("caption".into(), v_str(""));
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(80));
                props.insert("height".into(), v_int(25));
                props.insert("enabled".into(), v_bool(true));
                props.insert("visible".into(), v_bool(true));
            }
            "RLABEL" => {
                props.insert("caption".into(), v_str(""));
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(100));
                props.insert("height".into(), v_int(20));
                props.insert("visible".into(), v_bool(true));
                props.insert("alignment".into(), v_int(0));
                props.insert("color".into(), v_int(0xFFFFFF));
                props.insert("fontcolor".into(), v_int(0));
                props.insert("fontsize".into(), v_int(12));
            }
            "REDIT" => {
                props.insert("text".into(), v_str(""));
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(120));
                props.insert("height".into(), v_int(25));
                props.insert("enabled".into(), v_bool(true));
                props.insert("visible".into(), v_bool(true));
                props.insert("readonly".into(), v_bool(false));
                props.insert("maxlength".into(), v_int(0));
            }
            "RPANEL" => {
                props.insert("caption".into(), v_str(""));
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(200));
                props.insert("height".into(), v_int(100));
                props.insert("visible".into(), v_bool(true));
                props.insert("color".into(), v_int(0xFFFFFF));
            }
            "RCHECKBOX" => {
                props.insert("caption".into(), v_str(""));
                props.insert("checked".into(), v_int(0));
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(100));
                props.insert("height".into(), v_int(25));
                props.insert("enabled".into(), v_bool(true));
                props.insert("visible".into(), v_bool(true));
            }
            "RRADIOBUTTON" => {
                props.insert("caption".into(), v_str(""));
                props.insert("checked".into(), v_int(0));
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(100));
                props.insert("height".into(), v_int(25));
            }
            "RCOMBOBOX" => {
                // Items, selection and Text: rapidr_value::objects::list.
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(120));
                props.insert("height".into(), v_int(25));
            }
            "RLISTBOX" | "RFILELISTBOX" | "RDIRTREE" => {
                // Items and selection: rapidr_value::objects::list.
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(120));
                props.insert("height".into(), v_int(100));
            }
            // QTIMER: Enabled is True by default (manual).
            "RTIMER" => {
                props.insert("enabled".into(), v_bool(true));
                props.insert("interval".into(), v_int(1000));
            }
            "RIMAGE" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(100));
                props.insert("height".into(), v_int(100));
                props.insert("stretch".into(), v_bool(false));
            }
            "RCANVAS" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(400));
                props.insert("height".into(), v_int(300));
                props.insert("color".into(), v_int(0xFFFFFF));
                props.insert("pencolor".into(), v_int(0));
                props.insert("penwidth".into(), v_int(1));
                props.insert("brushcolor".into(), v_int(0xFFFFFF));
                props.insert("fontcolor".into(), v_int(0));
                props.insert("fontsize".into(), v_int(12));
                props.insert("fontname".into(), v_str("Arial"));
            }
            "RHEADER" => {
                // Sections: rapidr_value::objects::header; a canvas to draw on.
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(200));
                props.insert("height".into(), v_int(20));
                props.insert("color".into(), v_int(0xF0F0F0));
            }
            "RSTRINGGRID" => {
                // Cells, sizes and selection: rapidr_value::objects::grid.
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(300));
                props.insert("height".into(), v_int(200));
            }
            "RTABCONTROL" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(300));
                props.insert("height".into(), v_int(200));
                props.insert("tabindex".into(), v_int(0));
            }
            "RDESIGNSURFACE" => {
                props.insert("width".into(), v_int(640));
                props.insert("height".into(), v_int(480));
                props.insert("formcaption".into(), v_str("Form1"));
                props.insert("compcount".into(), v_int(0));
                props.insert("visible".into(), v_bool(true));
            }
            "RCODEEDITOR" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(400));
                props.insert("height".into(), v_int(300));
                props.insert("text".into(), v_str(""));
                props.insert("visible".into(), v_bool(true));
            }
            "RGROUPBOX" => {
                props.insert("caption".into(), v_str(""));
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(200));
                props.insert("height".into(), v_int(100));
                props.insert("visible".into(), v_bool(true));
            }
            "RMAINMENU" | "RPOPUPMENU" => {
                // Menus
            }
            "RMENUITEM" => {
                props.insert("caption".into(), v_str(""));
                props.insert("enabled".into(), v_bool(true));
                props.insert("checked".into(), v_bool(false));
            }
            "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG" => {
                props.insert("filename".into(), v_str(""));
                props.insert("filetitle".into(), v_str(""));
                props.insert("filter".into(), v_str(""));
                props.insert("filterindex".into(), v_int(1));
                props.insert("initialdir".into(), v_str(""));
                props.insert("title".into(), v_str(""));
                props.insert("selcount".into(), v_int(0));
                if type_name == "RFILEDIALOG" {
                    props.insert("caption".into(), v_str("Open"));
                    props.insert("filter".into(), v_str("All Files|*.*"));
                    props.insert("mode".into(), v_int(0));
                    props.insert("multiselect".into(), v_bool(false));
                    props.insert("warnifoverwrite".into(), v_bool(true));
                }
            }
            "RCOLORDIALOG" | "RFONTDIALOG" => {
                props.insert("color".into(), v_int(0));
            }
            "RSTATUSBAR" => {
                // Docked at the bottom (Align = alBottom) once it has a parent.
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(200));
                props.insert("height".into(), v_int(24));
                props.insert("simpletext".into(), v_str(""));
                props.insert("simplepanel".into(), v_bool(false));
                props.insert("panelcount".into(), v_int(0));
            }
            "RPROGRESS" => {
                props.insert("min".into(), v_int(0));
                props.insert("max".into(), v_int(100));
                props.insert("position".into(), v_int(0));
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(200));
                props.insert("height".into(), v_int(25));
            }
            "RRICHEDIT" | "RMEMO" => {
                props.insert("text".into(), v_str(""));
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(200));
                props.insert("height".into(), v_int(100));
                props.insert("readonly".into(), v_bool(false));
            }
            "RFILESTREAM" => {
                props.insert("filename".into(), v_str(""));
                props.insert("position".into(), v_int(0));
                props.insert("size".into(), v_int(0));
            }
            "RJSON" => {
                props.insert("text".into(), v_str(""));
                props.insert("filename".into(), v_str(""));
                props.insert("count".into(), v_int(0));
            }
            "RSTRINGLIST" => {
                props.insert("count".into(), v_int(0));
                props.insert("text".into(), v_str(""));
            }
            "RTOOLBAR" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(0));
                props.insert("height".into(), v_int(32));
            }
            "RSCROLLBAR" => {
                props.insert("min".into(), v_int(0));
                props.insert("max".into(), v_int(100));
                props.insert("position".into(), v_int(0));
            }
            "RDATETIMEPICKER" => {
                props.insert("date".into(), v_str(""));
                props.insert("time".into(), v_str(""));
            }
            "RTREEVIEW" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(200));
                props.insert("height".into(), v_int(200));
            }
            "RTRACKBAR" => {
                props.insert("min".into(), v_int(0));
                props.insert("max".into(), v_int(100));
                props.insert("position".into(), v_int(0));
            }
            "RUPDOWN" => {
                props.insert("min".into(), v_int(0));
                props.insert("max".into(), v_int(100));
                props.insert("position".into(), v_int(0));
            }
            "RPRINTER" => {
                props.insert("title".into(), v_str(""));
            }
            // Database components — properties managed by database.rs
            "RSQLITE" => {
                props.insert("connected".into(), v_int(0));
                props.insert("db".into(), v_str(""));
                props.insert("rowcount".into(), v_int(0));
                props.insert("colcount".into(), v_int(0));
                props.insert("fieldcount".into(), v_int(0));
                props.insert("tablecount".into(), v_int(0));
            }
            "RMYSQL" => {
                props.insert("connected".into(), v_int(0));
                props.insert("host".into(), v_str("localhost"));
                props.insert("port".into(), v_int(3306));
                props.insert("user".into(), v_str(""));
                props.insert("password".into(), v_str(""));
                props.insert("db".into(), v_str(""));
                props.insert("rowcount".into(), v_int(0));
                props.insert("colcount".into(), v_int(0));
                props.insert("fieldcount".into(), v_int(0));
                props.insert("dbcount".into(), v_int(0));
            }
            // Network components — properties managed by network.rs
            "RSOCKET" => {
                props.insert("host".into(), v_str(""));
                props.insert("port".into(), v_int(0));
                props.insert("connected".into(), v_int(0));
                props.insert("timeout".into(), v_int(5000));
            }
            "RSERVERSOCKET" => {
                props.insert("host".into(), v_str("0.0.0.0"));
                props.insert("port".into(), v_int(0));
                props.insert("clientcount".into(), v_int(0));
            }
            "RHTTP" => {
                props.insert("host".into(), v_str(""));
                props.insert("port".into(), v_int(80));
                props.insert("url".into(), v_str(""));
                props.insert("statuscode".into(), v_int(0));
                props.insert("responsetext".into(), v_str(""));
                props.insert("responseheaders".into(), v_str(""));
                props.insert("timeout".into(), v_int(5000));
                props.insert("usessl".into(), v_int(0));
            }
            "RSPLITTER" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(5));
                props.insert("height".into(), v_int(200));
                props.insert("minsize".into(), v_int(30));
                props.insert("visible".into(), v_bool(true));
            }
            "RSCROLLBOX" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(200));
                props.insert("height".into(), v_int(200));
                props.insert("visible".into(), v_bool(true));
            }
            "RLISTVIEW" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(300));
                props.insert("height".into(), v_int(200));
                props.insert("itemindex".into(), v_int(-1));
                props.insert("items".into(), v_str(""));
                props.insert("count".into(), v_int(0));
                props.insert("visible".into(), v_bool(true));
            }
            "RPROGRESSBAR" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(200));
                props.insert("height".into(), v_int(25));
                props.insert("min".into(), v_int(0));
                props.insert("max".into(), v_int(100));
                props.insert("position".into(), v_int(0));
                props.insert("visible".into(), v_bool(true));
            }
            _ => {
                // Unknown component type — just empty properties
            }
        }
        // QSTATUSBAR docks at the bottom, QSPLITTER at the left (layout.rs).
        let align = rapidr_value::layout::default_align(&tn);
        if align != rapidr_value::layout::Align::None {
            props.insert("align".into(), v_int(align.value()));
        }
        Self {
            type_name: tn,
            properties: props,
            creation_order: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Global component registry
// ---------------------------------------------------------------------------

/// A handler bound to a component's event (shared with the web runtime).
pub use rapidr_value::events::Handler as EventHandler;

/// Type alias for the indirect event dispatcher used by the bytecode
/// interpreter's NativeHost. Receives `(handler_id, args)`.
pub type IndirectDispatcher = Box<dyn Fn(u32, &[Value])>;

thread_local! {
    static COMPONENTS: RefCell<HashMap<String, RpComponent>> = RefCell::new(HashMap::new());
    static EVENT_HANDLERS: RefCell<HashMap<(String, String), EventHandler>> = RefCell::new(HashMap::new());
    static CREATION_COUNTER: RefCell<u32> = RefCell::new(0);
    static INDIRECT_DISPATCHER: RefCell<Option<IndirectDispatcher>> = const { RefCell::new(None) };
}

/// Install a thread-local indirect event dispatcher. Used by the
/// bytecode VM's NativeHost so that FLTK callbacks can re-enter the VM
/// and invoke a bytecode function by index.
///
/// Returns the previously-installed dispatcher, if any.
pub fn rp_set_event_dispatcher(d: IndirectDispatcher) -> Option<IndirectDispatcher> {
    INDIRECT_DISPATCHER.with(|s| s.borrow_mut().replace(d))
}

/// Remove the indirect event dispatcher (returns it if present).
pub fn rp_clear_event_dispatcher() -> Option<IndirectDispatcher> {
    INDIRECT_DISPATCHER.with(|s| s.borrow_mut().take())
}

/// Bind an indirect event handler (carries an opaque id, e.g. a
/// bytecode function index). Dispatch goes through
/// [`rp_set_event_dispatcher`].
pub fn rp_bind_event_indirect(name: &str, event: &str, handler_id: u32) {
    EVENT_HANDLERS.with(|h| {
        h.borrow_mut().insert(
            (name.to_lowercase(), event.to_lowercase()),
            EventHandler::Indirect(handler_id),
        );
    });
}

fn dispatch_indirect(handler_id: u32, args: &[Value]) {
    INDIRECT_DISPATCHER.with(|slot| {
        // `try_borrow` rather than `borrow` so a re-entrant dispatch (e.g.
        // a handler that fires another event synchronously) doesn't panic
        // — it simply prints a soft warning instead.
        let borrow = match slot.try_borrow() {
            Ok(b) => b,
            Err(_) => {
                eprintln!("[rapidr] event handler #{handler_id} fired re-entrantly while dispatcher was borrowed; ignoring");
                return;
            }
        };
        if let Some(d) = borrow.as_ref() {
            d(handler_id, args);
        } else if !SHUTTING_DOWN.with(|s| s.get()) {
            // After `rp_run_app` returns and timers/widgets are torn
            // down FLTK may drain a few queued callbacks. Suppress the
            // noisy warning during shutdown — it is harmless.
            eprintln!(
                "[rapidr] event handler #{handler_id} fired but no indirect dispatcher is registered"
            );
        }
    });
}

thread_local! {
    static SHUTTING_DOWN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Mark the runtime as shutting down so that late-firing FLTK timer
/// callbacks (which can be queued after the dispatcher has been
/// uninstalled) don't print a noisy "no dispatcher" warning.
pub fn rp_mark_shutting_down() {
    SHUTTING_DOWN.with(|s| s.set(true));
}

/// Disable all RTimer components and clear their indirect handlers so
/// that no further FLTK timeout ticks attempt to dispatch into a torn
/// down VM. Called during ShowModal shutdown and from
/// `rp_set_event_dispatcher(None)` so the runtime stays well-behaved.
pub fn rp_stop_all_timers() {
    let timers: Vec<String> = COMPONENTS.with(|c| {
        c.borrow()
            .iter()
            .filter_map(|(n, comp)| {
                if comp.type_name.eq_ignore_ascii_case("RTIMER") {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect()
    });
    for name in timers {
        // Setting enabled=0 makes the existing FLTK timeout closure skip
        // both `rp_fire_event` and `repeat_timeout3` — the timer
        // self-cancels on its next scheduled tick.
        rp_comp_set(&name, "enabled", v_int(0));
    }
}

/// Create a new component and register it in the global registry.
pub fn rp_create_component(name: &str, type_name: &str) {
    // A QFORMMDI is a QFORM whose client area holds child windows (mdi.rs).
    if type_name.eq_ignore_ascii_case("RFORMMDI") {
        rapidr_value::mdi::register(name);
        return rp_create_component(name, "RFORM");
    }
    let name_lower = name.to_lowercase();
    // Idempotent: if component already exists with the same type, skip
    let already_exists = COMPONENTS.with(|c| {
        c.borrow().contains_key(&name_lower)
    });
    if already_exists {
        return;
    }
    let mut comp = RpComponent::new(type_name);
    let order = CREATION_COUNTER.with(|c| {
        let mut counter = c.borrow_mut();
        let val = *counter;
        *counter += 1;
        val
    });
    comp.creation_order = order;
    COMPONENTS.with(|c| {
        c.borrow_mut().insert(name_lower, comp);
    });
    rapidr_value::objects::create(name, type_name);
}

/// `DIM lbl(1 TO 3) AS QLABEL`: one component per element, ids `lbl(1)`,
/// `lbl(2)`, … (rapidr_value::objects::object_ids); returns the array of ids.
pub fn rp_component_array(kind: &str, name: &str, bounds: &[(i64, i64)]) -> Value {
    match rapidr_value::objects::object_ids(name, bounds) {
        Ok((array, ids)) => {
            for id in ids {
                rp_create_component(&id, kind);
            }
            array
        }
        Err(e) => {
            crate::value::runtime_error(&format!("DIM {name}: {e}"));
        }
    }
}

/// Set a property on a registered component.
pub fn rp_comp_set(name: &str, prop: &str, val: Value) {
    let prop_lower = prop.to_lowercase();
    // Screen, Application, Clipboard, Mouse (globals.rs).
    if crate::globals::set(name, &prop_lower, &val) {
        return;
    }
    // A QFORMMDI's ChildMax, ChildCaption, ChildState, … (mdi.rs).
    if rapidr_value::mdi::is_mdi(name) && crate::mdi::set(name, &prop_lower, &val) {
        return;
    }
    // A form's size before (it paints again only when it changes).
    let form_size_before = (matches!(prop_lower.as_str(), "width" | "height") && rp_comp_type(name) == "RFORM").then(|| rp_comp_get(name, &prop_lower).to_i64());
    // A canvas's size before (it paints again only when it changes: a
    // library setting its size in its own OnPaint mustn't loop).
    let canvas_size_before = (matches!(prop_lower.as_str(), "width" | "height") && rapidr_value::objects::is_canvas(name)).then(|| rp_comp_get(name, &prop_lower).to_i64());
    // RapidR's forms and containers have no frame inside their size: the
    // client area is the whole component, less a form's in-window menu.
    // ClientWidth / ClientHeight: a form's inside (rapidr_value::layout::
    // form_client_size); for other components, their whole size.
    if matches!(prop_lower.as_str(), "clientwidth" | "clientheight") {
        let (prop, v) = if rp_comp_type(name) == "RFORM" {
            let (cw, ch) = form_client(name);
            let (cw, ch) = if prop_lower == "clientwidth" { (val.to_i64(), ch) } else { (cw, val.to_i64()) };
            let (w, h) = rapidr_value::layout::form_outer_size(cw, ch, rp_comp_get(name, "borderstyle").to_i64(), menu_height(name));
            if prop_lower == "clientwidth" { ("width", w) } else { ("height", h) }
        } else {
            (if prop_lower == "clientwidth" { "width" } else { "height" }, val.to_i64())
        };
        return rp_comp_set(name, prop, v_int(v));
    }

    // QFONT, QMEMORYSTREAM, QBITMAP, QIMAGELIST, QLISTVIEW's data (shared
    // with the web runtime).
    let before_dir = if rapidr_value::objects::is_dirtree(name) { rp_comp_get(name, "directory").to_string_val() } else { String::new() };
    if let Some(result) = rapidr_value::objects::set(name, &prop_lower, &val) {
        let picture = rapidr_value::objects::is_picture(name);
        match result {
            // `Image.BMP = "photo.png"`: not a BMP; FLTK shows it.
            Err(_) if picture && prop_lower == "bmp" => {
                rp_comp_set(name, "__imagefile", val.clone());
                #[cfg(feature = "gui")]
                crate::gui::image_method(name, "loadfromfile", std::slice::from_ref(&val));
            }
            Err(e) => eprintln!("[rapidr] {name}.{prop}: {e}"),
            Ok(()) => {}
        }
        if picture {
            picture_changed(name);
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_canvas(name) {
            crate::gui::redraw_widget(name);
        }
        // A QFILELISTBOX's directory changed: OnChange.
        if prop_lower == "directory" && rapidr_value::objects::is_file_list(name) {
            rp_fire_event(name, "onchange");
        }
        // A QDIRTREE: shown again; its directory changed: OnChange.
        if rapidr_value::objects::is_dirtree(name) {
            #[cfg(feature = "gui")]
            crate::gui::dirtree_refresh(name);
            if matches!(prop_lower.as_str(), "directory" | "initialdir") && before_dir != rp_comp_get(name, "directory").to_string_val() {
                rp_fire_event(name, "onchange");
            }
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_tree(name) {
            crate::gui::tree_refresh(name);
        } else if rapidr_value::objects::is_listview(name) {
            crate::gui::listview_refresh(name);
        } else if rapidr_value::objects::is_grid(name) {
            crate::gui::grid_refresh(name);
        } else if rapidr_value::objects::is_list(name) {
            crate::gui::list_refresh(name);
        }
        return;
    }
    // `Label.Font = Font` (a QFONT): copy the font's settings.
    if prop_lower == "font" {
        if let Some(props) = rapidr_value::objects::font_properties(&val.to_string_val()) {
            for (flat, v) in props {
                rp_comp_set(name, flat, v);
            }
            return;
        }
    }

    // Normalize dot-notation font properties → flat names for compatibility
    let aliases: &[(&str, &str)] = &[
        ("font.name", "fontname"),
        ("font.size", "fontsize"),
        ("font.bold", "fontbold"),
        ("font.italic", "fontitalic"),
        ("font.color", "fontcolor"),
    ];
    for &(dotted, flat) in aliases {
        if prop_lower == dotted {
            rp_comp_set(name, flat, val.clone());
        } else if prop_lower == flat {
            // Also store the dotted version
            COMPONENTS.with(|c| {
                if let Some(comp) = c.borrow_mut().get_mut(&name.to_lowercase()) {
                    comp.properties.insert(dotted.to_string(), val.clone());
                }
            });
        }
    }

    // Handle runtime GUI property updates
    #[cfg(feature = "gui")]
    {
        let comp_type = rp_comp_type(name);
        match comp_type.as_str() {
            "RDESIGNSURFACE" => {
                if crate::gui::design_surface_set(name, &prop_lower, &val) {
                    // Also store in registry
                }
            }
            _ => {}
        }
        // Update visible widgets when "visible" changes
        if prop_lower == "visible" {
            let v = match &val {
                Value::Boolean(b) => *b,
                Value::Integer(i) => *i != 0,
                _ => true,
            };
            crate::gui::gui_set_visible(name, v);
        }
        // A status bar redraws its panels / simple text.
        if comp_type == "RSTATUSBAR" && (prop_lower.starts_with("panel") || prop_lower.starts_with("simple")) {
            crate::gui::gui_redraw(name);
        }
        // Update caption/simpletext on the widget
        else if prop_lower == "caption" || prop_lower == "simpletext" {
            crate::gui::gui_set_caption(name, &val.to_string_val());
        }
        // Update text on TextEditor/CodeEditor
        if prop_lower == "text" && (comp_type == "RCODEEDITOR" || comp_type == "RRICHEDIT" || comp_type == "RMEMO") {
            crate::gui::gui_set_text(name, &val.to_string_val());
        }
        // Update text on Input (REDIT)
        if prop_lower == "text" && comp_type == "REDIT" {
            crate::gui::gui_set_input_value(name, &val.to_string_val());
        }
    }

    // Data science property updates
    #[cfg(feature = "datascience")]
    {
        let comp_type = rp_comp_type(name);
        match comp_type.as_str() {
            "RNUM" => crate::datascience::num_set_prop(name, &prop_lower, &val),
            "RPLOT" => crate::datascience::plot_set_prop(name, &prop_lower, &val),
            _ => {}
        }
    }

    COMPONENTS.with(|c| {
        let mut comps = c.borrow_mut();
        let key = name.to_lowercase();
        // Auto-vivify a "bag" component if the name is unknown — this
        // gives DIM'd UDT variables (e.g. `r.left = 10` on
        // `DIM r AS Rect`) a place to store and retrieve fields when no
        // real component exists. The TYPE statement is currently lowered
        // to a no-op by bcgen, so without this fallback `r.left = 10`
        // would silently drop the value.
        let comp = comps.entry(key).or_insert_with(|| RpComponent::new("RUDT"));
        // The program chose a font: the GUI applies it (defaults it doesn't).
        let font_prop = prop_lower.starts_with("font") && prop_lower != "font";
        if font_prop {
            comp.properties.insert("__fontset".into(), v_bool(true));
        }
        comp.properties.insert(prop_lower.clone(), val);
        #[cfg(feature = "gui")]
        if font_prop {
            drop(comps);
            crate::gui::gui_apply_font(name);
        }
    });
    // A panel's bevels drawn again.
    #[cfg(feature = "gui")]
    if rapidr_value::objects::bevel::default(&prop_lower).is_some() {
        crate::gui::redraw_widget(name);
    }
    // Align and geometry: lay out, move the widget (layout.rs).
    crate::layout::after_set(name, &prop_lower);
    // A parent whose widget exists already: the widget is made now.
    #[cfg(feature = "gui")]
    if prop_lower == "parent" && !crate::layout::is_quiet() {
        crate::gui::attach_late(name);
    }
    // `CoolBtn.Down = True`: the others of its group come up.
    #[cfg(feature = "gui")]
    if prop_lower == "down" {
        crate::gui::toggle_down_set(name);
    }
    // A QCANVAS's new size shows more or less of its surface.
    #[cfg(feature = "gui")]
    if matches!(prop_lower.as_str(), "width" | "height") && rapidr_value::objects::is_header(name) {
        crate::gui::redraw_widget(name);
    } else if matches!(prop_lower.as_str(), "width" | "height") && rapidr_value::objects::is_canvas(name) {
        crate::gui::canvas_redraw(name);
        // (a form's is fired below, once its size really changed)
        if !rapidr_value::objects::is_form_surface(name) && canvas_size_before != Some(rp_comp_get(name, &prop_lower).to_i64()) {
            rp_fire_event(name, "onpaint");
        }
    }
    // A form's new size: it paints again (drawn on its surface, or its
    // canvases' handlers).
    if form_size_before.is_some_and(|before| before != rp_comp_get(name, &prop_lower).to_i64()) && !crate::layout::is_quiet() && form_is_built(name) {
        // (an MDI form's maximized children follow)
        if rapidr_value::mdi::is_mdi(name) {
            crate::mdi::resized(name);
        }
        rp_fire_event(name, "onpaint");
    }
    // A QIMAGE's AutoSize / Stretch / Center, or its size with Stretch.
    if matches!(prop_lower.as_str(), "autosize" | "stretch" | "center" | "width" | "height") && rapidr_value::objects::is_picture(name) {
        if matches!(prop_lower.as_str(), "width" | "height") {
            store_prop(name, "__sized", v_bool(true));
        }
        if prop_lower == "autosize" {
            picture_changed(name);
        } else {
            #[cfg(feature = "gui")]
            crate::gui::picture_refresh(name);
        }
    }
    #[cfg(feature = "gui")]
    if matches!(prop_lower.as_str(), "left" | "top") && rp_comp_type(name) == "RFORM" {
        crate::gui::gui_move_form(name);
    }
    // A list view drawn in its color and font again.
    #[cfg(feature = "gui")]
    if (prop_lower == "color" || prop_lower.starts_with("font")) && rapidr_value::objects::is_listview(name) {
        crate::gui::listview_refresh(name);
    }
    // A timer enabled (again) or given another interval: it ticks.
    #[cfg(feature = "gui")]
    if matches!(prop_lower.as_str(), "enabled" | "interval") && rp_comp_type(name) == "RTIMER" {
        crate::gui::gui_timer_changed(name);
    }
    // A tree's image lists: its icons shown again.
    #[cfg(feature = "gui")]
    if matches!(prop_lower.as_str(), "images" | "stateimages") && rapidr_value::objects::is_tree(name) {
        crate::gui::tree_refresh(name);
    }
    #[cfg(feature = "gui")]
    if matches!(prop_lower.as_str(), "icon" | "icohandle") && matches!(rp_comp_type(name).as_str(), "RFORM" | "RFORMMDI") {
        crate::gui::gui_apply_icon(name);
    }
    // A form with / without its frame (bsNone): the window and its inside.
    if prop_lower == "borderstyle" && rp_comp_type(name) == "RFORM" {
        #[cfg(feature = "gui")]
        crate::gui::gui_set_form_border(name);
        crate::layout::realign(name, None);
    }
}

/// A form's ClientWidth / ClientHeight (rapidr_value::layout).
pub fn form_client(name: &str) -> (i64, i64) {
    rapidr_value::layout::form_client_size(
        rp_comp_get(name, "width").to_i64(),
        rp_comp_get(name, "height").to_i64(),
        rp_comp_get(name, "borderstyle").to_i64(),
        menu_height(name),
    )
}

/// The height of a form's in-window main menu (0 on macOS or without one).
fn menu_height(name: &str) -> i64 {
    #[cfg(feature = "gui")]
    return crate::gui::menu_offset(name) as i64;
    #[cfg(not(feature = "gui"))]
    {
        let _ = name;
        0
    }
}

/// Get a property from a registered component.
pub fn rp_comp_get(name: &str, prop: &str) -> Value {
    let prop_lower = prop.to_lowercase();
    // Screen, Application, Clipboard, Mouse (globals.rs).
    if let Some(v) = crate::globals::get(name, &prop_lower) {
        return v;
    }
    // A form's inside (its frame and main menu excluded); other
    // components have no frame inside their size.
    if matches!(prop_lower.as_str(), "clientwidth" | "clientheight") {
        let (w, h) = if rp_comp_type(name) == "RFORM" {
            form_client(name)
        } else {
            (rp_comp_get(name, "width").to_i64(), rp_comp_get(name, "height").to_i64())
        };
        return v_int(if prop_lower == "clientwidth" { w } else { h });
    }
    // A component's Handle (rapidr_value::handles).
    if prop_lower == "handle" && COMPONENTS.with(|c| c.borrow().contains_key(&name.to_lowercase())) {
        return v_int(rapidr_value::handles::handle_of(name));
    }
    // A QFORMMDI's ChildCount, ChildCaption, … (mdi.rs).
    if let Some(v) = rapidr_value::mdi::get(name, &prop_lower) {
        return v;
    }
    if let Some(v) = rapidr_value::objects::get(name, &prop_lower) {
        return v;
    }

    // Check GUI state overrides first
    #[cfg(feature = "gui")]
    {
        let comp_type = rp_comp_type(name);
        match comp_type.as_str() {
            "RDESIGNSURFACE" => {
                if let Some(v) = crate::gui::design_surface_get(name, &prop_lower) {
                    return v;
                }
            }
            "RCODEEDITOR" | "RRICHEDIT" | "RMEMO" => {
                if prop_lower == "text" {
                    return v_str(&crate::gui::gui_get_text(name));
                }
            }
            "REDIT" => {
                if prop_lower == "text" {
                    if let Some(val) = crate::gui::gui_get_input_value(name) {
                        return v_str(&val);
                    }
                }
            }
            _ => {}
        }
    }

    // Data science property overrides
    #[cfg(feature = "datascience")]
    {
        let comp_type = rp_comp_type(name);
        match comp_type.as_str() {
            "RNUM" => {
                let v = crate::datascience::num_get_prop(name, &prop_lower);
                if !matches!(v, Value::Null) { return v; }
            }
            "RDATAFRAME" => {
                let v = crate::datascience::dataframe_get_prop(name, &prop_lower);
                if !matches!(v, Value::Null) { return v; }
            }
            "RPLOT" => {
                let v = crate::datascience::plot_get_prop(name, &prop_lower);
                if !matches!(v, Value::Null) { return v; }
            }
            _ => {}
        }
    }

    COMPONENTS.with(|c| {
        c.borrow()
            .get(&name.to_lowercase())
            .and_then(|comp| comp.properties.get(&prop_lower))
            .cloned()
            // (a QRECT's fields, and positions, are 0 until set; a panel's
            // bevels RapidQ's defaults)
            .or_else(|| (comp_is_panel(name)).then(|| rapidr_value::objects::bevel::default(&prop_lower).map(v_int)).flatten())
            .unwrap_or_else(|| if matches!(prop_lower.as_str(), "left" | "top" | "right" | "bottom") { v_int(0) } else { v_null() })
    })
}

fn comp_is_panel(name: &str) -> bool {
    COMPONENTS.with(|c| c.try_borrow().ok().and_then(|c| c.get(&name.to_lowercase()).map(|x| x.type_name == "RPANEL")).unwrap_or(false))
}

/// Get the type name of a registered component.
pub fn rp_comp_type(name: &str) -> String {
    COMPONENTS.with(|c| {
        c.borrow()
            .get(&name.to_lowercase())
            .map(|comp| comp.type_name.clone())
            .unwrap_or_default()
    })
}

/// A QIMAGE's picture changed: with AutoSize the control takes the
/// picture's size; the widget shows it again.
fn picture_changed(name: &str) {
    // (AutoSize; or a picture loaded into a new QIMAGE, whose size the
    // program hasn't set: the picture's, as in RapidQ)
    if rp_comp_get(name, "autosize").to_bool() || !rp_comp_get(name, "__sized").to_bool() {
        if let Some(Some((w, h))) = rapidr_value::objects::with_picture(name, |b| {
            (!b.img.pixels.is_empty()).then_some((b.img.width as i64, b.img.height as i64))
        }) {
            if rp_comp_get(name, "width").to_i64() != w {
                rp_comp_set(name, "width", v_int(w));
            }
            if rp_comp_get(name, "height").to_i64() != h {
                rp_comp_set(name, "height", v_int(h));
            }
        }
    }
    #[cfg(feature = "gui")]
    crate::gui::picture_refresh(name);
}

/// Whether form `name`'s window exists (a size set while the form is being
/// declared paints nothing).
fn form_is_built(name: &str) -> bool {
    #[cfg(feature = "gui")]
    return crate::gui::form_window_exists(name);
    #[cfg(not(feature = "gui"))]
    {
        let _ = name;
        false
    }
}

/// Call a method on a registered component.
/// Dispatches to the appropriate backend based on component type.
pub fn rp_comp_method(name: &str, method: &str, args: &[Value]) -> Value {
    let comp_type = rp_comp_type(name);
    let method_lower = method.to_lowercase();
    // Screen, Application, Clipboard, Mouse (globals.rs).
    if let Some(v) = crate::globals::call(name, &method_lower, args) {
        return v;
    }
    // A file dialog's Files(i): the folder (0), then the picked names.
    if method_lower == "files" && matches!(comp_type.as_str(), "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG") {
        let i = args.first().map_or(0, Value::to_i64);
        let v = rp_comp_get(name, &format!("files({i})"));
        return if matches!(v, Value::Null) { v_str("") } else { v };
    }
    // A QFORMMDI's AddChild, CascadeChild, … (mdi.rs).
    if rapidr_value::mdi::is_mdi(name) {
        if let Some(v) = crate::mdi::method(name, &method_lower, args) {
            return v;
        }
    }

    // A QFORM gets its own drawing surface the first time it's drawn on.
    if comp_type == "RFORM"
        && rapidr_value::objects::is_drawing_method(&method_lower)
        && !(method_lower == "paint" && args.len() < 3)
        && !rapidr_value::objects::is_form_surface(name)
    {
        rapidr_value::objects::create_form_surface(name, rapidr_value::objects::form_color(&rp_comp_get(name, "color")));
    }

    if let Some(result) = rapidr_value::objects::call(name, &method_lower, args, &|id, p| rp_comp_get(id, p)) {
        if rapidr_value::objects::is_picture(name) {
            // `Image.LoadFromFile "photo.png"`: not a BMP; FLTK shows it.
            if result.is_err() && matches!(method_lower.as_str(), "loadfromfile" | "load") {
                if let Some(file) = args.first() {
                    rp_comp_set(name, "__imagefile", file.clone());
                }
                #[cfg(feature = "gui")]
                return crate::gui::image_method(name, &method_lower, args);
            }
            picture_changed(name);
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_canvas(name) {
            crate::gui::canvas_redraw(name);
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_dirtree(name) {
            crate::gui::dirtree_refresh(name);
        }
        // A QHEADER's sections changed (not a drawing on it): painted again.
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_header(name) && rapidr_value::objects::header::changes_sections(&method_lower) {
            crate::gui::header_refresh(name);
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_tree(name) {
            crate::gui::tree_refresh(name);
        } else if rapidr_value::objects::is_listview(name) {
            crate::gui::listview_refresh(name);
        } else if rapidr_value::objects::is_grid(name) {
            crate::gui::grid_refresh(name);
        } else if rapidr_value::objects::is_list(name) {
            crate::gui::list_refresh(name);
        }
        return result.unwrap_or_else(|e| {
            eprintln!("[rapidr] {name}.{method}: {e}");
            v_null()
        });
    }
    // Indexed sub-objects (`SB.Panel(0).Width = 100` → method
    // `panel.width=` with (0, 100); reading → `panel.width` with (0)): kept
    // as the component's properties `panel(0).width` unless the component
    // implements them (above: a QLISTVIEW's `Item(i)` / `Column(i)`).
    if let Some(v) = indexed_sub_object(name, &method_lower, args) {
        return v;
    }
    if comp_type == "RSTATUSBAR" {
        if let Some(v) = statusbar_method(name, &method_lower, args) {
            return v;
        }
    }

    match comp_type.as_str() {
        "RSQLITE" => {
            #[cfg(feature = "database")]
            {
                crate::database::sqlite_method(name, &method_lower, args)
            }
            #[cfg(not(feature = "database"))]
            {
                eprintln!("[WARN] Database support not compiled. Method {}.{}() ignored.", name, method);
                v_null()
            }
        }
        "RMYSQL" => {
            #[cfg(feature = "database")]
            {
                crate::database::mysql_method(name, &method_lower, args)
            }
            #[cfg(not(feature = "database"))]
            {
                eprintln!("[WARN] Database support not compiled. Method {}.{}() ignored.", name, method);
                v_null()
            }
        }
        "RSOCKET" => {
            #[cfg(feature = "network")]
            {
                crate::network::socket_method(name, &method_lower, args)
            }
            #[cfg(not(feature = "network"))]
            {
                eprintln!("[WARN] Network support not compiled. Method {}.{}() ignored.", name, method);
                v_null()
            }
        }
        "RSERVERSOCKET" => {
            #[cfg(feature = "network")]
            {
                crate::network::server_socket_method(name, &method_lower, args)
            }
            #[cfg(not(feature = "network"))]
            {
                eprintln!("[WARN] Network support not compiled. Method {}.{}() ignored.", name, method);
                v_null()
            }
        }
        "RHTTP" => {
            #[cfg(feature = "network")]
            {
                crate::network::http_method(name, &method_lower, args)
            }
            #[cfg(not(feature = "network"))]
            {
                eprintln!("[WARN] Network support not compiled. Method {}.{}() ignored.", name, method);
                v_null()
            }
        }
        "RJSON" => json_method(name, &method_lower, args),
        // Specialized GUI component method dispatch
        #[cfg(feature = "gui")]
        "RDESIGNSURFACE" => crate::gui::design_surface_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        #[cfg(feature = "gui")]
        "RCODEEDITOR" => crate::gui::code_editor_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RTABCONTROL" => crate::gui::tab_control_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RTREEVIEW" => crate::gui::tree_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RCANVAS" => crate::gui::canvas_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RHEADER" if matches!(method_lower.as_str(), "repaint" | "refresh" | "update" | "paint") => {
            crate::gui::header_refresh(name);
            v_null()
        }
        #[cfg(feature = "gui")]
        "RHEADER" => crate::gui::canvas_method(name, &method_lower, args),
        // Data science component methods
        #[cfg(feature = "datascience")]
        "RNUM" => crate::datascience::num_method(name, &method_lower, args),
        #[cfg(feature = "datascience")]
        "RDATAFRAME" => crate::datascience::dataframe_method(name, &method_lower, args),
        #[cfg(feature = "datascience")]
        "RPLOT" => crate::datascience::plot_method(name, &method_lower, args),
        // RImage methods
        #[cfg(feature = "gui")]
        "RIMAGE" => crate::gui::image_method(name, &method_lower, args),
        // GUI component methods — generic dispatch
        _ => gui_generic_method(name, &comp_type, &method_lower, args),
    }
}

/// Bind a 0-argument event handler to a component.
pub fn rp_bind_event(name: &str, event: &str, handler: fn()) {
    EVENT_HANDLERS.with(|h| {
        h.borrow_mut()
            .insert((name.to_lowercase(), event.to_lowercase()), EventHandler::Arity0(handler));
    });
}

/// Bind a 1-argument event handler to a component.
pub fn rp_bind_event_1(name: &str, event: &str, handler: fn(Value)) {
    EVENT_HANDLERS.with(|h| {
        h.borrow_mut()
            .insert((name.to_lowercase(), event.to_lowercase()), EventHandler::Arity1(handler));
    });
}

/// Bind a 2-argument event handler to a component.
pub fn rp_bind_event_2(name: &str, event: &str, handler: fn(Value, Value)) {
    EVENT_HANDLERS.with(|h| {
        h.borrow_mut()
            .insert((name.to_lowercase(), event.to_lowercase()), EventHandler::Arity2(handler));
    });
}

/// Bind a 3-argument event handler to a component.
pub fn rp_bind_event_3(name: &str, event: &str, handler: fn(Value, Value, Value)) {
    EVENT_HANDLERS.with(|h| {
        h.borrow_mut()
            .insert((name.to_lowercase(), event.to_lowercase()), EventHandler::Arity3(handler));
    });
}

/// Bind a 4-argument event handler to a component.
pub fn rp_bind_event_4(name: &str, event: &str, handler: fn(Value, Value, Value, Value)) {
    EVENT_HANDLERS.with(|h| {
        h.borrow_mut()
            .insert((name.to_lowercase(), event.to_lowercase()), EventHandler::Arity4(handler));
    });
}

/// Bind a 5-argument event handler to a component.
pub fn rp_bind_event_5(name: &str, event: &str, handler: fn(Value, Value, Value, Value, Value)) {
    EVENT_HANDLERS.with(|h| {
        h.borrow_mut()
            .insert((name.to_lowercase(), event.to_lowercase()), EventHandler::Arity5(handler));
    });
}

/// Bind a compiled handler of `n` parameters that writes them back (RapidQ's
/// event parameters are by reference: OnClose's `Action`, …).
pub fn rp_bind_event_out(name: &str, event: &str, n: usize, handler: fn(&mut [Value])) {
    bind_handler(name, event, EventHandler::Out(n, handler));
}

/// Runs the handler bound to `name`'s `event` with the event's arguments
/// (the firing component is passed last, as `Sender`); returns them as a
/// compiled handler left them. The handler is copied out first, so it may
/// bind or fire other events.
fn fire(name: &str, event: &str, args: &[Value]) -> Vec<Value> {
    let Some(handler) = lookup_handler(name, event) else { return args.to_vec() };
    rapidr_value::events::call(handler, name, args, dispatch_indirect)
}

/// Fire an event with no arguments (handlers get the Sender).
pub fn rp_fire_event(name: &str, event: &str) {
    let _ = fire(name, event, &[]);
}

/// Fire an event with 1 argument.
pub fn rp_fire_event_1(name: &str, event: &str, arg: Value) {
    let _ = fire(name, event, &[arg]);
}

/// Fire an event with 2 arguments.
pub fn rp_fire_event_2(name: &str, event: &str, arg1: Value, arg2: Value) {
    let _ = fire(name, event, &[arg1, arg2]);
}

/// Fire an event with any number of arguments.
pub fn rp_fire_event_args(name: &str, event: &str, args: &[Value]) {
    let _ = fire(name, event, args);
}

/// Fire an event with 5 arguments.
pub fn rp_fire_event_5(name: &str, event: &str, a1: Value, a2: Value, a3: Value, a4: Value, a5: Value) {
    let _ = fire(name, event, &[a1, a2, a3, a4, a5]);
}

/// Fires `event`, then runs `then` once its handler has run — at once in a
/// native build; in the interpreter, queued behind the handler
/// (`rapidr_value::events`). For events whose handler sets something the
/// runtime reads afterwards (a QFORMMDI's `ChildResult`).
///
/// `then` gets the arguments as the handler left them (RapidQ's event
/// parameters are by reference: OnClose's `Action`, OnSelectCell's
/// `CanSelect`, …).
pub fn rp_fire_event_then(name: &str, event: &str, args: &[Value], then: impl FnOnce(&[Value]) + 'static) {
    rapidr_value::events::fire_then(args, || fire(name, event, args), then);
}

/// Bind a bytecode EVENT handler to one instance (see [`EventHandler::IndirectThis`]).
pub fn rp_bind_event_indirect_this(name: &str, event: &str, handler_id: u32, this: Value) {
    bind_handler(name, event, EventHandler::IndirectThis(handler_id, this));
}

/// Bind a compiled closure (see [`EventHandler::Closure`]).
pub fn rp_bind_event_closure(name: &str, event: &str, f: std::rc::Rc<dyn Fn(&mut Vec<Value>)>) {
    bind_handler(name, event, EventHandler::Closure(f));
}

/// Whether the program handles `name`'s `event` (the runtime then does the
/// work to fire it, e.g. OnDrawCell for every cell).
pub fn rp_has_handler(name: &str, event: &str) -> bool {
    lookup_handler(name, event).is_some()
}

fn lookup_handler(name: &str, event: &str) -> Option<EventHandler> {
    EVENT_HANDLERS.with(|h| h.borrow().get(&(name.to_lowercase(), event.to_lowercase())).cloned())
}

fn bind_handler(name: &str, event: &str, handler: EventHandler) {
    EVENT_HANDLERS.with(|h| {
        h.borrow_mut().insert((name.to_lowercase(), event.to_lowercase()), handler);
    });
}

/// For the bytecode VM: `ShowModal` leaves its wait to the VM (see
/// `gui::gui_set_cooperative_waits`), which serves it with [`rp_pump_wait`].
pub fn rp_set_cooperative_waits(on: bool) {
    #[cfg(feature = "gui")]
    crate::gui::gui_set_cooperative_waits(on);
    #[cfg(not(feature = "gui"))]
    let _ = on;
}

/// Whether the last operation started a wait (asked once per operation).
pub fn rp_take_wait_started() -> bool {
    #[cfg(feature = "gui")]
    return crate::gui::gui_take_wait_started();
    #[cfg(not(feature = "gui"))]
    false
}

/// Starts waiting for the program's windows (the main event loop).
pub fn rp_begin_app_wait() {
    #[cfg(feature = "gui")]
    crate::gui::gui_begin_app_wait();
}

/// One step of the innermost wait: `None` while it goes on, `Some` when over.
pub fn rp_pump_wait() -> Option<Value> {
    #[cfg(feature = "gui")]
    return crate::gui::gui_pump_wait();
    #[cfg(not(feature = "gui"))]
    Some(v_null())
}

/// Start the GUI event loop (or no-op without GUI feature).
pub fn rp_run_app() {
    #[cfg(feature = "gui")]
    {
        crate::gui::run_gui_event_loop();
    }
    #[cfg(not(feature = "gui"))]
    {
        println!("[GUI] ShowModal called — GUI not compiled, returning immediately.");
    }
}

// ---------------------------------------------------------------------------
// RJSON methods
// ---------------------------------------------------------------------------

thread_local! {
    static JSON_STORES: RefCell<HashMap<String, serde_json::Value>> = RefCell::new(HashMap::new());
}

fn json_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "parse" => {
            let text = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(val) => {
                    rp_comp_set(name, "text", v_str(&text));
                    JSON_STORES.with(|s| s.borrow_mut().insert(name_lower, val));
                    v_int(1)
                }
                Err(e) => {
                    eprintln!("[WARN] RJSON.Parse() failed: {}", e);
                    v_int(0)
                }
            }
        }
        "stringify" => {
            JSON_STORES.with(|s| {
                if let Some(val) = s.borrow().get(&name_lower) {
                    v_str(&val.to_string())
                } else {
                    v_str("{}")
                }
            })
        }
        "prettify" => {
            JSON_STORES.with(|s| {
                if let Some(val) = s.borrow().get(&name_lower) {
                    v_str(&serde_json::to_string_pretty(val).unwrap_or_else(|_| "{}".into()))
                } else {
                    v_str("{}")
                }
            })
        }
        "get" => {
            let key = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            JSON_STORES.with(|s| {
                if let Some(root) = s.borrow().get(&name_lower) {
                    json_get_path(root, &key)
                } else {
                    v_str("")
                }
            })
        }
        "set" => {
            let key = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let val = args.get(1).cloned().unwrap_or(v_null());
            JSON_STORES.with(|s| {
                let mut store = s.borrow_mut();
                let root = store.entry(name_lower.clone()).or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                json_set_path(root, &key, &val);
            });
            v_null()
        }
        "has" => {
            let key = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            JSON_STORES.with(|s| {
                if let Some(root) = s.borrow().get(&name_lower) {
                    if let Some(obj) = root.as_object() {
                        v_int(if obj.contains_key(&key) { 1 } else { 0 })
                    } else { v_int(0) }
                } else { v_int(0) }
            })
        }
        "remove" => {
            let key = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            JSON_STORES.with(|s| {
                if let Some(root) = s.borrow_mut().get_mut(&name_lower) {
                    if let Some(obj) = root.as_object_mut() {
                        obj.remove(&key);
                    }
                }
            });
            v_null()
        }
        "count" => {
            JSON_STORES.with(|s| {
                if let Some(root) = s.borrow().get(&name_lower) {
                    match root {
                        serde_json::Value::Object(m) => v_int(m.len() as i64),
                        serde_json::Value::Array(a) => v_int(a.len() as i64),
                        _ => v_int(0),
                    }
                } else { v_int(0) }
            })
        }
        "keys" => {
            JSON_STORES.with(|s| {
                if let Some(root) = s.borrow().get(&name_lower) {
                    if let Some(obj) = root.as_object() {
                        let keys: Vec<String> = obj.keys().cloned().collect();
                        v_str(&keys.join(","))
                    } else { v_str("") }
                } else { v_str("") }
            })
        }
        "loadfile" => {
            let filename = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            match std::fs::read_to_string(&filename) {
                Ok(text) => {
                    match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(val) => {
                            rp_comp_set(name, "filename", v_str(&filename));
                            rp_comp_set(name, "text", v_str(&text));
                            JSON_STORES.with(|s| s.borrow_mut().insert(name_lower, val));
                            v_int(1)
                        }
                        Err(e) => {
                            eprintln!("[WARN] RJSON.LoadFile() parse error: {}", e);
                            v_int(0)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[WARN] RJSON.LoadFile() read error: {}", e);
                    v_int(0)
                }
            }
        }
        "savefile" => {
            let filename = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            JSON_STORES.with(|s| {
                if let Some(val) = s.borrow().get(&name_lower) {
                    let pretty = serde_json::to_string_pretty(val).unwrap_or_else(|_| "{}".into());
                    match std::fs::write(&filename, &pretty) {
                        Ok(()) => {
                            rp_comp_set(name, "filename", v_str(&filename));
                            v_int(1)
                        }
                        Err(e) => {
                            eprintln!("[WARN] RJSON.SaveFile() write error: {}", e);
                            v_int(0)
                        }
                    }
                } else {
                    match std::fs::write(&filename, "{}") {
                        Ok(()) => v_int(1),
                        Err(e) => {
                            eprintln!("[WARN] RJSON.SaveFile() write error: {}", e);
                            v_int(0)
                        }
                    }
                }
            })
        }
        "clear" => {
            JSON_STORES.with(|s| {
                s.borrow_mut().insert(name_lower, serde_json::Value::Object(serde_json::Map::new()));
            });
            v_null()
        }
        _ => {
            eprintln!("[WARN] RJSON.{}() not implemented", method);
            v_null()
        }
    }
}

fn json_get_path(root: &serde_json::Value, path: &str) -> Value {
    let mut current = root;
    for part in path.split('.') {
        if part.is_empty() { continue; }
        if let Ok(idx) = part.parse::<usize>() {
            if let Some(arr) = current.as_array() {
                if let Some(v) = arr.get(idx) { current = v; } else { return v_str(""); }
            } else { return v_str(""); }
        } else if let Some(obj) = current.as_object() {
            if let Some(v) = obj.get(part) { current = v; } else { return v_str(""); }
        } else { return v_str(""); }
    }
    match current {
        serde_json::Value::Null => v_str(""),
        serde_json::Value::Bool(b) => v_int(if *b { 1 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { v_int(i) }
            else if let Some(f) = n.as_f64() { v_dbl(f) }
            else { v_str(&n.to_string()) }
        }
        serde_json::Value::String(s) => v_str(s),
        other => v_str(&other.to_string()),
    }
}

fn json_set_path(root: &mut serde_json::Value, path: &str, val: &Value) {
    let parts: Vec<&str> = path.split('.').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() { return; }
    let mut current = root;
    for part in &parts[..parts.len()-1] {
        if let Ok(_idx) = part.parse::<usize>() {
            // array navigation not supported for set yet
            return;
        }
        if !current.as_object().map_or(false, |o| o.contains_key(*part)) {
            if let Some(obj) = current.as_object_mut() {
                obj.insert(part.to_string(), serde_json::Value::Object(serde_json::Map::new()));
            }
        }
        if let Some(obj) = current.as_object_mut() {
            current = obj.get_mut(*part).unwrap();
        } else { return; }
    }
    let last = parts.last().unwrap();
    let json_val = value_to_json(val);
    if let Some(obj) = current.as_object_mut() {
        obj.insert(last.to_string(), json_val);
    }
}

fn value_to_json(val: &Value) -> serde_json::Value {
    let s = val.to_string_val();
    if let Ok(i) = s.parse::<i64>() {
        serde_json::Value::Number(serde_json::Number::from(i))
    } else if let Ok(f) = s.parse::<f64>() {
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::String(s))
    } else if s == "true" || s == "1" {
        serde_json::Value::Bool(true)
    } else if s == "false" || s == "0" {
        serde_json::Value::Bool(false)
    } else {
        serde_json::Value::String(s)
    }
}

// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// Generic GUI component methods (stub for non-GUI builds, real with GUI)
// ---------------------------------------------------------------------------

fn gui_generic_method(name: &str, comp_type: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "showmodal" => {
            #[cfg(feature = "gui")]
            {
                crate::gui::gui_showmodal(name);
                return v_null();
            }
            #[cfg(not(feature = "gui"))]
            {
                println!("[GUI] {}.ShowModal() — GUI not compiled.", name);
                v_null()
            }
        }
        "show" => {
            #[cfg(feature = "gui")]
            {
                crate::gui::gui_show(name);
                return v_null();
            }
            #[cfg(not(feature = "gui"))]
            {
                println!("[GUI] {}.Show() — GUI not compiled.", name);
                v_null()
            }
        }
        "close" | "hide" => {
            #[cfg(feature = "gui")]
            {
                if method == "close" {
                    crate::gui::gui_close(name);
                } else {
                    crate::gui::gui_hide(name);
                }
                return v_null();
            }
            #[cfg(not(feature = "gui"))]
            {
                println!("[GUI] {}.Close()", name);
                v_null()
            }
        }
        // `Form.Repaint` (Refresh, Update, Paint): drawn again, OnPaint.
        "repaint" | "refresh" | "update" | "paint" => {
            #[cfg(feature = "gui")]
            crate::gui::canvas_redraw(name);
            rp_fire_event(name, "onpaint");
            v_null()
        }
        "center" => {
            #[cfg(feature = "gui")]
            {
                crate::gui::gui_center(name);
                return v_null();
            }
            #[cfg(not(feature = "gui"))]
            {
                println!("[GUI] {}.Center()", name);
                v_null()
            }
        }
        "setparent" => {
            let parent = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            #[cfg(feature = "gui")]
            {
                crate::gui::gui_set_parent(name, &parent);
                return v_null();
            }
            #[cfg(not(feature = "gui"))]
            {
                rp_comp_set(name, "parent", v_str(&parent));
                v_null()
            }
        }
        "clear" => {
            // For ListBox, ComboBox, StringGrid, etc.
            #[cfg(feature = "gui")]
            {
                crate::gui::gui_widget_clear(name);
            }
            rp_comp_set(name, "items", v_str(""));
            rp_comp_set(name, "count", v_int(0));
            rp_comp_set(name, "itemindex", v_int(-1));
            v_null()
        }
        "additems" | "additem" => {
            // Add items to ListBox/ComboBox
            let current = rp_comp_get(name, "items").to_string_val();
            for arg in args {
                let item = arg.to_string_val();
                let new_items = if current.is_empty() {
                    item.clone()
                } else {
                    format!("{}\n{}", current, item)
                };
                rp_comp_set(name, "items", v_str(&new_items));
                #[cfg(feature = "gui")]
                {
                    crate::gui::gui_widget_add_items(name, &item);
                }
            }
            let count = rp_comp_get(name, "items")
                .to_string_val()
                .lines()
                .count();
            rp_comp_set(name, "count", v_int(count as i64));
            v_null()
        }
        "deleteitems" | "deleteitem" | "removeitem" => {
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            if idx >= 0 {
                let items_str = rp_comp_get(name, "items").to_string_val();
                let mut items: Vec<&str> = items_str.lines().collect();
                let idx = idx as usize;
                if idx < items.len() {
                    items.remove(idx);
                }
                rp_comp_set(name, "items", v_str(&items.join("\n")));
                rp_comp_set(name, "count", v_int(items.len() as i64));
            }
            v_null()
        }
        "setfocus" | "focus" => {
            v_null()
        }
        "click" => {
            rp_fire_event(name, "onclick");
            v_null()
        }
        // (the title bar's own buttons are the system's: the set is kept,
        // and RapidQ's manual allows an icon to stay, "greyed out")
        "addbordericons" | "delbordericons" if comp_type == "RFORM" => {
            let bits = crate::value::builtins::border_icons(&rp_comp_get(name, "bordericons"), args, method == "addbordericons");
            rp_comp_set(name, "bordericons", bits);
            v_null()
        }
        "selectall" => { v_null() }
        "copy" => { v_null() }
        "paste" => { v_null() }
        "cut" => { v_null() }
        "execute" => {
            // For dialogs (POpenDialog, PSaveDialog)
            match comp_type {
                "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG" => {
                    // In non-GUI mode, return false
                    #[cfg(feature = "gui")]
                    {
                        return crate::gui::gui_dialog_execute(name, comp_type);
                    }
                    #[cfg(not(feature = "gui"))]
                    {
                        println!("[GUI] {}.Execute() — dialog stub", name);
                        v_int(0)
                    }
                }
                _ => v_null(),
            }
        }
        // Canvas drawing methods
        "line" | "rect" | "fillrect" | "circle" | "ellipse"
        | "setpixel" | "getpixel" | "drawtext" | "loadimage" | "saveimage" => {
            // Store drawing commands for later rendering
            v_null()
        }
        _ => {
            eprintln!("[WARN] {}.{}() not implemented for type {}", name, method, comp_type);
            v_null()
        }
    }
}

/// Check if a type name is a known component type.
pub fn is_component_type(type_name: &str) -> bool {
    matches!(
        type_name.to_uppercase().as_str(),
        "RFORM" | "RFORMMDI" | "RBUTTON" | "RLABEL" | "REDIT" | "RPANEL"
        | "RCHECKBOX" | "RRADIOBUTTON" | "RCOMBOBOX" | "RLISTBOX" | "RFILELISTBOX" | "RDIRTREE"
        | "RTIMER" | "RIMAGE" | "RCANVAS" | "RSTRINGGRID" | "RTABCONTROL"
        | "RTREEVIEW" | "RMAINMENU" | "RMENUITEM" | "RPOPUPMENU"
        | "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG"
        | "RTOOLBAR" | "RSTATUSBAR" | "RPROGRESS" | "RRICHEDIT" | "RMEMO"
        | "RSCROLLBAR" | "RUPDOWN" | "RDATETIMEPICKER" | "RMONTHCALENDAR"
        | "RHEADER" | "RRECT" | "RHEADERCONTROL" | "RIMAGELIST" | "RFILESTREAM" | "RJSON" | "RSTRINGLIST"
        | "RFONT" | "RMEMORYSTREAM" | "RBITMAP"
        | "RTRACKBAR" | "RSCROLLBOX" | "RSPLITTER" | "RPRINTER"
        | "RSQLITE" | "RMYSQL"
        | "RSOCKET" | "RSERVERSOCKET" | "RHTTP"
        | "RLISTVIEW" | "RPROGRESSBAR"
        | "RNUM" | "RPLOT" | "RDATAFRAME"
        | "RDESIGNSURFACE" | "RCODEEDITOR" | "RGROUPBOX"
    )
}

/// Get all child components whose "parent" property matches the given form name.
/// Stores a property without any of `rp_comp_set`'s effects.
pub(crate) fn store_prop(name: &str, prop: &str, val: Value) {
    COMPONENTS.with(|c| {
        if let Some(comp) = c.borrow_mut().get_mut(&name.to_lowercase()) {
            comp.properties.insert(prop.to_lowercase(), val);
        }
    });
}

pub fn get_children_of(parent_name: &str) -> Vec<(String, String)> {
    let parent_lower = parent_name.to_lowercase();
    COMPONENTS.with(|c| {
        let comps = c.borrow();
        let mut children: Vec<(String, String, u32)> = Vec::new();
        for (name, comp) in comps.iter() {
            if name == &parent_lower {
                continue;
            }
            if let Some(parent_val) = comp.properties.get("parent") {
                if parent_val.to_string_val().to_lowercase() == parent_lower {
                    children.push((name.clone(), comp.type_name.clone(), comp.creation_order));
                }
            }
        }
        // Sort by creation order to match the original CREATE block order
        children.sort_by_key(|c| c.2);
        children.into_iter().map(|(n, t, _)| (n, t)).collect()
    })
}

/// Check if a member name is a known method (not a property) for component types.
/// Used by codegen to decide whether `obj.member` (no parens) is a method call.
pub fn is_component_method(member: &str) -> bool {
    matches!(
        member.to_lowercase().as_str(),
        // Form/Widget methods
        "showmodal" | "close" | "show" | "hide" | "refresh" | "center" | "setparent"
        // Collection methods
        | "clear" | "additems" | "additem" | "deleteitems" | "deleteitem" | "removeitem"
        | "addrow" | "sort" | "find"
        // Focus/input methods
        | "setfocus" | "focus" | "click" | "selectall" | "copy" | "paste" | "cut"
        // Dialog methods
        | "execute"
        // Database methods
        | "connect" | "disconnect" | "query" | "fetchrow" | "fetchfield"
        | "fieldseek" | "rowseek" | "row" | "rowblob" | "escapestring"
        | "selectdb" | "createdb" | "dropdb"
        // Network methods
        | "write" | "writeline" | "read" | "readline"
        | "bind" | "listen" | "accept"
        | "start" | "stop" | "broadcast"
        | "get" | "post"
        // FileStream methods
        | "open" | "readall" | "eof"
        // StringList methods
        | "loadfromfile" | "savetofile" | "add" | "delete"
        // Canvas methods
        | "line" | "rect" | "fillrect" | "circle" | "ellipse"
        | "setpixel" | "getpixel" | "drawtext" | "loadimage" | "saveimage"
        // TreeView methods
        | "addroot" | "addchild" | "expand" | "collapse"
        // Design surface methods
        | "addcomponent" | "getname" | "gettype"
        | "getcompx" | "getcompy" | "getcompw" | "getcomph"
        | "setprop" | "getprop" | "setcompbounds" | "setname"
        | "selectcomp" | "removecomponent" | "clearall"
        // StringGrid methods
        | "cell" | "cells" | "setcell" | "setsuggestions"
        // CodeEditor methods
        | "getsublist" | "gotosub" | "gotoline"
        // TabControl methods
        | "addtabs" | "tab"
    )
}

/// QSTATUSBAR panels: `AddPanels "Ready", "Line 1"` appends panels, kept as
/// the component's properties `panel(i).caption` / `panel(i).width` (the
/// same keys `SB.Panel(i).Caption = …` writes) and `panelcount`; the GUI
/// draws them.
fn statusbar_method(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    match method {
        "addpanels" => {
            let mut n = rp_comp_get(name, "panelcount").to_i64().max(0);
            for a in args {
                rp_comp_set(name, &format!("panel({n}).caption"), v_str(&a.to_string_val()));
                n += 1;
            }
            rp_comp_set(name, "panelcount", v_int(n));
            Some(v_null())
        }
        _ => None,
    }
}

/// Generic storage for indexed sub-object members (see `rp_comp_method`).
fn indexed_sub_object(name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let (sub, member) = method.split_once('.')?;
    if sub.is_empty() || member.is_empty() || member.contains('.') {
        return None;
    }
    let setter = member.ends_with('=');
    let member = member.trim_end_matches('=');
    let n_index = if setter { args.len().checked_sub(1)? } else { args.len() };
    let index: Vec<String> = args[..n_index].iter().map(|v| v.to_string_val()).collect();
    let key = format!("{sub}({}).{member}", index.join(","));
    if setter {
        rp_comp_set(name, &key, args[n_index].clone());
        Some(Value::Null)
    } else {
        Some(rp_comp_get(name, &key))
    }
}
