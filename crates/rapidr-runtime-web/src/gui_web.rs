//! GUI web implementation — HTML5 DOM widget creation for all RapidR components.
//!
//! Each component type (RFORM, RBUTTON, RLABEL, etc.) maps to one or more
//! HTML elements styled with Tailwind CSS classes and absolute positioning.

use crate::value::{v_int, v_null, v_str, Value};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn document() -> web_sys::Document {
    web_sys::window()
        .expect("no window")
        .document()
        .expect("no document")
}

fn get_el(id: &str) -> Option<web_sys::HtmlElement> {
    document()
        .get_element_by_id(id)?
        .dyn_into::<web_sys::HtmlElement>()
        .ok()
}

pub fn create_el(tag: &str) -> web_sys::HtmlElement {
    document()
        .create_element(tag)
        .expect("create element failed")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("cast to HtmlElement failed")
}

/// Convert a BASIC BGR color integer to a CSS hex string.
fn bgr_to_css(bgr: i64) -> String {
    let r = bgr & 0xFF;
    let g = (bgr >> 8) & 0xFF;
    let b = (bgr >> 16) & 0xFF;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// Smart color conversion: if Value is a CSS string (#hex, named), use directly;
/// otherwise treat as BGR integer.
fn value_to_css_color(val: &Value) -> String {
    let s = val.to_string_val();
    if s.starts_with('#') || s.starts_with("rgb") || s.starts_with("hsl")
        || matches!(s.as_str(), "red" | "green" | "blue" | "black" | "white"
            | "yellow" | "cyan" | "magenta" | "orange" | "purple" | "pink"
            | "brown" | "gray" | "grey" | "steelblue" | "navy" | "teal"
            | "lime" | "olive" | "maroon" | "aqua" | "fuchsia" | "silver"
            | "transparent" | "coral" | "salmon" | "gold" | "indigo" | "violet")
    {
        s
    } else {
        bgr_to_css(val.to_i64())
    }
}

/// Compute an element ID from the RapidR component name.
fn comp_id(name: &str) -> String {
    format!("rr-{}", name.to_lowercase())
}

// ---------------------------------------------------------------------------
// Widget creation — one function per component type
// ---------------------------------------------------------------------------

pub fn gui_web_create_widget(name: &str, comp_type: &str, props: &HashMap<String, Value>) {
    let id = comp_id(name);
    match comp_type {
        "RFORM" => create_form(&id, props),
        "RBUTTON" => create_button(&id, name, props),
        "RLABEL" => create_label(&id, name, props),
        "REDIT" => create_edit(&id, name, props),
        "RMEMO" | "RRICHEDIT" => create_textarea(&id, name, props),
        "RPANEL" => create_panel(&id, name, props),
        "RCHECKBOX" => create_checkbox(&id, name, props),
        "RRADIOBUTTON" => create_radio(&id, name, props),
        "RCOMBOBOX" => create_select(&id, name, false, props),
        "RLISTBOX" => create_select(&id, name, true, props),
        "RTIMER" => { /* Timers are virtual — no DOM element, handled in object_web */ }
        "RIMAGE" => create_image(&id, name, props),
        "RCANVAS" => create_canvas(&id, name, props),
        "RSTRINGGRID" => create_table(&id, name, props),
        "RTABCONTROL" => create_tabcontrol(&id, name, props),
        "RTREEVIEW" => create_treeview(&id, name, props),
        "RMAINMENU" => create_mainmenu(&id, name, props),
        "RMENUITEM" => create_menuitem(&id, name, props),
        "RPOPUPMENU" => create_popupmenu(&id, name, props),
        "RGROUPBOX" => create_groupbox(&id, name, props),
        "RSTATUSBAR" => create_statusbar(&id, name, props),
        "RPROGRESS" | "RPROGRESSBAR" => create_progress(&id, name, props),
        "RSCROLLBOX" => create_scrollbox(&id, name, props),
        "RTRACKBAR" => create_range(&id, name, props),
        "RUPDOWN" => create_updown(&id, name, props),
        "RSCROLLBAR" => create_range(&id, name, props),
        "RTOOLBAR" => create_toolbar(&id, name, props),
        "RSPLITTER" => create_splitter(&id, name, props),
        "RLISTVIEW" => create_listview(&id, name, props),
        "RDATETIMEPICKER" => create_datetimepicker(&id, name, props),
        "RCODEEDITOR" => create_codeeditor(&id, name, props),
        "RDESIGNSURFACE" => create_panel(&id, name, props), // design surface is just a panel on web
        // Dialogs — these are virtual and use browser native dialogs
        "ROPENDIALOG" | "RSAVEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG" => { /* virtual */ }
        // Non-GUI components (SQLite, HTTP, etc.) — no DOM element
        "RSQLITE" | "RMYSQL" | "RSOCKET" | "RSERVERSOCKET" | "RHTTP"
        | "RFILESTREAM" | "RSTRINGLIST" | "RPRINTER" | "RFORMMDI"
        | "RNUM" | "RDATAFRAME" | "RPLOT" => { /* no DOM element */ }
        // Web-exclusive components
        "RWEBVIEW" => create_webview(&id, name, props),
        "RDOM" => create_dom_element(&id, name, props),
        "RJAVASCRIPT" => { /* virtual — no DOM element */ }
        "RWEBSTORAGE" => { /* virtual — no DOM element */ }
        "RWEBAUDIO" => create_audio(&id, name, props),
        "RWEBVIDEO" => create_video(&id, name, props),
        "RWEBNOTIFICATION" => { /* virtual — no DOM element */ }
        "RWEBGEOLOCATION" => { /* virtual — no DOM element */ }
        "RROUTER" => { /* virtual — hash-based routing, no DOM element */ }
        _ => {
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[WARN] Unknown component type '{}' for '{}'",
                comp_type, name
            )));
        }
    }
}

// ---------------------------------------------------------------------------
// Property get/set — universal DOM property access
// ---------------------------------------------------------------------------

pub fn gui_web_set_prop(name: &str, prop: &str, val: &Value) {
    let id = comp_id(name);
    let el = match get_el(&id) {
        Some(e) => e,
        None => return,
    };
    let s = val.to_string_val();
    let style = el.style();

    match prop {
        "caption" | "text" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                input.set_value(&s);
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                ta.set_value(&s);
            } else if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                sel.set_value(&s);
            } else if el.class_list().contains("rr-form") {
                // For forms, update the title text span — NOT the form's innerHTML,
                // which would destroy the titlebar and client area divs.
                if let Ok(Some(title_text)) = el.query_selector(".rr-form-title-text") {
                    title_text.set_text_content(Some(&s));
                }
            } else {
                el.set_inner_html(&s);
            }
        }
        "left" => {
            let _ = style.set_property("left", &format!("{}px", val.to_i64()));
        }
        "top" => {
            let _ = style.set_property("top", &format!("{}px", val.to_i64()));
        }
        "width" => {
            let v = val.to_i64();
            let _ = style.set_property("width", &format!("{}px", v));
            // For canvas, also set the canvas width attribute
            if let Ok(canvas) = el.clone().dyn_into::<web_sys::HtmlCanvasElement>() {
                canvas.set_width(v as u32);
            }
        }
        "height" => {
            let v = val.to_i64();
            let _ = style.set_property("height", &format!("{}px", v));
            if let Ok(canvas) = el.clone().dyn_into::<web_sys::HtmlCanvasElement>() {
                canvas.set_height(v as u32);
            }
        }
        "visible" => {
            if val.to_bool() {
                let _ = style.set_property("display", "");
            } else {
                let _ = style.set_property("display", "none");
            }
        }
        "enabled" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                input.set_disabled(!val.to_bool());
            } else if let Ok(btn) = el.clone().dyn_into::<web_sys::HtmlButtonElement>() {
                btn.set_disabled(!val.to_bool());
            } else if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                sel.set_disabled(!val.to_bool());
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                ta.set_disabled(!val.to_bool());
            }
        }
        "color" | "backcolor" => {
            let _ = style.set_property("background-color", &bgr_to_css(val.to_i64()));
        }
        "fontcolor" | "forecolor" => {
            let _ = style.set_property("color", &bgr_to_css(val.to_i64()));
        }
        "fontname" => {
            let _ = style.set_property("font-family", &s);
        }
        "fontsize" => {
            let _ = style.set_property("font-size", &format!("{}px", val.to_i64()));
        }
        "fontbold" => {
            let _ = style.set_property(
                "font-weight",
                if val.to_bool() { "bold" } else { "normal" },
            );
        }
        "fontitalic" => {
            let _ = style.set_property(
                "font-style",
                if val.to_bool() { "italic" } else { "normal" },
            );
        }
        "fontunderline" => {
            let _ = style.set_property(
                "text-decoration",
                if val.to_bool() { "underline" } else { "none" },
            );
        }
        "alignment" | "textalign" => {
            let align = match s.to_uppercase().as_str() {
                "0" | "LEFT" => "left",
                "1" | "RIGHT" => "right",
                "2" | "CENTER" => "center",
                _ => "left",
            };
            let _ = style.set_property("text-align", align);
        }
        "checked" | "value" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                input.set_checked(val.to_bool());
            }
        }
        "readonly" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                input.set_read_only(val.to_bool());
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                ta.set_read_only(val.to_bool());
            }
        }
        "passwordchar" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                if !s.is_empty() {
                    input.set_type("password");
                } else {
                    input.set_type("text");
                }
            }
        }
        "maxlength" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                input.set_max_length(val.to_i64() as i32);
            }
        }
        "selstart" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                let _ = input.set_selection_start(Some(val.to_i64() as u32));
            }
        }
        "sellength" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                let start = input.selection_start().unwrap_or(Some(0)).unwrap_or(0);
                let _ = input.set_selection_end(Some(start + val.to_i64() as u32));
            }
        }
        "listindex" | "itemindex" => {
            if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                sel.set_selected_index(val.to_i64() as i32);
            }
        }
        "min" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                input.set_min(&s);
            } else if let Ok(prog) = el.clone().dyn_into::<web_sys::HtmlProgressElement>() {
                // progress doesn't have min
                let _ = prog;
            }
        }
        "max" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                input.set_max(&s);
            } else if let Ok(prog) = el.clone().dyn_into::<web_sys::HtmlProgressElement>() {
                prog.set_max(val.to_f64());
            }
        }
        "position" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                input.set_value(&s);
            } else if let Ok(prog) = el.clone().dyn_into::<web_sys::HtmlProgressElement>() {
                prog.set_value(val.to_f64());
            }
        }
        "picture" | "src" => {
            if let Ok(img) = el.clone().dyn_into::<web_sys::HtmlImageElement>() {
                img.set_src(&s);
            } else if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                audio.set_src(&s);
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                video.set_src(&s);
            }
        }
        "stretch" => {
            if let Ok(_img) = el.clone().dyn_into::<web_sys::HtmlImageElement>() {
                if val.to_bool() {
                    let _ = style.set_property("object-fit", "fill");
                } else {
                    let _ = style.set_property("object-fit", "contain");
                }
            }
        }
        "tooltip" | "hint" => {
            el.set_title(&s);
        }
        "tabindex" => {
            // For tab controls, don't set DOM tabIndex — the property store handles it
            if !el.class_list().contains("rr-widget") || el.query_selector(".rr-tab-btn").ok().flatten().is_none() {
                el.set_tab_index(val.to_i64() as i32);
            }
        }
        "cursor" => {
            let cursor = match val.to_i64() {
                0 => "default",
                1 => "pointer",
                2 => "crosshair",
                3 => "text",
                4 => "wait",
                11 => "help",
                _ => "default",
            };
            let _ = style.set_property("cursor", cursor);
        }
        "bordercolor" => {
            let _ = style.set_property("border-color", &bgr_to_css(val.to_i64()));
        }
        "borderstyle" => {
            let bs = match val.to_i64() {
                0 => "none",
                1 => "solid",
                2 => "dashed",
                3 => "dotted",
                _ => "solid",
            };
            let _ = style.set_property("border-style", bs);
        }
        "borderwidth" => {
            let _ = style.set_property("border-width", &format!("{}px", val.to_i64()));
        }
        "scrollbars" => {
            let overflow = match val.to_i64() {
                0 => "hidden",
                1 => "scroll",
                2 => "auto",
                _ => "auto",
            };
            let _ = style.set_property("overflow", overflow);
        }
        "opacity" | "alpha" => {
            let alpha = val.to_f64();
            let _ = style.set_property("opacity", &format!("{}", alpha / 255.0));
        }
        // Web-exclusive: RWebView
        "html" => {
            el.set_inner_html(&s);
        }
        "url" => {
            if let Ok(iframe) = el.clone().dyn_into::<web_sys::HtmlIFrameElement>() {
                iframe.set_src(&s);
            }
        }
        "sandbox" => {
            if let Ok(iframe) = el.clone().dyn_into::<web_sys::HtmlIFrameElement>() {
                let _ = iframe
                    .set_attribute("sandbox", &s);
            }
        }
        // Web-exclusive: RDom
        "innerhtml" => {
            el.set_inner_html(&s);
        }
        "innertext" => {
            el.set_inner_text(&s);
        }
        "cssclass" => {
            el.set_class_name(&s);
        }
        "cssstyle" => {
            let _ = el.set_attribute("style", &s);
        }
        // Web-exclusive: RWebAudio/RWebVideo
        "volume" => {
            if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                audio.set_volume(val.to_f64());
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                video.set_volume(val.to_f64());
            }
        }
        "loop" => {
            if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                audio.set_loop(val.to_bool());
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                video.set_loop(val.to_bool());
            }
        }
        "autoplay" => {
            if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                audio.set_autoplay(val.to_bool());
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                video.set_autoplay(val.to_bool());
            }
        }
        "controls" => {
            if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                audio.set_controls(val.to_bool());
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                video.set_controls(val.to_bool());
            }
        }
        "poster" => {
            if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                video.set_poster(&s);
            }
        }
        _ => {
            // Store unrecognized props as data attributes
            let _ = el.set_attribute(&format!("data-rr-{}", prop), &s);
        }
    }
}

pub fn gui_web_get_prop(name: &str, prop: &str) -> Value {
    let id = comp_id(name);
    let el = match get_el(&id) {
        Some(e) => e,
        None => return v_null(),
    };
    let style = el.style();

    match prop {
        "caption" | "text" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                v_str(&input.value())
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                v_str(&ta.value())
            } else if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                v_str(&sel.value())
            } else {
                v_str(&el.inner_text())
            }
        }
        "left" => v_int(el.offset_left() as i64),
        "top" => v_int(el.offset_top() as i64),
        "width" => v_int(el.offset_width() as i64),
        "height" => v_int(el.offset_height() as i64),
        "visible" => {
            let display = style.get_property_value("display").unwrap_or_default();
            Value::Boolean(display != "none")
        }
        "enabled" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                Value::Boolean(!input.disabled())
            } else if let Ok(btn) = el.clone().dyn_into::<web_sys::HtmlButtonElement>() {
                Value::Boolean(!btn.disabled())
            } else {
                Value::Boolean(true)
            }
        }
        "checked" | "value" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                Value::Boolean(input.checked())
            } else {
                Value::Boolean(false)
            }
        }
        "listindex" | "itemindex" => {
            if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                v_int(sel.selected_index() as i64)
            } else {
                v_int(-1)
            }
        }
        "listcount" | "itemcount" => {
            if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                v_int(sel.length() as i64)
            } else {
                v_int(0)
            }
        }
        "position" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                v_str(&input.value())
            } else if let Ok(prog) = el.clone().dyn_into::<web_sys::HtmlProgressElement>() {
                Value::Double(prog.value())
            } else {
                v_int(0)
            }
        }
        "min" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                v_str(&input.min())
            } else {
                v_int(0)
            }
        }
        "max" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                v_str(&input.max())
            } else if let Ok(prog) = el.clone().dyn_into::<web_sys::HtmlProgressElement>() {
                Value::Double(prog.max())
            } else {
                v_int(100)
            }
        }
        "selstart" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                v_int(input.selection_start().unwrap_or(Some(0)).unwrap_or(0) as i64)
            } else {
                v_int(0)
            }
        }
        "seltext" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                let start = input.selection_start().unwrap_or(Some(0)).unwrap_or(0) as usize;
                let end = input.selection_end().unwrap_or(Some(0)).unwrap_or(0) as usize;
                let val = input.value();
                v_str(&val[start.min(val.len())..end.min(val.len())])
            } else {
                v_str("")
            }
        }
        "tooltip" | "hint" => v_str(&el.title()),
        // Web-exclusive: RDom
        "innerhtml" => v_str(&el.inner_html()),
        "innertext" => v_str(&el.inner_text()),
        "cssclass" => v_str(&el.class_name()),
        "tagname" => {
            v_str(&el.tag_name().to_lowercase())
        }
        // Web-exclusive: RWebAudio/RWebVideo
        "volume" => {
            if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                Value::Double(audio.volume())
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                Value::Double(video.volume())
            } else {
                Value::Double(1.0)
            }
        }
        "currenttime" => {
            if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                Value::Double(audio.current_time())
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                Value::Double(video.current_time())
            } else {
                Value::Double(0.0)
            }
        }
        "duration" => {
            if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                Value::Double(audio.duration())
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                Value::Double(video.duration())
            } else {
                Value::Double(0.0)
            }
        }
        "playing" | "paused" => {
            if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                Value::Boolean(!audio.paused())
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                Value::Boolean(!video.paused())
            } else {
                Value::Boolean(false)
            }
        }
        _ => {
            // Check data attributes
            match el.get_attribute(&format!("data-rr-{}", prop)) {
                Some(v) => v_str(&v),
                None => v_null(),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Component method dispatch
// ---------------------------------------------------------------------------

pub fn gui_web_method(name: &str, comp_type: &str, method: &str, args: &[Value]) -> Value {
    let id = comp_id(name);

    match (comp_type, method) {
        // LIST methods (RCOMBOBOX, RLISTBOX)
        (_, "additem") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                if let Ok(sel) = el.dyn_into::<web_sys::HtmlSelectElement>() {
                    let opt = document()
                        .create_element("option")
                        .unwrap()
                        .dyn_into::<web_sys::HtmlOptionElement>()
                        .unwrap();
                    let text = args[0].to_string_val();
                    opt.set_text(&text);
                    opt.set_value(&text);
                    let _ = sel.add_with_html_option_element(&opt);
                }
            }
            v_null()
        }
        // Canvas clear/cls — must come before generic (_, "clear")
        ("RCANVAS", "cls") | ("RCANVAS", "clear") => {
            canvas_clear(&id);
            v_null()
        }
        // RWEBSTORAGE clear — must come before generic (_, "clear")
        ("RWEBSTORAGE", "clear") => {
            crate::storage_web::storage_clear();
            v_null()
        }
        (_, "clear") => {
            if let Some(el) = get_el(&id) {
                if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                    while sel.length() > 0 {
                        sel.remove();
                    }
                } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                    ta.set_value("");
                } else {
                    el.set_inner_html("");
                }
            }
            v_null()
        }
        (_, "removeitem") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                if let Ok(sel) = el.dyn_into::<web_sys::HtmlSelectElement>() {
                    // web-sys remove() takes no args; remove specific option via child node
                    let idx = args[0].to_i64() as u32;
                    if let Some(opt) = sel.options().item(idx) {
                        let _ = sel.remove_child(&opt);
                    }
                }
            }
            v_null()
        }
        (_, "setfocus") | (_, "focus") => {
            if let Some(el) = get_el(&id) {
                let _ = el.focus();
            }
            v_null()
        }
        (_, "refresh") | (_, "repaint") | (_, "invalidate") => {
            // no-op on web — browser handles repainting
            v_null()
        }
        // RWEBNOTIFICATION show — must come before generic (_, "show")
        ("RWEBNOTIFICATION", "show") => {
            // Read title and body from stored properties
            let title = gui_web_get_prop(name, "title").to_string_val();
            let body = gui_web_get_prop(name, "body").to_string_val();
            let code = format!(
                r#"try {{ new Notification("{}", {{body: "{}"}}) }} catch(e) {{}}"#,
                title.replace('"', r#"\""#),
                body.replace('"', r#"\""#)
            );
            let _ = js_sys::eval(&code);
            v_null()
        }
        (_, "show") => {
            if let Some(el) = get_el(&id) {
                let _ = el.style().set_property("display", "");
                // If it's a form, bring to front
                if el.class_list().contains("rr-form") {
                    form_bring_to_front(&id);
                }
            }
            v_null()
        }
        (_, "hide") => {
            if let Some(el) = get_el(&id) {
                let _ = el.style().set_property("display", "none");
            }
            v_null()
        }
        (_, "close") if comp_type == "RFORM" => {
            if let Some(el) = get_el(&id) {
                let _ = el.style().set_property("display", "none");
            }
            v_null()
        }
        // Canvas draw methods
        ("RCANVAS", "line") if args.len() >= 4 => {
            canvas_line(&id, &args[0], &args[1], &args[2], &args[3], args.get(4));
            v_null()
        }
        ("RCANVAS", "rect" | "rectangle") if args.len() >= 4 => {
            canvas_rect(&id, &args[0], &args[1], &args[2], &args[3], args.get(4), false);
            v_null()
        }
        ("RCANVAS", "fillrect") if args.len() >= 4 => {
            canvas_rect(&id, &args[0], &args[1], &args[2], &args[3], args.get(4), true);
            v_null()
        }
        ("RCANVAS", "circle") if args.len() >= 3 => {
            canvas_circle(&id, &args[0], &args[1], &args[2], args.get(3), false);
            v_null()
        }
        ("RCANVAS", "fillcircle") if args.len() >= 3 => {
            canvas_circle(&id, &args[0], &args[1], &args[2], args.get(3), true);
            v_null()
        }
        ("RCANVAS", "drawtext" | "textout") if args.len() >= 3 => {
            canvas_text(&id, &args[0], &args[1], &args[2], args.get(3));
            v_null()
        }
        ("RCANVAS", "setpixel" | "pset") if args.len() >= 2 => {
            canvas_pixel(&id, &args[0], &args[1], args.get(2));
            v_null()
        }
        // StringGrid methods
        ("RSTRINGGRID", "setcell") if args.len() >= 3 => {
            grid_set_cell(&id, &args[0], &args[1], &args[2]);
            v_null()
        }
        ("RSTRINGGRID", "getcell") if args.len() >= 2 => {
            grid_get_cell(&id, &args[0], &args[1])
        }
        ("RSTRINGGRID", "setrowcount") if args.len() >= 1 => {
            grid_set_row_count(&id, args[0].to_i64() as usize);
            v_null()
        }
        ("RSTRINGGRID", "setcolcount") if args.len() >= 1 => {
            grid_set_col_count(&id, args[0].to_i64() as usize);
            v_null()
        }
        // TabControl methods
        ("RTABCONTROL", "addtab") if args.len() >= 1 => {
            tab_add(&id, &args[0].to_string_val());
            v_null()
        }
        ("RTABCONTROL", "removetab") if args.len() >= 1 => {
            tab_remove(&id, args[0].to_i64() as usize);
            v_null()
        }
        // TreeView methods
        ("RTREEVIEW", "addnode" | "additem") if args.len() >= 1 => {
            tree_add_node(&id, &args[0].to_string_val(), args.get(1).map(|v| v.to_string_val()).as_deref());
            v_null()
        }
        // Web-exclusive: RWebView
        ("RWEBVIEW", "sethtml") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                el.set_inner_html(&args[0].to_string_val());
            }
            v_null()
        }
        ("RWEBVIEW", "navigate") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                if let Ok(iframe) = el.dyn_into::<web_sys::HtmlIFrameElement>() {
                    iframe.set_src(&args[0].to_string_val());
                }
            }
            v_null()
        }
        // Web-exclusive: RDom
        ("RDOM", "create") => {
            // Created during rp_create_component
            v_null()
        }
        ("RDOM", "appendto") if args.len() >= 1 => {
            let parent_id = args[0].to_string_val();
            if let (Some(el), Some(parent)) = (get_el(&id), get_el(&comp_id(&parent_id))) {
                let _ = parent.append_child(&el);
            }
            v_null()
        }
        ("RDOM", "setattribute") if args.len() >= 2 => {
            if let Some(el) = get_el(&id) {
                let _ = el.set_attribute(&args[0].to_string_val(), &args[1].to_string_val());
            }
            v_null()
        }
        ("RDOM", "getattribute") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                match el.get_attribute(&args[0].to_string_val()) {
                    Some(v) => v_str(&v),
                    None => v_null(),
                }
            } else {
                v_null()
            }
        }
        ("RDOM", "addclass") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                let _ = el.class_list().add_1(&args[0].to_string_val());
            }
            v_null()
        }
        ("RDOM", "removeclass") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                let _ = el.class_list().remove_1(&args[0].to_string_val());
            }
            v_null()
        }
        ("RDOM", "toggleclass") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                let _ = el.class_list().toggle(&args[0].to_string_val());
            }
            v_null()
        }
        ("RDOM", "remove") => {
            if let Some(el) = get_el(&id) {
                el.remove();
            }
            v_null()
        }
        ("RDOM", "queryselector") if args.len() >= 1 => {
            match document().query_selector(&args[0].to_string_val()) {
                Ok(Some(el)) => v_str(&el.id()),
                _ => v_null(),
            }
        }
        // Web-exclusive: RJavaScript
        ("RJAVASCRIPT", "eval") if args.len() >= 1 => {
            match js_sys::eval(&args[0].to_string_val()) {
                Ok(result) => {
                    if let Some(s) = result.as_string() {
                        v_str(&s)
                    } else if let Some(n) = result.as_f64() {
                        Value::Double(n)
                    } else if let Some(b) = result.as_bool() {
                        Value::Boolean(b)
                    } else {
                        v_null()
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&e);
                    v_null()
                }
            }
        }
        ("RJAVASCRIPT", "call") if args.len() >= 1 => {
            let func_name = args[0].to_string_val();
            let js_args = js_sys::Array::new();
            for arg in args.iter().skip(1) {
                js_args.push(&JsValue::from_str(&arg.to_string_val()));
            }
            let code = format!("window['{}']", func_name);
            match js_sys::eval(&code) {
                Ok(func) => {
                    if let Ok(func) = func.dyn_into::<js_sys::Function>() {
                        match func.apply(&JsValue::NULL, &js_args) {
                            Ok(result) => {
                                if let Some(s) = result.as_string() {
                                    v_str(&s)
                                } else if let Some(n) = result.as_f64() {
                                    Value::Double(n)
                                } else {
                                    v_null()
                                }
                            }
                            Err(_) => v_null(),
                        }
                    } else {
                        v_null()
                    }
                }
                Err(_) => v_null(),
            }
        }
        // Web-exclusive: RWebStorage
        ("RWEBSTORAGE", "set") if args.len() >= 2 => {
            crate::storage_web::storage_set(&args[0].to_string_val(), &args[1].to_string_val());
            v_null()
        }
        ("RWEBSTORAGE", "get") if args.len() >= 1 => {
            crate::storage_web::storage_get(&args[0].to_string_val())
        }
        ("RWEBSTORAGE", "remove") if args.len() >= 1 => {
            crate::storage_web::storage_remove(&args[0].to_string_val());
            v_null()
        }
        // RWEBSTORAGE clear is handled above (before generic "clear")
        ("RWEBSTORAGE", "keys") => {
            crate::storage_web::storage_keys()
        }
        ("RWEBSTORAGE", "haskey") if args.len() >= 1 => {
            crate::storage_web::storage_has_key(&args[0].to_string_val())
        }
        // Web-exclusive: RWebAudio
        ("RWEBAUDIO", "play") => {
            if let Some(el) = get_el(&id) {
                if let Ok(audio) = el.dyn_into::<web_sys::HtmlAudioElement>() {
                    let _ = audio.play();
                }
            }
            v_null()
        }
        ("RWEBAUDIO", "pause") => {
            if let Some(el) = get_el(&id) {
                if let Ok(audio) = el.dyn_into::<web_sys::HtmlAudioElement>() {
                    audio.pause().ok();
                }
            }
            v_null()
        }
        ("RWEBAUDIO", "stop") => {
            if let Some(el) = get_el(&id) {
                if let Ok(audio) = el.dyn_into::<web_sys::HtmlAudioElement>() {
                    audio.pause().ok();
                    audio.set_current_time(0.0);
                }
            }
            v_null()
        }
        ("RWEBAUDIO", "seek") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                if let Ok(audio) = el.dyn_into::<web_sys::HtmlAudioElement>() {
                    audio.set_current_time(args[0].to_f64());
                }
            }
            v_null()
        }
        // Web-exclusive: RWebVideo — same pattern
        ("RWEBVIDEO", "play") => {
            if let Some(el) = get_el(&id) {
                if let Ok(video) = el.dyn_into::<web_sys::HtmlVideoElement>() {
                    let _ = video.play();
                }
            }
            v_null()
        }
        ("RWEBVIDEO", "pause") => {
            if let Some(el) = get_el(&id) {
                if let Ok(video) = el.dyn_into::<web_sys::HtmlVideoElement>() {
                    video.pause().ok();
                }
            }
            v_null()
        }
        ("RWEBVIDEO", "stop") => {
            if let Some(el) = get_el(&id) {
                if let Ok(video) = el.dyn_into::<web_sys::HtmlVideoElement>() {
                    video.pause().ok();
                    video.set_current_time(0.0);
                }
            }
            v_null()
        }
        ("RWEBVIDEO", "seek") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                if let Ok(video) = el.dyn_into::<web_sys::HtmlVideoElement>() {
                    video.set_current_time(args[0].to_f64());
                }
            }
            v_null()
        }
        ("RWEBVIDEO", "fullscreen") => {
            if let Some(el) = get_el(&id) {
                let _ = el.request_fullscreen();
            }
            v_null()
        }
        // Web-exclusive: RWebNotification
        ("RWEBNOTIFICATION", "requestpermission") => {
            let _ = js_sys::eval("Notification.requestPermission()");
            v_null()
        }
        // RWEBNOTIFICATION show is handled above (before generic "show")
        // Web-exclusive: RWebGeolocation
        ("RWEBGEOLOCATION", "getposition") => {
            let name_owned = name.to_string();
            let _ = js_sys::eval(&format!(
                r#"navigator.geolocation.getCurrentPosition(function(pos) {{
                    var el = document.getElementById('rr-{}');
                    if (el) {{
                        el.dataset.rrLatitude = pos.coords.latitude;
                        el.dataset.rrLongitude = pos.coords.longitude;
                        el.dataset.rrAccuracy = pos.coords.accuracy;
                    }}
                }})"#,
                name_owned.to_lowercase()
            ));
            v_null()
        }
        // Web-exclusive: RRouter
        ("RROUTER", "navigate") if args.len() >= 1 => {
            let route = args[0].to_string_val();
            if let Some(window) = web_sys::window() {
                if let Ok(loc) = window.location().set_hash(&route) {
                    let _ = loc;
                }
            }
            v_null()
        }
        ("RROUTER", "back") => {
            if let Some(window) = web_sys::window() {
                let _ = window.history().map(|h| h.back());
            }
            v_null()
        }
        ("RROUTER", "forward") => {
            if let Some(window) = web_sys::window() {
                let _ = window.history().map(|h| h.forward());
            }
            v_null()
        }
        // Fallback
        _ => {
            web_sys::console::warn_1(&JsValue::from_str(&format!(
                "[WARN] Unknown method '{}.{}()'",
                name, method
            )));
            v_null()
        }
    }
}

// ---------------------------------------------------------------------------
// Form creation
// ---------------------------------------------------------------------------

fn create_form(id: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_id(id);
    el.set_class_name("rr-form");

    let style = el.style();
    let _ = style.set_property("position", "absolute");
    let _ = style.set_property("background-color", "#f0f0f0");
    let _ = style.set_property("border", "1px solid #999");
    let _ = style.set_property("border-radius", "6px");
    let _ = style.set_property("box-shadow", "0 4px 16px rgba(0,0,0,0.25)");
    let _ = style.set_property("overflow", "hidden");
    let _ = style.set_property("font-family", "'Segoe UI', Tahoma, Geneva, Verdana, sans-serif");
    let _ = style.set_property("font-size", "13px");
    let _ = style.set_property("z-index", "10");

    apply_geometry(&el, props, 100, 100, 640, 480);

    // Title bar
    let titlebar = create_el("div");
    titlebar.set_class_name("rr-form-titlebar");
    let tb_style = titlebar.style();
    let _ = tb_style.set_property("display", "flex");
    let _ = tb_style.set_property("align-items", "center");
    let _ = tb_style.set_property("background", "linear-gradient(to bottom, #4a90d9, #357abd)");
    let _ = tb_style.set_property("color", "white");
    let _ = tb_style.set_property("padding", "0 4px 0 10px");
    let _ = tb_style.set_property("height", "29px");
    let _ = tb_style.set_property("font-weight", "bold");
    let _ = tb_style.set_property("font-size", "13px");
    let _ = tb_style.set_property("user-select", "none");
    let _ = tb_style.set_property("cursor", "default");

    // Title text (flex-grow to push buttons right)
    let title_span = create_el("span");
    title_span.set_class_name("rr-form-title-text");
    let _ = title_span.style().set_property("flex", "1");
    let _ = title_span.style().set_property("overflow", "hidden");
    let _ = title_span.style().set_property("text-overflow", "ellipsis");
    let _ = title_span.style().set_property("white-space", "nowrap");
    let caption = props
        .get("caption")
        .map(|v| v.to_string_val())
        .unwrap_or_default();
    title_span.set_inner_text(&caption);
    let _ = titlebar.append_child(&title_span);

    // Window control buttons (minimize, maximize, close)
    let btn_style = "border:none;background:transparent;color:white;font-size:16px;\
        width:28px;height:24px;cursor:pointer;display:flex;align-items:center;\
        justify-content:center;border-radius:3px;margin-left:2px;";
    let form_id_owned = id.to_string();

    // Minimize button
    let btn_min = create_el("button");
    btn_min.set_class_name("rr-form-btn-min");
    let _ = btn_min.set_attribute("style", btn_style);
    btn_min.set_inner_html("&#x2212;"); // minus sign
    let _ = btn_min.set_attribute("title", "Minimize");
    {
        let fid = form_id_owned.clone();
        let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            e.stop_propagation();
            form_minimize(&fid);
        });
        let _ = btn_min.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
        cb.forget();
    }
    let _ = titlebar.append_child(&btn_min);

    // Maximize button
    let btn_max = create_el("button");
    btn_max.set_class_name("rr-form-btn-max");
    let _ = btn_max.set_attribute("style", btn_style);
    btn_max.set_inner_html("&#x25A1;"); // square
    let _ = btn_max.set_attribute("title", "Maximize");
    {
        let fid = form_id_owned.clone();
        let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            e.stop_propagation();
            form_maximize(&fid);
        });
        let _ = btn_max.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
        cb.forget();
    }
    let _ = titlebar.append_child(&btn_max);

    // Close button
    let btn_close = create_el("button");
    btn_close.set_class_name("rr-form-btn-close");
    let _ = btn_close.set_attribute("style", btn_style);
    btn_close.set_inner_html("&#x2715;"); // X
    let _ = btn_close.set_attribute("title", "Close");
    {
        let fid = form_id_owned.clone();
        let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            e.stop_propagation();
            form_close(&fid);
        });
        let _ = btn_close.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
        cb.forget();
    }
    let _ = titlebar.append_child(&btn_close);

    let _ = el.append_child(&titlebar);

    // Client area
    let client = create_el("div");
    client.set_id(&format!("{}-client", id));
    let c_style = client.style();
    let _ = c_style.set_property("position", "relative");
    let _ = c_style.set_property("width", "100%");
    let _ = c_style.set_property("height", "calc(100% - 29px)");
    let _ = c_style.set_property("overflow", "auto");
    let _ = el.append_child(&client);

    // Hidden initially — shown via gui_web_finalize()
    let _ = style.set_property("display", "none");

    // Click-to-front: bring form to top of z-stack
    {
        let fid = id.to_string();
        let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |_e: web_sys::MouseEvent| {
            form_bring_to_front(&fid);
        });
        let _ = el.add_event_listener_with_callback("mousedown", cb.as_ref().unchecked_ref());
        cb.forget();
    }

    // Titlebar drag-to-move
    setup_form_drag(&titlebar, id);

    let _ = document().body().unwrap().append_child(&el);
}

fn get_parent_client(parent_name: &Option<String>) -> web_sys::HtmlElement {
    if let Some(parent) = parent_name {
        let parent_id = comp_id(parent);
        // Try to find a client area inside the parent
        if let Some(client) = get_el(&format!("{}-client", parent_id)) {
            return client;
        }
        if let Some(parent_el) = get_el(&parent_id) {
            return parent_el;
        }
    }
    // Fallback: append to body (gui_web_finalize will reparent into the form)
    document()
        .body()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
}

fn apply_geometry(el: &web_sys::HtmlElement, props: &HashMap<String, Value>, dl: i64, dt: i64, dw: i64, dh: i64) {
    let style = el.style();
    let left = props.get("left").map(|v| v.to_i64()).unwrap_or(dl);
    let top = props.get("top").map(|v| v.to_i64()).unwrap_or(dt);
    let width = props.get("width").map(|v| v.to_i64()).unwrap_or(dw);
    let height = props.get("height").map(|v| v.to_i64()).unwrap_or(dh);
    let _ = style.set_property("left", &format!("{}px", left));
    let _ = style.set_property("top", &format!("{}px", top));
    let _ = style.set_property("width", &format!("{}px", width));
    let _ = style.set_property("height", &format!("{}px", height));
}

pub fn setup_widget(el: &web_sys::HtmlElement, id: &str, name: &str, props: &HashMap<String, Value>) {
    el.set_id(id);
    let _ = el.set_attribute("data-rr-name", name);
    let style = el.style();
    let _ = style.set_property("position", "absolute");
    let _ = style.set_property("box-sizing", "border-box");
    apply_geometry(el, props, 0, 0, 100, 25);

    // Append to parent form's client area
    let parent = props.get("parent").map(|v| v.to_string_val());
    let parent_el = get_parent_client(&parent);
    let _ = parent_el.append_child(el);
}

// ---------------------------------------------------------------------------
// Individual widget creators
// ---------------------------------------------------------------------------

fn create_button(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("button");
    let caption = props.get("caption").map(|v| v.to_string_val()).unwrap_or_default();
    el.set_inner_text(&caption);
    el.set_class_name("rr-widget");
    setup_widget(&el, id, name, props);
}

fn create_label(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("span");
    let caption = props.get("caption").map(|v| v.to_string_val()).unwrap_or_default();
    el.set_inner_text(&caption);
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("overflow", "hidden");
    let _ = el.style().set_property("white-space", "nowrap");
    setup_widget(&el, id, name, props);
}

fn create_edit(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("input");
    if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
        input.set_type("text");
        if let Some(text) = props.get("text") {
            input.set_value(&text.to_string_val());
        }
    }
    el.set_class_name("rr-widget");
    setup_widget(&el, id, name, props);
}

fn create_textarea(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("textarea");
    if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
        if let Some(text) = props.get("text") {
            ta.set_value(&text.to_string_val());
        }
    }
    el.set_class_name("rr-widget");
    setup_widget(&el, id, name, props);
}

fn create_panel(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("overflow", "hidden");
    let _ = el.style().set_property("border", "1px solid #ccc");
    let _ = el.style().set_property("background", "#fafafa");
    setup_widget(&el, id, name, props);
}

fn create_checkbox(id: &str, name: &str, props: &HashMap<String, Value>) {
    let wrapper = create_el("label");
    wrapper.set_class_name("rr-widget");

    let cb = document().create_element("input").unwrap();
    let _ = cb.set_attribute("type", "checkbox");
    let _ = cb.set_attribute("id", &format!("{}-cb", id));
    let _ = wrapper.append_child(&cb);

    let span = create_el("span");
    let caption = props.get("caption").map(|v| v.to_string_val()).unwrap_or_default();
    span.set_inner_text(&caption);
    let _ = wrapper.append_child(&span);

    wrapper.set_id(id);
    let _ = wrapper.set_attribute("data-rr-name", name);
    let style = wrapper.style();
    let _ = style.set_property("position", "absolute");
    let _ = style.set_property("box-sizing", "border-box");
    apply_geometry(&wrapper, props, 0, 0, 120, 25);
    let parent = props.get("parent").map(|v| v.to_string_val());
    let _ = get_parent_client(&parent).append_child(&wrapper);
}

fn create_radio(id: &str, name: &str, props: &HashMap<String, Value>) {
    let wrapper = create_el("label");
    wrapper.set_class_name("rr-widget");

    let rb = document().create_element("input").unwrap();
    let _ = rb.set_attribute("type", "radio");
    let _ = rb.set_attribute("id", &format!("{}-rb", id));
    // Group radio buttons by parent
    let group = props.get("parent").map(|v| v.to_string_val()).unwrap_or_else(|| "default".to_string());
    let _ = rb.set_attribute("name", &format!("rr-radio-{}", group.to_lowercase()));
    let _ = wrapper.append_child(&rb);

    let span = create_el("span");
    let caption = props.get("caption").map(|v| v.to_string_val()).unwrap_or_default();
    span.set_inner_text(&caption);
    let _ = wrapper.append_child(&span);

    wrapper.set_id(id);
    let _ = wrapper.set_attribute("data-rr-name", name);
    let style = wrapper.style();
    let _ = style.set_property("position", "absolute");
    let _ = style.set_property("box-sizing", "border-box");
    apply_geometry(&wrapper, props, 0, 0, 120, 25);
    let parent = props.get("parent").map(|v| v.to_string_val());
    let _ = get_parent_client(&parent).append_child(&wrapper);
}

fn create_select(id: &str, name: &str, multiple: bool, props: &HashMap<String, Value>) {
    let el = create_el("select");
    if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
        sel.set_multiple(multiple);
    }
    el.set_class_name("rr-widget");
    setup_widget(&el, id, name, props);
}

fn create_image(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("img");
    if let Ok(img) = el.clone().dyn_into::<web_sys::HtmlImageElement>() {
        if let Some(src) = props.get("picture").or_else(|| props.get("src")) {
            img.set_src(&src.to_string_val());
        }
    }
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("object-fit", "contain");
    setup_widget(&el, id, name, props);
}

fn create_canvas(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("canvas");
    if let Ok(canvas) = el.clone().dyn_into::<web_sys::HtmlCanvasElement>() {
        let w = props.get("width").map(|v| v.to_i64()).unwrap_or(400) as u32;
        let h = props.get("height").map(|v| v.to_i64()).unwrap_or(300) as u32;
        canvas.set_width(w);
        canvas.set_height(h);
    }
    el.set_class_name("rr-widget");
    setup_widget(&el, id, name, props);
}

fn create_table(id: &str, name: &str, props: &HashMap<String, Value>) {
    let wrapper = create_el("div");
    wrapper.set_class_name("rr-widget");
    let _ = wrapper.style().set_property("overflow", "auto");
    let _ = wrapper.style().set_property("border", "1px solid #aaa");
    let _ = wrapper.style().set_property("background", "white");

    let table = create_el("table");
    table.set_id(&format!("{}-table", id));
    table.set_class_name("rr-grid");
    let _ = wrapper.append_child(&table);

    let rows = props.get("rowcount").map(|v| v.to_i64()).unwrap_or(5) as usize;
    let cols = props.get("colcount").map(|v| v.to_i64()).unwrap_or(3) as usize;
    grid_init_cells(&format!("{}-table", id), rows, cols);

    setup_widget(&wrapper, id, name, props);
}

fn create_tabcontrol(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("border", "1px solid #aaa");
    let _ = el.style().set_property("background", "white");

    // Tab bar
    let tab_bar = create_el("div");
    tab_bar.set_id(&format!("{}-tabs", id));
    let _ = tab_bar.style().set_property("display", "flex");
    let _ = tab_bar.style().set_property("border-bottom", "1px solid #ccc");
    let _ = tab_bar.style().set_property("background", "#f0f0f0");
    let _ = el.append_child(&tab_bar);

    // Tab content area
    let content = create_el("div");
    content.set_id(&format!("{}-content", id));
    let _ = content.style().set_property("position", "relative");
    let _ = content.style().set_property("width", "100%");
    let _ = content.style().set_property("height", "calc(100% - 32px)");
    let _ = el.append_child(&content);

    setup_widget(&el, id, name, props);
}

fn create_treeview(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("border", "1px solid #aaa");
    let _ = el.style().set_property("background", "white");
    let _ = el.style().set_property("overflow", "auto");
    let _ = el.style().set_property("font-size", "13px");
    let ul = create_el("ul");
    ul.set_id(&format!("{}-tree", id));
    let _ = ul.style().set_property("list-style", "none");
    let _ = ul.style().set_property("padding-left", "16px");
    let _ = el.append_child(&ul);
    setup_widget(&el, id, name, props);
}

fn create_mainmenu(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("nav");
    let _ = el.style().set_property("display", "flex");
    let _ = el.style().set_property("background", "#f0f0f0");
    let _ = el.style().set_property("border-bottom", "1px solid #ccc");
    let _ = el.style().set_property("font-size", "13px");
    el.set_id(id);
    let _ = el.set_attribute("data-rr-name", name);
    let style = el.style();
    let _ = style.set_property("position", "absolute");
    let _ = style.set_property("left", "0");
    let _ = style.set_property("top", "0");
    let _ = style.set_property("width", "100%");
    let _ = style.set_property("height", "28px");
    let _ = style.set_property("z-index", "100");
    let parent = props.get("parent").map(|v| v.to_string_val());
    let _ = get_parent_client(&parent).append_child(&el);
}

fn create_menuitem(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    let caption = props.get("caption").map(|v| v.to_string_val()).unwrap_or_default();
    el.set_inner_text(&caption);
    let _ = el.style().set_property("padding", "4px 12px");
    let _ = el.style().set_property("cursor", "pointer");
    el.set_id(id);
    let _ = el.set_attribute("data-rr-name", name);
    let parent = props.get("parent").map(|v| v.to_string_val());
    let _ = get_parent_client(&parent).append_child(&el);
}

fn create_popupmenu(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    let _ = el.style().set_property("position", "absolute");
    let _ = el.style().set_property("background", "white");
    let _ = el.style().set_property("border", "1px solid #ccc");
    let _ = el.style().set_property("box-shadow", "0 2px 8px rgba(0,0,0,0.15)");
    let _ = el.style().set_property("border-radius", "4px");
    let _ = el.style().set_property("padding", "4px 0");
    let _ = el.style().set_property("z-index", "50");
    el.set_id(id);
    let _ = el.set_attribute("data-rr-name", name);
    let _ = el.style().set_property("display", "none");
    let parent = props.get("parent").map(|v| v.to_string_val());
    let _ = get_parent_client(&parent).append_child(&el);
}

fn create_groupbox(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("fieldset");
    el.set_class_name("rr-widget");
    let legend = create_el("legend");
    let caption = props.get("caption").map(|v| v.to_string_val()).unwrap_or_default();
    legend.set_inner_text(&caption);
    let _ = el.append_child(&legend);
    setup_widget(&el, id, name, props);
}

fn create_statusbar(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    let _ = el.style().set_property("background", "#e8e8e8");
    let _ = el.style().set_property("border-top", "1px solid #ccc");
    let _ = el.style().set_property("font-size", "13px");
    let _ = el.style().set_property("padding", "2px 8px");
    el.set_id(id);
    let _ = el.set_attribute("data-rr-name", name);
    let style = el.style();
    let _ = style.set_property("position", "absolute");
    let _ = style.set_property("bottom", "0");
    let _ = style.set_property("left", "0");
    let _ = style.set_property("width", "100%");
    let _ = style.set_property("height", "24px");
    let parent = props.get("parent").map(|v| v.to_string_val());
    let _ = get_parent_client(&parent).append_child(&el);
}

fn create_progress(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("progress");
    if let Ok(prog) = el.clone().dyn_into::<web_sys::HtmlProgressElement>() {
        let max = props.get("max").map(|v| v.to_f64()).unwrap_or(100.0);
        prog.set_max(max);
        let val = props.get("position").map(|v| v.to_f64()).unwrap_or(0.0);
        prog.set_value(val);
    }
    el.set_class_name("rr-widget");
    setup_widget(&el, id, name, props);
}

fn create_scrollbox(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("border", "1px solid #aaa");
    let _ = el.style().set_property("background", "white");
    let _ = el.style().set_property("overflow", "auto");
    setup_widget(&el, id, name, props);
}

fn create_range(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("input");
    if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
        input.set_type("range");
        let min = props.get("min").map(|v| v.to_string_val()).unwrap_or_else(|| "0".to_string());
        let max = props.get("max").map(|v| v.to_string_val()).unwrap_or_else(|| "100".to_string());
        input.set_min(&min);
        input.set_max(&max);
    }
    el.set_class_name("rr-widget");
    setup_widget(&el, id, name, props);
}

fn create_updown(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("input");
    if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
        input.set_type("number");
        let min = props.get("min").map(|v| v.to_string_val()).unwrap_or_else(|| "0".to_string());
        let max = props.get("max").map(|v| v.to_string_val()).unwrap_or_else(|| "100".to_string());
        input.set_min(&min);
        input.set_max(&max);
    }
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("width", "80px");
    setup_widget(&el, id, name, props);
}

fn create_toolbar(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    let _ = el.style().set_property("display", "flex");
    let _ = el.style().set_property("align-items", "center");
    let _ = el.style().set_property("gap", "4px");
    let _ = el.style().set_property("background", "#f0f0f0");
    let _ = el.style().set_property("border-bottom", "1px solid #ccc");
    let _ = el.style().set_property("padding", "4px");
    el.set_id(id);
    let _ = el.set_attribute("data-rr-name", name);
    let style = el.style();
    let _ = style.set_property("position", "absolute");
    let _ = style.set_property("left", "0");
    let _ = style.set_property("top", "0");
    let _ = style.set_property("width", "100%");
    let parent = props.get("parent").map(|v| v.to_string_val());
    let _ = get_parent_client(&parent).append_child(&el);
}

fn create_splitter(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("background", "#ccc");
    let _ = el.style().set_property("cursor", "col-resize");
    let _ = el.style().set_property("width", "4px");
    setup_widget(&el, id, name, props);
}

fn create_listview(id: &str, name: &str, props: &HashMap<String, Value>) {
    // Use a table-based list view
    let wrapper = create_el("div");
    wrapper.set_class_name("rr-widget");
    let _ = wrapper.style().set_property("overflow", "auto");
    let _ = wrapper.style().set_property("border", "1px solid #aaa");
    let _ = wrapper.style().set_property("background", "white");
    let table = create_el("table");
    table.set_id(&format!("{}-table", id));
    table.set_class_name("rr-grid");
    let _ = wrapper.append_child(&table);
    setup_widget(&wrapper, id, name, props);
}

fn create_datetimepicker(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("input");
    if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
        input.set_type("datetime-local");
    }
    el.set_class_name("rr-widget");
    setup_widget(&el, id, name, props);
}

fn create_codeeditor(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("textarea");
    if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
        ta.set_wrap("off");
        if let Some(text) = props.get("text") {
            ta.set_value(&text.to_string_val());
        }
    }
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("background", "#1a1a2e");
    let _ = el.style().set_property("color", "#4ade80");
    let _ = el.style().set_property("padding", "8px");
    let _ = el.style().set_property("font-family", "monospace");
    let _ = el.style().set_property("font-size", "13px");
    let _ = el.style().set_property("resize", "none");
    let _ = el.style().set_property("border", "1px solid #444");
    let _ = el.style().set_property("tab-size", "4");
    setup_widget(&el, id, name, props);
}

// ---------------------------------------------------------------------------
// Web-exclusive widget creators
// ---------------------------------------------------------------------------

fn create_webview(id: &str, name: &str, props: &HashMap<String, Value>) {
    let has_url = props.contains_key("url");
    if has_url {
        let el = create_el("iframe");
        if let Ok(iframe) = el.clone().dyn_into::<web_sys::HtmlIFrameElement>() {
            if let Some(url) = props.get("url") {
                iframe.set_src(&url.to_string_val());
            }
            let _ = iframe.set_attribute("sandbox", "allow-scripts allow-same-origin");
        }
        el.set_class_name("rr-widget");
        let _ = el.style().set_property("border", "1px solid #aaa");
        setup_widget(&el, id, name, props);
    } else {
        let el = create_el("div");
        el.set_class_name("rr-widget");
        let _ = el.style().set_property("border", "1px solid #aaa");
        let _ = el.style().set_property("background", "white");
        let _ = el.style().set_property("overflow", "auto");
        if let Some(html) = props.get("html") {
            el.set_inner_html(&html.to_string_val());
        }
        setup_widget(&el, id, name, props);
    }
}

fn create_dom_element(id: &str, name: &str, props: &HashMap<String, Value>) {
    let tag = props
        .get("tagname")
        .map(|v| v.to_string_val())
        .unwrap_or_else(|| "div".to_string());
    let el = create_el(&tag);
    if let Some(css_class) = props.get("cssclass") {
        el.set_class_name(&css_class.to_string_val());
    }
    if let Some(css_style) = props.get("cssstyle") {
        let _ = el.set_attribute("style", &css_style.to_string_val());
    }
    if let Some(inner_html) = props.get("innerhtml") {
        el.set_inner_html(&inner_html.to_string_val());
    }
    if let Some(inner_text) = props.get("innertext") {
        el.set_inner_text(&inner_text.to_string_val());
    }
    el.set_id(id);
    let _ = el.set_attribute("data-rr-name", name);
    let _ = el.style().set_property("position", "absolute");
    let _ = el.style().set_property("box-sizing", "border-box");
    apply_geometry(&el, props, 0, 0, 100, 25);

    // Append to parent if specified, otherwise to body
    let parent = props.get("parentid").or_else(|| props.get("parent")).map(|v| v.to_string_val());
    let parent_el = get_parent_client(&parent);
    let _ = parent_el.append_child(&el);
}

fn create_audio(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("audio");
    if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
        if let Some(src) = props.get("src") {
            audio.set_src(&src.to_string_val());
        }
        audio.set_controls(props.get("controls").map(|v| v.to_bool()).unwrap_or(false));
        audio.set_loop(props.get("loop").map(|v| v.to_bool()).unwrap_or(false));
    }
    el.set_id(id);
    let _ = el.set_attribute("data-rr-name", name);
    // Audio elements are typically invisible unless controls are shown
    let parent = props.get("parent").map(|v| v.to_string_val());
    let _ = get_parent_client(&parent).append_child(&el);
}

fn create_video(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("video");
    if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
        if let Some(src) = props.get("src") {
            video.set_src(&src.to_string_val());
        }
        video.set_controls(props.get("controls").map(|v| v.to_bool()).unwrap_or(true));
        video.set_loop(props.get("loop").map(|v| v.to_bool()).unwrap_or(false));
        if let Some(poster) = props.get("poster") {
            video.set_poster(&poster.to_string_val());
        }
    }
    el.set_class_name("rr-widget");
    setup_widget(&el, id, name, props);
}

// ---------------------------------------------------------------------------
// Canvas drawing helpers
// ---------------------------------------------------------------------------

fn get_canvas_ctx(id: &str) -> Option<web_sys::CanvasRenderingContext2d> {
    let el = get_el(id)?;
    let canvas = el.dyn_into::<web_sys::HtmlCanvasElement>().ok()?;
    canvas
        .get_context("2d")
        .ok()?
        .map(|ctx| ctx.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
        .flatten()
}

fn canvas_clear(id: &str) {
    if let Some(el) = get_el(id) {
        if let Ok(canvas) = el.dyn_into::<web_sys::HtmlCanvasElement>() {
            if let Some(ctx) = get_canvas_ctx(id) {
                ctx.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
            }
        }
    }
}

fn canvas_line(id: &str, x1: &Value, y1: &Value, x2: &Value, y2: &Value, color: Option<&Value>) {
    if let Some(ctx) = get_canvas_ctx(id) {
        if let Some(c) = color {
            ctx.set_stroke_style_str(&value_to_css_color(c));
        }
        ctx.begin_path();
        ctx.move_to(x1.to_f64(), y1.to_f64());
        ctx.line_to(x2.to_f64(), y2.to_f64());
        ctx.stroke();
    }
}

fn canvas_rect(id: &str, x1v: &Value, y1v: &Value, x2v: &Value, y2v: &Value, color: Option<&Value>, fill: bool) {
    if let Some(ctx) = get_canvas_ctx(id) {
        if let Some(c) = color {
            let css = value_to_css_color(c);
            if fill {
                ctx.set_fill_style_str(&css);
            } else {
                ctx.set_stroke_style_str(&css);
            }
        }
        // Convert (x1, y1, x2, y2) to (x, y, w, h)
        let x1 = x1v.to_f64();
        let y1 = y1v.to_f64();
        let x2 = x2v.to_f64();
        let y2 = y2v.to_f64();
        let x = x1.min(x2);
        let y = y1.min(y2);
        let w = (x2 - x1).abs();
        let h = (y2 - y1).abs();
        if fill {
            ctx.fill_rect(x, y, w, h);
        } else {
            ctx.stroke_rect(x, y, w, h);
        }
    }
}

fn canvas_circle(id: &str, cx: &Value, cy: &Value, r: &Value, color: Option<&Value>, fill: bool) {
    if let Some(ctx) = get_canvas_ctx(id) {
        if let Some(c) = color {
            let css = value_to_css_color(c);
            if fill {
                ctx.set_fill_style_str(&css);
            } else {
                ctx.set_stroke_style_str(&css);
            }
        }
        ctx.begin_path();
        let _ = ctx.arc(
            cx.to_f64(),
            cy.to_f64(),
            r.to_f64(),
            0.0,
            std::f64::consts::PI * 2.0,
        );
        if fill {
            ctx.fill();
        } else {
            ctx.stroke();
        }
    }
}

fn canvas_text(id: &str, x: &Value, y: &Value, text: &Value, color: Option<&Value>) {
    if let Some(ctx) = get_canvas_ctx(id) {
        if let Some(c) = color {
            ctx.set_fill_style_str(&value_to_css_color(c));
        }
        let _ = ctx.fill_text(&text.to_string_val(), x.to_f64(), y.to_f64());
    }
}

fn canvas_pixel(id: &str, x: &Value, y: &Value, color: Option<&Value>) {
    if let Some(ctx) = get_canvas_ctx(id) {
        if let Some(c) = color {
            ctx.set_fill_style_str(&value_to_css_color(c));
        }
        ctx.fill_rect(x.to_f64(), y.to_f64(), 1.0, 1.0);
    }
}

// ---------------------------------------------------------------------------
// StringGrid helpers
// ---------------------------------------------------------------------------

fn grid_init_cells(table_id: &str, rows: usize, cols: usize) {
    if let Some(table) = get_el(table_id) {
        table.set_inner_html("");
        for _r in 0..rows {
            let tr = create_el("tr");
            for _c in 0..cols {
                let td = create_el("td");
                td.set_class_name("rr-grid-cell");
                let _ = tr.append_child(&td);
            }
            let _ = table.append_child(&tr);
        }
    }
}

fn grid_set_cell(id: &str, col: &Value, row: &Value, text: &Value) {
    let table_id = format!("{}-table", id);
    if let Some(table) = get_el(&table_id) {
        if let Ok(table_el) = table.dyn_into::<web_sys::HtmlTableElement>() {
            let r = row.to_i64() as u32;
            let c = col.to_i64() as u32;
            if let Some(rows) = table_el.rows().item(r) {
                if let Ok(row_el) = rows.dyn_into::<web_sys::HtmlTableRowElement>() {
                    if let Some(cell) = row_el.cells().item(c) {
                        if let Ok(cell_el) = cell.dyn_into::<web_sys::HtmlElement>() {
                            cell_el.set_inner_text(&text.to_string_val());
                        }
                    }
                }
            }
        }
    }
}

fn grid_get_cell(id: &str, col: &Value, row: &Value) -> Value {
    let table_id = format!("{}-table", id);
    if let Some(table) = get_el(&table_id) {
        if let Ok(table_el) = table.dyn_into::<web_sys::HtmlTableElement>() {
            let r = row.to_i64() as u32;
            let c = col.to_i64() as u32;
            if let Some(rows) = table_el.rows().item(r) {
                if let Ok(row_el) = rows.dyn_into::<web_sys::HtmlTableRowElement>() {
                    if let Some(cell) = row_el.cells().item(c) {
                        if let Ok(cell_el) = cell.dyn_into::<web_sys::HtmlElement>() {
                            return v_str(&cell_el.inner_text());
                        }
                    }
                }
            }
        }
    }
    v_str("")
}

fn grid_set_row_count(id: &str, count: usize) {
    let table_id = format!("{}-table", id);
    if let Some(table) = get_el(&table_id) {
        if let Ok(table_el) = table.dyn_into::<web_sys::HtmlTableElement>() {
            let current = table_el.rows().length() as usize;
            let cols = if current > 0 {
                if let Some(first_row) = table_el.rows().item(0) {
                    if let Ok(r) = first_row.dyn_into::<web_sys::HtmlTableRowElement>() {
                        r.cells().length() as usize
                    } else { 3 }
                } else { 3 }
            } else { 3 };

            if count > current {
                for _ in current..count {
                    let tr = create_el("tr");
                    for _ in 0..cols {
                        let td = create_el("td");
                        td.set_class_name("rr-grid-cell");
                        let _ = tr.append_child(&td);
                    }
                    let _ = table_el.append_child(&tr);
                }
            } else {
                for _ in count..current {
                    let _ = table_el.delete_row(-1);
                }
            }
        }
    }
}

fn grid_set_col_count(id: &str, _count: usize) {
    // For simplicity, rebuilding columns would require re-creating the entire grid.
    // This is a simplified implementation.
    let _ = id;
}

// ---------------------------------------------------------------------------
// TabControl helpers
// ---------------------------------------------------------------------------

fn tab_add(id: &str, title: &str) {
    let tabs_id = format!("{}-tabs", id);
    if let Some(tabs) = get_el(&tabs_id) {
        let idx = tabs.child_element_count();
        let btn = create_el("button");
        btn.set_inner_text(title);
        btn.set_class_name("rr-tab-btn");
        let _ = btn.set_attribute("data-tab-index", &idx.to_string());

        // Style: active first tab by default
        if idx == 0 {
            let _ = btn.style().set_property("background", "white");
            let _ = btn.style().set_property("border-bottom", "2px solid #4a90d9");
            let _ = btn.style().set_property("font-weight", "bold");
        }

        // Attach click handler: switch tab, update visual, fire onchange
        {
            let tab_ctrl_id = id.to_string();
            let tabs_bar_id = tabs_id.clone();
            let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |_e: web_sys::MouseEvent| {
                tab_switch(&tab_ctrl_id, &tabs_bar_id);
            });
            let _ = btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
            cb.forget();
        }

        let _ = tabs.append_child(&btn);
    }
}

/// Switch to the tab that was clicked — highlight it, update tabindex, fire onchange.
fn tab_switch(tab_ctrl_id: &str, tabs_bar_id: &str) {
    let doc = document();
    // Find which button was clicked by checking the active element
    // Instead, we iterate buttons and check the event target —
    // but since we're in closure context, find the active element.
    // Actually we read the clicked button's data-tab-index from the FocusEvent.
    // Simpler: check document.activeElement
    let active = doc.active_element();
    let clicked_idx: u32 = active
        .as_ref()
        .and_then(|el| el.get_attribute("data-tab-index"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Update visual: deactivate all, activate clicked
    if let Some(tabs_bar) = get_el(tabs_bar_id) {
        let children = tabs_bar.children();
        for i in 0..children.length() {
            if let Some(child) = children.item(i) {
                if let Ok(btn) = child.dyn_into::<web_sys::HtmlElement>() {
                    if i == clicked_idx {
                        let _ = btn.style().set_property("background", "white");
                        let _ = btn.style().set_property("border-bottom", "2px solid #4a90d9");
                        let _ = btn.style().set_property("font-weight", "bold");
                    } else {
                        let _ = btn.style().set_property("background", "#f0f0f0");
                        let _ = btn.style().set_property("border-bottom", "none");
                        let _ = btn.style().set_property("font-weight", "normal");
                    }
                }
            }
        }
    }

    // Update the tabindex property in the component store
    let comp_name = tab_ctrl_id
        .strip_prefix("rr-")
        .unwrap_or(tab_ctrl_id)
        .to_uppercase();
    crate::object_web::rp_comp_set_prop_only(&comp_name, "tabindex", v_int(clicked_idx as i64));

    // Fire the onchange event
    crate::object_web::rp_fire_event(&comp_name, "onchange");
}

fn tab_remove(id: &str, index: usize) {
    let tabs_id = format!("{}-tabs", id);
    if let Some(tabs) = get_el(&tabs_id) {
        if let Some(child) = tabs.child_nodes().item(index as u32) {
            tabs.remove_child(&child).ok();
        }
    }
}

// ---------------------------------------------------------------------------
// TreeView helpers
// ---------------------------------------------------------------------------

fn tree_add_node(id: &str, text: &str, _parent_node: Option<&str>) {
    let tree_id = format!("{}-tree", id);
    if let Some(tree) = get_el(&tree_id) {
        let li = create_el("li");
        li.set_inner_text(text);
        let _ = li.style().set_property("padding", "2px 0");
        let _ = li.style().set_property("cursor", "pointer");
        let _ = tree.append_child(&li);
    }
}

// ---------------------------------------------------------------------------
// Show form helper
// ---------------------------------------------------------------------------

pub fn gui_web_show_form(name: &str) {
    let id = comp_id(name);
    if let Some(el) = get_el(&id) {
        let _ = el.style().set_property("display", "block");
    }
}

/// Re-parent a DOM element to a different parent's client area.
pub fn gui_web_set_parent(name: &str, parent_name: &str) {
    let id = comp_id(name);
    let parent_client_id = format!("{}-client", comp_id(parent_name));
    if let Some(el) = get_el(&id) {
        // Try parent's client area first, then the parent element itself
        let parent_el = get_el(&parent_client_id)
            .or_else(|| get_el(&comp_id(parent_name)));
        if let Some(p) = parent_el {
            let _ = p.append_child(&el);
        }
    }
}

/// Auto-parent orphan widgets (those appended to body) to the first form,
/// then show all forms.
pub fn gui_web_finalize() {
    let doc = document();

    // Find the first form (default parent for orphan widgets)
    let form_el = match doc.query_selector(".rr-form") {
        Ok(Some(el)) => el,
        _ => return, // No form — nothing to do
    };
    let form_id = form_el.get_attribute("id").unwrap_or_default();
    let client_id = format!("{}-client", form_id);

    // Find the form's client area
    let client = match doc.get_element_by_id(&client_id) {
        Some(el) => el,
        None => form_el.clone(),
    };

    // Move orphan widgets from body into the first form's client area
    let selector = "body > [data-rr-name], body > .rr-widget, body > .rr-plot-container";
    if let Ok(orphans) = doc.query_selector_all(selector) {
        let mut elems: Vec<web_sys::Element> = Vec::new();
        for i in 0..orphans.length() {
            if let Some(node) = orphans.item(i) {
                if let Some(el) = node.dyn_ref::<web_sys::Element>() {
                    elems.push(el.clone());
                }
            }
        }
        for el in &elems {
            let _ = client.append_child(el);
        }
    }

    // Show all forms and set initial z-index stacking
    if let Ok(all_forms) = doc.query_selector_all(".rr-form") {
        for i in 0..all_forms.length() {
            if let Some(node) = all_forms.item(i) {
                if let Some(el) = node.dyn_ref::<web_sys::Element>() {
                    if let Ok(html) = el.clone().dyn_into::<web_sys::HtmlElement>() {
                        let _ = html.style().set_property("display", "block");
                        let _ = html.style().set_property("z-index", &format!("{}", 10 + i));
                    }
                }
            }
        }
    }

    // Inject hover styles for titlebar buttons
    inject_form_styles();
}

// ---------------------------------------------------------------------------
// Form window management
// ---------------------------------------------------------------------------

thread_local! {
    static FORM_Z_COUNTER: std::cell::Cell<i32> = std::cell::Cell::new(100);
    /// Stores (left, top, width, height) before maximize for each form id
    static FORM_SAVED_GEOMETRY: std::cell::RefCell<HashMap<String, (i32, i32, i32, i32)>> =
        std::cell::RefCell::new(HashMap::new());
}

fn form_bring_to_front(form_id: &str) {
    let next_z = FORM_Z_COUNTER.with(|c| {
        let z = c.get() + 1;
        c.set(z);
        z
    });
    if let Some(el) = get_el(form_id) {
        let _ = el.style().set_property("z-index", &next_z.to_string());
    }
}

fn form_minimize(form_id: &str) {
    // Minimize: create a small taskbar-like button at the bottom of the viewport
    if let Some(el) = get_el(form_id) {
        let _ = el.style().set_property("display", "none");
    }
    // Create or update a restore button in the taskbar
    let doc = document();
    let taskbar_id = "rr-taskbar";
    let taskbar = match doc.get_element_by_id(taskbar_id) {
        Some(tb) => tb.dyn_into::<web_sys::HtmlElement>().unwrap(),
        None => {
            let tb = create_el("div");
            tb.set_id(taskbar_id);
            let _ = tb.style().set_property("position", "fixed");
            let _ = tb.style().set_property("bottom", "0");
            let _ = tb.style().set_property("left", "0");
            let _ = tb.style().set_property("width", "100%");
            let _ = tb.style().set_property("height", "36px");
            let _ = tb.style().set_property("background", "linear-gradient(to top, #2c3e50, #34495e)");
            let _ = tb.style().set_property("display", "flex");
            let _ = tb.style().set_property("align-items", "center");
            let _ = tb.style().set_property("padding", "0 8px");
            let _ = tb.style().set_property("gap", "4px");
            let _ = tb.style().set_property("z-index", "99999");
            let _ = tb.style().set_property("box-shadow", "0 -2px 6px rgba(0,0,0,0.3)");
            let _ = doc.body().unwrap().append_child(&tb);
            tb
        }
    };

    let restore_id = format!("{}-restore", form_id);
    if doc.get_element_by_id(&restore_id).is_some() {
        return; // Already has a restore button
    }

    // Get the form caption for the button label
    let label = get_el(form_id)
        .and_then(|el| el.query_selector(".rr-form-title-text").ok().flatten())
        .map(|t| t.text_content().unwrap_or_default())
        .unwrap_or_else(|| form_id.to_string());

    let btn = create_el("button");
    btn.set_id(&restore_id);
    btn.set_inner_text(&label);
    let _ = btn.style().set_property("background", "#4a90d9");
    let _ = btn.style().set_property("color", "white");
    let _ = btn.style().set_property("border", "none");
    let _ = btn.style().set_property("border-radius", "3px");
    let _ = btn.style().set_property("padding", "4px 12px");
    let _ = btn.style().set_property("cursor", "pointer");
    let _ = btn.style().set_property("font-size", "12px");
    let _ = btn.style().set_property("max-width", "180px");
    let _ = btn.style().set_property("overflow", "hidden");
    let _ = btn.style().set_property("text-overflow", "ellipsis");
    let _ = btn.style().set_property("white-space", "nowrap");
    {
        let fid = form_id.to_string();
        let rid = restore_id.clone();
        let cb = Closure::<dyn FnMut()>::new(move || {
            // Restore the form
            if let Some(el) = get_el(&fid) {
                let _ = el.style().set_property("display", "block");
            }
            form_bring_to_front(&fid);
            // Remove the restore button
            if let Some(rb) = document().get_element_by_id(&rid) {
                rb.remove();
            }
            // If taskbar empty, hide it
            if let Some(tb) = document().get_element_by_id("rr-taskbar") {
                if tb.child_element_count() == 0 {
                    tb.remove();
                }
            }
        });
        let _ = btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
        cb.forget();
    }
    let _ = taskbar.append_child(&btn);
}

fn form_maximize(form_id: &str) {
    let el = match get_el(form_id) { Some(e) => e, None => return };
    let style = el.style();

    let is_maximized = FORM_SAVED_GEOMETRY.with(|sg| sg.borrow().contains_key(form_id));

    if is_maximized {
        // Restore from maximized state
        let (l, t, w, h) = FORM_SAVED_GEOMETRY.with(|sg| sg.borrow_mut().remove(form_id).unwrap());
        let _ = style.set_property("left", &format!("{}px", l));
        let _ = style.set_property("top", &format!("{}px", t));
        let _ = style.set_property("width", &format!("{}px", w));
        let _ = style.set_property("height", &format!("{}px", h));
        let _ = style.set_property("border-radius", "6px");
    } else {
        // Save current geometry and maximize
        let l = el.offset_left();
        let t = el.offset_top();
        let w = el.offset_width();
        let h = el.offset_height();
        FORM_SAVED_GEOMETRY.with(|sg| sg.borrow_mut().insert(form_id.to_string(), (l, t, w, h)));
        let _ = style.set_property("left", "0");
        let _ = style.set_property("top", "0");
        let _ = style.set_property("width", "100vw");
        let _ = style.set_property("height", "100vh");
        let _ = style.set_property("border-radius", "0");
    }
}

fn form_close(form_id: &str) {
    if let Some(el) = get_el(form_id) {
        let _ = el.style().set_property("display", "none");
    }
    // Fire onclose event
    let comp_name = form_id
        .strip_prefix("rr-")
        .unwrap_or(form_id)
        .to_uppercase();
    crate::object_web::rp_fire_event(&comp_name, "onclose");
}

/// Setup drag-to-move on a form's titlebar.
fn setup_form_drag(titlebar: &web_sys::HtmlElement, form_id: &str) {
    let form_id_owned = form_id.to_string();

    let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        // Don't drag if clicking a button in the titlebar
        if let Some(target) = e.target() {
            if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                if el.tag_name() == "BUTTON" {
                    return;
                }
            }
        }

        let form_el = match get_el(&form_id_owned) { Some(e) => e, None => return };
        let start_x = e.client_x();
        let start_y = e.client_y();
        let orig_left = form_el.offset_left();
        let orig_top = form_el.offset_top();

        let form_id_move = form_id_owned.clone();
        let move_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let dx = e.client_x() - start_x;
            let dy = e.client_y() - start_y;
            if let Some(f) = get_el(&form_id_move) {
                let _ = f.style().set_property("left", &format!("{}px", orig_left + dx));
                let _ = f.style().set_property("top", &format!("{}px", orig_top + dy));
            }
        });

        let doc = document();
        let _ = doc.add_event_listener_with_callback("mousemove", move_cb.as_ref().unchecked_ref());

        let move_ref: JsValue = move_cb.as_ref().into();
        let up_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::once(move |_e: web_sys::MouseEvent| {
            let doc = document();
            let _ = doc.remove_event_listener_with_callback("mousemove", move_ref.unchecked_ref());
            drop(move_cb); // prevent leak
        });
        let _ = doc.add_event_listener_with_callback("mouseup", up_cb.as_ref().unchecked_ref());
        up_cb.forget();
    });
    let _ = titlebar.add_event_listener_with_callback("mousedown", cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Inject CSS for form button hover effects.
fn inject_form_styles() {
    let doc = document();
    if doc.get_element_by_id("rr-form-styles").is_some() {
        return; // Already injected
    }
    let style = doc.create_element("style").unwrap();
    style.set_id("rr-form-styles");
    style.set_text_content(Some(
        ".rr-form-btn-min:hover,.rr-form-btn-max:hover{background:rgba(255,255,255,0.2)!important}\
         .rr-form-btn-close:hover{background:#e74c3c!important}\
         .rr-tab-btn{padding:6px 16px;border:none;background:#f0f0f0;cursor:pointer;font-size:13px;\
         border-bottom:2px solid transparent;transition:background 0.15s}\
         .rr-tab-btn:hover{background:#e0e0e0}"
    ));
    if let Ok(Some(head)) = doc.query_selector("head") {
        let _ = head.append_child(&style);
    }
}
