//! GUI backend using FLTK.
//!
//! This module provides actual GUI rendering when the "gui" feature is enabled.
//! Components are created as FLTK widgets and managed through a handle registry.

use std::cell::RefCell;
use std::collections::HashMap;

use fltk::{
    app,
    browser::HoldBrowser,
    button::{Button, CheckButton, RadioRoundButton},
    dialog,
    draw,
    enums::{Align, CallbackTrigger, Color, ColorDepth, Event, Font, FrameType, Key},
    frame::Frame,
    group::{Group, Scroll, Tabs},
    image::{RgbImage, SharedImage},
    input::Input,
    menu::{Choice, MenuBar, SysMenuBar},
    misc::Progress as FltkProgress,
    prelude::*,
    table::{Table, TableContext},
    text::{TextBuffer, TextEditor, StyleTableEntry},
    tree::Tree,
    valuator::HorNiceSlider,
    window::Window,
};

use fltk_theme::{ThemeType, WidgetTheme};

use crate::object::{rp_comp_get, rp_comp_set, rp_comp_type, rp_fire_event, rp_fire_event_1, rp_fire_event_2, rp_fire_event_5, rp_fire_event_args, rp_fire_event_then};
use crate::value::{v_int, v_null, v_str, Value};

// ---------------------------------------------------------------------------
// Widget handle registry
// ---------------------------------------------------------------------------

/// Each GUI component gets a unique handle. We store FLTK widgets in an enum
/// because they have different types. (FLTK widgets are handles: a clone
/// is the same widget.)
#[derive(Clone)]
enum GuiWidget {
    Window(Window),
    Button(Button),
    Frame(Frame),
    Input(Input),
    CheckButton(CheckButton),
    RadioButton(RadioRoundButton),
    Choice(Choice),
    /// A QCOMBOBOX with an edit box (csDropDown, the default; csSimple).
    InputChoice(fltk::misc::InputChoice),
    HoldBrowser(HoldBrowser),
    TextEditor(TextEditor),
    Group(Group),
    Tabs(Tabs),
    MenuBar(MenuBar),
    SysMenuBar(SysMenuBar),
    Progress(FltkProgress),
    Scroll(Scroll),
    Tree(Tree),
    Slider(HorNiceSlider),
    /// QSTRINGGRID: a table drawn from rapidr_value::objects::grid, with the
    /// input it edits cells in.
    Grid(Table, Input),
    ImageFrame(Frame), // RImage — Frame with drawn image
}

impl GuiWidget {
    /// The widget as a plain FLTK widget (a handle to the same one).
    fn base(&self) -> fltk::widget::Widget {
        match self {
            GuiWidget::Window(v) => v.as_base_widget(),
            GuiWidget::Button(v) => v.as_base_widget(),
            GuiWidget::Frame(v) | GuiWidget::ImageFrame(v) => v.as_base_widget(),
            GuiWidget::Input(v) => v.as_base_widget(),
            GuiWidget::CheckButton(v) => v.as_base_widget(),
            GuiWidget::RadioButton(v) => v.as_base_widget(),
            GuiWidget::Choice(v) => v.as_base_widget(),
            GuiWidget::InputChoice(v) => v.as_base_widget(),
            GuiWidget::HoldBrowser(v) => v.as_base_widget(),
            GuiWidget::TextEditor(v) => v.as_base_widget(),
            GuiWidget::Group(v) => v.as_base_widget(),
            GuiWidget::Tabs(v) => v.as_base_widget(),
            GuiWidget::MenuBar(v) => v.as_base_widget(),
            GuiWidget::SysMenuBar(v) => v.as_base_widget(),
            GuiWidget::Progress(v) => v.as_base_widget(),
            GuiWidget::Scroll(v) => v.as_base_widget(),
            GuiWidget::Tree(v) => v.as_base_widget(),
            GuiWidget::Slider(v) => v.as_base_widget(),
            GuiWidget::Grid(v, _) => v.as_base_widget(),
        }
    }
}

/// Whether `name` has a widget the user can see now (the test hooks'
/// `name.__shown`).
fn widget_shown(name: &str) -> bool {
    GUI_WIDGETS.with(|gw| gw.borrow().get(&name.to_lowercase()).map(GuiWidget::base)).is_some_and(|w| w.visible_r())
}

/// A component given a parent whose widget exists already (`Late.Parent =
/// Form` in an event handler, a QFORMMDI's child frame): its widget, and
/// those of the children it has, are made inside the parent now.
pub(crate) fn attach_late(name: &str) {
    let name = name.to_lowercase();
    if GUI_WIDGETS.with(|gw| gw.borrow().contains_key(&name)) {
        return;
    }
    let parent = rp_comp_get(&name, "parent").to_string_val().to_lowercase();
    let parent_is_container = GUI_WIDGETS.with(|gw| {
        matches!(gw.borrow().get(&parent), Some(GuiWidget::Window(_) | GuiWidget::Group(_) | GuiWidget::Tabs(_) | GuiWidget::Scroll(_)))
    });
    if parent.is_empty() || !parent_is_container {
        return;
    }
    let child_type = rp_comp_type(&name);
    if child_type == "RFORM" {
        return;
    }
    let (parent_x, parent_y) = get_widget_offset(&parent);
    let extra_y = if child_type != "RMAINMENU" { menu_offset(&parent) } else { 0 };
    let (orig_left, orig_top) = (rp_comp_get(&name, "left").to_i64(), rp_comp_get(&name, "top").to_i64());
    begin_widget(&parent);
    crate::layout::quietly(|| {
        rp_comp_set(&name, "left", v_int(orig_left + parent_x as i64));
        rp_comp_set(&name, "top", v_int(orig_top + parent_y as i64 + extra_y as i64));
        gui_create_widget(&name, &child_type);
        rp_comp_set(&name, "left", v_int(orig_left));
        rp_comp_set(&name, "top", v_int(orig_top));
    });
    build_children_recursive(&name);
    end_widget(&parent);
    if let Some(w) = GUI_WIDGETS.with(|gw| gw.borrow().get(&name).cloned()) {
        let mut b = w.base();
        if rp_comp_get(&name, "visible").to_string_val() != "0" && !matches!(rp_comp_get(&name, "visible"), Value::Boolean(false)) {
            b.show();
        }
        redraw_window_of(&w);
    }
}

// ---------------------------------------------------------------------------
// Design surface component tracking
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct DesignComp {
    name: String,
    type_name: String,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    props: HashMap<String, String>,
}

#[derive(Clone, Debug)]
struct DesignState {
    components: Vec<DesignComp>,
    selected: i32,
    form_w: i32,
    form_h: i32,
    form_caption: String,
    drag_mode: i32,      // 0=move, 1=resize-right, 2=resize-bottom, 3=resize-BR
    drag_offset_x: i32,  // mouse offset from component origin
    drag_offset_y: i32,
}

thread_local! {
    static GUI_WIDGETS: RefCell<HashMap<String, GuiWidget>> = RefCell::new(HashMap::new());
    static GUI_APP: RefCell<Option<app::App>> = RefCell::new(None);
    static GUI_TEXT_BUFFERS: RefCell<HashMap<String, TextBuffer>> = RefCell::new(HashMap::new());
    static GUI_STYLE_BUFFERS: RefCell<HashMap<String, TextBuffer>> = RefCell::new(HashMap::new());
    static DESIGN_SURFACES: RefCell<HashMap<String, DesignState>> = RefCell::new(HashMap::new());
    /// Maps tab control names to their child group names (tab_name -> group_widget_key)
    static TAB_GROUPS: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
    /// User-selected theme override: "light", "dark", "system", "aqua", "fluent", "sweet", or ""
    static THEME_OVERRIDE: RefCell<String> = RefCell::new(String::new());
    /// Active timer names (component names that are RTimer)
    static ACTIVE_TIMERS: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Set the application theme. Call before any window is shown.
/// Accepted values: "light", "dark", "system", "aqua", "fluent", "sweet".
pub fn set_theme(theme: &str) {
    THEME_OVERRIDE.with(|t| {
        *t.borrow_mut() = theme.to_lowercase();
    });
}

fn ensure_app() {
    // Check with an immutable borrow first to avoid conflicts with the event loop
    let needs_init = GUI_APP.with(|a| a.borrow().is_none());
    if !needs_init {
        return;
    }
    GUI_APP.with(|a| {
        let mut app_ref = a.borrow_mut();
        if app_ref.is_none() {
            let app = app::App::default();

            // Determine theme
            let theme_name = THEME_OVERRIDE.with(|t| t.borrow().clone());
            let theme_type = match theme_name.as_str() {
                "aqua" | "aquaclassic" => Some(ThemeType::AquaClassic),
                "fluent" | "metro" => Some(ThemeType::Metro),
                "aero" => Some(ThemeType::Aero),
                "sweet" | "dark" => Some(ThemeType::Dark),
                "greybird" | "light" => Some(ThemeType::Greybird),
                "highcontrast" => Some(ThemeType::HighContrast),
                "classic" => Some(ThemeType::Classic),
                "blue" => Some(ThemeType::Blue),
                "system" | "" => {
                    // Auto-detect OS
                    if cfg!(target_os = "macos") {
                        Some(ThemeType::AquaClassic)
                    } else if cfg!(target_os = "windows") {
                        Some(ThemeType::Metro)
                    } else {
                        // Linux and others
                        Some(ThemeType::Dark)
                    }
                }
                _ => None,
            };

            if let Some(tt) = theme_type {
                let widget_theme = WidgetTheme::new(tt);
                widget_theme.apply();
            } else {
                app::set_scheme(app::Scheme::Gtk);
            }

            *app_ref = Some(app);
        }
    });
    install_capture_hook();
}

/// For tests: with `RAPIDR_CAPTURE=<prefix>` set, the program saves every
/// open window (forms and dialogs) as `<prefix>-<n>.bmp` after
/// `RAPIDR_CAPTURE_DELAY` seconds (default 1.5), then exits — a way to
/// check desktop rendering without screen-recording permission.
/// `RAPIDR_TEST_EVENTS` lists `component.event`s to fire just before, and
/// `RAPIDR_TEST_DUMP` `component.property`s to print after them.
/// `RAPIDR_TEST_RESIZE=w,h` first resizes the frontmost form (to Width w,
/// Height h) as a user dragging its border would, and
/// `RAPIDR_TEST_SPLIT=splitter:delta` drags a QSPLITTER by `delta` pixels.
/// Under a GUI test only the test's own events (`RAPIDR_TEST_EVENTS`)
/// drive the program: the mouse and keyboard are ignored, since the test's
/// window takes the keyboard when it opens and whatever someone types
/// meanwhile would otherwise click its focused button.
fn ignore_user_input() {
    fn dispatch(ev: Event, win: app::WindowPtr) -> bool {
        match ev {
            Event::Push | Event::Released | Event::Drag | Event::KeyDown | Event::KeyUp | Event::Shortcut | Event::MouseWheel => false,
            // SAFETY: `win` is the window FLTK passed in for this event.
            _ => unsafe { app::handle_raw(ev, win) },
        }
    }
    // SAFETY: `dispatch` only forwards the window pointer it is given.
    unsafe { app::event_dispatch(dispatch) };
}

fn install_capture_hook() {
    let Ok(prefix) = std::env::var("RAPIDR_CAPTURE") else { return };
    ignore_user_input();
    let delay = std::env::var("RAPIDR_CAPTURE_DELAY").ok().and_then(|d| d.parse().ok()).unwrap_or(1.5);
    // `RAPIDR_TEST_EVENTS=b1.onclick,b2.onclick`: fire these first, as if
    // the user had clicked (tests of EVENT handlers and bindings).
    let events = std::env::var("RAPIDR_TEST_EVENTS").unwrap_or_default();
    let resize = std::env::var("RAPIDR_TEST_RESIZE").ok().and_then(|r| {
        let (w, h) = r.split_once(',')?;
        Some((w.trim().parse::<i32>().ok()?, h.trim().parse::<i32>().ok()?))
    });
    let split = std::env::var("RAPIDR_TEST_SPLIT").ok().and_then(|r| {
        let (name, delta) = r.rsplit_once(':')?;
        Some((name.trim().to_string(), delta.trim().parse::<i64>().ok()?))
    });
    app::add_timeout3(delay, move |_| {
        if let Some((name, delta)) = &split {
            if crate::layout::splitter_begin(name) {
                crate::layout::splitter_move(delta / 2);
                crate::layout::splitter_move(*delta);
                crate::layout::splitter_end();
            }
        }
        if let Some((w, h)) = resize {
            if let Some(mut win) = app::first_window() {
                // Width / Height of a form with a frame: the window is less.
                let (fw, fh) = rapidr_value::layout::form_frame(2);
                let (x, y) = (win.x(), win.y());
                win.resize(x, y, w - fw as i32, h - fh as i32);
            }
        }
        // One event per turn of the event loop, as real clicks come (the
        // bytecode VM runs a handler once the callback that fired it returns).
        let queue: Vec<String> = events.split(',').map(|e| e.trim().to_string()).filter(|e| !e.is_empty()).collect();
        fire_test_events(queue, prefix.clone());
    });
}

/// Fires the first of `queue`, then the rest a turn later; then (once the
/// last handlers have run) redraws and captures the windows.
fn fire_test_events(mut queue: Vec<String>, prefix: String) {
    if queue.is_empty() {
        app::redraw();
        app::add_timeout3(0.3, move |_| capture_windows(&prefix));
        return;
    }
    let e = queue.remove(0);
    if let Some((comp, event)) = e.rsplit_once('.') {
        // A toggle button's click goes through its group, as the widget's does.
        if event.eq_ignore_ascii_case("onclick") && is_toggle_button(comp) {
            toggle_press(&comp.to_lowercase());
        }
        let event = event.to_ascii_lowercase();
        // `grid.__cell_2_1`: the user clicks cell (2, 1).
        let cell = event.strip_prefix("__cell_").and_then(|rc| {
            let (c, r) = rc.split_once('_')?;
            Some((c.parse::<i64>().ok()?, r.parse::<i64>().ok()?))
        });
        match (event.as_str(), cell) {
            // `form.__close`: the window's close button.
            ("__close", _) => gui_close(comp),
            (_, Some((c, r))) => {
                grid_select(&comp.to_lowercase(), c, r);
            }
            _ => crate::object::rp_fire_event(comp, &event),
        }
    }
    app::add_timeout3(0.05, move |_| fire_test_events(queue.clone(), prefix.clone()));
}

fn capture_windows(prefix: &str) {
    // `RAPIDR_TEST_DUMP=b1.caption,b2.caption`: print these properties.
    for p in std::env::var("RAPIDR_TEST_DUMP").unwrap_or_default().split(',').filter(|p| !p.trim().is_empty()) {
        if let Some((comp, prop)) = p.trim().rsplit_once('.') {
            let value = if prop == "__shown" { i64::from(widget_shown(comp)).to_string() } else { rp_comp_get(comp, prop).to_string_val() };
            println!("{}={}", p.trim(), value);
        }
    }
    // Draw what handlers changed since the last redraw (the interpreter runs
    // them after the hook's own redraw).
    app::redraw();
    app::flush();
    let mut n = 0;
    for mut win in app::windows().unwrap_or_default() {
        if !win.shown() {
            continue;
        }
        let Ok(img) = draw::capture_window(&mut win) else { continue };
        let (w, h) = (img.data_w() as usize, img.data_h() as usize);
        let data = img.to_rgb_data();
        let channels = if w * h > 0 { data.len() / (w * h) } else { 0 };
        if channels < 3 {
            continue;
        }
        let pixels = data.chunks(channels).map(|p| (p[2] as u32) << 16 | (p[1] as u32) << 8 | p[0] as u32).collect();
        let bmp = rapidr_value::objects::codec::encode_bmp(&rapidr_value::objects::codec::Pixels { width: w, height: h, pixels });
        n += 1;
        let path = format!("{prefix}-{n}.bmp");
        match std::fs::write(&path, bmp) {
            Ok(()) => eprintln!("[rapidr] captured window '{}' to {path}", win.label()),
            Err(e) => eprintln!("[rapidr] can't write {path}: {e}"),
        }
    }
    std::process::exit(0);
}

/// A modal message with buttons in the given order, the first being the
/// default (Return) and Escape/closing meaning "none"; returns the index of
/// the button chosen. RapidQ's MESSAGEBOX/MESSAGEDLG default to the first
/// button, which FLTK's stock dialogs can't do with three buttons.
pub fn gui_choice(title: &str, text: &str, labels: &[&str]) -> Option<usize> {
    use fltk::button::ReturnButton;
    use std::rc::Rc;
    ensure_app();
    let (bw, bh, gap, pad) = (90, 28, 10, 16);
    draw::set_font(Font::Helvetica, app::font_size());
    let (tw, th) = draw::measure(text, true);
    let buttons_w = labels.len() as i32 * (bw + gap) - gap;
    let w = (tw + 2 * pad).max(buttons_w + 2 * pad).clamp(260, 900);
    let h = th.max(20) + 3 * pad + bh;
    let mut win = Window::default().with_size(w, h).with_label(title);
    win.make_modal(true);
    let mut msg = Frame::new(pad, pad, w - 2 * pad, th.max(20), None);
    msg.set_label(text);
    msg.set_align(Align::Left | Align::Top | Align::Inside | Align::Wrap);
    let chosen = Rc::new(std::cell::Cell::new(None));
    let mut x = w - pad - buttons_w;
    for (i, label) in labels.iter().enumerate() {
        let (y, chosen) = (h - pad - bh, chosen.clone());
        let mut win_ref = win.clone();
        let mut pick = move || {
            chosen.set(Some(i));
            win_ref.hide();
        };
        if i == 0 {
            ReturnButton::new(x, y, bw, bh, None).with_label(label).set_callback(move |_| pick());
        } else {
            Button::new(x, y, bw, bh, None).with_label(label).set_callback(move |_| pick());
        }
        x += bw + gap;
    }
    win.end();
    win.show();
    while win.shown() {
        if !app::wait() {
            break;
        }
    }
    chosen.get()
}

/// Makes sure FLTK is ready before a dialog is shown on its own.
pub fn gui_prepare_dialog() {
    ensure_app();
}

fn bgr_to_fltk_color(bgr: i64) -> Color {
    let r = (bgr & 0xFF) as u8;
    let g = ((bgr >> 8) & 0xFF) as u8;
    let b = ((bgr >> 16) & 0xFF) as u8;
    Color::from_rgb(r, g, b)
}

/// Create the actual FLTK widget for a component.
/// Called when properties have been set and we need to materialize the widget.
pub fn gui_create_widget(name: &str, comp_type: &str) {
    ensure_app();
    let name_lower = name.to_lowercase();

    // Idempotent: if widget already exists, skip creation
    let already_exists = GUI_WIDGETS.with(|gw| gw.borrow().contains_key(&name_lower));
    if already_exists {
        return;
    }

    match comp_type {
        "RFORM" => {
            // The window is the form's inside plus its in-window menu: the
            // window manager draws the frame (rapidr_value::layout).
            let (w, h) = form_window_size(name);
            let caption = rp_comp_get(name, "caption").to_string_val();

            // Check for parent form (RapidQ-style: assigning Parent removes from taskbar)
            let parent = rp_comp_get(name, "parent").to_string_val().to_lowercase();
            let has_parent = !parent.is_empty() && parent != "0";

            if has_parent {
                // Child form: position relative to parent, non-modal
                let lx = rp_comp_get(name, "left").to_i64() as i32;
                let ly = rp_comp_get(name, "top").to_i64() as i32;
                let x = if lx > 0 { lx } else { 50 };
                let y = if ly > 0 { ly } else { 50 };
                let mut win = Window::new(x, y, w, h, None);
                win.set_label(&caption);
                win.make_resizable(true);
                form_surface_overlay(&name_lower, w, h);
                win.end();
                let form = name_lower.clone();
                win.resize_callback(move |_, x, y, w, h| form_resized(&form, x, y, w, h));
                win.set_border(rp_comp_get(name, "borderstyle").to_i64() != 0);
                close_button(&mut win, &name_lower);
                GUI_WIDGETS.with(|gw| {
                    gw.borrow_mut().insert(name_lower, GuiWidget::Window(win));
                });
            } else {
                // Ensure this window is created as a TOP-LEVEL window, not
                // embedded inside whatever FLTK group/window is currently open.
                // This is critical for modal dialogs opened from within an
                // existing event loop (e.g. EventEditor opened from the IDE).
                Group::set_current(None::<&Group>);
                let mut win = Window::new(100, 100, w, h, None);
                win.set_label(&caption);
                win.make_resizable(true);
                form_surface_overlay(&name_lower, w, h);
                win.end();
                let form = name_lower.clone();
                win.resize_callback(move |_, x, y, w, h| form_resized(&form, x, y, w, h));
                win.set_border(rp_comp_get(name, "borderstyle").to_i64() != 0);
                close_button(&mut win, &name_lower);
                GUI_WIDGETS.with(|gw| {
                    gw.borrow_mut().insert(name_lower, GuiWidget::Window(win));
                });
            }
        }
        "RBUTTON" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "caption").to_string_val();
            let mut btn = Button::new(x, y, w, h, None);
            btn.set_label(&caption);
            btn.set_frame(FrameType::UpBox);

            // Color-based visual feedback (more pronounced for themed apps)
            let normal_color = btn.color();
            let normal_label_color = btn.label_color();
            let (r, g, b_c) = normal_color.to_rgb();
            let hover_color = Color::from_rgb(
                r.saturating_add(25).min(245),
                g.saturating_add(25).min(245),
                b_c.saturating_add(35).min(255),
            );
            let press_color = Color::from_rgb(
                r.saturating_sub(35),
                g.saturating_sub(35),
                b_c.saturating_sub(25),
            );
            let hover_label = Color::from_rgb(0, 60, 180);
            let name_for_press = name.to_lowercase();
            let focus_color = Color::from_rgb(
                r.saturating_add(10).min(245),
                g.saturating_add(15).min(248),
                b_c.saturating_add(40).min(255),
            );
            btn.handle(move |b, ev| {
                match ev {
                    Event::Enter => {
                        b.set_color(hover_color);
                        b.set_label_color(hover_label);
                        b.set_frame(FrameType::UpBox);
                        b.redraw();
                        true
                    }
                    Event::Leave => {
                        b.set_color(normal_color);
                        b.set_label_color(normal_label_color);
                        b.set_frame(FrameType::UpBox);
                        b.redraw();
                        true
                    }
                    Event::Push => {
                        press_begin(&name_for_press);
                        b.set_color(press_color);
                        b.set_frame(FrameType::DownBox);
                        b.redraw();
                        true // we handle the visual; Released will fire callback
                    }
                    Event::Released => {
                        b.set_color(hover_color);
                        b.set_frame(FrameType::UpBox);
                        b.redraw();
                        // A click: pressed on the button and released over it.
                        if press_end(&name_for_press) && app::event_inside_widget(b) {
                            b.do_callback();
                        }
                        true
                    }
                    Event::Focus => {
                        b.set_color(focus_color);
                        b.set_frame(FrameType::ThinUpBox);
                        b.redraw();
                        true
                    }
                    Event::Unfocus => {
                        b.set_color(normal_color);
                        b.set_label_color(normal_label_color);
                        b.set_frame(FrameType::UpBox);
                        b.redraw();
                        true
                    }
                    // Enter clicks; Space clicks when it's released (once,
                    // as a Windows button does).
                    Event::KeyDown | Event::KeyUp => match key_click(&name_for_press, ev) {
                        Some(true) => {
                            b.do_callback();
                            true
                        }
                        Some(false) => true,
                        None => false,
                    },
                    _ => false,
                }
            });

            let name_for_cb = name.to_lowercase();
            btn.set_callback(move |_| {
                rp_fire_event(&name_for_cb, "onclick");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Button(btn));
            });
        }
        "RCOOLBTN" => {
            // RCoolBtn — flat button with toggle (GroupIndex), multi-state BMP
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "caption").to_string_val();
            let flat = rp_comp_get(name, "flat").to_i64() != 0;
            let down = rp_comp_get(name, "down").to_bool();

            let mut btn = Button::new(x, y, w, h, None);
            btn.set_label(&caption);
            if down {
                btn.set_frame(FrameType::DownBox);
            } else if flat {
                btn.set_frame(FrameType::FlatBox);
            } else {
                btn.set_frame(FrameType::UpBox);
            }

            // Load BMP image if specified
            let bmp_path = rp_comp_get(name, "bmp").to_string_val();
            if !bmp_path.is_empty() {
                if let Some(mut img) = load_shared_image(&bmp_path) {
                    let num_bmps = rp_comp_get(name, "numbmps").to_i64().max(1) as i32;
                    if num_bmps > 1 {
                        // Multi-state BMP: crop to first frame (up state)
                        let iw = img.width() / num_bmps;
                        let ih = img.height();
                        img.scale(iw, ih, true, true);
                    }
                    btn.set_image(Some(img));
                }
            }

            let name_for_cb = name.to_lowercase();
            let name_for_handle = name.to_lowercase();
            let is_flat = flat;

            btn.handle(move |b, ev| {
                match ev {
                    Event::Enter => {
                        if is_flat {
                            b.set_frame(FrameType::ThinUpBox);
                            b.redraw();
                        }
                        true
                    }
                    Event::Leave => {
                        if is_flat {
                            b.set_frame(FrameType::FlatBox);
                            b.redraw();
                        }
                        true
                    }
                    Event::Push => {
                        press_begin(&name_for_handle);
                        b.set_frame(FrameType::DownBox);
                        b.redraw();
                        true
                    }
                    Event::Released | Event::KeyDown | Event::KeyUp => {
                        // A click (pressed on it and released over it, or
                        // Enter / Space): its group (rapidr_value::toggle_group);
                        // then it shows its Down.
                        let click = if ev == Event::Released {
                            press_end(&name_for_handle) && app::event_inside_widget(b)
                        } else {
                            match key_click(&name_for_handle, ev) {
                                Some(click) => click,
                                None => return false,
                            }
                        };
                        if click {
                            toggle_press(&name_for_handle);
                        }
                        let down = rp_comp_get(&name_for_handle, "down").to_bool();
                        b.set_frame(if down { FrameType::DownBox } else if is_flat { FrameType::FlatBox } else { FrameType::UpBox });
                        b.redraw();
                        if click {
                            b.do_callback();
                        }
                        true
                    }
                    _ => false,
                }
            });

            btn.set_callback(move |_| {
                rp_fire_event(&name_for_cb, "onclick");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Button(btn));
            });
        }
        "ROVALBTN" => {
            // ROvalBtn — oval/round button with color properties and toggle support
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "caption").to_string_val();
            let color_val = rp_comp_get(name, "color").to_i64();
            let highlight_val = rp_comp_get(name, "colorhighlight").to_i64();
            let shadow_val = rp_comp_get(name, "colorshadow").to_i64();
            let base_color = if color_val != 0 { bgr_to_fltk_color(color_val) } else { Color::from_rgb(220, 220, 220) };
            let hl_color = if highlight_val != 0 { bgr_to_fltk_color(highlight_val) } else { Color::from_rgb(255, 255, 255) };
            let sh_color = if shadow_val != 0 { bgr_to_fltk_color(shadow_val) } else { Color::from_rgb(128, 128, 128) };

            let mut btn = Button::new(x, y, w, h, None);
            btn.set_label(&caption);
            btn.set_frame(FrameType::OFlatFrame);

            // Custom draw for oval shape
            let name_for_draw = name.to_lowercase();
            let cap_for_draw = caption.clone();
            btn.draw(move |b| {
                let bx = b.x();
                let by = b.y();
                let bw = b.w();
                let bh = b.h();
                let is_down = rp_comp_get(&name_for_draw, "down").to_i64() != 0;
                // Draw oval background
                if is_down {
                    draw::set_draw_color(sh_color);
                } else {
                    draw::set_draw_color(base_color);
                }
                draw::draw_pie(bx, by, bw, bh, 0.0, 360.0);
                // Highlight arc (top-left)
                draw::set_draw_color(if is_down { sh_color } else { hl_color });
                draw::draw_arc(bx, by, bw, bh, 45.0, 225.0);
                // Shadow arc (bottom-right)
                draw::set_draw_color(if is_down { hl_color } else { sh_color });
                draw::draw_arc(bx, by, bw, bh, 225.0, 405.0);
                // Label centered
                draw::set_draw_color(Color::Black);
                draw::set_font(Font::Helvetica, 12);
                draw::draw_text2(&cap_for_draw, bx, by, bw, bh, Align::Center);
            });

            let name_for_cb = name.to_lowercase();
            let name_for_handle = name.to_lowercase();

            btn.handle(move |b, ev| {
                match ev {
                    Event::Push => {
                        toggle_press(&name_for_handle);
                        b.redraw();
                        b.do_callback();
                        true
                    }
                    _ => false,
                }
            });

            btn.set_callback(move |_| {
                rp_fire_event(&name_for_cb, "onclick");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Button(btn));
            });
        }
        "RLABEL" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "caption").to_string_val();
            let mut lbl = Frame::new(x, y, w, h, None);
            lbl.set_label(&caption);
            lbl.set_frame(FrameType::NoBox);
            // RapidQ's Alignment: taLeftJustify = 0 (default), taRightJustify
            // = 1, taCenter = 2; the text sits at the top, as with AutoSize.
            let horizontal = match rp_comp_get(name, "alignment").to_i64() {
                1 => Align::Right,
                2 => Align::Center,
                _ => Align::Left,
            };
            lbl.set_align(horizontal | Align::Top | Align::Inside | Align::Clip);
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Frame(lbl));
            });
        }
        "REDIT" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let text = rp_comp_get(name, "text").to_string_val();
            let mut inp = Input::new(x, y, w, h, None);
            inp.set_value(&text);
            let name_for_cb = name.to_lowercase();
            inp.set_callback(move |i| {
                rp_comp_set(&name_for_cb, "text", v_str(&i.value()));
                rp_fire_event(&name_for_cb, "onchange");
            });
            let normal_frame = FrameType::DownBox;
            let focus_frame = FrameType::BorderBox;
            inp.handle(move |w, ev| {
                match ev {
                    Event::Focus => { w.set_frame(focus_frame); w.redraw(); false }
                    Event::Unfocus => { w.set_frame(normal_frame); w.redraw(); false }
                    _ => false,
                }
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Input(inp));
            });
        }
        "RPANEL" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut grp = Group::new(x, y, w, h, None);
            // RapidQ's panel shows its Caption centered, under its children.
            grp.set_label(&rp_comp_get(name, "caption").to_string_val());
            grp.set_align(Align::Center | Align::Inside | Align::Clip);
            grp.end();
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Group(grp));
            });
        }
        "RCHECKBOX" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "caption").to_string_val();
            let mut cb = CheckButton::new(x, y, w, h, None);
            cb.set_label(&caption);
            let normal_lbl_color = cb.label_color();
            let hover_lbl_color = Color::from_rgb(0, 60, 180);
            cb.handle(move |c, ev| {
                match ev {
                    Event::Enter => {
                        c.set_label_color(hover_lbl_color);
                        c.redraw();
                        true
                    }
                    Event::Leave => {
                        c.set_label_color(normal_lbl_color);
                        c.redraw();
                        true
                    }
                    _ => false,
                }
            });
            let name_for_cb = name.to_lowercase();
            cb.set_callback(move |c| {
                rp_comp_set(&name_for_cb, "checked", v_int(if c.is_checked() { 1 } else { 0 }));
                rp_fire_event(&name_for_cb, "onclick");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::CheckButton(cb));
            });
        }
        "RRADIOBUTTON" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "caption").to_string_val();
            let mut rb = RadioRoundButton::new(x, y, w, h, None);
            rb.set_label(&caption);
            let normal_rb_color = rb.label_color();
            let hover_rb_color = Color::from_rgb(0, 60, 180);
            rb.handle(move |r, ev| {
                match ev {
                    Event::Enter => {
                        r.set_label_color(hover_rb_color);
                        r.redraw();
                        true
                    }
                    Event::Leave => {
                        r.set_label_color(normal_rb_color);
                        r.redraw();
                        true
                    }
                    _ => false,
                }
            });
            let name_for_cb = name.to_lowercase();
            rb.set_callback(move |b| {
                let is_checked = b.value();
                rp_comp_set(&name_for_cb, "checked", v_int(if is_checked { 1 } else { 0 }));
                rp_fire_event(&name_for_cb, "onclick");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::RadioButton(rb));
            });
        }
        "RCOMBOBOX" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let name_for_cb = name.to_lowercase();
            // Style (RAPIDQ.INC): csDropDown = 0 (the default) and csSimple
            // = 1 have an edit box; csDropDownList = 2 and the owner-draw
            // styles only pick from the list.
            let widget = if rp_comp_get(name, "style").to_i64() >= 2 {
                let mut choice = Choice::new(x, y, w, h, None);
                choice.set_callback(move |c| {
                    // The user's pick: the list's ItemIndex and Text, then OnChange.
                    let idx = c.value() as i64;
                    rapidr_value::objects::with_list_mut(&name_for_cb, |l| l.select(idx));
                    rp_fire_event(&name_for_cb, "onchange");
                });
                GuiWidget::Choice(choice)
            } else {
                let mut combo = fltk::misc::InputChoice::new(x, y, w, h, None);
                combo.set_trigger(CallbackTrigger::Changed);
                combo.set_callback(move |c| {
                    // Typed or picked: the Text (and the ItemIndex of the item
                    // it matches, else -1), then OnChange.
                    let text = c.value().unwrap_or_default();
                    rapidr_value::objects::with_list_mut(&name_for_cb, |l| l.set("text", &v_str(&text)));
                    rp_fire_event(&name_for_cb, "onchange");
                });
                GuiWidget::InputChoice(combo)
            };
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, widget);
            });
            list_refresh(name);
        }
        "RLISTBOX" | "RFILELISTBOX" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            // Style = lbOwnerDrawFixed / lbOwnerDrawVariable: the items are
            // drawn by OnDrawItem (a table of one column).
            if rapidr_value::objects::with_list(name, |l| l.owner_drawn()).unwrap_or(false) {
                owner_list_create(&name_lower, x, y, w, h);
                return;
            }
            let mut browser = HoldBrowser::new(x, y, w, h, None);
            let name_for_cb = name.to_lowercase();
            browser.set_callback(move |b| {
                let idx = b.value() as i64 - 1; // FLTK browsers are 1-indexed
                if idx < 0 {
                    return;
                }
                // MultiSelect: every line's selection (clicks toggle, shift
                // extends); otherwise the clicked item alone.
                let multi = rapidr_value::objects::with_list(&name_for_cb, |l| l.multi_select).unwrap_or(false);
                if multi {
                    let flags: Vec<bool> = (1..=b.size()).map(|line| b.selected(line)).collect();
                    rapidr_value::objects::with_list_mut(&name_for_cb, |l| l.set_selection(idx, &flags));
                } else {
                    rapidr_value::objects::with_list_mut(&name_for_cb, |l| l.select(idx));
                }
                rp_fire_event(&name_for_cb, if app::event_clicks() { "ondblclick" } else { "onclick" });
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::HoldBrowser(browser));
            });
            list_refresh(name);
        }
        "RDIRTREE" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut browser = HoldBrowser::new(x, y, w, h, None);
            let name_for_cb = name.to_lowercase();
            // A click selects a directory (OnChange); a double click opens
            // or closes it (rapidr_value::objects::dirtree).
            browser.set_callback(move |b| {
                let Ok(i) = usize::try_from(b.value() - 1) else { return };
                let double = app::event_clicks();
                let changed = rapidr_value::objects::with_dirtree(&name_for_cb, |t| {
                    if double {
                        t.toggle(i);
                    }
                    t.click(i)
                })
                .unwrap_or(false);
                if double {
                    dirtree_refresh(&name_for_cb);
                }
                if changed {
                    rp_fire_event(&name_for_cb, "onchange");
                }
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::HoldBrowser(browser));
            });
            dirtree_refresh(name);
        }
        "RRICHEDIT" | "RMEMO" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let text = rp_comp_get(name, "text").to_string_val();
            let mut buf = TextBuffer::default();
            buf.set_text(&text);
            let mut editor = TextEditor::new(x, y, w, h, None);
            editor.set_buffer(buf.clone());
            GUI_TEXT_BUFFERS.with(|tb| {
                tb.borrow_mut().insert(name_lower.clone(), buf);
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::TextEditor(editor));
            });
        }
        "RPROGRESS" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let min = rp_comp_get(name, "min").to_f64();
            let max = rp_comp_get(name, "max").to_f64();
            let pos = rp_comp_get(name, "position").to_f64();
            let mut prog = FltkProgress::new(x, y, w, h, None);
            prog.set_minimum(min);
            prog.set_maximum(max);
            prog.set_value(pos);
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Progress(prog));
            });
        }
        "RSTATUSBAR" => {
            // Docked at the bottom of its form by its Align (alBottom by
            // default, as in RapidQ): see layout.rs.
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut bar = Frame::new(x, y, w, h, None);
            let id = name_lower.clone();
            bar.draw(move |f| draw_statusbar(&id, f.x(), f.y(), f.w(), f.h()));
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Frame(bar));
            });
        }
        "RTABCONTROL" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let tabs = Tabs::new(x, y, w, h, None);

            // Create tab groups from stored AddTabs data
            let group_names = TAB_GROUPS.with(|tg| {
                tg.borrow().get(&name_lower).cloned().unwrap_or_default()
            });
            let labels_str = rp_comp_get(name, "_tab_labels").to_string_val();
            let labels: Vec<&str> = if labels_str.is_empty() {
                Vec::new()
            } else {
                labels_str.lines().collect()
            };

            for (i, grp_name) in group_names.iter().enumerate() {
                let label = labels.get(i).copied().unwrap_or("Tab");
                let mut grp = Group::new(x, y + 25, w, h - 25, None);
                grp.set_label(label);
                grp.end();
                GUI_WIDGETS.with(|gw| {
                    gw.borrow_mut().insert(grp_name.clone(), GuiWidget::Group(grp));
                });
            }

            tabs.end();
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower.clone(), GuiWidget::Tabs(tabs));
            });
        }
        "RGROUPBOX" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "caption").to_string_val();
            let mut grp = Group::new(x, y, w, h, None);
            grp.set_label(&caption);
            grp.set_frame(FrameType::EngravedBox);
            grp.set_align(Align::TopLeft | Align::Inside);
            grp.end();
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Group(grp));
            });
        }
        "RCODEEDITOR" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let text = rp_comp_get(name, "text").to_string_val();
            let mut buf = TextBuffer::default();
            buf.set_text(&text);

            // Create style buffer for syntax highlighting
            let mut style_buf = TextBuffer::default();
            let style_text = basic_syntax_highlight(&text);
            style_buf.set_text(&style_text);

            // Style table: A=keyword(blue), B=string(burgundy), C=comment(green), D=number(maroon), E=normal
            let styles = vec![
                StyleTableEntry { color: Color::from_rgb(0, 0, 180), font: Font::CourierBold, size: 13 },    // A - keywords
                StyleTableEntry { color: Color::from_rgb(163, 21, 21), font: Font::Courier, size: 13 },       // B - strings
                StyleTableEntry { color: Color::from_rgb(0, 128, 0), font: Font::CourierItalic, size: 13 },   // C - comments
                StyleTableEntry { color: Color::from_rgb(128, 0, 0), font: Font::Courier, size: 13 },         // D - numbers
                StyleTableEntry { color: Color::Black, font: Font::Courier, size: 13 },                       // E - normal
            ];

            let mut editor = TextEditor::new(x, y, w, h, None);
            editor.set_buffer(buf.clone());
            editor.set_text_font(Font::Courier);
            editor.set_text_size(13);
            editor.set_linenumber_width(40);
            editor.set_highlight_data(style_buf.clone(), styles);

            // Store style buffer for re-highlighting when text changes
            GUI_STYLE_BUFFERS.with(|sb| {
                sb.borrow_mut().insert(name_lower.clone(), style_buf);
            });

            // Re-highlight syntax on every text modification (typing, paste, etc.)
            {
                let nl = name_lower.clone();
                buf.add_modify_callback(move |_pos, _ins, _del, _restyled, _deleted_text| {
                    // Use try_borrow to avoid panicking if we're inside gui_set_text
                    // which may still hold a borrow on GUI_TEXT_BUFFERS.
                    GUI_TEXT_BUFFERS.with(|tb| {
                        if let Ok(bufs) = tb.try_borrow() {
                            if let Some(text_buf) = bufs.get(&nl) {
                                let text = text_buf.text();
                                let new_styles = basic_syntax_highlight(&text);
                                GUI_STYLE_BUFFERS.with(|sb| {
                                    if let Ok(mut styles) = sb.try_borrow_mut() {
                                        if let Some(style_buf) = styles.get_mut(&nl) {
                                            style_buf.set_text(&new_styles);
                                        }
                                    }
                                });
                            }
                        }
                    });
                });
            }

            GUI_TEXT_BUFFERS.with(|tb| {
                tb.borrow_mut().insert(name_lower.clone(), buf);
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::TextEditor(editor));
            });
        }
        "RMAINMENU" => {
            // MenuBar placed at the top of the parent form
            // SysMenuBar: on macOS it becomes the native system menu bar,
            // on other platforms it behaves like a normal in-window MenuBar.
            let parent = rp_comp_get(name, "parent").to_string_val();
            let pw = if parent.is_empty() {
                800
            } else {
                rp_comp_get(&parent, "width").to_i64() as i32
            };
            let pw = if parent.is_empty() { pw } else { rp_comp_get(&parent, "clientwidth").to_i64() as i32 };
            let mut mb = SysMenuBar::new(0, 0, pw, 30, None);
            mb.set_text_size(13);
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::SysMenuBar(mb));
            });
        }
        "RMENUITEM" => {
            // Menu items are added to their parent MenuBar
            let caption = rp_comp_get(name, "caption").to_string_val();
            let parent = rp_comp_get(name, "parent").to_string_val();

            // Skip submenu headers (items that have children) — FLTK auto-creates
            // parent submenus when child items use path separators like "&File/&New".
            let has_children = !crate::object::get_children_of(name).is_empty();
            if has_children {
                // This is a submenu header — let children create the submenu automatically
                return;
            }

            if !parent.is_empty() {
                // Walk up to find the MenuBar ancestor
                let mut mb_name = parent.to_lowercase();
                let mut found = false;
                for _ in 0..5 {
                    let ptype = rp_comp_type(&mb_name);
                    if ptype == "RMAINMENU" {
                        found = true;
                        break;
                    }
                    let pp = rp_comp_get(&mb_name, "parent").to_string_val().to_lowercase();
                    if pp.is_empty() { break; }
                    mb_name = pp;
                }
                if found {
                    // Build the full menu path
                    let path = build_menu_path(name);
                    let name_for_cb = name.to_lowercase();
                    GUI_WIDGETS.with(|gw| {
                        let mut widgets = gw.borrow_mut();
                        // Try SysMenuBar first (main menus), then MenuBar (popup menus)
                        if let Some(GuiWidget::SysMenuBar(ref mut mb)) = widgets.get_mut(&mb_name) {
                            if caption == "-" {
                                mb.add_choice(&path);
                            } else {
                                let cb_name = name_for_cb.clone();
                                mb.add(
                                    &path,
                                    fltk::enums::Shortcut::None,
                                    fltk::menu::MenuFlag::Normal,
                                    move |_| {
                                        rp_fire_event(&cb_name, "onclick");
                                    },
                                );
                            }
                        } else if let Some(GuiWidget::MenuBar(ref mut mb)) = widgets.get_mut(&mb_name) {
                            if caption == "-" {
                                mb.add_choice(&path);
                            } else {
                                let cb_name = name_for_cb.clone();
                                mb.add(
                                    &path,
                                    fltk::enums::Shortcut::None,
                                    fltk::menu::MenuFlag::Normal,
                                    move |_| {
                                        rp_fire_event(&cb_name, "onclick");
                                    },
                                );
                            }
                        }
                    });
                }
            }
            // MenuItems don't get their own widget entry
        }
        "RDESIGNSURFACE" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "formcaption").to_string_val();
            let cap = if caption.is_empty() { "Form1".to_string() } else { caption.clone() };

            // Check if this design surface has a parent (embedded in a form)
            let parent = rp_comp_get(name, "parent").to_string_val();
            let embedded = !parent.is_empty();

            if embedded {
                // Embedded design surface: use a Frame with custom draw/handle
                let mut frm = Frame::new(x, y, w, h, None);
                frm.set_frame(FrameType::DownBox);
                frm.set_color(Color::White);
                let ds_name = name_lower.clone();
                frm.draw(move |wid| {
                    draw_design_surface(&ds_name, wid.x(), wid.y(), wid.w(), wid.h());
                });
                let ds_name2 = name_lower.clone();
                frm.handle(move |wid, ev| {
                    handle_design_surface_frame_event(&ds_name2, wid, ev)
                });
                DESIGN_SURFACES.with(|ds| {
                    ds.borrow_mut().insert(name_lower.clone(), DesignState {
                        components: Vec::new(),
                        selected: -1,
                        form_w: w,
                        form_h: h,
                        form_caption: cap,
                        drag_mode: 0,
                        drag_offset_x: 0,
                        drag_offset_y: 0,
                    });
                });
                GUI_WIDGETS.with(|gw| {
                    gw.borrow_mut().insert(name_lower, GuiWidget::Frame(frm));
                });
            } else {
                // Standalone design surface: use a Window
                let mut win = Window::new(200, 200, w, h, None);
                win.set_label(&cap);
                win.set_color(Color::White);
                let ds_name = name_lower.clone();
                win.draw(move |wid| {
                    draw_design_surface(&ds_name, wid.x(), wid.y(), wid.w(), wid.h());
                });
                let ds_name2 = name_lower.clone();
                win.handle(move |wid, ev| {
                    handle_design_surface_event(&ds_name2, wid, ev)
                });
                win.end();
                DESIGN_SURFACES.with(|ds| {
                    ds.borrow_mut().insert(name_lower.clone(), DesignState {
                        components: Vec::new(),
                        selected: -1,
                        form_w: w,
                        form_h: h,
                        form_caption: cap,
                        drag_mode: 0,
                        drag_offset_x: 0,
                        drag_offset_y: 0,
                    });
                });
                GUI_WIDGETS.with(|gw| {
                    gw.borrow_mut().insert(name_lower, GuiWidget::Window(win));
                });
            }
        }
        "RSTRINGGRID" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            grid_create(&name_lower, x, y, w, h);
        }
        "RTREEVIEW" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut tree = Tree::new(x, y, w, h, None);
            tree.set_show_root(false);
            let name_for_cb = name.to_lowercase();
            tree.set_callback(move |t| {
                // Store the selected item label as a property
                if let Some(item) = t.first_selected_item() {
                    if let Some(label) = item.label() {
                        // Extract just the leaf label (after last '/')
                        let leaf = label.rsplit('/').next().unwrap_or(&label);
                        rp_comp_set(&name_for_cb, "selecteditem", v_str(leaf));
                    }
                }
                rp_fire_event(&name_for_cb, "onclick");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Tree(tree));
            });
        }
        "RTRACKBAR" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let min = rp_comp_get(name, "min").to_f64();
            let max = rp_comp_get(name, "max").to_f64();
            let pos = rp_comp_get(name, "position").to_f64();
            let mut slider = HorNiceSlider::new(x, y, w, h, None);
            slider.set_minimum(min);
            slider.set_maximum(max);
            slider.set_value(pos);
            slider.set_step(1.0, 1);
            let name_for_cb = name.to_lowercase();
            slider.set_callback(move |s| {
                rp_comp_set(&name_for_cb, "position", v_int(s.value() as i64));
                rp_fire_event(&name_for_cb, "onchange");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Slider(slider));
            });
        }
        "RCANVAS" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let bg_color = rp_comp_get(name, "color").to_i64();
            let mut frm = Frame::new(x, y, w, h, None);
            frm.set_frame(FrameType::FlatBox);
            frm.set_color(bgr_to_fltk_color(bg_color));

            // The canvas's surface is a shared model (rapidr_value::objects,
            // a Bitmap): every drawing method draws there, this shows it.
            let name_for_draw = name.to_lowercase();
            frm.draw(move |f| {
                let (ox, oy, w, h) = (f.x(), f.y(), f.w(), f.h());
                draw::draw_rect_fill(ox, oy, w, h, f.color());
                let rgba = rapidr_value::objects::with_canvas(&name_for_draw, w as i64, h as i64, |b| b.to_rgba());
                if let Some(rgba) = rgba {
                    if let Ok(mut img) = RgbImage::new(&rgba, w, h, ColorDepth::Rgba8) {
                        img.draw(ox, oy, w, h);
                    }
                }
            });

            let name_for_cb = name.to_lowercase();
            frm.handle(move |_, ev| {
                match ev {
                    Event::Push => {
                        press_begin(&name_for_cb);
                        let mx = app::event_x();
                        let my = app::event_y();
                        rp_fire_event(&name_for_cb, "onclick");
                        rp_fire_event_2(&name_for_cb, "onmousedown", v_int(mx as i64), v_int(my as i64));
                        true
                    }
                    Event::Released => {
                        if press_end(&name_for_cb) {
                            let mx = app::event_x();
                            let my = app::event_y();
                            rp_fire_event_2(&name_for_cb, "onmouseup", v_int(mx as i64), v_int(my as i64));
                        }
                        true
                    }
                    Event::Move | Event::Drag => {
                        let mx = app::event_x();
                        let my = app::event_y();
                        rp_fire_event_2(&name_for_cb, "onmousemove", v_int(mx as i64), v_int(my as i64));
                        true
                    }
                    _ => false,
                }
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Frame(frm));
            });
        }
        "RIMAGE" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut frm = Frame::new(x, y, w, h, None);
            frm.set_frame(FrameType::FlatBox);
            // A picture other than a BMP (PNG, JPEG, …: FLTK reads it).
            let file = rp_comp_get(name, "__imagefile").to_string_val();
            if !file.is_empty() {
                if let Some(mut img) = load_shared_image(&file) {
                    let stretch = rp_comp_get(name, "stretch").to_i64() != 0;
                    if stretch {
                        img.scale(w, h, true, true);
                    }
                    frm.set_image(Some(img));
                }
            }
            frm.handle(picture_mouse(&name_lower));
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower.clone(), GuiWidget::ImageFrame(frm));
            });
            picture_refresh(&name_lower);
        }
        "RSPLITTER" => {
            // Dragging it resizes the control next to it (layout.rs,
            // rapidr_value::layout::splitter_drag).
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut bar = Frame::new(x, y, w, h, None);
            bar.set_frame(FrameType::ThinUpBox);
            let id = name_lower.clone();
            let mut start = None::<(i32, i32)>;
            bar.handle(move |_, ev| {
                let vertical = matches!(rp_comp_get(&id, "align").to_i64(), 1 | 2);
                let cursor = if vertical { fltk::enums::Cursor::NS } else { fltk::enums::Cursor::WE };
                match ev {
                    Event::Enter => {
                        draw::set_cursor(cursor);
                        true
                    }
                    Event::Leave => {
                        if start.is_none() {
                            draw::set_cursor(fltk::enums::Cursor::Default);
                        }
                        true
                    }
                    Event::Push => {
                        if crate::layout::splitter_begin(&id) {
                            start = Some((app::event_x_root(), app::event_y_root()));
                        }
                        true
                    }
                    Event::Drag => {
                        if let Some((sx, sy)) = start {
                            let delta = if vertical { app::event_y_root() - sy } else { app::event_x_root() - sx };
                            crate::layout::splitter_move(delta as i64);
                        }
                        true
                    }
                    Event::Released => {
                        if start.take().is_some() {
                            crate::layout::splitter_end();
                        }
                        draw::set_cursor(fltk::enums::Cursor::Default);
                        true
                    }
                    _ => false,
                }
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Frame(bar));
            });
        }
        "RSCROLLBOX" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut scroll = Scroll::new(x, y, w, h, None);
            scroll.set_frame(FrameType::DownBox);
            scroll.end();
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Scroll(scroll));
            });
        }
        "RLISTVIEW" => {
            // Multi-column list — use HoldBrowser as a simple approximation
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut browser = HoldBrowser::new(x, y, w, h, None);
            browser.set_column_char('\t');
            browser.set_frame(FrameType::DownBox);
            browser.set_color(Color::White);
            let name_for_cb = name_lower.clone();
            browser.set_callback(move |b| {
                let line = b.value();
                if line <= 0 {
                    return;
                }
                let header = listview_header_shown(&name_for_cb);
                if header && line == 1 {
                    // The column header: OnColumnClick(Column%).
                    let widths = rapidr_value::objects::with_listview(&name_for_cb, |lv| lv.columns.iter().map(|c| c.width).collect::<Vec<_>>()).unwrap_or_default();
                    let sel = rapidr_value::objects::with_listview(&name_for_cb, |lv| lv.item_index).unwrap_or(-1);
                    b.deselect(0);
                    if sel >= 0 {
                        b.select(sel as i32 + 2);
                    }
                    let mut edge = b.x() as i64;
                    let x = app::event_x() as i64;
                    let column = widths.iter().position(|w| {
                        edge += w.max(&1);
                        x < edge
                    });
                    let column = column.unwrap_or(widths.len().saturating_sub(1)) as i64;
                    rp_fire_event_1(&name_for_cb, "oncolumnclick", v_int(column));
                    return;
                }
                let idx = line as i64 - 1 - header as i64;
                rapidr_value::objects::set(&name_for_cb, "itemindex", &v_int(idx));
                rp_fire_event(&name_for_cb, if app::event_clicks() { "ondblclick" } else { "onclick" });
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::HoldBrowser(browser));
            });
            listview_refresh(name);
        }
        "RMDICHILD" => {
            let (x, y, w, h) = (rp_comp_get(name, "left").to_i64() as i32, rp_comp_get(name, "top").to_i64() as i32, rp_comp_get(name, "width").to_i64() as i32, rp_comp_get(name, "height").to_i64() as i32);
            mdi_frame_create(name, x, y, w, h);
        }
        "RPROGRESSBAR" => {
            // Alias for RPROGRESS — uses the same FltkProgress widget
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let min_val = rp_comp_get(name, "min").to_i64() as f64;
            let max_val = rp_comp_get(name, "max").to_i64() as f64;
            let pos = rp_comp_get(name, "position").to_i64() as f64;
            let mut prog = FltkProgress::new(x, y, w, h, None);
            prog.set_minimum(min_val);
            prog.set_maximum(max_val);
            prog.set_value(pos);
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Progress(prog));
            });
        }
        "RPOPUPMENU" => {
            // Popup menu — use MenuBar at y=-30 so it's hidden until popup() is called
            let mb = MenuBar::new(-1000, -1000, 100, 30, None);
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::MenuBar(mb));
            });
        }
        "RSCROLLBAR" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let min_val = rp_comp_get(name, "min").to_i64() as f64;
            let max_val = rp_comp_get(name, "max").to_i64() as f64;
            let pos = rp_comp_get(name, "position").to_i64() as f64;
            let mut slider = HorNiceSlider::new(x, y, w, h, None);
            slider.set_minimum(min_val);
            slider.set_maximum(max_val);
            slider.set_value(pos);
            let name_for_cb = name.to_lowercase();
            slider.set_callback(move |s| {
                rp_comp_set(&name_for_cb, "position", v_int(s.value() as i64));
                rp_fire_event(&name_for_cb, "onchange");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Slider(slider));
            });
        }
        _ => {
            // Unknown GUI component type — skip widget creation
        }
    }

    // Apply initial visibility — hide widgets where Visible is explicitly set to 0
    // (missing or unset 'visible' defaults to visible)
    let vis_val = rp_comp_get(name, "visible");
    let explicitly_hidden = match &vis_val {
        v if v.to_string_val() == "false" => true,
        v if v.to_string_val() == "0" => true,
        _ => false,
    };
    if explicitly_hidden {
        gui_set_visible(name, false);
    }
    gui_apply_font(name);
}

/// Applies the font a program gave a component (`.Font = F`, `.FontName`,
/// `.Font.Size`, …) to its widget. Font names map to FLTK's portable faces
/// (Courier, Times, Symbol, otherwise Helvetica); sizes are pixels, as on
/// the web.
pub fn gui_apply_font(name: &str) {
    if !rp_comp_get(name, "__fontset").to_bool() {
        return;
    }
    let face = rp_comp_get(name, "fontname").to_string_val().to_lowercase();
    let (bold, italic) = (rp_comp_get(name, "fontbold").to_bool(), rp_comp_get(name, "fontitalic").to_bool());
    let has = |words: &[&str]| words.iter().any(|w| face.contains(w));
    let font = if has(&["courier", "mono", "consol", "fixed"]) {
        [Font::Courier, Font::CourierBold, Font::CourierItalic, Font::CourierBoldItalic]
    } else if has(&["times", "roman", "georgia", "garamond"]) || (face.contains("serif") && !face.contains("sans")) {
        [Font::Times, Font::TimesBold, Font::TimesItalic, Font::TimesBoldItalic]
    } else if has(&["symbol"]) {
        [Font::Symbol; 4]
    } else {
        [Font::Helvetica, Font::HelveticaBold, Font::HelveticaItalic, Font::HelveticaBoldItalic]
    }[usize::from(bold) + 2 * usize::from(italic)];
    let size = rp_comp_get(name, "fontsize").to_i64();
    let size = if size > 0 { size.min(512) as i32 } else { app::font_size() };
    let color = match rp_comp_get(name, "fontcolor") {
        Value::Null => None,
        c => Some(bgr_to_fltk_color(c.to_i64())),
    };
    macro_rules! label {
        ($w:expr) => {{
            $w.set_label_font(font);
            $w.set_label_size(size);
            if let Some(c) = color {
                $w.set_label_color(c);
            }
        }};
    }
    macro_rules! text {
        ($w:expr) => {{
            label!($w);
            $w.set_text_font(font);
            $w.set_text_size(size);
            if let Some(c) = color {
                $w.set_text_color(c);
            }
        }};
    }
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        let Some(widget) = widgets.get_mut(&name.to_lowercase()) else { return };
        match widget {
            GuiWidget::Window(w) => label!(w),
            GuiWidget::Button(w) => label!(w),
            GuiWidget::Frame(w) | GuiWidget::ImageFrame(w) => label!(w),
            GuiWidget::Input(w) => text!(w),
            GuiWidget::CheckButton(w) => label!(w),
            GuiWidget::RadioButton(w) => label!(w),
            GuiWidget::Choice(w) => text!(w),
            GuiWidget::InputChoice(w) => {
                label!(w);
                w.set_text_font(font);
                w.set_text_size(size);
                if let Some(c) = color {
                    w.input().set_text_color(c);
                }
            }
            GuiWidget::HoldBrowser(w) => {
                label!(w);
                w.set_text_size(size);
            }
            GuiWidget::TextEditor(w) => text!(w),
            GuiWidget::Group(w) => label!(w),
            GuiWidget::Tabs(w) => label!(w),
            GuiWidget::MenuBar(w) => text!(w),
            GuiWidget::SysMenuBar(w) => text!(w),
            GuiWidget::Progress(w) => label!(w),
            GuiWidget::Scroll(w) => label!(w),
            GuiWidget::Grid(w, _) => label!(w),
            GuiWidget::Tree(w) => label!(w),
            GuiWidget::Slider(w) => label!(w),
        }
    });
    redraw_widget(&name.to_lowercase());
}

// ---------------------------------------------------------------------------
// Helper: build menu path for a PMENUITEM
// ---------------------------------------------------------------------------

/// Walk up the parent chain from a PMENUITEM to build the full menu path
/// e.g. "File/&New" or "File/Save As..."
fn build_menu_path(name: &str) -> String {
    use crate::object::rp_comp_type;
    let mut parts = Vec::new();
    let my_caption = rp_comp_get(name, "caption").to_string_val();
    parts.push(my_caption);

    let mut current = name.to_lowercase();
    loop {
        let parent = rp_comp_get(&current, "parent").to_string_val().to_lowercase();
        if parent.is_empty() { break; }
        let ptype = rp_comp_type(&parent);
        if ptype == "RMENUITEM" {
            let pcap = rp_comp_get(&parent, "caption").to_string_val();
            parts.push(pcap);
        } else {
            // Reached the MenuBar — stop
            break;
        }
        current = parent;
    }
    parts.reverse();
    parts.join("/")
}

// ---------------------------------------------------------------------------
// BASIC syntax highlighting for code editor
// ---------------------------------------------------------------------------

/// Generate a style string for BASIC syntax highlighting.
/// A=keyword, B=string, C=comment, D=number, E=normal
fn basic_syntax_highlight(source: &str) -> String {
    static KEYWORDS: &[&str] = &[
        "SUB", "END", "FUNCTION", "DIM", "AS", "IF", "THEN", "ELSE", "ELSEIF",
        "FOR", "TO", "STEP", "NEXT", "WHILE", "WEND", "DO", "LOOP", "UNTIL",
        "SELECT", "CASE", "EXIT", "CREATE", "INTEGER", "STRING", "DOUBLE", "BOOLEAN",
        "AND", "OR", "NOT", "MOD", "TRUE", "FALSE", "CONST", "RETURN",
        "PRINT", "MSGBOX", "SHELL", "SHELLWAIT", "CALL",
        "RFORM", "RBUTTON", "RLABEL", "REDIT", "RCHECKBOX", "RRADIOBUTTON",
        "RCOMBOBOX", "RLISTBOX", "RPANEL", "RGROUPBOX", "RDESIGNSURFACE",
        "RCODEEDITOR", "RSTRINGGRID", "RTREEVIEW", "RCANVAS", "RTIMER",
        "RIMAGE", "RRICHEDIT", "RPROGRESSBAR", "RTRACKBAR", "RSCROLLBAR",
        "RSPLITTER", "RMAINMENU", "RMENUITEM", "RMYSQL", "RSQLITE",
        "RCOOLBTN", "ROVALBTN",
        "ROPENDIALOG", "RSAVEDIALOG", "RCOLORDIALOG", "RFONTDIALOG",
        "RFILESTREAM", "RJSON", "RHTTP", "RSOCKET", "$THEME",
        "LEFT", "RIGHT", "MID", "LEN", "INSTR", "UCASE", "LCASE",
        "VAL", "STR", "CHR", "ASC", "TRIM",
    ];

    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut styles = vec![b'E'; len];
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // Comment: ' to end of line
        if ch == '\'' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            for j in start..i {
                styles[j] = b'C';
            }
            continue;
        }

        // String literal: "..."
        if ch == '"' {
            let start = i;
            i += 1;
            while i < len && chars[i] != '"' && chars[i] != '\n' {
                i += 1;
            }
            if i < len && chars[i] == '"' {
                i += 1;
            }
            for j in start..i {
                styles[j] = b'B';
            }
            continue;
        }

        // Number
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            for j in start..i {
                styles[j] = b'D';
            }
            continue;
        }

        // Word (identifier or keyword)
        if ch.is_ascii_alphabetic() || ch == '_' || ch == '$' {
            let start = i;
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '$') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let upper = word.to_uppercase();
            if KEYWORDS.contains(&upper.as_str()) {
                for j in start..i {
                    styles[j] = b'A';
                }
            }
            continue;
        }

        i += 1;
    }

    String::from_utf8(styles).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Design surface rendering
// ---------------------------------------------------------------------------

/// Parse a color string (hex like "#RRGGBB" or "rgb(r,g,b)") into an FLTK Color.
fn parse_color_prop(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.starts_with('#') && s.len() >= 7 {
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        return Some(Color::from_rgb(r, g, b));
    }
    if s.starts_with("rgb(") && s.ends_with(')') {
        let inner = &s[4..s.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            return Some(Color::from_rgb(r, g, b));
        }
    }
    None
}

fn draw_design_surface(ds_name: &str, x: i32, y: i32, w: i32, h: i32) {
    // White background
    draw::set_draw_color(Color::White);
    draw::draw_rectf(x, y, w, h);

    // Grid dots
    draw::set_draw_color(Color::from_rgb(200, 200, 200));
    let mut gx = 0;
    while gx < w {
        let mut gy = 0;
        while gy < h {
            draw::draw_point(x + gx, y + gy);
            gy += 8;
        }
        gx += 8;
    }

    // Draw placed components with realistic widget appearances
    DESIGN_SURFACES.with(|ds| {
        let surfaces = ds.borrow();
        if let Some(state) = surfaces.get(ds_name) {
            for (i, comp) in state.components.iter().enumerate() {
                let cx = x + comp.x;
                let cy = y + comp.y;
                let label = comp.props.get("caption").unwrap_or(&comp.name);
                let tn = comp.type_name.as_str();

                // Resolve font from font.name/fontname property
                let comp_font = comp.props.get("font.name")
                    .or_else(|| comp.props.get("fontname"))
                    .and_then(|fn_name| {
                        if fn_name.is_empty() { return None; }
                        let font_names = app::get_font_names();
                        font_names.iter().position(|n| n.eq_ignore_ascii_case(fn_name))
                            .map(|idx| Font::by_index(idx))
                    })
                    .unwrap_or(Font::Helvetica);
                let comp_font_size = comp.props.get("font.size")
                    .or_else(|| comp.props.get("fontsize"))
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(12);

                match tn {
                    "RBUTTON" => {
                        // 3D raised button look
                        let bg = comp.props.get("color").and_then(|c| parse_color_prop(c)).unwrap_or(Color::from_rgb(225, 225, 225));
                        let (br, bg_g, bb) = bg.to_rgb();
                        draw::set_draw_color(bg);
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        // Highlight (top-left)
                        draw::set_draw_color(Color::from_rgb(br.saturating_add(30).min(255), bg_g.saturating_add(30).min(255), bb.saturating_add(30).min(255)));
                        draw::draw_line(cx, cy, cx + comp.w - 1, cy);
                        draw::draw_line(cx, cy, cx, cy + comp.h - 1);
                        // Shadow (bottom-right)
                        draw::set_draw_color(Color::from_rgb(br.saturating_sub(85), bg_g.saturating_sub(85), bb.saturating_sub(85)));
                        draw::draw_line(cx + comp.w - 1, cy, cx + comp.w - 1, cy + comp.h - 1);
                        draw::draw_line(cx, cy + comp.h - 1, cx + comp.w - 1, cy + comp.h - 1);
                        // Label centered
                        let fc = comp.props.get("fontcolor").and_then(|c| parse_color_prop(c)).unwrap_or(Color::Black);
                        draw::set_draw_color(fc);
                        draw::set_font(comp_font, comp_font_size);
                        draw::draw_text2(label, cx, cy, comp.w, comp.h, Align::Center);
                    }
                    "RLABEL" => {
                        // Labels: transparent background, just text
                        let fc = comp.props.get("fontcolor").and_then(|c| parse_color_prop(c)).unwrap_or(Color::Black);
                        draw::set_draw_color(fc);
                        draw::set_font(comp_font, comp_font_size);
                        draw::draw_text2(label, cx + 2, cy, comp.w - 4, comp.h, Align::Left | Align::Inside);
                    }
                    "REDIT" => {
                        // Sunken text field
                        draw::set_draw_color(Color::White);
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        // Sunken border
                        draw::set_draw_color(Color::from_rgb(130, 130, 130));
                        draw::draw_line(cx, cy, cx + comp.w - 1, cy);
                        draw::draw_line(cx, cy, cx, cy + comp.h - 1);
                        draw::set_draw_color(Color::from_rgb(245, 245, 245));
                        draw::draw_line(cx + comp.w - 1, cy, cx + comp.w - 1, cy + comp.h - 1);
                        draw::draw_line(cx, cy + comp.h - 1, cx + comp.w - 1, cy + comp.h - 1);
                        // Text content
                        let text = comp.props.get("text").map(|s| s.as_str()).unwrap_or(&comp.name);
                        draw::set_draw_color(Color::Black);
                        draw::set_font(Font::Helvetica, 12);
                        draw::draw_text2(text, cx + 4, cy, comp.w - 8, comp.h, Align::Left | Align::Inside);
                    }
                    "RCHECKBOX" => {
                        // Checkbox: box + label
                        let bx = cx + 2;
                        let by = cy + (comp.h - 13) / 2;
                        draw::set_draw_color(Color::White);
                        draw::draw_rectf(bx, by, 13, 13);
                        draw::set_draw_color(Color::from_rgb(130, 130, 130));
                        draw::draw_rect(bx, by, 13, 13);
                        let checked = comp.props.get("checked").map(|s| s.as_str()).unwrap_or("0");
                        if checked == "1" || checked.eq_ignore_ascii_case("true") {
                            draw::set_draw_color(Color::Black);
                            draw::draw_line(bx + 2, by + 6, bx + 5, by + 10);
                            draw::draw_line(bx + 5, by + 10, bx + 11, by + 2);
                        }
                        draw::set_draw_color(Color::Black);
                        draw::set_font(Font::Helvetica, 12);
                        draw::draw_text2(label, cx + 18, cy, comp.w - 20, comp.h, Align::Left | Align::Inside);
                    }
                    "RRADIOBUTTON" => {
                        // Radiobutton: circle + label
                        let ry = cy + comp.h / 2;
                        draw::set_draw_color(Color::White);
                        draw::draw_pie(cx + 2, ry - 6, 13, 13, 0.0, 360.0);
                        draw::set_draw_color(Color::from_rgb(130, 130, 130));
                        draw::draw_arc(cx + 2, ry - 6, 13, 13, 0.0, 360.0);
                        draw::set_draw_color(Color::Black);
                        draw::set_font(Font::Helvetica, 12);
                        draw::draw_text2(label, cx + 18, cy, comp.w - 20, comp.h, Align::Left | Align::Inside);
                    }
                    "RCOMBOBOX" => {
                        // Combo: edit field + dropdown arrow
                        draw::set_draw_color(Color::White);
                        draw::draw_rectf(cx, cy, comp.w - 18, comp.h);
                        draw::set_draw_color(Color::from_rgb(130, 130, 130));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                        // Arrow button
                        draw::set_draw_color(Color::from_rgb(225, 225, 225));
                        draw::draw_rectf(cx + comp.w - 18, cy + 1, 17, comp.h - 2);
                        draw::set_draw_color(Color::Black);
                        let ax = cx + comp.w - 12;
                        let ay = cy + comp.h / 2 - 1;
                        draw::draw_line(ax - 3, ay, ax + 3, ay);
                        draw::draw_line(ax - 2, ay + 1, ax + 2, ay + 1);
                        draw::draw_line(ax - 1, ay + 2, ax + 1, ay + 2);
                        // Text
                        draw::set_draw_color(Color::Black);
                        draw::set_font(Font::Helvetica, 12);
                        draw::draw_text2(&comp.name, cx + 4, cy, comp.w - 22, comp.h, Align::Left | Align::Inside);
                    }
                    "RLISTBOX" => {
                        // Listbox: sunken box with lines
                        draw::set_draw_color(Color::White);
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(130, 130, 130));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                        // Draw a few placeholder lines
                        draw::set_draw_color(Color::from_rgb(180, 180, 180));
                        draw::set_font(Font::Helvetica, 11);
                        draw::draw_text2("(ListBox)", cx + 4, cy + 2, comp.w - 8, 16, Align::Left | Align::Inside);
                    }
                    "RPANEL" | "RGROUPBOX" => {
                        // Panel/Group: etched border with optional caption
                        let bg = comp.props.get("color").and_then(|c| parse_color_prop(c)).unwrap_or(Color::from_rgb(240, 240, 240));
                        draw::set_draw_color(bg);
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        if tn == "RGROUPBOX" {
                            // Group box with caption in top border
                            draw::set_font(Font::Helvetica, 11);
                            let tw = draw::width(label) as i32 + 8;
                            draw::set_draw_color(Color::from_rgb(160, 160, 160));
                            draw::draw_line(cx, cy + 8, cx + 6, cy + 8);
                            draw::draw_line(cx + 6 + tw, cy + 8, cx + comp.w - 1, cy + 8);
                            draw::draw_line(cx, cy + 8, cx, cy + comp.h - 1);
                            draw::draw_line(cx + comp.w - 1, cy + 8, cx + comp.w - 1, cy + comp.h - 1);
                            draw::draw_line(cx, cy + comp.h - 1, cx + comp.w - 1, cy + comp.h - 1);
                            draw::set_draw_color(Color::Black);
                            draw::draw_text2(label, cx + 10, cy, tw, 16, Align::Left | Align::Inside);
                        } else {
                            draw::set_draw_color(Color::from_rgb(180, 180, 180));
                            draw::draw_rect(cx, cy, comp.w, comp.h);
                        }
                    }
                    "RPROGRESSBAR" => {
                        // Progress bar
                        draw::set_draw_color(Color::from_rgb(230, 230, 230));
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(60, 130, 200));
                        draw::draw_rectf(cx + 1, cy + 1, comp.w / 3, comp.h - 2);
                        draw::set_draw_color(Color::from_rgb(160, 160, 160));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                    }
                    "RTIMER" => {
                        // Timer: non-visual component icon
                        draw::set_draw_color(Color::from_rgb(240, 240, 255));
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(100, 100, 200));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(60, 60, 160));
                        draw::set_font(Font::Helvetica, 10);
                        draw::draw_text2(&comp.name, cx, cy, comp.w, comp.h, Align::Center);
                        draw::set_font(Font::Helvetica, 8);
                        draw::draw_text2("[Timer]", cx, cy + comp.h / 2 + 2, comp.w, comp.h / 2, Align::Top | Align::Center);
                    }
                    "RRICHEDIT" | "RMEMO" => {
                        // Multi-line text box: sunken
                        draw::set_draw_color(Color::White);
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(130, 130, 130));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(180, 180, 180));
                        draw::set_font(Font::Courier, 11);
                        draw::draw_text2("(RichEdit)", cx + 4, cy + 2, comp.w - 8, 16, Align::Left | Align::Inside);
                    }
                    "RCANVAS" => {
                        // Canvas area
                        let bg = comp.props.get("color").and_then(|c| parse_color_prop(c)).unwrap_or(Color::White);
                        draw::set_draw_color(bg);
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(160, 160, 160));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                        // Draw crosshairs to indicate canvas
                        draw::set_draw_color(Color::from_rgb(210, 210, 210));
                        draw::draw_line(cx + comp.w / 2, cy, cx + comp.w / 2, cy + comp.h);
                        draw::draw_line(cx, cy + comp.h / 2, cx + comp.w, cy + comp.h / 2);
                        draw::set_draw_color(Color::from_rgb(130, 130, 130));
                        draw::set_font(Font::Helvetica, 10);
                        draw::draw_text2(&comp.name, cx, cy, comp.w, comp.h, Align::Center);
                    }
                    "RIMAGE" => {
                        // Image placeholder
                        draw::set_draw_color(Color::from_rgb(245, 245, 245));
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(180, 180, 180));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                        // Diagonal lines to indicate image area
                        draw::draw_line(cx, cy, cx + comp.w, cy + comp.h);
                        draw::draw_line(cx + comp.w, cy, cx, cy + comp.h);
                        draw::set_draw_color(Color::from_rgb(130, 130, 130));
                        draw::set_font(Font::Helvetica, 10);
                        draw::draw_text2(&comp.name, cx, cy, comp.w, comp.h, Align::Center);
                    }
                    "RTREEVIEW" => {
                        // Tree view
                        draw::set_draw_color(Color::White);
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(130, 130, 130));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(100, 100, 100));
                        draw::set_font(Font::Helvetica, 10);
                        draw::draw_text2("+ Item 1", cx + 6, cy + 4, comp.w - 12, 14, Align::Left | Align::Inside);
                        draw::draw_text2("+ Item 2", cx + 6, cy + 18, comp.w - 12, 14, Align::Left | Align::Inside);
                    }
                    "RTRACKBAR" => {
                        // Trackbar / slider
                        let track_y = cy + comp.h / 2;
                        draw::set_draw_color(Color::from_rgb(180, 180, 180));
                        draw::draw_rectf(cx + 4, track_y - 2, comp.w - 8, 4);
                        // Thumb
                        let thumb_x = cx + comp.w / 3;
                        draw::set_draw_color(Color::from_rgb(200, 200, 200));
                        draw::draw_rectf(thumb_x - 5, cy + 4, 10, comp.h - 8);
                        draw::set_draw_color(Color::from_rgb(130, 130, 130));
                        draw::draw_rect(thumb_x - 5, cy + 4, 10, comp.h - 8);
                    }
                    "RSTRINGGRID" => {
                        // Grid
                        draw::set_draw_color(Color::White);
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(200, 210, 230));
                        draw::draw_rectf(cx, cy, comp.w, 20);
                        draw::set_draw_color(Color::from_rgb(160, 160, 160));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                        // Grid lines
                        let mid_x = cx + comp.w / 2;
                        draw::draw_line(mid_x, cy, mid_x, cy + comp.h);
                        for row in 0..4 {
                            let ly = cy + row * 20;
                            draw::draw_line(cx, ly, cx + comp.w, ly);
                        }
                        draw::set_draw_color(Color::Black);
                        draw::set_font(Font::Helvetica, 10);
                        draw::draw_text2(&comp.name, cx + 2, cy + 2, comp.w - 4, 16, Align::Left | Align::Inside);
                    }
                    "RMYSQL" | "RSQLITE" => {
                        // Database: non-visual icon
                        draw::set_draw_color(Color::from_rgb(255, 245, 230));
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(180, 140, 80));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(120, 80, 30));
                        draw::set_font(Font::Helvetica, 10);
                        let db_label = if tn == "RMYSQL" { "MySQL" } else { "SQLite" };
                        draw::draw_text2(db_label, cx, cy + 2, comp.w, comp.h / 2, Align::Center);
                        draw::set_font(Font::Helvetica, 9);
                        draw::draw_text2(&comp.name, cx, cy + comp.h / 2, comp.w, comp.h / 2, Align::Center);
                    }
                    "RCOOLBTN" => {
                        // Cool button: flat style with optional pressed look
                        let is_down = comp.props.get("down").map(|s| s == "1" || s.eq_ignore_ascii_case("true")).unwrap_or(false);
                        let is_flat = comp.props.get("flat").map(|s| s == "1" || s.eq_ignore_ascii_case("true")).unwrap_or(false);
                        if is_down {
                            draw::set_draw_color(Color::from_rgb(200, 210, 230));
                            draw::draw_rectf(cx, cy, comp.w, comp.h);
                            draw::set_draw_color(Color::from_rgb(130, 130, 130));
                            draw::draw_rect(cx, cy, comp.w, comp.h);
                        } else if !is_flat {
                            let bg = Color::from_rgb(225, 225, 225);
                            let (br, bg_g, bb) = bg.to_rgb();
                            draw::set_draw_color(bg);
                            draw::draw_rectf(cx, cy, comp.w, comp.h);
                            draw::set_draw_color(Color::from_rgb(br.saturating_add(30).min(255), bg_g.saturating_add(30).min(255), bb.saturating_add(30).min(255)));
                            draw::draw_line(cx, cy, cx + comp.w - 1, cy);
                            draw::draw_line(cx, cy, cx, cy + comp.h - 1);
                            draw::set_draw_color(Color::from_rgb(br.saturating_sub(85), bg_g.saturating_sub(85), bb.saturating_sub(85)));
                            draw::draw_line(cx + comp.w - 1, cy, cx + comp.w - 1, cy + comp.h - 1);
                            draw::draw_line(cx, cy + comp.h - 1, cx + comp.w - 1, cy + comp.h - 1);
                        } else {
                            draw::set_draw_color(Color::from_rgb(240, 240, 240));
                            draw::draw_rectf(cx, cy, comp.w, comp.h);
                        }
                        let fc = comp.props.get("fontcolor").and_then(|c| parse_color_prop(c)).unwrap_or(Color::Black);
                        draw::set_draw_color(fc);
                        draw::set_font(comp_font, comp_font_size);
                        draw::draw_text2(label, cx, cy, comp.w, comp.h, Align::Center);
                    }
                    "ROVALBTN" => {
                        // Oval button
                        let bg_c = comp.props.get("color").and_then(|c| parse_color_prop(c)).unwrap_or(Color::from_rgb(220, 220, 220));
                        draw::set_draw_color(bg_c);
                        draw::draw_pie(cx, cy, comp.w, comp.h, 0.0, 360.0);
                        draw::set_draw_color(Color::from_rgb(255, 255, 255));
                        draw::draw_arc(cx, cy, comp.w, comp.h, 45.0, 225.0);
                        draw::set_draw_color(Color::from_rgb(128, 128, 128));
                        draw::draw_arc(cx, cy, comp.w, comp.h, 225.0, 405.0);
                        let fc = comp.props.get("fontcolor").and_then(|c| parse_color_prop(c)).unwrap_or(Color::Black);
                        draw::set_draw_color(fc);
                        draw::set_font(comp_font, comp_font_size);
                        draw::draw_text2(label, cx, cy, comp.w, comp.h, Align::Center);
                    }
                    _ => {
                        // Generic fallback
                        draw::set_draw_color(Color::from_rgb(236, 236, 236));
                        draw::draw_rectf(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::from_rgb(160, 160, 160));
                        draw::draw_rect(cx, cy, comp.w, comp.h);
                        draw::set_draw_color(Color::Black);
                        draw::set_font(Font::Helvetica, 11);
                        draw::draw_text2(label, cx + 2, cy + 2, comp.w - 4, comp.h - 4, Align::Center);
                        draw::set_font(Font::Helvetica, 9);
                        draw::set_draw_color(Color::from_rgb(100, 100, 100));
                        draw::draw_text2(&comp.type_name, cx + 2, cy + 1, comp.w - 4, 12, Align::TopLeft);
                    }
                }

                // Selection border (blue highlight over everything)
                if i as i32 == state.selected {
                    draw::set_draw_color(Color::from_rgb(0, 120, 215));
                    draw::draw_rect(cx, cy, comp.w, comp.h);
                    draw_selection_handles(cx, cy, comp.w, comp.h);
                }
            }
        }
    });
}

fn draw_selection_handles(x: i32, y: i32, w: i32, h: i32) {
    draw::set_draw_color(Color::Blue);
    let sz = 5;
    // Corner handles
    draw::draw_rectf(x - sz / 2, y - sz / 2, sz, sz);
    draw::draw_rectf(x + w - sz / 2, y - sz / 2, sz, sz);
    draw::draw_rectf(x - sz / 2, y + h - sz / 2, sz, sz);
    draw::draw_rectf(x + w - sz / 2, y + h - sz / 2, sz, sz);
    // Midpoint handles
    draw::draw_rectf(x + w / 2 - sz / 2, y - sz / 2, sz, sz);
    draw::draw_rectf(x + w / 2 - sz / 2, y + h - sz / 2, sz, sz);
    draw::draw_rectf(x - sz / 2, y + h / 2 - sz / 2, sz, sz);
    draw::draw_rectf(x + w - sz / 2, y + h / 2 - sz / 2, sz, sz);
}

// ---------------------------------------------------------------------------
// Design surface mouse event handling
// ---------------------------------------------------------------------------

fn handle_design_surface_event(ds_name: &str, wid: &mut Window, ev: Event) -> bool {
    match ev {
        Event::Push => {
            let mx = app::event_x() - wid.x();
            let my = app::event_y() - wid.y();
            let clicks = app::event_clicks();

            // Check if clicking on a resize handle of the selected component first
            let handle_hit = DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(ds_name) {
                    let idx = state.selected;
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        let c = &state.components[idx as usize];
                        let hsz = 5;
                        // Bottom-right handle
                        if (mx - (c.x + c.w)).abs() <= hsz && (my - (c.y + c.h)).abs() <= hsz {
                            return Some(3); // resize BR
                        }
                        // Right-middle handle
                        if (mx - (c.x + c.w)).abs() <= hsz && (my - (c.y + c.h / 2)).abs() <= hsz {
                            return Some(1); // resize right
                        }
                        // Bottom-middle handle
                        if (mx - (c.x + c.w / 2)).abs() <= hsz && (my - (c.y + c.h)).abs() <= hsz {
                            return Some(2); // resize bottom
                        }
                    }
                }
                None
            });

            if let Some(mode) = handle_hit {
                // Start resize drag
                DESIGN_SURFACES.with(|ds| {
                    let mut surfaces = ds.borrow_mut();
                    if let Some(state) = surfaces.get_mut(ds_name) {
                        state.drag_mode = mode;
                    }
                });
                return true;
            }

            // Check if clicking on an existing component
            let hit = DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(ds_name) {
                    for i in (0..state.components.len()).rev() {
                        let c = &state.components[i];
                        if mx >= c.x && mx <= c.x + c.w && my >= c.y && my <= c.y + c.h {
                            return Some((i as i32, mx - c.x, my - c.y));
                        }
                    }
                }
                None
            });

            if let Some((idx, off_x, off_y)) = hit {
                DESIGN_SURFACES.with(|ds| {
                    let mut surfaces = ds.borrow_mut();
                    if let Some(state) = surfaces.get_mut(ds_name) {
                        state.selected = idx;
                        state.drag_mode = 0; // move
                        state.drag_offset_x = off_x;
                        state.drag_offset_y = off_y;
                    }
                });
                wid.redraw();

                if clicks {
                    rp_fire_event_1(ds_name, "ondblclick", v_int(idx as i64));
                } else {
                    rp_fire_event_1(ds_name, "onselect", v_int(idx as i64));
                }
            } else {
                DESIGN_SURFACES.with(|ds| {
                    let mut surfaces = ds.borrow_mut();
                    if let Some(state) = surfaces.get_mut(ds_name) {
                        state.selected = -1;
                        state.drag_mode = 0;
                    }
                });
                wid.redraw();
                rp_fire_event_2(ds_name, "onbgclick", v_int(mx as i64), v_int(my as i64));
            }
            true
        }
        Event::Drag => {
            let mx = app::event_x() - wid.x();
            let my = app::event_y() - wid.y();

            let result = DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(ds_name) {
                    let idx = state.selected;
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        let mode = state.drag_mode;
                        let c = &mut state.components[idx as usize];
                        match mode {
                            1 => {
                                // Resize right edge
                                let new_w = ((mx - c.x + 4) / 8) * 8;
                                c.w = new_w.max(16);
                            }
                            2 => {
                                // Resize bottom edge
                                let new_h = ((my - c.y + 4) / 8) * 8;
                                c.h = new_h.max(16);
                            }
                            3 => {
                                // Resize bottom-right corner
                                let new_w = ((mx - c.x + 4) / 8) * 8;
                                let new_h = ((my - c.y + 4) / 8) * 8;
                                c.w = new_w.max(16);
                                c.h = new_h.max(16);
                            }
                            _ => {
                                // Move, using offset from Push
                                let new_x = ((mx - state.drag_offset_x + 4) / 8) * 8;
                                let new_y = ((my - state.drag_offset_y + 4) / 8) * 8;
                                c.x = new_x.max(0);
                                c.y = new_y.max(0);
                            }
                        }
                        return Some((idx, c.x, c.y, c.w, c.h));
                    }
                }
                None
            });
            if let Some((idx, cx, cy, cw, ch)) = result {
                wid.redraw();
                rp_fire_event_5(ds_name, "onmove",
                    v_int(idx as i64), v_int(cx as i64), v_int(cy as i64),
                    v_int(cw as i64), v_int(ch as i64));
            }
            true
        }
        Event::Released => {
            // Reset drag mode
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(ds_name) {
                    state.drag_mode = 0;
                }
            });
            true
        }
        _ => false,
    }
}

fn handle_design_surface_frame_event(ds_name: &str, wid: &mut Frame, ev: Event) -> bool {
    match ev {
        Event::Push => {
            let mx = app::event_x() - wid.x();
            let my = app::event_y() - wid.y();
            let clicks = app::event_clicks();

            let handle_hit = DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(ds_name) {
                    let idx = state.selected;
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        let c = &state.components[idx as usize];
                        let hsz = 5;
                        if (mx - (c.x + c.w)).abs() <= hsz && (my - (c.y + c.h)).abs() <= hsz {
                            return Some(3);
                        }
                        if (mx - (c.x + c.w)).abs() <= hsz && (my - (c.y + c.h / 2)).abs() <= hsz {
                            return Some(1);
                        }
                        if (mx - (c.x + c.w / 2)).abs() <= hsz && (my - (c.y + c.h)).abs() <= hsz {
                            return Some(2);
                        }
                    }
                }
                None
            });

            if let Some(mode) = handle_hit {
                DESIGN_SURFACES.with(|ds| {
                    let mut surfaces = ds.borrow_mut();
                    if let Some(state) = surfaces.get_mut(ds_name) {
                        state.drag_mode = mode;
                    }
                });
                return true;
            }

            let hit = DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(ds_name) {
                    for i in (0..state.components.len()).rev() {
                        let c = &state.components[i];
                        if mx >= c.x && mx <= c.x + c.w && my >= c.y && my <= c.y + c.h {
                            return Some((i as i32, mx - c.x, my - c.y));
                        }
                    }
                }
                None
            });

            if let Some((idx, off_x, off_y)) = hit {
                DESIGN_SURFACES.with(|ds| {
                    let mut surfaces = ds.borrow_mut();
                    if let Some(state) = surfaces.get_mut(ds_name) {
                        state.selected = idx;
                        state.drag_mode = 0;
                        state.drag_offset_x = off_x;
                        state.drag_offset_y = off_y;
                    }
                });
                wid.redraw();
                if clicks {
                    rp_fire_event_1(ds_name, "ondblclick", v_int(idx as i64));
                } else {
                    rp_fire_event_1(ds_name, "onselect", v_int(idx as i64));
                }
            } else {
                DESIGN_SURFACES.with(|ds| {
                    let mut surfaces = ds.borrow_mut();
                    if let Some(state) = surfaces.get_mut(ds_name) {
                        state.selected = -1;
                        state.drag_mode = 0;
                    }
                });
                wid.redraw();
                rp_fire_event_2(ds_name, "onbgclick", v_int(mx as i64), v_int(my as i64));
            }
            true
        }
        Event::Drag => {
            let mx = app::event_x() - wid.x();
            let my = app::event_y() - wid.y();

            let result = DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(ds_name) {
                    let idx = state.selected;
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        let mode = state.drag_mode;
                        let c = &mut state.components[idx as usize];
                        match mode {
                            1 => {
                                let new_w = ((mx - c.x + 4) / 8) * 8;
                                c.w = new_w.max(16);
                            }
                            2 => {
                                let new_h = ((my - c.y + 4) / 8) * 8;
                                c.h = new_h.max(16);
                            }
                            3 => {
                                let new_w = ((mx - c.x + 4) / 8) * 8;
                                let new_h = ((my - c.y + 4) / 8) * 8;
                                c.w = new_w.max(16);
                                c.h = new_h.max(16);
                            }
                            _ => {
                                let new_x = ((mx - state.drag_offset_x + 4) / 8) * 8;
                                let new_y = ((my - state.drag_offset_y + 4) / 8) * 8;
                                c.x = new_x.max(0);
                                c.y = new_y.max(0);
                            }
                        }
                        return Some((idx, c.x, c.y, c.w, c.h));
                    }
                }
                None
            });
            if let Some((idx, cx, cy, cw, ch)) = result {
                wid.redraw();
                rp_fire_event_5(ds_name, "onmove",
                    v_int(idx as i64), v_int(cx as i64), v_int(cy as i64),
                    v_int(cw as i64), v_int(ch as i64));
            }
            true
        }
        Event::Released => {
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(ds_name) {
                    state.drag_mode = 0;
                }
            });
            true
        }
        _ => false,
    }
}

/// Register a timer component name so ShowModal can start it.
pub fn gui_register_timer(name: &str) {
    let name_lower = name.to_lowercase();
    ACTIVE_TIMERS.with(|t| {
        let mut timers = t.borrow_mut();
        if !timers.contains(&name_lower) {
            timers.push(name_lower);
        }
    });
}

/// Start all registered, enabled timers using fltk::app::add_timeout.
fn start_timers() {
    let timer_names: Vec<String> = ACTIVE_TIMERS.with(|t| t.borrow().clone());
    for tname in timer_names {
        schedule_timer(&tname);
    }
}

fn schedule_timer(name: &str) {
    let enabled = rp_comp_get(name, "enabled").to_i64();
    if enabled == 0 {
        return;
    }
    let interval_ms = rp_comp_get(name, "interval").to_i64();
    let secs = if interval_ms > 0 { interval_ms as f64 / 1000.0 } else { 1.0 };
    let name_owned = name.to_string();
    app::add_timeout3(secs, move |handle| {
        let enabled = rp_comp_get(&name_owned, "enabled").to_i64();
        if enabled != 0 {
            rp_fire_event(&name_owned, "ontimer");
            app::repeat_timeout3(secs, handle);
        }
    });
}

/// A wait the bytecode VM serves itself (see [`rp_set_cooperative_waits`]).
enum Wait {
    /// `Form.ShowModal`: until the form is closed.
    Form(String),
    /// The program's main event loop: until no window is left.
    App,
}

thread_local! {
    static COOPERATIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static WAITS: RefCell<Vec<Wait>> = const { RefCell::new(Vec::new()) };
    static WAIT_STARTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// For the bytecode VM (rapidr-vm-host-native): `ShowModal` returns at once
/// and leaves its wait to the VM, which pumps events ([`gui_pump_wait`])
/// and runs the handlers they queue itself — event handlers never run
/// inside a runtime call. Native builds keep blocking loops (their
/// handlers are plain Rust functions).
pub fn gui_set_cooperative_waits(on: bool) {
    COOPERATIVE.with(|c| c.set(on));
}

/// Whether the last operation started a wait (once).
pub fn gui_take_wait_started() -> bool {
    WAIT_STARTED.with(|w| w.replace(false))
}

/// Starts waiting for the program's windows (after the main program).
pub fn gui_begin_app_wait() {
    ensure_app();
    WAITS.with(|w| w.borrow_mut().push(Wait::App));
}

fn form_shown(name_lower: &str) -> bool {
    GUI_WIDGETS.with(|gw| matches!(gw.borrow().get(name_lower), Some(GuiWidget::Window(win)) if win.shown()))
}

/// One step of the innermost wait: `None` while it goes on (after handling
/// pending UI events once), `Some(Null)` when it's over.
pub fn gui_pump_wait() -> Option<Value> {
    let done = WAITS.with(|w| match w.borrow().last() {
        None => true,
        Some(Wait::Form(name)) => !form_shown(name),
        Some(Wait::App) => false,
    });
    // Don't hold a borrow while FLTK runs callbacks.
    if done || !app::wait() {
        let finished = WAITS.with(|w| w.borrow_mut().pop());
        if matches!(finished, Some(Wait::Form(_))) {
            // As the blocking ShowModal does when its form closes.
            crate::object::rp_stop_all_timers();
        }
        return Some(v_null());
    }
    None
}

/// Show a form as modal (blocking event loop; for the bytecode VM, a wait
/// it serves — see [`gui_set_cooperative_waits`]).
pub fn gui_showmodal(name: &str) {
    ensure_app();
    let name_lower = name.to_lowercase();

    // Build all widgets that are children of this form
    build_form_widgets(&name_lower);

    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(GuiWidget::Window(ref mut win)) = widgets.get_mut(&name_lower) {
            // Apply center if requested
            if rp_comp_get(name, "_center").to_i64() != 0 {
                let (sw, sh) = app::screen_size();
                let x = ((sw as i32) - win.w()) / 2;
                let y = ((sh as i32) - win.h()) / 2;
                win.set_pos(x, y);
            }
            win.show();
        }
    });

    // Fire OnShow event after widgets are built and window is shown
    rp_fire_event(name, "onshow");

    // Start all registered timers
    start_timers();

    if COOPERATIVE.with(|c| c.get()) {
        WAITS.with(|w| w.borrow_mut().push(Wait::Form(name_lower)));
        WAIT_STARTED.with(|w| w.set(true));
        return;
    }

    // Run the FLTK event loop — do NOT hold a borrow on GUI_APP during wait()
    // because callbacks may call ensure_app() which needs borrow_mut.
    while app::wait() {
        if !form_shown(&name_lower) {
            break;
        }
    }

    // The form has closed: disable any timers so their already-queued
    // FLTK timeouts don't fire after we tear down the dispatcher.
    crate::object::rp_stop_all_timers();
}

/// Hide a widget (form window or embedded frame): no OnClose.
pub fn gui_hide(name: &str) {
    GUI_WIDGETS.with(|gw| match gw.borrow_mut().get_mut(&name.to_lowercase()) {
        Some(GuiWidget::Window(win)) => win.hide(),
        Some(GuiWidget::Frame(frm)) => frm.hide(),
        _ => {}
    });
}

/// `Form.Close` and the window's close button: OnClose's `Action` (it
/// starts as `caHide`) decides whether the form goes, stays or is
/// minimized. Anything else just hides.
pub fn gui_close(name: &str) {
    use rapidr_value::events::{CloseAction, CA_HIDE};
    let name_lower = name.to_lowercase();
    let is_window = GUI_WIDGETS.with(|gw| matches!(gw.borrow().get(&name_lower), Some(GuiWidget::Window(_))));
    if !is_window {
        return gui_hide(name);
    }
    rp_fire_event_then(name, "onclose", &[v_int(CA_HIDE)], move |a| {
        GUI_WIDGETS.with(|gw| {
            if let Some(GuiWidget::Window(win)) = gw.borrow_mut().get_mut(&name_lower) {
                match CloseAction::of(&a[0]) {
                    CloseAction::Stay => {}
                    CloseAction::Minimize => win.iconize(),
                    CloseAction::Close => win.hide(),
                }
            }
        })
    });
}

/// The window's close button goes through OnClose ([`gui_close`]); FLTK's
/// Escape-closes-the-window doesn't (RapidQ forms stay).
fn close_button(win: &mut Window, name: &str) {
    let name = name.to_string();
    win.set_callback(move |_| {
        if app::event() == Event::Close {
            gui_close(&name);
        }
    });
}

/// Center a window on screen.
pub fn gui_center(name: &str) {
    let name_lower = name.to_lowercase();
    // Store as a flag — will be applied when the window is shown
    rp_comp_set(name, "_center", v_int(1));
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(GuiWidget::Window(ref mut win)) = widgets.get_mut(&name_lower) {
            let (sw, sh) = app::screen_size();
            let x = ((sw as i32) - win.w()) / 2;
            let y = ((sh as i32) - win.h()) / 2;
            win.set_pos(x, y);
        }
    });
    // Form.Left / Form.Top read where it went.
    if let Some(GuiWidget::Window(win)) = GUI_WIDGETS.with(|gw| gw.borrow().get(&name_lower).cloned()) {
        APPLYING.with(|a| a.set(a.get() + 1));
        crate::layout::quietly(|| {
            rp_comp_set(name, "left", v_int(win.x() as i64));
            rp_comp_set(name, "top", v_int(win.y() as i64));
        });
        APPLYING.with(|a| a.set(a.get() - 1));
    }
}

/// Execute a dialog (Open/Save/Color/Font).
pub fn gui_dialog_execute(name: &str, comp_type: &str) -> Value {
    ensure_app();
    match comp_type {
        "ROPENDIALOG" => {
            let filter = rp_comp_get(name, "filter").to_string_val();
            let title = rp_comp_get(name, "title").to_string_val();
            let mut dlg = dialog::NativeFileChooser::new(dialog::NativeFileChooserType::BrowseFile);
            if !title.is_empty() {
                dlg.set_title(&title);
            }
            if !filter.is_empty() {
                dlg.set_filter(&filter);
            }
            dlg.show();
            let filename = dlg.filename().to_string_lossy().to_string();
            if !filename.is_empty() {
                rp_comp_set(name, "filename", v_str(&filename));
                v_int(1)
            } else {
                v_int(0)
            }
        }
        "RSAVEDIALOG" => {
            let filter = rp_comp_get(name, "filter").to_string_val();
            let title = rp_comp_get(name, "title").to_string_val();
            let mut dlg = dialog::NativeFileChooser::new(dialog::NativeFileChooserType::BrowseSaveFile);
            if !title.is_empty() {
                dlg.set_title(&title);
            }
            if !filter.is_empty() {
                dlg.set_filter(&filter);
            }
            dlg.show();
            let filename = dlg.filename().to_string_lossy().to_string();
            if !filename.is_empty() {
                rp_comp_set(name, "filename", v_str(&filename));
                v_int(1)
            } else {
                v_int(0)
            }
        }
        "RCOLORDIALOG" => {
            // Show FLTK color chooser dialog
            if let Some((r, g, b)) = dialog::color_chooser("Choose Color", dialog::ColorMode::Rgb) {
                let hex = format!("#{:02X}{:02X}{:02X}", r, g, b);
                rp_comp_set(name, "color", v_str(&hex));
                v_int(1)
            } else {
                v_int(0)
            }
        }
        "RFONTDIALOG" => {
            // Full font picker with list, size, bold/italic, and live preview
            use std::rc::Rc;

            let current_name = rp_comp_get(name, "fontname").to_string_val();
            let current_name = if current_name.is_empty() { "Helvetica".to_string() } else { current_name };
            let current_size: i32 = rp_comp_get(name, "fontsize").to_string_val().parse().unwrap_or(12);

            let font_names = app::get_font_names();

            let mut win = Window::new(100, 100, 560, 430, None);
            win.set_label("Font Picker");

            // Font list
            let mut fl_lbl = Frame::new(10, 5, 250, 20, None);
            fl_lbl.set_label("Font:");
            fl_lbl.set_align(Align::Left | Align::Inside);
            let mut font_browser = HoldBrowser::new(10, 25, 250, 290, None);
            let mut cur_font_idx = 0i32;
            for (i, fn_name) in font_names.iter().enumerate() {
                font_browser.add(fn_name);
                if fn_name.eq_ignore_ascii_case(&current_name) {
                    cur_font_idx = i as i32 + 1;
                }
            }
            if cur_font_idx > 0 {
                font_browser.select(cur_font_idx);
            }

            // Size list
            let mut sz_lbl = Frame::new(270, 5, 80, 20, None);
            sz_lbl.set_label("Size:");
            sz_lbl.set_align(Align::Left | Align::Inside);
            let mut size_browser = HoldBrowser::new(270, 25, 70, 290, None);
            let sizes = [8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 32, 36, 48, 72];
            let mut cur_size_idx = 0i32;
            for (i, &sz) in sizes.iter().enumerate() {
                size_browser.add(&sz.to_string());
                if sz == current_size { cur_size_idx = i as i32 + 1; }
            }
            if cur_size_idx > 0 { size_browser.select(cur_size_idx); }

            // Bold / Italic checkboxes
            let mut bold_cb = CheckButton::new(350, 30, 90, 25, None);
            bold_cb.set_label("Bold");
            let mut italic_cb = CheckButton::new(450, 30, 90, 25, None);
            italic_cb.set_label("Italic");

            // Preview area
            let mut pv_lbl = Frame::new(350, 65, 200, 20, None);
            pv_lbl.set_label("Preview:");
            pv_lbl.set_align(Align::Left | Align::Inside);
            let mut preview = Frame::new(350, 85, 200, 230, None);
            preview.set_frame(FrameType::DownBox);
            preview.set_color(Color::White);
            preview.set_label("AaBbCc 123");
            preview.set_label_size(current_size);
            if cur_font_idx > 0 {
                preview.set_label_font(Font::by_index(cur_font_idx as usize - 1));
            }

            // OK / Cancel
            let mut ok_btn = Button::new(350, 390, 90, 30, None);
            ok_btn.set_label("OK");
            let mut cancel_btn = Button::new(450, 390, 90, 30, None);
            cancel_btn.set_label("Cancel");

            win.end();
            win.make_modal(true);
            win.show();

            let confirmed = Rc::new(std::cell::RefCell::new(false));

            // Helper: update preview from current selections
            macro_rules! update_preview_fn {
                ($preview:expr, $font_browser:expr, $size_browser:expr, $bold_cb:expr, $italic_cb:expr, $font_names:expr) => {{
                    let fi = $font_browser.value();
                    if fi > 0 && (fi as usize - 1) < $font_names.len() {
                        let mut idx = fi as usize - 1;
                        // In FLTK, bold = idx|1, italic = idx|2
                        if $bold_cb.value() { idx |= 1; }
                        if $italic_cb.value() { idx |= 2; }
                        if idx < $font_names.len() {
                            $preview.set_label_font(Font::by_index(idx));
                        } else {
                            $preview.set_label_font(Font::by_index(fi as usize - 1));
                        }
                    }
                    let si = $size_browser.value();
                    if si > 0 {
                        if let Some(sz_str) = $size_browser.text(si) {
                            if let Ok(sz) = sz_str.parse::<i32>() {
                                $preview.set_label_size(sz);
                            }
                        }
                    }
                    $preview.set_label("AaBbCc 123");
                    $preview.redraw();
                }};
            }

            // Font browser callback
            {
                let mut pv = preview.clone();
                let fb = font_browser.clone();
                let sb = size_browser.clone();
                let bc = bold_cb.clone();
                let ic = italic_cb.clone();
                let fns = font_names.clone();
                font_browser.set_callback(move |_| {
                    update_preview_fn!(pv, fb, sb, bc, ic, fns);
                });
            }
            // Size browser callback
            {
                let mut pv = preview.clone();
                let fb = font_browser.clone();
                let sb = size_browser.clone();
                let bc = bold_cb.clone();
                let ic = italic_cb.clone();
                let fns = font_names.clone();
                size_browser.set_callback(move |_| {
                    update_preview_fn!(pv, fb, sb, bc, ic, fns);
                });
            }
            // Bold checkbox callback
            {
                let mut pv = preview.clone();
                let fb = font_browser.clone();
                let sb = size_browser.clone();
                let bc = bold_cb.clone();
                let ic = italic_cb.clone();
                let fns = font_names.clone();
                bold_cb.set_callback(move |_| {
                    update_preview_fn!(pv, fb, sb, bc, ic, fns);
                });
            }
            // Italic checkbox callback
            {
                let mut pv = preview.clone();
                let fb = font_browser.clone();
                let sb = size_browser.clone();
                let bc = bold_cb.clone();
                let ic = italic_cb.clone();
                let fns = font_names.clone();
                italic_cb.set_callback(move |_| {
                    update_preview_fn!(pv, fb, sb, bc, ic, fns);
                });
            }

            // OK button
            {
                let c = confirmed.clone();
                let mut w = win.clone();
                ok_btn.set_callback(move |_| {
                    *c.borrow_mut() = true;
                    w.hide();
                });
            }
            // Cancel button
            {
                let mut w = win.clone();
                cancel_btn.set_callback(move |_| {
                    w.hide();
                });
            }

            while win.shown() {
                app::wait();
            }

            if *confirmed.borrow() {
                let fi = font_browser.value();
                let font_name = if fi > 0 && (fi as usize - 1) < font_names.len() {
                    font_names[fi as usize - 1].clone()
                } else {
                    "Helvetica".to_string()
                };
                let si = size_browser.value();
                let font_size = if si > 0 {
                    size_browser.text(si).unwrap_or_default().parse::<i32>().unwrap_or(12)
                } else { 12 };
                rp_comp_set(name, "fontname", v_str(&font_name));
                rp_comp_set(name, "fontsize", v_int(font_size as i64));
                rp_comp_set(name, "fontbold", v_int(if bold_cb.value() { 1 } else { 0 }));
                rp_comp_set(name, "fontitalic", v_int(if italic_cb.value() { 1 } else { 0 }));
                v_int(1)
            } else {
                v_int(0)
            }
        }
        _ => v_int(0),
    }
}

/// Start the GUI event loop (standalone, not attached to a form).
pub fn run_gui_event_loop() {
    ensure_app();
    // Don't hold a borrow on GUI_APP during the event loop
    app::run().ok();
}

// ---------------------------------------------------------------------------
// Internal: Build all widgets parented to a form
// ---------------------------------------------------------------------------

/// Materialize FLTK widgets for a form and all its children.
fn build_form_widgets(form_name: &str) {
    // Idempotent: if the form widget already exists, skip the whole build —
    // children were created on the first call (e.g. previous Show/ShowModal).
    let already_built = GUI_WIDGETS.with(|gw| gw.borrow().contains_key(&form_name.to_lowercase()));
    if already_built {
        return;
    }

    // First, create the form window
    gui_create_widget(form_name, "RFORM");

    // Recursively build all children
    build_children_recursive(form_name);

    // Fire OnLoad once, after the entire form tree is materialized.
    rp_fire_event(form_name, "onload");
    // Then the first OnPaint of the form and its canvases (RapidQ programs
    // draw there); the surfaces keep what's drawn, so later ones are only
    // asked for (Repaint) or follow a resize.
    fire_first_paint(form_name);
}

fn fire_first_paint(parent: &str) {
    rp_fire_event(parent, "onpaint");
    for (child, type_name) in crate::object::get_children_of(parent) {
        if type_name.eq_ignore_ascii_case("RCANVAS") {
            rp_fire_event(&child, "onpaint");
        } else {
            fire_first_paint(&child);
        }
    }
}

/// Recursively build child widgets of a parent container.
fn build_children_recursive(parent_name: &str) {
    let children = crate::object::get_children_of(parent_name);
    if children.is_empty() { return; }

    // Get parent widget offset for relative positioning
    let (parent_x, parent_y) = get_widget_offset(parent_name);

    // Check if the parent form has a main menu — if so, offset children below it.
    // On macOS, SysMenuBar goes to the system menu bar so no in-window offset needed.
    let menu_offset = menu_offset(parent_name);

    // Begin adding children to the parent widget
    begin_widget(parent_name);

    for (child_name, child_type) in &children {
        // Temporarily offset child position by parent's position for widget creation.
        // Save original values and restore after creation to prevent double-offset
        // if this function is ever called again.
        let orig_left = rp_comp_get(child_name, "left").to_i64() as i32;
        let orig_top = rp_comp_get(child_name, "top").to_i64() as i32;

        // The main menu doesn't get the menu offset
        let extra_y = if child_type != "RMAINMENU" { menu_offset } else { 0 };

        // (Temporary absolute positions: nothing is laid out or moved.)
        crate::layout::quietly(|| {
            if parent_x != 0 || parent_y != 0 || extra_y != 0 {
                rp_comp_set(child_name, "left", v_int((orig_left + parent_x) as i64));
                rp_comp_set(child_name, "top", v_int((orig_top + parent_y + extra_y) as i64));
            }

            gui_create_widget(child_name, child_type);

            // Restore original relative positions
            if parent_x != 0 || parent_y != 0 || extra_y != 0 {
                rp_comp_set(child_name, "left", v_int(orig_left as i64));
                rp_comp_set(child_name, "top", v_int(orig_top as i64));
            }
        });

        // For TabControls, also build children inside each tab group
        if child_type == "RTABCONTROL" {
            let tab_grps = TAB_GROUPS.with(|tg| {
                tg.borrow().get(&child_name.to_lowercase()).cloned().unwrap_or_default()
            });
            for grp_name in &tab_grps {
                build_children_recursive(grp_name);
            }
        }

        // Recursively build any other children
        build_children_recursive(child_name);
    }

    // End parent widget
    end_widget(parent_name);
}

fn begin_widget(name: &str) {
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(widget) = widgets.get_mut(name) {
            match widget {
                GuiWidget::Window(ref mut w) => { w.begin(); }
                GuiWidget::Group(ref mut g) => { g.begin(); }
                GuiWidget::Tabs(ref mut t) => { t.begin(); }
                GuiWidget::Scroll(ref mut s) => { s.begin(); }
                _ => {}
            }
        }
    });
}

fn end_widget(name: &str) {
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(widget) = widgets.get_mut(name) {
            match widget {
                GuiWidget::Window(ref mut w) => { w.end(); }
                GuiWidget::Group(ref mut g) => { g.end(); }
                GuiWidget::Tabs(ref mut t) => { t.end(); }
                GuiWidget::Scroll(ref mut s) => { s.end(); }
                _ => {}
            }
        }
    });
}

/// Get the position offset of a parent widget for relative child positioning.
/// Returns (0, 0) for top-level Windows (children use absolute coords).
fn get_widget_offset(name: &str) -> (i32, i32) {
    GUI_WIDGETS.with(|gw| {
        let widgets = gw.borrow();
        if let Some(widget) = widgets.get(name) {
            match widget {
                GuiWidget::Window(_) => (0, 0), // Window children use absolute coords
                GuiWidget::Group(ref g) => (g.x(), g.y()),
                GuiWidget::Tabs(ref t) => (t.x(), t.y()),
                GuiWidget::Scroll(ref s) => (s.x(), s.y()),
                _ => (0, 0),
            }
        } else {
            (0, 0)
        }
    })
}

// ---------------------------------------------------------------------------
// Live geometry (layout.rs keeps Left/Top/Width/Height; this moves widgets)
// ---------------------------------------------------------------------------

thread_local! {
    /// Set while this runtime resizes widgets itself (a form's resize
    /// callback then isn't a user resize).
    static APPLYING: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// The height of a form's in-window main menu (children sit below it); 0
/// without one, and on macOS, where the menu is the system menu bar.
pub fn menu_offset(form: &str) -> i32 {
    let has_menu = crate::object::get_children_of(form).iter().any(|(_, t)| t == "RMAINMENU");
    if has_menu && !cfg!(target_os = "macos") { 30 } else { 0 }
}

fn resize_widget(widget: &mut GuiWidget, x: i32, y: i32, w: i32, h: i32) {
    match widget {
        GuiWidget::Window(v) => v.resize(x, y, w, h),
        GuiWidget::Button(v) => v.resize(x, y, w, h),
        GuiWidget::Frame(v) | GuiWidget::ImageFrame(v) => v.resize(x, y, w, h),
        GuiWidget::Input(v) => v.resize(x, y, w, h),
        GuiWidget::CheckButton(v) => v.resize(x, y, w, h),
        GuiWidget::RadioButton(v) => v.resize(x, y, w, h),
        GuiWidget::Choice(v) => v.resize(x, y, w, h),
        GuiWidget::InputChoice(v) => v.resize(x, y, w, h),
        GuiWidget::HoldBrowser(v) => v.resize(x, y, w, h),
        GuiWidget::TextEditor(v) => v.resize(x, y, w, h),
        GuiWidget::Group(v) => v.resize(x, y, w, h),
        GuiWidget::Tabs(v) => v.resize(x, y, w, h),
        GuiWidget::MenuBar(v) => v.resize(x, y, w, h),
        GuiWidget::SysMenuBar(v) => v.resize(x, y, w, h),
        GuiWidget::Progress(v) => v.resize(x, y, w, h),
        GuiWidget::Scroll(v) => v.resize(x, y, w, h),
        GuiWidget::Tree(v) => v.resize(x, y, w, h),
        GuiWidget::Slider(v) => v.resize(x, y, w, h),
        GuiWidget::Grid(v, _) => v.resize(x, y, w, h),
    }
}

fn redraw_window_of(widget: &GuiWidget) {
    macro_rules! redraw_win {
        ($v:expr) => {
            if let Some(mut w) = $v.window() {
                w.redraw();
            }
        };
    }
    match widget {
        GuiWidget::Window(v) => v.clone().redraw(),
        GuiWidget::Button(v) => redraw_win!(v),
        GuiWidget::Frame(v) | GuiWidget::ImageFrame(v) => redraw_win!(v),
        GuiWidget::Group(v) => redraw_win!(v),
        GuiWidget::Scroll(v) => redraw_win!(v),
        GuiWidget::Grid(v, _) => redraw_win!(v),
        GuiWidget::HoldBrowser(v) => redraw_win!(v),
        GuiWidget::TextEditor(v) => redraw_win!(v),
        GuiWidget::Input(v) => redraw_win!(v),
        GuiWidget::Tabs(v) => redraw_win!(v),
        GuiWidget::CheckButton(v) => redraw_win!(v),
        GuiWidget::RadioButton(v) => redraw_win!(v),
        GuiWidget::Choice(v) => redraw_win!(v),
        GuiWidget::InputChoice(v) => redraw_win!(v),
        GuiWidget::Progress(v) => redraw_win!(v),
        GuiWidget::Tree(v) => redraw_win!(v),
        GuiWidget::Slider(v) => redraw_win!(v),
        GuiWidget::MenuBar(v) => redraw_win!(v),
        GuiWidget::SysMenuBar(v) => redraw_win!(v),
    }
}

/// Moves / resizes `name`'s widget (if it's built) to its Left / Top /
/// Width / Height, then its children's, which FLTK would otherwise have
/// scaled with it. A form keeps its screen position (see
/// [`gui_move_form`]).
pub fn gui_apply_geometry(name: &str) {
    let name = name.to_lowercase();
    let Some(mut widget) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|w| w.get(&name).cloned())) else { return };
    let comp_type = rp_comp_type(&name);
    if comp_type == "RMAINMENU" {
        return;
    }
    let n = |p: &str| rp_comp_get(&name, p).to_i64().clamp(-100_000, 100_000) as i32;
    let rect = if let GuiWidget::Window(win) = &widget {
        let (w, h) = form_window_size(&name);
        (win.x(), win.y(), w, h)
    } else {
        let parent = rp_comp_get(&name, "parent").to_string_val().to_lowercase();
        let (px, py) = get_widget_offset(&parent);
        (px + n("left"), py + menu_offset(&parent) + n("top"), n("width").max(0), n("height").max(0))
    };
    let current = match &widget {
        GuiWidget::Window(v) => (v.x(), v.y(), v.w(), v.h()),
        _ => (-1, -1, -1, -1),
    };
    if current != rect {
        APPLYING.with(|a| a.set(a.get() + 1));
        resize_widget(&mut widget, rect.0, rect.1, rect.2, rect.3);
        APPLYING.with(|a| a.set(a.get() - 1));
        redraw_window_of(&widget);
    }
    // A form's main menu spans its width.
    if let GuiWidget::Window(_) = widget {
        for (child, t) in crate::object::get_children_of(&name) {
            if t == "RMAINMENU" {
                if let Some(GuiWidget::SysMenuBar(mut mb)) = GUI_WIDGETS.with(|gw| gw.borrow().get(&child).cloned()) {
                    mb.resize(0, 0, rect.2, mb.h());
                }
            }
        }
    }
    for (child, _) in crate::object::get_children_of(&name) {
        gui_apply_geometry(&child);
    }
    // A tab control's pages sit below its tabs.
    if comp_type == "RTABCONTROL" {
        let pages = TAB_GROUPS.with(|tg| tg.borrow().get(&name).cloned().unwrap_or_default());
        for page in pages {
            if let Some(mut g) = GUI_WIDGETS.with(|gw| gw.borrow().get(&page).cloned()) {
                resize_widget(&mut g, rect.0, rect.1 + 25, rect.2, (rect.3 - 25).max(0));
            }
            for (child, _) in crate::object::get_children_of(&page) {
                gui_apply_geometry(&child);
            }
        }
    }
}

/// The FLTK window of a form: its inside plus the in-window main menu
/// (Width / Height less the frame the window manager draws).
fn form_window_size(name: &str) -> (i32, i32) {
    let (fw, fh) = rapidr_value::layout::form_frame(rp_comp_get(name, "borderstyle").to_i64());
    let w = (rp_comp_get(name, "width").to_i64() - fw).clamp(1, 100_000);
    let h = (rp_comp_get(name, "height").to_i64() - fh).clamp(1, 100_000);
    (w as i32, h as i32)
}

/// `Form.BorderStyle`: bsNone (0) takes away the window's frame.
pub fn gui_set_form_border(name: &str) {
    let name = name.to_lowercase();
    if let Some(GuiWidget::Window(mut win)) = GUI_WIDGETS.with(|gw| gw.borrow().get(&name).cloned()) {
        APPLYING.with(|a| a.set(a.get() + 1));
        win.set_border(rp_comp_get(&name, "borderstyle").to_i64() != 0);
        APPLYING.with(|a| a.set(a.get() - 1));
    }
    gui_apply_geometry(&name);
}

/// `Form.Left` / `Form.Top` set by the program: the window moves there.
pub fn gui_move_form(name: &str) {
    let name = name.to_lowercase();
    if APPLYING.with(|a| a.get()) > 0 {
        return;
    }
    if let Some(GuiWidget::Window(mut win)) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|w| w.get(&name).cloned())) {
        let (x, y) = (rp_comp_get(&name, "left").to_i64() as i32, rp_comp_get(&name, "top").to_i64() as i32);
        if (win.x(), win.y()) != (x, y) {
            APPLYING.with(|a| a.set(a.get() + 1));
            win.set_pos(x, y);
            APPLYING.with(|a| a.set(a.get() - 1));
        }
    }
}

/// The user moved or resized a form: its Left / Top / Width / Height
/// follow, its aligned children are laid out again (the others keep their
/// places, as in RapidQ — FLTK's proportional scaling is undone) and
/// OnResize fires.
fn form_resized(form: &str, x: i32, y: i32, w: i32, h: i32) {
    if APPLYING.with(|a| a.get()) > 0 {
        return;
    }
    // FLTK calls this from inside `show()` / `resize()` too, which the
    // runtime may call while it holds the widget table: handle it on the
    // next turn of the event loop then.
    if GUI_WIDGETS.with(|gw| gw.try_borrow_mut().is_err()) {
        let form = form.to_string();
        app::add_timeout3(0.0, move |_| form_resized(&form, x, y, w, h));
        return;
    }
    APPLYING.with(|a| a.set(a.get() + 1));
    crate::layout::quietly(|| {
        rp_comp_set(form, "left", v_int(x as i64));
        rp_comp_set(form, "top", v_int(y as i64));
    });
    APPLYING.with(|a| a.set(a.get() - 1));
    // The window is the inside: Width / Height add the frame.
    let (fw, fh) = rapidr_value::layout::form_frame(rp_comp_get(form, "borderstyle").to_i64());
    let (w, h) = (w as i64 + fw, h as i64 + fh);
    let same = rp_comp_get(form, "width").to_i64() == w && rp_comp_get(form, "height").to_i64() == h;
    if same {
        return;
    }
    crate::layout::quietly(|| {
        rp_comp_set(form, "width", v_int(w));
        rp_comp_set(form, "height", v_int(h));
    });
    crate::layout::realign(form, None);
    gui_apply_geometry(form);
    rp_fire_event(form, "onresize");
    rp_fire_event(form, "onpaint");
}

// ---------------------------------------------------------------------------
// Non-blocking Show (for secondary windows like DesignSurface)
// ---------------------------------------------------------------------------

/// Show a form window without blocking. The event loop is driven by ShowModal
/// on the main form.
pub fn gui_show(name: &str) {
    ensure_app();
    let name_lower = name.to_lowercase();
    let comp_type = crate::object::rp_comp_type(name);

    // Check if widget already exists — if so, just show it
    let already_exists = GUI_WIDGETS.with(|gw| gw.borrow().contains_key(&name_lower));
    if already_exists {
        GUI_WIDGETS.with(|gw| {
            let mut widgets = gw.borrow_mut();
            match widgets.get_mut(&name_lower) {
                Some(GuiWidget::Window(ref mut win)) => { win.show(); }
                Some(GuiWidget::Frame(ref mut frm)) => { frm.show(); }
                _ => {}
            }
        });
        return;
    }

    // Build widgets for the first time
    if comp_type == "RDESIGNSURFACE" {
        gui_create_widget(name, &comp_type);
    } else {
        build_form_widgets(&name_lower);
    }

    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        match widgets.get_mut(&name_lower) {
            Some(GuiWidget::Window(ref mut win)) => { win.show(); }
            Some(GuiWidget::Frame(ref mut frm)) => { frm.show(); }
            _ => {}
        }
    });

    // Fire OnShow event after widgets are built and shown
    rp_fire_event(name, "onshow");
}

// ---------------------------------------------------------------------------
// Design surface methods
// ---------------------------------------------------------------------------

/// Handle method calls on a PDESIGNSURFACE component.
pub fn design_surface_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "addcomponent" => {
            // AddComponent(type, name, x, y, w, h)
            let type_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let comp_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let x = args.get(2).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let y = args.get(3).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let w = args.get(4).map(|v| v.to_i64()).unwrap_or(80) as i32;
            let h = args.get(5).map(|v| v.to_i64()).unwrap_or(25) as i32;
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(&name_lower) {
                    let mut props = HashMap::new();
                    props.insert("caption".to_string(), comp_name.clone());
                    state.components.push(DesignComp {
                        name: comp_name,
                        type_name,
                        x, y, w, h,
                        props,
                    });
                    state.selected = (state.components.len() - 1) as i32;
                }
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "getname" => {
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        return v_str(&state.components[idx as usize].name);
                    }
                }
                v_str("")
            })
        }
        "gettype" => {
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        return v_str(&state.components[idx as usize].type_name);
                    }
                }
                v_str("")
            })
        }
        "getcompx" => {
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        return v_int(state.components[idx as usize].x as i64);
                    }
                }
                v_int(0)
            })
        }
        "getcompy" => {
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        return v_int(state.components[idx as usize].y as i64);
                    }
                }
                v_int(0)
            })
        }
        "getcompw" => {
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        return v_int(state.components[idx as usize].w as i64);
                    }
                }
                v_int(0)
            })
        }
        "getcomph" => {
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        return v_int(state.components[idx as usize].h as i64);
                    }
                }
                v_int(0)
            })
        }
        "setprop" => {
            // SetProp(index, propname, value)
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            let prop = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let val = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        state.components[idx as usize].props.insert(prop.to_lowercase(), val);
                    }
                }
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "getprop" => {
            // GetProp(index, propname)
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            let prop = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        if let Some(val) = state.components[idx as usize].props.get(&prop.to_lowercase()) {
                            return v_str(val);
                        }
                    }
                }
                v_str("")
            })
        }
        "setcompbounds" => {
            // SetCompBounds(index, x, y, w, h)
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            let x = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let y = args.get(2).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let w = args.get(3).map(|v| v.to_i64()).unwrap_or(80) as i32;
            let h = args.get(4).map(|v| v.to_i64()).unwrap_or(25) as i32;
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        let c = &mut state.components[idx as usize];
                        c.x = x; c.y = y; c.w = w; c.h = h;
                    }
                }
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "setname" => {
            // SetName(index, newname)
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            let new_name = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        state.components[idx as usize].name = new_name;
                    }
                }
            });
            v_null()
        }
        "selectcomp" => {
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(&name_lower) {
                    state.selected = idx as i32;
                }
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "removecomponent" => {
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(-1);
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(&name_lower) {
                    if idx >= 0 && (idx as usize) < state.components.len() {
                        state.components.remove(idx as usize);
                        state.selected = -1;
                    }
                }
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "clearall" => {
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(&name_lower) {
                    state.components.clear();
                    state.selected = -1;
                }
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "show" => {
            gui_show(name);
            v_null()
        }
        "hide" => {
            gui_hide(name);
            v_null()
        }
        "count" => {
            DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(&name_lower) {
                    v_int(state.components.len() as i64)
                } else {
                    v_int(0)
                }
            })
        }
        _ => {
            eprintln!("[WARN] DesignSurface.{}() not implemented", method);
            v_null()
        }
    }
}

/// Get a design surface property
pub fn design_surface_get(name: &str, prop: &str) -> Option<Value> {
    let name_lower = name.to_lowercase();
    let prop_lower = prop.to_lowercase();
    match prop_lower.as_str() {
        "compcount" | "count" => {
            Some(DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(&name_lower) {
                    v_int(state.components.len() as i64)
                } else {
                    v_int(0)
                }
            }))
        }
        "formcaption" => {
            Some(DESIGN_SURFACES.with(|ds| {
                let surfaces = ds.borrow();
                if let Some(state) = surfaces.get(&name_lower) {
                    v_str(&state.form_caption)
                } else {
                    v_str("")
                }
            }))
        }
        _ => None,
    }
}

/// Set a design surface property
pub fn design_surface_set(name: &str, prop: &str, val: &Value) -> bool {
    let name_lower = name.to_lowercase();
    let prop_lower = prop.to_lowercase();
    match prop_lower.as_str() {
        "formcaption" => {
            let cap = val.to_string_val();
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(&name_lower) {
                    state.form_caption = cap;
                }
            });
            true
        }
        "width" | "height" => {
            let v = val.to_i64() as i32;
            DESIGN_SURFACES.with(|ds| {
                let mut surfaces = ds.borrow_mut();
                if let Some(state) = surfaces.get_mut(&name_lower) {
                    if prop_lower == "width" { state.form_w = v; }
                    if prop_lower == "height" { state.form_h = v; }
                }
            });
            true
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// String grid methods
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Code editor methods
// ---------------------------------------------------------------------------

/// Handle method calls on a PCODEEDITOR component.
pub fn code_editor_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "getsublist" => {
            // Return a newline-separated list of SUB/FUNCTION names from the code
            let text = GUI_TEXT_BUFFERS.with(|tb| {
                let bufs = tb.borrow();
                bufs.get(&name_lower).map(|b| b.text()).unwrap_or_default()
            });
            let mut subs = Vec::new();
            for line in text.lines() {
                let trimmed = line.trim().to_uppercase();
                if trimmed.starts_with("SUB ") || trimmed.starts_with("FUNCTION ") {
                    // Extract the name
                    let parts: Vec<&str> = line.trim().split('(').collect();
                    let decl = parts[0];
                    let sub_name = decl.split_whitespace().nth(1).unwrap_or("");
                    if !sub_name.is_empty() {
                        subs.push(sub_name.to_string());
                    }
                }
            }
            v_str(&subs.join("\n"))
        }
        "gotosub" => {
            let sub_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let text = GUI_TEXT_BUFFERS.with(|tb| {
                let bufs = tb.borrow();
                bufs.get(&name_lower).map(|b| b.text()).unwrap_or_default()
            });
            let target = sub_name.to_uppercase();
            for (i, line) in text.lines().enumerate() {
                let upper = line.trim().to_uppercase();
                if (upper.starts_with("SUB ") || upper.starts_with("FUNCTION "))
                    && upper.contains(&target) {
                    // Scroll to this line
                    GUI_WIDGETS.with(|gw| {
                        let mut widgets = gw.borrow_mut();
                        if let Some(GuiWidget::TextEditor(ref mut ed)) = widgets.get_mut(&name_lower) {
                            // Position to line
                            GUI_TEXT_BUFFERS.with(|tb| {
                                let bufs = tb.borrow();
                                if let Some(_buf) = bufs.get(&name_lower) {
                                    // Calculate byte offset for line i
                                    let mut offset = 0;
                                    for (j, ln) in text.lines().enumerate() {
                                        if j == i { break; }
                                        offset += ln.len() + 1; // +1 for newline
                                    }
                                    ed.set_insert_position(offset as i32);
                                    ed.show_insert_position();
                                }
                            });
                        }
                    });
                    break;
                }
            }
            v_null()
        }
        "gotoline" => {
            let line_num = args.first().map(|v| v.to_i64()).unwrap_or(0);
            let text = GUI_TEXT_BUFFERS.with(|tb| {
                let bufs = tb.borrow();
                bufs.get(&name_lower).map(|b| b.text()).unwrap_or_default()
            });
            let mut offset = 0;
            for (i, ln) in text.lines().enumerate() {
                if i as i64 >= line_num { break; }
                offset += ln.len() + 1;
            }
            GUI_WIDGETS.with(|gw| {
                let mut widgets = gw.borrow_mut();
                if let Some(GuiWidget::TextEditor(ref mut ed)) = widgets.get_mut(&name_lower) {
                    ed.set_insert_position(offset as i32);
                    ed.show_insert_position();
                }
            });
            v_null()
        }
        _ => {
            eprintln!("[WARN] CodeEditor.{}() not implemented", method);
            v_null()
        }
    }
}

// ---------------------------------------------------------------------------
// Tab control methods
// ---------------------------------------------------------------------------

/// Handle method calls on a PTABCONTROL component.
pub fn tab_control_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "addtabs" => {
            // Store tab names — actual FLTK Groups are created during gui_create_widget
            let mut group_names = Vec::new();
            let mut labels = Vec::new();
            for (i, arg) in args.iter().enumerate() {
                let tab_label = arg.to_string_val();
                let grp_name = format!("{}__tab_{}", name_lower, i);
                group_names.push(grp_name);
                labels.push(tab_label);
            }
            TAB_GROUPS.with(|tg| {
                tg.borrow_mut().insert(name_lower.clone(), group_names);
            });
            // Store labels for gui_create_widget to use
            rp_comp_set(name, "_tab_labels", v_str(&labels.join("\n")));
            v_null()
        }
        "tab" => {
            // Tab(index) — returns a reference name for the group
            let idx = args.first().map(|v| v.to_i64()).unwrap_or(0) as usize;
            TAB_GROUPS.with(|tg| {
                let groups = tg.borrow();
                if let Some(tabs) = groups.get(&name_lower) {
                    if idx < tabs.len() {
                        return v_str(&tabs[idx]);
                    }
                }
                v_str("")
            })
        }
        _ => {
            eprintln!("[WARN] TabControl.{}() not implemented", method);
            v_null()
        }
    }
}

// ---------------------------------------------------------------------------
// TreeView methods
// ---------------------------------------------------------------------------

pub fn tree_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "addroot" => {
            let label = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            GUI_WIDGETS.with(|gw| {
                let mut widgets = gw.borrow_mut();
                if let Some(GuiWidget::Tree(ref mut tree)) = widgets.get_mut(&name_lower) {
                    tree.add(&label);
                }
            });
            v_null()
        }
        "addchild" => {
            // AddChild(parentPath, childLabel)
            let parent = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let child = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let path = format!("{}/{}", parent, child);
            GUI_WIDGETS.with(|gw| {
                let mut widgets = gw.borrow_mut();
                if let Some(GuiWidget::Tree(ref mut tree)) = widgets.get_mut(&name_lower) {
                    tree.add(&path);
                }
            });
            v_null()
        }
        "clear" => {
            GUI_WIDGETS.with(|gw| {
                let mut widgets = gw.borrow_mut();
                if let Some(GuiWidget::Tree(ref mut tree)) = widgets.get_mut(&name_lower) {
                    tree.clear();
                }
            });
            v_null()
        }
        "expand" | "fullexpand" => {
            // Expand all or a specific node
            v_null()
        }
        "collapse" | "fullcollapse" => {
            v_null()
        }
        "show" => {
            gui_show(name);
            v_null()
        }
        "hide" => {
            gui_hide(name);
            v_null()
        }
        _ => {
            eprintln!("[WARN] TreeView.{}() not implemented", method);
            v_null()
        }
    }
}

// ---------------------------------------------------------------------------
// Canvas methods (drawing on a Frame widget via FLTK draw)
// ---------------------------------------------------------------------------

/// RImage method dispatch — loadfromfile, loadfromplot, etc.
pub fn image_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "loadfromfile" | "load" => {
            let path = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            if !path.is_empty() {
                load_image_file(&name_lower, &path);
            }
            v_null()
        }
        "loadfromplot" => {
            // Render the plot to PNG bytes in memory and load directly into the widget.
            let plot_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            #[cfg(feature = "datascience")]
            {
                let png_bytes = crate::datascience::plot_render_to_bytes(&plot_name);
                if !png_bytes.is_empty() {
                    if let Ok(mut img) = fltk::image::PngImage::from_data(&png_bytes) {
                        GUI_WIDGETS.with(|gw| {
                            let mut widgets = gw.borrow_mut();
                            if let Some(GuiWidget::ImageFrame(ref mut frm)) = widgets.get_mut(&name_lower) {
                                let w = frm.w();
                                let h = frm.h();
                                let stretch = rp_comp_get(&name_lower, "stretch").to_i64() != 0;
                                if stretch && w > 0 && h > 0 {
                                    img.scale(w, h, true, true);
                                }
                                frm.set_image(Some(img));
                                frm.redraw();
                            }
                        });
                    }
                }
            }
            #[cfg(not(feature = "datascience"))]
            {
                eprintln!("[WARN] datascience not compiled — loadfromplot unavailable");
                let _ = plot_name;
            }
            v_null()
        }
        "clear" | "cls" => {
            GUI_WIDGETS.with(|gw| {
                let mut widgets = gw.borrow_mut();
                if let Some(GuiWidget::ImageFrame(ref mut frm)) = widgets.get_mut(&name_lower) {
                    frm.set_image(None::<SharedImage>);
                    frm.redraw();
                }
            });
            v_null()
        }
        _ => {
            eprintln!("[WARN] RImage.{}() not implemented", method);
            v_null()
        }
    }
}

/// RapidQ's mouse button (mbLeft = 0, mbRight = 1, mbMiddle = 2) of the
/// current FLTK event.
fn mouse_button() -> i64 {
    match app::event_mouse_button() {
        app::MouseButton::Right => 1,
        app::MouseButton::Middle => 2,
        _ => 0,
    }
}

/// RapidQ's Shift state (RAPIDQ.INC: ssShift = 256, ssCtrl = 16, ssAlt = 1)
/// of the current FLTK event.
fn mouse_shift() -> i64 {
    let s = app::event_state();
    let mut shift = 0;
    if s.contains(fltk::enums::Shortcut::Shift) {
        shift |= 256;
    }
    if s.contains(fltk::enums::Shortcut::Ctrl) {
        shift |= 16;
    }
    if s.contains(fltk::enums::Shortcut::Alt) {
        shift |= 1;
    }
    shift
}

/// A QIMAGE's mouse events (manual): OnMouseDown / OnMouseUp (Button, X,
/// Y, Shift), OnMouseMove (X, Y, Shift), OnClick, OnDblClick; X and Y are
/// in the image.
fn picture_mouse(name: &str) -> impl FnMut(&mut Frame, Event) -> bool {
    let name = name.to_string();
    move |f, ev| {
        let (x, y) = (v_int((app::event_x() - f.x()) as i64), v_int((app::event_y() - f.y()) as i64));
        match ev {
            Event::Push => {
                press_begin(&name);
                crate::object::rp_fire_event_args(&name, "onmousedown", &[v_int(mouse_button()), x, y, v_int(mouse_shift())]);
                true
            }
            Event::Released => {
                if press_end(&name) {
                    crate::object::rp_fire_event_args(&name, "onmouseup", &[v_int(mouse_button()), x, y, v_int(mouse_shift())]);
                    if app::event_inside_widget(f) {
                        rp_fire_event(&name, if app::event_clicks() { "ondblclick" } else { "onclick" });
                    }
                }
                true
            }
            Event::Move | Event::Drag => {
                crate::object::rp_fire_event_args(&name, "onmousemove", &[x, y, v_int(mouse_shift())]);
                true
            }
            // Receive Move events.
            Event::Enter | Event::Leave => true,
            _ => false,
        }
    }
}

/// `MOUSEX` / `MOUSEY`: the mouse pointer relative to the active form's
/// client area (below its menu).
pub fn mouse_in_form() -> (i64, i64) {
    let (sx, sy) = app::get_mouse();
    let Some(win) = app::first_window() else { return (sx as i64, sy as i64) };
    let ptr = win.as_widget_ptr();
    let form = GUI_WIDGETS.with(|gw| {
        gw.try_borrow().ok().and_then(|w| {
            w.iter().find_map(|(n, g)| match g {
                GuiWidget::Window(v) if v.as_widget_ptr() == ptr => Some(n.clone()),
                _ => None,
            })
        })
    });
    let menu = form.as_deref().map_or(0, menu_offset);
    ((sx - win.x_root()) as i64, (sy - win.y_root() - menu) as i64)
}

/// Shows a QIMAGE's picture (rapidr_value::objects, a Bitmap): at the top
/// left, centered (Center) or scaled to the control (Stretch); with
/// Transparent, pixels of the transparent color show what's behind.
/// An image without a picture keeps what it shows (a PNG loaded by FLTK).
pub fn picture_refresh(name: &str) {
    let name_lower = name.to_lowercase();
    let Some(Some((w, h, rgba, transparent))) = rapidr_value::objects::with_picture(&name_lower, |b| {
        (!b.img.pixels.is_empty()).then(|| (b.img.width as i32, b.img.height as i32, b.to_rgba(), b.transparent))
    }) else {
        return;
    };
    let stretch = rp_comp_get(&name_lower, "stretch").to_bool();
    let center = rp_comp_get(&name_lower, "center").to_bool();
    GUI_WIDGETS.with(|gw| {
        let Ok(mut widgets) = gw.try_borrow_mut() else { return };
        let Some(GuiWidget::ImageFrame(frm)) = widgets.get_mut(&name_lower) else { return };
        let Ok(img) = RgbImage::new(&rgba, w, h, ColorDepth::Rgba8) else { return };
        let Ok(mut img) = SharedImage::from_image(&img) else { return };
        if stretch && frm.w() > 0 && frm.h() > 0 {
            img.scale(frm.w(), frm.h(), false, true);
        }
        frm.set_frame(if transparent { FrameType::NoBox } else { FrameType::FlatBox });
        frm.set_align(if center || stretch { Align::Center | Align::Inside } else { Align::Left | Align::Top | Align::Inside });
        frm.set_image(Some(img));
        frm.redraw();
        if transparent {
            if let Some(mut parent) = frm.parent() {
                parent.redraw();
            }
        }
    });
}

/// An image for a widget: a QBITMAP (by id, or the `data:` URL its `.BMP`
/// returns), a BMP file, or any other image file FLTK reads.
fn load_shared_image(src: &str) -> Option<SharedImage> {
    let bitmap = src.starts_with("data:") || rapidr_value::objects::exists(src) || src.to_ascii_lowercase().ends_with(".bmp");
    if bitmap {
        if let Ok(b) = rapidr_value::objects::load_image(&v_str(src)) {
            let rgb = RgbImage::new(&b.to_rgba(), b.img.width as i32, b.img.height as i32, ColorDepth::Rgba8).ok()?;
            return SharedImage::from_image(&rgb).ok();
        }
    }
    SharedImage::load(src).ok()
}

/// Load an image file into an RImage widget.
fn load_image_file(name: &str, path: &str) {
    if let Some(mut img) = load_shared_image(path) {
        GUI_WIDGETS.with(|gw| {
            let mut widgets = gw.borrow_mut();
            if let Some(GuiWidget::ImageFrame(ref mut frm)) = widgets.get_mut(name) {
                let w = frm.w();
                let h = frm.h();
                let stretch = rp_comp_get(name, "stretch").to_i64() != 0;
                if stretch && w > 0 && h > 0 {
                    img.scale(w, h, true, true);
                }
                frm.set_image(Some(img));
                frm.redraw();
            }
        });
    } else {
        eprintln!("[WARN] RImage: could not load '{}'", path);
    }
}

thread_local! {
}

pub fn canvas_method(name: &str, method: &str, _args: &[Value]) -> Value {
    match method {
        // Drawing is the shared model's (objects::call); these are the widget's.
        "paint" | "refresh" | "update" | "repaint" => {
            redraw_widget(&name.to_lowercase());
            crate::object::rp_fire_event(name, "onpaint");
        }
        "show" => gui_show(name),
        "hide" => gui_hide(name),
        _ => eprintln!("[WARN] Canvas.{}() not implemented", method),
    }
    v_null()
}

// ---------------------------------------------------------------------------
// QFORMMDI child windows (the model: rapidr_value::mdi; mdi.rs applies it)
// ---------------------------------------------------------------------------

/// A drag on a child window's frame: its frame component, what it moves
/// (true: the size), and where the mouse and the frame started.
struct MdiDrag {
    frame: String,
    resize: bool,
    mouse: (i32, i32),
    start: (i64, i64),
}

thread_local! {
    static MDI_DRAG: RefCell<Option<MdiDrag>> = const { RefCell::new(None) };
}

/// An `RMDICHILD`: a child window's frame — border, title bar with the
/// title and its buttons — drawn from its component (`caption`, `active`).
fn mdi_frame_create(name: &str, x: i32, y: i32, w: i32, h: i32) {
    use rapidr_value::mdi::{Action, BORDER, TITLE_HEIGHT};
    let mut f = Frame::new(x, y, w, h, None);
    let draw_name = name.to_string();
    f.draw(move |f| {
        let (b, t) = (BORDER as i32, TITLE_HEIGHT as i32);
        let active = rp_comp_get(&draw_name, "active").to_bool();
        let title = rp_comp_get(&draw_name, "caption").to_string_val();
        let maximized = rp_comp_get(&draw_name, "childstate").to_i64() == 2;
        draw::draw_box(FrameType::UpBox, f.x(), f.y(), f.w(), f.h(), Color::from_rgb(212, 208, 200));
        let bar = if active { Color::from_rgb(10, 36, 106) } else { Color::from_rgb(128, 128, 128) };
        draw::set_draw_color(bar);
        draw::draw_rectf(f.x() + b, f.y() + b, f.w() - 2 * b, t - 2);
        draw::set_draw_color(Color::White);
        draw::set_font(Font::HelveticaBold, 12);
        draw::push_clip(f.x() + b + 4, f.y() + b, (f.w() - 2 * b - 3 * (t - 2) - 8).max(0), t - 2);
        draw::draw_text2(&title, f.x() + b + 4, f.y() + b, f.w(), t - 2, Align::Left | Align::Inside);
        draw::pop_clip();
        for (slot, glyph) in [(0, "x"), (1, if maximized { "=" } else { "\u{25a1}" }), (2, "_")] {
            let bx = f.x() + f.w() - b - (slot + 1) * (t - 2) + 1;
            draw::draw_box(FrameType::UpBox, bx, f.y() + b + 2, t - 5, t - 6, Color::from_rgb(212, 208, 200));
            draw::set_draw_color(Color::Black);
            draw::set_font(Font::HelveticaBold, 11);
            draw::draw_text2(glyph, bx, f.y() + b + 2, t - 5, t - 6, Align::Center);
        }
    });
    let frame = name.to_string();
    f.handle(move |f, ev| {
        let form = rp_comp_get(&frame, "__form").to_string_val();
        let component = rp_comp_get(&frame, "__component").to_string_val();
        let (ex, ey) = (app::event_x() - f.x(), app::event_y() - f.y());
        match ev {
            Event::Push => {
                if let Some(action) = rapidr_value::mdi::button_at(f.w() as i64, ex as i64, ey as i64) {
                    crate::mdi::user(&form, &component, action);
                    return true;
                }
                if ey < (BORDER + TITLE_HEIGHT) as i32 && app::event_clicks() {
                    crate::mdi::user(&form, &component, Action::ToggleMaximize);
                    return true;
                }
                crate::mdi::user(&form, &component, Action::Activate);
                let corner = ex > f.w() - 12 && ey > f.h() - 12;
                let start = if corner {
                    (rp_comp_get(&frame, "width").to_i64(), rp_comp_get(&frame, "height").to_i64())
                } else {
                    (rp_comp_get(&frame, "left").to_i64(), rp_comp_get(&frame, "top").to_i64())
                };
                if corner || ey < (BORDER + TITLE_HEIGHT) as i32 {
                    MDI_DRAG.with(|d| *d.borrow_mut() = Some(MdiDrag { frame: frame.clone(), resize: corner, mouse: (app::event_x(), app::event_y()), start }));
                }
                true
            }
            Event::Drag => {
                let Some((resize, mouse, start)) = MDI_DRAG.with(|d| d.borrow().as_ref().filter(|d| d.frame == frame).map(|d| (d.resize, d.mouse, d.start))) else { return false };
                let (dx, dy) = ((app::event_x() - mouse.0) as i64, (app::event_y() - mouse.1) as i64);
                let action = if resize { Action::Resize(start.0 + dx, start.1 + dy) } else { Action::Move(start.0 + dx, start.1 + dy) };
                crate::mdi::user(&form, &component, action);
                true
            }
            Event::Released => {
                MDI_DRAG.with(|d| *d.borrow_mut() = None);
                true
            }
            _ => false,
        }
    });
    GUI_WIDGETS.with(|gw| {
        gw.borrow_mut().insert(name.to_lowercase(), GuiWidget::Frame(f));
    });
}

/// Puts these widgets on top of their parents' other children, in order
/// (the last on top): a QFORMMDI's frames and components.
pub(crate) fn stack_widgets(names: &[String]) {
    for name in names {
        let Some(w) = GUI_WIDGETS.with(|gw| gw.borrow().get(&name.to_lowercase()).cloned()) else { continue };
        let w = w.base();
        if let Some(mut parent) = w.parent() {
            if parent.find(&w) != parent.children() - 1 {
                parent.remove(&w);
                parent.add(&w);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Widget property updates (called when properties change at runtime)
// ---------------------------------------------------------------------------

/// Update the visible state of a widget.
pub fn gui_set_visible(name: &str, visible: bool) {
    let name_lower = name.to_lowercase();
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(widget) = widgets.get_mut(&name_lower) {
            match widget {
                GuiWidget::Window(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Button(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Frame(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Input(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::CheckButton(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::RadioButton(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Choice(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::InputChoice(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::HoldBrowser(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::TextEditor(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Group(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Tabs(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::MenuBar(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::SysMenuBar(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Progress(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Scroll(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Grid(ref mut w, _) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Tree(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Slider(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::ImageFrame(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
            }
        }
    });
}

/// Update the widget caption or text.
pub fn gui_set_caption(name: &str, text: &str) {
    let name_lower = name.to_lowercase();
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(widget) = widgets.get_mut(&name_lower) {
            match widget {
                GuiWidget::Frame(ref mut w) => { w.set_label(text); }
                GuiWidget::Button(ref mut w) => { w.set_label(text); }
                GuiWidget::Window(ref mut w) => { w.set_label(text); }
                GuiWidget::Group(ref mut w) if rp_comp_type(&name_lower) == "RPANEL" => {
                    w.set_label(text);
                    w.redraw();
                }
                _ => {}
            }
        }
    });
}

/// Update the text content of a TextEditor/TextBuffer.
pub fn gui_set_text(name: &str, text: &str) {
    let name_lower = name.to_lowercase();
    // Clone the buffer handle (cheap pointer clone) and release the RefCell borrow
    // BEFORE calling set_text, because set_text fires the modify callback synchronously
    // which tries to borrow the same RefCell → "RefCell already mutably borrowed" panic.
    let buf_clone = GUI_TEXT_BUFFERS.with(|tb| {
        tb.borrow().get(&name_lower).cloned()
    });
    if let Some(mut buf) = buf_clone {
        buf.set_text(text);
    }
    // The modify callback already handles syntax re-highlighting,
    // so no explicit re-highlight is needed here.
}

/// Get the text content of a TextEditor/TextBuffer.
pub fn gui_get_text(name: &str) -> String {
    let name_lower = name.to_lowercase();
    GUI_TEXT_BUFFERS.with(|tb| {
        let bufs = tb.borrow();
        bufs.get(&name_lower).map(|b| b.text()).unwrap_or_default()
    })
}

/// Get the current value of an Input widget (REDIT).
/// Returns None if the widget doesn't exist or isn't an Input.
pub fn gui_get_input_value(name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();
    GUI_WIDGETS.with(|gw| {
        let widgets = gw.borrow();
        if let Some(GuiWidget::Input(ref inp)) = widgets.get(&name_lower) {
            Some(inp.value())
        } else {
            None
        }
    })
}

/// Set the value of an Input widget (REDIT).
pub fn gui_set_input_value(name: &str, text: &str) {
    let name_lower = name.to_lowercase();
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(GuiWidget::Input(ref mut inp)) = widgets.get_mut(&name_lower) {
            let _ = inp.set_value(text);
        }
    });
}

/// A QSTATUSBAR: its panels left to right (`Panel(i).Width` wide, 100 by
/// default; the last one takes the rest), or its SimpleText when
/// `SimplePanel` is set or it has no panels.
fn draw_statusbar(id: &str, x: i32, y: i32, w: i32, h: i32) {
    draw::draw_box(FrameType::FlatBox, x, y, w, h, Color::BackGround);
    draw::set_font(Font::Helvetica, 12);
    draw::set_draw_color(Color::Black);
    let count = rp_comp_get(id, "panelcount").to_i64().clamp(0, 256) as i32;
    if count == 0 || rp_comp_get(id, "simplepanel").to_bool() {
        let text = rp_comp_get(id, "simpletext").to_string_val();
        draw::draw_box(FrameType::ThinDownBox, x + 1, y + 2, w - 2, h - 3, Color::BackGround);
        draw::set_draw_color(Color::Black);
        draw::draw_text2(&text, x + 5, y, w - 10, h, Align::Left | Align::Inside | Align::Clip);
        return;
    }
    let mut px = x + 1;
    for i in 0..count {
        let width = rp_comp_get(id, &format!("panel({i}).width")).to_i64();
        let pw = if i == count - 1 { (x + w - 1 - px).max(0) } else if width > 0 { width.min(10_000) as i32 } else { 100 };
        let caption = rp_comp_get(id, &format!("panel({i}).caption")).to_string_val();
        draw::draw_box(FrameType::ThinDownBox, px, y + 2, pw - 2, h - 3, Color::BackGround);
        draw::set_draw_color(Color::Black);
        draw::push_clip(px + 2, y, (pw - 6).max(0), h);
        draw::draw_text2(&caption, px + 4, y, (pw - 8).max(0), h, Align::Left | Align::Inside);
        draw::pop_clip();
        px += pw;
    }
}

/// Fills a QLISTBOX's browser or a QCOMBOBOX's choice from its items
/// (rapidr_value::objects::list), selecting ItemIndex. Items are shown as
/// written: FLTK's `@` formatting codes and a menu's `/`, `&` and `\` are
/// escaped.
/// Shows a QDIRTREE's rows (indented, `+` closed / `-` open), the
/// selected directory selected.
pub fn dirtree_refresh(name: &str) {
    let name = name.to_lowercase();
    let Some((lines, selected)) = rapidr_value::objects::with_dirtree(&name, |t| {
        let lines: Vec<String> = t.rows().iter().map(rapidr_value::objects::dirtree::DirTree::row_text).collect();
        (lines, t.selected_row())
    }) else {
        return;
    };
    let widget = GUI_WIDGETS.with(|gw| gw.borrow().get(&name).cloned());
    if let Some(GuiWidget::HoldBrowser(mut b)) = widget {
        // Keep the scroll position.
        let top = b.position();
        b.clear();
        for line in &lines {
            b.add(&format!("@.{line}"));
        }
        b.set_position(top);
        if let Some(i) = selected {
            b.select(i as i32 + 1);
            // The selected directory in view.
            if !b.displayed(i as i32 + 1) {
                b.middle_line(i as i32 + 1);
            }
        }
        b.redraw();
    }
}

pub fn list_refresh(name: &str) {
    let name = name.to_lowercase();
    let Some((items, index, top, multi)) = rapidr_value::objects::with_list(&name, |l| {
        let multi = l.multi_select.then(|| (0..l.items.len()).map(|i| l.is_selected(i)).collect::<Vec<_>>());
        (l.items.clone(), l.item_index, l.top_index, multi)
    }) else {
        return;
    };
    let widget = GUI_WIDGETS.with(|gw| gw.borrow().get(&name).cloned());
    match widget {
        Some(GuiWidget::HoldBrowser(mut b)) => {
            b.clear();
            // A MultiSelect list box shows several selected items.
            b.set_type(if multi.is_some() { fltk::browser::BrowserType::Multi } else { fltk::browser::BrowserType::Hold });
            for item in &items {
                b.add(&format!("@.{}", item.replace(['\n', '\r', '\t'], " ")));
            }
            match &multi {
                Some(flags) => {
                    for (i, _) in flags.iter().enumerate().filter(|(_, on)| **on) {
                        b.select(i as i32 + 1);
                    }
                }
                None if index >= 0 => b.select(index as i32 + 1),
                None => {}
            }
            if top > 0 {
                b.top_line(top as i32 + 1);
            }
            b.redraw();
        }
        // An owner-drawn list box: a table with a row per item, each as
        // tall as OnMeasureItem said (lbOwnerDrawVariable).
        Some(GuiWidget::Grid(mut t, _)) => {
            if list_measure(&name) {
                return;
            }
            let heights = rapidr_value::objects::with_list(&name, |l| (0..l.items.len()).map(|i| l.item_h(i) as i32).collect::<Vec<_>>()).unwrap_or_default();
            t.set_rows(items.len() as i32);
            for (i, h) in heights.iter().enumerate() {
                t.set_row_height(i as i32, *h);
            }
            t.set_col_width_all((t.w() - 4 - 16).max(10));
            if top > 0 {
                t.set_row_position(top as i32);
            }
            t.redraw();
            list_owner_draw(&name);
        }
        Some(GuiWidget::Choice(mut c)) => {
            c.clear();
            for item in &items {
                c.add_choice(&menu_label(item));
            }
            c.set_value(index as i32);
            c.redraw();
        }
        Some(GuiWidget::InputChoice(mut c)) => {
            c.clear();
            for item in &items {
                c.add(&menu_label(item));
            }
            // The edit box shows the Text (typed, or the picked item's).
            let text = rapidr_value::objects::with_list(&name, |l| l.text.clone()).unwrap_or_default();
            if c.value().as_deref() != Some(text.as_str()) {
                c.set_value(&text);
            }
            c.redraw();
        }
        _ => {}
    }
}

/// An owner-drawn list box (`Style = lbOwnerDrawFixed / lbOwnerDrawVariable`):
/// a table of one column whose cells are the items, drawn from
/// rapidr_value::objects::list (`render_item`) — what OnDrawItem drew, or
/// the plain item. A click or the arrow keys select an item (OnClick).
fn owner_list_create(name: &str, x: i32, y: i32, w: i32, h: i32) {
    let mut table = Table::new(x, y, w, h, None);
    table.set_frame(FrameType::DownBox);
    table.set_color(Color::White);
    let hidden = Input::new(0, 0, 0, 0, None);
    table.end();
    let mut hidden = hidden;
    hidden.hide();
    table.set_cols(1);
    table.set_col_header(false);
    table.set_row_header(false);
    table.set_col_resize(false);
    table.set_row_resize(false);
    let draw_name = name.to_string();
    table.draw_cell(move |t, ctx, row, _col, cx, cy, cw, ch| {
        if ctx != TableContext::Cell || row < 0 {
            return;
        }
        let font = rapidr_value::objects::font_from_props(&draw_name, &|id, p| rp_comp_get(id, p));
        let Some((iw, ih, rgba)) = rapidr_value::objects::list_item_pixels(&draw_name, row as usize, cw as i64, &font) else { return };
        if let Ok(mut img) = RgbImage::new(&rgba, iw as i32, ih as i32, ColorDepth::Rgba8) {
            draw::push_clip(cx, cy, cw, ch);
            img.draw(cx, cy, iw as i32, ih as i32);
            draw::pop_clip();
        }
        let _ = t;
    });
    let handle_name = name.to_string();
    table.handle(move |t, ev| owner_list_handle(&handle_name, t, ev));
    GUI_WIDGETS.with(|gw| {
        gw.borrow_mut().insert(name.to_string(), GuiWidget::Grid(table, hidden));
    });
    list_refresh(name);
}

/// Selects item `i` the way a click does (a MultiSelect list toggles it).
fn owner_list_select(name: &str, i: i64) {
    rapidr_value::objects::with_list_mut(name, |l| {
        if l.multi_select {
            if let Some(s) = l.selected.get_mut(i as usize) {
                *s = !*s;
            }
            l.item_index = i;
        } else {
            l.select(i);
        }
    });
}

fn owner_list_handle(name: &str, t: &mut Table, ev: Event) -> bool {
    let count = rapidr_value::objects::with_list(name, |l| l.items.len() as i64).unwrap_or(0);
    match ev {
        Event::Push => {
            let _ = t.take_focus();
            if let Some((TableContext::Cell, row, _, _)) = t.cursor2rowcol() {
                if (row as i64) < count {
                    owner_list_select(name, row as i64);
                    list_refresh(name);
                    rp_fire_event(name, if app::event_clicks() { "ondblclick" } else { "onclick" });
                }
            }
            true
        }
        Event::Focus | Event::Unfocus => true,
        Event::KeyDown => {
            let current = rapidr_value::objects::with_list(name, |l| l.item_index).unwrap_or(-1);
            let page = i64::from((t.h() / rapidr_value::objects::with_list(name, |l| l.row_height()).unwrap_or(16) as i32).max(1));
            let next = match app::event_key() {
                Key::Down => current + 1,
                Key::Up => (current - 1).max(0),
                Key::PageDown => current + page,
                Key::PageUp => (current - page).max(0),
                Key::Home => 0,
                Key::End => count - 1,
                _ => return false,
            };
            let next = next.clamp(0, (count - 1).max(0));
            if count > 0 && next != current {
                owner_list_select(name, next);
                // Scrolled so the item shows.
                let (top, bottom, _, _) = t.visible_cells();
                if (next as i32) < top {
                    t.set_row_position(next as i32);
                } else if (next as i32) > bottom {
                    t.set_row_position((next as i32 - (bottom - top)).max(0));
                }
                list_refresh(name);
                rp_fire_event(name, "onclick");
            }
            true
        }
        _ => false,
    }
}

/// OnMeasureItem(Index, Height) for each item of a `lbOwnerDrawVariable`
/// list whose items changed (rapidr_value::objects::list::ItemList::
/// measure_needed): each answer is the item's height, and the list is shown
/// again once the last is in. `true` while answers are still to come.
fn list_measure(name: &str) -> bool {
    if crate::object::rp_has_handler(name, "onmeasureitem") {
        let asks = rapidr_value::objects::with_list_mut(name, |l| l.measure_needed()).unwrap_or_default();
        for (round, i, h) in asks {
            let list = name.to_string();
            rp_fire_event_then(name, "onmeasureitem", &[v_int(i as i64), v_int(h)], move |a| {
                if rapidr_value::objects::with_list_mut(&list, |l| l.measured(round, i, a[1].to_i64())).unwrap_or(false) {
                    list_refresh(&list);
                }
            });
        }
    }
    rapidr_value::objects::with_list(name, |l| l.measuring()).unwrap_or(false)
}

/// OnDrawItem(Index, State, Rect): fired for every item after the list
/// changed (rapidr_value::objects::list::ItemList::owner_draw_needed); what
/// the handler draws is kept per item and shown by [`owner_list_create`]'s
/// cells. Each item's Rect is a QRECT (a property bag).
fn list_owner_draw(name: &str) {
    if !crate::object::rp_has_handler(name, "ondrawitem") {
        return;
    }
    if !rapidr_value::objects::with_list_mut(name, |l| l.owner_draw_needed()).unwrap_or(false) {
        return;
    }
    let width = GUI_WIDGETS.with(|gw| match gw.borrow().get(name) {
        Some(GuiWidget::Grid(t, _)) => t.col_width(0) as i64,
        _ => 0,
    });
    let items = rapidr_value::objects::with_list(name, |l| l.owner_draw_items(width)).unwrap_or_default();
    for (i, state, (left, top, right, bottom)) in items {
        let rect = format!("{name}.itemrect({i})");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            crate::object::rp_comp_set(&rect, prop, v_int(v));
        }
        rp_fire_event_args(name, "ondrawitem", &[v_int(i as i64), v_int(state), v_str(&rect)]);
    }
}

/// An item as an FLTK menu entry, shown as written: `\`, `/`, `&` and a
/// leading `_` are FLTK menu syntax.
fn menu_label(item: &str) -> String {
    let label = item.replace('\\', "\\\\").replace('/', "\\/").replace('&', "&&").replace(['\n', '\r', '\t'], " ");
    if label.starts_with('_') { format!("\\{label}") } else { label }
}

/// Whether a QLISTVIEW shows its column header (it has columns, and
/// ShowColumnHeaders isn't False).
fn listview_header_shown(name: &str) -> bool {
    let has_columns = rapidr_value::objects::with_listview(name, |lv| !lv.columns.is_empty()).unwrap_or(false);
    let show = rp_comp_get(name, "showcolumnheaders");
    has_columns && (matches!(show, Value::Null) || show.to_bool())
}

/// Fills a QLISTVIEW's browser from its data (rapidr_value::objects::
/// listview): a bold header line, then one line per item — the caption and
/// its sub-items in the columns. Text is shown as-is (`@.` turns off FLTK's
/// `@` formatting codes).
pub fn listview_refresh(name: &str) {
    let name_lower = name.to_lowercase();
    let header = listview_header_shown(&name_lower);
    let Some((widths, lines, selected)) = rapidr_value::objects::with_listview(&name_lower, |lv| {
        let clean = |t: &str| t.replace(['\t', '\n', '\r'], " ");
        let widths: Vec<i32> = lv.columns.iter().map(|c| c.width.clamp(1, 10_000) as i32).collect();
        let n_cols = lv.columns.len().max(1);
        let mut lines = Vec::with_capacity(lv.items.len() + 1);
        if header {
            lines.push(lv.columns.iter().map(|c| format!("@B49@b@.{}", clean(&c.caption))).collect::<Vec<_>>().join("\t"));
        }
        for item in &lv.items {
            let cells = std::iter::once(&item.caption).chain(item.sub_items.iter()).take(n_cols);
            lines.push(cells.map(|c| format!("@.{}", clean(c))).collect::<Vec<_>>().join("\t"));
        }
        (widths, lines, lv.item_index)
    }) else {
        return;
    };
    GUI_WIDGETS.with(|gw| {
        if let Some(GuiWidget::HoldBrowser(b)) = gw.borrow_mut().get_mut(&name_lower) {
            let scroll = b.position();
            b.clear();
            if !widths.is_empty() {
                b.set_column_widths(&widths);
            }
            for line in &lines {
                b.add(line);
            }
            if selected >= 0 {
                b.select(selected as i32 + 1 + header as i32);
            }
            b.set_position(scroll);
            b.redraw();
        }
    });
}

// ---------------------------------------------------------------------------
// QSTRINGGRID
// ---------------------------------------------------------------------------
//
// The grid's data lives in rapidr_value::objects::grid (shared with the web
// runtime); this is its view: an FLTK table whose column header is the first
// fixed row and whose row header is the first fixed column (more fixed rows
// or columns are drawn like them but scroll). Clicking selects a cell
// (OnSelectCell, OnClick), double-clicking OnDblClick; with goEditing,
// Enter, F2, typing or a double click edit the cell in place (every click
// with goAlwaysShowEditor) and Enter / leaving the cell stores it
// (OnSetEditText, then RapidR's OnChange). An ellipsis column (ColumnStyle
// gcsEllipsis) — or, for RapidR's IDE, a cell reading "..." — shows a
// button: OnEllipsisClick(Col, Row) (and OnDblClick).

/// The header row / column the table shows (0 or 1 each).
fn grid_headers(name: &str) -> (i32, i32) {
    rapidr_value::objects::with_grid(name, |g| (g.fixed_rows().min(1) as i32, g.fixed_cols().min(1) as i32)).unwrap_or((0, 0))
}

/// The grid cell (col, row) a table context/row/col shows.
fn grid_cell_of(name: &str, ctx: TableContext, row: i32, col: i32) -> Option<(i64, i64)> {
    let (hr, hc) = grid_headers(name);
    let (c, r) = match ctx {
        TableContext::Cell => (col + hc, row + hr),
        TableContext::ColHeader => (col + hc, 0),
        TableContext::RowHeader => (0, row + hr),
        _ => return None,
    };
    Some((c as i64, r as i64))
}

/// Whether cell (col, row) shows an ellipsis button.
fn grid_has_ellipsis(g: &rapidr_value::objects::grid::StringGrid, col: usize, row: usize) -> bool {
    let fixed = row < g.fixed_rows() || col < g.fixed_cols();
    !fixed && (g.column_style(col) == rapidr_value::objects::grid::GCS_ELLIPSIS || g.cell(col, row) == "...")
}

fn grid_create(name: &str, x: i32, y: i32, w: i32, h: i32) {
    let mut table = Table::new(x, y, w, h, None);
    table.set_frame(FrameType::DownBox);
    table.set_color(Color::White);
    let mut editor = Input::new(0, 0, 0, 0, None);
    editor.set_frame(FrameType::BorderBox);
    editor.hide();
    table.end();

    let draw_name = name.to_string();
    table.draw_cell(move |t, ctx, row, col, cx, cy, cw, ch| grid_draw_cell(&draw_name, t, ctx, row, col, cx, cy, cw, ch));

    // Enter stores the edited cell.
    let enter_name = name.to_string();
    editor.set_trigger(CallbackTrigger::EnterKeyAlways);
    editor.set_callback(move |_| grid_finish_edit(&enter_name, true));
    // Leaving the cell stores it too; Escape drops the edit.
    let edit_name = name.to_string();
    editor.handle(move |_, ev| match ev {
        Event::Unfocus => {
            grid_finish_edit(&edit_name, true);
            false
        }
        Event::KeyDown if app::event_key() == Key::Escape => {
            grid_finish_edit(&edit_name, false);
            true
        }
        _ => false,
    });

    let handle_name = name.to_string();
    table.handle(move |t, ev| grid_handle(&handle_name, t, ev));
    GUI_WIDGETS.with(|gw| {
        gw.borrow_mut().insert(name.to_string(), GuiWidget::Grid(table, editor));
    });
    grid_refresh(name);
}

/// Sizes the table from the grid's data and redraws it.
pub fn grid_refresh(name: &str) {
    let name = name.to_lowercase();
    let Some((rows, cols, widths, heights, (hr, hc), (col_sizing, row_sizing))) = rapidr_value::objects::with_grid(&name, |g| {
        use rapidr_value::objects::grid::{GO_COL_SIZING, GO_ROW_SIZING};
        let hr = g.fixed_rows().min(1);
        let hc = g.fixed_cols().min(1);
        (g.row_count(), g.col_count(), g.col_widths.clone(), g.row_heights.clone(), (hr, hc), (g.has_option(GO_COL_SIZING), g.has_option(GO_ROW_SIZING)))
    }) else {
        return;
    };
    GUI_WIDGETS.with(|gw| {
        if let Some(GuiWidget::Grid(t, _)) = gw.borrow_mut().get_mut(&name) {
            let size = |v: i64| v.clamp(0, 10_000) as i32;
            t.set_col_header(hr == 1);
            t.set_row_header(hc == 1);
            // goColSizing / goRowSizing: the user drags the headers' borders
            // ([`grid_sync_sizes`] keeps the new sizes).
            t.set_col_resize(col_sizing);
            t.set_row_resize(row_sizing);
            if hr == 1 {
                t.set_col_header_height(size(heights[0]));
            }
            if hc == 1 {
                t.set_row_header_width(size(widths[0]));
            }
            t.set_rows((rows - hr) as i32);
            t.set_cols((cols - hc) as i32);
            for (i, w) in widths.iter().enumerate().skip(hc) {
                t.set_col_width((i - hc) as i32, size(*w));
            }
            for (i, h) in heights.iter().enumerate().skip(hr) {
                t.set_row_height((i - hr) as i32, size(*h));
            }
            t.redraw();
        }
    });
    grid_owner_draw(&name);
}

/// The user resized columns or rows (goColSizing / goRowSizing): their new
/// sizes go into ColWidths / RowHeights.
fn grid_sync_sizes(name: &str, t: &Table) {
    let (hr, hc) = grid_headers(name);
    let changed = rapidr_value::objects::with_grid_mut(name, |g| {
        let before = (g.col_widths.clone(), g.row_heights.clone());
        for i in 0..g.col_widths.len() {
            let w = if i < hc as usize { t.row_header_width() } else { t.col_width(i as i32 - hc) };
            g.col_widths[i] = w.max(0) as i64;
        }
        for i in 0..g.row_heights.len() {
            let h = if i < hr as usize { t.col_header_height() } else { t.row_height(i as i32 - hr) };
            g.row_heights[i] = h.max(0) as i64;
        }
        before != (g.col_widths.clone(), g.row_heights.clone())
    })
    .unwrap_or(false);
    if changed {
        grid_refresh(name);
    }
}

/// OnDrawCell(Col, Row, State, Rect): fired for every cell after the grid
/// changed (rapidr_value::objects::grid::StringGrid::owner_draw_needed);
/// what the handler draws is kept per cell and drawn over it
/// ([`grid_draw_cell`]). Each cell's Rect is a QRECT (a property bag).
fn grid_owner_draw(name: &str) {
    if !crate::object::rp_has_handler(name, "ondrawcell") {
        return;
    }
    if !rapidr_value::objects::with_grid_mut(name, |g| g.owner_draw_needed()).unwrap_or(false) {
        return;
    }
    let cells = rapidr_value::objects::with_grid(name, |g| g.owner_draw_cells()).unwrap_or_default();
    for (col, row, state, (left, top, right, bottom)) in cells {
        let rect = format!("{name}.cellrect({col},{row})");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            crate::object::rp_comp_set(&rect, prop, v_int(v));
        }
        rp_fire_event_args(name, "ondrawcell", &[v_int(col as i64), v_int(row as i64), v_int(state), v_str(&rect)]);
    }
}

/// An &HBBGGRR color as FLTK's.
fn bgr_color(c: u32) -> Color {
    Color::from_rgb(c as u8, (c >> 8) as u8, (c >> 16) as u8)
}

/// Draws what OnDrawCell drew on a cell whose top left is (x, y).
fn grid_replay(ops: &[rapidr_value::objects::grid::CellDraw], x: i32, y: i32) {
    use rapidr_value::objects::grid::CellDraw;
    let p = |v: i64| v.clamp(-100_000, 100_000) as i32;
    for op in ops {
        match op {
            CellDraw::Line(x1, y1, x2, y2, c) => {
                draw::set_draw_color(bgr_color(*c));
                draw::draw_line(x + p(*x1), y + p(*y1), x + p(*x2), y + p(*y2));
            }
            CellDraw::Rect(x1, y1, x2, y2, c) => {
                draw::set_draw_color(bgr_color(*c));
                draw::draw_rect(x + p(*x1.min(x2)), y + p(*y1.min(y2)), p((x2 - x1).abs()), p((y2 - y1).abs()));
            }
            CellDraw::Fill(x1, y1, x2, y2, c) => {
                draw::set_draw_color(bgr_color(*c));
                draw::draw_rectf(x + p(*x1.min(x2)), y + p(*y1.min(y2)), p((x2 - x1).abs()), p((y2 - y1).abs()));
            }
            CellDraw::Ellipse(x1, y1, x2, y2, c, fill) => {
                let (l, t, w, h) = (x + p(*x1.min(x2)), y + p(*y1.min(y2)), p((x2 - x1).abs()), p((y2 - y1).abs()));
                if let Some(f) = fill {
                    draw::set_draw_color(bgr_color(*f));
                    draw::draw_pie(l, t, w, h, 0.0, 360.0);
                }
                draw::set_draw_color(bgr_color(*c));
                draw::draw_arc(l, t, w, h, 0.0, 360.0);
            }
            CellDraw::Pixel(px, py, c) => {
                draw::set_draw_color(bgr_color(*c));
                draw::draw_point(x + p(*px), y + p(*py));
            }
            CellDraw::Text(tx, ty, text, c, bg) => {
                draw::set_font(Font::Helvetica, 13);
                let (tx, ty) = (x + p(*tx), y + p(*ty));
                if let Some(bg) = bg {
                    draw::set_draw_color(bgr_color(*bg));
                    draw::draw_rectf(tx, ty, draw::width(text) as i32, draw::height());
                }
                draw::set_draw_color(bgr_color(*c));
                draw::draw_text2(text, tx, ty, 0, draw::height(), Align::Left | Align::Top | Align::Inside);
            }
            CellDraw::Image(ix, iy, b) => {
                let (w, h) = (b.img.width as i32, b.img.height as i32);
                if w > 0 && h > 0 {
                    if let Ok(mut img) = RgbImage::new(&b.to_rgba(), w, h, ColorDepth::Rgba8) {
                        img.draw(x + p(*ix), y + p(*iy), w, h);
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn grid_draw_cell(name: &str, t: &mut Table, ctx: TableContext, row: i32, col: i32, x: i32, y: i32, w: i32, h: i32) {
    // The corner where the header row and column meet is cell (0, 0),
    // which the table itself leaves blank.
    if ctx == TableContext::EndPage && t.col_header() && t.row_header() {
        let (x, y) = (t.x() + t.frame().dx(), t.y() + t.frame().dy());
        let (w, h) = (t.row_header_width(), t.col_header_height());
        let text = rapidr_value::objects::with_grid(name, |g| g.cell(0, 0).to_string()).unwrap_or_default();
        draw::push_clip(x, y, w, h);
        draw::draw_box(FrameType::ThinUpBox, x, y, w, h, Color::from_rgb(212, 208, 200));
        draw::set_font(Font::Helvetica, 13);
        draw::set_draw_color(Color::Black);
        draw::draw_text2(&text, x + 3, y, (w - 6).max(0), h, Align::Left | Align::Inside | Align::Clip);
        draw::pop_clip();
        return;
    }
    let Some((c, r)) = grid_cell_of(name, ctx, row, col) else { return };
    let Some((text, fixed, selected, ellipsis, lines, drawn, list)) = rapidr_value::objects::with_grid(name, |g| {
        use rapidr_value::objects::grid::{GO_FIXED_HORZ_LINE, GO_HORZ_LINE};
        let (cu, ru) = (c as usize, r as usize);
        let fixed = ru < g.fixed_rows() || cu < g.fixed_cols();
        let selected = g.is_selected(cu, ru);
        let lines = if fixed { g.has_option(GO_FIXED_HORZ_LINE) } else { g.has_option(GO_HORZ_LINE) };
        let drawn = g.owner_drawing.get(&(cu, ru)).cloned();
        let list = (g.col, g.row) == (c, r) && g.list_items(cu, ru).is_some();
        (g.cell(cu, ru).to_string(), fixed, selected, grid_has_ellipsis(g, cu, ru), lines, drawn, list)
    }) else {
        return;
    };
    draw::push_clip(x, y, w, h);
    if fixed {
        draw::draw_box(FrameType::ThinUpBox, x, y, w, h, Color::from_rgb(212, 208, 200));
    } else {
        draw::set_draw_color(if selected { Color::from_rgb(0, 120, 215) } else { Color::White });
        draw::draw_rectf(x, y, w, h);
        if lines {
            draw::set_draw_color(Color::from_rgb(192, 192, 192));
            draw::draw_rect(x, y, w, h);
        }
    }
    let button = if ellipsis || list { h.min(w) } else { 0 };
    draw::set_font(Font::Helvetica, 13);
    draw::set_draw_color(if selected { Color::White } else { Color::Black });
    // At Left + 2, Top + 2, as Delphi's grid draws a cell's text (an
    // OnDrawCell TextOut there covers it exactly).
    if !(ellipsis && text == "...") {
        draw::draw_text2(&text, x + 2, y + 2, (w - 4 - button).max(0), (h - 2).max(0), Align::Left | Align::Top | Align::Inside | Align::Clip);
    }
    if ellipsis {
        draw::draw_box(FrameType::ThinUpBox, x + w - button, y, button, h, Color::from_rgb(230, 230, 230));
        draw::set_draw_color(Color::Black);
        draw::draw_text2("...", x + w - button, y, button, h, Align::Center);
    }
    // A gcsList column's selected cell: its drop-down button.
    if list {
        let (bx, bw) = (x + w - button, button);
        draw::draw_box(FrameType::ThinUpBox, bx, y, bw, h, Color::from_rgb(230, 230, 230));
        draw::set_draw_color(Color::Black);
        let (cx, cy) = (bx + bw / 2, y + h / 2);
        draw::draw_polygon(cx - 4, cy - 2, cx + 4, cy - 2, cx, cy + 2);
    }
    if let Some(ops) = drawn {
        grid_replay(&ops, x, y);
    }
    draw::pop_clip();
}

/// Selects a cell the way a click or an arrow key does (fixed cells can't
/// be selected). Returns whether it moved.
fn grid_select(name: &str, c: i64, r: i64) -> bool {
    grid_user_select(name, c, r, false)
}

/// The user dragged or shift-clicked to a cell: with goRangeSelect the
/// range grows to it (rapidr_value::objects::grid::StringGrid::extend_to).
/// Returns whether the selection changed.
fn grid_extend(name: &str, c: i64, r: i64) -> bool {
    grid_user_select(name, c, r, true)
}

/// A selection the user made (`StringGrid::user_select`): when it moves,
/// OnSelectCell(Col, Row, CanSelect) is fired, and `CanSelect = 0` puts the
/// selection back.
fn grid_user_select(name: &str, c: i64, r: i64, extend: bool) -> bool {
    let Some(before) = rapidr_value::objects::with_grid_mut(name, |g| g.user_select(c, r, extend)).flatten() else {
        return false;
    };
    let grid = name.to_string();
    rp_fire_event_then(name, "onselectcell", &[v_int(c), v_int(r), v_int(-1)], move |a| {
        if a[2].to_i64() == 0 {
            rapidr_value::objects::with_grid_mut(&grid, |g| g.set_selection(before));
            grid_refresh(&grid);
        }
    });
    grid_refresh(name);
    true
}

/// Starts editing the selected cell (with `initial` text, or its own).
fn grid_start_edit(name: &str, t: &mut Table, initial: Option<String>) {
    let Some((c, r, text, editable)) = rapidr_value::objects::with_grid(name, |g| (g.col, g.row, g.cell(g.col.max(0) as usize, g.row.max(0) as usize).to_string(), g.editable())) else {
        return;
    };
    if !editable || c < 0 || r < 0 {
        return;
    }
    let (hr, hc) = grid_headers(name);
    let Some((x, y, w, h)) = t.find_cell(TableContext::Cell, r as i32 - hr, c as i32 - hc) else { return };
    GUI_WIDGETS.with(|gw| {
        if let Some(GuiWidget::Grid(_, editor)) = gw.borrow_mut().get_mut(name) {
            editor.resize(x, y, w, h);
            editor.set_value(initial.as_deref().unwrap_or(&text));
            editor.show();
            let _ = editor.take_focus();
            let end = editor.value().len() as i32;
            let _ = editor.set_position(end);
            editor.redraw();
        }
    });
}

/// Ends an edit: stores the text (`keep`) and fires OnSetEditText(Col,
/// Row, Value$) and OnChange.
fn grid_finish_edit(name: &str, keep: bool) {
    let value = GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        let Some(GuiWidget::Grid(t, editor)) = widgets.get_mut(name) else { return None };
        if !editor.visible() {
            return None;
        }
        editor.hide();
        t.redraw();
        Some(editor.value())
    });
    let (Some(value), true) = (value, keep) else { return };
    grid_store(name, value);
}

/// The user entered `value` in the selected cell (edited it, or picked it
/// from a gcsList column's drop-down): stored, then OnSetEditText(Col, Row,
/// Value) and RapidR's OnChange, if it changed.
fn grid_store(name: &str, value: String) {
    let changed = rapidr_value::objects::with_grid_mut(name, |g| {
        let (c, r) = (g.col, g.row);
        if c < 0 || r < 0 || g.cell(c as usize, r as usize) == value {
            return None;
        }
        g.set_cell(c as usize, r as usize, value.clone());
        Some((c, r))
    })
    .flatten();
    if let Some((c, r)) = changed {
        rp_fire_event_args(name, "onsetedittext", &[v_int(c), v_int(r), v_str(&value)]);
        rp_fire_event(name, "onchange");
        grid_refresh(name);
    }
}

/// A gcsList column's drop-down: its ColumnList under the selected cell
/// (x, y, w, h); the pick is stored like an edit.
fn grid_drop_down(name: &str, items: &[String], (x, y, w, h): (i32, i32, i32, i32)) {
    if items.is_empty() {
        return;
    }
    // A menu button over the cell, not in any window, just to pop its list.
    let current = fltk::group::Group::try_current();
    fltk::group::Group::set_current(None::<&fltk::group::Group>);
    let mut button = fltk::menu::MenuButton::new(x, y, w, h, None);
    if let Some(group) = current {
        fltk::group::Group::set_current(Some(&group));
    }
    for item in items {
        button.add_choice(&menu_label(item));
    }
    let picked = button.popup().map(|_| button.value());
    fltk::menu::MenuButton::delete(button);
    if let Some(i) = picked.and_then(|i| usize::try_from(i).ok()) {
        if let Some(value) = items.get(i) {
            grid_store(name, value.clone());
        }
    }
}

fn grid_handle(name: &str, t: &mut Table, ev: Event) -> bool {
    use rapidr_value::objects::grid::GO_ALWAYS_SHOW_EDITOR;
    match ev {
        Event::Push => {
            grid_finish_edit(name, true);
            let Some((ctx, row, col, _)) = t.cursor2rowcol() else { return false };
            let Some((c, r)) = grid_cell_of(name, ctx, row, col) else { return false };
            // An ellipsis button.
            let on_button = rapidr_value::objects::with_grid(name, |g| grid_has_ellipsis(g, c as usize, r as usize)).unwrap_or(false)
                && t.find_cell(ctx, row, col).is_some_and(|(x, _, w, h)| app::event_x() >= x + w - h.min(w));
            let _ = t.take_focus();
            // The drop-down button of a gcsList column's selected cell.
            let focused = rapidr_value::objects::with_grid(name, |g| (g.col, g.row) == (c, r)).unwrap_or(false);
            if focused {
                if let Some(list) = rapidr_value::objects::with_grid(name, |g| g.list_text(c as usize, r as usize)).flatten() {
                    if let Some(cell) = t.find_cell(ctx, row, col).filter(|&(x, _, w, h)| app::event_x() >= x + w - h.min(w)) {
                        // OnListDropDown(Col, Row, S) may change the items.
                        let grid = name.to_string();
                        rp_fire_event_then(name, "onlistdropdown", &[v_int(c), v_int(r), v_str(&list)], move |a| {
                            grid_drop_down(&grid, &rapidr_value::objects::grid::list_lines(&a[2].to_string_val()), cell);
                        });
                        return true;
                    }
                }
            }
            if app::is_event_shift() && matches!(ctx, TableContext::Cell) {
                grid_extend(name, c, r);
                return true;
            }
            grid_select(name, c, r);
            if on_button {
                rp_fire_event_2(name, "onellipsisclick", v_int(c), v_int(r));
                rp_fire_event(name, "ondblclick");
                return true;
            }
            rp_fire_event(name, "onclick");
            let always = rapidr_value::objects::with_grid(name, |g| g.has_option(GO_ALWAYS_SHOW_EDITOR)).unwrap_or(false);
            if app::event_clicks() {
                rp_fire_event(name, "ondblclick");
                grid_start_edit(name, t, None);
            } else if always {
                grid_start_edit(name, t, None);
            }
            // Let the table resize columns / scroll too.
            matches!(ctx, TableContext::Cell)
        }
        // The end of a drag: resized columns / rows keep their sizes.
        Event::Released => {
            grid_sync_sizes(name, t);
            false
        }
        // Dragging over cells selects a range (goRangeSelect).
        Event::Drag => {
            let Some((TableContext::Cell, row, col, _)) = t.cursor2rowcol() else { return false };
            let Some((c, r)) = grid_cell_of(name, TableContext::Cell, row, col) else { return false };
            grid_extend(name, c, r);
            true
        }
        Event::Focus | Event::Unfocus => true,
        Event::KeyDown => {
            let key = app::event_key();
            let Some((c, r)) = rapidr_value::objects::with_grid(name, |g| (g.col, g.row)) else { return false };
            let moved = match key {
                Key::Up => Some((c, r - 1)),
                Key::Down => Some((c, r + 1)),
                Key::Left => Some((c - 1, r)),
                Key::Right => Some((c + 1, r)),
                _ => None,
            };
            if let Some((nc, nr)) = moved {
                let moved = if app::is_event_shift() { grid_extend(name, nc, nr) } else { grid_select(name, nc, nr) };
                if moved {
                    let (hr, hc) = grid_headers(name);
                    // Keep the cell in view.
                    let (top, left) = (t.row_position(), t.col_position());
                    let (_, bottom, _, right) = t.visible_cells();
                    let (tr, tc) = (nr as i32 - hr, nc as i32 - hc);
                    if tr < top || tr > bottom {
                        t.set_row_position(if tr < top { tr } else { top + (tr - bottom) });
                    }
                    if tc < left || tc > right {
                        t.set_col_position(if tc < left { tc } else { left + (tc - right) });
                    }
                }
                return true;
            }
            if key == Key::Enter || key == Key::KPEnter || key == Key::F2 {
                grid_start_edit(name, t, None);
                return true;
            }
            let typed = app::event_text();
            if !typed.is_empty() && typed.chars().all(|ch| !ch.is_control()) && !app::is_event_ctrl() && !app::is_event_command() {
                grid_start_edit(name, t, Some(typed));
                return true;
            }
            false
        }
        _ => false,
    }
}

/// The first child of a form's window: shows the form's own drawing surface
/// (`Form.TextOut`, `Form.Line`, … — rapidr_value::objects, a Bitmap) under
/// the form's controls, whose pixels of the form's color show the window
/// through. It takes no events and draws nothing until the form is drawn on.
fn form_surface_overlay(form: &str, w: i32, h: i32) {
    let mut frm = Frame::new(0, 0, w, h, None);
    frm.set_frame(FrameType::NoBox);
    let form_name = form.to_string();
    frm.draw(move |f| {
        let menu = menu_offset(&form_name);
        let (cw, ch) = (f.w(), f.h() - menu);
        if ch <= 0 {
            return;
        }
        let rgba = rapidr_value::objects::with_canvas(&form_name, cw as i64, ch as i64, |b| b.to_rgba());
        if let Some(rgba) = rgba {
            if let Ok(mut img) = RgbImage::new(&rgba, cw, ch, ColorDepth::Rgba8) {
                img.draw(f.x(), f.y() + menu, cw, ch);
            }
        }
    });
    GUI_WIDGETS.with(|gw| {
        gw.borrow_mut().insert(format!("{form}.surface"), GuiWidget::Frame(frm));
    });
}

/// Whether form `name` has its window yet.
pub fn form_window_exists(name: &str) -> bool {
    GUI_WIDGETS.with(|gw| gw.try_borrow().map_or(true, |w| matches!(w.get(&name.to_lowercase()), Some(GuiWidget::Window(_)))))
}

/// A QCANVAS's or a QFORM's surface changed: show it again.
pub fn canvas_redraw(name: &str) {
    let name = name.to_lowercase();
    if rapidr_value::objects::is_form_surface(&name) {
        redraw_widget(&format!("{name}.surface"));
    } else {
        redraw_widget(&name);
    }
}

/// Redraw a component's widget (after a property it draws changed).
pub fn gui_redraw(name: &str) {
    redraw_widget(&name.to_lowercase());
}

thread_local! {
    /// The widget the mouse button went down on (only it gets the release).
    static PRESSED: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The mouse button went down on `name`.
fn press_begin(name: &str) {
    PRESSED.with(|p| *p.borrow_mut() = Some(name.to_string()));
}

/// The mouse button came up over `name`: whether it went down on it (a
/// release alone — the system can deliver one when a window appears — is
/// no click and no mouse-up).
fn press_end(name: &str) -> bool {
    PRESSED.with(|p| p.borrow_mut().take_if(|n| n == name).is_some())
}

/// A key on a focused button: `Some(true)` when it clicks (Enter; Space
/// when released after being pressed on this button), `Some(false)` when
/// the button takes the key without clicking, `None` for other keys.
fn key_click(name: &str, ev: Event) -> Option<bool> {
    let key = app::event_key();
    match (ev, key) {
        (Event::KeyDown, Key::Enter) => Some(true),
        (Event::KeyDown, k) if k == Key::from_char(' ') => {
            press_begin(name);
            Some(false)
        }
        (Event::KeyUp, k) if k == Key::from_char(' ') => Some(press_end(name)),
        _ => None,
    }
}

fn is_toggle_button(name: &str) -> bool {
    matches!(rp_comp_type(name).as_str(), "RCOOLBTN" | "ROVALBTN")
}

/// The toggle buttons sharing `name`'s parent.
fn toggle_members(name: &str) -> Vec<rapidr_value::toggle_group::Member> {
    let parent = rp_comp_get(name, "parent").to_string_val();
    crate::object::get_children_of(&parent)
        .into_iter()
        .filter(|(_, t)| matches!(t.as_str(), "RCOOLBTN" | "ROVALBTN"))
        .map(|(n, _)| rapidr_value::toggle_group::Member {
            group: rp_comp_get(&n, "groupindex").to_i64(),
            down: rp_comp_get(&n, "down").to_bool(),
            name: n,
        })
        .collect()
}

/// Stores the new Down values and shows them.
fn toggle_apply(changes: Vec<(String, bool)>) {
    for (n, down) in changes {
        crate::object::store_prop(&n, "down", v_int(if down { -1 } else { 0 }));
        let flat = rp_comp_get(&n, "flat").to_bool();
        let cool = rp_comp_type(&n) == "RCOOLBTN";
        GUI_WIDGETS.with(|gw| {
            if let Some(GuiWidget::Button(b)) = gw.borrow_mut().get_mut(&n) {
                if cool {
                    b.set_frame(if down { FrameType::DownBox } else if flat { FrameType::FlatBox } else { FrameType::UpBox });
                }
                b.redraw();
            }
        });
    }
}

/// The user pressed a QCOOLBTN / QOVALBTN.
fn toggle_press(name: &str) {
    let allow_all_up = rp_comp_get(name, "allowallup").to_bool();
    toggle_apply(rapidr_value::toggle_group::press(name, allow_all_up, &toggle_members(name)));
}

/// The program set a button's Down: the others of its group come up, and
/// it shows the new state.
pub(crate) fn toggle_down_set(name: &str) {
    if !is_toggle_button(name) {
        return;
    }
    let name = name.to_lowercase();
    let down = rp_comp_get(&name, "down").to_bool();
    let mut changes = rapidr_value::toggle_group::set_down(&name, down, &toggle_members(&name));
    changes.push((name, down));
    toggle_apply(changes);
}

/// Trigger a widget redraw.
pub fn redraw_widget(name: &str) {
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(widget) = widgets.get_mut(name) {
            match widget {
                GuiWidget::Window(ref mut w) => { w.redraw(); }
                GuiWidget::Group(ref mut w) => { w.redraw(); }
                GuiWidget::Scroll(ref mut w) => { w.redraw(); }
                GuiWidget::Grid(ref mut w, _) => { w.redraw(); }
                GuiWidget::Frame(ref mut w) => { w.redraw(); }
                GuiWidget::ImageFrame(ref mut w) => { w.redraw(); }
                GuiWidget::Button(ref mut w) => { w.redraw(); }
                GuiWidget::Input(ref mut w) => { w.redraw(); }
                GuiWidget::CheckButton(ref mut w) => { w.redraw(); }
                GuiWidget::RadioButton(ref mut w) => { w.redraw(); }
                GuiWidget::Choice(ref mut w) => { w.redraw(); }
                GuiWidget::InputChoice(ref mut w) => { w.redraw(); }
                GuiWidget::HoldBrowser(ref mut w) => { w.redraw(); }
                GuiWidget::TextEditor(ref mut w) => { w.redraw(); }
                GuiWidget::Tabs(ref mut w) => { w.redraw(); }
                GuiWidget::MenuBar(ref mut w) => { w.redraw(); }
                GuiWidget::SysMenuBar(ref mut w) => { w.redraw(); }
                GuiWidget::Progress(ref mut w) => { w.redraw(); }
                GuiWidget::Tree(ref mut w) => { w.redraw(); }
                GuiWidget::Slider(ref mut w) => { w.redraw(); }
            }
        }
    });
}

/// Add items to a list-type widget (HoldBrowser, Choice).
pub fn gui_widget_add_items(name: &str, items_text: &str) {
    let name_lower = name.to_lowercase();
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(widget) = widgets.get_mut(&name_lower) {
            match widget {
                GuiWidget::HoldBrowser(ref mut b) => {
                    for line in items_text.lines() {
                        if !line.is_empty() {
                            b.add(line);
                        }
                    }
                }
                GuiWidget::Choice(ref mut c) => {
                    for line in items_text.lines() {
                        if !line.is_empty() {
                            c.add_choice(line);
                        }
                    }
                }
                _ => {}
            }
        }
    });
}

/// Clear items from a list-type widget.
pub fn gui_widget_clear(name: &str) {
    let name_lower = name.to_lowercase();
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(widget) = widgets.get_mut(&name_lower) {
            match widget {
                GuiWidget::HoldBrowser(ref mut b) => { b.clear(); }
                GuiWidget::Choice(ref mut c) => { c.clear(); }
                _ => {}
            }
        }
    });
}

/// Set the parent of a widget (re-parent it into another Group/Tabs).
pub fn gui_set_parent(child_name: &str, parent_name: &str) {
    let _child_lower = child_name.to_lowercase();
    let _parent_lower = parent_name.to_lowercase();
    // This is complex in FLTK — just record it in the component registry.
    // The actual re-parenting happens during build_form_widgets.
    rp_comp_set(child_name, "parent", v_str(parent_name));
}
