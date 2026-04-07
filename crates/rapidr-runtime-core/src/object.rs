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
                props.insert("text".into(), v_str(""));
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(120));
                props.insert("height".into(), v_int(25));
                props.insert("itemindex".into(), v_int(-1));
                props.insert("items".into(), v_str(""));
                props.insert("count".into(), v_int(0));
            }
            "RLISTBOX" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(120));
                props.insert("height".into(), v_int(100));
                props.insert("itemindex".into(), v_int(-1));
                props.insert("items".into(), v_str(""));
                props.insert("count".into(), v_int(0));
            }
            "RTIMER" => {
                props.insert("enabled".into(), v_bool(false));
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
            "RSTRINGGRID" => {
                props.insert("left".into(), v_int(0));
                props.insert("top".into(), v_int(0));
                props.insert("width".into(), v_int(300));
                props.insert("height".into(), v_int(200));
                props.insert("rowcount".into(), v_int(0));
                props.insert("colcount".into(), v_int(0));
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
            "ROPENDIALOG" | "RSAVEDIALOG" => {
                props.insert("filename".into(), v_str(""));
                props.insert("filter".into(), v_str(""));
                props.insert("initialdir".into(), v_str(""));
                props.insert("title".into(), v_str(""));
            }
            "RCOLORDIALOG" | "RFONTDIALOG" => {
                props.insert("color".into(), v_int(0));
            }
            "RSTATUSBAR" => {
                props.insert("simpletext".into(), v_str(""));
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
            "RFORMMDI" => {
                props.insert("caption".into(), v_str(""));
                props.insert("width".into(), v_int(800));
                props.insert("height".into(), v_int(600));
                props.insert("left".into(), v_int(100));
                props.insert("top".into(), v_int(100));
                props.insert("visible".into(), v_bool(true));
                props.insert("color".into(), v_int(0xFFFFFF));
                props.insert("borderstyle".into(), v_int(2));
                props.insert("childcount".into(), v_int(0));
                props.insert("childmax".into(), v_int(1024));
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

/// Handler enum supporting event callbacks with 0, 1, or 2 parameters.
#[derive(Clone)]
pub enum EventHandler {
    Arity0(fn()),
    Arity1(fn(Value)),
    Arity2(fn(Value, Value)),
    Arity3(fn(Value, Value, Value)),
    Arity4(fn(Value, Value, Value, Value)),
    Arity5(fn(Value, Value, Value, Value, Value)),
}

thread_local! {
    static COMPONENTS: RefCell<HashMap<String, RpComponent>> = RefCell::new(HashMap::new());
    static EVENT_HANDLERS: RefCell<HashMap<(String, String), EventHandler>> = RefCell::new(HashMap::new());
    static CREATION_COUNTER: RefCell<u32> = RefCell::new(0);
}

/// Create a new component and register it in the global registry.
pub fn rp_create_component(name: &str, type_name: &str) {
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
}

/// Set a property on a registered component.
pub fn rp_comp_set(name: &str, prop: &str, val: Value) {
    let prop_lower = prop.to_lowercase();

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
        // Update caption/simpletext on the widget
        if prop_lower == "caption" || prop_lower == "simpletext" {
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
        if let Some(comp) = c.borrow_mut().get_mut(&name.to_lowercase()) {
            comp.properties.insert(prop_lower, val);
        }
    });
}

/// Get a property from a registered component.
pub fn rp_comp_get(name: &str, prop: &str) -> Value {
    let prop_lower = prop.to_lowercase();

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
            "RSTRINGGRID" => {
                if let Some(v) = crate::gui::string_grid_get(name, &prop_lower) {
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
            .unwrap_or_else(v_null)
    })
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

/// Call a method on a registered component.
/// Dispatches to the appropriate backend based on component type.
pub fn rp_comp_method(name: &str, method: &str, args: &[Value]) -> Value {
    let comp_type = rp_comp_type(name);
    let method_lower = method.to_lowercase();

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
        "RFILESTREAM" => filestream_method(name, &method_lower, args),
        "RJSON" => json_method(name, &method_lower, args),
        "RSTRINGLIST" => stringlist_method(name, &method_lower, args),
        // Specialized GUI component method dispatch
        #[cfg(feature = "gui")]
        "RDESIGNSURFACE" => crate::gui::design_surface_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RSTRINGGRID" => crate::gui::string_grid_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RCODEEDITOR" => crate::gui::code_editor_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RTABCONTROL" => crate::gui::tab_control_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RTREEVIEW" => crate::gui::tree_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RCANVAS" => crate::gui::canvas_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RFORMMDI" => crate::gui::formmdi_method(name, &method_lower, args),
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

/// Fire an event on a component (called by GUI backend).
pub fn rp_fire_event(name: &str, event: &str) {
    let handler = EVENT_HANDLERS.with(|h| {
        h.borrow()
            .get(&(name.to_lowercase(), event.to_lowercase()))
            .cloned()
    });
    if let Some(handler) = handler {
        match handler {
            EventHandler::Arity0(f) => f(),
            EventHandler::Arity1(f) => f(v_null()),
            EventHandler::Arity2(f) => f(v_null(), v_null()),
            EventHandler::Arity3(f) => f(v_null(), v_null(), v_null()),
            EventHandler::Arity4(f) => f(v_null(), v_null(), v_null(), v_null()),
            EventHandler::Arity5(f) => f(v_null(), v_null(), v_null(), v_null(), v_null()),
        }
    }
}

/// Fire an event with 1 argument.
pub fn rp_fire_event_1(name: &str, event: &str, arg: Value) {
    let handler = EVENT_HANDLERS.with(|h| {
        h.borrow()
            .get(&(name.to_lowercase(), event.to_lowercase()))
            .cloned()
    });
    if let Some(handler) = handler {
        match handler {
            EventHandler::Arity0(f) => f(),
            EventHandler::Arity1(f) => f(arg),
            EventHandler::Arity2(f) => f(arg, v_null()),
            EventHandler::Arity3(f) => f(arg, v_null(), v_null()),
            EventHandler::Arity4(f) => f(arg, v_null(), v_null(), v_null()),
            EventHandler::Arity5(f) => f(arg, v_null(), v_null(), v_null(), v_null()),
        }
    }
}

/// Fire an event with 2 arguments.
pub fn rp_fire_event_2(name: &str, event: &str, arg1: Value, arg2: Value) {
    let handler = EVENT_HANDLERS.with(|h| {
        h.borrow()
            .get(&(name.to_lowercase(), event.to_lowercase()))
            .cloned()
    });
    if let Some(handler) = handler {
        match handler {
            EventHandler::Arity0(f) => f(),
            EventHandler::Arity1(f) => f(arg1),
            EventHandler::Arity2(f) => f(arg1, arg2),
            EventHandler::Arity3(f) => f(arg1, arg2, v_null()),
            EventHandler::Arity4(f) => f(arg1, arg2, v_null(), v_null()),
            EventHandler::Arity5(f) => f(arg1, arg2, v_null(), v_null(), v_null()),
        }
    }
}

/// Fire an event with 5 arguments.
pub fn rp_fire_event_5(name: &str, event: &str, a1: Value, a2: Value, a3: Value, a4: Value, a5: Value) {
    let handler = EVENT_HANDLERS.with(|h| {
        h.borrow()
            .get(&(name.to_lowercase(), event.to_lowercase()))
            .cloned()
    });
    if let Some(handler) = handler {
        match handler {
            EventHandler::Arity0(f) => f(),
            EventHandler::Arity1(f) => f(a1),
            EventHandler::Arity2(f) => f(a1, a2),
            EventHandler::Arity3(f) => f(a1, a2, a3),
            EventHandler::Arity4(f) => f(a1, a2, a3, a4),
            EventHandler::Arity5(f) => f(a1, a2, a3, a4, a5),
        }
    }
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
// RFileStream methods
// ---------------------------------------------------------------------------

thread_local! {
    static FILESTREAMS: RefCell<HashMap<String, std::io::BufReader<std::fs::File>>> = RefCell::new(HashMap::new());
    static FILESTREAM_WRITERS: RefCell<HashMap<String, std::fs::File>> = RefCell::new(HashMap::new());
}

fn filestream_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "open" => {
            let filename = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let mode = args.get(1).map(|v| v.to_i64()).unwrap_or(0);
            let result = if mode == 65535 {
                // fmCreate — write mode
                std::fs::File::create(&filename).map(|f| {
                    FILESTREAM_WRITERS.with(|w| {
                        w.borrow_mut().insert(name_lower.clone(), f);
                    });
                })
            } else {
                // Read mode
                std::fs::File::open(&filename).map(|f| {
                    use std::io::BufReader;
                    FILESTREAMS.with(|s| {
                        s.borrow_mut().insert(name_lower.clone(), BufReader::new(f));
                    });
                })
            };
            match result {
                Ok(()) => {
                    rp_comp_set(name, "filename", v_str(&filename));
                    v_int(1)
                }
                Err(_) => v_int(0),
            }
        }
        "close" => {
            FILESTREAMS.with(|s| { s.borrow_mut().remove(&name_lower); });
            FILESTREAM_WRITERS.with(|w| { w.borrow_mut().remove(&name_lower); });
            v_null()
        }
        "readline" | "readln" => {
            use std::io::BufRead;
            FILESTREAMS.with(|s| {
                if let Some(reader) = s.borrow_mut().get_mut(&name_lower) {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => v_str(""),
                        Ok(_) => {
                            let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
                            v_str(trimmed)
                        }
                        Err(_) => v_str(""),
                    }
                } else {
                    v_str("")
                }
            })
        }
        "writeline" | "writeln" => {
            use std::io::Write;
            let text = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            FILESTREAM_WRITERS.with(|w| {
                if let Some(writer) = w.borrow_mut().get_mut(&name_lower) {
                    let _ = writeln!(writer, "{}", text);
                }
            });
            v_null()
        }
        "eof" => {
            // Check if we're at EOF
            use std::io::BufRead;
            FILESTREAMS.with(|s| {
                if let Some(reader) = s.borrow_mut().get_mut(&name_lower) {
                    let buf = reader.fill_buf().unwrap_or(&[]);
                    v_int(if buf.is_empty() { 1 } else { 0 })
                } else {
                    v_int(1)
                }
            })
        }
        "readall" => {
            use std::io::Read;
            FILESTREAMS.with(|s| {
                if let Some(reader) = s.borrow_mut().get_mut(&name_lower) {
                    let mut buf = String::new();
                    let _ = reader.read_to_string(&mut buf);
                    v_str(&buf)
                } else {
                    v_str("")
                }
            })
        }
        _ => {
            eprintln!("[WARN] RFileStream.{}() not implemented", method);
            v_null()
        }
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
// RStringList methods
// ---------------------------------------------------------------------------

thread_local! {
    static STRINGLISTS: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

fn stringlist_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    STRINGLISTS.with(|sl| {
        let mut lists = sl.borrow_mut();
        let list = lists.entry(name_lower.clone()).or_insert_with(Vec::new);
        match method {
            "clear" => {
                list.clear();
                rp_comp_set(name, "count", v_int(0));
                v_null()
            }
            "add" => {
                let item = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                list.push(item);
                rp_comp_set(name, "count", v_int(list.len() as i64));
                v_null()
            }
            "delete" => {
                let idx = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
                if idx < list.len() {
                    list.remove(idx);
                }
                rp_comp_set(name, "count", v_int(list.len() as i64));
                v_null()
            }
            "loadfromfile" => {
                let filename = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                if let Ok(content) = std::fs::read_to_string(&filename) {
                    *list = content.lines().map(String::from).collect();
                    rp_comp_set(name, "count", v_int(list.len() as i64));
                }
                v_null()
            }
            "savetofile" => {
                let filename = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                let content = list.join("\n");
                let _ = std::fs::write(&filename, content);
                v_null()
            }
            "sort" => {
                list.sort();
                v_null()
            }
            "find" => {
                let needle = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                match list.iter().position(|s| *s == needle) {
                    Some(pos) => v_int(pos as i64),
                    None => v_int(-1),
                }
            }
            "items" | "item" => {
                let idx = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
                if idx < list.len() {
                    v_str(&list[idx])
                } else {
                    v_str("")
                }
            }
            _ => {
                eprintln!("[WARN] RStringList.{}() not implemented", method);
                v_null()
            }
        }
    })
}

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
                crate::gui::gui_close(name);
                return v_null();
            }
            #[cfg(not(feature = "gui"))]
            {
                println!("[GUI] {}.Close()", name);
                v_null()
            }
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
        "refresh" => {
            v_null()
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
        "selectall" => { v_null() }
        "copy" => { v_null() }
        "paste" => { v_null() }
        "cut" => { v_null() }
        "execute" => {
            // For dialogs (POpenDialog, PSaveDialog)
            match comp_type {
                "ROPENDIALOG" | "RSAVEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG" => {
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
        // StringGrid methods
        "addrow" => {
            let rowcount = rp_comp_get(name, "rowcount").to_i64();
            rp_comp_set(name, "rowcount", v_int(rowcount + 1));
            v_null()
        }
        "cells" | "cell" => {
            // Get/set cell value: grid.Cells(col, row)
            let col = args.first().map(|v| v.to_i64()).unwrap_or(0);
            let row = args.get(1).map(|v| v.to_i64()).unwrap_or(0);
            let key = format!("_cell_{}_{}", col, row);
            rp_comp_get(name, &key)
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
        | "RCHECKBOX" | "RRADIOBUTTON" | "RCOMBOBOX" | "RLISTBOX"
        | "RTIMER" | "RIMAGE" | "RCANVAS" | "RSTRINGGRID" | "RTABCONTROL"
        | "RTREEVIEW" | "RMAINMENU" | "RMENUITEM" | "RPOPUPMENU"
        | "ROPENDIALOG" | "RSAVEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG"
        | "RTOOLBAR" | "RSTATUSBAR" | "RPROGRESS" | "RRICHEDIT" | "RMEMO"
        | "RSCROLLBAR" | "RUPDOWN" | "RDATETIMEPICKER" | "RMONTHCALENDAR"
        | "RHEADERCONTROL" | "RIMAGELIST" | "RFILESTREAM" | "RJSON" | "RSTRINGLIST"
        | "RTRACKBAR" | "RSCROLLBOX" | "RSPLITTER" | "RPRINTER"
        | "RSQLITE" | "RMYSQL"
        | "RSOCKET" | "RSERVERSOCKET" | "RHTTP"
        | "RLISTVIEW" | "RPROGRESSBAR"
        | "RNUM" | "RPLOT" | "RDATAFRAME"
        | "RDESIGNSURFACE" | "RCODEEDITOR" | "RGROUPBOX"
    )
}

/// Get all child components whose "parent" property matches the given form name.
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
        "showmodal" | "close" | "show" | "hide" | "refresh" | "center"
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
