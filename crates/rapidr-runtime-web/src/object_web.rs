//! Component registry and event system for the web runtime.
//!
//! Mirrors the desktop `object.rs` API — same function signatures so that
//! generated code works identically on both targets.

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
    // (the system tray's strip: drawn when the program changes its icons)
    rapidr_value::tray::set_on_change(crate::tray_web::changed);
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

    // What every runtime gives it: the language registry's defaults,
    // RapidQ's as RC.EXE reads them (rapidr_value::component_defaults), then
    // the web's own by type (a type not listed: a generic place and size).
    let mut props: HashMap<String, Value> = rapidr_value::component_defaults::creation(type_name).into_iter().collect();
    match utype.as_str() {
        // Nothing more (and not the generic place and size below): what the
        // registry and the shared models give them.
        "RFORM" | "RBUTTON" | "RLABEL" | "REDIT" | "RMEMO" | "RRICHEDIT" | "RCHECKBOX" | "RRADIOBUTTON" | "RCOMBOBOX" | "RLISTBOX" | "RFILELISTBOX" | "RDIRTREE"
        | "RTIMER" | "RIMAGE" | "RCANVAS" | "RDXSCREEN" | "RDXTIMER" | "RHEADER" | "RSTRINGGRID" | "RPROGRESS" | "RPROGRESSBAR" | "RSQLITE" | "RJSON" | "ROPENDIALOG"
        | "RSAVEDIALOG" | "RFILEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG" | "RPANEL" | "RTRACKBAR" | "RWEBVIEW" | "RWEBSTORAGE" | "RWEBNOTIFICATION" | "RNUM"
        | "RDATAFRAME" | "RCOOLBTN" => {}
        "RDESIGNSURFACE" | "RPLOT" => {
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
        }
        // (QDXSOUND: its sound's properties are the model's)
        "RDXSOUND" => {
            crate::directx_web::install_sound_device();
        }
        // (QDXJOYSTICK: the page looks for its events like a timer's ticks
        // — directx_web::timer_fired)
        "RDXJOYSTICK" => {
            crate::directx_web::install_joystick_source();
        }
        "RWEBAUDIO" | "RWEBVIDEO" => {
            props.insert("src".to_string(), v_str(""));
        }
        "ROVALBTN" => {
            props.insert("color".to_string(), v_int(0xDCDCDC));
            props.insert("colorhighlight".to_string(), v_int(0xFFFFFF));
            props.insert("colorshadow".to_string(), v_int(0x808080));
        }
        "RFILESTREAM" => {
            // In-browser virtual file: text + filename, plus a download/pickfile bridge.
            props.insert("text".to_string(), v_str(""));
            props.insert("eof".to_string(), v_bool(false));
            props.insert("mimetype".to_string(), v_str("text/plain"));
        }
        _ => {
            // Generic defaults
            props.insert("left".to_string(), v_int(0));
            props.insert("top".to_string(), v_int(0));
            props.insert("width".to_string(), v_int(100));
            props.insert("height".to_string(), v_int(25));
        }
    }
    // (the web's own elements show until the program hides them, and say so)
    if matches!(utype.as_str(), "RWEBVIEW" | "RDOM" | "RWEBAUDIO" | "RWEBVIDEO") {
        props.entry("visible".to_string()).or_insert(v_bool(true));
    }
    // Its size: RapidQ's, the same on every runtime (rapidr_value::layout).
    if let Some((w, h)) = rapidr_value::layout::default_size(&utype) {
        props.insert("left".to_string(), props.get("left").cloned().unwrap_or(v_int(0)));
        props.insert("top".to_string(), props.get("top").cloned().unwrap_or(v_int(0)));
        props.insert("width".to_string(), v_int(w));
        props.insert("height".to_string(), v_int(h));
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
    // (the web-only components' elements, once the component is
    // registered: overlay_web, placed by the UI kernel's host)
    if crate::overlay_web::is_overlay(&rp_comp_type(&name_clone)) {
        crate::overlay_web::create(&name_clone, &rp_comp_type(&name_clone));
    }
    // (the kernel's tree has it)
    crate::kernel_web::created(&name_clone);
    install_object_hooks();
    if rapidr_value::objects::create(name, type_name) {
        rapidr_value::objects::set_file_io(web_read_file, web_write_file);
        // (the I/O and media lane's: their devices, a QDOWNLOAD's gauge)
        if rapidr_value::objects::rqlib::is_type(type_name) {
            crate::io_web::created(name, type_name);
        }
    }
}

/// How shared objects print on the web (`Printer.EndDoc`, [`web_print`]).
pub fn install_object_hooks() {
    rapidr_value::objects::set_print_hook(web_print);
    // QREGISTRY's keys: the page's local storage.
    rapidr_value::registry::set_io(registry_load, registry_save);
    // RND's first seed (wasm has no clock).
    rapidr_value::builtins::set_entropy(|| (js_sys::Math::random() * 9_007_199_254_740_992.0) as u64);
}

/// QREGISTRY's store in the page's local storage (`None`: nothing yet, or
/// no local storage — the keys then last while the page does).
fn registry_load() -> Option<String> {
    web_sys::window()?.local_storage().ok()??.get_item("rapidr.registry").ok()?
}

fn registry_save(text: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item("rapidr.registry", text);
    }
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

/// A path as RapidQ on Windows compares them: `\\` and `/` alike, no
/// leading `./`, any case.
pub(crate) fn file_key(path: &str) -> String {
    let p = path.trim().replace('\\', "/");
    p.trim_start_matches("./").to_lowercase()
}

/// The name a file saved this session is kept under, if one matches `path`.
fn saved_name(path: &str) -> Option<String> {
    let key = file_key(path);
    SAVED_FILES.with(|f| f.borrow().keys().find(|n| file_key(n) == key).cloned())
}

/// A file of the program's project (the IDE's assets, a bundle's files):
/// `window.__rapidr_assets` maps names to data URLs.
fn asset_bytes(path: &str) -> Option<Vec<u8>> {
    let key = file_key(path);
    let data = crate::database_web::get_rapidr_asset(&path.replace('\\', "/")).or_else(|| {
        let window: JsValue = web_sys::window()?.into();
        let assets = js_sys::Reflect::get(&window, &JsValue::from_str("__rapidr_assets")).ok()?;
        let names = js_sys::Object::keys(assets.dyn_ref::<js_sys::Object>()?);
        let name = names.iter().filter_map(|n| n.as_string()).find(|n| file_key(n) == key || file_key(n.strip_prefix("assets/").unwrap_or(n)) == key)?;
        js_sys::Reflect::get(&assets, &JsValue::from_str(&name)).ok()?.as_string()
    })?;
    crate::database_web::decode_base64(&data)
}

/// Whether the program can open `path`: a file saved this session or one of
/// its project's files (FILEEXISTS).
pub fn web_file_exists(path: &str) -> bool {
    !path.trim().is_empty() && (saved_name(path).is_some() || asset_bytes(path).is_some())
}

/// A file the program saved this session or one of its project's files —
/// never fetched (RSQLITE's `Connect(file)`: a name that's neither is a
/// new database, not a request to the server).
pub fn web_project_file(path: &str) -> Option<Vec<u8>> {
    if path.trim().is_empty() {
        return None;
    }
    saved_name(path).and_then(|n| SAVED_FILES.with(|f| f.borrow().get(&n).cloned())).or_else(|| asset_bytes(path))
}

/// Reads a file for an object (`Bitmap.LoadFromFile`, `ImageList.AddBMPFile`):
/// one saved earlier this session, one of the project's files, or one
/// shipped with the page (fetched synchronously, as the program expects the
/// data on the next line). Names match as on Windows (any case, `\\`).
fn web_read_file(path: &str) -> Result<Vec<u8>, String> {
    if let Some(bytes) = saved_name(path).and_then(|n| SAVED_FILES.with(|f| f.borrow().get(&n).cloned())) {
        return Ok(bytes);
    }
    if let Some(bytes) = asset_bytes(path) {
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
    if let Some(name) = saved_name(path) {
        SAVED_FILES.with(|f| f.borrow_mut().remove(&name));
    }
}

pub(crate) fn web_write_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    web_store_file(path, bytes.to_vec());
    // (a name an Open / Save dialog answered: the user's real file too)
    crate::file_picker_web::written(path, bytes);
    // (a page that keeps the files its program writes — RapidR Studio's,
    // in the browser's private file system — hears each one:
    // `window.RAPIDR_FILE_SINK(path, bytes)`)
    if let Some(w) = web_sys::window() {
        if let Ok(f) = js_sys::Reflect::get(&w, &"RAPIDR_FILE_SINK".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
            let _ = f.call2(&JsValue::NULL, &JsValue::from_str(path), &js_sys::Uint8Array::from(bytes).into());
        }
    }
    Ok(())
}

/// The names in the page's store under folder `folder` (directly in it):
/// RPROJECT.OpenFolder's look at a folder picked on the web.
pub fn stored_names_in(folder: &str) -> Vec<String> {
    let prefix = format!("{}/", folder.trim_end_matches('/').replace('\\', "/"));
    let lower = prefix.to_lowercase();
    SAVED_FILES.with(|f| {
        let mut names: Vec<String> = f
            .borrow()
            .keys()
            .filter(|k| k.to_lowercase().starts_with(&lower))
            .map(|k| k[prefix.len()..].to_string())
            .filter(|rest| !rest.contains('/'))
            .collect();
        names.sort();
        names
    })
}

/// `bytes` as file `path` in the page's store (over a file of the same
/// name in another case, as on Windows) — a file the user picked to open,
/// read whole before Execute returns.
pub(crate) fn web_store_file(path: &str, bytes: Vec<u8>) {
    let name = saved_name(path).unwrap_or_else(|| path.to_string());
    SAVED_FILES.with(|f| f.borrow_mut().insert(name, bytes));
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
    // (a font style keeps 1 or 0, RapidQ's: rapidr_value::objects::font)
    let val = rapidr_value::objects::font::component_style(prop, &val).map_or(val, |(_, _, v)| v);
    // (the program's first font change of a component: the font it had from
    // its parents becomes its own — Delphi's ParentFont ends)
    let t = rp_comp_type(name);
    if !t.is_empty() && !rapidr_value::objects::TYPES.contains(&t.as_str()) {
        for (p, v) in rapidr_value::objects::own_font_from_parents(name, prop, &|i, p| rp_comp_get(i, p)) {
            set_property(name, p, v);
        }
    }
    // (a Color or a Parent changed: the canvases' backdrops follow)
    let backdrops = prop.eq_ignore_ascii_case("color") || prop.eq_ignore_ascii_case("parent");
    // (an AutoSize QLABEL's Caption, WordWrap, AutoSize, font or Parent: it
    // takes its text's size — layout_web.rs, rapidr_value::autosize)
    let labels = crate::layout_web::labels_before_set(name, prop);
    // (the program chose a Font.Color — or a whole Font: RapidQ's
    // ParentFont no longer applies, rapidr_value::component_defaults::
    // font_color_read)
    if matches!(prop.to_ascii_lowercase().as_str(), "font.color" | "fontcolor" | "font") {
        COMPONENTS.with(|c| {
            if let Some(comp) = c.borrow_mut().get_mut(&name.to_uppercase()) {
                comp.properties.insert("__fontcolorset".into(), v_bool(true));
            }
        });
    }
    set_property(name, prop, val);
    if let Some(before) = labels {
        crate::layout_web::labels_after_set(name, before);
    }
    if backdrops {
        refresh_canvas_backdrops();
    }
}

/// What `name.Color` reads in a program: the Color the program set, else
/// RapidQ's (rapidr_value::component_defaults::color_read — clBtnFace for a
/// form or a panel, the parent's for a label / canvas / group box, clWindow
/// for the rest), as RC.EXE reads them.
pub fn program_color(name: &str) -> Value {
    fn read(name: &str, depth: u32) -> Value {
        let stored = rp_comp_get(name, "color");
        rapidr_value::component_defaults::color_read(&rp_comp_type(name), &stored, || {
            let parent = rp_comp_get(name, "parent").to_string_val();
            (depth < 32 && !parent.is_empty() && !parent.eq_ignore_ascii_case(name)).then(|| read(&parent, depth + 1))
        })
        .unwrap_or(stored)
    }
    read(name, 0)
}

/// Every QCANVAS shows its parent's colour where nothing is drawn, as
/// RapidQ's (a TPaintBox) does whatever its own Color: their models'
/// backdrops made the parents' colours again.
/// What `name.Font.Color` reads in a program: the one the program set,
/// else its parent's (ParentFont), else clWindowText — RapidQ's, as RC.EXE
/// reads it (rapidr_value::component_defaults::font_color_read).
pub fn program_font_color(name: &str) -> Value {
    fn read(name: &str, depth: u32) -> Value {
        let set = rp_comp_get(name, "__fontcolorset").to_bool();
        rapidr_value::component_defaults::font_color_read(set, rp_comp_get(name, "fontcolor"), || {
            let parent = rp_comp_get(name, "parent").to_string_val();
            (depth < 32 && !parent.is_empty() && !parent.eq_ignore_ascii_case(name) && !rp_comp_type(&parent).is_empty()).then(|| read(&parent, depth + 1))
        })
    }
    read(name, 0)
}

/// The Font.Color a component's text is drawn in, for the kernel: its own
/// when the program set it, else the nearest parent's the program set
/// (RapidQ's ParentFont), else what's stored (the theme's text).
pub fn drawn_font_color(name: &str, prop: &str) -> Value {
    let mut at = name.to_string();
    for _ in 0..32 {
        if rp_comp_get(&at, "__fontcolorset").to_bool() {
            return rp_comp_get(&at, prop);
        }
        let parent = rp_comp_get(&at, "parent").to_string_val();
        if parent.is_empty() || parent.eq_ignore_ascii_case(&at) || rp_comp_type(&parent).is_empty() {
            break;
        }
        at = parent;
    }
    rp_comp_get(name, prop)
}

/// `Form.Pixel(x, y)` as RapidQ reads it (rapidr_value::component_defaults::
/// form_pixel: -1 unless shown, outside, or over a window of its own; a
/// graphic control's pixel), or None: the form's surface answers.
fn form_pixel(name: &str, args: &[Value]) -> Option<Value> {
    let (x, y) = (args[0].to_i64(), args[1].to_i64());
    let shown = rp_comp_get(name, "visible").to_bool();
    let client = (rp_comp_get(name, "clientwidth").to_i64(), rp_comp_get(name, "clientheight").to_i64());
    let mut kids: Vec<(u32, rapidr_value::component_defaults::PixelChild)> = COMPONENTS.with(|m| {
        m.borrow()
            .iter()
            .filter(|(_, c)| c.properties.get("parent").is_some_and(|p| p.to_string_val().eq_ignore_ascii_case(name)))
            .map(|(id, c)| (c.creation_order, id.clone(), c.type_name.clone()))
            .collect::<Vec<_>>()
    })
    .into_iter()
    .filter(|(_, id, _)| rp_comp_get(id, "visible").to_bool() || matches!(rp_comp_get(id, "visible"), Value::Null))
    .map(|(order, id, type_name)| {
        let g = |p: &str| rp_comp_get(&id, p).to_i64();
        let color = match rp_comp_get(&id, "color") {
            Value::Null => None,
            v => Some(v.to_i64()),
        };
        (order, rapidr_value::component_defaults::PixelChild { rect: (g("left"), g("top"), g("width"), g("height")), id, type_name, color })
    })
    .collect();
    kids.sort_by_key(|(o, _)| *o);
    let kids: Vec<_> = kids.into_iter().map(|(_, k)| k).collect();
    let form_color = program_color(name).to_i64();
    rapidr_value::component_defaults::form_pixel(shown, client, x, y, form_color, &kids).map(v_int)
}

fn refresh_canvas_backdrops() {
    let canvases: Vec<String> = COMPONENTS.with(|m| m.borrow().iter().filter(|(_, comp)| comp.type_name == "RCANVAS").map(|(n, _)| n.clone()).collect());
    for c in canvases {
        let parent = rp_comp_get(&c, "parent").to_string_val();
        let color = if parent.is_empty() { Value::Null } else { program_color(&parent) };
        if rapidr_value::objects::set_backdrop(&c, rapidr_value::objects::form_color(&color) as u32) {
            crate::kernel_web::redraw();
        }
    }
}

fn set_property(name: &str, prop: &str, val: Value) {
    // A QRECT's / QNOTIFYICONDATA's field: stored as RapidQ stores it
    // (rapidr_value::objects::record), nothing else.
    if rapidr_value::objects::is_record(name) {
        rapidr_value::objects::set(name, prop, &val);
        return;
    }
    let val = rapidr_value::layout::property_value(prop, val);
    // QBUTTON Kind: its caption and ModalResult (rapidr_value::events).
    if prop.eq_ignore_ascii_case("kind") {
        if let Some((caption, mr)) = rapidr_value::events::button_kind(val.to_i64()) {
            let current = rp_comp_get(name, "caption").to_string_val();
            if current.is_empty() || (1..=10).any(|k| rapidr_value::events::button_kind(k).map(|(c, _)| c) == Some(current.as_str())) {
                rp_comp_set(name, "caption", crate::value::v_str(caption));
            }
            rp_comp_set(name, "modalresult", v_int(mr));
        }
    }
    // (the I/O and media lane's: a QDOWNLOAD's StateGauge / SpeedLbl, and
    // these objects' own properties — io_web.rs)
    if let Some((sub, member)) = crate::io_web::sub_component(name, &prop.to_lowercase()) {
        return rp_comp_set(&sub, &member, val);
    }
    if rapidr_value::objects::rqlib::exists(name) {
        if let Some(Ok(())) = rapidr_value::objects::rqlib::set(name, &prop.to_lowercase(), &val) {
            crate::io_web::fire_events(name);
            return;
        }
    }
    // A modal form's ModalResult set: the form closes (ShowModal returns it).
    if prop.eq_ignore_ascii_case("modalresult") && val.to_i64() != 0 && rp_comp_type(name) == "RFORM" && rapidr_ui_app::forms::is_modal(name) {
        rp_comp_set_prop_only(name, "modalresult", val);
        crate::kernel_web::close_form(&name.to_uppercase());
        return;
    }
    let uname = name.to_uppercase();
    let lprop = prop.to_lowercase();
    // Screen, Application, Clipboard, Mouse (globals_web.rs).
    if crate::globals_web::set(name, &lprop, &val) {
        return;
    }
    // A QFORM's / QSCROLLBOX's AutoScroll, HorzPosition, … (scroll_web.rs).
    if crate::scroll_web::set(name, &lprop, &val) {
        return;
    }
    // A QFORMMDI's ChildMax, ChildCaption, ChildState, … (mdi_web.rs).
    if rapidr_value::mdi::is_mdi(name) && crate::mdi_web::set(name, &lprop, &val) {
        return;
    }
    // (I1) RapidR Studio's RPROJECT, RLANGUAGESERVICE, RPROGRAMSESSION (studio_web.rs).
    if rapidr_studio::is_studio_type(&rp_comp_type(&uname)) && crate::studio_web::set(&rp_comp_type(&uname), name, &lprop, &val) {
        return;
    }
    // (I1) An RDOCKMANAGER's DocumentMode, ActiveDocument, … (dock_web.rs).
    if rp_comp_type(&uname) == "RDOCKMANAGER" && crate::dock_web::set(name, &lprop, &val) {
        return;
    }
    // (the dialogs lane's) A QFONTDIALOG's Name / Size / Color are its flat
    // FontName / FontSize / FontColor too: one value, as on the desktop.
    if let Some(other) = rapidr_value::font_dialog::alias(&lprop).filter(|_| rp_comp_type(&uname) == "RFONTDIALOG") {
        rp_comp_set_prop_only(&uname, other, val.clone());
    }
    // A QDIGDISPLAY is as big as its Display (QDigDisplay.inc sizes it so).
    let val = match rapidr_value::objects::digdisplay_text(name) {
        Some(text) if matches!(lprop.as_str(), "width" | "height") => {
            let (w, h) = rapidr_value::objects::digdisplay::size(&text);
            v_int(if lprop == "width" { w } else { h })
        }
        _ => val,
    };
    // A QGLASSFRAME's Transparency, TransparentColor, Moveable: as RC.EXE
    // stores them (rapidr_value::objects::glass).
    let val = if rp_comp_type(&uname) == "RGLASSFRAME" { rapidr_value::objects::glass::stored(&lprop, &val).unwrap_or(val) } else { val };
    // A QBEVEL's Shape / Style set its bevels (QBevel.inc's setters).
    if rp_comp_type(&uname) == "RBEVEL" {
        let other = rp_comp_get(&uname, if lprop == "shape" { "style" } else { "shape" }).to_i64();
        if let Some(bevels) = rapidr_value::objects::bevel::qbevel_set(&lprop, val.to_i64(), other) {
            for (p, v) in bevels {
                rp_comp_set_prop_only(&uname, p, v_int(v));
            }
        }
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
    // Constraints (RapidR, from Delphi; rapidr_value::layout): MinWidth …
    // MaxHeight — `Constraints.MinWidth` is MinWidth — bound every size the
    // component takes; setting one brings its size within them (as on the
    // desktop).
    if let Some(flat) = rapidr_value::layout::constraint_alias(&lprop) {
        return rp_comp_set(name, flat, val);
    }
    let real = || !matches!(rp_comp_type(&uname).as_str(), "" | "RUDT");
    let val = match val {
        v if matches!(lprop.as_str(), "width" | "height") && real() => {
            let k = crate::layout_web::constraints_of(&uname);
            if k.is_none() {
                v
            } else {
                v_int(if lprop == "width" { k.width(v.to_i64()) } else { k.height(v.to_i64()) })
            }
        }
        v => v,
    };
    if rapidr_value::layout::CONSTRAINT_PROPERTIES.contains(&lprop.as_str()) && real() {
        if let Some(k) = crate::layout_web::constraints_of(&uname).with(&lprop, val.to_i64()) {
            for (p, v) in k.properties() {
                rp_comp_set_prop_only(&uname, p, v_int(v));
            }
            for p in ["width", "height"] {
                let size = rp_comp_get_stored(&uname, p);
                if !matches!(size, Value::Null) {
                    let bounded = if p == "width" { k.width(size.to_i64()) } else { k.height(size.to_i64()) };
                    if bounded != size.to_i64() {
                        rp_comp_set(&uname, p, v_int(bounded));
                    }
                }
            }
            return;
        }
    }

    // (a menu's change shows once the program's code returns: menu_web)
    if rapidr_value::objects::menu::is_menu(name) {
        crate::kernel_web::redraw();
    }
    // QFONT, QMEMORYSTREAM, QBITMAP, QIMAGELIST (shared with the desktop runtime).
    let before_dir = if rapidr_value::objects::is_dirtree(name) { rp_comp_get_stored_dir(name) } else { String::new() };
    if let Some(result) = rapidr_value::objects::set(name, &lprop, &val) {
        let picture = rapidr_value::objects::is_picture(name);
        match result {
            // `Image.BMP = "photo.png"`: not a BMP — nothing loaded, as on
            // the desktop.
            Err(_) if picture && lprop == "bmp" => not_a_bmp(&val.to_string_val()),
            Err(e) => object_error(name, prop, &e),
            Ok(()) => {}
        }
        // A QDIGDISPLAY's new Display: its size.
        if lprop == "display" {
            if let Some(text) = rapidr_value::objects::digdisplay_text(name) {
                let (w, h) = rapidr_value::objects::digdisplay::size(&text);
                rp_comp_set(&uname, "width", v_int(w));
                rp_comp_set(&uname, "height", v_int(h));
            }
        }
        if picture {
            picture_changed(&uname);
        }
        // A QFILELISTBOX's directory changed: OnChange.
        if lprop == "directory" && rapidr_value::objects::is_file_list(name) {
            rp_fire_event(&uname, "onchange");
        }
        // A QDIRTREE's directory changed: OnChange.
        if rapidr_value::objects::is_dirtree(name) && matches!(lprop.as_str(), "directory" | "initialdir") && before_dir != rp_comp_get_stored_dir(name) {
            rp_fire_event(&uname, "onchange");
        }
        // (drawn again: a tree's rows built again first)
        if rapidr_value::objects::is_tree(name) {
            crate::kernel_web::tree_refresh(&uname);
        } else {
            crate::kernel_web::redraw();
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
    for (dotted, flat) in [("font.name", "fontname"), ("font.size", "fontsize"), ("font.bold", "fontbold"), ("font.italic", "fontitalic"), ("font.underline", "fontunderline"), ("font.strikeout", "fontstrikeout"), ("font.color", "fontcolor")] {
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
            // (a Color the program chose, as the desktop records it)
            if lprop == "color" {
                comp.properties.insert("__colorset".into(), v_bool(true));
            }

            if comp.type_name == "RTIMER" || comp.type_name == "RDXTIMER" || comp.type_name == "RDXJOYSTICK" || comp.type_name == "RCOMPORT" {
                if lprop == "enabled" || lprop == "interval" {
                    drop(comps);
                    update_timer(&uname);
                    return;
                }
            }
        }
    });

    // A combo box that's owner-drawn now (its Style, which the list model
    // leaves stored here too): the element changes.
    if lprop == "style" && rapidr_value::objects::with_list(name, |l| l.combo && l.owner_drawn()).unwrap_or(false) {
        crate::kernel_web::redraw();
    }

    // Handle parent re-parenting
    if lprop == "parent" {
        // (a web-only component's element goes where its new parent is; the
        // kernel's tree follows)
        if crate::overlay_web::is_overlay(&rp_comp_type(&uname)) {
            crate::overlay_web::set_prop(&uname, "parent", &val);
        }
        crate::kernel_web::rebuild();
        crate::layout_web::after_set(&uname, &lprop);
        // (the DirectX lane's: a QDXSCREEN put on a form already shown)
        if rp_comp_type(&uname) == "RDXSCREEN" {
            crate::directx_web::parented(&uname);
        }
        // (a QGLASSFRAME shades what it's now over)
        if rp_comp_type(&uname) == "RGLASSFRAME" {
            crate::kernel_web::redraw();
        }
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
        crate::kernel_web::redraw();
        return;
    }

    // A QGLASSFRAME's shade drawn again.
    if comp_type == "RGLASSFRAME" && matches!(lprop.as_str(), "transparency" | "transparentcolor" | "color") {
        crate::kernel_web::set_prop(&uname, &lprop, &val);
        crate::kernel_web::redraw();
        return;
    }
    // (a colour under a QGLASSFRAME changed: its shade with it, as the
    // desktop's kernel draws it over what is there)
    if lprop == "color" && COMPONENTS.with(|c| c.borrow().values().any(|comp| comp.type_name == "RGLASSFRAME")) {
        crate::kernel_web::redraw();
    }
    // A panel's bevels drawn again.
    if (rapidr_value::objects::bevel::default(&lprop).is_some() && comp_type == "RPANEL") || (comp_type == "RBEVEL" && (rapidr_value::objects::bevel::default(&lprop).is_some() || matches!(lprop.as_str(), "shape" | "style"))) {
        crate::kernel_web::redraw();
        return;
    }
    // Pass to GUI layer for DOM update
    crate::kernel_web::set_prop(&uname, &lprop, &val);
    // Align (layout_web).
    crate::layout_web::after_set(&uname, &lprop);
    // (I1) A dock manager or its floating window resized: its panes placed.
    crate::dock_web::after_set(&uname, &lprop);
    // A QCANVAS's new size (its surface follows).
    if matches!(lprop.as_str(), "width" | "height") && rapidr_value::objects::is_header(&uname) {
        crate::kernel_web::redraw();
    } else if matches!(lprop.as_str(), "width" | "height") && rapidr_value::objects::is_canvas(&uname) {
        crate::kernel_web::redraw();
        if !rapidr_value::objects::is_form_surface(&uname) && canvas_size_before != Some(rp_comp_get_stored(name, &lprop).to_i64()) {
            rapidr_value::events::post_paint(&uname);
        }
    }
    // A form's new size: it paints again.
    let form_size_changed = form_size_before.is_some_and(|before| before != rp_comp_get_stored(name, &lprop).to_i64()) && !crate::layout_web::is_quiet();
    if form_size_changed {
        // (an MDI form's maximized children follow)
        if rapidr_value::mdi::is_mdi(&uname) {
            crate::mdi_web::resized(&uname);
        }
        rapidr_value::events::post_paint(&uname);
    }
    // A child window's frame shows its title and whether it's active.
    if matches!(lprop.as_str(), "caption" | "active" | "childstate") && rp_comp_type(&uname) == "RMDICHILD" {
        crate::kernel_web::redraw();
    }
    // A tree's image lists: its icons shown again.
    if matches!(lprop.as_str(), "images" | "stateimages") && rapidr_value::objects::is_tree(&uname) {
        crate::kernel_web::tree_refresh(&uname);
    }
    if matches!(lprop.as_str(), "width" | "height") && rapidr_value::objects::is_picture(&uname) {
        store_prop(&uname, "__sized", v_bool(true));
    }
    // A QIMAGE's AutoSize / Stretch / Center.
    if matches!(lprop.as_str(), "autosize" | "stretch" | "center") && rapidr_value::objects::is_picture(&uname) {
        picture_changed(&uname);
    }
    // `CoolBtn.Down = True`: the others of its group come up.
    if lprop == "down" {
        crate::kernel_web::redraw();
    }
    // A QTRACKBAR's size or Enabled: drawn again.
    if matches!(lprop.as_str(), "width" | "height" | "enabled") && rapidr_value::objects::is_trackbar(&uname) {
        crate::kernel_web::redraw();
    }
    // A QTABCONTROL's size, colour, font or Enabled: drawn again.
    if rapidr_value::objects::is_tabcontrol(&uname)
        && matches!(lprop.as_str(), "width" | "height" | "enabled" | "color" | "font" | "fontname" | "fontsize" | "fontbold" | "fontitalic" | "fontcolor" | "font.name" | "font.size" | "font.bold" | "font.italic" | "font.color")
    {
        crate::kernel_web::redraw();
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

/// A property as it is now: the web's own components' (webapi_web), a
/// window's or a web-only element's (kernel_web); Null where only the
/// stored value says.
fn live_prop(name: &str, prop: &str) -> Value {
    let t = rp_comp_type(name);
    if crate::webapi_web::is_webapi(&t) {
        return crate::webapi_web::get_prop(name, &t, prop).unwrap_or_else(v_null);
    }
    crate::kernel_web::get_prop(name, prop)
}

pub fn rp_comp_get(name: &str, prop: &str) -> Value {
    let uname = name.to_uppercase();
    let lprop = prop.to_lowercase();
    // Screen, Application, Clipboard, Mouse (globals_web.rs).
    if let Some(v) = crate::globals_web::get(name, &lprop) {
        return v;
    }
    // (the I/O and media lane's: io_web.rs)
    if let Some((sub, member)) = crate::io_web::sub_component(name, &lprop) {
        return rp_comp_get(&sub, &member);
    }
    if let Some(v) = rapidr_value::objects::rqlib::get(name, &lprop) {
        return v;
    }
    // Form.Scale (RapidR's): the page's scale.
    if lprop == "scale" && rp_comp_type(&uname) == "RFORM" {
        return crate::globals_web::get("screen", "scale").unwrap_or(Value::Double(1.0));
    }
    // A form's inside (its frame and main menu excluded); other components
    // have no frame inside their size.
    if matches!(lprop.as_str(), "clientwidth" | "clientheight") {
        let (w, h) = if matches!(rp_comp_type(&uname).as_str(), "RFORM" | "RSCROLLBOX") {
            crate::scroll_web::client(&uname)
        } else {
            (rp_comp_get(name, "width").to_i64(), rp_comp_get(name, "height").to_i64())
        };
        return v_int(if lprop == "clientwidth" { w } else { h });
    }
    // A component's Handle (rapidr_value::handles).
    if lprop == "handle" && COMPONENTS.with(|c| c.borrow().contains_key(&uname)) {
        return v_int(rapidr_value::handles::handle_of(name));
    }
    // `Constraints.MinWidth` is MinWidth (rapidr_value::layout).
    if let Some(flat) = rapidr_value::layout::constraint_alias(&lprop) {
        return rp_comp_get(name, flat);
    }
    // A QFORMMDI's ChildCount, ChildCaption, … (mdi_web.rs).
    if let Some(v) = rapidr_value::mdi::get(name, &lprop) {
        return v;
    }
    // A QFORM's MDIChildCount, TileMode (form_members_web.rs).
    if matches!(lprop.as_str(), "mdichildcount" | "tilemode") {
        if let Some(v) = crate::form_members_web::get(name, &rp_comp_type(&uname), &lprop) {
            return v;
        }
    }
    // (I1) RapidR Studio's components (studio_web.rs).
    {
        let t = rp_comp_type(name);
        if rapidr_studio::is_studio_type(&t) {
            if let Some(v) = crate::studio_web::get(&t, name, &lprop) {
                return v;
            }
        }
    }
    // (I1) An RDOCKMANAGER's PaneCount, ActiveDocument, … (dock_web.rs).
    if rp_comp_type(name) == "RDOCKMANAGER" {
        if let Some(v) = rapidr_value::dock::runtime::rt_get(name, &lprop) {
            return v;
        }
    }
    // A QFORM's / QSCROLLBOX's AutoScroll, HorzPosition, … (scroll_web.rs).
    if let Some(v) = crate::scroll_web::get(name, &lprop) {
        return v;
    }
    if let Some(v) = rapidr_value::objects::get(name, &lprop) {
        return v;
    }

    // Check data-science / database component properties
    // An RDOM: its element's first
    let comp_type = rp_comp_type(&uname);
    if comp_type == "RDOM" {
        let live = live_prop(&uname, &lprop);
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
        let live = live_prop(&uname, &lprop);
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
            match live_prop(&uname, &lprop) {
                Value::Null if matches!(lprop.as_str(), "left" | "top") => stored.unwrap_or(v_int(0)),
                Value::Null => stored.unwrap_or_else(v_null),
                // (Enabled keeps the number stored, as RapidQ's — `Enabled
                // = -1` reads -1 — while the element agrees)
                live if lprop == "enabled" => match stored {
                    Some(n @ (Value::Integer(_) | Value::Double(_))) if n.to_bool() == live.to_bool() => n,
                    _ => live,
                },
                live => live,
            }
        }
        // (a panel's bevels: RapidQ's defaults until set; Anchors and
        // Constraints: akLeft + akTop, none)
        _ => stored
            .or_else(|| (matches!(rp_comp_type(&uname).as_str(), "RPANEL" | "RBEVEL")).then(|| rapidr_value::objects::bevel::default(&lprop).map(v_int)).flatten())
            .or_else(|| rapidr_value::layout::default_property(&rp_comp_type(&uname), &lprop).map(v_int))
            .unwrap_or_else(v_null),
    }
}

/// `Obj.Member` read without parentheses in a program: the method when the
/// object's type has one by that name (`WHILE MySQL.FetchRow`), else the
/// property (`UpDown.Max`) — rapidr_value::members, as on the desktop.
pub fn rp_comp_value(name: &str, member: &str) -> Value {
    let lower = member.to_ascii_lowercase();
    // (a Boolean reads 1, as RapidQ's: rapidr_value::property_read)
    if rapidr_value::members::is_value_method_name(&lower) && rapidr_value::members::is_value_method(&rp_comp_type(name), &lower) {
        return rapidr_value::property_read(rp_comp_method(name, &lower, &[]));
    }
    rp_comp_read(name, member)
}

/// `Obj.Sub.Prop` read by a program (codegen): as [`rp_comp_value`]'s.
pub fn rp_comp_read(name: &str, prop: &str) -> Value {
    if prop.eq_ignore_ascii_case("color") {
        return program_color(name);
    }
    if (prop.eq_ignore_ascii_case("font.color") || prop.eq_ignore_ascii_case("fontcolor")) && !rp_comp_type(name).is_empty() && !rapidr_value::objects::TYPES.contains(&rp_comp_type(name).as_str()) {
        return program_font_color(name);
    }
    // (Font.Name, Size, Bold …: its own, else its parent's — RapidQ's ParentFont —
    // else MS Sans Serif 8, as RC.EXE reads them)
    if let Some(flat) = rapidr_value::objects::font_flat_name(prop) {
        let t = rp_comp_type(name);
        if !t.is_empty() && !rapidr_value::objects::TYPES.contains(&t.as_str()) {
            return rapidr_value::property_read(rapidr_value::objects::inherited_font_prop(name, flat, &|i, p| rp_comp_get(i, p)));
        }
    }
    // (a property the theme draws while unset reads the registry's default:
    // rapidr_value::component_defaults::unset_read)
    let v = rp_comp_get(name, prop);
    let v = match v {
        Value::Null => rapidr_value::component_defaults::unset_read(&rp_comp_type(name), prop).unwrap_or(Value::Null),
        v => v,
    };
    rapidr_value::property_read(v)
}

/// `x = Obj.Method(…)` in a program: the method's result as RapidQ gives it
/// (a Boolean reads 1, `rapidr_value::property_read`).
pub fn rp_comp_call(name: &str, method: &str, args: &[Value]) -> Value {
    rapidr_value::property_read(rp_comp_method(name, method, args))
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
    // HideTitleBar, ShapeForm, QFORM's MDI methods, StartDrag (form_members_web.rs).
    if let Some(v) = crate::form_members_web::method(name, &rp_comp_type(&uname), &lmethod, args) {
        return v;
    }
    // `Label.Font.AddStyles(fsBold)` / `DelStyles`: the component's styles.
    if let Some(changes) = rapidr_value::objects::font::component_style_call(&lmethod, args) {
        for (p, v) in changes {
            rp_comp_set(name, p, v);
        }
        return v_null();
    }
    // (the I/O and media lane's: io_web.rs)
    if let Some((sub, member)) = crate::io_web::sub_component(name, &lmethod) {
        return rp_comp_method(&sub, &member, args);
    }
    if rapidr_value::objects::rqlib::exists(name) {
        return crate::io_web::method(name, &lmethod, args);
    }
    // A file dialog's Files(i): the folder (0), then the picked names.
    if lmethod == "files" && matches!(rp_comp_type(name).as_str(), "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG") {
        let i = args.first().map_or(0, Value::to_i64);
        let v = rp_comp_get_stored(name, &format!("files({i})"));
        return if matches!(v, Value::Null) { v_str("") } else { v };
    }
    // A QFORMMDI's AddChild, CascadeChild, … (mdi_web.rs).
    if rapidr_value::mdi::is_mdi(name) {
        if let Some(v) = crate::mdi_web::method(name, &lmethod, args) {
            return v;
        }
    }
    // (I1) RapidR Studio's RPROJECT, RLANGUAGESERVICE, RPROGRAMSESSION (studio_web.rs).
    {
        let t = rp_comp_type(name);
        if rapidr_studio::is_studio_type(&t) {
            if let Some(v) = crate::studio_web::call(&t, name, &method.to_ascii_lowercase(), args) {
                return v;
            }
        }
    }
    // (I1) An RDOCKMANAGER's AddPane, SaveLayout, … (dock_web.rs).
    if rp_comp_type(name) == "RDOCKMANAGER" {
        if let Some(v) = crate::dock_web::method(name, &lmethod, args) {
            return v;
        }
    }

    // Indexed sub-objects (`SB.Panel(0).Width = 100` → method
    // `panel.width=` with (0, 100); reading → `panel.width` with (0)): kept
    // as the component's properties `panel(0).width` unless the component
    // implements them.
    // `Form.Pixel(x, y)` read: RapidQ's -1s and its children's pixels.
    if rp_comp_type(name) == "RFORM" && args.len() == 2 && lmethod.as_str() == "pixel" {
        if let Some(v) = form_pixel(name, args) {
            return v;
        }
    }
    // A QFORM gets its own drawing surface the first time it's drawn on.
    if rp_comp_type(name) == "RFORM"
        && rapidr_value::objects::is_drawing_method(&lmethod)
        && !(lmethod == "paint" && args.len() < 3)
        && !rapidr_value::objects::is_form_surface(name)
    {
        rapidr_value::objects::create_form_surface(name, rapidr_value::objects::form_color(&rp_comp_get_stored(name, "color")));
    }
    // `PopupMenu.Popup(X, Y)`.
    if lmethod == "popup" && rapidr_value::objects::menu::kind(name) == Some(rapidr_value::objects::menu::Kind::Popup) {
        crate::kernel_web::popup(name, args.first().map_or(0, Value::to_i64), args.get(1).map_or(0, Value::to_i64));
        return v_null();
    }
    if rapidr_value::objects::menu::is_menu(name) {
        crate::kernel_web::redraw();
    }
    // QEDIT / QRICHEDIT: the element's text first; then Copy/Cut/Paste with
    // the clipboard, Line(i), AddStrings, … on the model, shown again.
    if rapidr_value::objects::is_textedit(name) {
        let clip = rapidr_value::objects::textedit_clipboard(
            name,
            &lmethod,
            &|| crate::globals_web::get("clipboard", "text").map(|v| v.to_string_val()).unwrap_or_default(),
            &|s| {
                crate::globals_web::set("clipboard", "text", &v_str(s));
            },
        );
        let result = clip.map(Ok).or_else(|| rapidr_value::objects::call(name, &lmethod, args, &|id, p| rp_comp_get(id, p)));
        if let Some(result) = result {
            crate::kernel_web::redraw();
            return result.unwrap_or_else(|e| {
                object_error(name, method, &e);
                v_null()
            });
        }
    }
    if let Some(result) = rapidr_value::objects::call(name, &lmethod, args, &|id, p| rp_comp_get(id, p)) {
        if rapidr_value::objects::is_picture(name) {
            // `Image.LoadFromFile "photo.png"`: not a BMP — nothing loaded,
            // as on the desktop.
            if result.is_err() && matches!(lmethod.as_str(), "loadfromfile" | "load") {
                not_a_bmp(&args.first().map(|v| v.to_string_val()).unwrap_or_default());
                return v_null();
            }
            picture_changed(&uname);
        }
        // `ImageList.Draw Target, X, Y, Index`: a target QIMAGE shows it.
        if let Some(target) = args.first().map(Value::to_string_val).filter(|t| lmethod == "draw" && rapidr_value::objects::is_picture(t)) {
            picture_changed(&target.to_uppercase());
        }
        // (drawn again: a tree's rows built again first)
        if rapidr_value::objects::is_tree(name) {
            crate::kernel_web::tree_refresh(&uname);
        } else {
            crate::kernel_web::redraw();
        }
        return result.unwrap_or_else(|e| {
            object_error(name, method, &e);
            v_null()
        });
    }
    if let Some(v) = indexed_sub_object(name, &lmethod, args) {
        return v;
    }
    // A QSTATUSBAR's AddPanels / Clear (rapidr_value::statusbar).
    if rp_comp_type(&uname) == "RSTATUSBAR" {
        let get = |p: &str| rp_comp_get_stored(name, p);
        let mut set = |p: &str, v: Value| rp_comp_set(name, p, v);
        if let Some(v) = rapidr_value::statusbar::call(&lmethod, args, &get, &mut set) {
            return v;
        }
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
        "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG"
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

    // The web's own components without a window (RJAVASCRIPT, RWEBSTORAGE
    // …): theirs first — a name a form's method has too (Show) is theirs.
    if let Some(v) = crate::webapi_web::method(&uname, &comp_type, &lmethod, args) {
        return v;
    }
    // `Image.LoadFromPlot Plot`: the chart's pixels become the picture, as
    // the desktop's.
    if lmethod == "loadfromplot" && rapidr_value::objects::is_picture(&uname) {
        image_from_plot(&uname, &args.first().map(Value::to_string_val).unwrap_or_default());
        return v_null();
    }
    // The windows' (the UI kernel's) and the web-only elements' methods.
    if let Some(v) = crate::kernel_web::method(&uname, &comp_type, &lmethod, args) {
        return v;
    }
    web_sys::console::error_1(&JsValue::from_str(&format!(
        "[RapidR][NotImplemented] {name}.{method}() — method not implemented on web runtime (component type: {comp_type}). This call will return Null. Native target may support it."
    )));
    v_null()
}

/// A QIMAGE's picture from plot `plot`'s chart, its size with AutoSize, as
/// the desktop's `LoadFromPlot`: the chart's pixels, drawn again at the
/// page's scale for the screen (datascience_web.rs).
fn image_from_plot(name: &str, plot: &str) {
    let Some((w, h)) = crate::datascience_web::load_into_picture(name, plot) else { return };
    if rp_comp_get(name, "stretch").to_i64() == 0 && rp_comp_get(name, "autosize").to_bool() {
        rp_comp_set(name, "width", v_int(w));
        rp_comp_set(name, "height", v_int(h));
    }
    picture_changed(name);
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
        // The page's files (as OPEN's and EXTRACTRESOURCE's: saved this
        // session, else the project's), as the desktop's are on disk.
        "loadfile" => {
            let filename = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let Ok(text) = web_read_file(&filename).map(|b| String::from_utf8_lossy(&b).into_owned()) else {
                object_error(name, "LoadFile", &format!("can't read {filename}"));
                return v_int(0);
            };
            if js_sys::JSON::parse(&text).is_err() {
                object_error(name, "LoadFile", &format!("{filename} isn't JSON"));
                return v_int(0);
            }
            rp_comp_set(name, "filename", v_str(&filename));
            rp_comp_set(name, "text", v_str(&text));
            JSON_WEB_STORES.with(|s| s.borrow_mut().insert(name_lower, text));
            v_int(1)
        }
        "savefile" => {
            let filename = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            // (pretty-printed, as the desktop writes it)
            let text = JSON_WEB_STORES.with(|s| s.borrow().get(&name_lower).cloned()).unwrap_or_else(|| "{}".into());
            let pretty = js_sys::JSON::parse(&text)
                .ok()
                .and_then(|v| js_sys::JSON::stringify_with_replacer_and_space(&v, &wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::from_f64(2.0)).ok())
                .and_then(|s| s.as_string())
                .unwrap_or(text);
            match web_write_file(&filename, pretty.as_bytes()) {
                Ok(()) => {
                    rp_comp_set(name, "filename", v_str(&filename));
                    v_int(1)
                }
                Err(e) => {
                    object_error(name, "SaveFile", &e);
                    v_int(0)
                }
            }
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
    let _ = js_sys::Reflect::set(&current, &last_key, &json_web_value(val));
}

/// A value set into JSON as the desktop's RJSON sets it
/// (rapidr-runtime-core's value_to_json): text that is a number becomes a
/// number, "true" / "false" a boolean, anything else a string.
fn json_web_value(val: &Value) -> wasm_bindgen::JsValue {
    let s = val.to_string_val();
    match s.parse::<f64>() {
        Ok(f) if f.is_finite() => wasm_bindgen::JsValue::from_f64(f),
        _ if s == "true" => wasm_bindgen::JsValue::TRUE,
        _ if s == "false" => wasm_bindgen::JsValue::FALSE,
        _ => wasm_bindgen::JsValue::from_str(&s),
    }
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
            let doc = crate::page_web::document();
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

/// Open / Save where the program can't wait for the kernel's (a Rust-built
/// page): the browser's prompt asks for a name; the answer in FileName,
/// FileTitle, Files(…), SelCount (rapidr_value::file_dialog).
fn web_file_dialog(name: &str, save: bool) -> Value {
    use rapidr_value::file_dialog as fd;
    let prop = |p: &str| rp_comp_get_stored(name, p).to_string_val();
    let default_ext = prop("defaultext");
    let owner = name.to_string();
    let answer = move |names: Vec<String>| {
        let mut names = names;
        if save {
            if let Some(first) = names.first_mut() {
                *first = fd::with_default_ext(first, &default_ext);
            }
        }
        if names.is_empty() {
            return;
        }
        let picked = fd::picked(&names);
        rp_comp_set_prop_only(&owner, "filename", v_str(&picked.file_name));
        rp_comp_set_prop_only(&owner, "filetitle", v_str(&picked.file_title));
        rp_comp_set_prop_only(&owner, "selcount", v_int(picked.sel_count));
        for (i, f) in picked.files.iter().enumerate() {
            rp_comp_set_prop_only(&owner, &format!("files({i})"), v_str(f));
        }
    };
    let chosen = web_sys::window().and_then(|w| w.prompt_with_message_and_default(if save { "Save as:" } else { "Open file:" }, &prop("filename")).ok().flatten());
    let names: Vec<String> = chosen.map(|c| c.split(';').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect()).unwrap_or_default();
    let picked = !names.is_empty();
    answer(names);
    v_int(if picked { -1 } else { 0 })
}

fn dialog_web_method(name: &str, comp_type: &str, method: &str, args: &[Value]) -> Value {
    // (the colour, font, Open and Save dialogs are the kernel's — the
    // desktop's — a wait the VM serves: rapidr_ui_app::dialogs::execute;
    // a Rust-built page, which can't wait, gets the browser's below)
    if method == "execute" {
        if let Some(v) = crate::kernel_web::execute(name, comp_type) {
            return v;
        }
    }
    // (the dialogs lane's) A QCOLORDIALOG's Colors(i), 1 to 16: read, or
    // `Colors(i) = c` (its second argument), as on the desktop.
    if method == "colors" && comp_type == "RCOLORDIALOG" {
        let key = format!("colors({})", args.first().map_or(0, Value::to_i64));
        if let Some(c) = args.get(1) {
            rp_comp_set_prop_only(name, &key, v_int(c.to_i64() & 0xFF_FFFF));
            return v_null();
        }
        return match rp_comp_get_stored(name, &key) {
            Value::Null => v_int(0),
            v => v,
        };
    }
    // A QFONTDIALOG's AddStyles, DelStyles, AddOptions, DelOptions,
    // GetFont(F), SetFont(F), FontName(i).
    if comp_type == "RFONTDIALOG" {
        let get = |p: &str| rp_comp_get_stored(name, p);
        let mut set = |p: &str, v: Value| rp_comp_set(name, p, v);
        if let Some(v) = rapidr_value::font_dialog::call(method, args, &get, &mut set) {
            return v;
        }
    }
    if method != "execute" {
        return v_null();
    }
    match comp_type {
        "ROPENDIALOG" => web_file_dialog(name, false),
        "RSAVEDIALOG" => web_file_dialog(name, true),
        "RFILEDIALOG" => web_file_dialog(name, rp_comp_get_stored(name, "mode").to_i64() == 1),
        // (a Rust-built page can't wait: the browser's colour input)
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
            let doc = crate::page_web::document();
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
            // (a Rust-built page can't wait: the browser's prompt() for the
            // name and the size)
            let req = rapidr_value::font_dialog::request(&|p| rp_comp_get_stored(name, p));
            if let Some(window) = web_sys::window() {
                if let Ok(Some(fname)) = window.prompt_with_message_and_default("Font name:", &req.font.name) {
                    if let Ok(Some(fsize)) = window.prompt_with_message_and_default("Font size (pt):", &req.font.size.to_string()) {
                        let mut font = req.font.clone();
                        font.name = fname;
                        if let Ok(n) = fsize.parse::<i64>() {
                            font.size = n;
                        }
                        for (p, v) in rapidr_value::font_dialog::properties(&font) {
                            rp_comp_set_prop_only(name, p, v);
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
    crate::kernel_web::ended();
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
    if event == "onclick" {
        button_modal_result(name);
    }
}

/// The form a component is on (itself for a form).
pub fn form_of(name: &str) -> Option<String> {
    let mut cur = name.to_uppercase();
    for _ in 0..32 {
        if rp_comp_type(&cur) == "RFORM" {
            return Some(cur);
        }
        let p = rp_comp_get_stored(&cur, "parent").to_string_val().to_uppercase();
        if p.is_empty() {
            return None;
        }
        cur = p;
    }
    None
}

/// A button with a ModalResult (or Kind bkClose) clicked: its form gets that
/// result, which closes it when it's shown modally (as the desktop).
fn button_modal_result(name: &str) {
    if rp_comp_type(name) == "RFORM" {
        return;
    }
    let mr = rp_comp_get(name, "modalresult").to_i64();
    let close = rp_comp_get(name, "kind").to_i64() == 6;
    if mr == 0 && !close {
        return;
    }
    if let Some(form) = form_of(name) {
        if mr != 0 {
            rp_comp_set(&form, "modalresult", v_int(mr));
        } else {
            crate::kernel_web::close_form(&form);
        }
    }
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
/// Stores a property without any of `rp_comp_set`'s effects.
fn store_prop(name: &str, prop: &str, val: Value) {
    COMPONENTS.with(|c| {
        if let Some(comp) = c.borrow_mut().get_mut(&name.to_uppercase()) {
            comp.properties.insert(prop.to_lowercase(), val);
        }
    });
}

fn picture_changed(name: &str) {
    // (AutoSize; or a picture loaded into a new QIMAGE, whose size the
    // program hasn't set: the picture's, as in RapidQ)
    if rp_comp_get_stored(name, "autosize").to_bool() || !rp_comp_get_stored(name, "__sized").to_bool() {
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
    crate::kernel_web::redraw();
}

/// A QIMAGE asked for an image file other than a BMP (PNG, JPEG, …): not
/// loaded, as the desktop says.
fn not_a_bmp(path: &str) {
    web_sys::console::warn_1(&JsValue::from_str(&format!("[WARN] RImage: could not load '{path}'")));
}

/// A QDIRTREE's Directory (to see whether a store changed it).
fn rp_comp_get_stored_dir(name: &str) -> String {
    rapidr_value::objects::get(name, "directory").map(|v| v.to_string_val()).unwrap_or_default()
}

thread_local! {
    /// (component, event) pairs with a DOM listener ([`bind_dom_event`]).
    static DOM_BOUND: RefCell<std::collections::HashSet<(String, String)>> = RefCell::new(std::collections::HashSet::new());
}

fn bind_dom_event(name: &str, event: &str) {
    // (every binding comes here: OnStartDrag makes a drag source, OnHint
    // the hints' receiver)
    rapidr_value::events::bound(name, event);
    let id = format!("rr-{}", name.to_lowercase());
    let name_owned = name.to_string();
    let event_owned = event.to_string();

    // Timer events are handled specially — they don't need DOM binding
    // (nor a QDXJOYSTICK's: looked for at its ticks)
    if event == "ontimer"
        || (rp_comp_type(name) == "RDXJOYSTICK" && rapidr_value::objects::joystick::EVENTS.contains(&event))
        || (rp_comp_type(name) == "RCOMPORT" && rapidr_value::objects::rqlib::look_events().contains(&event))
        || rapidr_value::objects::rqlib::media_timer(name).is_some()
    {
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

    // (the UI kernel fires the events of what it draws; the page's own
    // elements are only the web-only components': overlay_web)
    if !crate::overlay_web::is_overlay(&rp_comp_type(name)) {
        return;
    }

    let doc = web_sys::window().unwrap().document().unwrap();
    let el = match doc.get_element_by_id(&id) {
        Some(e) => e,
        None => return, // Virtual components (Timer, etc.) don't have DOM elements
    };

    let dom_event_name = match event {
        "onclick" => "click",
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
    // One listener per component and event: a handler given again (a
    // TYPE's EVENT, then the program's own) must not run twice — the
    // listener fires whichever handler is current.
    if !DOM_BOUND.with(|b| b.borrow_mut().insert((name.to_uppercase(), event.to_string()))) {
        return;
    }

    // Create a JavaScript closure that fires the RapidR event
    let name_for_closure = name_owned.clone();
    let event_for_closure = event_owned.clone();

    // Key events (rapidr_value::input): OnKeyDown / OnKeyUp (Key, Shift)
    // and OnKeyPress (Key) for a key that types.
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
            let rect = el.get_bounding_client_rect();
            let x = (e.client_x() as f64 - rect.left()) as i64;
            let y = (e.client_y() as f64 - rect.top()) as i64;
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

pub(crate) fn update_timer(name: &str) {
    let uname = name.to_uppercase();
    // (QTIMER, QDXTIMER and QDXJOYSTICK tick in the app's timer heap, as on
    // the desktop; the I/O lane's devices on the page's intervals)
    if matches!(rp_comp_type(&uname).as_str(), "RTIMER" | "RDXTIMER" | "RDXJOYSTICK") {
        return crate::kernel_web::timer_changed(&uname);
    }

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

    // Check if we have an ontimer event handler registered (a QDXJOYSTICK:
    // one of its events')
    let events: &[&str] = match rp_comp_type(&uname).as_str() {
        "RDXJOYSTICK" => &rapidr_value::objects::joystick::EVENTS,
        "RCOMPORT" => rapidr_value::objects::rqlib::look_events(),
        _ => &["ontimer"],
    };
    let has_handler = EVENT_HANDLERS.with(|eh| {
        let eh = eh.borrow();
        events.iter().any(|e| eh.contains_key(&(uname.clone(), e.to_string())))
    });
    // (the I/O lane's media objects: their Timer is their model's, and ticks
    // with or without an OnChange — a play's end is noticed there)
    let media = rapidr_value::objects::rqlib::media_timer(&uname);
    let (enabled, interval, has_handler) = match media {
        Some((i, on)) => (on, i, true),
        None => (enabled, interval, has_handler),
    };

    // (the DirectX lane's: a QDXTIMER's Interval 0 is a screen refresh)
    let interval = crate::directx_web::timer_interval(&uname, interval);
    if enabled && has_handler && interval > 0 {
        let name_for_closure = uname.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            if crate::directx_web::timer_fired(&name_for_closure) {
                rp_fire_event(&name_for_closure, "ontimer");
            }
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

/// Whether a type name is a component the compilers create, and whether a
/// member is some component's method: the language registry's
/// (crates/rapidr-lang), as the desktop runtime's.
pub use rapidr_lang::{is_component_method, is_component_type};

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
// Theme — Application.Theme: the UI kernel draws in it (globals_web)
// ---------------------------------------------------------------------------

pub fn set_theme(theme: &str) {
    crate::globals_web::name_theme(theme);
}

pub fn gui_register_timer(name: &str) {
    // (the app's timer heap, as the desktop's: rapidr_ui_app::timers)
    crate::kernel_web::register_timer(name);
}

/// The program's timers (QTIMER, QDXTIMER, QDXJOYSTICK …): what a modal
/// form's end stops, as the desktop's `rp_stop_all_timers`.
pub fn timer_names() -> Vec<String> {
    COMPONENTS.with(|c| c.borrow().iter().filter(|(_, comp)| matches!(comp.type_name.as_str(), "RTIMER" | "RDXTIMER" | "RDXJOYSTICK" | "RCOMPORT" | "RMIDI" | "RWAVE" | "RVIDEO" | "RCDAUDIO")).map(|(n, _)| n.clone()).collect())
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
    let doc = crate::page_web::document();
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
