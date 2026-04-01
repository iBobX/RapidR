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
    enums::{Align, Color, Event, Font, FrameType, Key},
    frame::Frame,
    group::{Group, Scroll, Tabs},
    image::SharedImage,
    input::Input,
    menu::{Choice, MenuBar},
    misc::Progress as FltkProgress,
    output::Output,
    prelude::*,
    text::{TextBuffer, TextEditor},
    tree::Tree,
    valuator::HorNiceSlider,
    window::Window,
};

use fltk_theme::{ThemeType, WidgetTheme};

use crate::object::{rp_comp_get, rp_comp_set, rp_comp_type, rp_fire_event, rp_fire_event_1, rp_fire_event_2, rp_fire_event_5};
use crate::value::{v_int, v_null, v_str, Value};

// ---------------------------------------------------------------------------
// Widget handle registry
// ---------------------------------------------------------------------------

/// Each GUI component gets a unique handle. We store FLTK widgets in an enum
/// because they have different types.
enum GuiWidget {
    Window(Window),
    Button(Button),
    Frame(Frame),
    Input(Input),
    Output(Output),
    CheckButton(CheckButton),
    RadioButton(RadioRoundButton),
    Choice(Choice),
    HoldBrowser(HoldBrowser),
    TextEditor(TextEditor),
    Group(Group),
    Tabs(Tabs),
    MenuBar(MenuBar),
    Progress(FltkProgress),
    Scroll(Scroll),
    Tree(Tree),
    Slider(HorNiceSlider),
    ImageFrame(Frame), // RImage — Frame with drawn image
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

// ---------------------------------------------------------------------------
// String grid row tracking
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct StringGridRow {
    cols: Vec<String>,
}

impl StringGridRow {
    fn new(values: Vec<String>) -> Self {
        if values.is_empty() {
            Self { cols: vec![String::new()] }
        } else {
            Self { cols: values }
        }
    }

    fn get(&self, idx: usize) -> &str {
        self.cols.get(idx).map(|s| s.as_str()).unwrap_or("")
    }
}

#[derive(Clone, Debug)]
struct StringGridState {
    rows: Vec<StringGridRow>,
    selected_row: i32,
    cols: i32,
    suggestions: Vec<String>,
}

thread_local! {
    static GUI_WIDGETS: RefCell<HashMap<String, GuiWidget>> = RefCell::new(HashMap::new());
    static GUI_APP: RefCell<Option<app::App>> = RefCell::new(None);
    static GUI_TEXT_BUFFERS: RefCell<HashMap<String, TextBuffer>> = RefCell::new(HashMap::new());
    static DESIGN_SURFACES: RefCell<HashMap<String, DesignState>> = RefCell::new(HashMap::new());
    static STRING_GRIDS: RefCell<HashMap<String, StringGridState>> = RefCell::new(HashMap::new());
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
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "caption").to_string_val();
            let mut win = Window::new(100, 100, w, h, None);
            win.set_label(&caption);
            win.make_resizable(true);
            win.end();
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Window(win));
            });
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

            // Color-based visual feedback (FrameType changes are invisible with fltk-theme)
            let normal_color = btn.color();
            let hover_color = normal_color.lighter();
            let press_color = normal_color.darker();
            btn.handle(move |b, ev| {
                match ev {
                    Event::Enter => {
                        b.set_color(hover_color);
                        b.redraw();
                        true
                    }
                    Event::Leave => {
                        b.set_color(normal_color);
                        b.set_frame(FrameType::UpBox);
                        b.redraw();
                        true
                    }
                    Event::Push => {
                        b.set_color(press_color);
                        b.set_frame(FrameType::DownBox);
                        b.redraw();
                        false // let default handle process the click
                    }
                    Event::Released => {
                        b.set_color(normal_color);
                        b.set_frame(FrameType::UpBox);
                        b.redraw();
                        false
                    }
                    Event::Focus => {
                        b.set_frame(FrameType::ThinUpBox);
                        b.redraw();
                        true
                    }
                    Event::Unfocus => {
                        b.set_frame(FrameType::UpBox);
                        b.redraw();
                        true
                    }
                    Event::KeyDown => {
                        let key = app::event_key();
                        if key == Key::Enter || key == Key::from_char(' ') {
                            b.do_callback();
                            true
                        } else {
                            false
                        }
                    }
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
        "RLABEL" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "caption").to_string_val();
            let mut lbl = Frame::new(x, y, w, h, None);
            lbl.set_label(&caption);
            lbl.set_frame(FrameType::NoBox);
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
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Input(inp));
            });
        }
        "RPANEL" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let grp = Group::new(x, y, w, h, None);
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
            let mut choice = Choice::new(x, y, w, h, None);
            // Add any pre-set items
            let items = rp_comp_get(name, "items").to_string_val();
            for item in items.lines() {
                if !item.is_empty() {
                    choice.add_choice(item);
                }
            }
            let name_for_cb = name.to_lowercase();
            choice.set_callback(move |c| {
                let idx = c.value();
                rp_comp_set(&name_for_cb, "itemindex", v_int(idx as i64));
                if let Some(text) = c.choice() {
                    rp_comp_set(&name_for_cb, "text", v_str(&text));
                }
                rp_fire_event(&name_for_cb, "onchange");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Choice(choice));
            });
        }
        "RLISTBOX" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut browser = HoldBrowser::new(x, y, w, h, None);
            let items = rp_comp_get(name, "items").to_string_val();
            for item in items.lines() {
                if !item.is_empty() {
                    browser.add(item);
                }
            }
            let name_for_cb = name.to_lowercase();
            browser.set_callback(move |b| {
                let idx = b.value() - 1; // FLTK browsers are 1-indexed
                rp_comp_set(&name_for_cb, "itemindex", v_int(idx as i64));
                rp_fire_event(&name_for_cb, "onclick");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::HoldBrowser(browser));
            });
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
            // StatusBar will be drawn as an Output at the bottom of a form
            let mut out = Output::new(0, 0, 100, 25, None);
            let text = rp_comp_get(name, "simpletext").to_string_val();
            out.set_value(&text);
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Output(out));
            });
        }
        "RTABCONTROL" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut tabs = Tabs::new(x, y, w, h, None);

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
            let mut editor = TextEditor::new(x, y, w, h, None);
            editor.set_buffer(buf.clone());
            editor.set_text_font(Font::Courier);
            editor.set_text_size(13);
            editor.set_linenumber_width(40);
            GUI_TEXT_BUFFERS.with(|tb| {
                tb.borrow_mut().insert(name_lower.clone(), buf);
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::TextEditor(editor));
            });
        }
        "RMAINMENU" => {
            // MenuBar placed at the top of the parent form
            let parent = rp_comp_get(name, "parent").to_string_val();
            let pw = if parent.is_empty() {
                800
            } else {
                rp_comp_get(&parent, "width").to_i64() as i32
            };
            let mut mb = MenuBar::new(0, 0, pw, 30, None);
            mb.set_text_size(13);
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::MenuBar(mb));
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
                        if let Some(GuiWidget::MenuBar(ref mut mb)) = widgets.get_mut(&mb_name) {
                            if caption == "-" {
                                // Separator — add a dummy disabled item
                                mb.add_choice(&path);
                            } else {
                                mb.add(
                                    &path,
                                    fltk::enums::Shortcut::None,
                                    fltk::menu::MenuFlag::Normal,
                                    move |_| {
                                        rp_fire_event(&name_for_cb, "onclick");
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
            let num_cols = rp_comp_get(name, "cols").to_i64().max(1) as usize;
            let col_width = rp_comp_get(name, "colwidth").to_i64();
            let col_w = if col_width > 0 { col_width as i32 } else { (w / num_cols as i32).max(60) };

            let mut scroll = Scroll::new(x, y, w, h, None);
            scroll.set_frame(FrameType::DownBox);

            // Populate from stored state (rows added before widget creation)
            let rows = STRING_GRIDS.with(|sg| {
                sg.borrow().get(&name_lower).map(|s| s.rows.clone()).unwrap_or_default()
            });
            let sg_name = name_lower.clone();
            let row_h = 22;
            for (row_idx, row) in rows.iter().enumerate() {
                let row_y = y + (row_idx as i32) * row_h;
                for (col_idx, col_val) in row.cols.iter().enumerate() {
                    let cell_x = x + (col_idx as i32) * col_w;
                    let is_header = row_idx == 0;
                    if is_header {
                        let mut lbl = Frame::new(cell_x, row_y, col_w, row_h, None);
                        lbl.set_label(col_val);
                        lbl.set_frame(FrameType::FlatBox);
                        lbl.set_color(Color::from_rgb(200, 210, 230));
                        lbl.set_align(Align::Left | Align::Inside);
                    } else {
                        let mut inp = Input::new(cell_x, row_y, col_w, row_h, None);
                        inp.set_value(col_val);
                        inp.set_frame(FrameType::ThinUpBox);
                        let sg_cb = sg_name.clone();
                        let ri = row_idx;
                        let ci = col_idx;
                        inp.set_callback(move |i| {
                            let val = i.value();
                            STRING_GRIDS.with(|sg| {
                                let mut grids = sg.borrow_mut();
                                if let Some(state) = grids.get_mut(&sg_cb) {
                                    if ri < state.rows.len() && ci < state.rows[ri].cols.len() {
                                        state.rows[ri].cols[ci] = val.clone();
                                        state.selected_row = ri as i32;
                                    }
                                }
                            });
                            rp_fire_event(&sg_cb, "onchange");
                        });
                    }
                }
            }

            scroll.end();
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower.clone(), GuiWidget::Scroll(scroll));
            });
            // Initialize grid state if not already done
            STRING_GRIDS.with(|sg| {
                let mut grids = sg.borrow_mut();
                if !grids.contains_key(&name_lower) {
                    grids.insert(name_lower, StringGridState {
                        rows: Vec::new(),
                        selected_row: -1,
                        cols: 2,
                        suggestions: Vec::new(),
                    });
                }
            });
        }
        "RTREEVIEW" => {
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut tree = Tree::new(x, y, w, h, None);
            tree.set_show_root(false);
            let name_for_cb = name.to_lowercase();
            tree.set_callback(move |_t| {
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

            // Install draw callback to render stored canvas commands
            let name_for_draw = name.to_lowercase();
            frm.draw(move |f| {
                // Fill background
                draw::draw_rect_fill(f.x(), f.y(), f.w(), f.h(), f.color());
                CANVAS_CMDS.with(|cmds| {
                    let map = cmds.borrow();
                    if let Some(cmd_list) = map.get(&name_for_draw) {
                        for cmd in cmd_list {
                            match cmd {
                                DrawCmd::Line(x1, y1, x2, y2, color) => {
                                    draw::set_draw_color(*color);
                                    draw::draw_line(*x1, *y1, *x2, *y2);
                                }
                                DrawCmd::Rect(rx, ry, rw, rh, color) => {
                                    draw::set_draw_color(*color);
                                    draw::draw_rect(*rx, *ry, *rw, *rh);
                                }
                                DrawCmd::FillRect(rx, ry, rw, rh, color) => {
                                    draw::set_draw_color(*color);
                                    draw::draw_rect_fill(*rx, *ry, *rw, *rh, *color);
                                }
                                DrawCmd::Circle(cx, cy, r, color) => {
                                    draw::set_draw_color(*color);
                                    draw::draw_circle(*cx as f64, *cy as f64, *r as f64);
                                }
                                DrawCmd::DrawText(text, tx, ty, color, font_size) => {
                                    draw::set_draw_color(*color);
                                    draw::set_font(Font::Helvetica, *font_size);
                                    draw::draw_text2(text, *tx, *ty, 0, 0, Align::Left);
                                }
                                DrawCmd::Ellipse(ex, ey, ew, eh, color) => {
                                    draw::set_draw_color(*color);
                                    draw::draw_arc(*ex, *ey, *ew, *eh, 0.0, 360.0);
                                }
                                DrawCmd::Pixel(px, py, color) => {
                                    draw::draw_rect_fill(*px, *py, 1, 1, *color);
                                }
                            }
                        }
                    }
                });
            });

            let name_for_cb = name.to_lowercase();
            frm.handle(move |_, ev| {
                match ev {
                    Event::Push => {
                        let mx = app::event_x();
                        let my = app::event_y();
                        rp_fire_event(&name_for_cb, "onclick");
                        rp_fire_event_2(&name_for_cb, "onmousedown", v_int(mx as i64), v_int(my as i64));
                        true
                    }
                    Event::Released => {
                        let mx = app::event_x();
                        let my = app::event_y();
                        rp_fire_event_2(&name_for_cb, "onmouseup", v_int(mx as i64), v_int(my as i64));
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
            // Load image if BMP/filename is set
            let bmp = rp_comp_get(name, "bmp").to_string_val();
            if !bmp.is_empty() {
                if let Ok(mut img) = SharedImage::load(&bmp) {
                    let stretch = rp_comp_get(name, "stretch").to_i64() != 0;
                    if stretch {
                        img.scale(w, h, true, true);
                    }
                    frm.set_image(Some(img));
                }
            }
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::ImageFrame(frm));
            });
        }
        "RSPLITTER" => {
            // Splitter — implemented as a thin resizable group
            let x = rp_comp_get(name, "left").to_i64() as i32;
            let y = rp_comp_get(name, "top").to_i64() as i32;
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let mut grp = Group::new(x, y, w, h, None);
            grp.set_frame(FrameType::ThinUpBox);
            grp.end();
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Group(grp));
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
            let name_for_cb = name.to_lowercase();
            browser.set_callback(move |b| {
                let idx = b.value() - 1;
                rp_comp_set(&name_for_cb, "itemindex", v_int(idx as i64));
                rp_fire_event(&name_for_cb, "onclick");
            });
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::HoldBrowser(browser));
            });
        }
        "RFORMMDI" => {
            // MDI parent form — create a resizable Window
            let w = rp_comp_get(name, "width").to_i64() as i32;
            let h = rp_comp_get(name, "height").to_i64() as i32;
            let caption = rp_comp_get(name, "caption").to_string_val();
            let mut win = Window::new(100, 100, w, h, None);
            win.set_label(&caption);
            win.make_resizable(true);
            win.end();
            GUI_WIDGETS.with(|gw| {
                gw.borrow_mut().insert(name_lower, GuiWidget::Window(win));
            });
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
            use fltk::valuator::Scrollbar;
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
// Design surface rendering
// ---------------------------------------------------------------------------

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

    // Draw placed components
    DESIGN_SURFACES.with(|ds| {
        let surfaces = ds.borrow();
        if let Some(state) = surfaces.get(ds_name) {
            for (i, comp) in state.components.iter().enumerate() {
                let cx = x + comp.x;
                let cy = y + comp.y;

                // Component background
                draw::set_draw_color(Color::from_rgb(236, 236, 236));
                draw::draw_rectf(cx, cy, comp.w, comp.h);

                // Border (blue if selected)
                if i as i32 == state.selected {
                    draw::set_draw_color(Color::Blue);
                } else {
                    draw::set_draw_color(Color::Black);
                }
                draw::draw_rect(cx, cy, comp.w, comp.h);

                // Component label
                draw::set_draw_color(Color::Black);
                draw::set_font(Font::Helvetica, 11);
                let label = comp.props.get("caption").unwrap_or(&comp.name);
                draw::draw_text2(label, cx + 2, cy + 2, comp.w - 4, comp.h - 4, Align::Center);

                // Type tag (small, top-left)
                draw::set_font(Font::Helvetica, 9);
                draw::set_draw_color(Color::from_rgb(100, 100, 100));
                draw::draw_text2(&comp.type_name, cx + 2, cy + 1, comp.w - 4, 12, Align::TopLeft);

                // Selection handles
                if i as i32 == state.selected {
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

/// Show a form as modal (blocking event loop).
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

    // Start all registered timers
    start_timers();

    // Run the FLTK event loop
    GUI_APP.with(|a| {
        if let Some(ref app) = *a.borrow() {
            while app.wait() {
                // Check if the main window is still shown
                let shown = GUI_WIDGETS.with(|gw| {
                    let widgets = gw.borrow();
                    if let Some(GuiWidget::Window(ref win)) = widgets.get(&name_lower) {
                        win.shown()
                    } else {
                        false
                    }
                });
                if !shown {
                    break;
                }
            }
        }
    });
}

/// Close a form.
pub fn gui_close(name: &str) {
    let name_lower = name.to_lowercase();
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(GuiWidget::Window(ref mut win)) = widgets.get_mut(&name_lower) {
            win.hide();
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
            // FLTK doesn't have a native font dialog. Use a simple input dialog.
            let current = rp_comp_get(name, "fontname").to_string_val();
            let prompt = format!("Font name (current: {}):", if current.is_empty() { "Helvetica" } else { &current });
            let result = dialog::input_default(&prompt, &current);
            if let Some(font_name) = result {
                if !font_name.is_empty() {
                    rp_comp_set(name, "fontname", v_str(&font_name));
                    v_int(1)
                } else {
                    v_int(0)
                }
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
    GUI_APP.with(|a| {
        if let Some(ref app) = *a.borrow() {
            app.run().ok();
        }
    });
}

// ---------------------------------------------------------------------------
// Internal: Build all widgets parented to a form
// ---------------------------------------------------------------------------

/// Materialize FLTK widgets for a form and all its children.
fn build_form_widgets(form_name: &str) {
    // First, create the form window
    gui_create_widget(form_name, "RFORM");

    // Recursively build all children
    build_children_recursive(form_name);
}

/// Recursively build child widgets of a parent container.
fn build_children_recursive(parent_name: &str) {
    let children = crate::object::get_children_of(parent_name);
    if children.is_empty() { return; }

    // Get parent widget offset for relative positioning
    let (parent_x, parent_y) = get_widget_offset(parent_name);

    // Begin adding children to the parent widget
    begin_widget(parent_name);

    for (child_name, child_type) in &children {
        // Temporarily offset child position by parent's position for widget creation.
        // Save original values and restore after creation to prevent double-offset
        // if this function is ever called again.
        let orig_left = rp_comp_get(child_name, "left").to_i64() as i32;
        let orig_top = rp_comp_get(child_name, "top").to_i64() as i32;
        if parent_x != 0 || parent_y != 0 {
            rp_comp_set(child_name, "left", v_int((orig_left + parent_x) as i64));
            rp_comp_set(child_name, "top", v_int((orig_top + parent_y) as i64));
        }

        gui_create_widget(child_name, child_type);

        // Restore original relative positions
        if parent_x != 0 || parent_y != 0 {
            rp_comp_set(child_name, "left", v_int(orig_left as i64));
            rp_comp_set(child_name, "top", v_int(orig_top as i64));
        }

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
            gui_close(name);
            v_null()
        }
        _ => {
            eprintln!("[WARN] DesignSurface.{}() not implemented", method);
            v_null()
        }
    }
}

/// Get a design surface property (CompCount, FormCaption, etc.)
pub fn design_surface_get(name: &str, prop: &str) -> Option<Value> {
    let name_lower = name.to_lowercase();
    let prop_lower = prop.to_lowercase();
    match prop_lower.as_str() {
        "compcount" => {
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

/// Handle method calls on a PSTRINGGRID component.
pub fn string_grid_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "clear" => {
            STRING_GRIDS.with(|sg| {
                let mut grids = sg.borrow_mut();
                let state = grids.entry(name_lower.clone()).or_insert_with(|| StringGridState {
                    rows: Vec::new(),
                    selected_row: -1,
                    cols: 2,
                    suggestions: Vec::new(),
                });
                state.rows.clear();
                state.selected_row = -1;
            });
            // Clear the visual scroll widget
            GUI_WIDGETS.with(|gw| {
                let mut widgets = gw.borrow_mut();
                if let Some(GuiWidget::Scroll(ref mut scroll)) = widgets.get_mut(&name_lower) {
                    scroll.clear();
                    scroll.redraw();
                }
            });
            v_null()
        }
        "addrow" => {
            // AddRow(col0, col1, col2, ...) — any number of columns
            let row_values: Vec<String> = args.iter().map(|v| v.to_string_val()).collect();
            let row_idx = STRING_GRIDS.with(|sg| {
                let mut grids = sg.borrow_mut();
                let state = grids.entry(name_lower.clone()).or_insert_with(|| StringGridState {
                    rows: Vec::new(),
                    selected_row: -1,
                    cols: row_values.len() as i32,
                    suggestions: Vec::new(),
                });
                let n_cols = row_values.len().max(state.cols as usize);
                if row_values.len() > state.cols as usize {
                    state.cols = row_values.len() as i32;
                }
                let mut padded = row_values.clone();
                while padded.len() < n_cols { padded.push(String::new()); }
                state.rows.push(StringGridRow::new(padded));
                state.rows.len() - 1
            });
            // Add visual row to the Scroll widget
            let sg_name = name_lower.clone();
            GUI_WIDGETS.with(|gw| {
                let mut widgets = gw.borrow_mut();
                if let Some(GuiWidget::Scroll(ref mut scroll)) = widgets.get_mut(&name_lower) {
                    let sx = scroll.x();
                    let sy = scroll.y();
                    let sw = scroll.w();
                    let row_h = 22;
                    let row_y = sy + (row_idx as i32) * row_h;
                    let n = row_values.len();
                    let col_w = if n > 0 { sw / n as i32 } else { sw };
                    let is_header = row_idx == 0;

                    scroll.begin();
                    for (ci, cell_val) in row_values.iter().enumerate() {
                        let cell_x = sx + (ci as i32) * col_w;
                        if is_header {
                            let mut lbl = Frame::new(cell_x, row_y, col_w, row_h, None);
                            lbl.set_label(cell_val);
                            lbl.set_frame(FrameType::FlatBox);
                            lbl.set_color(Color::from_rgb(200, 210, 230));
                            lbl.set_align(Align::Left | Align::Inside);
                        } else {
                            let mut inp = Output::new(cell_x, row_y, col_w, row_h, None);
                            inp.set_value(cell_val);
                            inp.set_frame(FrameType::ThinUpBox);
                            let sg_cb = sg_name.clone();
                            let ri = row_idx;
                            let col_i = ci;
                            inp.set_callback(move |i| {
                                let val = i.value();
                                STRING_GRIDS.with(|sg| {
                                    let mut grids = sg.borrow_mut();
                                    if let Some(state) = grids.get_mut(&sg_cb) {
                                        if ri < state.rows.len() && col_i < state.rows[ri].cols.len() {
                                            state.rows[ri].cols[col_i] = val.clone();
                                            state.selected_row = ri as i32;
                                        }
                                    }
                                });
                                rp_fire_event(&sg_cb, "onchange");
                            });
                        }
                    }
                    scroll.end();
                    scroll.redraw();
                }
            });
            v_null()
        }
        "cell" => {
            // Cell(row, col)
            let row = args.first().map(|v| v.to_i64()).unwrap_or(0);
            let col = args.get(1).map(|v| v.to_i64()).unwrap_or(0);
            STRING_GRIDS.with(|sg| {
                let grids = sg.borrow();
                if let Some(state) = grids.get(&name_lower) {
                    if (row as usize) < state.rows.len() {
                        let r = &state.rows[row as usize];
                        return v_str(r.get(col as usize));
                    }
                }
                v_str("")
            })
        }
        "setcell" => {
            // SetCell(row, col, value)
            let row = args.first().map(|v| v.to_i64()).unwrap_or(0);
            let col = args.get(1).map(|v| v.to_i64()).unwrap_or(0);
            let val = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
            STRING_GRIDS.with(|sg| {
                let mut grids = sg.borrow_mut();
                if let Some(state) = grids.get_mut(&name_lower) {
                    if (row as usize) < state.rows.len() {
                        let r = &mut state.rows[row as usize];
                        while r.cols.len() <= col as usize { r.cols.push(String::new()); }
                        r.cols[col as usize] = val;
                    }
                }
            });
            v_null()
        }
        "setsuggestions" => {
            let sugg_text = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            let suggs: Vec<String> = sugg_text.lines().map(|l| l.to_string()).collect();
            STRING_GRIDS.with(|sg| {
                let mut grids = sg.borrow_mut();
                let state = grids.entry(name_lower.clone()).or_insert_with(|| StringGridState {
                    rows: Vec::new(),
                    selected_row: -1,
                    cols: 2,
                    suggestions: Vec::new(),
                });
                state.suggestions = suggs;
            });
            v_null()
        }
        _ => {
            eprintln!("[WARN] StringGrid.{}() not implemented", method);
            v_null()
        }
    }
}

/// Get a string grid property (Rows, SelectedRow, Cols)
pub fn string_grid_get(name: &str, prop: &str) -> Option<Value> {
    let name_lower = name.to_lowercase();
    match prop.to_lowercase().as_str() {
        "rows" | "rowcount" => {
            Some(STRING_GRIDS.with(|sg| {
                let grids = sg.borrow();
                if let Some(state) = grids.get(&name_lower) {
                    v_int(state.rows.len() as i64)
                } else {
                    v_int(0)
                }
            }))
        }
        "selectedrow" => {
            Some(STRING_GRIDS.with(|sg| {
                let grids = sg.borrow();
                if let Some(state) = grids.get(&name_lower) {
                    v_int(state.selected_row as i64)
                } else {
                    v_int(-1)
                }
            }))
        }
        "cols" | "colcount" => {
            Some(STRING_GRIDS.with(|sg| {
                let grids = sg.borrow();
                if let Some(state) = grids.get(&name_lower) {
                    v_int(state.cols as i64)
                } else {
                    v_int(2)
                }
            }))
        }
        _ => None,
    }
}

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
            gui_close(name);
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

/// Canvas drawing commands stored for batch rendering
#[derive(Clone, Debug)]
enum DrawCmd {
    Line(i32, i32, i32, i32, Color),
    Rect(i32, i32, i32, i32, Color),
    FillRect(i32, i32, i32, i32, Color),
    Circle(i32, i32, i32, Color),
    DrawText(String, i32, i32, Color, i32),
    Ellipse(i32, i32, i32, i32, Color),
    Pixel(i32, i32, Color),
}

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
            // Render the current matplotlib plot and load it into this image widget.
            // The first arg is the matplotlib component name.
            let plot_name = args.first().map(|v| v.to_string_val()).unwrap_or_default();
            #[cfg(feature = "datascience")]
            {
                let png_path = crate::datascience::matplotlib_render_to_file(&plot_name);
                load_image_file(&name_lower, &png_path);
            }
            #[cfg(not(feature = "datascience"))]
            {
                eprintln!("[WARN] datascience not compiled — loadfromplot unavailable");
                let _ = plot_name;
            }
            v_null()
        }
        "clear" => {
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

/// Load an image file into an RImage widget.
fn load_image_file(name: &str, path: &str) {
    if let Ok(mut img) = SharedImage::load(path) {
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
    static CANVAS_CMDS: RefCell<HashMap<String, Vec<DrawCmd>>> = RefCell::new(HashMap::new());
}

pub fn canvas_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "line" => {
            let x1 = args.first().map(|v| v.to_i64()).unwrap_or(0) as i32;
            let y1 = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let x2 = args.get(2).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let y2 = args.get(3).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let color_val = args.get(4).map(|v| v.to_i64()).unwrap_or_else(|| rp_comp_get(name, "pencolor").to_i64());
            CANVAS_CMDS.with(|cmds| {
                cmds.borrow_mut().entry(name_lower.clone()).or_default()
                    .push(DrawCmd::Line(x1, y1, x2, y2, bgr_to_fltk_color(color_val)));
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "rect" => {
            let x = args.first().map(|v| v.to_i64()).unwrap_or(0) as i32;
            let y = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let w = args.get(2).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let h = args.get(3).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let color_val = args.get(4).map(|v| v.to_i64()).unwrap_or_else(|| rp_comp_get(name, "pencolor").to_i64());
            CANVAS_CMDS.with(|cmds| {
                cmds.borrow_mut().entry(name_lower.clone()).or_default()
                    .push(DrawCmd::Rect(x, y, w, h, bgr_to_fltk_color(color_val)));
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "fillrect" => {
            let x = args.first().map(|v| v.to_i64()).unwrap_or(0) as i32;
            let y = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let w = args.get(2).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let h = args.get(3).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let color_val = args.get(4).map(|v| v.to_i64()).unwrap_or_else(|| rp_comp_get(name, "brushcolor").to_i64());
            CANVAS_CMDS.with(|cmds| {
                cmds.borrow_mut().entry(name_lower.clone()).or_default()
                    .push(DrawCmd::FillRect(x, y, w, h, bgr_to_fltk_color(color_val)));
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "circle" => {
            let cx = args.first().map(|v| v.to_i64()).unwrap_or(0) as i32;
            let cy = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let r = args.get(2).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let color_val = args.get(3).map(|v| v.to_i64()).unwrap_or_else(|| rp_comp_get(name, "pencolor").to_i64());
            CANVAS_CMDS.with(|cmds| {
                cmds.borrow_mut().entry(name_lower.clone()).or_default()
                    .push(DrawCmd::Circle(cx, cy, r, bgr_to_fltk_color(color_val)));
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "drawtext" => {
            // Support both conventions:
            //   drawtext text, x, y [, color [, fontsize]]
            //   drawtext x, y, text [, color [, fontsize]] (if first arg is numeric)
            let first_is_number = match args.first() {
                Some(Value::Integer(_)) | Some(Value::Double(_)) => true,
                _ => false,
            };
            let (text, x, y) = if first_is_number {
                let xv = args.first().map(|v| v.to_i64()).unwrap_or(0) as i32;
                let yv = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as i32;
                let tv = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
                (tv, xv, yv)
            } else {
                let tv = args.first().map(|v| v.to_string_val()).unwrap_or_default();
                let xv = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as i32;
                let yv = args.get(2).map(|v| v.to_i64()).unwrap_or(0) as i32;
                (tv, xv, yv)
            };
            let color_val = args.get(3).map(|v| v.to_i64()).unwrap_or_else(|| rp_comp_get(name, "fontcolor").to_i64());
            let font_size = args.get(4).map(|v| v.to_i64() as i32).unwrap_or_else(|| rp_comp_get(name, "fontsize").to_i64() as i32);
            CANVAS_CMDS.with(|cmds| {
                cmds.borrow_mut().entry(name_lower.clone()).or_default()
                    .push(DrawCmd::DrawText(text, x, y, bgr_to_fltk_color(color_val), font_size));
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "clear" | "cls" => {
            CANVAS_CMDS.with(|cmds| {
                cmds.borrow_mut().remove(&name_lower);
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "rectangle" => canvas_method(name, "rect", args),
        "pset" | "setpixel" => {
            let px = args.first().map(|v| v.to_i64()).unwrap_or(0) as i32;
            let py = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let color_val = args.get(2).map(|v| v.to_i64()).unwrap_or_else(|| rp_comp_get(name, "pencolor").to_i64());
            CANVAS_CMDS.with(|cmds| {
                cmds.borrow_mut().entry(name_lower.clone()).or_default()
                    .push(DrawCmd::Pixel(px, py, bgr_to_fltk_color(color_val)));
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "ellipse" => {
            let ex = args.first().map(|v| v.to_i64()).unwrap_or(0) as i32;
            let ey = args.get(1).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let ew = args.get(2).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let eh = args.get(3).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let color_val = args.get(4).map(|v| v.to_i64()).unwrap_or_else(|| rp_comp_get(name, "pencolor").to_i64());
            CANVAS_CMDS.with(|cmds| {
                cmds.borrow_mut().entry(name_lower.clone()).or_default()
                    .push(DrawCmd::Ellipse(ex, ey, ew, eh, bgr_to_fltk_color(color_val)));
            });
            redraw_widget(&name_lower);
            v_null()
        }
        "paint" | "refresh" | "update" => {
            redraw_widget(&name_lower);
            v_null()
        }
        "show" => { gui_show(name); v_null() }
        "hide" => { gui_close(name); v_null() }
        _ => {
            eprintln!("[WARN] Canvas.{}() not implemented", method);
            v_null()
        }
    }
}

// ---------------------------------------------------------------------------
// QFormMDI methods
// ---------------------------------------------------------------------------

thread_local! {
    static MDI_CHILDREN: RefCell<HashMap<String, Vec<MdiChild>>> = RefCell::new(HashMap::new());
}

#[derive(Clone, Debug)]
struct MdiChild {
    title: String,
    comp_index: i32,
    group_key: String,
}

pub fn formmdi_method(name: &str, method: &str, args: &[Value]) -> Value {
    let name_lower = name.to_lowercase();
    match method {
        "showmodal" => {
            gui_showmodal(name);
            v_null()
        }
        "show" => {
            gui_show(name);
            v_null()
        }
        "close" | "hide" => {
            gui_close(name);
            v_null()
        }
        "addchild" => {
            // AddChild(handle, title, index, left, top, width, height, default_size)
            let _handle = args.first().map(|v| v.to_i64()).unwrap_or(0);
            let title = args.get(1).map(|v| v.to_string_val()).unwrap_or_default();
            let index = args.get(2).map(|v| v.to_i64()).unwrap_or(0) as i32;
            let _left = args.get(3).map(|v| v.to_i64()).unwrap_or(10) as i32;
            let _top = args.get(4).map(|v| v.to_i64()).unwrap_or(10) as i32;
            let _width = args.get(5).map(|v| v.to_i64()).unwrap_or(400) as i32;
            let _height = args.get(6).map(|v| v.to_i64()).unwrap_or(300) as i32;

            let group_key = format!("{}__mdi_child_{}", name_lower, index);
            MDI_CHILDREN.with(|mc| {
                let mut children = mc.borrow_mut();
                let child_list = children.entry(name_lower.clone()).or_default();
                child_list.push(MdiChild {
                    title: title.clone(),
                    comp_index: index,
                    group_key,
                });
                let count = child_list.len() as i64;
                rp_comp_set(name, "childcount", v_int(count));
            });
            v_null()
        }
        "closechild" => {
            // Close active child
            v_null()
        }
        "closeallchild" => {
            MDI_CHILDREN.with(|mc| {
                mc.borrow_mut().remove(&name_lower);
            });
            rp_comp_set(name, "childcount", v_int(0));
            v_null()
        }
        "cascadechild" | "sethorzchild" | "setvertchild" | "iconarrangechild" => {
            v_null()
        }
        "center" => {
            v_null()
        }
        _ => {
            eprintln!("[WARN] FormMDI.{}() not implemented", method);
            v_null()
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
                GuiWidget::Output(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::CheckButton(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::RadioButton(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Choice(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::HoldBrowser(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::TextEditor(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Group(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Tabs(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::MenuBar(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Progress(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
                GuiWidget::Scroll(ref mut w) => { if visible { w.show(); } else { w.hide(); } }
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
                GuiWidget::Output(ref mut w) => { let _ = w.set_value(text); }
                GuiWidget::Window(ref mut w) => { w.set_label(text); }
                _ => {}
            }
        }
    });
}

/// Update the text content of a TextEditor/TextBuffer.
pub fn gui_set_text(name: &str, text: &str) {
    let name_lower = name.to_lowercase();
    GUI_TEXT_BUFFERS.with(|tb| {
        let mut bufs = tb.borrow_mut();
        if let Some(buf) = bufs.get_mut(&name_lower) {
            buf.set_text(text);
        }
    });
}

/// Get the text content of a TextEditor/TextBuffer.
pub fn gui_get_text(name: &str) -> String {
    let name_lower = name.to_lowercase();
    GUI_TEXT_BUFFERS.with(|tb| {
        let bufs = tb.borrow();
        bufs.get(&name_lower).map(|b| b.text()).unwrap_or_default()
    })
}

/// Trigger a widget redraw.
fn redraw_widget(name: &str) {
    GUI_WIDGETS.with(|gw| {
        let mut widgets = gw.borrow_mut();
        if let Some(widget) = widgets.get_mut(name) {
            match widget {
                GuiWidget::Window(ref mut w) => { w.redraw(); }
                GuiWidget::Group(ref mut w) => { w.redraw(); }
                GuiWidget::Scroll(ref mut w) => { w.redraw(); }
                GuiWidget::Frame(ref mut w) => { w.redraw(); }
                GuiWidget::ImageFrame(ref mut w) => { w.redraw(); }
                GuiWidget::Button(ref mut w) => { w.redraw(); }
                GuiWidget::Input(ref mut w) => { w.redraw(); }
                GuiWidget::Output(ref mut w) => { w.redraw(); }
                GuiWidget::CheckButton(ref mut w) => { w.redraw(); }
                GuiWidget::RadioButton(ref mut w) => { w.redraw(); }
                GuiWidget::Choice(ref mut w) => { w.redraw(); }
                GuiWidget::HoldBrowser(ref mut w) => { w.redraw(); }
                GuiWidget::TextEditor(ref mut w) => { w.redraw(); }
                GuiWidget::Tabs(ref mut w) => { w.redraw(); }
                GuiWidget::MenuBar(ref mut w) => { w.redraw(); }
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
