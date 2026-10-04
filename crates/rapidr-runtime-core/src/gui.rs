//! GUI backend using FLTK.
//!
//! This module provides actual GUI rendering when the "gui" feature is enabled.
//! Components are created as FLTK widgets and managed through a handle registry.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use fltk::{
    app,
    browser::HoldBrowser,
    button::{Button, CheckButton, RadioRoundButton},
    dialog,
    draw,
    enums::{Align, CallbackTrigger, Color, ColorDepth, Event, Font, FrameType, Key},
    frame::Frame,
    group::Group,
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

use crate::object::{rp_comp_get, rp_comp_set, rp_comp_type, rp_fire_event, rp_fire_event_1, rp_fire_event_2, rp_fire_event_args, rp_fire_event_then};
use crate::value::{v_int, v_null, v_str, Value};
use rapidr_value::objects::code;

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
    MenuBar(MenuBar),
    SysMenuBar(SysMenuBar),
    Progress(FltkProgress),
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
            GuiWidget::MenuBar(v) => v.as_base_widget(),
            GuiWidget::SysMenuBar(v) => v.as_base_widget(),
            GuiWidget::Progress(v) => v.as_base_widget(),
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
        matches!(gw.borrow().get(&parent), Some(GuiWidget::Window(_) | GuiWidget::Group(_)))
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

thread_local! {
    static GUI_WIDGETS: RefCell<HashMap<String, GuiWidget>> = RefCell::new(HashMap::new());
    static GUI_APP: RefCell<Option<app::App>> = RefCell::new(None);
    static GUI_TEXT_BUFFERS: RefCell<HashMap<String, TextBuffer>> = RefCell::new(HashMap::new());
    static GUI_STYLE_BUFFERS: RefCell<HashMap<String, TextBuffer>> = RefCell::new(HashMap::new());
    /// Maps tab control names to their child group names (tab_name -> group_widget_key)
    /// `$THEME` (see [`look_for`]); "" for the platform's own look
    static THEME_OVERRIDE: RefCell<String> = RefCell::new(String::new());
    /// Active timer names (component names that are RTimer)
    static ACTIVE_TIMERS: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Set the application theme (`$THEME`, see [`look_for`]). Call before any
/// window is shown.
pub fn set_theme(theme: &str) {
    THEME_OVERRIDE.with(|t| {
        *t.borrow_mut() = theme.to_lowercase();
    });
}

/// The desktop look for `$THEME name` (or the platform's own, `""` /
/// `"system"` / `"light"`) on `os` (`std::env::consts::OS`): a key of
/// [`apply_look`]. By default a program looks like its platform, in light
/// colors (RapidQ programs are made for them): classic Aqua on macOS, Metro
/// on Windows, Gleam elsewhere. Any fltk-theme theme or scheme, and FLTK's
/// own schemes, can be asked for by name (`$THEME`, `RAPIDR_THEME`).
fn look_for(name: &str, os: &str) -> &'static str {
    const NAMED: &[&str] = &[
        // fltk-theme widget themes (frames and colors)
        "classic", "aero", "metro", "aquaclassic", "greybird", "blue", "dark", "highcontrast",
        // fltk-theme widget schemes (frames; light colors)
        "aqua", "fluent", "clean", "gleam", "svg", "sweet", "fleet1", "fleet2",
        // FLTK's own schemes
        "base", "gtk", "plastic", "oxy",
    ];
    let name = name.trim().to_ascii_lowercase();
    if let Some(n) = NAMED.iter().find(|n| **n == name) {
        return n;
    }
    match name.as_str() {
        "windows" | "win10" | "win11" => "fluent",
        // (fltk-theme 0.7.9's crystal scheme panics: its neighbour instead)
        "crystal" => "clean",
        "mac" | "macos" => "aqua",
        "linux" | "xfce" => "greybird",
        "windows7" | "win7" => "aero",
        "windows8" | "win8" => "metro",
        "windows95" | "win95" | "win2000" => "classic",
        // The platform's own. (fltk-theme's modern Aqua and Fluent schemes
        // don't show RapidR's default buttons yet: the classic themes.)
        _ => match os {
            "macos" => "aquaclassic",
            "windows" => "metro",
            _ => "gleam",
        },
    }
}

/// Applies a look from [`look_for`].
fn apply_look(look: &str) {
    use fltk_theme::{SchemeType, WidgetScheme};
    let theme = |t: ThemeType| WidgetTheme::new(t).apply();
    let scheme = |s: SchemeType| WidgetScheme::new(s).apply();
    match look {
        "classic" => theme(ThemeType::Classic),
        "aero" => theme(ThemeType::Aero),
        "metro" => theme(ThemeType::Metro),
        "aquaclassic" => theme(ThemeType::AquaClassic),
        "greybird" => theme(ThemeType::Greybird),
        "blue" => theme(ThemeType::Blue),
        "dark" => theme(ThemeType::Dark),
        "highcontrast" => theme(ThemeType::HighContrast),
        "base" => app::set_scheme(app::Scheme::Base),
        "gtk" => app::set_scheme(app::Scheme::Gtk),
        "plastic" => app::set_scheme(app::Scheme::Plastic),
        "oxy" => app::set_scheme(app::Scheme::Oxy),
        _ => {
            // A scheme: its frames, in the platform's light colors.
            match look {
                "aqua" => scheme(SchemeType::Aqua),
                "clean" => scheme(SchemeType::Clean),
                "gleam" => scheme(SchemeType::Gleam),
                "svg" => scheme(SchemeType::SvgBased),
                "sweet" => scheme(SchemeType::Sweet),
                "fleet1" => scheme(SchemeType::Fleet1),
                "fleet2" => scheme(SchemeType::Fleet2),
                _ => scheme(SchemeType::Fluent),
            }
            let (window, selection) = if look == "aqua" { ((236, 236, 236), (0, 122, 255)) } else { ((240, 240, 240), (0, 120, 215)) };
            app::background(window.0, window.1, window.2);
            app::background2(255, 255, 255);
            app::foreground(0, 0, 0);
            app::set_selection_color(selection.0, selection.1, selection.2);
        }
    }
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

            // `$THEME`, else the `RAPIDR_THEME` environment variable.
            let theme_name = THEME_OVERRIDE.with(|t| t.borrow().clone());
            let theme_name = if theme_name.is_empty() { std::env::var("RAPIDR_THEME").unwrap_or_default() } else { theme_name };
            apply_look(look_for(&theme_name, std::env::consts::OS));

            *app_ref = Some(app);
        }
    });
    install_input_dispatch();
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
    IGNORE_USER.with(|i| i.set(true));
}

thread_local! {
    static IGNORE_USER: Cell<bool> = const { Cell::new(false) };
    /// The component a mouse button went down on: it gets the drag and the
    /// button's release (rapidr_value::input).
    static MOUSE_CAPTURE: RefCell<Option<String>> = const { RefCell::new(None) };
    static LAST_MOVE: RefCell<Option<(Option<String>, i32, i32)>> = const { RefCell::new(None) };
}

/// Every FLTK event passes here first: the program's keyboard and mouse
/// events (OnKeyDown / OnKeyPress / OnKeyUp to the focused component and
/// its form, OnMouseDown / OnMouseMove / OnMouseUp to the component under
/// the mouse, with RapidQ's arguments: rapidr_value::input); under a GUI
/// test the user's input is dropped ([`ignore_user_input`]).
fn install_input_dispatch() {
    fn dispatch(ev: Event, win: app::WindowPtr) -> bool {
        let user = matches!(ev, Event::Push | Event::Released | Event::Drag | Event::KeyDown | Event::KeyUp | Event::Shortcut | Event::MouseWheel);
        if user && IGNORE_USER.with(Cell::get) {
            return false;
        }
        if matches!(ev, Event::KeyDown | Event::KeyUp) {
            let chain = component_chain(app::focus().map(|w| w.as_base_widget()));
            let chain = if chain.is_empty() { window_component(win).into_iter().collect() } else { chain };
            let vk = fltk_vk(app::event_key().bits());
            key_events(&chain, ev == Event::KeyDown, vk, mouse_shift(), &app::event_text());
        }
        // A form's / scroll box's scroll bars take their clicks first.
        if let Some(handled) = scroll_bars_event(ev, win) {
            return handled;
        }
        // (the input lane's: then a status bar's size grip, as Windows'
        // sizing border — no OnMouseDown)
        if let Some(handled) = size_grip_event(ev, win) {
            return handled;
        }
        // SAFETY: `win` is the window FLTK passed in for this event.
        let handled = unsafe { app::handle_raw(ev, win) };
        // The wheel no component used scrolls the form / scroll box under
        // the mouse.
        if ev == Event::MouseWheel && !handled && scroll_wheel(win) {
            return true;
        }
        let kind = match ev {
            Event::Push => Some(rapidr_value::input::Mouse::Down),
            Event::Released => Some(rapidr_value::input::Mouse::Up),
            Event::Move | Event::Drag => Some(rapidr_value::input::Mouse::Move),
            _ => None,
        };
        if let Some(kind) = kind {
            let target = match ev {
                Event::Drag => MOUSE_CAPTURE.with(|c| c.borrow().clone()),
                Event::Released => MOUSE_CAPTURE.with(|c| c.borrow_mut().take()),
                _ => component_under_mouse(win),
            };
            if ev == Event::Push {
                MOUSE_CAPTURE.with(|c| *c.borrow_mut() = target.clone());
            }
            // (FLTK passes a move on twice: the second isn't a new one)
            let spot = (target.clone(), app::event_x(), app::event_y());
            let repeated = kind == rapidr_value::input::Mouse::Move && LAST_MOVE.with(|m| m.replace(Some(spot.clone())) == Some(spot));
            if ev == Event::Move && !repeated {
                apply_cursor(target.as_deref(), win);
            }
            if let Some(name) = target.filter(|_| !repeated) {
                let (x, y) = widget_origin(&name, win);
                // (the input lane's: a double click's second press, and a
                // release over what was pressed — OnClick / OnDblClick)
                let inside = ev != Event::Released || component_under_mouse(win).as_deref() == Some(name.as_str());
                mouse_event(&name, kind, button_of(mouse_button()), (app::event_x() - x, app::event_y() - y), mouse_shift(), (fltk_double(), inside));
            }
        }
        handled
    }
    // SAFETY: `dispatch` only forwards the window pointer it is given.
    unsafe { app::event_dispatch(dispatch) };
}

thread_local! {
    /// The pointer shown last (rapidr_value::input::Cursor).
    static SHOWN_CURSOR: Cell<rapidr_value::input::Cursor> = const { Cell::new(rapidr_value::input::Cursor::Default) };
}

/// The mouse pointer over component `name`: Screen.Cursor, else its own
/// Cursor (crDefault: the arrow). Only a change is shown, so widgets that
/// set their own (a splitter's) keep theirs.
fn apply_cursor(name: Option<&str>, win: app::WindowPtr) {
    use rapidr_value::input::Cursor as C;
    let screen = crate::globals::screen_cursor();
    let code = if screen != 0 { screen } else { name.map_or(0, |n| rp_comp_get(n, "cursor").to_i64()) };
    // (on a QHEADER section's edge: the resize cursor)
    let grip = name.filter(|n| rapidr_value::objects::is_header(n)).is_some_and(|n| {
        let x = app::event_x() - widget_origin(n, win).0;
        rapidr_value::objects::with_header(n, |h| h.on_grip(x as i64)).unwrap_or(false)
    });
    let grip = grip
        || name.filter(|n| rapidr_value::objects::is_listview(n)).is_some_and(|n| {
            let (ox, oy) = widget_origin(n, win);
            rapidr_value::objects::with_listview(n, |lv| lv.on_grip((app::event_x() - ox) as i64, (app::event_y() - oy) as i64)).unwrap_or(false)
        });
    let code = if grip && code == 0 { -9 } else { code };
    let cursor = C::of(code);
    if SHOWN_CURSOR.with(|s| s.replace(cursor)) == cursor {
        return;
    }
    let fl = match cursor {
        C::Default | C::Arrow | C::NoDrop => fltk::enums::Cursor::Default,
        C::None => fltk::enums::Cursor::None,
        C::Cross => fltk::enums::Cursor::Cross,
        C::IBeam => fltk::enums::Cursor::Insert,
        C::Move => fltk::enums::Cursor::Move,
        C::SizeNESW => fltk::enums::Cursor::NESW,
        C::SizeNS => fltk::enums::Cursor::NS,
        C::SizeNWSE => fltk::enums::Cursor::NWSE,
        C::SizeWE => fltk::enums::Cursor::WE,
        C::UpArrow => fltk::enums::Cursor::N,
        C::Wait | C::Progress => fltk::enums::Cursor::Wait,
        C::Help => fltk::enums::Cursor::Help,
        C::Hand => fltk::enums::Cursor::Hand,
    };
    let Some(form) = window_component(win) else { return };
    if let Some(GuiWidget::Window(mut w)) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(&form).cloned())) {
        w.set_cursor(fl);
    }
}

fn button_of(b: i64) -> rapidr_value::input::Button {
    match b {
        1 => rapidr_value::input::Button::Right,
        2 => rapidr_value::input::Button::Middle,
        _ => rapidr_value::input::Button::Left,
    }
}

/// Fires `name`'s mouse event (QIMAGE fires its own: `picture_mouse`);
/// `double`: a double click's second press (or its release), `inside`: a
/// release over the component pressed.
fn mouse_event(name: &str, kind: rapidr_value::input::Mouse, button: rapidr_value::input::Button, (x, y): (i32, i32), shift: i64, (double, inside): (bool, bool)) {
    if kind == rapidr_value::input::Mouse::Down && button == rapidr_value::input::Button::Right && auto_popup(name) {
        return;
    }
    if rp_comp_type(name).eq_ignore_ascii_case("RIMAGE") {
        return;
    }
    if rapidr_value::objects::is_header(name) && button == rapidr_value::input::Button::Left {
        header_mouse(name, kind, x as i64);
    }
    if rapidr_value::objects::is_listview(name) && (button == rapidr_value::input::Button::Left || kind == rapidr_value::input::Mouse::Move) {
        listview_mouse(name, kind, x as i64, y as i64, shift, double);
    }
    vcl_clicks(name, kind, button, (double, inside), || rp_fire_event_args(name, kind.event(), &kind.args(button, x as i64, y as i64, shift)));
}

/// (the input lane's) Windows' double click: the second press of a pair
/// (FLTK counts on — a third press is a single one again).
fn fltk_double() -> bool {
    app::event_clicks_num() % 2 == 1
}

/// (the input lane's) A mouse event `fire` of a VCL control with
/// csClickEvents and csDoubleClicks — QFORM, QPANEL, QLABEL, QGROUPBOX,
/// QSCROLLBOX, QIMAGE: a double click's second press is OnDblClick before
/// its OnMouseDown (WM_LBUTTONDBLCLK), and a single click let go over it
/// OnClick before its OnMouseUp. QCANVAS (no OnDblClick in RapidQ) clicks
/// at every release. As the kernel host does (`components::canvas`).
fn vcl_clicks(name: &str, kind: rapidr_value::input::Mouse, button: rapidr_value::input::Button, (double, inside): (bool, bool), fire: impl FnOnce()) {
    use rapidr_value::input::{Button, Mouse};
    let t = rp_comp_type(name).to_ascii_uppercase();
    let doubles = matches!(t.as_str(), "RFORM" | "RPANEL" | "RLABEL" | "RGROUPBOX" | "RSCROLLBOX" | "RIMAGE");
    if (doubles || t == "RCANVAS") && button == Button::Left {
        match kind {
            Mouse::Down if doubles && double => rp_fire_event(name, "ondblclick"),
            Mouse::Up if inside && !(doubles && double) => rp_fire_event(name, "onclick"),
            _ => {}
        }
    }
    fire();
}

/// Key events for `chain` (the focused component first, its form last):
/// OnKeyDown / OnKeyUp (Key, Shift) and, for a key that types, OnKeyPress
/// (Key), to the component and then its form.
fn key_events(chain: &[String], down: bool, vk: i64, shift: i64, text: &str) {
    // (a key pressed in the program's windows is INKEY$'s too, as in the
    // web page; a console program's come from its terminal)
    if down {
        if let Some(k) = rapidr_value::console::inkey_of(vk, text) {
            rapidr_value::console::push_key(k);
        }
    }
    // (a list view moves its selection first, as Windows' does)
    if let Some(first) = chain.first().filter(|n| down && rapidr_value::objects::is_listview(n)) {
        listview_key(first, vk, shift);
    }
    let targets = rapidr_value::input::key_targets(chain, |form| rp_comp_get(form, "keypreview").to_bool());
    let press = if down { rapidr_value::input::press_code(vk, text) } else { None };
    // (all OnKeyDowns, then the OnKeyPresses, as Windows' WM_KEYDOWN then WM_CHAR)
    for name in &targets {
        rp_fire_event_2(name, if down { "onkeydown" } else { "onkeyup" }, v_int(vk), v_int(shift));
    }
    if let Some(key) = press {
        for name in &targets {
            rp_fire_event_1(name, "onkeypress", v_int(key));
        }
    }
}

/// Each widget's component, by widget address (none while the widgets are
/// being changed: FLTK dispatches events from inside `show`, `hide`, …).
fn components_by_widget() -> HashMap<usize, String> {
    GUI_WIDGETS.with(|gw| gw.try_borrow().map(|gw| gw.iter().map(|(n, w)| (w.base().as_widget_ptr() as usize, n.clone())).collect()).unwrap_or_default())
}

/// The components from widget `w` (or its nearest parent that is one) up to
/// its form.
fn component_chain(w: Option<fltk::widget::Widget>) -> Vec<String> {
    let map = components_by_widget();
    let mut chain = Vec::new();
    let mut w = w;
    while let Some(widget) = w {
        if let Some(name) = map.get(&(widget.as_widget_ptr() as usize)) {
            chain.push(name.clone());
        }
        w = widget.parent().map(|p| p.as_base_widget());
    }
    chain
}

fn window_component(win: app::WindowPtr) -> Option<String> {
    components_by_widget().remove(&(win as usize))
}

/// The component under the mouse in window `win`: the smallest shown
/// widget there (the innermost), else the window's form.
fn component_under_mouse(win: app::WindowPtr) -> Option<String> {
    let (ex, ey) = (app::event_x(), app::event_y());
    GUI_WIDGETS.with(|gw| {
        let gw = gw.try_borrow().ok()?;
        let mut best: Option<(i64, &String)> = None;
        for (name, widget) in gw.iter() {
            let w = widget.base();
            if w.as_widget_ptr() as usize == win as usize || !w.visible_r() {
                continue;
            }
            let in_win = w.window().is_some_and(|ww| ww.as_widget_ptr() as usize == win as usize);
            if !in_win || ex < w.x() || ey < w.y() || ex >= w.x() + w.w() || ey >= w.y() + w.h() {
                continue;
            }
            let area = i64::from(w.w()) * i64::from(w.h());
            if best.is_none_or(|(a, _)| area < a) {
                best = Some((area, name));
            }
        }
        best.map(|(_, n)| n.clone())
    })
    .or_else(|| window_component(win))
}

/// Where `name`'s widget starts in window `win`'s coordinates (a window's
/// own events are in its coordinates already).
fn widget_origin(name: &str, win: app::WindowPtr) -> (i32, i32) {
    GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(name).map(GuiWidget::base))).filter(|w| w.as_widget_ptr() as usize != win as usize).map_or((0, 0), |w| (w.x(), w.y()))
}

/// The Windows virtual-key code of an FLTK key (rapidr_value::input).
fn fltk_vk(key: i32) -> i64 {
    match key {
        0xff08 => 8,
        0xff09 => 9,
        0xff0d | 0xff8d => 13,
        0xff13 => 19,
        0xff14 => 145,
        0xff1b => 27,
        0xff50 => 36,
        0xff51 => 37,
        0xff52 => 38,
        0xff53 => 39,
        0xff54 => 40,
        0xff55 => 33,
        0xff56 => 34,
        0xff57 => 35,
        0xff61 => 44,
        0xff63 => 45,
        0xff67 => 93,
        0xff7f => 144,
        0xffe1 | 0xffe2 => 16,
        0xffe3 | 0xffe4 => 17,
        0xffe5 => 20,
        0xffe7 | 0xffe8 => 91,
        0xffe9 | 0xffea => 18,
        0xffff => 46,
        // The keypad: digits 96–105, * + - . / 106, 107, 109, 110, 111.
        k if (0xff80 + 0x30..=0xff80 + 0x39).contains(&k) => 96 + (k - 0xff80 - 0x30) as i64,
        k if (0xff80..0xffbd).contains(&k) => match (k - 0xff80) as u8 {
            b'*' => 106,
            b'+' => 107,
            b'-' => 109,
            b'.' => 110,
            b'/' => 111,
            _ => 0,
        },
        k if (0xffbe..=0xffd5).contains(&k) => 111 + (k - 0xffbd) as i64,
        k if (0..256).contains(&k) => char::from_u32(k as u32).and_then(rapidr_value::input::vk_of_char).unwrap_or(0),
        _ => 0,
    }
}

fn install_capture_hook() {
    use crate::ui::testhooks::Capture;
    let Some(Capture { prefix, delay, events, resize, split }) = Capture::from_env() else { return };
    ignore_user_input();
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
        // `RAPIDR_TEST_EVENTS` (ui::testhooks), one per turn of the event
        // loop, as real clicks come (the bytecode VM runs a handler once
        // the callback that fired it returns).
        fire_test_events(events.clone(), prefix.clone());
    });
}

/// Fires the first of `queue`, then the rest a turn later; then (once the
/// last handlers have run) redraws and captures the windows.
fn fire_test_events(mut queue: Vec<crate::ui::testhooks::TestEvent>, prefix: String) {
    use crate::ui::testhooks::Action;
    if queue.is_empty() {
        app::redraw();
        app::add_timeout3(0.3, move |_| capture_windows(&prefix));
        return;
    }
    let e = queue.remove(0);
    let comp = e.comp.as_str();
    let comp_lower = e.comp_lower();
    // A toggle button's click goes through its group, as the widget's does.
    if matches!(&e.action, Action::Fire(ev) if ev == "onclick") && is_toggle_button(comp) {
        toggle_press(&comp_lower);
    }
    let widget = || GUI_WIDGETS.with(|gw| gw.borrow().get(&comp_lower).map(GuiWidget::base));
    // (as a click: the pick, then OnClick; OnClick before a toggle)
    match e.action {
        // (the input lane's: a click on the node already selected is the
        // widget's callback's — an edit after a pause)
        Action::Node(i) => {
            TREE_CLICKS.with(|c| c.set(c.get() + 1));
            if rapidr_value::objects::with_tree(&comp_lower, |m| m.item_index) == Some(i) {
                tree_reselected(&comp_lower, i as usize, false);
            } else {
                tree_user_select(&comp_lower, i as usize);
                rp_fire_event(&comp_lower, "onclick");
            }
        }
        // (a list view: F2 on it)
        Action::Edit if rapidr_value::objects::is_listview(&comp_lower) => listview_key(&comp_lower, 113, 0),
        Action::Enter | Action::Escape if rapidr_value::objects::is_listview(&comp_lower) => {
            LISTVIEW_EDITORS.with(|ed| {
                if let Some((editor, _)) = ed.borrow_mut().get_mut(&comp_lower) {
                    editor.set_value("Renamed");
                }
            });
            listview_end_edit(&comp_lower, e.action == Action::Enter);
        }
        // F2: the selected node edited; `__enter` types "Renamed" and
        // Enter in its editor, `__escape` drops the edit.
        Action::Edit => {
            if let Some(i) = rapidr_value::objects::with_tree(&comp_lower, |m| usize::try_from(m.item_index).ok()).flatten() {
                tree_begin_edit(&comp_lower, i);
            }
        }
        Action::Enter | Action::Escape => {
            TREE_EDITORS.with(|ed| {
                if let Some((editor, _)) = ed.borrow_mut().get_mut(&comp_lower) {
                    editor.set_value("Renamed");
                }
            });
            tree_end_edit(&comp_lower, e.action == Action::Enter);
        }
        Action::Toggle(i) => {
            let open = !rapidr_value::objects::with_tree(&comp_lower, |m| m.nodes.get(i as usize).is_some_and(|n| n.expanded)).unwrap_or(true);
            rp_fire_event(&comp_lower, "onclick");
            tree_user_toggle(&comp_lower, i as usize, open);
        }
        Action::Item(i) => {
            if rapidr_value::objects::with_list(&comp_lower, |l| l.combo).unwrap_or(false) {
                owner_combo_pick(&comp_lower, i);
            } else {
                owner_list_select(&comp_lower, i);
                list_refresh(&comp_lower);
                rp_fire_event(&comp_lower, "onclick");
            }
        }
        Action::Key(vk) => {
            let chain = component_chain(widget());
            let text = rapidr_value::input::text_of_vk(vk);
            key_events(&chain, true, vk, 0, &text);
            // (a track bar moves as its widget's key handler moves it)
            trackbar_input(&comp_lower, |t, _, _| matches!(vk, 33..=40) && t.key(vk));
            tab_control_input(&comp_lower, |t, w, h, font| matches!(vk, 37..=40) && t.key(vk, w, h, font));
            key_events(&chain, false, vk, 0, "");
        }
        // (the design surface's own handler: the shared model's events)
        Action::Mouse(kind, x, y) if rapidr_value::objects::is_design(&comp_lower) => design_test_mouse(&comp_lower, kind, x, y, false),
        Action::Mouse(kind, x, y) => {
            hook_mouse(&comp_lower, kind, x, y, false);
            let (fx, fy) = (x as f64, y as f64);
            match kind {
                rapidr_value::input::Mouse::Down => {
                    trackbar_input(&comp_lower, |t, w, h| t.mouse_down(fx, fy, w, h).1);
                    tab_control_input(&comp_lower, |t, w, h, font| t.mouse_down(x, y, w, h, font).is_some_and(|r| r.0));
                }
                rapidr_value::input::Mouse::Move => trackbar_input(&comp_lower, |t, w, h| t.drag(fx, fy, w, h)),
                _ => {}
            }
        }
        // (the input lane's) `pn.__dblclick_3_4`: press, release, then the
        // double click's press and release.
        Action::DblClick(x, y) => {
            use rapidr_value::input::Mouse;
            for (kind, double) in [(Mouse::Down, false), (Mouse::Up, false), (Mouse::Down, true), (Mouse::Up, true)] {
                if rapidr_value::objects::is_design(&comp_lower) {
                    design_test_mouse(&comp_lower, kind, x, y, double);
                } else {
                    hook_mouse(&comp_lower, kind, x, y, double);
                }
            }
        }
        Action::Ignored => {}
        // `form.__close`: the window's close button.
        Action::Close => gui_close(comp),
        // `grid.__cell_2_1`: the user clicks cell (2, 1).
        Action::Cell(c, r) => {
            grid_select(&comp_lower, c, r);
        }
        Action::Fire(ref event) => crate::object::rp_fire_event(comp, event),
    }
    app::add_timeout3(0.05, move |_| fire_test_events(queue.clone(), prefix.clone()));
}

/// `ds.__mousedown_x_y` … on a design surface: what its handler does with
/// the mouse (design_surface_event). As on the kernel, a scripted press is a
/// single click; `ds.__dblclick_x_y`'s second press (`double`) is a double
/// click.
fn design_test_mouse(ds: &str, kind: rapidr_value::input::Mouse, x: i64, y: i64, double: bool) {
    use rapidr_value::input::Mouse;
    let heard = match kind {
        Mouse::Down => rapidr_value::objects::with_design_mut(ds, |d| d.mouse_down(x, y, double)),
        Mouse::Move => rapidr_value::objects::with_design_mut(ds, |d| d.mouse_drag(x, y)),
        Mouse::Up => rapidr_value::objects::with_design_mut(ds, |d| {
            d.mouse_up();
            None
        }),
    };
    redraw_widget(ds);
    if let Some(e) = heard.flatten() {
        fire_design_event(ds, &e);
    }
}

/// A test's mouse event at (x, y) in `comp` (`double`: a double click's
/// second press or its release), as the real input's dispatch fires it: a
/// scroll box's / form's bars take it first (no OnMouseDown for them); a
/// QIMAGE as its widget's handler.
fn hook_mouse(comp: &str, kind: rapidr_value::input::Mouse, x: i64, y: i64, double: bool) {
    use rapidr_value::input::Button;
    if scroll_bars_hook(comp, kind, x, y) || grip_hook(comp, kind, x, y) {
        return;
    }
    // (a list view takes the focus at a press, as its widget's handler)
    if kind == rapidr_value::input::Mouse::Down && rapidr_value::objects::is_listview(comp) {
        if let Some(mut w) = GUI_WIDGETS.with(|gw| gw.borrow().get(comp).map(GuiWidget::base)) {
            let _ = w.take_focus();
        }
    }
    if rp_comp_type(comp).eq_ignore_ascii_case("RIMAGE") {
        vcl_clicks(comp, kind, Button::Left, (double, true), || rp_fire_event_args(comp, kind.event(), &kind.args(Button::Left, x, y, 0)));
    } else {
        mouse_event(comp, kind, Button::Left, (x as i32, y as i32), 0, (double, true));
    }
}

fn capture_windows(prefix: &str) {
    // `RAPIDR_TEST_DUMP=b1.caption,b2.caption`: print these properties.
    crate::ui::testhooks::print_dump(widget_shown, |comp, prop| rp_comp_get(comp, prop).to_string_val());
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

/// QCOLORDIALOG on FLTK: its colour chooser (hue / saturation wheel,
/// value, RGB boxes) titled `title`, starting at `bgr` (&HBBGGRR), with OK
/// and Cancel; the colour chosen, `None` for Cancel or the close box.
fn fltk_color_dialog(title: &str, bgr: i64) -> Option<i64> {
    use fltk::group::ColorChooser;
    use std::rc::Rc;
    ensure_app();
    let mut win = Window::default().with_size(240, 230).with_label(title);
    win.make_modal(true);
    let mut chooser = ColorChooser::new(8, 8, 224, 180, None);
    let _ = chooser.set_rgb((bgr & 0xFF) as u8, (bgr >> 8 & 0xFF) as u8, (bgr >> 16 & 0xFF) as u8);
    let chosen = Rc::new(std::cell::Cell::new(false));
    let mut ok = fltk::button::ReturnButton::new(82, 198, 72, 23, "OK");
    let mut cancel = Button::new(160, 198, 72, 23, "Cancel");
    let (c, mut w) = (chosen.clone(), win.clone());
    ok.set_callback(move |_| {
        c.set(true);
        w.hide();
    });
    let mut w = win.clone();
    cancel.set_callback(move |_| w.hide());
    win.end();
    win.show();
    while win.shown() {
        if !app::wait() {
            break;
        }
    }
    let (r, g, b) = chooser.rgb_color();
    chosen.get().then(|| i64::from(b) << 16 | i64::from(g) << 8 | i64::from(r))
}

/// QFONTDIALOG on FLTK, laid out as the kernel's and the web's
/// (`rapidr_value::font_dialog::layout`): the faces (the shared ones —
/// FLTK draws every face with its own three families), styles and sizes,
/// with fdEffects Strikeout, Underline and the colour, the sample; OK,
/// Cancel, Apply (fdApplyButton: the font so far stored, then `owner`'s
/// OnApply), a disabled Help (fdShowHelp). The font chosen, `None` for
/// Cancel or the close box.
fn fltk_font_dialog(owner: &str, title: &str, req: rapidr_value::font_dialog::Request) -> Option<rapidr_value::objects::font::Font> {
    use rapidr_value::font_dialog::{self as fd, layout as fl};
    use std::rc::Rc;
    ensure_app();
    let names = req.names(&crate::ui::choose_dialogs::font_names(Vec::new));
    let (sizes, colors, effects) = (req.sizes(), req.colors(), req.has(fd::FD_EFFECTS));
    let at = |(x, y, w, h): fl::Rect| (x as i32, y as i32, w as i32, h as i32);
    let (w, h) = fl::size(effects);
    let mut win = Window::default().with_size(w as i32, h as i32).with_label(title);
    win.make_modal(true);
    let font = Rc::new(RefCell::new(req.font.clone()));
    let label = |rect: fl::Rect, text: &str| {
        let (x, y, w, h) = at(rect);
        let mut f = Frame::new(x, y, w, h, None);
        f.set_label(text);
        f.set_align(Align::Left | Align::Inside);
        f.set_label_size(13);
    };
    let group = |rect: fl::Rect, text: &str| {
        let (x, y, w, h) = at(rect);
        let mut f = Frame::new(x, y + 6, w, h - 6, None);
        f.set_frame(FrameType::EngravedFrame);
        f.set_label(text);
        f.set_label_size(13);
        f.set_align(Align::TopLeft);
    };
    let browser = |rect: fl::Rect, items: &[String], selected: Option<usize>| {
        let (x, y, w, h) = at(rect);
        let mut b = HoldBrowser::new(x, y, w, h, None);
        for i in items {
            b.add(&i.replace('@', "@@"));
        }
        if let Some(s) = selected {
            b.select(s as i32 + 1);
        }
        b
    };
    let f = req.font.clone();
    label(fl::FONT_LABEL, "&Font:");
    label(fl::STYLE_LABEL, "Font st&yle:");
    label(fl::SIZE_LABEL, "&Size:");
    let unselected = |o: i64| req.has(o);
    let mut face = browser(fl::FONT_LIST, &names, names.iter().position(|n| n.eq_ignore_ascii_case(&f.name)).filter(|_| !unselected(fd::FD_NO_FACE_SEL)));
    let styles: Vec<String> = fd::STYLES.iter().map(|s| s.to_string()).collect();
    let style_index = usize::from(f.styles & 2 != 0) + 2 * usize::from(f.styles & 1 != 0);
    let mut style = browser(fl::STYLE_LIST, &styles, (!unselected(fd::FD_NO_STYLE_SEL)).then_some(style_index));
    let size_items: Vec<String> = sizes.iter().map(i64::to_string).collect();
    let mut size = browser(fl::SIZE_LIST, &size_items, sizes.iter().position(|s| *s == f.size).filter(|_| !unselected(fd::FD_NO_SIZE_SEL)));
    let (mut strike, mut under, mut color) = (CheckButton::default(), CheckButton::default(), Choice::default());
    if effects {
        group(fl::EFFECTS, "Effects");
        let check = |rect: fl::Rect, text: &str, on: bool| {
            let (x, y, w, h) = at(rect);
            let mut c = CheckButton::new(x, y, w, h, None);
            c.set_label(text);
            c.set_label_size(13);
            c.set_checked(on);
            c
        };
        strike = check(fl::STRIKEOUT, "Stri&keout", f.styles & 8 != 0);
        under = check(fl::UNDERLINE, "&Underline", f.styles & 4 != 0);
        label(fl::COLOR_LABEL, "&Color:");
        let (x, y, w, h) = at(fl::COLOR_LIST);
        color = Choice::new(x, y, w, h, None);
        for (n, _) in &colors {
            color.add_choice(n);
        }
        color.set_value(colors.iter().position(|(_, c)| *c == f.color).map_or(-1, |i| i as i32));
    }
    let (sample_group, sample_rect) = fl::sample(effects);
    group(sample_group, "Sample");
    let (x, y, w, h) = at(sample_rect);
    let mut sample = Frame::new(x, y, w, h, None);
    {
        let font = font.clone();
        sample.draw(move |s| {
            let f = font.borrow();
            let c = Color::from_rgb((f.color & 0xFF) as u8, (f.color >> 8 & 0xFF) as u8, (f.color >> 16 & 0xFF) as u8);
            draw::set_font(fltk_face(&f), font_pixels(&f));
            draw::set_draw_color(c);
            let tw = draw::width(fd::SAMPLE) as i32;
            let (left, base) = (s.x() + (s.w() - tw) / 2, s.y() + (s.h() + draw::height()) / 2 - draw::descent());
            draw::draw_text(fd::SAMPLE, left, base);
            let thick = (font_pixels(&f) / 14).max(1);
            if f.styles & 4 != 0 {
                draw::draw_rect_fill(left, base + 1, tw, thick, c);
            }
            if f.styles & 8 != 0 {
                draw::draw_rect_fill(left, base - (f64::from(font_pixels(&f)) * 0.3).round() as i32, tw, thick, c);
            }
        });
    }
    // (any change: the choice read again and the sample drawn)
    let update: Rc<dyn Fn()> = {
        let (face, style, size, strike, under, color, font, sample) = (face.clone(), style.clone(), size.clone(), strike.clone(), under.clone(), color.clone(), font.clone(), sample.clone());
        let (names, sizes, colors) = (names.clone(), sizes.clone(), colors.clone());
        Rc::new(move || {
            let mut f = font.borrow_mut();
            if let Some(n) = usize::try_from(face.value() - 1).ok().and_then(|i| names.get(i)) {
                f.name = n.clone();
            }
            if let Ok(st) = usize::try_from(style.value() - 1) {
                f.styles = (f.styles & !3) | u8::from(st & 2 != 0) | u8::from(st & 1 != 0) << 1;
            }
            if let Some(s) = usize::try_from(size.value() - 1).ok().and_then(|i| sizes.get(i)) {
                f.size = *s;
            }
            if effects {
                f.styles = (f.styles & 3) | u8::from(under.is_checked()) << 2 | u8::from(strike.is_checked()) << 3;
                if let Some((_, c)) = usize::try_from(color.value()).ok().and_then(|i| colors.get(i)) {
                    f.color = *c;
                }
            }
            drop(f);
            sample.clone().redraw();
        })
    };
    macro_rules! on_change {
        ($($w:ident),*) => {$({
            let update = update.clone();
            $w.set_callback(move |_| update());
        })*};
    }
    on_change!(face, style, size);
    if effects {
        on_change!(strike, under, color);
    }
    let button = |rect: fl::Rect, text: &str| {
        let (x, y, w, h) = at(rect);
        let mut b = Button::new(x, y, w, h, None);
        b.set_label(text);
        b.set_label_size(13);
        b
    };
    let chosen = Rc::new(std::cell::Cell::new(false));
    let (ox, oy, ow, oh) = at(fl::OK);
    let mut ok = fltk::button::ReturnButton::new(ox, oy, ow, oh, "OK");
    let (c, mut w) = (chosen.clone(), win.clone());
    ok.set_callback(move |_| {
        c.set(true);
        w.hide();
    });
    let mut cancel = button(fl::CANCEL, "Cancel");
    let mut w = win.clone();
    cancel.set_callback(move |_| w.hide());
    let apply = req.has(fd::FD_APPLY_BUTTON);
    if apply {
        let mut b = button(fl::APPLY, "&Apply");
        let (owner, font, update) = (owner.to_string(), font.clone(), update.clone());
        b.set_callback(move |_| {
            update();
            let f = font.borrow().clone();
            crate::ui::choose_dialogs::font_applied(&owner, &f);
        });
    }
    if req.has(fd::FD_SHOW_HELP) {
        button(if apply { fl::HELP } else { fl::APPLY }, "&Help").deactivate();
    }
    win.end();
    win.show();
    while win.shown() {
        if !app::wait() {
            break;
        }
    }
    let f = font.borrow().clone();
    chosen.get().then_some(f)
}

/// Shared shapes (`trackbar::Shape`: a message box's icon) drawn with their
/// (0, 0) at (`x`, `y`): fills, then outlines, as the trackbar's are.
fn draw_shapes(shapes: &[rapidr_value::objects::trackbar::Shape], x: i32, y: i32) {
    let (ox, oy) = (f64::from(x), f64::from(y));
    let rgb = |c: u32| Color::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8);
    for shape in shapes {
        if let Some(c) = shape.fill.filter(|_| shape.points.len() > 2) {
            draw::set_draw_color(rgb(c));
            draw::begin_complex_polygon();
            for (px, py) in &shape.points {
                draw::vertex(ox + px, oy + py);
            }
            draw::end_complex_polygon();
        }
        if let Some(c) = shape.stroke {
            draw::set_draw_color(rgb(c));
            draw::begin_loop();
            for (px, py) in &shape.points {
                draw::vertex(ox + px, oy + py);
            }
            draw::end_loop();
        }
    }
}

/// A modal message laid out as every runtime lays it out (the dialogs
/// lane's; `rapidr_value::dialogs::message_layout`, Delphi's MessageDlg):
/// `icon` at the top left (the shared shapes), the text right of it in the
/// lines `dialogs::wrap` breaks it into, the buttons (Windows' 75 × 23)
/// centred under both in the given order, the first the default (Return);
/// Escape and the close box mean "none". Returns the index of the button
/// chosen. `beep`: the icon's sound as it shows (Windows' MessageBox), never
/// under a test.
pub fn gui_choice(title: &str, text: &str, labels: &[&str], icon: Option<rapidr_value::dialogs::MsgIcon>, beep: bool) -> Option<usize> {
    use fltk::button::ReturnButton;
    use rapidr_value::dialogs::{self as d, MsgIcon};
    use std::rc::Rc;
    ensure_app();
    let font = rapidr_value::objects::font::Font::default();
    let lines = d::wrap(text, &font, d::WRAP);
    let measure = |s: &str| rapidr_value::objects::text::text_size(s, &font);
    let line_h = measure("Ag").1.max(1);
    let text_w = lines.iter().map(|l| measure(l).0).max().unwrap_or(0);
    let layout = d::message_layout(text_w, lines.len() as i64 * line_h, labels.len(), icon.is_some());
    let px = |v: i64| v.clamp(-100_000, 100_000) as i32;
    let mut win = Window::default().with_size(px(layout.size.0), px(layout.size.1)).with_label(title);
    win.make_modal(true);
    if let (Some(icon), Some((x, y, w, h))) = (icon, layout.icon) {
        let mut pic = Frame::new(px(x), px(y), px(w), px(h), None);
        pic.draw(move |f| draw_shapes(&d::icon_shapes(icon), f.x(), f.y()));
    }
    let (tx, ty, _, _) = layout.text;
    for (i, line) in lines.iter().enumerate() {
        let mut msg = Frame::new(px(tx), px(ty + i as i64 * line_h), px(text_w + 2), px(line_h), None);
        // (FLTK draws `@…` as a symbol: shown as typed)
        msg.set_label(&line.replace('@', "@@"));
        msg.set_label_font(fltk_face(&font));
        msg.set_label_size(font_pixels(&font));
        msg.set_align(Align::Left | Align::Top | Align::Inside);
    }
    let chosen = Rc::new(std::cell::Cell::new(None));
    for (i, (label, &(x, y, w, h))) in labels.iter().zip(&layout.buttons).enumerate() {
        let chosen = chosen.clone();
        let mut win_ref = win.clone();
        let mut pick = move || {
            chosen.set(Some(i));
            win_ref.hide();
        };
        let caption = d::button_caption(label);
        if i == 0 {
            ReturnButton::new(px(x), px(y), px(w), px(h), None).with_label(&caption).set_callback(move |_| pick());
        } else {
            Button::new(px(x), px(y), px(w), px(h), None).with_label(&caption).set_callback(move |_| pick());
        }
    }
    win.end();
    if beep && !crate::ui::testhooks::under_test() {
        // (Windows' sounds as FLTK's beep names them: its "password" one is
        // MB_ICONWARNING's)
        dialog::beep(match icon {
            Some(MsgIcon::Error) => dialog::BeepType::Error,
            Some(MsgIcon::Question) => dialog::BeepType::Question,
            Some(MsgIcon::Warning) => dialog::BeepType::Password,
            Some(MsgIcon::Information) => dialog::BeepType::Message,
            None => dialog::BeepType::Default,
        });
    }
    win.show();
    while win.shown() {
        if !app::wait() {
            break;
        }
    }
    chosen.get()
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
                // (the WindowState lane's: iconized and restored by the user)
                window_state_handle(&mut win, &name_lower);
                let form = name_lower.clone();
                win.draw(move |w| {
                    scale_check(&form, w.pixels_per_unit());
                    scroll_bars_draw(&form, 0, menu_offset(&form), w.w(), w.h())
                });
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
                // (the WindowState lane's: iconized and restored by the user)
                window_state_handle(&mut win, &name_lower);
                let form = name_lower.clone();
                win.draw(move |w| {
                    scale_check(&form, w.pixels_per_unit());
                    scroll_bars_draw(&form, 0, menu_offset(&form), w.w(), w.h())
                });
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
            // (RapidR's handler first: it returns true for what it handles alone)
            btn.super_handle_first(false);
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

            // (RapidR's handler first: it returns true for what it handles alone)
            btn.super_handle_first(false);
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

            // (RapidR's handler first: it returns true for what it handles alone)
            btn.super_handle_first(false);
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
            let text = rapidr_value::objects::with_textedit(name, |t| t.raw()).unwrap_or_else(|| rp_comp_get(name, "text").to_string_val());
            let mut inp = Input::new(x, y, w, h, None);
            inp.set_value(&text);
            let name_for_cb = name.to_lowercase();
            inp.set_callback(move |_| {
                // (what the user typed goes to the text model: Modified)
                text_pull(&name_for_cb);
                rp_fire_event(&name_for_cb, "onchange");
            });
            let normal_frame = FrameType::DownBox;
            let focus_frame = FrameType::BorderBox;
            // (RapidR's handler first: it returns true for what it handles alone)
            inp.super_handle_first(false);
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
            // Its bevels (rapidr_value::objects::bevel), over what FLTK drew.
            let bevel_name = name_lower.clone();
            grp.draw(move |g| panel_bevels(&bevel_name, g.x(), g.y(), g.w(), g.h()));
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
            // (RapidR's handler first: it returns true for what it handles alone)
            cb.super_handle_first(false);
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
            // (RapidR's handler first: it returns true for what it handles alone)
            rb.super_handle_first(false);
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
            // csOwnerDrawFixed / csOwnerDrawVariable: drawn by OnDrawItem.
            if rapidr_value::objects::with_list(name, |l| l.owner_drawn()).unwrap_or(false) {
                owner_combo_create(&name_lower, x, y, w, h);
                return;
            }
            // Style (RAPIDQ.INC): csDropDown = 0 (the default) and csSimple
            // = 1 have an edit box; csDropDownList = 2 only picks from the
            // list.
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
            // Style = lbOwnerDrawFixed / lbOwnerDrawVariable (the items are
            // drawn by OnDrawItem) or Columns: a table (`owner_list_create`).
            if rapidr_value::objects::with_list(name, |l| l.custom_drawn()).unwrap_or(false) {
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
            let text = rapidr_value::objects::with_textedit(name, |t| t.raw()).unwrap_or_else(|| rp_comp_get(name, "text").to_string_val());
            let mut buf = TextBuffer::default();
            buf.set_text(&text);
            let mut editor = TextEditor::new(x, y, w, h, None);
            editor.set_buffer(buf.clone());
            // The user's typing: the text model (Modified), then OnChange —
            // not while the program's own text is being shown (text_push).
            let name_for_cb = name_lower.clone();
            buf.add_modify_callback(move |_, _, _, _, _| {
                if TEXT_PUSHING.with(std::cell::Cell::get) {
                    return;
                }
                let n = name_for_cb.clone();
                // (after FLTK finished the change: pulling now would read
                // a half-updated buffer)
                app::add_timeout3(0.0, move |_| {
                    text_pull(&n);
                    rp_fire_event(&n, "onchange");
                });
            });
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
            // A container whose first child draws the tabs from the shared
            // model (rapidr_value::objects::tabcontrol) and takes their
            // clicks; the program's components go over it.
            let grp = Group::new(x, y, w, h, None);
            let mut back = Frame::new(x, y, w, h, None);
            let id = name_lower.clone();
            back.draw(move |f| tab_control_draw(&id, f));
            let id = name_lower.clone();
            back.handle(move |f, ev| tab_control_event(&id, f, ev));
            grp.end();
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Group(grp));
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
            // (its text model's: what the program set before it was shown)
            let text = rapidr_value::objects::with_textedit(name, |t| t.raw()).unwrap_or_else(|| rp_comp_get(name, "text").to_string_val());
            let mut buf = TextBuffer::default();
            buf.set_text(&text);

            // The style buffer: a token letter per byte (A keyword, B string,
            // C comment, D number, E the rest: rapidr_value::objects::code).
            let mut style_buf = TextBuffer::default();
            style_buf.set_text(&code::style_bytes(&text));
            let entry = |t: code::Token| {
                let st = t.style();
                let font = match (st.bold, st.italic) {
                    (true, _) => Font::CourierBold,
                    (_, true) => Font::CourierItalic,
                    _ => Font::Courier,
                };
                StyleTableEntry { color: Color::from_hex(st.color), font, size: 13 }
            };
            let styles = [code::Token::Keyword, code::Token::String, code::Token::Comment, code::Token::Number, code::Token::Normal].map(entry).to_vec();

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

            // Every change of the text (typing, paste, the program's text)
            // coloured again; the user's typing goes to the text model,
            // then OnChange (as a QMEMO's). A selection changing alone is
            // neither.
            {
                let nl = name_lower.clone();
                buf.add_modify_callback(move |_pos, ins, del, _restyled, _deleted_text| {
                    if ins == 0 && del == 0 {
                        return;
                    }
                    // Use try_borrow to avoid panicking if we're inside gui_set_text
                    // which may still hold a borrow on GUI_TEXT_BUFFERS.
                    GUI_TEXT_BUFFERS.with(|tb| {
                        if let Ok(bufs) = tb.try_borrow() {
                            if let Some(text_buf) = bufs.get(&nl) {
                                let new_styles = code::style_bytes(&text_buf.text());
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
                    if TEXT_PUSHING.with(std::cell::Cell::get) {
                        return;
                    }
                    let n = nl.clone();
                    app::add_timeout3(0.0, move |_| {
                        text_pull(&n);
                        rp_fire_event(&n, "onchange");
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
            // No macOS "Window" menu (RapidQ has none): FLTK's keeps a list
            // of windows under it and crashes growing that list once the
            // program's own items have replaced the menu (msweep.bas: a
            // splash form shown and closed, then the main form).
            let h = rapidr_value::layout::MAIN_MENU_HEIGHT as i32;
            if cfg!(target_os = "macos") && menu_in_window() {
                // (a macOS SysMenuBar is always the system's: a plain bar)
                let mut mb = MenuBar::new(0, 0, pw, h, None);
                mb.set_text_size(13);
                GUI_WIDGETS.with(|gw| {
                    gw.borrow_mut().insert(name_lower.clone(), GuiWidget::MenuBar(mb));
                });
            } else {
                #[cfg(target_os = "macos")]
                SysMenuBar::set_window_menu_style(fltk::menu::WindowMenuStyle::NoWindowMenu);
                let mut mb = SysMenuBar::new(0, 0, pw, h, None);
                mb.set_text_size(13);
                GUI_WIDGETS.with(|gw| {
                    gw.borrow_mut().insert(name_lower.clone(), GuiWidget::SysMenuBar(mb));
                });
            }
            schedule_menu_sync();
        }
        // (drawn by their menu from the shared model: menu_rebuild)
        "RMENUITEM" => {}
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

            // (its components, selection and drags: the shared model,
            // rapidr_value::objects::design)
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
                // (RapidR's handler first: it returns true for what it handles alone)
                frm.super_handle_first(false);
                frm.handle(move |wid, ev| {
                    let (mx, my) = (app::event_x() - wid.x(), app::event_y() - wid.y());
                    let handled = design_surface_event(&ds_name2, ev, mx, my);
                    if handled {
                        wid.redraw();
                    }
                    handled
                });
                GUI_WIDGETS.with(|gw| {
                    gw.borrow_mut().insert(name_lower, GuiWidget::Frame(frm));
                });
            } else {
                // Standalone design surface: use a Window (drawn and
                // handled in its own coordinates)
                let mut win = Window::new(200, 200, w, h, None);
                win.set_label(&cap);
                win.set_color(Color::White);
                let ds_name = name_lower.clone();
                win.draw(move |wid| {
                    draw_design_surface(&ds_name, 0, 0, wid.w(), wid.h());
                });
                let ds_name2 = name_lower.clone();
                // (RapidR's handler first: it returns true for what it handles alone)
                win.super_handle_first(false);
                win.handle(move |wid, ev| {
                    let handled = design_surface_event(&ds_name2, ev, app::event_x(), app::event_y());
                    if handled {
                        wid.redraw();
                    }
                    handled
                });
                win.end();
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
            tree_create(&name_lower, x, y, w, h);
        }
        "RTRACKBAR" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            // Drawn from the shared model (rapidr_value::objects::trackbar):
            // the same shapes as the web's.
            let mut frm = Frame::new(x, y, w, h, None);
            frm.set_frame(FrameType::FlatBox);
            let name_for_draw = name_lower.clone();
            frm.draw(move |f| {
                let Some(shapes) = rapidr_value::objects::with_trackbar(&name_for_draw, |t| t.shapes(f.w() as f64, f.h() as f64, f.active_r())) else { return };
                draw::push_clip(f.x(), f.y(), f.w(), f.h());
                draw::draw_rect_fill(f.x(), f.y(), f.w(), f.h(), f.color());
                let (ox, oy) = (f.x() as f64, f.y() as f64);
                let rgb = |c: u32| Color::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8);
                for shape in shapes {
                    if let Some(c) = shape.fill.filter(|_| shape.points.len() > 2) {
                        draw::set_draw_color(rgb(c));
                        draw::begin_complex_polygon();
                        for (px, py) in &shape.points {
                            draw::vertex(ox + px, oy + py);
                        }
                        draw::end_complex_polygon();
                    }
                    if let Some(c) = shape.stroke {
                        draw::set_draw_color(rgb(c));
                        if shape.points.len() == 2 {
                            draw::begin_line();
                        } else {
                            draw::begin_loop();
                        }
                        for (px, py) in &shape.points {
                            draw::vertex(ox + px, oy + py);
                        }
                        if shape.points.len() == 2 {
                            draw::end_line();
                        } else {
                            draw::end_loop();
                        }
                    }
                }
                if fltk::app::focus().is_some_and(|w| w.as_widget_ptr() == f.as_widget_ptr()) {
                    draw::set_draw_color(Color::from_rgb(120, 120, 120));
                    draw::set_line_style(draw::LineStyle::Dot, 1);
                    draw::draw_rect(f.x(), f.y(), f.w(), f.h());
                    draw::set_line_style(draw::LineStyle::Solid, 0);
                }
                draw::pop_clip();
            });
            let name_for_cb = name_lower.clone();
            let dragging = std::rc::Rc::new(std::cell::Cell::new(false));
            frm.handle(move |f, ev| {
                let (mx, my) = ((app::event_x() - f.x()) as f64, (app::event_y() - f.y()) as f64);
                let (w, h) = (f.w() as f64, f.h() as f64);
                let changed = match ev {
                    Event::Focus | Event::Unfocus => {
                        f.redraw();
                        return true;
                    }
                    Event::Push => {
                        let _ = f.take_focus();
                        let (drag, moved) = rapidr_value::objects::with_trackbar_mut(&name_for_cb, |t| t.mouse_down(mx, my, w, h)).unwrap_or_default();
                        dragging.set(drag);
                        f.redraw();
                        moved
                    }
                    Event::Drag if dragging.get() => rapidr_value::objects::with_trackbar_mut(&name_for_cb, |t| t.drag(mx, my, w, h)).unwrap_or(false),
                    Event::Released => {
                        dragging.set(false);
                        false
                    }
                    Event::KeyDown => {
                        let vk = fltk_vk(app::event_key().bits());
                        let Some(changed) = rapidr_value::objects::with_trackbar_mut(&name_for_cb, |t| matches!(vk, 33..=40).then(|| t.key(vk))).flatten() else { return false };
                        changed
                    }
                    _ => return false,
                };
                if changed {
                    f.redraw();
                    rp_fire_event(&name_for_cb, "onchange");
                }
                true
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Frame(frm));
            });
        }
        "RCANVAS" | "RHEADER" => {
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
                note_display_scale(f);
                // (a QHEADER's faces are painted again at a new size)
                if rapidr_value::objects::is_header(&name_for_draw) && HEADER_SIZES.with(|s| s.borrow().get(&name_for_draw) != Some(&(w, h))) {
                    let name = name_for_draw.clone();
                    app::add_timeout3(0.0, move |_| header_refresh(&name));
                }
                let shown = rapidr_value::objects::with_canvas(&name_for_draw, w as i64, h as i64, |b| b.display_rgba());
                if let Some(mut img) = shown.and_then(|(pw, ph, rgba, scale)| display_image(pw, ph, &rgba, scale)) {
                    img.draw(ox, oy, w, h);
                }
            });

            let name_for_cb = name.to_lowercase();
            // (RapidR's handler first: it returns true for what it handles alone)
            frm.super_handle_first(false);
            frm.handle(move |_, ev| {
                match ev {
                    // (Its mouse events, OnClick at the release:
                    // `install_input_dispatch`.)
                    Event::Push => {
                        press_begin(&name_for_cb);
                        true
                    }
                    Event::Released => {
                        press_end(&name_for_cb);
                        true
                    }
                    Event::Move | Event::Drag => true,
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
            // (RapidR's handler first: it returns true for what it handles alone)
            frm.super_handle_first(false);
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
            // (RapidR's handler first: it returns true for what it handles alone)
            bar.super_handle_first(false);
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
            // Its components inside its edge, clipped to it; its scroll bars
            // (scroll.rs, rapidr_value::scrollbars) drawn over them.
            let mut grp = Group::new(x, y, w, h, None);
            grp.set_frame(if crate::scroll::border(name) > 0 { FrameType::DownBox } else { FrameType::FlatBox });
            grp.set_clip_children(true);
            grp.end();
            let id = name_lower.clone();
            grp.draw(move |g| {
                let b = crate::scroll::border(&id) as i32;
                scroll_bars_draw(&id, g.x() + b, g.y() + b, g.w(), g.h());
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Group(grp));
            });
        }
        "RLISTVIEW" => {
            // Drawn by the shared model (rapidr_value::objects::listview):
            // this shows it and passes it the mouse (`mouse_event`), the
            // keys (`key_events`) and the wheel.
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut frm = Frame::new(x, y, w, h, None);
            frm.set_frame(FrameType::FlatBox);
            let name_for_draw = name_lower.clone();
            frm.draw(move |f| {
                note_display_scale(f);
                let focused = app::focus().is_some_and(|w| w.as_widget_ptr() == f.as_widget_ptr());
                listview_prepare_sized(&name_for_draw, f.w(), f.h(), focused);
                let shown = rapidr_value::objects::listview_paint(&name_for_draw, listview_background(&name_for_draw)).map(|mut b| b.display_rgba());
                if let Some(mut img) = shown.and_then(|(pw, ph, rgba, scale)| display_image(pw, ph, &rgba, scale)) {
                    img.draw(f.x(), f.y(), f.w(), f.h());
                }
            });
            let name_for_cb = name_lower.clone();
            frm.super_handle_first(false);
            frm.handle(move |f, ev| match ev {
                // (it takes the keyboard's focus, as a list view does)
                Event::Focus | Event::Unfocus => {
                    f.redraw();
                    true
                }
                Event::Push => {
                    let _ = f.take_focus();
                    true
                }
                Event::MouseWheel => {
                    let notches = match app::event_dy() {
                        app::MouseWheel::Up => -1,
                        app::MouseWheel::Down => 1,
                        _ => 0,
                    };
                    listview_prepare(&name_for_cb);
                    if notches != 0 && rapidr_value::objects::with_listview_mut(&name_for_cb, |lv| lv.wheel(notches)).unwrap_or(false) {
                        f.redraw();
                    }
                    true
                }
                Event::Leave => {
                    if rapidr_value::objects::with_listview_mut(&name_for_cb, |lv| lv.mouse_leave()).unwrap_or(false) {
                        f.redraw();
                    }
                    false
                }
                // (the arrows move in the list, not to the next control)
                Event::KeyDown => matches!(app::event_key(), Key::Up | Key::Down | Key::Left | Key::Right | Key::PageUp | Key::PageDown | Key::Home | Key::End)
                    || app::event_text() == " ",
                Event::Move | Event::Drag | Event::Released | Event::Enter => true,
                _ => false,
            });
            // The caption's editor (ReadOnly False): a sibling over it.
            let mut editor = Input::new(0, 0, 0, 0, None);
            editor.set_frame(FrameType::BorderBox);
            editor.hide();
            let enter_name = name_lower.clone();
            editor.set_trigger(CallbackTrigger::EnterKeyAlways);
            editor.set_callback(move |_| listview_end_edit(&enter_name, true));
            let edit_name = name_lower.clone();
            editor.super_handle_first(false);
            editor.handle(move |_, ev| match ev {
                Event::Unfocus => {
                    listview_end_edit(&edit_name, true);
                    false
                }
                Event::KeyDown if app::event_key() == Key::Escape => {
                    listview_end_edit(&edit_name, false);
                    true
                }
                _ => false,
            });
            LISTVIEW_EDITORS.with(|e| e.borrow_mut().insert(name_lower.clone(), (editor, None)));
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Frame(frm));
            });
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
            // A pop-up menu: its items live in a menu widget that's never
            // shown itself (Popup(X, Y) pulls them down: gui_menu_popup).
            let mut mb = MenuBar::new(0, 0, 0, 0, None);
            mb.hide();
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::MenuBar(mb));
            });
            schedule_menu_sync();
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
            GuiWidget::MenuBar(w) => text!(w),
            GuiWidget::SysMenuBar(w) => text!(w),
            GuiWidget::Progress(w) => label!(w),
            GuiWidget::Grid(w, _) => label!(w),
            GuiWidget::Tree(w) => label!(w),
            GuiWidget::Slider(w) => label!(w),
        }
    });
    redraw_widget(&name.to_lowercase());
}

// ---------------------------------------------------------------------------
// Menus (rapidr_value::objects::menu: the tree and items' state)
// ---------------------------------------------------------------------------

thread_local! {
    /// The menu revision the widgets show, and whether a rebuild is queued.
    static MENU_SYNC: std::cell::Cell<(u64, bool)> = const { std::cell::Cell::new((u64::MAX, false)) };
}

/// A menu's item, as FLTK takes it: its path with `/` escaped.
fn menu_path_label(path: &[String], caption: &str) -> String {
    let esc = |s: &str| s.replace('\\', "\\\\").replace('/', "\\/");
    path.iter().map(|p| esc(p)).chain(std::iter::once(esc(caption))).collect::<Vec<_>>().join("/")
}

fn fltk_shortcut(sc: Option<rapidr_value::objects::menu::Shortcut>) -> fltk::enums::Shortcut {
    use fltk::enums::{Key, Shortcut};
    let Some(sc) = sc else { return Shortcut::None };
    let key = match sc.vk {
        8 => Key::BackSpace,
        9 => Key::Tab,
        13 => Key::Enter,
        27 => Key::Escape,
        33 => Key::PageUp,
        34 => Key::PageDown,
        35 => Key::End,
        36 => Key::Home,
        37 => Key::Left,
        38 => Key::Up,
        39 => Key::Right,
        40 => Key::Down,
        45 => Key::Insert,
        46 => Key::Delete,
        112..=123 => Key::from_i32(Key::F1.bits() + (sc.vk - 112) as i32),
        v => Key::from_char(char::from_u32(v as u32).unwrap_or(' ').to_ascii_lowercase()),
    };
    let mut s = Shortcut::None | key;
    if sc.ctrl {
        s = s | Shortcut::Ctrl;
    }
    if sc.shift {
        s = s | Shortcut::Shift;
    }
    if sc.alt {
        s = s | Shortcut::Alt;
    }
    s
}

/// Fills a menu widget with its menu's items (rapidr_value's model).
fn menu_rebuild<M: MenuExt>(mb: &mut M, root: &str) {
    use fltk::menu::MenuFlag;
    mb.clear();
    for e in rapidr_value::objects::menu::entries(root) {
        let mut flags = if e.submenu { MenuFlag::Submenu } else { MenuFlag::Normal };
        if e.checked && !e.submenu {
            flags |= if e.radio { MenuFlag::Radio | MenuFlag::Value } else { MenuFlag::Toggle | MenuFlag::Value };
        }
        if !e.enabled {
            flags |= MenuFlag::Inactive;
        }
        if e.divider_after {
            flags |= MenuFlag::MenuDivider;
        }
        let name = e.name.clone();
        let label = menu_path_label(&e.path, &e.caption);
        let shortcut = if e.submenu { fltk::enums::Shortcut::None } else { fltk_shortcut(e.shortcut) };
        mb.add(&label, shortcut, flags, move |_| {
            // (the menu is rebuilt after the handler: not while FLTK is
            // still in this item)
            rp_fire_event(&name, "onclick");
            schedule_menu_sync();
        });
    }
}

/// Rebuilds every built menu if the model changed since they were drawn.
fn menu_sync() {
    let rev = rapidr_value::objects::menu::revision();
    if MENU_SYNC.with(|m| m.get().0) == rev {
        return;
    }
    MENU_SYNC.with(|m| m.set((rev, false)));
    let built: Vec<(String, GuiWidget)> = GUI_WIDGETS.with(|gw| {
        gw.try_borrow().map(|gw| gw.iter().filter(|(n, _)| rapidr_value::objects::menu::kind(n).is_some_and(|k| k != rapidr_value::objects::menu::Kind::Item)).map(|(n, w)| (n.clone(), w.clone())).collect()).unwrap_or_default()
    });
    for (name, w) in built {
        match w {
            GuiWidget::SysMenuBar(mut mb) => {
                menu_rebuild(&mut mb, &name);
                mb.redraw();
                dump_menu(&name, &mb);
            }
            GuiWidget::MenuBar(mut mb) => {
                menu_rebuild(&mut mb, &name);
                dump_menu(&name, &mb);
            }
            _ => {}
        }
    }
}

/// `RAPIDR_DUMP_MENUS=1`: what FLTK holds after each rebuild, on stderr
/// (each item's path, and `*` checked / `!` disabled).
fn dump_menu<M: MenuExt>(name: &str, mb: &M) {
    if std::env::var_os("RAPIDR_DUMP_MENUS").is_none() {
        return;
    }
    let mut items = Vec::new();
    for i in 0..mb.size() {
        if let Some(it) = mb.at(i) {
            if let Some(label) = it.label() {
                let path = mb.item_pathname(Some(&it)).unwrap_or(label);
                let mut flags = String::new();
                if it.value() {
                    flags.push('*');
                }
                if !it.active() {
                    flags.push('!');
                }
                items.push(format!("{path}{flags}"));
            }
        }
    }
    eprintln!("[menu {name}] {}", items.join(" | "));
}

/// A menu changed (an item's Caption, Checked, …, AddItems): the widgets
/// are rebuilt once the program's code returns to the event loop.
pub fn schedule_menu_sync() {
    if MENU_SYNC.with(|m| m.get().1) {
        return;
    }
    MENU_SYNC.with(|m| {
        let (rev, _) = m.get();
        m.set((rev, true));
    });
    app::add_timeout3(0.0, |_| {
        MENU_SYNC.with(|m| {
            let (rev, _) = m.get();
            m.set((rev, false));
        });
        menu_sync();
    });
}

/// `PopupMenu.Popup(X, Y)`: OnPopup, then the menu at that place on the
/// screen; the item picked fires its OnClick.
pub fn gui_menu_popup(name: &str, x: i32, y: i32) {
    let name = name.to_lowercase();
    rp_fire_event(&name, "onpopup");
    MENU_SYNC.with(|m| m.set((u64::MAX, m.get().1)));
    menu_sync();
    let Some(GuiWidget::MenuBar(mb)) = GUI_WIDGETS.with(|gw| gw.borrow().get(&name).cloned()) else { return };
    let (wx, wy) = mb.top_window().map_or((0, 0), |w| (w.x(), w.y()));
    let Some(menu) = mb.menu() else { return };
    let align = rapidr_value::objects::menu::with(&name, |n| n.alignment).unwrap_or(0);
    let width = {
        // (paRight / paCenter: the menu's right edge / middle at X)
        let longest = rapidr_value::objects::menu::entries(&name).iter().filter(|e| e.path.is_empty()).map(|e| e.caption.chars().count()).max().unwrap_or(0);
        (longest as i32 * 7 + 40).max(60)
    };
    let x = match align {
        1 => x - width,
        2 => x - width / 2,
        _ => x,
    };
    if let Some(mut item) = menu.pulldown(x - wx, y - wy, 0, 0, None, Some(&mb)) {
        item.do_callback(&mb);
    }
}

/// A pop-up menu's widget, made in the front window if the program DIMmed the
/// menu (no form of its own) or its form isn't built yet.
pub fn ensure_menu_widget(name: &str) {
    let name = name.to_lowercase();
    if GUI_WIDGETS.with(|gw| gw.borrow().contains_key(&name)) {
        return;
    }
    let Some(mut win) = app::first_window() else { return };
    let mut mb = MenuBar::new(0, 0, 0, 0, None);
    mb.hide();
    win.add(&mb);
    GUI_WIDGETS.with(|gw| gw.borrow_mut().insert(name, GuiWidget::MenuBar(mb)));
}

/// A right click on a component whose PopupMenu names a menu with
/// AutoPopup on: that menu, under the mouse.
fn auto_popup(name: &str) -> bool {
    let menu = rp_comp_get(name, "popupmenu").to_string_val();
    if menu.is_empty() || !rapidr_value::objects::menu::with(&menu, |n| n.kind == rapidr_value::objects::menu::Kind::Popup && n.auto_popup).unwrap_or(false) {
        return false;
    }
    ensure_menu_widget(&menu);
    gui_menu_popup(&menu, app::event_x_root(), app::event_y_root());
    true
}

// ---------------------------------------------------------------------------
// Design surface rendering
// ---------------------------------------------------------------------------

/// A designed component's colour property ("#RRGGBB", "rgb(r,g,b)", or a
/// RapidQ &HBBGGRR / QCOLORDIALOG number: objects::design::parse_color).
fn parse_color_prop(s: &str) -> Option<Color> {
    rapidr_value::objects::design::parse_color(s).map(Color::from_hex)
}

/// A designed component in FLTK's coordinates (the shared model's, copied
/// for drawing).
struct Placed {
    name: String,
    type_name: String,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    props: std::collections::BTreeMap<String, String>,
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
    let model = rapidr_value::objects::with_design(ds_name, |d| {
        let placed: Vec<Placed> = d
            .components
            .iter()
            .map(|c| Placed { name: c.name.clone(), type_name: c.type_name.clone(), x: c.x as i32, y: c.y as i32, w: c.w as i32, h: c.h as i32, props: c.props.clone() })
            .collect();
        (placed, d.selection())
    });
    {
        if let Some((components, selected)) = model {
            for (i, comp) in components.iter().enumerate() {
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
                if selected == Some(i) {
                    draw::set_draw_color(Color::from_rgb(0, 120, 215));
                    draw::draw_rect(cx, cy, comp.w, comp.h);
                    draw_selection_handles(cx, cy, comp.w, comp.h);
                }
            }
        }
    }
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

/// The mouse on design surface `ds_name` at (mx, my) of it, through the
/// shared model (rapidr_value::objects::design): a press selects (OnSelect,
/// OnDblClick on a double click) or clears (OnBgClick), a drag moves or
/// resizes (OnMove), a release ends it. Whether it was the surface's.
fn design_surface_event(ds_name: &str, ev: Event, mx: i32, my: i32) -> bool {
    let (mx, my) = (i64::from(mx), i64::from(my));
    let heard = match ev {
        Event::Push => rapidr_value::objects::with_design_mut(ds_name, |d| d.mouse_down(mx, my, app::event_clicks())),
        Event::Drag => rapidr_value::objects::with_design_mut(ds_name, |d| d.mouse_drag(mx, my)),
        Event::Released => rapidr_value::objects::with_design_mut(ds_name, |d| {
            d.mouse_up();
            None
        }),
        _ => return false,
    };
    if let Some(e) = heard.flatten() {
        fire_design_event(ds_name, &e);
    }
    true
}

/// A design surface's event, fired with its arguments.
fn fire_design_event(ds_name: &str, e: &rapidr_value::objects::design::DesignEvent) {
    let args: Vec<Value> = e.args().into_iter().map(v_int).collect();
    crate::object::rp_fire_event_args(ds_name, e.event(), &args);
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

thread_local! {
    /// Timers with an FLTK timeout going (never two for one timer).
    static SCHEDULED_TIMERS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

fn timer_seconds(name: &str) -> f64 {
    let interval_ms = rp_comp_get(name, "interval").to_i64();
    if interval_ms > 0 { interval_ms as f64 / 1000.0 } else { 1.0 }
}

/// Starts timer `name` ticking if it's enabled and isn't already (it
/// fires while FLTK runs: a modal form, the main loop, DOEVENTS). Each
/// tick reads its Interval again; a disabled timer stops at its next tick.
fn schedule_timer(name: &str) {
    let name = name.to_lowercase();
    if rp_comp_get(&name, "enabled").to_i64() == 0 || !SCHEDULED_TIMERS.with(|s| s.borrow_mut().insert(name.clone())) {
        return;
    }
    app::add_timeout3(timer_seconds(&name), move |handle| {
        if rp_comp_get(&name, "enabled").to_i64() != 0 {
            rp_fire_event(&name, "ontimer");
            app::repeat_timeout3(timer_seconds(&name), handle);
        } else {
            SCHEDULED_TIMERS.with(|s| s.borrow_mut().remove(&name));
        }
    });
}

/// A timer's Enabled or Interval changed: it ticks if it's enabled now.
pub fn gui_timer_changed(name: &str) {
    if GUI_APP.with(|a| a.try_borrow().is_ok_and(|a| a.is_some())) {
        schedule_timer(name);
    }
}

/// `DOEVENTS`: the program's windows, timers and events get their turn
/// (what's pending is handled, then the program goes on).
pub fn gui_doevents() {
    if GUI_APP.with(|a| a.try_borrow().map_or(true, |a| a.is_none())) {
        return;
    }
    start_timers();
    show_pending();
    let _ = app::wait_for(0.0);
}

/// INPUT$'s wait in a program with windows: events are served (timers run,
/// windows repaint) until a key reaches INKEY$'s queue. `None` without a
/// window shown (the terminal waits instead); `Some(false)` when the last
/// window closed first.
pub fn gui_wait_key() -> Option<bool> {
    if GUI_APP.with(|a| a.try_borrow().map_or(true, |a| a.is_none())) {
        return None;
    }
    if !fltk::app::windows().is_some_and(|w| w.iter().any(|w| w.shown())) {
        return None;
    }
    start_timers();
    while !rapidr_value::console::key_waiting() {
        show_pending();
        if !app::wait() {
            return Some(false);
        }
    }
    Some(true)
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
    show_pending();
    let done = WAITS.with(|w| match w.borrow().last() {
        None => true,
        Some(Wait::Form(name)) => !form_shown(name),
        Some(Wait::App) => false,
    });
    // Don't hold a borrow while FLTK runs callbacks.
    if done || !app::wait() {
        let finished = WAITS.with(|w| w.borrow_mut().pop());
        if let Some(Wait::Form(form)) = finished {
            // As the blocking ShowModal does when its form closes: what
            // ShowModal returns is the form's ModalResult.
            crate::object::rp_stop_all_timers();
            return Some(v_int(modal_ended(&form)));
        }
        return Some(v_null());
    }
    None
}

/// Show a form as modal (blocking event loop; for the bytecode VM, a wait
/// it serves — see [`gui_set_cooperative_waits`]).
pub fn gui_showmodal(name: &str) -> i64 {
    ensure_app();
    let name_lower = name.to_lowercase();
    crate::object::store_prop(&name_lower, "modalresult", v_int(0));
    MODAL_FORMS.with(|m| m.borrow_mut().push(name_lower.clone()));

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
    owner_draw_shown_grids();
    // (the WindowState lane's: shown maximized or minimized as asked)
    apply_shown_state(name);

    // Fire OnShow event after widgets are built and window is shown
    rp_fire_event(name, "onshow");
    after_show(name);

    // Start all registered timers
    start_timers();

    if COOPERATIVE.with(|c| c.get()) {
        WAITS.with(|w| w.borrow_mut().push(Wait::Form(name_lower)));
        WAIT_STARTED.with(|w| w.set(true));
        return 0;
    }

    // Run the FLTK event loop — do NOT hold a borrow on GUI_APP during wait()
    // because callbacks may call ensure_app() which needs borrow_mut.
    show_pending();
    while app::wait() {
        show_pending();
        if !form_shown(&name_lower) {
            break;
        }
    }

    // The form has closed: disable any timers so their already-queued
    // FLTK timeouts don't fire after we tear down the dispatcher.
    crate::object::rp_stop_all_timers();
    modal_ended(&name_lower)
}

thread_local! {
    /// The forms shown modally now, innermost last.
    static MODAL_FORMS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Whether `name` is shown modally now (setting its ModalResult closes it).
pub fn is_modal(name: &str) -> bool {
    let n = name.to_lowercase();
    MODAL_FORMS.with(|m| m.borrow().contains(&n))
}

/// A modal form closed: what its ShowModal returns.
fn modal_ended(name_lower: &str) -> i64 {
    MODAL_FORMS.with(|m| m.borrow_mut().retain(|f| f != name_lower));
    rapidr_value::events::modal_result(rp_comp_get(name_lower, "modalresult").to_i64())
}

/// Hide a widget (form window or embedded frame): no OnClose.
pub fn gui_hide(name: &str) {
    // (a handle to the widget: FLTK dispatches events while hiding)
    match GUI_WIDGETS.with(|gw| gw.borrow().get(&name.to_lowercase()).cloned()) {
        Some(GuiWidget::Window(mut win)) => win.hide(),
        Some(GuiWidget::Frame(mut frm)) => frm.hide(),
        _ => {}
    }
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
        if let Some(GuiWidget::Window(mut win)) = GUI_WIDGETS.with(|gw| gw.borrow().get(&name_lower).cloned()) {
            match CloseAction::of(&a[0]) {
                CloseAction::Stay => {}
                CloseAction::Minimize => win.iconize(),
                CloseAction::Close => win.hide(),
            }
        }
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

/// An Open / Save dialog (ui::file_dialog: the request and the answer are
/// host-neutral): FLTK's native chooser shown for the request.
fn file_dialog(name: &str, save: bool, multi: bool) -> Value {
    use rapidr_value::file_dialog as fd;
    crate::ui::file_dialog::execute(name, save, multi, |req| {
        let kind = match (req.save, req.multi) {
            (true, _) => dialog::NativeFileChooserType::BrowseSaveFile,
            (false, true) => dialog::NativeFileChooserType::BrowseMultiFile,
            (false, false) => dialog::NativeFileChooserType::BrowseFile,
        };
        let mut dlg = dialog::NativeFileChooser::new(kind);
        if let Some(title) = &req.title {
            dlg.set_title(title);
        }
        if !req.filters.is_empty() {
            dlg.set_filter(&fd::fltk_filter(&req.filters));
            dlg.set_filter_value(req.filter_index as i32);
        }
        if let Some(dir) = &req.dir {
            let _ = dlg.set_directory(dir);
        }
        if let Some(file) = &req.file_name {
            dlg.set_preset_file(file);
        }
        if req.confirm_overwrite {
            dlg.set_option(dialog::FileDialogOptions::SaveAsConfirm);
        }
        dlg.show();
        dlg.filenames().iter().map(|p| p.to_string_lossy().into_owned()).collect()
    })
}

/// Execute a dialog (Open/Save/Color/Font).
pub fn gui_dialog_execute(name: &str, comp_type: &str) -> Value {
    ensure_app();
    match comp_type {
        // (QOPENDIALOG, QSAVEDIALOG, RAPIDQ2.INC's QFILEDIALOG)
        "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG" => match crate::ui::file_dialog::kind(name, comp_type) {
            Some((save, multi)) => file_dialog(name, save, multi),
            None => v_int(0),
        },
        // (the dialogs lane's: Caption, the Color it starts with and the
        // answers as on every host — ui::choose_dialogs; FLTK's chooser has
        // no basic or custom swatches, so Colors(i) come back as they were)
        "RCOLORDIALOG" => crate::ui::choose_dialogs::color_execute(name, |title, state| (fltk_color_dialog(title, state.color), state.custom)),
        // (the dialogs lane's: laid out as every host's, answering the same
        // properties — ui::choose_dialogs, rapidr_value::font_dialog)
        "RFONTDIALOG" => crate::ui::choose_dialogs::font_execute(name, |title, req| fltk_font_dialog(name, title, req)),
        _ => v_int(0),
    }
}

/// Start the GUI event loop (standalone, not attached to a form).
pub fn run_gui_event_loop() {
    ensure_app();
    // Don't hold a borrow on GUI_APP during the event loop (app::run, with
    // the windows made visible showing as it waits)
    show_pending();
    while app::wait() {
        show_pending();
    }
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
    gui_apply_icon(form_name);

    // Recursively build all children
    build_children_recursive(form_name);

    // Fire OnLoad once, after the entire form tree is materialized.
    rp_fire_event(form_name, "onload");
    // Then, once the window is shown (`after_show`), the first OnPaint of
    // the form and its canvases (RapidQ programs draw there); the surfaces
    // keep what's drawn, so later ones are only asked for (Repaint) or
    // follow a resize.
    FIRST_PAINT.with(|f| f.borrow_mut().insert(form_name.to_lowercase()));
}

thread_local! {
    /// Forms built whose first OnPaint waits for their window to show.
    static FIRST_PAINT: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// A form's window shown: what's drawn from now on is kept at its screen's
/// scale, then (the first time) its OnPaint — as Windows' WM_PAINT comes
/// once a window shows, after OnShow — so the program draws on the
/// high-DPI surface from the start, whatever order events arrive in.
fn after_show(name: &str) {
    let name = name.to_lowercase();
    if let Some(GuiWidget::Window(win)) = GUI_WIDGETS.with(|gw| gw.borrow().get(&name).cloned()) {
        let scale = forced_scale().unwrap_or_else(|| f64::from(win.pixels_per_unit()));
        rapidr_value::objects::bitmap::set_display_scale(scale);
        FORM_SCALES.with(|f| f.borrow_mut().insert(name.clone(), scale));
    }
    if FIRST_PAINT.with(|f| f.borrow_mut().remove(&name)) {
        fire_first_paint(&name);
    }
}

/// `RAPIDR_SCALE`: the screen's scale for tests.
fn forced_scale() -> Option<f64> {
    std::env::var("RAPIDR_SCALE").ok().and_then(|s| s.parse::<f64>().ok()).filter(|s| *s > 0.0)
}

thread_local! {
    /// Each shown form's screen scale (Form.Scale), for OnScaleChanged.
    static FORM_SCALES: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
}

/// Form.Scale: the scale of the screen the form shows on (Screen.Scale
/// before it shows).
pub fn form_scale(name: &str) -> f64 {
    FORM_SCALES.with(|f| f.borrow().get(&name.to_lowercase()).copied()).unwrap_or_else(|| forced_scale().unwrap_or_else(rapidr_value::objects::bitmap::exact_scale))
}

/// A form drawn: moved to a screen with another scale, it's told
/// (OnScaleChanged, RapidR's) and drawn again at it (OnPaint, its canvases').
fn scale_check(form: &str, pixels_per_unit: f32) {
    if forced_scale().is_some() {
        return;
    }
    let scale = f64::from(pixels_per_unit);
    let changed = FORM_SCALES.with(|f| match f.borrow_mut().insert(form.to_string(), scale) {
        Some(old) => (old - scale).abs() > f64::EPSILON,
        None => false,
    });
    if changed {
        rapidr_value::objects::bitmap::set_display_scale(scale);
        let form = form.to_string();
        // (not inside the draw: what the handlers draw is drawn next)
        app::add_timeout3(0.0, move |_| {
            rp_fire_event(&form, "onscalechanged");
            fire_first_paint(&form);
        });
    }
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
                // (a scroll box's components sit inside its edge)
                GuiWidget::Group(ref g) => {
                    let b = crate::scroll::border(name) as i32;
                    (g.x() + b, g.y() + b)
                }
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
    if has_menu && menu_in_window() { rapidr_value::layout::MAIN_MENU_HEIGHT as i32 } else { 0 }
}

/// Whether a QMAINMENU is a bar inside its form's window, as on Windows and
/// in the browser: everywhere but macOS, where it's the system menu bar —
/// unless `RAPIDR_MENU=window` asks for the in-window bar there too (the
/// same ClientHeight on every platform).
pub fn menu_in_window() -> bool {
    !cfg!(target_os = "macos") || std::env::var("RAPIDR_MENU").is_ok_and(|v| v.eq_ignore_ascii_case("window"))
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
        GuiWidget::MenuBar(v) => v.resize(x, y, w, h),
        GuiWidget::SysMenuBar(v) => v.resize(x, y, w, h),
        GuiWidget::Progress(v) => v.resize(x, y, w, h),
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
        GuiWidget::Grid(v, _) => redraw_win!(v),
        GuiWidget::HoldBrowser(v) => redraw_win!(v),
        GuiWidget::TextEditor(v) => redraw_win!(v),
        GuiWidget::Input(v) => redraw_win!(v),
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
    // A form's Constraints: the user can't drag it outside them.
    if let GuiWidget::Window(win) = &mut widget {
        form_size_range(&name, win);
    }
    // A form's main menu spans its width.
    if let GuiWidget::Window(_) = widget {
        for (child, t) in crate::object::get_children_of(&name) {
            if t == "RMAINMENU" {
                match GUI_WIDGETS.with(|gw| gw.borrow().get(&child).cloned()) {
                    Some(GuiWidget::SysMenuBar(mut mb)) => mb.resize(0, 0, rect.2, mb.h()),
                    Some(GuiWidget::MenuBar(mut mb)) => mb.resize(0, 0, rect.2, mb.h()),
                    _ => {}
                }
            }
        }
    }
    for (child, _) in crate::object::get_children_of(&name) {
        gui_apply_geometry(&child);
    }
    // A tab control's tabs (its first child) cover it.
    if comp_type == "RTABCONTROL" {
        if let Some(GuiWidget::Group(g)) = GUI_WIDGETS.with(|gw| gw.borrow().get(&name).cloned()) {
            if let Some(mut back) = g.child(0) {
                back.resize(g.x(), g.y(), g.w(), g.h());
            }
        }
    }
}

/// A form's MinWidth … MaxHeight as its window's size range (the window
/// is Width / Height less the frame); left alone while it has none.
fn form_size_range(name: &str, win: &mut Window) {
    let k = crate::layout::constraints_of(name);
    let had = rp_comp_get(name, "__sizerange").to_bool();
    if k.is_none() && !had {
        return;
    }
    crate::object::store_prop(name, "__sizerange", crate::value::v_bool(!k.is_none()));
    let (fw, fh) = rapidr_value::layout::form_frame(rp_comp_get(name, "borderstyle").to_i64());
    let inside = |v: i64, frame: i64| if v > 0 { (v - frame).clamp(1, 100_000) as i32 } else { 0 };
    let (min_w, min_h) = (inside(k.min_width, fw).max(1), inside(k.min_height, fh).max(1));
    win.size_range(min_w, min_h, inside(k.max_width, fw), inside(k.max_height, fh));
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

/// A form's window icon: its `IcoHandle` / `Icon` (an ICO, BMP, PNG or
/// SVG), else the application's (`Application.Icon`); none: the system's.
/// (macOS shows no window icons; Windows and Linux do.)
pub fn gui_apply_icon(name: &str) {
    let name = name.to_lowercase();
    let Some(GuiWidget::Window(mut win)) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|w| w.get(&name).cloned())) else { return };
    let own = ["icohandle", "icon"].into_iter().map(|p| rp_comp_get(&name, p)).find(rapidr_value::objects::has_icon);
    let icon = own.or_else(rapidr_value::globals::application_icon).and_then(|v| rapidr_value::objects::icon_pixels(&v));
    let image = icon.and_then(|(w, h, rgba, _)| fltk::image::RgbImage::new(&rgba, w as i32, h as i32, fltk::enums::ColorDepth::Rgba8).ok());
    win.set_icon(image);
}

/// `Application.Icon` changed: every form without its own icon.
pub fn gui_apply_icons() {
    let forms: Vec<String> = GUI_WIDGETS.with(|gw| gw.borrow().iter().filter(|(_, w)| matches!(w, GuiWidget::Window(_))).map(|(n, _)| n.clone()).collect());
    for form in forms {
        gui_apply_icon(&form);
    }
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

// ------------------------------------------- the WindowState lane's --

thread_local! {
    /// The bounds a maximized form goes back to, under a test's hooks
    /// (`gui_set_window_state`).
    static NORMAL_BOUNDS: RefCell<HashMap<String, rapidr_value::window_state::Bounds>> = RefCell::new(HashMap::new());
}

/// `Form.WindowState` set (it was `from`): the window maximized (FLTK's
/// maximize), minimized (iconized) or restored — its resize comes back
/// through `form_resized` (Left … Height follow, OnResize). Under a test's
/// hooks the window isn't the system's to maximize (a locked screen, an
/// animated zoom): the form takes the work area itself, as the kernel's
/// headless host does (`rapidr_value::window_state::change`), and
/// minimizing changes nothing to see. A form not shown yet takes its state
/// when it shows.
pub fn gui_set_window_state(name: &str, from: i64) {
    use rapidr_value::window_state as ws;
    let name = name.to_lowercase();
    let Some(GuiWidget::Window(mut win)) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|w| w.get(&name).cloned())) else { return };
    if !win.shown() {
        return;
    }
    let to = ws::of(rp_comp_get(&name, "windowstate").to_i64());
    if crate::ui::testhooks::under_test() {
        let get = |p: &str| rp_comp_get(&name, p).to_i64();
        let current = (get("left"), get("top"), get("width"), get("height"));
        let (x, y, w, h) = app::screen_work_area(0);
        let saved = NORMAL_BOUNDS.with(|n| n.borrow().get(&name).copied());
        let (bounds, keep) = ws::change(from, to, current, saved, (i64::from(x), i64::from(y), i64::from(w), i64::from(h)));
        NORMAL_BOUNDS.with(|n| match keep {
            Some(b) => n.borrow_mut().insert(name.clone(), b),
            None => n.borrow_mut().remove(&name),
        });
        if let Some((left, top, w, h)) = bounds {
            // (as a user's drag: form_resized follows)
            let (fw, fh) = rapidr_value::layout::form_frame(get("borderstyle"));
            let px = |v: i64| v.clamp(-100_000, 100_000) as i32;
            win.resize(px(left), px(top), px((w - fw).max(1)), px((h - fh).max(1)));
        }
        return;
    }
    match to {
        ws::WS_MAXIMIZED => {
            if from == ws::WS_MINIMIZED {
                win.show();
            }
            win.maximize();
        }
        ws::WS_MINIMIZED => win.iconize(),
        _ => {
            if from == ws::WS_MINIMIZED {
                win.show();
            }
            if win.maximize_active() {
                win.un_maximize();
            }
        }
    }
}

/// A form shown with a WindowState asked for before (maximized,
/// minimized): so now — once (a form already maximized under a test's hooks
/// stays as it is).
fn apply_shown_state(name: &str) {
    use rapidr_value::window_state as ws;
    let name = name.to_lowercase();
    let state = ws::of(rp_comp_get(&name, "windowstate").to_i64());
    if state != ws::WS_NORMAL && !NORMAL_BOUNDS.with(|n| n.borrow().contains_key(&name)) {
        gui_set_window_state(&name, ws::WS_NORMAL);
    }
}

/// What form `form`'s window is now — iconized, maximized or neither — into
/// its WindowState when the user changed it (not under a test's hooks,
/// whose windows the system doesn't maximize).
fn note_window_state(form: &str) {
    use rapidr_value::window_state as ws;
    if crate::ui::testhooks::under_test() {
        return;
    }
    let Some(GuiWidget::Window(win)) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|w| w.get(&form.to_lowercase()).cloned())) else { return };
    if !win.shown() {
        return;
    }
    let now = if !win.visible() {
        ws::WS_MINIMIZED
    } else if win.maximize_active() {
        ws::WS_MAXIMIZED
    } else {
        ws::WS_NORMAL
    };
    if rp_comp_get(form, "windowstate").to_i64() != now {
        crate::object::store_prop(form, "windowstate", v_int(now));
    }
}

/// A form's window tells its iconizing and restoring (FLTK's hide / show
/// while it's shown).
fn window_state_handle(win: &mut Window, name: &str) {
    let name = name.to_string();
    win.handle(move |_, ev| {
        if matches!(ev, Event::Hide | Event::Show) {
            let name = name.clone();
            // (after FLTK has updated the window)
            app::add_timeout3(0.0, move |_| note_window_state(&name));
        }
        false
    });
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
    // (the WindowState lane's: the user maximized or restored it)
    note_window_state(form);
    APPLYING.with(|a| a.set(a.get() + 1));
    crate::layout::quietly(|| {
        rp_comp_set(form, "left", v_int(x as i64));
        rp_comp_set(form, "top", v_int(y as i64));
    });
    APPLYING.with(|a| a.set(a.get() - 1));
    // The window is the inside: Width / Height add the frame — within the
    // form's Constraints.
    let (fw, fh) = rapidr_value::layout::form_frame(rp_comp_get(form, "borderstyle").to_i64());
    let asked = (w as i64 + fw, h as i64 + fh);
    let (w, h) = crate::layout::constraints_of(form).size(asked.0, asked.1);
    let same = rp_comp_get(form, "width").to_i64() == w && rp_comp_get(form, "height").to_i64() == h;
    if same {
        // (dragged outside them: the window goes back)
        if (w, h) != asked {
            gui_apply_geometry(form);
        }
        return;
    }
    crate::layout::quietly(|| {
        rp_comp_set(form, "width", v_int(w));
        rp_comp_set(form, "height", v_int(h));
    });
    crate::layout::client_changed(form);
    crate::scroll::update(form);
    gui_apply_geometry(form);
    rp_fire_event(form, "onresize");
    rp_fire_event(form, "onpaint");
}

/// Whether a form's window shows (`None` before it's built).
pub fn window_shown(name: &str) -> Option<bool> {
    GUI_WIDGETS.with(|gw| match gw.borrow().get(&name.to_lowercase()) {
        Some(GuiWidget::Window(win)) => Some(win.shown()),
        _ => None,
    })
}

/// `Form.Visible = True`: the form's Show. A form not built yet (its
/// `Visible = 1` inside its own CREATE, before its components exist) shows
/// as soon as the program waits — DoEvents, ShowModal, its event loop.
pub fn gui_show_visible(name: &str) {
    if window_shown(name).is_some() {
        gui_show(name);
        return;
    }
    ensure_app();
    PENDING_SHOWS.with(|p| p.borrow_mut().push(name.to_string()));
}

thread_local! {
    /// Windows a `Visible = True` shows once the program waits.
    static PENDING_SHOWS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// The program waits (its event loop, ShowModal, DoEvents): the windows it
/// made visible show (unless it hid them again meanwhile).
fn show_pending() {
    for name in PENDING_SHOWS.with(|p| std::mem::take(&mut *p.borrow_mut())) {
        if window_shown(&name) != Some(true) && crate::object::rp_comp_get(&name, "visible").to_bool() {
            gui_show(&name);
        }
    }
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
        // (a form shown again after Hide / Close gets OnShow again, as in
        // Delphi; one already showing doesn't)
        let was_hidden = GUI_WIDGETS.with(|gw| matches!(gw.borrow().get(&name_lower), Some(GuiWidget::Window(win)) if !win.shown()));
        GUI_WIDGETS.with(|gw| {
            let mut widgets = gw.borrow_mut();
            match widgets.get_mut(&name_lower) {
                Some(GuiWidget::Window(ref mut win)) => { win.show(); }
                Some(GuiWidget::Frame(ref mut frm)) => { frm.show(); }
                _ => {}
            }
        });
        owner_draw_shown_grids();
        if was_hidden {
            rp_fire_event(name, "onshow");
            after_show(name);
        }
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

    owner_draw_shown_grids();
    // (the WindowState lane's: shown maximized or minimized as asked)
    apply_shown_state(name);
    // Fire OnShow event after widgets are built and shown
    rp_fire_event(name, "onshow");
    after_show(name);
}

// ---------------------------------------------------------------------------
// Design surface methods
// ---------------------------------------------------------------------------

/// An RDESIGNSURFACE's methods the shared model leaves to the host (its
/// AddComponent, GetProp, SelectComp …: rapidr_value::objects::design).
pub fn design_surface_method(name: &str, method: &str, _args: &[Value]) -> Value {
    match method {
        "show" => gui_show(name),
        "hide" => gui_hide(name),
        _ => eprintln!("[WARN] DesignSurface.{}() not implemented", method),
    }
    v_null()
}

// ---------------------------------------------------------------------------
// String grid methods
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tab control methods
// ---------------------------------------------------------------------------

/// A QTABCONTROL's font (its text is measured with it: the shared model
/// lays tabs out the same on every runtime).
fn tab_control_font(name: &str) -> rapidr_value::objects::font::Font {
    rapidr_value::objects::font_from_props(name, &|id, p| rp_comp_get(id, p))
}

/// The FLTK face for a QFONT's name and style.
fn fltk_face(font: &rapidr_value::objects::font::Font) -> Font {
    let face = font.name.to_lowercase();
    let (bold, italic) = (font.styles & 1 != 0, font.styles & 2 != 0);
    let has = |words: &[&str]| words.iter().any(|w| face.contains(w));
    let faces = if has(&["courier", "mono", "consol", "fixed"]) {
        [Font::Courier, Font::CourierBold, Font::CourierItalic, Font::CourierBoldItalic]
    } else if has(&["times", "roman", "georgia", "garamond"]) || (face.contains("serif") && !face.contains("sans")) {
        [Font::Times, Font::TimesBold, Font::TimesItalic, Font::TimesBoldItalic]
    } else {
        [Font::Helvetica, Font::HelveticaBold, Font::HelveticaItalic, Font::HelveticaBoldItalic]
    };
    faces[usize::from(bold) + 2 * usize::from(italic)]
}

/// A QFONT's size in pixels (points at 96 dpi; negative: pixels).
fn font_pixels(font: &rapidr_value::objects::font::Font) -> i32 {
    font.pixel_size().min(512) as i32
}

/// Draws a QTABCONTROL's tabs (its first child, `f`, covers it).
fn tab_control_draw(name: &str, f: &mut Frame) {
    use rapidr_value::objects::tabcontrol::Op;
    let (ox, oy, w, h) = (f.x(), f.y(), f.w(), f.h());
    let font = tab_control_font(name);
    let color = rapidr_value::objects::form_color(&rp_comp_get(name, "color"));
    let focused = app::focus().is_some_and(|w| w.as_widget_ptr() == f.as_widget_ptr());
    let Some(ops) = rapidr_value::objects::with_tabcontrol(name, |t| t.ops(w as i64, h as i64, &font, color, f.active_r(), focused)) else { return };
    draw::push_clip(ox, oy, w, h);
    for op in ops {
        match op {
            Op::Fill { rect: (x, y, rw, rh), color } => draw::draw_rect_fill(ox + x as i32, oy + y as i32, rw as i32, rh as i32, Color::from_hex(color)),
            Op::Text { rect: (x, y, rw, rh), text, angle, font, color } => {
                draw::set_font(fltk_face(&font), font_pixels(&font));
                draw::set_draw_color(Color::from_hex(color));
                let tw = draw::width(&text);
                let (asc, desc) = (f64::from(draw::height() - draw::descent()), f64::from(draw::descent()));
                let (cx, cy) = (f64::from(ox) + x as f64 + rw as f64 / 2.0, f64::from(oy) + y as f64 + rh as f64 / 2.0);
                let mid = (asc - desc) / 2.0;
                // (text drawn as is: no '@' symbols, no '&' underlines)
                match angle {
                    90 => draw::draw_text_angled(90, &text, (cx + mid).round() as i32, (cy + tw / 2.0).round() as i32),
                    -90 => draw::draw_text_angled(-90, &text, (cx - mid).round() as i32, (cy - tw / 2.0).round() as i32),
                    _ => draw::draw_text(&text, (cx - tw / 2.0).round() as i32, (cy + mid).round() as i32),
                }
            }
            Op::Focus { rect: (x, y, rw, rh) } => {
                draw::set_draw_color(Color::Black);
                draw::set_line_style(draw::LineStyle::Dot, 1);
                draw::draw_rect(ox + x as i32, oy + y as i32, rw as i32, rh as i32);
                draw::set_line_style(draw::LineStyle::Solid, 0);
            }
            Op::Arrow { points, color } => {
                draw::set_draw_color(Color::from_hex(color));
                draw::begin_polygon();
                for (px, py) in points {
                    draw::vertex(f64::from(ox) + px, f64::from(oy) + py);
                }
                draw::end_polygon();
            }
        }
    }
    draw::pop_clip();
}

/// A QTABCONTROL's tabs: a click (or the arrow keys) picks a tab
/// (OnChange), the scroll buttons scroll them, HotTrack follows the mouse.
fn tab_control_event(name: &str, f: &mut Frame, ev: Event) -> bool {
    let (w, h) = (f.w() as i64, f.h() as i64);
    let (mx, my) = ((app::event_x() - f.x()) as i64, (app::event_y() - f.y()) as i64);
    let font = tab_control_font(name);
    // (the whole control drawn again: its components lie over the tabs'
    // widget, which FLTK doesn't draw again by itself)
    let redraw = |f: &Frame| match f.parent() {
        Some(mut p) => p.redraw(),
        None => f.clone().redraw(),
    };
    let changed = match ev {
        Event::Focus | Event::Unfocus => {
            redraw(f);
            return true;
        }
        Event::Enter => return true,
        Event::Move | Event::Leave => {
            let at = (ev == Event::Move).then_some((mx, my));
            if rapidr_value::objects::with_tabcontrol_mut(name, |t| t.mouse_move(at, w, h, &font)) == Some(true) {
                redraw(f);
            }
            return ev == Event::Move;
        }
        Event::Push => {
            let Some((changed, focus)) = rapidr_value::objects::with_tabcontrol_mut(name, |t| t.mouse_down(mx, my, w, h, &font)).flatten() else { return false };
            if focus {
                let _ = f.take_focus();
            }
            redraw(f);
            changed
        }
        Event::KeyDown if app::focus().is_some_and(|w| w.as_widget_ptr() == f.as_widget_ptr()) => {
            let vk = fltk_vk(app::event_key().bits());
            if !matches!(vk, 37..=40) {
                return false;
            }
            rapidr_value::objects::with_tabcontrol_mut(name, |t| t.key(vk, w, h, &font)).unwrap_or(false)
        }
        _ => return false,
    };
    if changed {
        redraw(f);
        rp_fire_event(name, "onchange");
    }
    true
}

/// A QTABCONTROL changed by the program: drawn again, its aligned
/// components laid out in its area again.
pub fn tab_control_changed(name: &str) {
    redraw_widget(name);
    crate::layout::client_changed(name);
}

/// A QPANEL's BevelOuter / BevelInner frames.
fn panel_bevels(name: &str, x: i32, y: i32, w: i32, h: i32) {
    let prop = |p: &str| rp_comp_get(name, p).to_i64();
    let frames = rapidr_value::objects::bevel::frames(prop("bevelouter"), prop("bevelinner"), prop("bevelwidth"), prop("borderwidth"));
    for f in frames {
        let i = f.inset as i32;
        let (x0, y0, x1, y1) = (x + i, y + i, x + w - 1 - i, y + h - 1 - i);
        if x1 <= x0 || y1 <= y0 {
            break;
        }
        draw::set_draw_color(Color::from_hex(f.top_left));
        draw::draw_line(x0, y0, x1, y0);
        draw::draw_line(x0, y0, x0, y1);
        draw::set_draw_color(Color::from_hex(f.bottom_right));
        draw::draw_line(x0, y1, x1, y1);
        draw::draw_line(x1, y0, x1, y1);
    }
}

// ---------------------------------------------------------------------------
// TreeView methods
// ---------------------------------------------------------------------------

pub fn tree_method(name: &str, method: &str, args: &[Value]) -> Value {
    // (the nodes are the shared model's: rapidr_value::objects::tree)
    match method {
        // GetItemAt(X, Y): the node shown there (-1: none).
        "getitemat" => {
            let y = args.get(1).map_or(0, Value::to_i64) as i32;
            let Some(GuiWidget::Tree(t)) = GUI_WIDGETS.with(|gw| gw.borrow().get(&name.to_lowercase()).cloned()) else { return v_int(-1) };
            let at = t.y() + y;
            let shown = |k: usize| rapidr_value::objects::with_tree(name, |m| m.is_visible(k)).unwrap_or(false);
            let hit = tree_items(&t).iter().enumerate().find(|(k, item)| shown(*k) && item.y() <= at && at < item.y() + item.h()).map(|(k, _)| k as i64);
            return v_int(hit.unwrap_or(-1));
        }
        "show" => gui_show(name),
        "hide" => gui_hide(name),
        _ => eprintln!("[WARN] TreeView.{}() not implemented", method),
    }
    v_null()
}

thread_local! {
    /// Each tree widget's items were built from this shape of its nodes
    /// (`TreeView::shape_hash`); a rebuild is due once it changes.
    static TREE_SHAPES: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
    /// Trees whose rebuild waits for the event loop's next turn.
    static TREES_TO_BUILD: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Each tree's in-place editor, and the node it edits.
    static TREE_EDITORS: RefCell<HashMap<String, (Input, Option<usize>)>> = RefCell::new(HashMap::new());
    /// Bumped by every click on a tree: a click on the selected node starts
    /// an edit a moment later unless another click (a double click) came.
    static TREE_CLICKS: Cell<u64> = const { Cell::new(0) };
}

/// A QTREEVIEW: an FLTK tree showing rapidr_value::objects::tree's nodes.
/// What the user does asks the program first — OnChanging (Index,
/// AllowChange), OnExpanding / OnCollapsing (Index, Allow…) — and changes
/// the nodes if it may, then OnChange / OnExpanded / OnCollapsed; OnClick
/// and OnDblClick follow a click.
fn tree_create(name: &str, x: i32, y: i32, w: i32, h: i32) {
    let mut tree = Tree::new(x, y, w, h, None);
    // (a flat white box, as the lists: the theme shades the empty part)
    tree.set_frame(FrameType::BorderBox);
    tree.set_color(Color::White);
    tree.set_show_root(false);
    tree.set_select_mode(fltk::tree::TreeSelect::Single);
    // (a click on the selected node calls back too: it starts an edit)
    tree.set_item_reselect_mode(fltk::tree::TreeItemReselectMode::Always);
    let cb_name = name.to_string();
    tree.set_callback(move |t| tree_callback(&cb_name, t));
    // F2 edits the selected node's text.
    let key_name = name.to_string();
    tree.super_handle_first(false);
    tree.handle(move |_, ev| {
        // HideSelection: the selection shows only while the tree has focus.
        if matches!(ev, Event::Focus | Event::Unfocus) && rapidr_value::objects::with_tree(&key_name, |m| m.hide_selection).unwrap_or(false) {
            let name = key_name.clone();
            app::add_timeout3(0.0, move |_| tree_sync(&name));
        }
        if ev == Event::KeyDown && app::event_key() == Key::F2 {
            if let Some(i) = rapidr_value::objects::with_tree(&key_name, |m| usize::try_from(m.item_index).ok()).flatten() {
                tree_begin_edit(&key_name, i);
            }
            return true;
        }
        false
    });
    // The in-place editor: a sibling over the node's text, drawn after it.
    let mut editor = Input::new(0, 0, 0, 0, None);
    editor.set_frame(FrameType::BorderBox);
    editor.hide();
    let enter_name = name.to_string();
    editor.set_trigger(CallbackTrigger::EnterKeyAlways);
    editor.set_callback(move |_| tree_end_edit(&enter_name, true));
    // Leaving it keeps the edit, as Windows does; Escape drops it.
    let edit_name = name.to_string();
    editor.super_handle_first(false);
    editor.handle(move |_, ev| match ev {
        Event::Unfocus => {
            tree_end_edit(&edit_name, true);
            false
        }
        Event::KeyDown if app::event_key() == Key::Escape => {
            tree_end_edit(&edit_name, false);
            true
        }
        _ => false,
    });
    TREE_EDITORS.with(|e| e.borrow_mut().insert(name.to_string(), (editor, None)));
    GUI_WIDGETS.with(|gw| {
        gw.borrow_mut().insert(name.to_string(), GuiWidget::Tree(tree));
    });
    TREE_SHAPES.with(|s| s.borrow_mut().remove(name));
    tree_refresh(name);
}

/// The node an FLTK item shows: its place in FLTK's depth-first order
/// (the model's), the hidden root not counted.
fn tree_index_of(t: &Tree, item: &fltk::tree::TreeItem) -> Option<usize> {
    let mut it = t.first()?.next();
    let mut k = 0;
    while let Some(x) = it {
        if x == *item {
            return Some(k);
        }
        k += 1;
        it = x.next();
    }
    None
}

fn tree_items(t: &Tree) -> Vec<fltk::tree::TreeItem> {
    let mut items = Vec::new();
    let mut it = t.first().and_then(|r| r.next());
    while let Some(x) = it {
        it = x.next();
        items.push(x);
    }
    items
}

fn tree_callback(name: &str, t: &mut Tree) {
    use fltk::tree::TreeReason;
    let Some(item) = t.callback_item() else { return };
    let Some(i) = tree_index_of(t, &item) else { return };
    let clicked = matches!(app::event(), Event::Push | Event::Released);
    if clicked {
        TREE_CLICKS.with(|c| c.set(c.get() + 1));
    }
    match t.callback_reason() {
        // A click on the node already selected: an edit, unless it is
        // (or becomes) a double click.
        TreeReason::Reselected if clicked => tree_reselected(name, i, fltk_double()),
        TreeReason::Selected | TreeReason::Reselected => {
            tree_user_select(name, i);
            if clicked {
                rp_fire_event(name, "onclick");
                if app::event_clicks() {
                    rp_fire_event(name, "ondblclick");
                }
            }
        }
        reason @ (TreeReason::Opened | TreeReason::Closed) => {
            if clicked {
                rp_fire_event(name, "onclick");
            }
            tree_user_toggle(name, i, reason == TreeReason::Opened);
        }
        // A click on no node leaves the selection as it was (Windows).
        TreeReason::Deselected => tree_refresh(name),
        _ => {}
    }
}

/// A click on node `i`, the one selected: OnClick, then OnDblClick for a
/// double click — else its edit after a pause (Windows' double-click time),
/// unless another click comes first.
fn tree_reselected(name: &str, i: usize, double: bool) {
    rp_fire_event(name, "onclick");
    if double {
        rp_fire_event(name, "ondblclick");
        return;
    }
    let click = TREE_CLICKS.with(Cell::get);
    let tree = name.to_string();
    app::add_timeout3(0.5, move |_| {
        if TREE_CLICKS.with(Cell::get) == click {
            tree_begin_edit(&tree, i);
        }
    });
}

/// The user picked node `i` (FLTK shows it picked already): OnChanging may
/// refuse (the old one shows again); then OnChange.
fn tree_user_select(name: &str, i: usize) {
    if rapidr_value::objects::with_tree(name, |m| m.item_index) == Some(i as i64) {
        return;
    }
    let tree = name.to_string();
    rp_fire_event_then(name, "onchanging", &[v_int(i as i64), v_int(-1)], move |a| {
        let allowed = a[1].to_i64() != 0;
        if allowed {
            rapidr_value::objects::with_tree(&tree, |m| m.select(i as i64));
        }
        tree_refresh(&tree);
        if allowed {
            rp_fire_event_1(&tree, "onchange", v_int(i as i64));
        }
    });
}

/// The user expanded (`open`) or collapsed node `i`: OnExpanding /
/// OnCollapsing may refuse; then OnExpanded / OnCollapsed.
fn tree_user_toggle(name: &str, i: usize, open: bool) {
    let tree = name.to_string();
    rp_fire_event_then(name, if open { "onexpanding" } else { "oncollapsing" }, &[v_int(i as i64), v_int(-1)], move |a| {
        let allowed = a[1].to_i64() != 0;
        if allowed {
            rapidr_value::objects::with_tree(&tree, |m| m.set_expanded(i, open, false));
        }
        tree_refresh(&tree);
        if allowed {
            rp_fire_event_1(&tree, if open { "onexpanded" } else { "oncollapsed" }, v_int(i as i64));
        }
    });
}

/// The user starts editing node `i`'s text (F2, or a click on the selected
/// node): not in a ReadOnly tree; OnEditing(Index, AllowEdit) may refuse;
/// then an editor over the node's text.
fn tree_begin_edit(name: &str, i: usize) {
    if rapidr_value::objects::with_tree(name, |m| m.read_only || i >= m.nodes.len()).unwrap_or(true) {
        return;
    }
    let tree = name.to_string();
    rp_fire_event_then(name, "onediting", &[v_int(i as i64), v_int(-1)], move |a| {
        if a[1].to_i64() == 0 {
            return;
        }
        let Some(GuiWidget::Tree(mut t)) = GUI_WIDGETS.with(|gw| gw.borrow().get(&tree).cloned()) else { return };
        let Some(item) = tree_items(&t).get(i).cloned() else { return };
        let Some(text) = rapidr_value::objects::with_tree(&tree, |m| m.nodes.get(i).map(|n| n.text.clone())).flatten() else { return };
        t.show_item_middle(&item);
        t.redraw();
        app::flush();
        let (x, y, h) = (item.label_x(), item.label_y(), item.label_h().max(t.item_label_size() + 6));
        let w = (t.x() + t.w() - x - 3).max(40);
        TREE_EDITORS.with(|e| {
            if let Some((editor, editing)) = e.borrow_mut().get_mut(&tree) {
                *editing = Some(i);
                editor.resize(x, y - 1, w, h + 2);
                editor.set_value(&text);
                editor.show();
                let _ = editor.take_focus();
                let _ = editor.set_position(text.len() as i32);
                let _ = editor.set_mark(0);
                editor.redraw();
            }
        });
    });
}

/// Ends an edit: with `keep`, OnEdited(Index, S) — S the text, which the
/// program may change — and the node gets it.
fn tree_end_edit(name: &str, keep: bool) {
    let ended = TREE_EDITORS.with(|e| {
        let mut editors = e.borrow_mut();
        let (editor, editing) = editors.get_mut(name)?;
        let i = editing.take()?;
        let text = editor.value();
        editor.hide();
        Some((i, text))
    });
    let Some((i, text)) = ended else { return };
    if let Some(GuiWidget::Tree(mut t)) = GUI_WIDGETS.with(|gw| gw.borrow().get(name).cloned()) {
        let _ = t.take_focus();
        t.redraw();
    }
    if !keep {
        return;
    }
    let tree = name.to_string();
    rp_fire_event_then(name, "onedited", &[v_int(i as i64), v_str(&text)], move |a| {
        let text = a[1].to_string_val();
        rapidr_value::objects::with_tree(&tree, |m| m.set_text(i, text));
        tree_refresh(&tree);
    });
}

/// Shows the tree's nodes: expanded, selected and icons at once on the
/// items there are; new texts or levels rebuild the items on the event
/// loop's next turn (never inside the tree's own callback). Fires
/// OnDeletion for nodes the program deleted.
pub fn tree_refresh(name: &str) {
    let name = name.to_lowercase();
    for i in rapidr_value::objects::with_tree(&name, |m| m.take_deleted()).unwrap_or_default() {
        rp_fire_event_1(&name, "ondeletion", v_int(i as i64));
    }
    let Some(shape) = rapidr_value::objects::with_tree(&name, |m| m.shape_hash()) else { return };
    if TREE_SHAPES.with(|s| s.borrow().get(&name) != Some(&shape)) {
        let first = TREES_TO_BUILD.with(|b| {
            let mut b = b.borrow_mut();
            let first = b.is_empty();
            if !b.contains(&name) {
                b.push(name.clone());
            }
            first
        });
        if first {
            app::add_timeout3(0.0, |_| {
                for name in TREES_TO_BUILD.with(|b| std::mem::take(&mut *b.borrow_mut())) {
                    tree_build(&name);
                }
            });
        }
        return;
    }
    tree_sync(&name);
}

/// Builds the widget's items from the nodes.
fn tree_build(name: &str) {
    let Some(GuiWidget::Tree(mut t)) = GUI_WIDGETS.with(|gw| gw.borrow().get(name).cloned()) else { return };
    let Some((nodes, shape)) = rapidr_value::objects::with_tree(name, |m| (m.nodes.iter().map(|n| (n.text.clone(), n.level)).collect::<Vec<_>>(), m.shape_hash())) else { return };
    let Some(root) = t.root() else { return };
    t.clear_children(&root);
    let mut parents: Vec<fltk::tree::TreeItem> = Vec::new();
    for (text, level) in nodes {
        parents.truncate(level);
        let parent = parents.last().cloned().unwrap_or_else(|| root.clone());
        if let Some(item) = t.insert(&parent, &text, parent.children()) {
            parents.push(item);
        }
    }
    TREE_SHAPES.with(|s| s.borrow_mut().insert(name.to_string(), shape));
    tree_sync(name);
}

thread_local! {
    /// Trees asking their program for icons (OnGetImageIndex): what that
    /// changes shows without asking again.
    static TREES_ASKING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    /// What each tree showed when it last asked (`TreeView::view_hash`).
    static TREES_ASKED: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
}

/// OnGetImageIndex (Index) for each shown node, OnGetSelectedIndex (Index)
/// for the selected one: the program sets `Item(Index).ImageIndex` /
/// `.SelectedIndex` there, as Windows asks when it draws a node.
fn tree_ask_images(name: &str) {
    let ask = |e: &str| crate::object::rp_has_handler(name, e);
    let key = name.to_lowercase();
    let Some(view) = rapidr_value::objects::with_tree(name, |m| m.view_hash()) else { return };
    if !(ask("ongetimageindex") || ask("ongetselectedindex")) || TREES_ASKED.with(|a| a.borrow().get(&key) == Some(&view)) {
        return;
    }
    if !TREES_ASKING.with(|a| a.borrow_mut().insert(key.clone())) {
        return;
    }
    TREES_ASKED.with(|a| a.borrow_mut().insert(key.clone(), view));
    let (rows, selected) = rapidr_value::objects::with_tree(name, |m| (m.visible_rows(), m.item_index)).unwrap_or_default();
    for i in rows {
        let event = if i as i64 == selected { "ongetselectedindex" } else { "ongetimageindex" };
        rp_fire_event_1(name, event, v_int(i as i64));
    }
    TREES_ASKING.with(|a| a.borrow_mut().remove(&key));
}

/// Expanded, selected, icons and looks from the nodes onto the items.
fn tree_sync(name: &str) {
    use fltk::tree::TreeConnectorStyle;
    tree_ask_images(name);
    let Some(GuiWidget::Tree(mut t)) = GUI_WIDGETS.with(|gw| gw.borrow().get(name).cloned()) else { return };
    let Some((flags, selected, show, hide)) = rapidr_value::objects::with_tree(name, |m| {
        let flags: Vec<(bool, i64, i64, i64)> = m.nodes.iter().map(|n| (n.expanded, n.image_index, n.selected_index, n.state_index)).collect();
        (flags, m.item_index, (m.show_lines, m.show_buttons, m.indent), m.hide_selection)
    }) else {
        return;
    };
    // (HideSelection: none shown while the tree hasn't focus)
    let focused = app::focus().is_some_and(|f| f.as_widget_ptr() == t.as_widget_ptr());
    let shown_selected = if hide && !focused { -1 } else { selected };
    let images = rp_comp_get(name, "images").to_string_val();
    let state_images = rp_comp_get(name, "stateimages").to_string_val();
    t.set_connector_style(if show.0 { TreeConnectorStyle::Dotted } else { TreeConnectorStyle::None });
    t.set_show_collapse(show.1);
    t.set_connector_width(show.2.clamp(4, 200) as i32);
    let items = tree_items(&t);
    for (k, mut item) in items.iter().cloned().enumerate() {
        let Some(&(expanded, image, selected_image, state)) = flags.get(k) else { break };
        if item.has_children() {
            if expanded { item.open() } else { item.close() }
        }
        if !images.is_empty() || !state_images.is_empty() {
            let i = if k as i64 == selected { selected_image } else { image };
            let icon = rapidr_value::objects::tree_icon(&images, &state_images, i, state).and_then(|(w, h, rgba, scale)| display_image(w, h, &rgba, scale));
            item.set_user_icon(icon);
        }
    }
    match usize::try_from(shown_selected).ok().and_then(|i| items.get(i)) {
        Some(item) => {
            let _ = t.select_only(item, false);
        }
        None => {
            if let Some(root) = t.root() {
                let _ = t.deselect_all(&root, false);
            }
        }
    }
    t.redraw();
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
    // (the raw number: fltk's `event_mouse_button` panics when no button
    // is down, as on a mouse move)
    match app::event_button() {
        3 => 1,
        2 => 2,
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
            // (the input lane's: OnDblClick before a double click's second
            // OnMouseDown, OnClick before a single click's OnMouseUp)
            Event::Push => {
                press_begin(&name);
                vcl_clicks(&name, rapidr_value::input::Mouse::Down, button_of(mouse_button()), (fltk_double(), true), || {
                    crate::object::rp_fire_event_args(&name, "onmousedown", &[v_int(mouse_button()), x, y, v_int(mouse_shift())]);
                });
                true
            }
            Event::Released => {
                if press_end(&name) {
                    vcl_clicks(&name, rapidr_value::input::Mouse::Up, button_of(mouse_button()), (fltk_double(), app::event_inside_widget(f)), || {
                        crate::object::rp_fire_event_args(&name, "onmouseup", &[v_int(mouse_button()), x, y, v_int(mouse_shift())]);
                    });
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

/// An image of what a bitmap shows (rapidr_value's `display_rgba`: `w` ×
/// `h` device pixels, `scale` of them a pixel), sized in pixels: on a
/// high-DPI screen FLTK draws every device pixel, so it isn't blurry.
/// The screen's scale where `w` is shown (device pixels per pixel: 2 on a
/// Retina screen), for bitmaps to keep what they show at it
/// (rapidr_value::objects::bitmap); `RAPIDR_SCALE` sets it (tests).
fn note_display_scale(w: &impl WidgetExt) {
    let scale = std::env::var("RAPIDR_SCALE").ok().and_then(|s| s.parse::<f64>().ok()).or_else(|| {
        let top = w.top_window()?.as_widget_ptr();
        GUI_WIDGETS.with(|gw| {
            gw.try_borrow().ok()?.values().find_map(|x| match x {
                GuiWidget::Window(win) if win.as_widget_ptr() == top => Some(f64::from(win.pixels_per_unit())),
                _ => None,
            })
        })
    });
    if let Some(s) = scale.filter(|s| *s > 0.0) {
        rapidr_value::objects::bitmap::set_display_scale(s);
    }
}

fn display_image(w: usize, h: usize, rgba: &[u8], scale: usize) -> Option<RgbImage> {
    let mut img = RgbImage::new(rgba, w as i32, h as i32, ColorDepth::Rgba8).ok()?;
    if scale > 1 {
        img.scale((w / scale) as i32, (h / scale) as i32, false, true);
    }
    Some(img)
}

/// Shows a QIMAGE's picture (rapidr_value::objects, a Bitmap): at the top
/// left, centered (Center) or scaled to the control (Stretch); with
/// Transparent, pixels of the transparent color show what's behind.
/// An image without a picture keeps what it shows (a PNG loaded by FLTK).
pub fn picture_refresh(name: &str) {
    let name_lower = name.to_lowercase();
    let Some(Some((pw, ph, rgba, scale, transparent))) = rapidr_value::objects::with_picture(&name_lower, |b| {
        let transparent = b.transparent || b.alpha_channel().is_some();
        (!b.img.pixels.is_empty()).then(|| {
            let (pw, ph, rgba, scale) = b.display_rgba();
            (pw, ph, rgba, scale, transparent)
        })
    }) else {
        return;
    };
    let stretch = rp_comp_get(&name_lower, "stretch").to_bool();
    let center = rp_comp_get(&name_lower, "center").to_bool();
    GUI_WIDGETS.with(|gw| {
        let Ok(mut widgets) = gw.try_borrow_mut() else { return };
        let Some(GuiWidget::ImageFrame(frm)) = widgets.get_mut(&name_lower) else { return };
        let Some(img) = display_image(pw, ph, &rgba, scale) else { return };
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
        if let Ok(mut b) = rapidr_value::objects::load_image(&v_str(src)) {
            let (pw, ph, rgba, scale) = b.display_rgba();
            return SharedImage::from_image(&display_image(pw, ph, &rgba, scale)?).ok();
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
    // (RapidR's handler first: it returns true for what it handles alone)
    f.super_handle_first(false);
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
                GuiWidget::MenuBar(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::SysMenuBar(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Progress(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Grid(ref mut w, _) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Tree(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Slider(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::ImageFrame(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
            }
        }
    });
    // (a form shown this way: its scale, its first OnPaint)
    if visible {
        after_show(&name_lower);
    }
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

thread_local! {
    /// The scroll bars held down: their container, its client origin in
    /// the window, the mouse's last place there.
    static SCROLL_CAPTURE: RefCell<Option<(String, i32, i32, i64, i64)>> = const { RefCell::new(None) };
}

/// Draws `name`'s scroll bars (rapidr_value::scrollbars) with its client
/// area at (ox, oy): after its components, so they're always on top.
fn scroll_bars_draw(name: &str, ox: i32, oy: i32, _w: i32, _h: i32) {
    let (w, h) = crate::scroll::area(name);
    let Some(ops) = rapidr_value::scrollbars::with(name, |s| (s.vert.shown || s.horz.shown).then(|| s.ops(w, h))).flatten() else { return };
    draw_fill_ops(&ops, ox, oy);
}

/// Fills and arrows (rapidr_value's drawing ops) at (ox, oy).
fn draw_fill_ops(ops: &[rapidr_value::objects::tabcontrol::Op], ox: i32, oy: i32) {
    use rapidr_value::objects::tabcontrol::Op;
    for op in ops {
        match op {
            Op::Fill { rect: (x, y, rw, rh), color } => draw::draw_rect_fill(ox + *x as i32, oy + *y as i32, *rw as i32, *rh as i32, Color::from_hex(*color)),
            Op::Arrow { points, color } => {
                draw::set_draw_color(Color::from_hex(*color));
                draw::begin_polygon();
                for (px, py) in points {
                    draw::vertex(f64::from(ox) + px, f64::from(oy) + py);
                }
                draw::end_polygon();
            }
            _ => {}
        }
    }
}

/// The scrolling containers in window `win` with their client area's
/// origin there, innermost (smallest) first.
fn scrollers_in(win: app::WindowPtr) -> Vec<(String, i32, i32)> {
    let mut found: Vec<(i64, String, i32, i32)> = Vec::new();
    if let Some(form) = window_component(win) {
        found.push((i64::MAX, form.clone(), 0, menu_offset(&form)));
    }
    GUI_WIDGETS.with(|gw| {
        let Ok(gw) = gw.try_borrow() else { return };
        for (name, widget) in gw.iter() {
            let GuiWidget::Group(g) = widget else { continue };
            if !g.visible_r() || rp_comp_type(name) != "RSCROLLBOX" || !g.window().is_some_and(|w| w.as_widget_ptr() as usize == win as usize) {
                continue;
            }
            let b = crate::scroll::border(name) as i32;
            found.push((i64::from(g.w()) * i64::from(g.h()), name.clone(), g.x() + b, g.y() + b));
        }
    });
    found.sort_by_key(|f| f.0);
    found.into_iter().map(|(_, n, x, y)| (n, x, y)).collect()
}

/// A press, drag or release on a form's / scroll box's bars: `Some` when
/// the bars took it.
fn scroll_bars_event(ev: Event, win: app::WindowPtr) -> Option<bool> {
    match ev {
        Event::Push => {
            for (name, ox, oy) in scrollers_in(win) {
                let (w, h) = crate::scroll::area(&name);
                let (x, y) = ((app::event_x() - ox) as i64, (app::event_y() - oy) as i64);
                let Some(shift) = rapidr_value::scrollbars::with(&name, |s| s.on_bars(x, y, w, h))
                    .filter(|on| *on)
                    .and_then(|_| rapidr_value::scrollbars::with_mut(&name, |s| s.mouse_down(x, y, w, h)))
                else {
                    continue;
                };
                crate::scroll::user_scrolled(&name, shift);
                SCROLL_CAPTURE.with(|c| *c.borrow_mut() = Some((name.clone(), ox, oy, x, y)));
                // (an arrow or the track held down repeats, as Windows')
                app::add_timeout3(0.4, move |handle| {
                    let Some((held, _, _, mx, my)) = SCROLL_CAPTURE.with(|c| c.borrow().clone()) else { return };
                    if held != name {
                        return;
                    }
                    let (w, h) = crate::scroll::area(&name);
                    let shift = rapidr_value::scrollbars::with_mut(&name, |s| s.repeat(mx, my, w, h));
                    crate::scroll::user_scrolled(&name, shift);
                    app::repeat_timeout3(0.05, handle);
                });
                return Some(true);
            }
            None
        }
        Event::Drag => {
            let (name, ox, oy, _, _) = SCROLL_CAPTURE.with(|c| c.borrow().clone())?;
            let (x, y) = ((app::event_x() - ox) as i64, (app::event_y() - oy) as i64);
            SCROLL_CAPTURE.with(|c| *c.borrow_mut() = Some((name.clone(), ox, oy, x, y)));
            let (w, h) = crate::scroll::area(&name);
            let shift = rapidr_value::scrollbars::with_mut(&name, |s| s.mouse_drag(x, y, w, h));
            crate::scroll::user_scrolled(&name, shift);
            Some(true)
        }
        Event::Released => {
            let (name, ..) = SCROLL_CAPTURE.with(|c| c.borrow_mut().take())?;
            let (w, h) = crate::scroll::area(&name);
            let shift = rapidr_value::scrollbars::with_mut(&name, |s| s.mouse_up(w, h));
            crate::scroll::user_scrolled(&name, shift);
            Some(true)
        }
        _ => None,
    }
}

/// The test hooks' mouse on a form's / scroll box's bars (`x`, `y` in its
/// widget, as the web's): whether the bars took it.
fn scroll_bars_hook(name: &str, kind: rapidr_value::input::Mouse, x: i64, y: i64) -> bool {
    use rapidr_value::input::Mouse;
    if !crate::scroll::scrolls(name) {
        return false;
    }
    let b = crate::scroll::border(name);
    let (x, y) = (x - b, y - b);
    let (w, h) = crate::scroll::area(name);
    let held = SCROLL_CAPTURE.with(|c| c.borrow().as_ref().is_some_and(|(n, ..)| n == name));
    let shift = match kind {
        Mouse::Down => {
            let Some(shift) = rapidr_value::scrollbars::with_mut(name, |s| s.mouse_down(x, y, w, h)) else { return false };
            SCROLL_CAPTURE.with(|c| *c.borrow_mut() = Some((name.to_string(), 0, 0, x, y)));
            shift
        }
        Mouse::Move if held => rapidr_value::scrollbars::with_mut(name, |s| s.mouse_drag(x, y, w, h)),
        Mouse::Up if held => {
            SCROLL_CAPTURE.with(|c| c.borrow_mut().take());
            rapidr_value::scrollbars::with_mut(name, |s| s.mouse_up(w, h))
        }
        _ => return false,
    };
    crate::scroll::user_scrolled(name, shift);
    true
}

/// The wheel over a form / scroll box with a bar: it scrolls.
fn scroll_wheel(win: app::WindowPtr) -> bool {
    let (vertical, notches) = match (app::event_dy(), app::event_dx()) {
        (app::MouseWheel::Up, _) => (true, -1),
        (app::MouseWheel::Down, _) => (true, 1),
        (_, app::MouseWheel::Left) => (false, -1),
        (_, app::MouseWheel::Right) => (false, 1),
        _ => return false,
    };
    let horizontal = !vertical || app::event_state().contains(fltk::enums::Shortcut::Shift);
    for (name, ox, oy) in scrollers_in(win) {
        let (w, h) = crate::scroll::area(&name);
        let (x, y) = ((app::event_x() - ox) as i64, (app::event_y() - oy) as i64);
        if x < 0 || y < 0 || x >= w || y >= h {
            continue;
        }
        let shift = rapidr_value::scrollbars::with_mut(&name, |s| s.wheel(notches, horizontal, w, h));
        if shift != (0, 0) {
            crate::scroll::user_scrolled(&name, shift);
            return true;
        }
    }
    false
}

/// The test hooks' keys and mouse on a QTRACKBAR (as its widget's handler):
/// `f` changes the model (given the widget's size); OnChange if it moved.
fn trackbar_input(name: &str, f: impl FnOnce(&mut rapidr_value::objects::trackbar::TrackBar, f64, f64) -> bool) {
    let Some(mut w) = GUI_WIDGETS.with(|gw| gw.borrow().get(name).map(GuiWidget::base)) else { return };
    let (ww, wh) = (w.w() as f64, w.h() as f64);
    if rapidr_value::objects::with_trackbar_mut(name, |t| f(t, ww, wh)) == Some(true) {
        w.redraw();
        rp_fire_event(name, "onchange");
    }
}

/// The test hooks' keys and clicks on a QTABCONTROL (as its tabs' handler):
/// `f` changes the model; OnChange if the selection changed.
fn tab_control_input(name: &str, f: impl FnOnce(&mut rapidr_value::objects::tabcontrol::TabControl, i64, i64, &rapidr_value::objects::font::Font) -> bool) {
    if !rapidr_value::objects::is_tabcontrol(name) {
        return;
    }
    let (w, h) = (rp_comp_get(name, "width").to_i64(), rp_comp_get(name, "height").to_i64());
    let font = tab_control_font(name);
    if rapidr_value::objects::with_tabcontrol_mut(name, |t| f(t, w, h, &font)) == Some(true) {
        redraw_widget(name);
        rp_fire_event(name, "onchange");
    }
}

thread_local! {
    /// While the program's text is put in a widget (its own changes aren't
    /// the user's), and the model revision each widget shows.
    static TEXT_PUSHING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static TEXT_SHOWN: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
    /// The GotoLine / GotoSub each code editor scrolled to last.
    static TEXT_REVEALED: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
}

fn char_to_byte(s: &str, c: usize) -> i32 {
    s.char_indices().nth(c).map_or(s.len(), |(b, _)| b) as i32
}

fn byte_to_char(s: &str, b: i32) -> usize {
    let b = (b.max(0) as usize).min(s.len());
    s.char_indices().take_while(|(i, _)| *i < b).count()
}

/// A QEDIT's / QRICHEDIT's widget text and selection copied to its model
/// (rapidr_value::objects::textedit), before the program reads them.
pub fn text_pull(name: &str) {
    let name = name.to_lowercase();
    let Some(widget) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(&name).cloned())) else { return };
    let (text, start, len) = match widget {
        GuiWidget::Input(inp) => {
            let text = inp.value();
            let (a, b) = (inp.position().min(inp.mark()), inp.position().max(inp.mark()));
            let (ca, cb) = (byte_to_char(&text, a), byte_to_char(&text, b));
            (text, ca, cb - ca)
        }
        GuiWidget::TextEditor(ed) => {
            let Some(buf) = GUI_TEXT_BUFFERS.with(|tb| tb.borrow().get(&name).cloned()) else { return };
            let text = buf.text();
            let (a, b) = buf.selection_position().unwrap_or((ed.insert_position(), ed.insert_position()));
            let (ca, cb) = (byte_to_char(&text, a.min(b)), byte_to_char(&text, a.max(b)));
            (text, ca, cb - ca)
        }
        _ => return,
    };
    rapidr_value::objects::with_textedit_mut(&name, |t| t.user_edit(&text, start, len));
}

/// A QEDIT's / QRICHEDIT's model shown in its widget again, if the program
/// changed it since (text, selection, ReadOnly).
pub fn text_push(name: &str) {
    let name = name.to_lowercase();
    let Some((rev, raw, start, len, read_only, reveal)) = rapidr_value::objects::with_textedit(&name, |t| (t.revision, t.raw(), t.sel_start, t.sel_len, t.read_only, t.reveal)) else { return };
    if TEXT_SHOWN.with(|s| s.borrow().get(&name) == Some(&rev)) {
        return;
    }
    let Some(widget) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(&name).cloned())) else { return };
    // (an RCODEEDITOR's GotoLine / GotoSub: the caret scrolled into view)
    let revealed = TEXT_REVEALED.with(|r| r.borrow_mut().insert(name.clone(), reveal)).unwrap_or(0) != reveal;
    TEXT_SHOWN.with(|s| s.borrow_mut().insert(name.clone(), rev));
    let (a, b) = (char_to_byte(&raw, start), char_to_byte(&raw, start + len));
    TEXT_PUSHING.with(|p| p.set(true));
    match widget {
        GuiWidget::Input(mut inp) => {
            if inp.value() != raw {
                inp.set_value(&raw);
            }
            let _ = inp.set_position(b);
            let _ = inp.set_mark(a);
            inp.set_readonly(read_only);
        }
        GuiWidget::TextEditor(mut ed) => {
            if let Some(mut buf) = GUI_TEXT_BUFFERS.with(|tb| tb.borrow().get(&name).cloned()) {
                if buf.text() != raw {
                    buf.set_text(&raw);
                }
                if a == b {
                    buf.unselect();
                } else {
                    buf.select(a, b);
                }
            }
            ed.set_insert_position(b);
            if revealed {
                ed.show_insert_position();
            }
        }
        _ => {}
    }
    TEXT_PUSHING.with(|p| p.set(false));
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
    // (the input lane's: the size grip, the boxes ending before it)
    let grip = status_grip(id);
    if grip {
        draw_size_grip(x, y, w, h);
    }
    let w = if grip { w - rapidr_value::layout::STATUS_GRIP as i32 } else { w };
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

// ------------------------------------------------- (the input lane's) --

thread_local! {
    /// A QSTATUSBAR's size grip held: its form, and the mouse's offset from
    /// the window's inside's bottom-right.
    static GRIP_HELD: RefCell<Option<(String, i32, i32)>> = const { RefCell::new(None) };
}

/// Whether QSTATUSBAR `name` shows its size grip (Delphi's SizeGrip, True
/// unless set: on a sizeable form, docked at its bottom —
/// `rapidr_value::layout::status_grip`, as the kernel host's).
fn status_grip(name: &str) -> bool {
    let parent = rp_comp_get(name, "parent").to_string_val();
    let on_form = matches!(rp_comp_type(&parent).to_ascii_uppercase().as_str(), "RFORM" | "RFORMMDI");
    let size_grip = match rp_comp_get(name, "sizegrip") {
        Value::Null => true,
        v => v.to_bool(),
    };
    let border = match rp_comp_get(&parent, "borderstyle") {
        Value::Null => 2,
        v => v.to_i64(),
    };
    rapidr_value::layout::status_grip(size_grip, on_form, border, crate::layout::align_of(name))
}

/// Windows' classic size grip at the bottom-right of a bar at (x, y), `w`
/// × `h`: three raised ridges, each a white line over two grey ones.
fn draw_size_grip(x: i32, y: i32, w: i32, h: i32) {
    let g = rapidr_value::layout::STATUS_GRIP as i32;
    let (cx, cy) = (x + w - 1, y + h - 1);
    for base in [1, 5, 9] {
        for (d, color) in [(base, Color::from_rgb(128, 128, 128)), (base + 1, Color::from_rgb(128, 128, 128)), (base + 2, Color::White)] {
            draw::set_draw_color(color);
            for k in 0..=d {
                let (px, py) = (cx - d + k, cy - k);
                if px >= x + w - g + 2 && py >= y + h - g + 2 {
                    draw::draw_point(px, py);
                }
            }
        }
    }
}

/// A press at (x, y) of status bar `name`'s widget: on its size grip, the
/// window's resize begins. Whether it was.
fn grip_press(name: &str, x: i32, y: i32) -> bool {
    let Some(bar) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(name).map(GuiWidget::base))) else { return false };
    let g = rapidr_value::layout::STATUS_GRIP as i32;
    if x < bar.w() - g || y < bar.h() - g || x >= bar.w() || y >= bar.h() || !status_grip(name) {
        return false;
    }
    // (docked at the bottom, its full width: its bottom-right is the window's inside's)
    let form = rp_comp_get(name, "parent").to_string_val().to_lowercase();
    GRIP_HELD.with(|held| *held.borrow_mut() = Some((form, bar.w() - x, bar.h() - y)));
    true
}

/// The mouse at (x, y) of the window's inside with a grip held: the window
/// resized with it, as the user's drag of its border (`form_resized`:
/// Width / Height, OnResize). Whether one is held.
fn grip_drag(x: i32, y: i32) -> bool {
    let Some((form, ox, oy)) = GRIP_HELD.with(|held| held.borrow().clone()) else { return false };
    if let Some(GuiWidget::Window(mut win)) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(&form).cloned())) {
        let (wx, wy) = (win.x(), win.y());
        win.resize(wx, wy, (x + ox).max(1), (y + oy).max(1));
    }
    true
}

/// A press, drag or release on a status bar's size grip: `Some` when the
/// grip took it (nothing else hears it).
fn size_grip_event(ev: Event, win: app::WindowPtr) -> Option<bool> {
    match ev {
        Event::Push => {
            let (ex, ey) = (app::event_x(), app::event_y());
            let bars: Vec<(String, i32, i32)> = GUI_WIDGETS.with(|gw| {
                let Ok(gw) = gw.try_borrow() else { return Vec::new() };
                gw.iter()
                    .filter(|(n, w)| {
                        let w = w.base();
                        w.visible_r() && rp_comp_type(n) == "RSTATUSBAR" && w.window().is_some_and(|ww| ww.as_widget_ptr() as usize == win as usize)
                    })
                    .map(|(n, w)| (n.clone(), w.base().x(), w.base().y()))
                    .collect()
            });
            bars.into_iter().any(|(n, bx, by)| grip_press(&n, ex - bx, ey - by)).then_some(true)
        }
        Event::Drag => grip_drag(app::event_x(), app::event_y()).then_some(true),
        Event::Released => GRIP_HELD.with(|held| held.borrow_mut().take()).map(|_| true),
        _ => None,
    }
}

/// The test hooks' mouse on a status bar's size grip (`x`, `y` in the
/// bar): whether the grip took it, as the real input's.
fn grip_hook(name: &str, kind: rapidr_value::input::Mouse, x: i64, y: i64) -> bool {
    use rapidr_value::input::Mouse;
    match kind {
        Mouse::Down => rp_comp_type(name) == "RSTATUSBAR" && grip_press(name, x as i32, y as i32),
        Mouse::Move => {
            let Some(bar) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(name).map(GuiWidget::base))) else { return false };
            grip_drag(bar.x() + x as i32, bar.y() + y as i32)
        }
        Mouse::Up => GRIP_HELD.with(|held| held.borrow_mut().take()).is_some(),
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
        // An owner-drawn or multi-column list box: a table with a row per
        // item, each as tall as OnMeasureItem said (lbOwnerDrawVariable),
        // or the items down its columns (Columns).
        Some(GuiWidget::Grid(mut t, _)) => {
            let multi = rapidr_value::objects::with_list(&name, |l| l.multi_column()).unwrap_or(false);
            // What shows: the table's inside less a scroll bar.
            let (vw, vh) = if multi { (t.w() - 4, t.h() - 4 - 16) } else { (t.w() - 4 - 16, t.h() - 4) };
            rapidr_value::objects::with_list_mut(&name, |l| l.set_view(i64::from(vw), i64::from(vh)));
            if list_measure(&name) {
                return;
            }
            let Some((per, cw, rh, heights)) = rapidr_value::objects::with_list(&name, |l| {
                let (per, cw) = l.column_layout();
                (per, cw, l.row_height(), (0..l.items.len()).map(|i| l.item_h(i) as i32).collect::<Vec<_>>())
            }) else {
                return;
            };
            if multi {
                let n = items.len() as i64;
                t.set_rows(per.min(n.max(1)) as i32);
                t.set_cols(((n + per - 1) / per) as i32);
                t.set_row_height_all(rh as i32);
                t.set_col_width_all(cw as i32);
            } else {
                t.set_rows(items.len() as i32);
                t.set_cols(1);
                for (i, h) in heights.iter().enumerate() {
                    t.set_row_height(i as i32, *h);
                }
                t.set_col_width_all(vw.max(10));
            }
            if top > 0 {
                t.set_row_position(top as i32);
            }
            t.redraw();
            list_owner_draw(&name);
        }
        // An owner-drawn combo box (`owner_combo_create`): its items are
        // as wide as its drop-down.
        Some(GuiWidget::Frame(mut f)) if rapidr_value::objects::with_list(&name, |l| l.combo && l.owner_drawn()).unwrap_or(false) => {
            rapidr_value::objects::with_list_mut(&name, |l| l.set_view(i64::from(f.w() - 8), 10_000));
            if list_measure(&name) {
                return;
            }
            f.redraw();
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
    // (a flat white box: the theme's DownBox shades what the items don't cover)
    table.set_frame(FrameType::BorderBox);
    table.set_color(Color::White);
    // (the scroll bars take the table's color: gray, to be seen)
    for mut bar in [table.scrollbar(), table.hscrollbar()] {
        bar.set_color(Color::from_rgb(225, 225, 225));
    }
    table_scrollbars(&mut table);
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
    table.draw_cell(move |_, ctx, row, col, cx, cy, cw, ch| {
        if ctx != TableContext::Cell || row < 0 || col < 0 {
            return;
        }
        let Some(i) = owner_list_index(&draw_name, row, col) else {
            // (a cell past the last item, in columns)
            draw::draw_rect_fill(cx, cy, cw, ch, Color::White);
            return;
        };
        let font = rapidr_value::objects::font_from_props(&draw_name, &|id, p| rp_comp_get(id, p));
        let Some((iw, ih, rgba, scale)) = rapidr_value::objects::list_item_pixels(&draw_name, i, cw as i64, &font) else { return };
        if let Some(mut img) = display_image(iw, ih, &rgba, scale) {
            draw::push_clip(cx, cy, cw, ch);
            img.draw(cx, cy, (iw / scale) as i32, (ih / scale) as i32);
            draw::pop_clip();
        }
    });
    let handle_name = name.to_string();
    // (RapidR's handler first: it returns true for what it handles alone)
    table.super_handle_first(false);
    table.handle(move |t, ev| owner_list_handle(&handle_name, t, ev));
    GUI_WIDGETS.with(|gw| {
        gw.borrow_mut().insert(name.to_string(), GuiWidget::Grid(table, hidden));
    });
    list_refresh(name);
}

/// Draws a table's scroll bars after the table (FLTK's table leaves them
/// unpainted here).
fn table_scrollbars(table: &mut Table) {
    table.draw(|t| {
        for mut bar in [t.scrollbar(), t.hscrollbar()] {
            if bar.visible() {
                t.draw_child(&mut bar);
            }
        }
    });
}

/// An owner-drawn QCOMBOBOX (`Style` csOwnerDrawFixed / csOwnerDrawVariable):
/// a box showing the selected item as OnDrawItem drew it (the shared list
/// model's `render_item`) and a button; a click drops down the items, each
/// as drawn, and the pick is the ItemIndex (OnChange). Up / Down pick the
/// item before / after.
fn owner_combo_create(name: &str, x: i32, y: i32, w: i32, h: i32) {
    let mut f = Frame::new(x, y, w, h, None);
    f.set_frame(FrameType::DownBox);
    f.set_color(Color::White);
    let draw_name = name.to_string();
    f.draw(move |f| {
        let button = f.h().min(20);
        let (bx, by, bw, bh) = (f.x() + f.w() - button - 2, f.y() + 2, button, f.h() - 4);
        let index = rapidr_value::objects::with_list(&draw_name, |l| l.item_index).unwrap_or(-1);
        let font = rapidr_value::objects::font_from_props(&draw_name, &|id, p| rp_comp_get(id, p));
        if let Some((iw, ih, rgba, scale)) = usize::try_from(index).ok().and_then(|i| rapidr_value::objects::list_item_pixels(&draw_name, i, i64::from(bx - f.x() - 2), &font)) {
            if let Some(mut img) = display_image(iw, ih, &rgba, scale) {
                let (lw, lh) = ((iw / scale) as i32, (ih / scale) as i32);
                draw::push_clip(f.x() + 2, f.y() + 2, bx - f.x() - 2, f.h() - 4);
                img.draw(f.x() + 2, f.y() + (f.h() - lh) / 2, lw, lh);
                draw::pop_clip();
            }
        }
        draw::draw_box(FrameType::ThinUpBox, bx, by, bw, bh, Color::from_rgb(230, 230, 230));
        draw::set_draw_color(Color::Black);
        let (cx, cy) = (bx + bw / 2, by + bh / 2);
        draw::draw_polygon(cx - 4, cy - 2, cx + 4, cy - 2, cx, cy + 2);
    });
    let handle_name = name.to_string();
    // (RapidR's handler first: it returns true for what it handles alone)
    f.super_handle_first(false);
    f.handle(move |f, ev| match ev {
        Event::Push => {
            let _ = f.take_focus();
            owner_combo_drop(&handle_name, f);
            true
        }
        Event::Focus | Event::Unfocus => true,
        Event::KeyDown => {
            let step = match app::event_key() {
                Key::Down => 1,
                Key::Up => -1,
                _ => return false,
            };
            let Some((index, count)) = rapidr_value::objects::with_list(&handle_name, |l| (l.item_index, l.items.len() as i64)) else { return false };
            let next = (index + step).clamp(0, (count - 1).max(0));
            if count > 0 && next != index {
                owner_combo_pick(&handle_name, next);
            }
            true
        }
        _ => false,
    });
    GUI_WIDGETS.with(|gw| {
        gw.borrow_mut().insert(name.to_string(), GuiWidget::Frame(f));
    });
    list_refresh(name);
}

/// The combo box's drop-down: its items as drawn, under the box.
fn owner_combo_drop(name: &str, f: &Frame) {
    let font = rapidr_value::objects::font_from_props(name, &|id, p| rp_comp_get(id, p));
    let count = rapidr_value::objects::with_list(name, |l| l.items.len().min(rapidr_value::objects::list::MAX_OWNER_DRAWN)).unwrap_or(0);
    if count == 0 {
        return;
    }
    let current = fltk::group::Group::try_current();
    fltk::group::Group::set_current(None::<&fltk::group::Group>);
    let mut menu = fltk::menu::MenuButton::new(f.x(), f.y(), f.w(), f.h(), None);
    if let Some(group) = current {
        fltk::group::Group::set_current(Some(&group));
    }
    for i in 0..count {
        menu.add_choice(" ");
        let Some((iw, ih, rgba, scale)) = rapidr_value::objects::list_item_pixels(name, i, i64::from(f.w() - 8), &font) else { continue };
        if let (Some(mut item), Some(img)) = (menu.at(i as i32), display_image(iw, ih, &rgba, scale)) {
            item.add_image(Some(img), true);
        }
    }
    let picked = menu.popup().map(|_| menu.value());
    fltk::menu::MenuButton::delete(menu);
    if let Some(i) = picked.filter(|&i| i >= 0) {
        owner_combo_pick(name, i64::from(i));
    }
}

/// The user picked item `i` of an owner-drawn combo box: its ItemIndex,
/// then OnChange.
fn owner_combo_pick(name: &str, i: i64) {
    rapidr_value::objects::with_list_mut(name, |l| l.select(i));
    list_refresh(name);
    rp_fire_event(name, "onchange");
}

/// Selects item `i` the way a click does (MultiSelect: Shift / Ctrl held
/// extend or toggle, `ItemList::click`).
fn owner_list_select(name: &str, i: i64) {
    let (shift, ctrl) = (app::is_event_shift(), app::is_event_ctrl() || app::is_event_command());
    rapidr_value::objects::with_list_mut(name, |l| l.click(i, shift, ctrl));
}

/// The item in table cell (row, col): one per row, or with `Columns` down
/// each column (rapidr_value::objects::list::ItemList::column_layout).
fn owner_list_index(name: &str, row: i32, col: i32) -> Option<usize> {
    rapidr_value::objects::with_list(name, |l| {
        let i = if l.multi_column() { col as i64 * l.column_layout().0 + row as i64 } else { row as i64 };
        usize::try_from(i).ok().filter(|&i| i < l.items.len())
    })
    .flatten()
}

fn owner_list_handle(name: &str, t: &mut Table, ev: Event) -> bool {
    let Some((count, per, multi)) = rapidr_value::objects::with_list(name, |l| (l.items.len() as i64, l.column_layout().0, l.multi_column())) else { return false };
    match ev {
        // A click on an item selects it; anywhere else (the scroll bars) is
        // the table's.
        Event::Push => {
            let Some((TableContext::Cell, row, col, _)) = t.cursor2rowcol() else { return false };
            let _ = t.take_focus();
            t.redraw();
            if let Some(i) = owner_list_index(name, row, col) {
                owner_list_select(name, i as i64);
                list_refresh(name);
                rp_fire_event(name, if app::event_clicks() { "ondblclick" } else { "onclick" });
            }
            true
        }
        Event::Focus | Event::Unfocus => true,
        Event::KeyDown => {
            let current = rapidr_value::objects::with_list(name, |l| l.item_index).unwrap_or(-1);
            let page = if multi { per } else { i64::from((t.h() / rapidr_value::objects::with_list(name, |l| l.row_height()).unwrap_or(16) as i32).max(1)) };
            let next = match app::event_key() {
                Key::Down => current + 1,
                Key::Up => (current - 1).max(0),
                // In columns: the item beside.
                Key::Right if multi => current + per,
                Key::Left if multi => (current - per).max(0),
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
                let (top, bottom, left, right) = t.visible_cells();
                let (row, col) = if multi { ((next % per) as i32, (next / per) as i32) } else { (next as i32, 0) };
                if row < top {
                    t.set_row_position(row);
                } else if row > bottom {
                    t.set_row_position((row - (bottom - top)).max(0));
                }
                if col < left {
                    t.set_col_position(col);
                } else if col > right {
                    t.set_col_position((col - (right - left)).max(0));
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
    if !rapidr_value::objects::with_list_mut(name, |l| l.owner_drawn() && l.owner_draw_needed()).unwrap_or(false) {
        return;
    }
    let items = rapidr_value::objects::with_list(name, |l| l.owner_draw_items()).unwrap_or_default();
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

thread_local! {
    /// Each QLISTVIEW's caption editor, and the item it edits.
    static LISTVIEW_EDITORS: RefCell<HashMap<String, (Input, Option<usize>)>> = RefCell::new(HashMap::new());
}

/// Edits item `i`'s caption in place (F2, a click on the selected item).
fn listview_begin_edit(name: &str, i: usize) {
    let name = name.to_lowercase();
    listview_prepare(&name);
    let Some(GuiWidget::Frame(f)) = GUI_WIDGETS.with(|gw| gw.borrow().get(&name).cloned()) else { return };
    let Some(Some((l, t, r, b))) = rapidr_value::objects::with_listview_mut(&name, |lv| lv.editor_rect(i)) else { return };
    let Some(text) = rapidr_value::objects::with_listview(&name, |lv| lv.items.get(i).map(|it| it.caption.clone())).flatten() else { return };
    redraw_widget(&name);
    let font = rapidr_value::objects::font_from_props(&name, &|id, p| rp_comp_get(id, p));
    LISTVIEW_EDITORS.with(|e| {
        if let Some((editor, editing)) = e.borrow_mut().get_mut(&name) {
            *editing = Some(i);
            editor.resize(f.x() + l as i32, f.y() + t as i32, (r - l) as i32, (b - t) as i32);
            editor.set_text_size((font.pixel_size() as i32).clamp(8, 72));
            editor.set_value(&text);
            editor.show();
            let _ = editor.take_focus();
            let _ = editor.set_position(text.len() as i32);
            let _ = editor.set_mark(0);
            editor.redraw();
        }
    });
}

/// Ends a caption's edit: with `keep`, the item gets the text
/// (OnChange (Index, ctText)).
fn listview_end_edit(name: &str, keep: bool) {
    let ended = LISTVIEW_EDITORS.with(|e| {
        let mut editors = e.borrow_mut();
        let (editor, editing) = editors.get_mut(name)?;
        let i = editing.take()?;
        let text = editor.value();
        editor.hide();
        Some((i, text))
    });
    let Some((i, text)) = ended else { return };
    if let Some(GuiWidget::Frame(mut f)) = GUI_WIDGETS.with(|gw| gw.borrow().get(name).cloned()) {
        let _ = f.take_focus();
        f.redraw();
    }
    if keep {
        let events = rapidr_value::objects::with_listview_mut(name, |lv| lv.edited(i, text)).unwrap_or_default();
        redraw_widget(name);
        listview_fire(name, events);
    }
}

/// A QLISTVIEW changed: its control shows it again.
pub fn listview_refresh(name: &str) {
    redraw_widget(&name.to_lowercase());
}

/// A QLISTVIEW's Color (a list view is white unless the program says).
fn listview_background(name: &str) -> u32 {
    match rp_comp_get(name, "color") {
        Value::Null => 0xFFFFFF,
        Value::String(s) if s.is_empty() => 0xFFFFFF,
        v => rapidr_value::objects::form_color(&v) as u32,
    }
}

/// Gives a QLISTVIEW's model its control's size and font.
fn listview_prepare_sized(name: &str, w: i32, h: i32, focused: bool) {
    let font = rapidr_value::objects::font_from_props(name, &|id, p| rp_comp_get(id, p));
    rapidr_value::objects::listview_setup(name, w as i64, h as i64, &font, focused);
}

/// [`listview_prepare_sized`] from the widget (before the mouse or a key).
fn listview_prepare(name: &str) {
    let Some(w) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(&name.to_lowercase()).map(GuiWidget::base))) else { return };
    let focused = app::focus().is_some_and(|f| f.as_widget_ptr() == w.as_widget_ptr());
    listview_prepare_sized(name, w.w(), w.h(), focused);
}

/// Fires what the user did to a QLISTVIEW (rapidr_value::objects::
/// listview::Event), in order.
fn listview_fire(name: &str, events: Vec<rapidr_value::objects::listview::Event>) {
    use rapidr_value::objects::listview::Event as E;
    for e in events {
        match e {
            E::Click => rp_fire_event(name, "onclick"),
            E::DblClick => rp_fire_event(name, "ondblclick"),
            E::ColumnClick(i) => rp_fire_event_1(name, "oncolumnclick", v_int(i as i64)),
            E::Change(i, ct) => rp_fire_event_2(name, "onchange", v_int(i as i64), v_int(ct)),
            E::Edit(i) => listview_begin_edit(name, i),
            E::EditSoon(i) => {
                let (lv, clicks) = (name.to_string(), rapidr_value::objects::with_listview(name, |lv| lv.clicks).unwrap_or(0));
                app::add_timeout3(0.5, move |_| {
                    if rapidr_value::objects::with_listview(&lv, |m| m.clicks) == Some(clicks) {
                        listview_begin_edit(&lv, i);
                    }
                });
            }
        }
    }
}

/// The mouse on a QLISTVIEW (x, y in it; `shift`: RapidQ's Shift).
fn listview_mouse(name: &str, kind: rapidr_value::input::Mouse, x: i64, y: i64, shift: i64, double: bool) {
    use rapidr_value::input::Mouse;
    listview_prepare(name);
    // (⌘ on macOS picks as Ctrl does on Windows)
    let ctrl = shift & 16 != 0 || app::event_state().contains(fltk::enums::Shortcut::Meta);
    let (events, changed) = match kind {
        Mouse::Down => (rapidr_value::objects::with_listview_mut(name, |lv| lv.mouse_down(x, y, shift & 256 != 0, ctrl, double)).unwrap_or_default(), true),
        Mouse::Up => (rapidr_value::objects::with_listview_mut(name, |lv| lv.mouse_up(x, y)).unwrap_or_default(), true),
        Mouse::Move => (Vec::new(), rapidr_value::objects::with_listview_mut(name, |lv| lv.mouse_move(x, y)).unwrap_or(false)),
    };
    if changed {
        redraw_widget(&name.to_lowercase());
    }
    listview_fire(name, events);
}

/// A key down on a focused QLISTVIEW.
fn listview_key(name: &str, vk: i64, shift: i64) {
    listview_prepare(name);
    let ctrl = shift & 16 != 0 || app::event_state().contains(fltk::enums::Shortcut::Meta);
    let (events, changed) = rapidr_value::objects::with_listview_mut(name, |lv| lv.key_down(vk, shift & 256 != 0, ctrl)).unwrap_or_default();
    if changed {
        redraw_widget(&name.to_lowercase());
    }
    listview_fire(name, events);
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
    table_scrollbars(&mut table);

    // Enter stores the edited cell.
    let enter_name = name.to_string();
    editor.set_trigger(CallbackTrigger::EnterKeyAlways);
    editor.set_callback(move |_| grid_finish_edit(&enter_name, true));
    // Leaving the cell stores it too; Escape drops the edit.
    let edit_name = name.to_string();
    // (RapidR's handler first: it returns true for what it handles alone)
    editor.super_handle_first(false);
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
    // (RapidR's handler first: it returns true for what it handles alone)
    table.super_handle_first(false);
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
    // (VisibleRowCount / VisibleColCount: what fits inside its frame)
    if let Some((w, h)) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(&name).map(|w| (w.base().w(), w.base().h())))) {
        rapidr_value::objects::with_grid_mut(&name, |g| g.view = ((w - 4) as i64, (h - 4) as i64));
    }
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
/// Only for a grid on screen, as RapidQ paints (a hidden grid's cells are
/// drawn when its form shows: [`owner_draw_shown_grids`]) — a program sets
/// up what its OnDrawCell reads before showing the form.
fn grid_owner_draw(name: &str) {
    if !crate::object::rp_has_handler(name, "ondrawcell") {
        return;
    }
    let on_screen = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(&name.to_lowercase()).map(|w| w.base().visible_r())).unwrap_or(false));
    if !on_screen {
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

/// The grids a form just showed: their OnDrawCell, held back while hidden.
fn owner_draw_shown_grids() {
    let grids: Vec<String> = GUI_WIDGETS.with(|gw| {
        gw.try_borrow().map(|gw| gw.iter().filter(|(_, w)| matches!(w, GuiWidget::Grid(..))).map(|(n, _)| n.clone()).collect()).unwrap_or_default()
    });
    for g in grids {
        grid_owner_draw(&g);
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
                    let (pw, ph, rgba, scale) = b.clone().display_rgba();
                    if let Some(mut img) = display_image(pw, ph, &rgba, scale) {
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

thread_local! {
    /// A column (true) / row being moved by the mouse, from where.
    static GRID_MOVE: Cell<Option<(bool, i64)>> = const { Cell::new(None) };
}

/// What a press on cell (c, r) moves: its column (true: a fixed row's cell,
/// goColMoving) or its row (false: a fixed column's, goRowMoving).
fn grid_move_kind(name: &str, c: i64, r: i64) -> Option<bool> {
    use rapidr_value::objects::grid::{GO_COL_MOVING, GO_ROW_MOVING};
    rapidr_value::objects::with_grid(name, |g| {
        let (fr, fc) = (g.fixed_rows() as i64, g.fixed_cols() as i64);
        if r < fr && c >= fc && g.has_option(GO_COL_MOVING) {
            Some(true)
        } else if c < fc && r >= fr && g.has_option(GO_ROW_MOVING) {
            Some(false)
        } else {
            None
        }
    })
    .flatten()
}

fn grid_handle(name: &str, t: &mut Table, ev: Event) -> bool {
    use rapidr_value::objects::grid::GO_ALWAYS_SHOW_EDITOR;
    match ev {
        Event::Push => {
            grid_finish_edit(name, true);
            let Some((ctx, row, col, resize)) = t.cursor2rowcol() else { return false };
            // On a header's border (goColSizing / goRowSizing): FLTK's table
            // resizes. Any other click is the grid's alone: FLTK's own
            // handling of a header click (selecting the column) redrew
            // only part of the table, leaving the rest blank.
            let resizing = !matches!(resize, fltk::table::TableResizeFlag::None);
            if resizing && !matches!(ctx, TableContext::Cell) {
                return false;
            }
            let Some((c, r)) = grid_cell_of(name, ctx, row, col) else { return false };
            // A fixed row's cell dragged (goColMoving) moves its column, a
            // fixed column's (goRowMoving) its row: on the release.
            if let Some(moving) = grid_move_kind(name, c, r) {
                GRID_MOVE.with(|m| m.set(Some((moving, if moving { c } else { r }))));
                return true;
            }
            // An ellipsis button.
            let on_button = rapidr_value::objects::with_grid(name, |g| grid_has_ellipsis(g, c as usize, r as usize)).unwrap_or(false)
                && t.find_cell(ctx, row, col).is_some_and(|(x, _, w, h)| app::event_x() >= x + w - h.min(w));
            let _ = t.take_focus();
            // All of it drawn again: FLTK's table handles the click first
            // (cfltk) and redraws only part of itself — a header click left
            // the headers and the other cells blank.
            t.redraw();
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
            !resizing
        }
        // The end of a drag: a moved column / row …
        Event::Released if GRID_MOVE.with(|m| m.get()).is_some() => {
            let Some((cols, from)) = GRID_MOVE.with(|m| m.take()) else { return false };
            let to = t.cursor2rowcol().and_then(|(ctx, row, col, _)| grid_cell_of(name, ctx, row, col));
            if let Some((c, r)) = to {
                let to = if cols { c } else { r };
                if rapidr_value::objects::with_grid_mut(name, |g| if cols { g.move_col(from as usize, to as usize) } else { g.move_row(from as usize, to as usize) }).unwrap_or(false) {
                    grid_refresh(name);
                }
            }
            t.redraw();
            true
        }
        // … resized columns / rows keep their sizes.
        Event::Released => {
            grid_sync_sizes(name, t);
            t.redraw();
            false
        }
        // (a column / row being moved: no range selected on the way)
        Event::Drag if GRID_MOVE.with(|m| m.get()).is_some() => true,
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
        note_display_scale(f);
        let shown = rapidr_value::objects::with_canvas(&form_name, cw as i64, ch as i64, |b| b.display_rgba());
        if let Some(mut img) = shown.and_then(|(pw, ph, rgba, scale)| display_image(pw, ph, &rgba, scale)) {
            img.draw(f.x(), f.y() + menu, cw, ch);
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

thread_local! {
    /// Each QHEADER's size when its faces were last painted.
    static HEADER_SIZES: RefCell<HashMap<String, (i32, i32)>> = RefCell::new(HashMap::new());
    static HEADERS_PAINTING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Paints a shown QHEADER's section faces again and fires OnDrawSection
/// (Index, Pressed, Rect) for its owner-drawn ones, which draw on it.
pub fn header_refresh(name: &str) {
    let name = name.to_lowercase();
    let Some((w, h)) = GUI_WIDGETS.with(|gw| gw.try_borrow().ok().and_then(|gw| gw.get(&name).map(|w| (w.base().w(), w.base().h())))) else { return };
    if !HEADERS_PAINTING.with(|p| p.borrow_mut().insert(name.clone())) {
        return;
    }
    HEADER_SIZES.with(|s| s.borrow_mut().insert(name.clone(), (w, h)));
    for (i, pressed, (left, top, right, bottom)) in rapidr_value::objects::paint_header(&name, w as i64, h as i64) {
        let rect = format!("{name}.sectionrect({i})");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            crate::object::rp_comp_set(&rect, prop, v_int(v));
        }
        rp_fire_event_args(&name, "ondrawsection", &[v_int(i as i64), v_int(if pressed { -1 } else { 0 }), v_str(&rect)]);
    }
    HEADERS_PAINTING.with(|p| p.borrow_mut().remove(&name));
    redraw_widget(&name);
}

/// The left button on a QHEADER at x: sections pressed, clicked, resized
/// (OnSectionClick, OnSectionTrack, OnSectionResize).
fn header_mouse(name: &str, kind: rapidr_value::input::Mouse, x: i64) {
    use rapidr_value::input::Mouse;
    use rapidr_value::objects::header::{Action, TS_END};
    let before = rapidr_value::objects::with_header(name, |h| h.clone());
    let actions = rapidr_value::objects::with_header(name, |h| match kind {
        Mouse::Down => h.press(x),
        Mouse::Move => h.drag_to(x),
        Mouse::Up => h.release(x),
    })
    .unwrap_or_default();
    for action in actions {
        match action {
            Action::Click(i) => rp_fire_event_1(name, "onsectionclick", v_int(i as i64)),
            Action::Track(i, width, state) => {
                rp_fire_event_args(name, "onsectiontrack", &[v_int(i as i64), v_int(width), v_int(state)]);
                if state == TS_END {
                    rp_fire_event_1(name, "onsectionresize", v_int(i as i64));
                }
            }
        }
    }
    let after = rapidr_value::objects::with_header(name, |h| h.clone());
    if before.map(|h| (h.pressed, h.sections)) != after.map(|h| (h.pressed, h.sections)) {
        header_refresh(name);
    }
}

/// A QCANVAS's or a QFORM's surface changed: show it again.
pub fn canvas_redraw(name: &str) {
    let name = name.to_lowercase();
    if rapidr_value::objects::is_form_surface(&name) {
        // The whole window: the surface lies under the controls, which
        // must be drawn over it again (redrawing the surface alone painted
        // over them).
        redraw_widget(&name);
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

#[cfg(test)]
mod look_tests {
    use super::look_for;

    #[test]
    fn a_program_looks_like_its_platform_unless_it_names_a_look() {
        assert_eq!(look_for("", "macos"), "aquaclassic");
        assert_eq!(look_for("system", "windows"), "metro");
        assert_eq!(look_for("light", "linux"), "gleam");
        assert_eq!(look_for("mac", "linux"), "aqua");
        assert_eq!(look_for("Dark", "macos"), "dark");
        assert_eq!(look_for("AquaClassic", "windows"), "aquaclassic");
        assert_eq!(look_for("win7", "macos"), "aero");
        assert_eq!(look_for("crystal", "macos"), "clean");
        assert_eq!(look_for("no such look", "freebsd"), "gleam");
    }
}
