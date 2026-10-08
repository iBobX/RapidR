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
        // What every runtime gives it: the language registry's defaults,
        // RapidQ's as RC.EXE reads them (rapidr_value::component_defaults).
        let mut props: HashMap<String, Value> = rapidr_value::component_defaults::creation(type_name).into_iter().collect();
        let tn = type_name.to_uppercase();
        // Its size: RapidQ's, the same on every runtime (rapidr_value::layout).
        if let Some((w, h)) = rapidr_value::layout::default_size(&tn) {
            props.insert("width".into(), v_int(w));
            props.insert("height".into(), v_int(h));
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
    /// How many handlers have been queued for the bytecode VM.
    static VM_QUEUED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many event handlers have been queued for the bytecode VM so far (it
/// runs them after the host operation or the wait's turn that fired them).
/// A native build runs its handlers as they're fired: always 0.
pub fn rp_vm_events_queued() -> u64 {
    VM_QUEUED.with(std::cell::Cell::get)
}

/// Install a thread-local indirect event dispatcher. Used by the
/// bytecode VM's NativeHost so that the desktop host's events can reach
/// the VM and invoke a bytecode function by index.
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
            VM_QUEUED.with(|q| q.set(q.get() + 1));
            d(handler_id, args);
        } else if !SHUTTING_DOWN.with(|s| s.get()) {
            // After the program ends and timers/widgets are torn
            // down the host may still deliver a few queued events.
            // Suppress the noisy warning during shutdown — it is harmless.
            eprintln!(
                "[rapidr] event handler #{handler_id} fired but no indirect dispatcher is registered"
            );
        }
    });
}

thread_local! {
    static SHUTTING_DOWN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Mark the runtime as shutting down so that late-firing timer ticks
/// (which can be queued after the dispatcher has been uninstalled) don't
/// print a noisy "no dispatcher" warning.
pub fn rp_mark_shutting_down() {
    SHUTTING_DOWN.with(|s| s.set(true));
}

/// A timer the runtime ticks (QTIMER, and the DirectX lane's QDXTIMER and
/// QDXJOYSTICK — its events looked for at each tick).
fn is_timer_type(type_name: &str) -> bool {
    matches!(type_name.to_ascii_uppercase().as_str(), "RTIMER" | "RDXTIMER" | "RDXJOYSTICK" | "RCOMPORT" | "RMIDI" | "RWAVE" | "RVIDEO" | "RCDAUDIO")
}

/// Disable all RTimer components and clear their indirect handlers so
/// that no further timer ticks attempt to dispatch into a torn down
/// VM. Called during ShowModal shutdown and from
/// `rp_set_event_dispatcher(None)` so the runtime stays well-behaved.
pub fn rp_stop_all_timers() {
    let timers: Vec<String> = COMPONENTS.with(|c| {
        c.borrow()
            .iter()
            .filter_map(|(n, comp)| {
                if is_timer_type(&comp.type_name) {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect()
    });
    for name in timers {
        // Setting enabled=0 makes the host's timer skip `rp_fire_event`
        // and not re-arm (Enabled is read again at each tick) — the timer
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
    // (the DirectX lane's: QDXSOUND plays on the sound device)
    if type_name.eq_ignore_ascii_case("RDXSOUND") {
        crate::sound::install_dx_device();
    }
    // (QDXJOYSTICK: its gamepads' source)
    if type_name.eq_ignore_ascii_case("RDXJOYSTICK") {
        crate::joystick::install();
    }
    // (the I/O and media lane's: their devices, a QDOWNLOAD's gauge)
    if rapidr_value::objects::rqlib::is_type(type_name) {
        crate::io::created(name, type_name);
    }
    // (I4) a design surface reads its Source with the designer (rapidr-studio)
    #[cfg(feature = "studio")]
    if type_name.eq_ignore_ascii_case("RDESIGNSURFACE") {
        rapidr_studio::design::install();
    }
}

/// (I4) What an RDESIGNSURFACE's call left to hear (OnSourceEdit, OnChange,
/// OnSelect …), fired.
fn design_events(name: &str) {
    for e in rapidr_value::objects::take_design_events(name) {
        rp_fire_event_args(name, e.event(), &e.args());
    }
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
    // (a Color or a Parent changed: the canvases' backdrops follow)
    let backdrops = prop.eq_ignore_ascii_case("color") || prop.eq_ignore_ascii_case("parent");
    // (an AutoSize QLABEL's Caption, WordWrap, AutoSize, font or Parent: it
    // takes its text's size — layout.rs, rapidr_value::autosize)
    let labels = crate::layout::labels_before_set(name, prop);
    // (the program chose a Font.Color — or a whole Font: RapidQ's
    // ParentFont no longer applies, rapidr_value::component_defaults::
    // font_color_read)
    if matches!(prop.to_ascii_lowercase().as_str(), "font.color" | "fontcolor" | "font") {
        COMPONENTS.with(|c| {
            if let Some(comp) = c.borrow_mut().get_mut(&name.to_lowercase()) {
                comp.properties.insert("__fontcolorset".into(), v_bool(true));
            }
        });
    }
    set_property(name, prop, val);
    if let Some(before) = labels {
        crate::layout::labels_after_set(name, before);
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
            #[cfg(feature = "gui")]
            crate::ui::canvas_redraw(&c);
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
            if rp_comp_get(name, "caption").to_string_val().is_empty() || (1..=10).any(|k| rapidr_value::events::button_kind(k).map(|(c, _)| c) == Some(rp_comp_get(name, "caption").to_string_val().as_str())) {
                rp_comp_set(name, "caption", v_str(caption));
            }
            rp_comp_set(name, "modalresult", v_int(mr));
        }
    }
    // A modal form's ModalResult set: the form closes (ShowModal returns it).
    if prop.eq_ignore_ascii_case("modalresult") && val.to_i64() != 0 && rp_comp_type(name) == "RFORM" {
        #[cfg(feature = "gui")]
        if crate::ui::is_modal(name) {
            let (n, v) = (name.to_string(), val.clone());
            store_prop(&n, "modalresult", v);
            crate::ui::gui_close(&n);
            return;
        }
    }
    let prop_lower = prop.to_lowercase();
    // Screen, Application, Clipboard, Mouse (globals.rs).
    if crate::globals::set(name, &prop_lower, &val) {
        return;
    }
    // (the I/O and media lane's: a QDOWNLOAD's StateGauge / SpeedLbl, and
    // these objects' own properties — io.rs)
    if let Some((sub, member)) = crate::io::sub_component(name, &prop_lower) {
        return rp_comp_set(&sub, &member, val);
    }
    if rapidr_value::objects::rqlib::exists(name) {
        if let Some(Ok(())) = rapidr_value::objects::rqlib::set(name, &prop_lower, &val) {
            crate::io::fire_events(name);
            return;
        }
    }
    // A QDIGDISPLAY is as big as its Display (QDigDisplay.inc sizes it so).
    let val = match rapidr_value::objects::digdisplay_text(name) {
        Some(text) if matches!(prop_lower.as_str(), "width" | "height") => {
            let (w, h) = rapidr_value::objects::digdisplay::size(&text);
            v_int(if prop_lower == "width" { w } else { h })
        }
        _ => val,
    };
    // A QGLASSFRAME's Transparency, TransparentColor, Moveable: as RC.EXE
    // stores them (rapidr_value::objects::glass).
    let val = if rp_comp_type(name) == "RGLASSFRAME" { rapidr_value::objects::glass::stored(&prop_lower, &val).unwrap_or(val) } else { val };
    // A QBEVEL's Shape / Style set its bevels (QBevel.inc's setters).
    if rp_comp_type(name) == "RBEVEL" {
        let other = rp_comp_get(name, if prop_lower == "shape" { "style" } else { "shape" }).to_i64();
        if let Some(bevels) = rapidr_value::objects::bevel::qbevel_set(&prop_lower, val.to_i64(), other) {
            for (p, v) in bevels {
                store_prop(name, p, v_int(v));
            }
        }
    }
    // A QFORM's / QSCROLLBOX's AutoScroll, HorzPosition, … (scroll.rs).
    if crate::scroll::set(name, &prop_lower, &val) {
        return;
    }
    // A QFORMMDI's ChildMax, ChildCaption, ChildState, … (mdi.rs).
    if rapidr_value::mdi::is_mdi(name) && crate::mdi::set(name, &prop_lower, &val) {
        return;
    }
    // (I1) RapidR Studio's RPROJECT, RLANGUAGESERVICE, RPROGRAMSESSION (studio.rs).
    #[cfg(feature = "studio")]
    if rapidr_studio::is_studio_type(&rp_comp_type(name)) && crate::studio::set(&rp_comp_type(name), name, &prop_lower, &val) {
        return;
    }
    // (I1) An RDOCKMANAGER's DocumentMode, ActiveDocument, … (dock.rs).
    if rp_comp_type(name) == "RDOCKMANAGER" && crate::dock::set(name, &prop_lower, &val) {
        return;
    }
    // (I1 / L-PANELS) A panel's Target, Filter, Page, … (panels.rs).
    if rapidr_value::panels::is_panel(&rp_comp_type(name)) && crate::panels::set(name, &prop_lower, &val) {
        return;
    }
    // (the dialogs lane's) A QFONTDIALOG's Name / Size / Color are its flat
    // FontName / FontSize / FontColor too: one value.
    if let Some(other) = rapidr_value::font_dialog::alias(&prop_lower).filter(|_| rp_comp_type(name) == "RFONTDIALOG") {
        store_prop(name, other, val.clone());
    }
    // A form's size before (it paints again only when it changes).
    let form_size_before = (matches!(prop_lower.as_str(), "width" | "height") && rp_comp_type(name) == "RFORM").then(|| rp_comp_get(name, &prop_lower).to_i64());
    // (the WindowState lane's) A form's WindowState before (its window
    // follows: maximized, minimized, restored); kept as wsNormal …
    // wsMaximized.
    let state_before = (prop_lower == "windowstate" && rp_comp_type(name) == "RFORM").then(|| rp_comp_get(name, "windowstate").to_i64());
    let val = if state_before.is_some() { v_int(rapidr_value::window_state::of(val.to_i64())) } else { val };
    // A canvas's size before (it paints again only when it changes: a
    // library setting its size in its own OnPaint mustn't loop).
    #[cfg_attr(not(feature = "gui"), allow(unused_variables))]
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
    // Constraints (RapidR, from Delphi; rapidr_value::layout): MinWidth …
    // MaxHeight — `Constraints.MinWidth` is MinWidth — bound every size the
    // component takes; setting one brings its size within them.
    if let Some(flat) = rapidr_value::layout::constraint_alias(&prop_lower) {
        return rp_comp_set(name, flat, val);
    }
    let real = || !matches!(rp_comp_type(name).as_str(), "" | "RUDT");
    let val = match val {
        v if matches!(prop_lower.as_str(), "width" | "height") && real() => {
            let k = crate::layout::constraints_of(name);
            if k.is_none() {
                v
            } else {
                v_int(if prop_lower == "width" { k.width(v.to_i64()) } else { k.height(v.to_i64()) })
            }
        }
        v => v,
    };
    if rapidr_value::layout::CONSTRAINT_PROPERTIES.contains(&prop_lower.as_str()) && real() {
        if let Some(k) = crate::layout::constraints_of(name).with(&prop_lower, val.to_i64()) {
            for (p, v) in k.properties() {
                store_prop(name, p, v_int(v));
            }
            for p in ["width", "height"] {
                let size = rp_comp_get(name, p);
                if !matches!(size, Value::Null) {
                    let bounded = if p == "width" { k.width(size.to_i64()) } else { k.height(size.to_i64()) };
                    if bounded != size.to_i64() {
                        rp_comp_set(name, p, v_int(bounded));
                    }
                }
            }
            // (a form: the user can't drag it outside them)
            #[cfg(feature = "gui")]
            if rp_comp_type(name) == "RFORM" {
                crate::ui::gui_apply_geometry(name);
            }
            return;
        }
    }

    // QFONT, QMEMORYSTREAM, QBITMAP, QIMAGELIST, QLISTVIEW's data (shared
    // with the web runtime).
    let before_dir = if rapidr_value::objects::is_dirtree(name) { rp_comp_get(name, "directory").to_string_val() } else { String::new() };
    // (a menu's change shows once the program's code returns)
    #[cfg(feature = "gui")]
    if rapidr_value::objects::menu::is_menu(name) {
        crate::ui::schedule_menu_sync();
    }
    if let Some(result) = rapidr_value::objects::set(name, &prop_lower, &val) {
        let picture = rapidr_value::objects::is_picture(name);
        match result {
            // `Image.BMP = "photo.png"`: not a BMP; the host loads it.
            Err(_) if picture && prop_lower == "bmp" => {
                rp_comp_set(name, "__imagefile", val.clone());
                #[cfg(feature = "gui")]
                crate::ui::image_method(name, "loadfromfile", std::slice::from_ref(&val));
            }
            Err(e) => eprintln!("[rapidr] {name}.{prop}: {e}"),
            Ok(()) => {}
        }
        // A QDIGDISPLAY's new Display: its size (the control drawn again below).
        if prop_lower == "display" {
            if let Some(text) = rapidr_value::objects::digdisplay_text(name) {
                let (w, h) = rapidr_value::objects::digdisplay::size(&text);
                rp_comp_set(name, "width", v_int(w));
                rp_comp_set(name, "height", v_int(h));
            }
        }
        if picture {
            picture_changed(name);
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_canvas(name) || rapidr_value::objects::is_trackbar(name) || rapidr_value::objects::is_design(name) || rapidr_value::objects::is_diff(name) {
            crate::ui::redraw_widget(name);
        }
        if rapidr_value::objects::is_design(name) {
            design_events(name);
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_tabcontrol(name) {
            crate::ui::tab_control_changed(name);
        }
        // A QFILELISTBOX's directory changed: OnChange.
        if prop_lower == "directory" && rapidr_value::objects::is_file_list(name) {
            rp_fire_event(name, "onchange");
        }
        // A QDIRTREE: shown again; its directory changed: OnChange.
        if rapidr_value::objects::is_dirtree(name) {
            #[cfg(feature = "gui")]
            crate::ui::dirtree_refresh(name);
            if matches!(prop_lower.as_str(), "directory" | "initialdir") && before_dir != rp_comp_get(name, "directory").to_string_val() {
                rp_fire_event(name, "onchange");
            }
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_textedit(name) {
            crate::ui::text_push(name);
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_tree(name) {
            crate::ui::tree_refresh(name);
        } else if rapidr_value::objects::is_listview(name) {
            crate::ui::listview_refresh(name);
        } else if rapidr_value::objects::is_grid(name) {
            crate::ui::grid_refresh(name);
        } else if rapidr_value::objects::is_list(name) {
            crate::ui::list_refresh(name);
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
        // (I1 / L-PANELS: the inspector's font parts)
        ("font.underline", "fontunderline"),
        ("font.strikeout", "fontstrikeout"),
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
        // Update visible widgets when "visible" changes
        if prop_lower == "visible" {
            let v = match &val {
                Value::Boolean(b) => *b,
                Value::Integer(i) => *i != 0,
                _ => true,
            };
            // A window's `Visible = True` is its Show (OnShow included); a
            // form inside another (an MDI child's) is only drawn or not.
            if v && is_window(name) {
                crate::ui::gui_show_visible(name);
            } else {
                crate::ui::gui_set_visible(name, v);
            }
        }
        // A status bar redraws its panels / simple text.
        if comp_type == "RSTATUSBAR" && (prop_lower.starts_with("panel") || prop_lower.starts_with("simple")) {
            crate::ui::gui_redraw(name);
        }
        // Update caption/simpletext on the widget
        else if prop_lower == "caption" || prop_lower == "simpletext" {
            crate::ui::gui_set_caption(name, &val.to_string_val());
        }
        // Update text on TextEditor/CodeEditor
        if prop_lower == "text" && (comp_type == "RCODEEDITOR" || comp_type == "RRICHEDIT" || comp_type == "RMEMO") {
            crate::ui::gui_set_text(name, &val.to_string_val());
        }
        // Update text on Input (REDIT)
        if prop_lower == "text" && comp_type == "REDIT" {
            crate::ui::gui_set_input_value(name, &val.to_string_val());
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
        // (and a Color: a host paints it, even the creation default white)
        if prop_lower == "color" {
            comp.properties.insert("__colorset".into(), v_bool(true));
        }
        comp.properties.insert(prop_lower.clone(), val);
        #[cfg(feature = "gui")]
        if font_prop {
            drop(comps);
            crate::ui::gui_apply_font(name);
        }
    });
    // A panel's bevels drawn again (a QBEVEL's Shape / Style too).
    #[cfg(feature = "gui")]
    if rapidr_value::objects::bevel::default(&prop_lower).is_some() || matches!(prop_lower.as_str(), "shape" | "style") {
        crate::ui::redraw_widget(name);
    }
    // Align and geometry: lay out, move the widget (layout.rs).
    crate::layout::after_set(name, &prop_lower);
    // (I1) A dock manager or its floating window resized: its panes placed.
    crate::dock::after_set(name, &prop_lower);
    // (I1 / L-PANELS) An inspector showing it follows (panels.rs).
    crate::panels::after_set(name, &prop_lower);
    // A QTABCONTROL's colour, font or Enabled: drawn again (its tabs
    // measured again).
    #[cfg(feature = "gui")]
    if rapidr_value::objects::is_tabcontrol(name) && (prop_lower.starts_with("font") || matches!(prop_lower.as_str(), "color" | "enabled")) {
        crate::ui::tab_control_changed(name);
    }
    // A parent whose widget exists already: the widget is made now.
    #[cfg(feature = "gui")]
    if prop_lower == "parent" && !crate::layout::is_quiet() {
        crate::ui::attach_late(name);
    }
    // `CoolBtn.Down = True`: the others of its group come up.
    #[cfg(feature = "gui")]
    if prop_lower == "down" {
        crate::ui::toggle_down_set(name);
    }
    // A QCANVAS's new size shows more or less of its surface.
    #[cfg(feature = "gui")]
    if matches!(prop_lower.as_str(), "width" | "height") && rapidr_value::objects::is_header(name) {
        crate::ui::redraw_widget(name);
    } else if matches!(prop_lower.as_str(), "width" | "height") && rapidr_value::objects::is_canvas(name) {
        crate::ui::canvas_redraw(name);
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
            crate::ui::picture_refresh(name);
        }
    }
    #[cfg(feature = "gui")]
    if matches!(prop_lower.as_str(), "left" | "top") && rp_comp_type(name) == "RFORM" {
        crate::ui::gui_move_form(name);
    }
    #[cfg(feature = "gui")]
    if let Some(from) = state_before {
        crate::ui::gui_set_window_state(name, from);
    }
    #[cfg(not(feature = "gui"))]
    let _ = state_before;
    // A list view drawn in its color and font again.
    #[cfg(feature = "gui")]
    if (prop_lower == "color" || prop_lower.starts_with("font")) && rapidr_value::objects::is_listview(name) {
        crate::ui::listview_refresh(name);
    }
    // A timer enabled (again) or given another interval: it ticks.
    #[cfg(feature = "gui")]
    if matches!(prop_lower.as_str(), "enabled" | "interval") && is_timer_type(&rp_comp_type(name)) {
        crate::ui::gui_timer_changed(name);
    }
    // (the DirectX lane's) A QDXSCREEN put on a form already shown: set up.
    #[cfg(feature = "gui")]
    if prop_lower == "parent" && rp_comp_type(name) == "RDXSCREEN" {
        crate::directx::parented(name);
    }
    // A tree's image lists: its icons shown again.
    #[cfg(feature = "gui")]
    if matches!(prop_lower.as_str(), "images" | "stateimages") && rapidr_value::objects::is_tree(name) {
        crate::ui::tree_refresh(name);
    }
    #[cfg(feature = "gui")]
    if matches!(prop_lower.as_str(), "icon" | "icohandle") && matches!(rp_comp_type(name).as_str(), "RFORM" | "RFORMMDI") {
        crate::ui::gui_apply_icon(name);
    }
    // A form with / without its frame (bsNone): the window and its inside.
    if prop_lower == "borderstyle" && rp_comp_type(name) == "RFORM" {
        #[cfg(feature = "gui")]
        crate::ui::gui_set_form_border(name);
        crate::layout::client_changed(name);
    }
}

/// A form's ClientWidth / ClientHeight (rapidr_value::layout), less the
/// scroll bars it shows (scroll.rs).
pub fn form_client(name: &str) -> (i64, i64) {
    crate::scroll::client(name)
}

/// A form's inside: its frame and main menu excluded (its scroll bars not).
pub fn form_area(name: &str) -> (i64, i64) {
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
    return crate::ui::menu_offset(name) as i64;
    #[cfg(not(feature = "gui"))]
    {
        let _ = name;
        0
    }
}

/// Get a property from a registered component.
/// A form of its own (a window): a QFORM without a parent.
pub fn is_window(name: &str) -> bool {
    rp_comp_type(name) == "RFORM" && rp_comp_get(name, "parent").to_string_val().is_empty()
}

pub fn rp_comp_get(name: &str, prop: &str) -> Value {
    let prop_lower = prop.to_lowercase();
    // Form.Scale (RapidR's): its screen's device pixels per pixel.
    #[cfg(feature = "gui")]
    if prop_lower == "scale" && rp_comp_type(name) == "RFORM" {
        return Value::Double(crate::ui::form_scale(name));
    }
    // A window's Visible: whether it shows (Show, ShowModal, Visible = True
    // until Hide / Close or the user closes it).
    #[cfg(feature = "gui")]
    if prop_lower == "visible" && is_window(name) {
        let stored = || COMPONENTS.with(|c| c.borrow().get(&name.to_lowercase()).and_then(|comp| comp.properties.get("visible")).is_some_and(Value::to_bool));
        return v_bool(crate::ui::window_shown(name).unwrap_or_else(stored));
    }
    // Screen, Application, Clipboard, Mouse (globals.rs).
    if let Some(v) = crate::globals::get(name, &prop_lower) {
        return v;
    }
    // (the I/O and media lane's: io.rs)
    if let Some((sub, member)) = crate::io::sub_component(name, &prop_lower) {
        return rp_comp_get(&sub, &member);
    }
    if let Some(v) = crate::io::frames_timer_get(name, &prop_lower) {
        return v;
    }
    if let Some(v) = rapidr_value::objects::rqlib::get(name, &prop_lower) {
        return v;
    }
    // A form's inside (its frame and main menu excluded); other
    // components have no frame inside their size.
    if matches!(prop_lower.as_str(), "clientwidth" | "clientheight") {
        let (w, h) = if matches!(rp_comp_type(name).as_str(), "RFORM" | "RSCROLLBOX") {
            crate::scroll::client(name)
        } else {
            (rp_comp_get(name, "width").to_i64(), rp_comp_get(name, "height").to_i64())
        };
        return v_int(if prop_lower == "clientwidth" { w } else { h });
    }
    // A component's Handle (rapidr_value::handles).
    if prop_lower == "handle" && COMPONENTS.with(|c| c.borrow().contains_key(&name.to_lowercase())) {
        return v_int(rapidr_value::handles::handle_of(name));
    }
    // `Constraints.MinWidth` is MinWidth (rapidr_value::layout).
    if let Some(flat) = rapidr_value::layout::constraint_alias(&prop_lower) {
        return rp_comp_get(name, flat);
    }
    // A QFORMMDI's ChildCount, ChildCaption, … (mdi.rs).
    if let Some(v) = rapidr_value::mdi::get(name, &prop_lower) {
        return v;
    }
    // (I1) An RDOCKMANAGER's PaneCount, ActiveDocument, … (dock.rs).
    if rp_comp_type(name) == "RDOCKMANAGER" {
        if let Some(v) = rapidr_value::dock::runtime::rt_get(name, &prop_lower) {
            return v;
        }
    }
    // (I1 / L-PANELS) A panel's RowCount, Count, LineCount, … (panels.rs).
    if let Some(v) = crate::panels::get(name, &prop_lower) {
        return v;
    }
    // (I1) RapidR Studio's components (studio.rs).
    #[cfg(feature = "studio")]
    {
        let t = rp_comp_type(name);
        if rapidr_studio::is_studio_type(&t) {
            if let Some(v) = crate::studio::get(&t, name, &prop_lower) {
                return v;
            }
        }
    }
    // A QFORM's / QSCROLLBOX's AutoScroll, HorzPosition, … (scroll.rs).
    if let Some(v) = crate::scroll::get(name, &prop_lower) {
        return v;
    }
    // (what the user typed and selected, before the program reads it)
    #[cfg(feature = "gui")]
    if rapidr_value::objects::is_textedit(name) {
        crate::ui::text_pull(name);
    }
    if let Some(v) = rapidr_value::objects::get(name, &prop_lower) {
        return v;
    }

    // Check GUI state overrides first
    #[cfg(feature = "gui")]
    {
        let comp_type = rp_comp_type(name);
        match comp_type.as_str() {
            // (RDESIGNSURFACE's CompCount and FormCaption, RCODEEDITOR's
            // Text: the shared models' — objects::design, objects::textedit)
            "RRICHEDIT" | "RMEMO" => {
                if prop_lower == "text" {
                    return v_str(&crate::ui::gui_get_text(name));
                }
            }
            "REDIT" => {
                if prop_lower == "text" {
                    if let Some(val) = crate::ui::gui_get_input_value(name) {
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
            // (Anchors and Constraints: akLeft + akTop, none)
            .or_else(|| rapidr_value::layout::default_property(&rp_comp_type(name), &prop_lower).map(v_int))
            .unwrap_or_else(|| if matches!(prop_lower.as_str(), "left" | "top" | "right" | "bottom") { v_int(0) } else { v_null() })
    })
}

fn comp_is_panel(name: &str) -> bool {
    COMPONENTS.with(|c| c.try_borrow().ok().and_then(|c| c.get(&name.to_lowercase()).map(|x| matches!(x.type_name.as_str(), "RPANEL" | "RBEVEL"))).unwrap_or(false))
}

/// Get the type name of a registered component.
/// `Obj.Member` read without parentheses in a program: the method when the
/// object's type has one by that name (`WHILE MySQL.FetchRow`), else the
/// property (`UpDown.Max`) — rapidr_value::members, the same rule for native
/// builds, the interpreter and the web.
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
    crate::ui::picture_refresh(name);
}

/// Whether form `name`'s window exists (a size set while the form is being
/// declared paints nothing).
fn form_is_built(name: &str) -> bool {
    #[cfg(feature = "gui")]
    return crate::ui::form_window_exists(name);
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
    // (the I/O and media lane's: QCGI, QCOMPORT, QDOWNLOAD … — io.rs)
    if rapidr_value::objects::rqlib::exists(name) {
        if let Some((sub, member)) = crate::io::sub_component(name, &method_lower) {
            return rp_comp_method(&sub, &member, args);
        }
        let v = if method_lower == "leechfile" && rapidr_value::objects::rqlib::is_download(name) {
            crate::io::leech_file(name)
        } else {
            match rapidr_value::objects::call(name, &method_lower, args, &|id, p| rp_comp_get(id, p)) {
                Some(Ok(v)) => v,
                Some(Err(e)) => crate::value::runtime_error(&format!("{name}.{method}: {e}")),
                None => {
                    eprintln!("[rapidr] {name}.{method}: no such method");
                    v_null()
                }
            }
        };
        crate::io::fire_events(name);
        return v;
    }
    if method_lower == "files" && matches!(comp_type.as_str(), "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG") {
        let i = args.first().map_or(0, Value::to_i64);
        let v = rp_comp_get(name, &format!("files({i})"));
        return if matches!(v, Value::Null) { v_str("") } else { v };
    }
    // (the dialogs lane's) A QFONTDIALOG's AddStyles, DelStyles,
    // AddOptions, DelOptions, GetFont(F), SetFont(F), FontName(i).
    if comp_type == "RFONTDIALOG" {
        let get = |p: &str| rp_comp_get(name, p);
        let mut set = |p: &str, v: Value| rp_comp_set(name, p, v);
        if let Some(v) = rapidr_value::font_dialog::call(&method_lower, args, &get, &mut set) {
            return v;
        }
    }
    // (the dialogs lane's) A QCOLORDIALOG's Colors(i), 1 to 16: read, or
    // `Colors(i) = c` (its second argument).
    if method_lower == "colors" && comp_type == "RCOLORDIALOG" {
        let key = format!("colors({})", args.first().map_or(0, Value::to_i64));
        if let Some(c) = args.get(1) {
            rp_comp_set(name, &key, v_int(c.to_i64() & 0xFF_FFFF));
            return v_null();
        }
        return match rp_comp_get(name, &key) {
            Value::Null => v_int(0),
            v => v,
        };
    }
    // A QFORMMDI's AddChild, CascadeChild, … (mdi.rs).
    if rapidr_value::mdi::is_mdi(name) {
        if let Some(v) = crate::mdi::method(name, &method_lower, args) {
            return v;
        }
    }
    // (I1) RapidR Studio's RPROJECT, RLANGUAGESERVICE, RPROGRAMSESSION (studio.rs).
    #[cfg(feature = "studio")]
    if rapidr_studio::is_studio_type(&comp_type) {
        if let Some(v) = crate::studio::call(&comp_type, name, &method_lower, args) {
            return v;
        }
    }
    // (I1) An RDOCKMANAGER's AddPane, SaveLayout, … (dock.rs).
    if comp_type == "RDOCKMANAGER" {
        if let Some(v) = crate::dock::method(name, &method_lower, args) {
            return v;
        }
    }
    // (I1 / L-PANELS) A panel's AddButton, AddCommand, Write, … (panels.rs).
    if let Some(v) = crate::panels::method(name, &method_lower, args) {
        return v;
    }

    // `Form.Pixel(x, y)` read: RapidQ's -1s and its children's pixels.
    if rp_comp_type(name) == "RFORM" && args.len() == 2 && method_lower.as_str() == "pixel" {
        if let Some(v) = form_pixel(name, args) {
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

    // `PopupMenu.Popup(X, Y)` (screen coordinates, as RapidQ's).
    if method_lower == "popup" && rapidr_value::objects::menu::kind(name) == Some(rapidr_value::objects::menu::Kind::Popup) {
        #[cfg(feature = "gui")]
        {
            crate::ui::ensure_menu_widget(name);
            crate::ui::gui_menu_popup(name, args.first().map_or(0, Value::to_i64) as i32, args.get(1).map_or(0, Value::to_i64) as i32);
        }
        return v_null();
    }
    #[cfg(feature = "gui")]
    if rapidr_value::objects::menu::is_menu(name) {
        crate::ui::schedule_menu_sync();
    }
    // QEDIT / QRICHEDIT: the widget's text first; then Copy/Cut/Paste with
    // the clipboard, Line(i), AddStrings, … on the model, shown again.
    if rapidr_value::objects::is_textedit(name) {
        #[cfg(feature = "gui")]
        crate::ui::text_pull(name);
        let clip = rapidr_value::objects::textedit_clipboard(
            name,
            &method_lower,
            &|| crate::globals::get("clipboard", "text").map(|v| v.to_string_val()).unwrap_or_default(),
            &|s| {
                crate::globals::set("clipboard", "text", &v_str(s));
            },
        );
        let result = clip.map(Ok).or_else(|| rapidr_value::objects::call(name, &method_lower, args, &|id, p| rp_comp_get(id, p)));
        if let Some(result) = result {
            #[cfg(feature = "gui")]
            crate::ui::text_push(name);
            // (an RCODEEDITOR's ApplyPatches / Undo / Redo: OnChange)
            if rapidr_value::objects::take_code_change(name) {
                rp_fire_event(name, "onchange");
            }
            return result.unwrap_or_else(|e| {
                eprintln!("[rapidr] {name}.{method}: {e}");
                v_null()
            });
        }
    }
    if let Some(result) = rapidr_value::objects::call(name, &method_lower, args, &|id, p| rp_comp_get(id, p)) {
        if rapidr_value::objects::is_picture(name) {
            // `Image.LoadFromFile "photo.png"`: not a BMP; the host loads it.
            if result.is_err() && matches!(method_lower.as_str(), "loadfromfile" | "load") {
                if let Some(file) = args.first() {
                    rp_comp_set(name, "__imagefile", file.clone());
                }
                #[cfg(feature = "gui")]
                return crate::ui::image_method(name, &method_lower, args);
            }
            picture_changed(name);
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_canvas(name) {
            crate::ui::canvas_redraw(name);
        } else if rapidr_value::objects::is_dxscreen(name) {
            // (the DirectX lane's: a Flip shows the back buffer)
            if method_lower == "flip" {
                crate::ui::redraw_widget(name);
            }
        } else if rapidr_value::objects::is_trackbar(name) || rapidr_value::objects::is_design(name) || rapidr_value::objects::is_diff(name) {
            crate::ui::redraw_widget(name);
        }
        if rapidr_value::objects::is_design(name) {
            design_events(name);
        } else if rapidr_value::objects::is_tabcontrol(name) {
            crate::ui::tab_control_changed(name);
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_dirtree(name) {
            crate::ui::dirtree_refresh(name);
        }
        // (I1 / L-PANELS) A designer's selection or props changed: the
        // inspectors following it read it again.
        if rapidr_value::objects::is_design(name) {
            crate::panels::designer_changed(name);
        }
        // A QHEADER's sections changed (not a drawing on it): painted again.
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_header(name) && rapidr_value::objects::header::changes_sections(&method_lower) {
            crate::ui::header_refresh(name);
        }
        #[cfg(feature = "gui")]
        if rapidr_value::objects::is_tree(name) {
            crate::ui::tree_refresh(name);
        } else if rapidr_value::objects::is_listview(name) {
            crate::ui::listview_refresh(name);
        } else if rapidr_value::objects::is_grid(name) {
            crate::ui::grid_refresh(name);
        } else if rapidr_value::objects::is_list(name) {
            crate::ui::list_refresh(name);
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
    // A QSTATUSBAR's AddPanels / Clear (rapidr_value::statusbar).
    if comp_type == "RSTATUSBAR" {
        let get = |p: &str| rp_comp_get(name, p);
        let mut set = |p: &str, v: Value| rp_comp_set(name, p, v);
        if let Some(v) = rapidr_value::statusbar::call(&method_lower, args, &get, &mut set) {
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
        "RDESIGNSURFACE" => crate::ui::design_surface_method(name, &method_lower, args),
        // (RCODEEDITOR's GetSubList, GotoSub, GotoLine: its text model's,
        // above; the rest as any component's)
        #[cfg(feature = "gui")]
        "RTREEVIEW" => crate::ui::tree_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RCANVAS" => crate::ui::canvas_method(name, &method_lower, args),
        #[cfg(feature = "gui")]
        "RHEADER" if matches!(method_lower.as_str(), "repaint" | "refresh" | "update" | "paint") => {
            crate::ui::header_refresh(name);
            v_null()
        }
        #[cfg(feature = "gui")]
        "RHEADER" => crate::ui::canvas_method(name, &method_lower, args),
        // Data science component methods
        #[cfg(feature = "datascience")]
        "RNUM" => crate::datascience::num_method(name, &method_lower, args),
        #[cfg(feature = "datascience")]
        "RDATAFRAME" => crate::datascience::dataframe_method(name, &method_lower, args),
        #[cfg(feature = "datascience")]
        "RPLOT" => crate::datascience::plot_method(name, &method_lower, args),
        // RImage methods
        #[cfg(feature = "gui")]
        "RIMAGE" => crate::ui::image_method(name, &method_lower, args),
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
    if defers(&handler) {
        let (n, e, a) = (name.to_string(), event.to_string(), args.to_vec());
        defer(name, event, Box::new(move || drop(fire(&n, &e, &a))));
        return args.to_vec();
    }
    rapidr_value::events::call(handler, name, args, dispatch_indirect)
}

/// Fire an event with no arguments (handlers get the Sender).
pub fn rp_fire_event(name: &str, event: &str) {
    if lookup_handler(name, event).is_some_and(|h| defers(&h)) {
        let (n, e) = (name.to_string(), event.to_string());
        return defer(name, event, Box::new(move || rp_fire_event(&n, &e)));
    }
    let _ = fire(name, event, &[]);
    if event == "onclick" {
        button_modal_result(name);
    }
}

/// The form a component is on (itself for a form).
pub fn form_of(name: &str) -> Option<String> {
    let mut cur = name.to_lowercase();
    for _ in 0..32 {
        if rp_comp_type(&cur) == "RFORM" {
            return Some(cur);
        }
        let p = rp_comp_get(&cur, "parent").to_string_val().to_lowercase();
        if p.is_empty() {
            return None;
        }
        cur = p;
    }
    None
}

/// A button with a ModalResult (or Kind bkClose) clicked: its form gets that
/// result, which closes it when it's shown modally (RapidQ / Delphi).
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
            #[cfg(feature = "gui")]
            crate::ui::gui_close(&form);
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

/// Fires `event`, then runs `then` once its handler has run — at once in a
/// native build; in the interpreter, queued behind the handler
/// (`rapidr_value::events`). For events whose handler sets something the
/// runtime reads afterwards (a QFORMMDI's `ChildResult`).
///
/// `then` gets the arguments as the handler left them (RapidQ's event
/// parameters are by reference: OnClose's `Action`, OnSelectCell's
/// `CanSelect`, …).
pub fn rp_fire_event_then(name: &str, event: &str, args: &[Value], then: impl FnOnce(&[Value]) + 'static) {
    if lookup_handler(name, event).is_some_and(|h| defers(&h)) {
        let (n, e, a) = (name.to_string(), event.to_string(), args.to_vec());
        return defer(name, event, Box::new(move || rp_fire_event_then(&n, &e, &a, then)));
    }
    rapidr_value::events::fire_then(args, || fire(name, event, args), then);
}

// ---------------------------------------------------------------------------
// Handlers fired inside a host callback (docs/desktop-host-plan.md §1.5)
// ---------------------------------------------------------------------------

thread_local! {
    /// Set while a desktop host's callback is on the stack. A winit pump
    /// can't be re-entered, so program code (which may ShowModal or
    /// DOEVENTS) mustn't run there.
    static IN_HOST_CALLBACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Native handlers fired meanwhile, with what follows them (a button's
    /// ModalResult, an [`rp_fire_event_then`] continuation), oldest first.
    static DEFERRED: RefCell<std::collections::VecDeque<Box<dyn FnOnce()>>> = RefCell::new(std::collections::VecDeque::new());
}

/// Runs `f` as a host callback: a native handler fired inside it waits
/// until [`rp_run_deferred`]. (Bytecode handlers are queued by the VM
/// anyway.) Nests; the flag is restored however `f` ends.
pub fn rp_in_host_callback<R>(f: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            IN_HOST_CALLBACK.with(|c| c.set(self.0));
        }
    }
    let _restore = Restore(IN_HOST_CALLBACK.with(|c| c.replace(true)));
    f()
}

/// Whether a host callback is on the stack.
pub fn rp_host_callback_active() -> bool {
    IN_HOST_CALLBACK.with(std::cell::Cell::get)
}

/// Runs the handlers host callbacks deferred, in the order they were fired
/// (and any they fire, directly); returns how many. Nothing inside a host
/// callback, nor in a tracking tick's turn ([`rp_program_turn`]: what was
/// put off waits for the pump — run there, a message box put off for after
/// a held menu would only put itself off again).
pub fn rp_run_deferred() -> usize {
    let mut ran = 0;
    while !rp_host_callback_active() && !IN_TURN.with(std::cell::Cell::get) {
        let Some(job) = DEFERRED.with(|d| d.borrow_mut().pop_front()) else { break };
        job();
        ran += 1;
    }
    ran
}

/// Whether firing `handler` now must wait: a native handler inside a host
/// callback.
fn defers(handler: &EventHandler) -> bool {
    rp_host_callback_active() && !matches!(handler, EventHandler::Indirect(_) | EventHandler::IndirectThis(..))
}

/// Queues `job` (a handler a host callback fired); debug builds say which,
/// so the host's offending paths get found.
fn defer(name: &str, event: &str, job: Box<dyn FnOnce()>) {
    #[cfg(debug_assertions)]
    eprintln!("[rapidr] {name}.{event} fired inside a host callback: deferred");
    #[cfg(not(debug_assertions))]
    let _ = (name, event);
    DEFERRED.with(|d| d.borrow_mut().push_back(job));
}

// ---------------------------------------------------------------------------
// The program's turn inside the system's own loop (docs/desktop-host-plan.md,
// "Timers during native menu tracking")
// ---------------------------------------------------------------------------
//
// While a native menu is tracked, the system holds the pump: the host's
// tracking tick gives the program a turn from inside it — its due timers
// fire, and their handlers run as they would after the pump (native ones
// directly, inside [`rp_program_turn`]; the VM's by the wait it's in, which
// lent itself with [`rp_pump_wait_serving`]). What can't run in there (a
// wait of its own) is queued with [`rp_defer_job`] for after the pump.

type Server = *mut (dyn FnMut() + 'static);

thread_local! {
    /// The bytecode VM's waits that lent themselves, innermost last: each
    /// runs the handlers the VM has queued (taken out while it runs).
    static SERVERS: RefCell<Vec<Option<Server>>> = const { RefCell::new(Vec::new()) };
    /// Inside a tracking tick's turn ([`rp_program_turn`]).
    static IN_TURN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// [`rp_pump_wait`], with `serve` lent for its duration: when the system
/// holds the pump (a native menu tracked), the host's tracking tick fires
/// the program's due timers and `serve` runs the handlers they queued for
/// the VM — which waits in this call, between instructions, so it can lend
/// itself (its `Host::pump_serving`).
pub fn rp_pump_wait_serving(serve: &mut dyn FnMut()) -> Option<Value> {
    let p: *mut (dyn FnMut() + '_) = serve;
    // SAFETY: only the lifetime is erased. The entry is popped before this
    // returns (`Pop`, unwinding too), so the pointer never outlives `serve`'s
    // borrow, and `rp_serve_program` takes it out while it calls it (never
    // two `&mut` at once).
    let p: Server = unsafe { std::mem::transmute::<*mut (dyn FnMut() + '_), Server>(p) };
    SERVERS.with(|s| s.borrow_mut().push(Some(p)));
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            SERVERS.with(|s| s.borrow_mut().pop());
        }
    }
    let _pop = Pop;
    rp_pump_wait()
}

/// The handlers queued for the VM run now, by the innermost wait that lent
/// itself ([`rp_pump_wait_serving`]); without one (a native build, whose
/// handlers ran as they were fired) nothing happens.
pub fn rp_serve_program() {
    let Some(p) = SERVERS.with(|s| s.borrow_mut().last_mut().and_then(Option::take)) else { return };
    struct Back(Server);
    impl Drop for Back {
        fn drop(&mut self) {
            SERVERS.with(|s| {
                if let Some(top) = s.borrow_mut().last_mut() {
                    *top = Some(self.0);
                }
            });
        }
    }
    let _back = Back(p);
    // SAFETY: lent by an `rp_pump_wait_serving` still running below (its
    // entry is the innermost: anything pushed since was popped); taken out
    // of the list while it runs, so it isn't called inside itself.
    unsafe { (*p)() }
}

/// Runs `f` as the program's turn, inside a host callback too: the handlers
/// it fires run now, as between pumps (the host's tracking tick: the system
/// holds the pump, so after it would be too late). The flag comes back
/// however `f` ends.
pub fn rp_program_turn<R>(f: impl FnOnce() -> R) -> R {
    struct Restore(bool, bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            IN_HOST_CALLBACK.with(|c| c.set(self.0));
            IN_TURN.with(|c| c.set(self.1));
        }
    }
    let _restore = Restore(IN_HOST_CALLBACK.with(|c| c.replace(false)), IN_TURN.with(|c| c.replace(true)));
    f()
}

/// Runs `job` after the pump: with the handlers the host callbacks
/// deferred, in order ([`rp_run_deferred`]) — what a tracking tick can't do
/// inside the system's loop (a message box waits for the menu to close).
pub fn rp_defer_job(job: Box<dyn FnOnce()>) {
    DEFERRED.with(|d| d.borrow_mut().push_back(job));
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

/// For the bytecode VM: `ShowModal` — and every other builtin that waits
/// for the user (DOEVENTS, Popup, the dialogs, INPUT$) — leaves its wait to
/// the VM (see `ui::gui_set_cooperative_waits`), which serves it with
/// [`rp_pump_wait`].
pub fn rp_set_cooperative_waits(on: bool) {
    #[cfg(feature = "gui")]
    crate::ui::gui_set_cooperative_waits(on);
    #[cfg(not(feature = "gui"))]
    let _ = on;
}

/// Whether the last operation started a wait (asked once per operation).
pub fn rp_take_wait_started() -> bool {
    #[cfg(feature = "gui")]
    return crate::ui::gui_take_wait_started();
    #[cfg(not(feature = "gui"))]
    false
}

/// One step of the innermost wait: `None` while it goes on, `Some` when over.
pub fn rp_pump_wait() -> Option<Value> {
    #[cfg(feature = "gui")]
    return crate::ui::gui_pump_wait();
    #[cfg(not(feature = "gui"))]
    Some(v_null())
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
                        // (the other keys keep their order)
                        obj.shift_remove(&key);
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
                // (the form's ModalResult; the interpreter's comes when
                // its wait ends: gui_pump_wait)
                return v_int(crate::ui::gui_showmodal(name));
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
                crate::ui::gui_show(name);
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
                    crate::ui::gui_close(name);
                } else {
                    crate::ui::gui_hide(name);
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
            crate::ui::canvas_redraw(name);
            rp_fire_event(name, "onpaint");
            v_null()
        }
        "center" => {
            #[cfg(feature = "gui")]
            {
                crate::ui::gui_center(name);
                return v_null();
            }
            #[cfg(not(feature = "gui"))]
            {
                println!("[GUI] {}.Center()", name);
                v_null()
            }
        }
        "clear" => {
            // For ListBox, ComboBox, StringGrid, etc.
            #[cfg(feature = "gui")]
            {
                crate::ui::gui_widget_clear(name);
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
                    crate::ui::gui_widget_add_items(name, &item);
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
            // (L-PANELS: the kernel's focus to it, as the web's SetFocus)
            #[cfg(feature = "gui")]
            if let Some(form) = form_of(name) {
                rapidr_ui_app::windows::push_op(rapidr_ui_app::WindowOp::Focus(form.to_lowercase(), name.to_lowercase()));
            }
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
                        return crate::ui::gui_dialog_execute(name, comp_type);
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

/// Whether a type name is a component the compilers create, and whether a
/// member is some component's method: the language registry's
/// (crates/rapidr-lang), as the web runtime's.
pub use rapidr_lang::{is_component_method, is_component_type};

/// A stored property, without any of `rp_comp_get`'s lookups.
pub(crate) fn stored(name: &str, prop: &str) -> Option<Value> {
    COMPONENTS.with(|c| c.borrow().get(&name.to_lowercase()).and_then(|comp| comp.properties.get(prop).cloned()))
}

/// A stored property as an integer (0 when unset).
pub(crate) fn stored_int(name: &str, prop: &str) -> i64 {
    stored(name, prop).map_or(0, |v| v.to_i64())
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

/// Every component: (name, type), in creation order.
pub fn all_components() -> Vec<(String, String)> {
    COMPONENTS.with(|c| {
        let mut all: Vec<(String, String, u32)> = c.borrow().iter().map(|(n, comp)| (n.clone(), comp.type_name.clone(), comp.creation_order)).collect();
        all.sort_by_key(|c| c.2);
        all.into_iter().map(|(n, t, _)| (n, t)).collect()
    })
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

#[cfg(test)]
mod deferred_tests {
    use super::*;

    thread_local! {
        static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn log(s: &str) {
        LOG.with(|l| l.borrow_mut().push(s.to_string()));
    }

    fn logged() -> Vec<String> {
        LOG.with(|l| l.take())
    }

    fn on_a() {
        log("a");
    }

    fn on_b(x: Value) {
        log(&format!("b {}", x.to_string_val()));
    }

    fn sets_action(args: &mut [Value]) {
        log("close");
        args[0] = v_int(0);
    }

    #[test]
    fn handlers_run_at_once_outside_a_host_callback() {
        rp_bind_event("t1", "onclick", on_a);
        rp_fire_event("t1", "onclick");
        assert_eq!(logged(), ["a"]);
        assert_eq!(rp_run_deferred(), 0);
    }

    #[test]
    fn inside_a_host_callback_handlers_wait_their_turn_in_order() {
        rp_bind_event("t2", "onclick", on_a);
        rp_bind_event_1("t2", "onchange", on_b);
        rp_in_host_callback(|| {
            assert!(rp_host_callback_active());
            rp_fire_event("t2", "onclick");
            rp_fire_event_1("t2", "onchange", v_str("x"));
            rp_fire_event("t2", "onnothing");
            // Not while the callback is on the stack.
            assert_eq!(rp_run_deferred(), 0);
        });
        assert!(!rp_host_callback_active());
        assert!(logged().is_empty());
        assert_eq!(rp_run_deferred(), 2);
        assert_eq!(logged(), ["a", "b x"]);
        assert_eq!(rp_run_deferred(), 0);
    }

    #[test]
    fn a_continuation_follows_its_deferred_handler_with_its_arguments() {
        rp_bind_event_out("t3", "onclose", 2, sets_action);
        rp_in_host_callback(|| {
            rp_fire_event_then("t3", "onclose", &[v_int(1)], |out| log(&format!("then {}", out[0].to_i64())));
        });
        assert!(logged().is_empty());
        rp_run_deferred();
        assert_eq!(logged(), ["close", "then 0"]);
        // No handler: the continuation runs at once, even in a callback.
        rp_in_host_callback(|| rp_fire_event_then("t3", "onnone", &[v_int(1)], |out| log(&format!("then {}", out[0].to_i64()))));
        assert_eq!(logged(), ["then 1"]);
    }

    #[test]
    fn bytecode_handlers_go_to_the_vm_at_once() {
        rp_set_event_dispatcher(Box::new(|id, _| log(&format!("vm {id}"))));
        rp_bind_event_indirect("t4", "onclick", 7);
        rp_in_host_callback(|| rp_fire_event("t4", "onclick"));
        assert_eq!(logged(), ["vm 7"]);
        assert_eq!(rp_run_deferred(), 0);
        rp_clear_event_dispatcher();
    }

    // (timers during native menu tracking)
    #[test]
    fn a_tracking_ticks_turn_runs_native_handlers_inside_a_callback() {
        rp_bind_event("t5", "ontimer", on_a);
        rp_in_host_callback(|| {
            rp_program_turn(|| {
                assert!(!rp_host_callback_active());
                rp_fire_event("t5", "ontimer");
                // (what can't run inside the system's loop waits for the pump)
                rp_defer_job(Box::new(|| log("after")));
                // (nothing deferred runs here — DOEVENTS asks — only after
                // the pump)
                assert_eq!(rp_run_deferred(), 0);
            });
            assert!(rp_host_callback_active(), "the callback's flag comes back");
            assert_eq!(rp_run_deferred(), 0);
        });
        assert_eq!(logged(), ["a"]);
        assert_eq!(rp_run_deferred(), 1);
        assert_eq!(logged(), ["after"]);
        let r = std::panic::catch_unwind(|| rp_in_host_callback(|| rp_program_turn(|| panic!("in a turn"))));
        assert!(r.is_err());
        assert!(!rp_host_callback_active());
    }

    #[test]
    fn the_vms_handlers_run_by_the_innermost_wait_that_lent_itself() {
        // (no wait lent: nothing to serve — a native build)
        rp_serve_program();
        let served = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let outer = served.clone();
        // (the serving functions, as the VM's waits lend them; the pump wait
        // itself isn't run here, so they're registered by hand)
        let mut serve_outer = move || {
            outer.borrow_mut().push("outer");
            // (inside itself it isn't called again)
            rp_serve_program();
        };
        let p: *mut (dyn FnMut() + '_) = &mut serve_outer;
        let p: Server = unsafe { std::mem::transmute::<*mut (dyn FnMut() + '_), Server>(p) };
        SERVERS.with(|s| s.borrow_mut().push(Some(p)));
        rp_serve_program();
        let inner = served.clone();
        let mut serve_inner = move || inner.borrow_mut().push("inner");
        let q: *mut (dyn FnMut() + '_) = &mut serve_inner;
        let q: Server = unsafe { std::mem::transmute::<*mut (dyn FnMut() + '_), Server>(q) };
        SERVERS.with(|s| s.borrow_mut().push(Some(q)));
        rp_serve_program();
        SERVERS.with(|s| s.borrow_mut().pop());
        rp_serve_program();
        SERVERS.with(|s| s.borrow_mut().pop());
        rp_serve_program();
        assert_eq!(*served.borrow(), ["outer", "inner", "outer"]);
        assert!(SERVERS.with(|s| s.borrow().is_empty()));
    }

    #[test]
    fn callbacks_nest_and_restore_the_flag_on_unwind() {
        rp_in_host_callback(|| {
            rp_in_host_callback(|| ());
            assert!(rp_host_callback_active());
        });
        assert!(!rp_host_callback_active());
        let r = std::panic::catch_unwind(|| rp_in_host_callback(|| panic!("in a callback")));
        assert!(r.is_err());
        assert!(!rp_host_callback_active());
    }
}
