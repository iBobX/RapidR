//! Component registry and event system for the web runtime.
//!
//! Mirrors the desktop `object.rs` API — same function signatures so that
//! generated code works identically on both targets.

use crate::gui_web;
use crate::value::{v_bool, v_int, v_null, v_str, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// ---------------------------------------------------------------------------
// Component struct
// ---------------------------------------------------------------------------

pub struct RpComponent {
    pub type_name: String,
    pub properties: HashMap<String, Value>,
    pub creation_order: u32,
}

// ---------------------------------------------------------------------------
// Thread-local storage (single-threaded in WASM, but keeps API compatible)
// ---------------------------------------------------------------------------

/// A handler bound to a component's event (shared with the desktop runtime).
use rapidr_value::events::Handler as EventHandler;

/// Type alias for the indirect event dispatcher used by the bytecode
/// interpreter's WebHost. Receives `(handler_id, args)`.
pub type IndirectDispatcher = Box<dyn Fn(u32, &[Value])>;

thread_local! {
    static COMPONENTS: RefCell<HashMap<String, RpComponent>> = RefCell::new(HashMap::new());
    static EVENT_HANDLERS: RefCell<HashMap<(String, String), EventHandler>> = RefCell::new(HashMap::new());
    static CREATION_COUNTER: RefCell<u32> = RefCell::new(0);
    static TIMER_HANDLES: RefCell<HashMap<String, i32>> = RefCell::new(HashMap::new());
    static INDIRECT_DISPATCHER: RefCell<Option<IndirectDispatcher>> = const { RefCell::new(None) };
}

/// Install a thread-local indirect event dispatcher (used by the
/// bytecode VM's WebHost). Returns the previously-installed one.
pub fn rp_set_event_dispatcher(d: IndirectDispatcher) -> Option<IndirectDispatcher> {
    INDIRECT_DISPATCHER.with(|s| s.borrow_mut().replace(d))
}

/// Remove the indirect event dispatcher (returns it if present).
pub fn rp_clear_event_dispatcher() -> Option<IndirectDispatcher> {
    INDIRECT_DISPATCHER.with(|s| s.borrow_mut().take())
}

/// Bind an indirect event handler (carries an opaque id, e.g. a
/// bytecode function index).
pub fn rp_bind_event_indirect(name: &str, event: &str, handler_id: u32) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();
    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Indirect(handler_id));
    });
    bind_dom_event(&uname, &levent);
}

fn dispatch_indirect(handler_id: u32, args: &[Value]) {
    INDIRECT_DISPATCHER.with(|slot| {
        if let Some(d) = slot.borrow().as_ref() {
            d(handler_id, args);
        }
    });
}

// ---------------------------------------------------------------------------
// Component creation
// ---------------------------------------------------------------------------

pub fn rp_create_component(name: &str, type_name: &str) {
    // A QFORMMDI is a QFORM whose client area holds child windows (mdi_web.rs).
    if type_name.eq_ignore_ascii_case("RFORMMDI") {
        rapidr_value::mdi::register(name);
        return rp_create_component(name, "RFORM");
    }
    let uname = name.to_uppercase();
    let utype = type_name.to_uppercase();

    // Idempotent: skip if already created
    let already = COMPONENTS.with(|c| c.borrow().contains_key(&uname));
    if already {
        return;
    }

    let order = CREATION_COUNTER.with(|c| {
        let mut c = c.borrow_mut();
        *c += 1;
        *c
    });

    let mut props = HashMap::new();
    // Set default properties based on type
    match utype.as_str() {
        "RFORM" => {
            props.insert("caption".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(100));
            props.insert("top".to_string(), v_int(100));
            props.insert("width".to_string(), v_int(640));
            props.insert("height".to_string(), v_int(480));
            props.insert("visible".to_string(), v_bool(true));
        }
        "RBUTTON" => {
            props.insert("caption".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(100));
            props.insert("height".to_string(), v_int(30));
        }
        "RLABEL" => {
            props.insert("caption".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(100));
            props.insert("height".to_string(), v_int(20));
        }
        "REDIT" => {
            props.insert("text".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(120));
            props.insert("height".to_string(), v_int(25));
        }
        "RMEMO" | "RRICHEDIT" => {
            props.insert("text".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(200));
            props.insert("height".to_string(), v_int(150));
        }
        "RPANEL" | "RDESIGNSURFACE" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(200));
            props.insert("height".to_string(), v_int(150));
        }
        "RCHECKBOX" | "RRADIOBUTTON" => {
            props.insert("caption".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(120));
            props.insert("height".to_string(), v_int(25));
        }
        "RCOMBOBOX" | "RLISTBOX" | "RFILELISTBOX" | "RDIRTREE" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(150));
            props.insert("height".to_string(), v_int(25));
        }
        // QTIMER: Enabled is True by default (manual).
        "RTIMER" => {
            props.insert("interval".to_string(), v_int(1000));
            props.insert("enabled".to_string(), v_bool(true));
        }
        "RIMAGE" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(100));
            props.insert("height".to_string(), v_int(100));
        }
        "RCANVAS" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(400));
            props.insert("height".to_string(), v_int(300));
        }
        "RSTRINGGRID" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(300));
            props.insert("height".to_string(), v_int(200));
        }
        "RPROGRESS" | "RPROGRESSBAR" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(200));
            props.insert("height".to_string(), v_int(25));
            props.insert("min".to_string(), v_int(0));
            props.insert("max".to_string(), v_int(100));
            props.insert("position".to_string(), v_int(0));
        }
        "RWEBVIEW" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(400));
            props.insert("height".to_string(), v_int(300));
        }
        "RWEBAUDIO" | "RWEBVIDEO" => {
            props.insert("src".to_string(), v_str(""));
            props.insert("volume".to_string(), Value::Double(1.0));
        }
        "RWEBSTORAGE" => {
            props.insert("storagetype".to_string(), v_str("local"));
        }
        "RWEBNOTIFICATION" => {
            props.insert("title".to_string(), v_str("Notification"));
            props.insert("body".to_string(), v_str(""));
        }
        "RNUM" => {
            // Non-visual component — no DOM element
        }
        "RDATAFRAME" => {
            // Non-visual component — no DOM element
            crate::datascience_web::init_dataframe(&uname);
        }
        "RPLOT" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(600));
            props.insert("height".to_string(), v_int(400));
        }
        "RSQLITE" => {
            props.insert("connected".to_string(), v_int(0));
            props.insert("db".to_string(), v_str(""));
            props.insert("rowcount".to_string(), v_int(0));
            props.insert("colcount".to_string(), v_int(0));
            props.insert("fieldcount".to_string(), v_int(0));
        }
        "RCOOLBTN" => {
            props.insert("caption".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(80));
            props.insert("height".to_string(), v_int(30));
            props.insert("flat".to_string(), v_bool(false));
            props.insert("groupindex".to_string(), v_int(0));
            props.insert("down".to_string(), v_bool(false));
            props.insert("allowallup".to_string(), v_bool(false));
            props.insert("numbmps".to_string(), v_int(1));
        }
        "ROVALBTN" => {
            props.insert("caption".to_string(), v_str(""));
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(60));
            props.insert("height".to_string(), v_int(60));
            props.insert("color".to_string(), v_int(0xDCDCDC));
            props.insert("colorhighlight".to_string(), v_int(0xFFFFFF));
            props.insert("colorshadow".to_string(), v_int(0x808080));
            props.insert("flat".to_string(), v_bool(false));
            props.insert("groupindex".to_string(), v_int(0));
            props.insert("down".to_string(), v_bool(false));
        }
        "RJSON" => {
            props.insert("text".to_string(), v_str(""));
            props.insert("filename".to_string(), v_str(""));
            props.insert("count".to_string(), v_int(0));
        }
        "RFILESTREAM" => {
            // In-browser virtual file: text + filename, plus a download/pickfile bridge.
            props.insert("text".to_string(), v_str(""));
            props.insert("filename".to_string(), v_str(""));
            props.insert("position".to_string(), v_int(0));
            props.insert("eof".to_string(), v_bool(false));
            props.insert("mimetype".to_string(), v_str("text/plain"));
        }
        "ROPENDIALOG" | "RSAVEDIALOG" => {
            props.insert("filename".to_string(), v_str(""));
            props.insert("filter".to_string(), v_str("*.*"));
            props.insert("title".to_string(), v_str(""));
        }
        "RCOLORDIALOG" => {
            props.insert("color".to_string(), v_int(0xFFFFFF));
        }
        "RFONTDIALOG" => {
            props.insert("fontname".to_string(), v_str("Segoe UI"));
            props.insert("fontsize".to_string(), v_int(12));
            props.insert("fontbold".to_string(), v_bool(false));
            props.insert("fontitalic".to_string(), v_bool(false));
        }
        _ => {
            // Generic defaults
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(100));
            props.insert("height".to_string(), v_int(25));
        }
    }

    // QSTATUSBAR docks at the bottom, QSPLITTER at the left (layout_web).
    let align = rapidr_value::layout::default_align(&utype);
    if align != rapidr_value::layout::Align::None {
        props.insert("align".to_string(), v_int(align.value()));
    }
    // Their sizes, as on the desktop.
    let sizes: &[(&str, i64)] = match utype.as_str() {
        "RSTATUSBAR" => &[("left", 0), ("top", 0), ("width", 200), ("height", 24)],
        "RSPLITTER" => &[("left", 0), ("top", 0), ("width", 5), ("height", 200)],
        _ => &[],
    };
    for &(p, v) in sizes {
        props.insert(p.to_string(), v_int(v));
    }

    // Create the DOM element (skip for non-visual components)
    match utype.as_str() {
        "RNUM" | "RDATAFRAME" | "RSQLITE" => {
            // Non-visual: no DOM element
        }
        "RPLOT" => {
            crate::datascience_web::create_plot_widget(
                &format!("rr-{}", uname.to_lowercase()),
                &uname,
                &props,
            );
        }
        _ => {
            gui_web::gui_web_create_widget(&uname, &utype, &props);
        }
    }

    let name_clone = uname.clone();
    COMPONENTS.with(|c| {
        c.borrow_mut().insert(
            uname,
            RpComponent {
                type_name: utype,
                properties: props,
                creation_order: order,
            },
        );
    });

    gui_web::setup_data_binding(&name_clone);
    install_object_hooks();
    if rapidr_value::objects::create(name, type_name) {
        rapidr_value::objects::set_file_io(web_read_file, web_write_file);
        // (its element may exist already)
        if rapidr_value::objects::is_dirtree(name) {
            gui_web::render_dirtree(&name.to_uppercase());
        }
        if rapidr_value::objects::is_canvas(name) {
            gui_web::render_canvas(&name.to_uppercase());
        }
    }
}

/// How shared objects print on the web (`Printer.EndDoc`, [`web_print`]).
pub fn install_object_hooks() {
    rapidr_value::objects::set_print_hook(web_print);
    // RND's first seed (wasm has no clock).
    rapidr_value::builtins::set_entropy(|| (js_sys::Math::random() * 9_007_199_254_740_992.0) as u64);
}

/// `Printer.EndDoc` on the web: the document (a PDF) opens in a new tab,
/// where the browser's viewer prints it; if pop-ups are blocked, it's
/// downloaded instead.
fn web_print(job: &rapidr_value::objects::printer::PrintJob) -> Result<(), String> {
    let bytes = js_sys::Uint8Array::from(job.pdf.as_slice());
    let parts = js_sys::Array::of1(&bytes);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/pdf");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options).map_err(|_| "can't make the PDF".to_string())?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| "can't make the PDF".to_string())?;
    let window = web_sys::window().ok_or("no window")?;
    if window.open_with_url_and_target(&url, "_blank").ok().flatten().is_none() {
        let doc = window.document().ok_or("no document")?;
        let a = doc.create_element("a").map_err(|_| "no link")?.dyn_into::<web_sys::HtmlAnchorElement>().map_err(|_| "no link")?;
        a.set_href(&url);
        a.set_download(&format!("{}.pdf", if job.title.is_empty() { "RapidR print" } else { &job.title }));
        a.click();
    }
    Ok(())
}

thread_local! {
    /// Files a program saved (`Bitmap.SaveToFile`): the page has no file
    /// system, so they live for the session and can be loaded back.
    static SAVED_FILES: RefCell<HashMap<String, Vec<u8>>> = RefCell::new(HashMap::new());
}

/// Reads a file for an object (`Bitmap.LoadFromFile`, `ImageList.AddBMPFile`):
/// one saved earlier this session, or one shipped with the page (fetched
/// synchronously, as the program expects the data on the next line).
fn web_read_file(path: &str) -> Result<Vec<u8>, String> {
    if let Some(bytes) = SAVED_FILES.with(|f| f.borrow().get(path).cloned()) {
        return Ok(bytes);
    }
    let fail = |_| format!("can't read {path}");
    let xhr = web_sys::XmlHttpRequest::new().map_err(fail)?;
    xhr.open_with_async("GET", path, false).map_err(fail)?;
    // Bytes as-is: each byte arrives as one character U+0000..U+00FF (+ &HF700).
    xhr.override_mime_type("text/plain; charset=x-user-defined").map_err(fail)?;
    xhr.send().map_err(fail)?;
    match xhr.status() {
        Ok(200) | Ok(0) => {}
        _ => return Err(format!("can't read {path} (not found)")),
    }
    let text = xhr.response_text().ok().flatten().unwrap_or_default();
    Ok(text.chars().map(|c| (c as u32 & 0xFF) as u8).collect())
}

/// Makes the page's own files the ones BASIC file I/O and the stream
/// objects read and write (once).
pub fn install_file_hooks() {
    rapidr_value::objects::set_file_io(web_read_file, web_write_file);
}

/// A file's length (0 if it can't be read).
pub fn web_file_len(path: &str) -> i64 {
    web_read_file(path).map_or(0, |b| b.len() as i64)
}

/// `KILL`: the page's saved copy of the file goes.
pub fn web_remove_file(path: &str) {
    SAVED_FILES.with(|f| f.borrow_mut().remove(path));
}

fn web_write_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    SAVED_FILES.with(|f| f.borrow_mut().insert(path.to_string(), bytes.to_vec()));
    Ok(())
}

fn object_error(name: &str, what: &str, e: &str) {
    web_sys::console::warn_1(&JsValue::from_str(&format!("[rapidr] {name}.{what}: {e}")));
}

// ---------------------------------------------------------------------------
// Property access
// ---------------------------------------------------------------------------

/// Update the property in the component store only (no DOM side-effects).
/// Used by internal tab switching, etc.
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

pub fn rp_comp_set_prop_only(name: &str, prop: &str, val: Value) {
    let uname = name.to_uppercase();
    let lprop = prop.to_lowercase();
    COMPONENTS.with(|c| {
        if let Some(comp) = c.borrow_mut().get_mut(&uname) {
            comp.properties.insert(lprop, val);
        }
    });
}

/// Read the stored property value directly, bypassing live-DOM lookups.
pub fn rp_comp_get_stored(name: &str, prop: &str) -> Value {
    let uname = name.to_uppercase();
    let lprop = prop.to_lowercase();
    COMPONENTS.with(|c| {
        c.borrow()
            .get(&uname)
            .and_then(|comp| comp.properties.get(&lprop).cloned())
            .unwrap_or_else(v_null)
    })
}

pub fn rp_comp_set(name: &str, prop: &str, val: Value) {
    let uname = name.to_uppercase();
    let lprop = prop.to_lowercase();
    // Screen, Application, Clipboard, Mouse (globals_web.rs).
    if crate::globals_web::set(name, &lprop, &val) {
        return;
    }
    // A QFORMMDI's ChildMax, ChildCaption, ChildState, … (mdi_web.rs).
    if rapidr_value::mdi::is_mdi(name) && crate::mdi_web::set(name, &lprop, &val) {
        return;
    }
    // A form's size before (it paints again only when it changes).
    let form_size_before = (matches!(lprop.as_str(), "width" | "height") && rp_comp_type(name) == "RFORM").then(|| rp_comp_get_stored(name, &lprop).to_i64());
    // A canvas's size before (it paints again only when it changes).
    let canvas_size_before = (matches!(lprop.as_str(), "width" | "height") && rapidr_value::objects::is_canvas(&uname)).then(|| rp_comp_get_stored(name, &lprop).to_i64());
    // ClientWidth / ClientHeight: a form's inside (layout_web::form_client);
    // for other components, their whole size.
    if matches!(lprop.as_str(), "clientwidth" | "clientheight") {
        let width = lprop == "clientwidth";
        let (prop, v) = if rp_comp_type(&uname) == "RFORM" {
            let (cw, ch) = crate::layout_web::form_client(&uname);
            let (cw, ch) = if width { (val.to_i64(), ch) } else { (cw, val.to_i64()) };
            let (w, h) = crate::layout_web::form_outer(&uname, cw, ch);
            if width { ("width", w) } else { ("height", h) }
        } else {
            (if width { "width" } else { "height" }, val.to_i64())
        };
        return rp_comp_set(name, prop, v_int(v));
    }

    // QFONT, QMEMORYSTREAM, QBITMAP, QIMAGELIST (shared with the desktop runtime).
    let before_dir = if rapidr_value::objects::is_dirtree(name) { rp_comp_get_stored_dir(name) } else { String::new() };
    if let Some(result) = rapidr_value::objects::set(name, &lprop, &val) {
        let picture = rapidr_value::objects::is_picture(name);
        match result {
            // `Image.BMP = "photo.png"`: not a BMP; the browser shows it.
            Err(_) if picture && lprop == "bmp" => show_image_file(&uname, &val.to_string_val()),
            Err(e) => object_error(name, prop, &e),
            Ok(()) => {}
        }
        if picture {
            picture_changed(&uname);
        }
        if rapidr_value::objects::is_canvas(name) {
            gui_web::render_canvas(&uname);
        }
        // A list box that's owner-drawn or in columns now (its Style,
        // Columns): the element changes.
        if matches!(lprop.as_str(), "style" | "columns") && rapidr_value::objects::with_list(name, |l| l.custom_drawn()).unwrap_or(false) {
            gui_web::convert_to_owner_list(&uname);
        }
        // A QFILELISTBOX's directory changed: OnChange.
        if lprop == "directory" && rapidr_value::objects::is_file_list(name) {
            rp_fire_event(&uname, "onchange");
        }
        // A QDIRTREE: shown again; its directory changed: OnChange.
        if rapidr_value::objects::is_dirtree(name) {
            gui_web::render_dirtree(&uname);
            if matches!(lprop.as_str(), "directory" | "initialdir") && before_dir != rp_comp_get_stored_dir(name) {
                rp_fire_event(&uname, "onchange");
            }
        }
        if rapidr_value::objects::is_tree(name) {
            gui_web::render_tree(&uname);
        } else if rapidr_value::objects::is_listview(name) {
            gui_web::render_listview(&uname);
        } else if rapidr_value::objects::is_grid(name) {
            gui_web::render_grid(&uname);
        } else if rapidr_value::objects::is_list(name) {
            gui_web::render_list(&uname);
        }
        return;
    }
    // `Label.Font = Font` (a QFONT): copy the font's settings.
    if lprop == "font" {
        if let Some(props) = rapidr_value::objects::font_properties(&val.to_string_val()) {
            for (flat, v) in props {
                rp_comp_set(name, flat, v);
            }
            return;
        }
    }

    // `Form.Font.Size = 12` and `FontSize = 12` are one property (as on the
    // desktop): the flat name is what drawing and the DOM read.
    for (dotted, flat) in [("font.name", "fontname"), ("font.size", "fontsize"), ("font.bold", "fontbold"), ("font.italic", "fontitalic"), ("font.color", "fontcolor")] {
        if lprop == dotted {
            rp_comp_set(name, flat, val.clone());
        } else if lprop == flat {
            COMPONENTS.with(|c| {
                if let Some(comp) = c.borrow_mut().get_mut(&uname) {
                    comp.properties.insert(dotted.to_string(), val.clone());
                }
            });
        }
    }

    // Handle timer interval/enabled specially
    COMPONENTS.with(|c| {
        let mut comps = c.borrow_mut();
        // An unknown name is a property bag (a DIM'd QRECT or TYPE
        // variable), as on the desktop: its fields are kept.
        if !comps.contains_key(&uname) {
            comps.insert(uname.clone(), RpComponent { type_name: "RUDT".into(), properties: HashMap::new(), creation_order: 0 });
        }
        if let Some(comp) = comps.get_mut(&uname) {
            comp.properties.insert(lprop.clone(), val.clone());

            if comp.type_name == "RTIMER" {
                if lprop == "enabled" || lprop == "interval" {
                    drop(comps);
                    update_timer(&uname);
                    return;
                }
            }
        }
    });

    if lprop == "datasource" || lprop == "datafield" {
        crate::gui_web::setup_data_binding(&uname);
    }

    // A combo box that's owner-drawn now (its Style, which the list model
    // leaves stored here too): the element changes.
    if lprop == "style" && rapidr_value::objects::with_list(name, |l| l.combo && l.owner_drawn()).unwrap_or(false) {
        gui_web::convert_to_owner_combo(&uname);
    }

    // Handle parent re-parenting
    if lprop == "parent" {
        gui_web::gui_web_set_parent(&uname, &val.to_string_val().to_uppercase());
        crate::layout_web::after_set(&uname, &lprop);
        return;
    }

    // Handle data-science / database component property sets
    let comp_type = rp_comp_type(&uname);
    match comp_type.as_str() {
        "RNUM" => {
            crate::datascience_web::num_set_prop(&uname, &lprop, &val);
            return;
        }
        "RPLOT" => {
            crate::datascience_web::plot_set_prop(&uname, &lprop, &val);
            // Also pass through for visual properties (left/top/width/height)
            if matches!(lprop.as_str(), "title" | "xlabel" | "ylabel" | "grid" | "dpi") {
                return;
            }
        }
        "RSQLITE" | "RDATAFRAME" | "RMYSQL" => {
            // These use the generic property store only (already inserted above)
            return;
        }
        _ => {}
    }

    // A status bar redraws its panels / simple text.
    if comp_type == "RSTATUSBAR" && (lprop.starts_with("panel") || lprop.starts_with("simple")) {
        gui_web::render_statusbar(&uname);
        return;
    }

    // A panel's bevels drawn again.
    if rapidr_value::objects::bevel::default(&lprop).is_some() && comp_type == "RPANEL" {
        gui_web::render_panel_bevels(&uname);
        return;
    }
    // Pass to GUI layer for DOM update
    gui_web::gui_web_set_prop(&uname, &lprop, &val);
    // Align (layout_web).
    crate::layout_web::after_set(&uname, &lprop);
    // A QCANVAS's new size (its surface follows).
    if matches!(lprop.as_str(), "width" | "height") && rapidr_value::objects::is_canvas(&uname) {
        gui_web::render_canvas(&uname);
        if !rapidr_value::objects::is_form_surface(&uname) && canvas_size_before != Some(rp_comp_get_stored(name, &lprop).to_i64()) {
            rp_fire_event(&uname, "onpaint");
        }
    }
    // A form's new size: it paints again.
    let form_size_changed = form_size_before.is_some_and(|before| before != rp_comp_get_stored(name, &lprop).to_i64()) && !crate::layout_web::is_quiet();
    if form_size_changed {
        // (an MDI form's maximized children follow)
        if rapidr_value::mdi::is_mdi(&uname) {
            crate::mdi_web::resized(&uname);
        }
        rp_fire_event(&uname, "onpaint");
    }
    // A child window's frame shows its title and whether it's active.
    if matches!(lprop.as_str(), "caption" | "active" | "childstate") && rp_comp_type(&uname) == "RMDICHILD" {
        gui_web::mdi_frame_update(&uname);
    }
    // A QIMAGE's AutoSize / Stretch / Center.
    if matches!(lprop.as_str(), "autosize" | "stretch" | "center") && rapidr_value::objects::is_picture(&uname) {
        picture_changed(&uname);
    }
    // `CoolBtn.Down = True`: the others of its group come up.
    if lprop == "down" {
        gui_web::toggle_down_set(&uname);
    }
}

pub fn rp_sync_bound_widgets(db_name: &str, field_vals: &HashMap<String, String>, has_row: bool) {
    let uname = db_name.to_uppercase();
    let mut bounds = Vec::new();
    COMPONENTS.with(|c| {
        for (comp_name, comp) in c.borrow().iter() {
            let ds = comp.properties.get("datasource").map(|v| v.to_string_val().to_uppercase());
            let df = comp.properties.get("datafield").map(|v| v.to_string_val().to_uppercase());
            if let (Some(ds_val), Some(df_val)) = (ds, df) {
                if ds_val == uname {
                    let val = if has_row {
                        field_vals.get(&df_val).cloned().unwrap_or_default()
                    } else {
                        String::new()
                    };
                    bounds.push((comp_name.clone(), val));
                }
            }
        }
    });
    
    for (comp_name, val) in bounds {
        let comp_type = rp_comp_type(&comp_name);
        let prop_name = match comp_type.as_str() {
            "RCHECKBOX" | "RRADIOBUTTON" => "checked",
            "RCOMBOBOX" | "RLISTBOX" => "text",
            _ => "text",
        };
        
        if prop_name == "checked" {
            let bval = val == "1" || val.to_lowercase() == "true";
            rp_comp_set(&comp_name, prop_name, crate::value::v_bool(bval));
        } else {
            rp_comp_set(&comp_name, prop_name, v_str(&val));
        }
    }
}

pub fn rp_comp_get(name: &str, prop: &str) -> Value {
    let uname = name.to_uppercase();
    let lprop = prop.to_lowercase();
    // Screen, Application, Clipboard, Mouse (globals_web.rs).
    if let Some(v) = crate::globals_web::get(name, &lprop) {
        return v;
    }
    // A form's inside (its frame and main menu excluded); other components
    // have no frame inside their size.
    if matches!(lprop.as_str(), "clientwidth" | "clientheight") {
        let (w, h) = if rp_comp_type(&uname) == "RFORM" {
            crate::layout_web::form_client(&uname)
        } else {
            (rp_comp_get(name, "width").to_i64(), rp_comp_get(name, "height").to_i64())
        };
        return v_int(if lprop == "clientwidth" { w } else { h });
    }
    // A component's Handle (rapidr_value::handles).
    if lprop == "handle" && COMPONENTS.with(|c| c.borrow().contains_key(&uname)) {
        return v_int(rapidr_value::handles::handle_of(name));
    }
    // A QFORMMDI's ChildCount, ChildCaption, … (mdi_web.rs).
    if let Some(v) = rapidr_value::mdi::get(name, &lprop) {
        return v;
    }
    if let Some(v) = rapidr_value::objects::get(name, &lprop) {
        return v;
    }

    // Check data-science / database component properties
    // If RDOM, query live DOM first
    let comp_type = rp_comp_type(&uname);
    if comp_type == "RDOM" {
        let live = gui_web::gui_web_get_prop(&uname, &lprop);
        if !matches!(live, Value::Null) {
            return live;
        }
    }

    match comp_type.as_str() {
        "RNUM" => {
            let v = crate::datascience_web::num_get_prop(&uname, &lprop);
            if !matches!(v, Value::Null) { return v; }
        }
        "RDATAFRAME" => {
            let v = crate::datascience_web::dataframe_get_prop(&uname, &lprop);
            if !matches!(v, Value::Null) { return v; }
        }
        "RPLOT" => {
            let v = crate::datascience_web::plot_get_prop(&uname, &lprop);
            if !matches!(v, Value::Null) { return v; }
        }
        "RSQLITE" => {
            // Uses generic property store — fall through
        }
        _ => {}
    }

    // Check stored properties first (for non-DOM properties)
    let stored = COMPONENTS.with(|c| {
        c.borrow()
            .get(&uname)
            .and_then(|comp| comp.properties.get(&lprop).cloned())
    });

    // Positions are what the program (or layout) set; sizes too while the
    // element isn't rendered (a form not shown yet, a hidden control).
    if matches!(lprop.as_str(), "left" | "top") {
        if let Some(v) = stored.clone() {
            return v;
        }
    }
    // A QRECT's Right / Bottom (Left / Top below): 0 until set.
    if matches!(lprop.as_str(), "right" | "bottom") {
        return stored.unwrap_or(v_int(0));
    }
    if matches!(lprop.as_str(), "width" | "height") {
        let live = gui_web::gui_web_get_prop(&uname, &lprop);
        return match stored {
            Some(v) if live.to_i64() == 0 => v,
            _ => live,
        };
    }

    // For visual properties, prefer live DOM values
    match lprop.as_str() {
        "caption" | "text" | "left" | "top" | "width" | "height" | "visible" | "enabled"
        | "checked" | "value" | "listindex" | "itemindex" | "listcount" | "position"
        | "min" | "max" | "selstart" | "seltext" | "innerhtml" | "innertext" | "tagname"
        | "volume" | "currenttime" | "duration" | "playing" | "paused"
        | "url" | "html" | "sandbox" | "src" | "picture" | "controls" | "loop" | "autoplay"
        | "poster" | "storagetype" | "latitude" | "longitude" | "accuracy" | "title" | "body"
        | "route" | "hash" | "cssstyle" | "cssclass" => {
            // A component with no element (a QTIMER's Enabled): what was stored.
            match gui_web::gui_web_get_prop(&uname, &lprop) {
                Value::Null if matches!(lprop.as_str(), "left" | "top") => stored.unwrap_or(v_int(0)),
                Value::Null => stored.unwrap_or_else(v_null),
                live => live,
            }
        }
        // (a panel's bevels: RapidQ's defaults until set)
        _ => stored
            .or_else(|| (rp_comp_type(&uname) == "RPANEL").then(|| rapidr_value::objects::bevel::default(&lprop).map(v_int)).flatten())
            .unwrap_or_else(v_null),
    }
}

pub fn rp_comp_type(name: &str) -> String {
    let uname = name.to_uppercase();
    COMPONENTS.with(|c| {
        c.borrow()
            .get(&uname)
            .map(|comp| comp.type_name.clone())
            .unwrap_or_default()
    })
}

// ---------------------------------------------------------------------------
// Component methods
// ---------------------------------------------------------------------------

pub fn rp_comp_method(name: &str, method: &str, args: &[Value]) -> Value {
    let uname = name.to_uppercase();
    let lmethod = method.to_lowercase();
    // Screen, Application, Clipboard, Mouse (globals_web.rs).
    if let Some(v) = crate::globals_web::call(name, &lmethod, args) {
        return v;
    }
    // A QFORMMDI's AddChild, CascadeChild, … (mdi_web.rs).
    if rapidr_value::mdi::is_mdi(name) {
        if let Some(v) = crate::mdi_web::method(name, &lmethod, args) {
            return v;
        }
    }

    // Indexed sub-objects (`SB.Panel(0).Width = 100` → method
    // `panel.width=` with (0, 100); reading → `panel.width` with (0)): kept
    // as the component's properties `panel(0).width` unless the component
    // implements them.
    // A QFORM gets its own drawing surface the first time it's drawn on.
    if rp_comp_type(name) == "RFORM"
        && rapidr_value::objects::is_drawing_method(&lmethod)
        && !(lmethod == "paint" && args.len() < 3)
        && !rapidr_value::objects::is_form_surface(name)
    {
        rapidr_value::objects::create_form_surface(name, rapidr_value::objects::form_color(&rp_comp_get_stored(name, "color")));
    }
    if let Some(result) = rapidr_value::objects::call(name, &lmethod, args, &|id, p| rp_comp_get(id, p)) {
        if rapidr_value::objects::is_picture(name) {
            // `Image.LoadFromFile "photo.png"`: not a BMP; the browser shows it.
            if result.is_err() && matches!(lmethod.as_str(), "loadfromfile" | "load") {
                show_image_file(&uname, &args.first().map(|v| v.to_string_val()).unwrap_or_default());
                return v_null();
            }
            picture_changed(&uname);
        }
        if rapidr_value::objects::is_canvas(name) {
            gui_web::render_canvas(&uname);
        }
        if rapidr_value::objects::is_dirtree(name) {
            gui_web::render_dirtree(&uname);
        }
        if rapidr_value::objects::is_tree(name) {
            gui_web::render_tree(&uname);
        } else if rapidr_value::objects::is_listview(name) {
            gui_web::render_listview(&uname);
        } else if rapidr_value::objects::is_grid(name) {
            gui_web::render_grid(&uname);
        } else if rapidr_value::objects::is_list(name) {
            gui_web::render_list(&uname);
        }
        return result.unwrap_or_else(|e| {
            object_error(name, method, &e);
            v_null()
        });
    }
    if let Some(v) = indexed_sub_object(name, &lmethod, args) {
        return v;
    }
    if rp_comp_type(&uname) == "RSTATUSBAR" && lmethod == "addpanels" {
        // `AddPanels "Ready", "Line 1"`: panels `panel(i).caption`, as on
        // the desktop (rapidr-runtime-core's `statusbar_method`).
        let mut n = rp_comp_get_stored(name, "panelcount").to_i64().max(0);
        for a in args {
            rp_comp_set(name, &format!("panel({n}).caption"), v_str(&a.to_string_val()));
            n += 1;
        }
        rp_comp_set(name, "panelcount", v_int(n));
        return v_null();
    }
    let comp_type = rp_comp_type(&uname);
    if comp_type.is_empty() {
        web_sys::console::warn_1(&JsValue::from_str(&format!(
            "[WARN] Component '{}' not found",
            name
        )));
        return v_null();
    }

    // JSON special handling
    if comp_type == "RJSON" {
        return json_web_method(&uname, &lmethod, args);
    }

    // FileStream — in-browser file pick / download bridge
    if comp_type == "RFILESTREAM" {
        return filestream_web_method(&uname, &lmethod, args);
    }

    // Native browser dialogs (synchronous via prompt() / async file input)
    if matches!(
        comp_type.as_str(),
        "ROPENDIALOG" | "RSAVEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG"
    ) {
        return dialog_web_method(&uname, &comp_type, &lmethod, args);
    }

    // HTTP special handling
    if comp_type == "RHTTP" {
        return crate::network_web::http_method(&uname, &lmethod, args);
    }

    // WebSocket special handling
    if comp_type == "RSOCKET" || comp_type == "RSERVERSOCKET" {
        return crate::network_web::websocket_method(&uname, &lmethod, args);
    }

    // Data-science special handling
    if comp_type == "RNUM" {
        return crate::datascience_web::num_method(&uname, &lmethod, args);
    }
    if comp_type == "RDATAFRAME" {
        return crate::datascience_web::dataframe_method(&uname, &lmethod, args);
    }
    if comp_type == "RPLOT" {
        return crate::datascience_web::plot_method(&uname, &lmethod, args);
    }

    // Database special handling
    if comp_type == "RSQLITE" {
        return crate::database_web::sqlite_method(&uname, &lmethod, args);
    }
    if comp_type == "RMYSQL" {
        return crate::database_web::mysql_method(&uname, &lmethod, args);
    }

    // Delegate to GUI layer
    gui_web::gui_web_method(&uname, &comp_type, &lmethod, args)
}

// ---------------------------------------------------------------------------
// RJSON web methods — uses js_sys::JSON for parsing/stringifying
// ---------------------------------------------------------------------------

thread_local! {
    static JSON_WEB_STORES: std::cell::RefCell<std::collections::HashMap<String, String>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

fn json_web_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "parse" => {
            let text = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            // Validate JSON via js_sys
            let js_str = wasm_bindgen::JsValue::from_str(&text);
            match js_sys::JSON::parse(&text) {
                Ok(_) => {
                    rp_comp_set(name, "text", v_str(&text));
                    JSON_WEB_STORES.with(|s| s.borrow_mut().insert(name_lower, text));
                    v_int(1)
                }
                Err(_) => {
                    web_sys::console::warn_1(&js_str);
                    v_int(0)
                }
            }
        }
        "stringify" | "prettify" => {
            JSON_WEB_STORES.with(|s| {
                if let Some(text) = s.borrow().get(&name_lower) {
                    if method == "prettify" {
                        // Parse to JsValue then stringify with indent
                        if let Ok(val) = js_sys::JSON::parse(text) {
                            let indent = wasm_bindgen::JsValue::from_f64(2.0);
                            if let Ok(pretty) = js_sys::JSON::stringify_with_replacer_and_space(
                                &val,
                                &wasm_bindgen::JsValue::NULL,
                                &indent,
                            ) {
                                return v_str(&pretty.as_string().unwrap_or_default());
                            }
                        }
                    }
                    v_str(text)
                } else {
                    v_str("{}")
                }
            })
        }
        "get" => {
            let key = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            JSON_WEB_STORES.with(|s| {
                if let Some(text) = s.borrow().get(&name_lower) {
                    if let Ok(root) = js_sys::JSON::parse(text) {
                        let result = json_web_get_path(&root, &key);
                        return result;
                    }
                }
                v_str("")
            })
        }
        "set" => {
            let key = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let val = args.get(1).cloned().unwrap_or(v_null());
            JSON_WEB_STORES.with(|s| {
                let mut store = s.borrow_mut();
                let text = store.entry(name_lower.clone()).or_insert_with(|| "{}".to_string());
                if let Ok(root) = js_sys::JSON::parse(text) {
                    json_web_set_path(&root, &key, &val);
                    if let Ok(updated) = js_sys::JSON::stringify(&root) {
                        *text = updated.as_string().unwrap_or_default();
                    }
                }
            });
            v_null()
        }
        "has" => {
            let key = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            JSON_WEB_STORES.with(|s| {
                if let Some(text) = s.borrow().get(&name_lower) {
                    if let Ok(root) = js_sys::JSON::parse(text) {
                        let js_key = wasm_bindgen::JsValue::from_str(&key);
                        let has = js_sys::Reflect::has(&root, &js_key).unwrap_or(false);
                        return v_int(if has { 1 } else { 0 });
                    }
                }
                v_int(0)
            })
        }
        "remove" => {
            let key = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            JSON_WEB_STORES.with(|s| {
                let mut store = s.borrow_mut();
                if let Some(text) = store.get_mut(&name_lower) {
                    if let Ok(root) = js_sys::JSON::parse(text) {
                        let js_key = wasm_bindgen::JsValue::from_str(&key);
                        if let Some(obj) = root.dyn_ref::<js_sys::Object>() {
                            let _ = js_sys::Reflect::delete_property(obj, &js_key);
                        }
                        if let Ok(updated) = js_sys::JSON::stringify(&root) {
                            *text = updated.as_string().unwrap_or_default();
                        }
                    }
                }
            });
            v_null()
        }
        "count" => {
            JSON_WEB_STORES.with(|s| {
                if let Some(text) = s.borrow().get(&name_lower) {
                    if let Ok(root) = js_sys::JSON::parse(text) {
                        if let Some(obj) = root.dyn_ref::<js_sys::Object>() {
                            let keys = js_sys::Object::keys(obj);
                            return v_int(keys.length() as i64);
                        }
                        if let Some(arr) = root.dyn_ref::<js_sys::Array>() {
                            return v_int(arr.length() as i64);
                        }
                    }
                }
                v_int(0)
            })
        }
        "keys" => {
            JSON_WEB_STORES.with(|s| {
                if let Some(text) = s.borrow().get(&name_lower) {
                    if let Ok(root) = js_sys::JSON::parse(text) {
                        if let Some(obj) = root.dyn_ref::<js_sys::Object>() {
                            let keys = js_sys::Object::keys(obj);
                            let mut result = Vec::new();
                            for i in 0..keys.length() {
                                if let Some(k) = keys.get(i).as_string() {
                                    result.push(k);
                                }
                            }
                            return v_str(&result.join(","));
                        }
                    }
                }
                v_str("")
            })
        }
        "loadfile" | "savefile" => {
            // File operations not available in web context
            web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
                &format!("RJSON.{}() is not available in web context", method)
            ));
            v_int(0)
        }
        "clear" => {
            JSON_WEB_STORES.with(|s| {
                s.borrow_mut().insert(name_lower, "{}".to_string());
            });
            v_null()
        }
        _ => {
            web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
                &format!("RJSON.{}() not implemented", method)
            ));
            v_null()
        }
    }
}

fn json_web_get_path(root: &wasm_bindgen::JsValue, path: &str) -> Value {
    let mut current = root.clone();
    for part in path.split('.') {
        if part.is_empty() { continue; }
        let js_key = wasm_bindgen::JsValue::from_str(part);
        match js_sys::Reflect::get(&current, &js_key) {
            Ok(val) => {
                if val.is_undefined() || val.is_null() { return v_str(""); }
                current = val;
            }
            Err(_) => return v_str(""),
        }
    }
    if let Some(s) = current.as_string() {
        v_str(&s)
    } else if let Some(b) = current.as_bool() {
        v_int(if b { 1 } else { 0 })
    } else if let Some(f) = current.as_f64() {
        if f == f.floor() && f.abs() < i64::MAX as f64 {
            v_int(f as i64)
        } else {
            use crate::value::v_dbl;
            v_dbl(f)
        }
    } else if let Ok(s) = js_sys::JSON::stringify(&current) {
        v_str(&s.as_string().unwrap_or_default())
    } else {
        v_str("")
    }
}

fn json_web_set_path(root: &wasm_bindgen::JsValue, path: &str, val: &Value) {
    let parts: Vec<&str> = path.split('.').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() { return; }
    let mut current = root.clone();
    for part in &parts[..parts.len()-1] {
        let js_key = wasm_bindgen::JsValue::from_str(part);
        match js_sys::Reflect::get(&current, &js_key) {
            Ok(val) if !val.is_undefined() && !val.is_null() => {
                current = val;
            }
            _ => {
                let new_obj = js_sys::Object::new();
                let _ = js_sys::Reflect::set(&current, &js_key, &new_obj.into());
                if let Ok(v) = js_sys::Reflect::get(&current, &js_key) {
                    current = v;
                } else { return; }
            }
        }
    }
    let last_key = wasm_bindgen::JsValue::from_str(parts.last().unwrap());
    let js_val = wasm_bindgen::JsValue::from_str(&val.to_string_val());
    let _ = js_sys::Reflect::set(&current, &last_key, &js_val);
}

// ---------------------------------------------------------------------------
// RFILESTREAM (web): the shared stream (rapidr_value::objects, the same as
// the desktop) handles Open/Read*/Write*/Seek/…, with files read from the
// page and saved for the session (web_read_file / web_write_file). These
// page-only extras remain: `Download` saves the stream's content as a file,
// `PickFile` loads a file the user picks, `LoadFromUrl` fetches one; both
// then fire the component's `onload` event.
// ---------------------------------------------------------------------------

fn fs_get_text(name: &str) -> String {
    rapidr_value::objects::stream_bytes(name).map(|b| rapidr_value::objects::codec::bytes_to_string(&b)).unwrap_or_default()
}

fn fs_set_text(name: &str, text: &str) {
    rapidr_value::objects::stream_load(name, rapidr_value::objects::codec::string_to_bytes(text));
}

fn filestream_web_method(name: &str, method: &str, args: &[Value]) -> Value {
    match method {
        "download" => {
            let filename = COMPONENTS
                .with(|c| {
                    c.borrow()
                        .get(name)
                        .and_then(|comp| comp.properties.get("filename").map(|v| v.to_string_val()))
                })
                .unwrap_or_else(|| "untitled.txt".to_string());
            let mime = COMPONENTS
                .with(|c| {
                    c.borrow()
                        .get(name)
                        .and_then(|comp| comp.properties.get("mimetype").map(|v| v.to_string_val()))
                })
                .unwrap_or_else(|| "text/plain".to_string());
            let text = fs_get_text(name);
            trigger_download(&filename, &mime, &text);
            v_int(1)
        }
        "pickfile" => {
            // Open a hidden <input type="file"> and read the chosen file's text.
            // Optional first arg = accept filter (e.g. ".rr,.txt").
            let accept = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let doc = crate::gui_web::document();
            let input_el = match doc.create_element("input") {
                Ok(el) => el,
                Err(_) => return v_int(0),
            };
            let input = match input_el.dyn_into::<web_sys::HtmlInputElement>() {
                Ok(i) => i,
                Err(_) => return v_int(0),
            };
            input.set_type("file");
            if !accept.is_empty() {
                let _ = input.set_attribute("accept", &accept);
            }
            let _ = input.style().set_property("display", "none");

            let name_for_cb = name.to_string();
            let input_clone = input.clone();
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_e: web_sys::Event| {
                let files = match input_clone.files() {
                    Some(f) => f,
                    None => return,
                };
                let file = match files.item(0) {
                    Some(f) => f,
                    None => return,
                };
                let fname = file.name();
                rp_comp_set_prop_only(&name_for_cb, "filename", v_str(&fname));

                let reader = match web_sys::FileReader::new() {
                    Ok(r) => r,
                    Err(_) => return,
                };
                let reader_clone = reader.clone();
                let name_for_load = name_for_cb.clone();
                let onload = Closure::<dyn FnMut(web_sys::ProgressEvent)>::new(
                    move |_ev: web_sys::ProgressEvent| {
                        if let Ok(result) = reader_clone.result() {
                            if let Some(text) = result.as_string() {
                                fs_set_text(&name_for_load, &text);
                                rp_fire_event(&name_for_load, "onload");
                            }
                        }
                    },
                );
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                let _ = reader.read_as_text(&file);
            });
            input.set_onchange(Some(cb.as_ref().unchecked_ref()));
            cb.forget();

            if let Some(body) = doc.body() {
                let _ = body.append_child(&input);
            }
            input.click();
            v_int(1)
        }
        "loadfromurl" => {
            // Async fetch — fires `onload` with text in `text` property when done.
            let url = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            if url.is_empty() {
                return v_int(0);
            }
            let name_for_cb = name.to_string();
            let promise = web_sys::window().unwrap().fetch_with_str(&url);
            let future = wasm_bindgen_futures::JsFuture::from(promise);
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(resp_val) = future.await {
                    if let Ok(resp) = resp_val.dyn_into::<web_sys::Response>() {
                        if let Ok(text_promise) = resp.text() {
                            if let Ok(text_val) =
                                wasm_bindgen_futures::JsFuture::from(text_promise).await
                            {
                                if let Some(text) = text_val.as_string() {
                                    fs_set_text(&name_for_cb, &text);
                                    rp_fire_event(&name_for_cb, "onload");
                                }
                            }
                        }
                    }
                }
            });
            v_int(1)
        }
        _ => {
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[WARN] RFILESTREAM.{}() not implemented on web",
                method
            )));
            v_null()
        }
    }
}

// ---------------------------------------------------------------------------
// Native browser dialogs for ROPENDIALOG / RSAVEDIALOG / RCOLORDIALOG /
// RFONTDIALOG. Browsers cannot open synchronous native file pickers, so
// `Execute` uses what's available: `prompt()` for filenames, a hidden
// `<input type="color">` for color, and a small inline form for fonts.
// File *content* loading should go through RFILESTREAM.PickFile().
// ---------------------------------------------------------------------------

fn dialog_web_method(name: &str, comp_type: &str, method: &str, args: &[Value]) -> Value {
    if method != "execute" {
        return v_null();
    }
    let _ = args;
    match comp_type {
        "RSAVEDIALOG" => {
            // Prompt for a filename; default = current `filename` prop.
            let cur = COMPONENTS.with(|c| {
                c.borrow()
                    .get(name)
                    .and_then(|comp| comp.properties.get("filename").map(|v| v.to_string_val()))
                    .unwrap_or_default()
            });
            if let Some(window) = web_sys::window() {
                if let Ok(Some(fname)) = window.prompt_with_message_and_default(
                    "Save as filename:",
                    &cur,
                ) {
                    if !fname.is_empty() {
                        rp_comp_set_prop_only(name, "filename", v_str(&fname));
                        return v_int(1);
                    }
                }
            }
            v_int(0)
        }
        "ROPENDIALOG" => {
            // Use a hidden file input to let the user pick a file.
            // We only capture its *name* into `filename` (sync). To actually
            // load the contents, use RFILESTREAM.PickFile() instead.
            let doc = crate::gui_web::document();
            let input_el = match doc.create_element("input") {
                Ok(el) => el,
                Err(_) => return v_int(0),
            };
            let input = match input_el.dyn_into::<web_sys::HtmlInputElement>() {
                Ok(i) => i,
                Err(_) => return v_int(0),
            };
            input.set_type("file");
            let _ = input.style().set_property("display", "none");
            let name_for_cb = name.to_string();
            let input_clone = input.clone();
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_e: web_sys::Event| {
                if let Some(files) = input_clone.files() {
                    if let Some(file) = files.item(0) {
                        rp_comp_set_prop_only(&name_for_cb, "filename", v_str(&file.name()));
                        rp_fire_event(&name_for_cb, "onclose");
                    }
                }
            });
            input.set_onchange(Some(cb.as_ref().unchecked_ref()));
            cb.forget();
            if let Some(body) = doc.body() {
                let _ = body.append_child(&input);
            }
            input.click();
            v_int(1)
        }
        "RCOLORDIALOG" => {
            let cur = COMPONENTS
                .with(|c| {
                    c.borrow()
                        .get(name)
                        .and_then(|comp| comp.properties.get("color").map(|v| v.to_i64()))
                })
                .unwrap_or(0xFFFFFF);
            let r = (cur & 0xFF) as i64;
            let g = ((cur >> 8) & 0xFF) as i64;
            let b = ((cur >> 16) & 0xFF) as i64;
            let default_hex = format!("#{:02x}{:02x}{:02x}", r, g, b);
            let doc = crate::gui_web::document();
            let input = match doc
                .create_element("input")
                .ok()
                .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok())
            {
                Some(i) => i,
                None => return v_int(0),
            };
            input.set_type("color");
            input.set_value(&default_hex);
            let _ = input.style().set_property("display", "none");
            let name_for_cb = name.to_string();
            let input_clone = input.clone();
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_e: web_sys::Event| {
                let hex = input_clone.value();
                // hex = "#rrggbb" — parse to 0xBBGGRR (RapidR uses BGR ordering).
                if hex.len() == 7 && hex.starts_with('#') {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        i64::from_str_radix(&hex[1..3], 16),
                        i64::from_str_radix(&hex[3..5], 16),
                        i64::from_str_radix(&hex[5..7], 16),
                    ) {
                        let bgr = (b << 16) | (g << 8) | r;
                        rp_comp_set_prop_only(&name_for_cb, "color", v_int(bgr));
                        rp_fire_event(&name_for_cb, "onchange");
                    }
                }
            });
            input.set_onchange(Some(cb.as_ref().unchecked_ref()));
            cb.forget();
            if let Some(body) = doc.body() {
                let _ = body.append_child(&input);
            }
            input.click();
            v_int(1)
        }
        "RFONTDIALOG" => {
            // Browsers have no font picker. Use prompt() for name + size.
            let (cur_name, cur_size) = COMPONENTS.with(|c| {
                let comps = c.borrow();
                let comp = comps.get(name);
                (
                    comp.and_then(|c| c.properties.get("fontname").map(|v| v.to_string_val()))
                        .unwrap_or_else(|| "Segoe UI".to_string()),
                    comp.and_then(|c| c.properties.get("fontsize").map(|v| v.to_i64()))
                        .unwrap_or(12),
                )
            });
            if let Some(window) = web_sys::window() {
                if let Ok(Some(fname)) =
                    window.prompt_with_message_and_default("Font name:", &cur_name)
                {
                    if let Ok(Some(fsize)) = window.prompt_with_message_and_default(
                        "Font size (pt):",
                        &cur_size.to_string(),
                    ) {
                        rp_comp_set_prop_only(name, "fontname", v_str(&fname));
                        if let Ok(n) = fsize.parse::<i64>() {
                            rp_comp_set_prop_only(name, "fontsize", v_int(n));
                        }
                        return v_int(1);
                    }
                }
            }
            v_int(0)
        }
        _ => v_null(),
    }
}

// ---------------------------------------------------------------------------
// Event binding
// ---------------------------------------------------------------------------

/// Whether the program handles `name`'s `event` (the runtime then does the
/// work to fire it, e.g. OnDrawCell for every cell).
pub fn rp_has_handler(name: &str, event: &str) -> bool {
    EVENT_HANDLERS.with(|eh| eh.borrow().contains_key(&(name.to_uppercase(), event.to_lowercase())))
}

pub fn rp_bind_event(name: &str, event: &str, handler: fn()) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity0(handler));
    });

    bind_dom_event(&uname, &levent);
}

pub fn rp_bind_event_1(name: &str, event: &str, handler: fn(Value)) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity1(handler));
    });

    bind_dom_event(&uname, &levent);
}

pub fn rp_bind_event_2(name: &str, event: &str, handler: fn(Value, Value)) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity2(handler));
    });

    bind_dom_event(&uname, &levent);
}

pub fn rp_bind_event_3(name: &str, event: &str, handler: fn(Value, Value, Value)) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity3(handler));
    });

    bind_dom_event(&uname, &levent);
}

pub fn rp_bind_event_4(name: &str, event: &str, handler: fn(Value, Value, Value, Value)) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity4(handler));
    });

    bind_dom_event(&uname, &levent);
}

pub fn rp_bind_event_5(name: &str, event: &str, handler: fn(Value, Value, Value, Value, Value)) {
    let uname = name.to_uppercase();
    let levent = event.to_lowercase();

    EVENT_HANDLERS.with(|eh| {
        eh.borrow_mut()
            .insert((uname.clone(), levent.clone()), EventHandler::Arity5(handler));
    });

    bind_dom_event(&uname, &levent);
}

// ---------------------------------------------------------------------------
// Event firing
// ---------------------------------------------------------------------------

thread_local! {
    /// The program ran END: nothing more of it runs.
    static ENDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// END in the browser, as the desktop's (which exits): what was written to
/// files is kept, the forms close (without OnClose), timers stop and no
/// event reaches the program any more.
pub fn end_program() {
    if ENDED.with(|e| e.replace(true)) {
        return;
    }
    rapidr_value::basic_files::close_all();
    TIMER_HANDLES.with(|th| {
        if let Some(window) = web_sys::window() {
            for (_, handle) in th.borrow_mut().drain() {
                window.clear_interval_with_handle(handle);
            }
        }
    });
    let forms: Vec<String> = COMPONENTS.with(|c| c.borrow().iter().filter(|(_, comp)| comp.type_name.eq_ignore_ascii_case("RFORM")).map(|(n, _)| n.clone()).collect());
    for form in forms {
        gui_web::hide_form(&form);
    }
    web_sys::console::log_1(&JsValue::from_str("[RapidR] Program ended."));
}

/// Whether the program ran END.
pub fn program_ended() -> bool {
    ENDED.with(|e| e.get())
}

/// Runs the handler bound to `name`'s `event` with the event's arguments
/// (the firing component is passed last, as `Sender`); returns them as a
/// compiled handler left them. The handler is copied out first, so it may
/// bind or fire other events.
fn fire(name: &str, event: &str, args: &[Value]) -> Vec<Value> {
    if program_ended() {
        return args.to_vec();
    }
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

/// Bind a bytecode EVENT handler to one instance (see [`EventHandler::IndirectThis`]).
pub fn rp_bind_event_indirect_this(name: &str, event: &str, handler_id: u32, this: Value) {
    bind_handler(name, event, EventHandler::IndirectThis(handler_id, this));
}

/// Bind a compiled closure (see [`EventHandler::Closure`]).
pub fn rp_bind_event_closure(name: &str, event: &str, f: std::rc::Rc<dyn Fn(&mut Vec<Value>)>) {
    bind_handler(name, event, EventHandler::Closure(f));
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

/// Bind a compiled handler of `n` parameters that writes them back (RapidQ's
/// event parameters are by reference: OnClose's `Action`, …).
pub fn rp_bind_event_out(name: &str, event: &str, n: usize, handler: fn(&mut [Value])) {
    bind_handler(name, event, EventHandler::Out(n, handler));
}

fn lookup_handler(name: &str, event: &str) -> Option<EventHandler> {
    EVENT_HANDLERS.with(|h| h.borrow().get(&(name.to_uppercase(), event.to_lowercase())).cloned())
}

fn bind_handler(name: &str, event: &str, handler: EventHandler) {
    let (uname, levent) = (name.to_uppercase(), event.to_lowercase());
    EVENT_HANDLERS.with(|h| {
        h.borrow_mut().insert((uname.clone(), levent.clone()), handler);
    });
    bind_dom_event(&uname, &levent);
}

pub fn rp_rebind_component_events(name: &str) {
    let uname = name.to_uppercase();
    let events: Vec<String> = EVENT_HANDLERS.with(|eh| {
        eh.borrow()
            .keys()
            .filter(|(comp, _)| comp == &uname)
            .map(|(_, ev)| ev.clone())
            .collect()
    });
    for ev in events {
        bind_dom_event(name, &ev);
    }
}

// ---------------------------------------------------------------------------
// DOM event binding — wire up browser events to fire RapidR events
// ---------------------------------------------------------------------------

/// A QIMAGE's picture changed: with AutoSize the control takes the
/// picture's size; the element shows it again (as on the desktop).
fn picture_changed(name: &str) {
    if rp_comp_get_stored(name, "autosize").to_bool() {
        if let Some(Some((w, h))) = rapidr_value::objects::with_picture(name, |b| {
            (!b.img.pixels.is_empty()).then_some((b.img.width as i64, b.img.height as i64))
        }) {
            if rp_comp_get_stored(name, "width").to_i64() != w {
                rp_comp_set(name, "width", v_int(w));
            }
            if rp_comp_get_stored(name, "height").to_i64() != h {
                rp_comp_set(name, "height", v_int(h));
            }
        }
    }
    crate::gui_web::render_picture(name);
}

/// A QIMAGE showing an image file other than a BMP (PNG, JPEG, …).
fn show_image_file(name: &str, path: &str) {
    let id = format!("rr-{}", name.to_lowercase());
    if let Some(img) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(&id))
        .and_then(|e| e.dyn_into::<web_sys::HtmlImageElement>().ok())
    {
        img.set_src(path);
    }
}

/// A QDIRTREE's Directory (to see whether a store changed it).
fn rp_comp_get_stored_dir(name: &str) -> String {
    rapidr_value::objects::get(name, "directory").map(|v| v.to_string_val()).unwrap_or_default()
}

fn bind_dom_event(name: &str, event: &str) {
    let id = format!("rr-{}", name.to_lowercase());
    let name_owned = name.to_string();
    let event_owned = event.to_string();

    // Timer events are handled specially — they don't need DOM binding
    if event == "ontimer" {
        update_timer(name);
        return;
    }

    // Router events — bind to window hashchange
    if event == "onroutechange" {
        let name_for_closure = name_owned.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            rp_fire_event(&name_for_closure, "onroutechange");
        });
        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback(
                "hashchange",
                closure.as_ref().unchecked_ref(),
            );
        }
        closure.forget();
        return;
    }

    // A QLISTVIEW's clicks go through its rows and header (gui_web's
    // `create_listview`), which set ItemIndex first.
    if matches!(event, "onclick" | "ondblclick" | "ondoubleclick" | "oncolumnclick") && rapidr_value::objects::is_listview(name) {
        return;
    }
    // A QTREEVIEW fires its clicks itself (gui_web's `create_treeview`).
    if matches!(event, "onclick" | "ondblclick" | "ondoubleclick" | "onchange") && rapidr_value::objects::is_tree(name) {
        return;
    }
    // A QDIRTREE fires OnChange itself (gui_web's `create_dirtree`).
    if event == "onchange" && rapidr_value::objects::is_dirtree(name) {
        return;
    }
    // A QIMAGE fires its mouse events itself (gui_web's `picture_mouse`).
    if matches!(event, "onclick" | "ondblclick" | "ondoubleclick" | "onmousedown" | "onmouseup" | "onmousemove") && rapidr_value::objects::is_picture(name) {
        return;
    }
    // A QSTRINGGRID fires its events itself (gui_web's `create_grid`).
    if matches!(event, "onclick" | "ondblclick" | "ondoubleclick" | "onchange" | "onselectcell" | "onsetedittext" | "onellipsisclick") && rapidr_value::objects::is_grid(name) {
        return;
    }

    let doc = web_sys::window().unwrap().document().unwrap();
    let el = match doc.get_element_by_id(&id) {
        Some(e) => e,
        None => return, // Virtual components (Timer, etc.) don't have DOM elements
    };

    let dom_event_name = match event {
        "onclick" => {
            if el.tag_name().to_uppercase() == "SELECT" {
                "change"
            } else {
                "click"
            }
        }
        "ondblclick" | "ondoubleclick" => "dblclick",
        "onchange" => "input",
        "onkeypress" | "onkeydown" => "keydown",
        "onkeyup" => "keyup",
        "onmousedown" => "mousedown",
        "onmouseup" => "mouseup",
        "onmousemove" => "mousemove",
        "onmouseover" | "onmouseenter" => "mouseenter",
        "onmouseout" | "onmouseleave" => "mouseleave",
        "onfocus" | "ongotfocus" => "focus",
        "onblur" | "onlostfocus" => "blur",
        "onscroll" => "scroll",
        "onload" => "load",
        "onresize" => "resize",
        "onplay" => "play",
        "onpause" => "pause",
        "onended" => "ended",
        "ontimeupdate" => "timeupdate",
        "oninput" => "input",
        "onclose" => "close",
        "onpermissionchange" => return, // handled differently
        _ => return,
    };

    // Create a JavaScript closure that fires the RapidR event
    let name_for_closure = name_owned.clone();
    let event_for_closure = event_owned.clone();

    // Key events (rapidr_value::input): OnKeyDown / OnKeyUp (Key, Shift)
    // and OnKeyPress (Key) for a key that types; they bubble from the
    // focused control to its form, as the desktop gives them to both.
    if dom_event_name == "keydown" || dom_event_name == "keyup" {
        let closure = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            use rapidr_value::input;
            let key = e.key();
            let vk = match e.key_code() {
                0 => input::vk_of_key(&key, &e.code()).unwrap_or(0),
                k => k as i64,
            };
            let shift = input::shift_state(e.shift_key(), e.ctrl_key(), e.alt_key());
            if event_for_closure == "onkeypress" {
                if let Some(k) = input::press_code(vk, &key) {
                    rp_fire_event_1(&name_for_closure, "onkeypress", v_int(k));
                }
            } else {
                rp_fire_event_2(&name_for_closure, &event_for_closure, v_int(vk), v_int(shift));
            }
        });
        let _ = el.add_event_listener_with_callback(dom_event_name, closure.as_ref().unchecked_ref());
        closure.forget();
        return;
    }

    // Mouse events (rapidr_value::input): (Button, X, Y, Shift), OnMouseMove
    // (X, Y, Shift), X and Y in the component (a form's below its title
    // bar); only the innermost component gets them, as on the desktop.
    if dom_event_name == "mousemove"
        || dom_event_name == "mousedown"
        || dom_event_name == "mouseup"
    {
        let closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            use rapidr_value::input::{shift_state, Button, Mouse};
            let Some(el) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
            let inner = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).and_then(|t| t.closest(".rr-widget, .rr-form").ok().flatten());
            if inner.is_some_and(|i| i != el) {
                return;
            }
            let rect = el.get_bounding_client_rect();
            let title = el
                .query_selector(":scope > .rr-form-titlebar")
                .ok()
                .flatten()
                .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
                .map_or(0, |t| t.offset_height() as i64);
            let x = (e.client_x() as f64 - rect.left()) as i64;
            let y = (e.client_y() as f64 - rect.top()) as i64 - title;
            let kind = match event_for_closure.as_str() {
                "onmousedown" => Mouse::Down,
                "onmouseup" => Mouse::Up,
                _ => Mouse::Move,
            };
            let shift = shift_state(e.shift_key(), e.ctrl_key(), e.alt_key());
            rp_fire_event_args(&name_for_closure, kind.event(), &kind.args(Button::from_dom(e.button()), x, y, shift));
        });
        let _ = el.add_event_listener_with_callback(dom_event_name, closure.as_ref().unchecked_ref());
        closure.forget();
        return;
    }

    // For all other events, fire arity 0
    let closure = Closure::<dyn FnMut()>::new(move || {
        rp_fire_event(&name_for_closure, &event_for_closure);
    });
    let _ = el.add_event_listener_with_callback(dom_event_name, closure.as_ref().unchecked_ref());
    closure.forget();
}

// ---------------------------------------------------------------------------
// Timer management
// ---------------------------------------------------------------------------

fn update_timer(name: &str) {
    let uname = name.to_uppercase();

    // Clear existing timer
    TIMER_HANDLES.with(|th| {
        let mut handles = th.borrow_mut();
        if let Some(handle) = handles.remove(&uname) {
            if let Some(window) = web_sys::window() {
                window.clear_interval_with_handle(handle);
            }
        }
    });

    // Check if timer should be running
    let (enabled, interval) = COMPONENTS.with(|c| {
        let comps = c.borrow();
        if let Some(comp) = comps.get(&uname) {
            let enabled = comp
                .properties
                .get("enabled")
                .map(|v| v.to_bool())
                .unwrap_or(false);
            let interval = comp
                .properties
                .get("interval")
                .map(|v| v.to_i64())
                .unwrap_or(1000);
            (enabled, interval)
        } else {
            (false, 1000)
        }
    });

    // Check if we have an ontimer event handler registered
    let has_handler = EVENT_HANDLERS.with(|eh| {
        eh.borrow().contains_key(&(uname.clone(), "ontimer".to_string()))
    });

    if enabled && has_handler && interval > 0 {
        let name_for_closure = uname.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            rp_fire_event(&name_for_closure, "ontimer");
        });

        if let Some(window) = web_sys::window() {
            match window.set_interval_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                interval as i32,
            ) {
                Ok(handle) => {
                    TIMER_HANDLES.with(|th| {
                        th.borrow_mut().insert(uname, handle);
                    });
                }
                Err(e) => {
                    web_sys::console::error_1(&e);
                }
            }
        }
        closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Component type checking
// ---------------------------------------------------------------------------

pub fn is_component_type(type_name: &str) -> bool {
    matches!(
        type_name.to_uppercase().as_str(),
        "RFORM"
            | "RBUTTON"
            | "RLABEL"
            | "REDIT"
            | "RPANEL"
            | "RCHECKBOX"
            | "RRADIOBUTTON"
            | "RCOMBOBOX"
            | "RLISTBOX"
            | "RFILELISTBOX"
            | "RDIRTREE"
            | "RTIMER"
            | "RIMAGE"
            | "RCANVAS"
            | "RSTRINGGRID"
            | "RTABCONTROL"
            | "RTREEVIEW"
            | "RMAINMENU"
            | "RMENUITEM"
            | "RPOPUPMENU"
            | "RGROUPBOX"
            | "RDESIGNSURFACE"
            | "RCODEEDITOR"
            | "ROPENDIALOG"
            | "RSAVEDIALOG"
            | "RCOLORDIALOG"
            | "RFONTDIALOG"
            | "RSTATUSBAR"
            | "RPROGRESS"
            | "RPROGRESSBAR"
            | "RRICHEDIT"
            | "RMEMO"
            | "RFILESTREAM"
            | "RJSON"
            | "RSTRINGLIST"
            | "RTOOLBAR"
            | "RSCROLLBAR"
            | "RDATETIMEPICKER"
            | "RTRACKBAR"
            | "RUPDOWN"
            | "RPRINTER"
            | "RSQLITE"
            | "RMYSQL"
            | "RSOCKET"
            | "RSERVERSOCKET"
            | "RHTTP"
            | "RSPLITTER"
            | "RSCROLLBOX"
            | "RLISTVIEW"
            | "RNUM"
            | "RDATAFRAME"
            | "RPLOT"
            // Web-exclusive
            | "RWEBVIEW"
            | "RDOM"
            | "RJAVASCRIPT"
            | "RWEBSTORAGE"
            | "RWEBAUDIO"
            | "RWEBVIDEO"
            | "RWEBNOTIFICATION"
            | "RWEBGEOLOCATION"
            | "RROUTER"
    )
}

pub fn is_component_method(member: &str) -> bool {
    matches!(
        member.to_lowercase().as_str(),
        "additem"
            | "clear"
            | "removeitem"
            | "deleteitem"
            | "setfocus"
            | "focus"
            | "refresh"
            | "repaint"
            | "invalidate"
            | "show"
            | "showmodal"
            | "setparent"
            | "hide"
            | "close"
            | "cls"
            | "line"
            | "rect"
            | "rectangle"
            | "fillrect"
            | "circle"
            | "fillcircle"
            | "drawtext"
            | "textout"
            | "setpixel"
            | "pset"
            | "setcell"
            | "getcell"
            | "setrowcount"
            | "setcolcount"
            | "addtab"
            | "removetab"
            | "addnode"
            | "add"
            | "insert"
            | "delete"
            | "remove"
            | "get"
            | "strings"
            | "indexof"
            | "find"
            | "sort"
            | "savetofile"
            | "loadfromfile"
            // Network methods
            | "open"
            | "send"
            | "receive"
            | "connect"
            | "disconnect"
            | "listen"
            // Web-exclusive methods
            | "sethtml"
            | "navigate"
            | "create"
            | "appendto"
            | "setattribute"
            | "getattribute"
            | "addclass"
            | "removeclass"
            | "toggleclass"
            | "queryselector"
            | "queryselectorall"
            | "eval"
            | "call"
            | "set"
            | "haskey"
            | "keys"
            | "play"
            | "pause"
            | "stop"
            | "seek"
            | "fullscreen"
            | "requestpermission"
            | "getposition"
            | "watchposition"
            | "clearwatch"
            | "addroute"
            | "back"
            | "forward"
            // RNum methods
            | "arange"
            | "linspace"
            | "zeros"
            | "ones"
            | "full"
            | "fromlist"
            | "from_list"
            | "sum"
            | "mean"
            | "min"
            | "max"
            | "std"
            | "var"
            | "variance"
            | "median"
            | "argmin"
            | "argmax"
            | "count"
            | "ptp"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "arcsin"
            | "acos"
            | "arccos"
            | "atan"
            | "arctan"
            | "sqrt"
            | "abs"
            | "exp"
            | "log"
            | "ln"
            | "log2"
            | "log10"
            | "floor"
            | "ceil"
            | "round"
            | "sign"
            | "reciprocal"
            | "square"
            | "negative"
            | "neg"
            | "subtract"
            | "sub"
            | "multiply"
            | "mul"
            | "divide"
            | "div"
            | "power"
            | "pow"
            | "mod"
            | "fmod"
            | "clip"
            | "clamp"
            | "reverse"
            | "flip"
            | "unique"
            | "shuffle"
            | "append"
            | "concatenate"
            | "slice"
            | "cumsum"
            | "cumprod"
            | "diff"
            | "dot"
            | "norm"
            | "normalize"
            | "any"
            | "all"
            | "nonzero"
            | "searchsorted"
            | "rand"
            | "random"
            | "randn"
            | "random_normal"
            | "normal"
            | "uniform"
            | "random_uniform"
            | "randint"
            | "choice"
            | "tolist"
            | "tostring"
            | "print"
            // RDataFrame methods
            | "loadfromcsv"
            | "readcsv"
            | "read_csv"
            | "savetocsv"
            | "to_csv"
            | "head"
            | "tail"
            | "cell"
            | "cellbyname"
            | "at"
            | "iloc"
            | "select"
            | "sort_values"
            | "filter"
            | "query"
            | "groupby"
            | "group_by"
            | "drop_column"
            | "rename_column"
            | "addcolumn"
            | "add_column"
            | "set_column"
            | "fillna"
            | "fill_null"
            | "dropna"
            | "drop_nulls"
            | "describe"
            | "value_counts"
            | "nunique"
            | "corr"
            | "correlation"
            | "sample"
            | "nlargest"
            | "nsmallest"
            | "info"
            | "dtypes"
            | "shape"
            | "merge"
            | "join"
            | "concat"
            | "transpose"
            | "t"
            | "apply"
            | "replace"
            | "columns"
            | "rows"
            | "rowcount"
            | "len"
            | "togrid"
            | "to_grid"
            | "display"
            // RPlot methods
            | "plot"
            | "bar"
            | "barh"
            | "scatter"
            | "step"
            | "area"
            | "fill_between"
            | "hist"
            | "histogram"
            | "pie"
            | "hline"
            | "axhline"
            | "vline"
            | "axvline"
            | "annotate"
            | "legend"
            | "savefig"
            | "save"
            | "render"
            | "figsize"
            | "xlim"
            | "ylim"
            // RSQLite methods
            | "fetchrow"
            | "fetchfield"
            | "fieldseek"
            | "rowseek"
            | "row"
            | "escapestring"
            | "execute"
    )
}

pub fn get_children_of(parent_name: &str) -> Vec<(String, String)> {
    let uname = parent_name.to_uppercase();
    COMPONENTS.with(|c| {
        let comps = c.borrow();
        let mut children: Vec<(String, String, u32)> = comps
            .iter()
            .filter(|(_, comp)| {
                comp.properties
                    .get("parent")
                    .map(|v| v.to_string_val().to_uppercase() == uname)
                    .unwrap_or(false)
            })
            .map(|(name, comp)| (name.clone(), comp.type_name.clone(), comp.creation_order))
            .collect();
        children.sort_by_key(|c| c.2);
        children.into_iter().map(|(n, t, _)| (n, t)).collect()
    })
}

// ---------------------------------------------------------------------------
// Run app — no-op on web (the browser IS the event loop)
// ---------------------------------------------------------------------------

pub fn rp_run_app() {
    // On web, the browser event loop handles everything.
    // This is intentionally a no-op.
}

// ---------------------------------------------------------------------------
// Theme — no-op on web (styling comes from rapidr-rrcss::RR_BASE_CSS)
// ---------------------------------------------------------------------------

pub fn set_theme(_theme: &str) {
    // Themes don't apply to web — the shared RR_BASE_CSS provides the styling
}

pub fn gui_register_timer(_name: &str) {
    // Timers are handled via DOM setInterval in update_timer()
}

pub fn rp_comp_get_all_properties(name: &str) -> Option<(String, std::collections::HashMap<String, Value>)> {
    let uname = name.to_uppercase();
    COMPONENTS.with(|c| {
        c.borrow().get(&uname).map(|comp| {
            (comp.type_name.clone(), comp.properties.clone())
        })
    })
}

/// Offers `text` to the user as a file download via Blob + object URL.
/// Content and filename are passed as data, never spliced into JS source.
fn trigger_download(filename: &str, mime: &str, text: &str) {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(text));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts) else { return };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else { return };
    let doc = crate::gui_web::document();
    if let Some(a) = doc
        .create_element("a")
        .ok()
        .and_then(|el| el.dyn_into::<web_sys::HtmlAnchorElement>().ok())
    {
        a.set_href(&url);
        a.set_download(filename);
        let _ = a.style().set_property("display", "none");
        if let Some(body) = doc.body() {
            let _ = body.append_child(&a);
            a.click();
            let _ = body.remove_child(&a);
        }
    }
    let _ = web_sys::Url::revoke_object_url(&url);
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
