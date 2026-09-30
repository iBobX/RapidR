//! GUI web implementation — HTML5 DOM widget creation for all RapidR components.
//!
//! Each component type (RFORM, RBUTTON, RLABEL, etc.) maps to one or more
//! HTML elements styled via the shared rapidr-rrcss base stylesheet and absolute positioning.

use crate::value::{v_int, v_null, v_str, Value};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn document() -> web_sys::Document {
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

fn strip_ampersands(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                result.push('&');
                chars.next();
            }
        } else {
            result.push(c);
        }
    }
    result
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

/// Whether any of the program's forms is showing.
pub fn any_form_shown() -> bool {
    let Ok(forms) = document().query_selector_all(".rr-form") else { return false };
    (0..forms.length()).filter_map(|i| forms.item(i)?.dyn_into::<web_sys::HtmlElement>().ok()).any(|f| f.is_connected() && (f.offset_width() > 0 || f.offset_height() > 0))
}

/// Whether `name` has an element the user can see (rendered, not hidden).
pub fn element_shown(name: &str) -> bool {
    document()
        .get_element_by_id(&comp_id(name))
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
        .is_some_and(|el| el.is_connected() && (el.offset_width() > 0 || el.offset_height() > 0))
}

/// Compute an element ID from the RapidR component name.
fn comp_id(name: &str) -> String {
    format!("rr-{}", name.to_lowercase())
}

fn value_to_jsvalue(v: &Value) -> JsValue {
    match v {
        Value::Null => JsValue::NULL,
        Value::Boolean(b) => JsValue::from_bool(*b),
        Value::Integer(n) => JsValue::from_f64(*n as f64),
        Value::Double(d) => JsValue::from_f64(*d),
        Value::String(s) => JsValue::from_str(s),
        Value::Array(a) => a.borrow().data.iter().map(value_to_jsvalue).collect::<js_sys::Array>().into(),
        Value::Object(o) => JsValue::from_str(&o.id),
    }
}

fn jsvalue_to_value(v: &JsValue) -> Value {
    if v.is_null() || v.is_undefined() {
        Value::Null
    } else if let Some(b) = v.as_bool() {
        Value::Boolean(b)
    } else if let Some(n) = v.as_f64() {
        if n.fract() == 0.0 && n >= (i64::MIN as f64) && n <= (i64::MAX as f64) {
            Value::Integer(n as i64)
        } else {
            Value::Double(n)
        }
    } else if let Some(s) = v.as_string() {
        v_str(&s)
    } else {
        let s = js_sys::JSON::stringify(v)
            .ok()
            .and_then(|js_str| js_str.as_string())
            .unwrap_or_else(|| "null".to_string());
        if s == "null" {
            if let Some(to_str_func) = js_sys::Reflect::get(v, &JsValue::from_str("toString")).ok() {
                if let Ok(to_str) = to_str_func.dyn_into::<js_sys::Function>() {
                    if let Ok(res) = to_str.call0(v) {
                        if let Some(res_s) = res.as_string() {
                            return v_str(&res_s);
                        }
                    }
                }
            }
            Value::Null
        } else {
            v_str(&s)
        }
    }
}

// ---------------------------------------------------------------------------
// Widget creation — one function per component type
// ---------------------------------------------------------------------------

pub fn gui_web_create_widget(name: &str, comp_type: &str, props: &HashMap<String, Value>) {
    note_display_scale();
    inject_form_styles();
    let id = comp_id(name);
    match comp_type {
        "RFORM" => create_form(&id, name, props),
        "RBUTTON" => create_button(&id, name, props),
        "RCOOLBTN" => create_coolbtn(&id, name, props),
        "ROVALBTN" => create_ovalbtn(&id, name, props),
        "RLABEL" => create_label(&id, name, props),
        "REDIT" => create_edit(&id, name, props),
        "RMEMO" | "RRICHEDIT" => create_textarea(&id, name, props),
        "RPANEL" => create_panel(&id, name, props),
        "RCHECKBOX" => create_checkbox(&id, name, props),
        "RRADIOBUTTON" => create_radio(&id, name, props),
        // Style (RAPIDQ.INC): csDropDown = 0 (the default) and csSimple = 1
        // have an edit box; csDropDownList = 2 only picks from the list.
        // csOwnerDrawFixed / csOwnerDrawVariable: drawn by OnDrawItem.
        "RCOMBOBOX" if matches!(props.get("style").map(Value::to_i64), Some(3 | 4)) => create_owner_combo(&id, name, props),
        "RCOMBOBOX" if props.get("style").map_or(0, Value::to_i64) < 2 => create_edit_combo(&id, name, props),
        "RCOMBOBOX" => create_select(&id, name, false, props),
        // Style = lbOwnerDrawFixed / lbOwnerDrawVariable: drawn by OnDrawItem.
        "RLISTBOX" | "RFILELISTBOX" if matches!(props.get("style").map(|v| v.to_i64()), Some(1 | 2)) => create_owner_list(&id, name, props),
        "RLISTBOX" | "RFILELISTBOX" => create_select(&id, name, true, props),
        "RDIRTREE" => create_dirtree(&id, name, props),
        "RTIMER" => { /* Timers are virtual — no DOM element, handled in object_web */ }
        "RIMAGE" => create_image(&id, name, props),
        "RCANVAS" => create_canvas(&id, name, props),
        "RHEADER" => {
            create_canvas(&id, name, props);
            header_mouse_events(&id, name);
        }
        "RSTRINGGRID" => create_grid(&id, name, props),
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
        "RMDICHILD" => create_mdi_frame(&id, name, props),
        "RSPLITTER" => create_splitter(&id, name, props),
        "RLISTVIEW" => create_listview(&id, name, props),
        "RDATETIMEPICKER" => create_datetimepicker(&id, name, props),
        "RCODEEDITOR" => create_codeeditor(&id, name, props),
        "RDESIGNSURFACE" => create_panel(&id, name, props), // design surface is just a panel on web
        // Data-science widgets — visual placeholders updated by their own methods
        "RPLOT" => crate::datascience_web::create_plot_widget(&id, name, props),
        "RDATAFRAME" => crate::datascience_web::create_dataframe_widget(&id, name, props),
        "RNUM" => crate::datascience_web::create_num_widget(&id, name, props),
        // Dialogs — these are virtual and use browser native dialogs
        "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG" => { /* virtual */ }
        // Non-GUI components (SQLite, HTTP, etc.) — no DOM element
        "RSQLITE" | "RMYSQL" | "RSOCKET" | "RSERVERSOCKET" | "RHTTP"
        | "RFILESTREAM" | "RJSON" | "RSTRINGLIST" | "RPRINTER" | "RUDT" | "RRECT" | "RIMAGELIST" => { /* no DOM element */ }
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
    if let Some(el) = get_el(&id) {
        let _ = el.set_attribute("data-rr-type", comp_type);
    }
}

// ---------------------------------------------------------------------------
// Property get/set — universal DOM property access
// ---------------------------------------------------------------------------

/// RGBA pixels as a PNG data URL (an off-screen canvas).
fn rgba_data_url(w: usize, h: usize, rgba: &[u8]) -> Option<String> {
    let data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(rgba), w as u32, h as u32).ok()?;
    let off = document().create_element("canvas").ok()?.dyn_into::<web_sys::HtmlCanvasElement>().ok()?;
    off.set_width(w as u32);
    off.set_height(h as u32);
    let ctx = off.get_context("2d").ok().flatten()?.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()?;
    ctx.put_image_data(&data, 0.0, 0.0).ok()?;
    off.to_data_url().ok()
}

/// A form's title bar icon, as on the desktop: its `IcoHandle` / `Icon`
/// (an ICO, BMP, PNG or SVG), else the application's; none: hidden.
pub fn apply_form_icon(name: &str) {
    let Some(img) = get_el(&comp_id(name)).and_then(|e| e.query_selector(":scope > .rr-form-titlebar > .rr-form-icon").ok().flatten()) else { return };
    let Ok(img) = img.dyn_into::<web_sys::HtmlImageElement>() else { return };
    note_display_scale();
    let own = ["icohandle", "icon"].into_iter().map(|p| crate::object_web::rp_comp_get_stored(name, p)).find(rapidr_value::objects::has_icon);
    let icon = own.or_else(rapidr_value::globals::application_icon).and_then(|v| rapidr_value::objects::icon_pixels(&v));
    match icon.and_then(|(w, h, rgba, _)| rgba_data_url(w, h, &rgba)) {
        Some(url) => {
            img.set_src(&url);
            let _ = img.style().set_property("display", "");
        }
        None => {
            let _ = img.style().set_property("display", "none");
        }
    }
}

/// `Application.Icon` changed: the page's icon, and every form without its
/// own.
pub fn apply_application_icon() {
    let doc = document();
    let url = rapidr_value::globals::application_icon().and_then(|v| rapidr_value::objects::icon_pixels(&v)).and_then(|(w, h, rgba, _)| rgba_data_url(w, h, &rgba));
    if let Some(url) = url {
        let link = doc.query_selector("link[rel~='icon']").ok().flatten().or_else(|| {
            let l = doc.create_element("link").ok()?;
            let _ = l.set_attribute("rel", "icon");
            doc.query_selector("head").ok().flatten()?.append_child(&l).ok()?;
            Some(l)
        });
        if let Some(link) = link {
            let _ = link.set_attribute("href", &url);
        }
    }
    if let Ok(forms) = doc.query_selector_all(".rr-form[data-rr-name]") {
        for i in 0..forms.length() {
            if let Some(name) = forms.item(i).and_then(|f| f.dyn_into::<web_sys::Element>().ok()).and_then(|f| f.get_attribute("data-rr-name")) {
                apply_form_icon(&name);
            }
        }
    }
}

pub fn gui_web_set_prop(name: &str, prop: &str, val: &Value) {
    let id = comp_id(name);
    let el = match get_el(&id) {
        Some(e) => e,
        None => return,
    };
    let s = val.to_string_val();
    let style = el.style();

    if matches!(prop, "icon" | "icohandle") && el.class_list().contains("rr-form") {
        apply_form_icon(name);
        return;
    }
    // A QFORMMDI child's frame draws its own caption (mdi_frame_update).
    if el.class_list().contains("rr-mdichild") && matches!(prop, "caption" | "text") {
        return;
    }
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
                    title_text.set_text_content(Some(&strip_ampersands(&s)));
                }
            } else if let Ok(Some(caption)) = el.query_selector(":scope > .rr-panel-caption") {
                caption.set_text_content(Some(&strip_ampersands(&s)));
            } else if el.tag_name().to_uppercase() == "LABEL" {
                if let Ok(Some(span)) = el.query_selector("span") {
                    span.set_text_content(Some(&strip_ampersands(&s)));
                } else {
                    el.set_text_content(Some(&strip_ampersands(&s)));
                }
            } else {
                // Plain text only: captions often show DB/HTTP/AI data, so
                // markup must never be interpreted here. Use RDOM.InnerHTML
                // or RWebView.HTML when markup is intended.
                el.set_text_content(Some(&strip_ampersands(&s)));
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
        "disabled" => {
            let is_disabled = val.to_bool();
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                input.set_disabled(is_disabled);
            } else if let Ok(btn) = el.clone().dyn_into::<web_sys::HtmlButtonElement>() {
                btn.set_disabled(is_disabled);
            } else if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                sel.set_disabled(is_disabled);
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                ta.set_disabled(is_disabled);
            } else {
                if is_disabled {
                    let _ = el.set_attribute("disabled", "");
                } else {
                    let _ = el.remove_attribute("disabled");
                }
            }
        }
        "color" | "backcolor" => {
            let _ = style.set_property("background-color", &value_to_css_color(val));
        }
        "fontcolor" | "forecolor" => {
            let _ = style.set_property("color", &value_to_css_color(val));
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
        "fontunderline" | "fontstrikeout" => {
            // Both can be on: build the decoration from the two settings.
            let on = |p: &str| if p == prop { val.to_bool() } else { crate::object_web::rp_comp_get(name, p).to_bool() };
            let decoration = match (on("fontunderline"), on("fontstrikeout")) {
                (true, true) => "underline line-through",
                (true, false) => "underline",
                (false, true) => "line-through",
                (false, false) => "none",
            };
            let _ = style.set_property("text-decoration", decoration);
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
                if prop == "checked" {
                    input.set_checked(val.to_bool());
                } else {
                    input.set_value(&s);
                }
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                ta.set_value(&s);
            } else if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                sel.set_value(&s);
            } else {
                let _ = el.set_attribute("value", &s);
            }
        }
        "readonly" | "read_only" => {
            let is_readonly = val.to_bool();
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                input.set_read_only(is_readonly);
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                ta.set_read_only(is_readonly);
            } else {
                if is_readonly {
                    let _ = el.set_attribute("readonly", "");
                } else {
                    let _ = el.remove_attribute("readonly");
                }
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
            let resolved = if s.starts_with("assets/") || (!s.contains("://") && !s.starts_with("data:")) {
                crate::database_web::get_rapidr_asset(&s).unwrap_or(s.clone())
            } else {
                s.clone()
            };
            if let Ok(img) = el.clone().dyn_into::<web_sys::HtmlImageElement>() {
                img.set_src(&resolved);
            } else if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                audio.set_src(&resolved);
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                video.set_src(&resolved);
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
        // RAPIDQ.INC's crDefault 0, crHandPoint -21, … (rapidr_value::input)
        "cursor" => {
            let _ = style.set_property("cursor", rapidr_value::input::Cursor::of(val.to_i64()).css());
        }
        "bordercolor" => {
            let _ = style.set_property("border-color", &value_to_css_color(val));
        }
        "borderstyle" => {
            // A form's frame (bsNone: none); another control's border line.
            if el.class_list().contains("rr-form") {
                let has_menu = el.query_selector(":scope > nav[data-rr-type=\"RMAINMENU\"]").ok().flatten().is_some();
                place_form_chrome(&el, val.to_i64(), has_menu);
            } else {
                let _ = style.set_property("border-style", if val.to_i64() == 0 { "none" } else { "solid" });
            }
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
            if let Ok(iframe) = el.clone().dyn_into::<web_sys::HtmlIFrameElement>() {
                iframe.set_srcdoc(&s);
            } else {
                el.set_inner_html(&s);
            }
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
        "tagname" => {
            let target_tag = s.to_lowercase();
            let current_tag = el.tag_name().to_lowercase();
            if current_tag != target_tag {
                let new_el = create_el(&target_tag);
                new_el.set_id(&id);

                let attrs = el.attributes();
                for i in 0..attrs.length() {
                    if let Some(attr) = attrs.item(i) {
                        let attr_name = attr.name();
                        let attr_val = attr.value();
                        let _ = new_el.set_attribute(&attr_name, &attr_val);
                    }
                }

                while let Some(child) = el.first_child() {
                    let _ = new_el.append_child(&child);
                }

                let is_head_tag = matches!(
                    target_tag.as_str(),
                    "style" | "script" | "link" | "meta" | "title"
                );

                if is_head_tag {
                    let new_style = new_el.style();
                    let _ = new_style.remove_property("position");
                    let _ = new_style.remove_property("box-sizing");
                    let _ = new_style.remove_property("left");
                    let _ = new_style.remove_property("top");
                    let _ = new_style.remove_property("width");
                    let _ = new_style.remove_property("height");
                    let _ = new_style.remove_property("border");
                    let _ = new_style.remove_property("background");

                    if let Some(parent) = el.parent_node() {
                        let _ = parent.remove_child(&el);
                    }

                    if let Ok(Some(head)) = document().query_selector("head") {
                        let _ = head.append_child(&new_el);
                    } else {
                        let _ = document().body().unwrap().append_child(&new_el);
                    }
                } else {
                    let was_head_tag = matches!(
                        current_tag.as_str(),
                        "style" | "script" | "link" | "meta" | "title"
                    );
                    if was_head_tag {
                        let new_style = new_el.style();
                        let _ = new_style.set_property("position", "absolute");
                        let _ = new_style.set_property("box-sizing", "border-box");
                        let _ = new_style.set_property("border", "1px solid #aaa");
                        let _ = new_style.set_property("background", "white");

                        let props = crate::object_web::rp_comp_get_all_properties(name)
                            .map(|(_, p)| p)
                            .unwrap_or_default();
                        apply_geometry(&new_el, &props, 0, 0, 100, 25);

                        let parent_val = crate::object_web::rp_comp_get_stored(name, "parentid");
                        let parent = if matches!(parent_val, Value::Null) {
                            crate::object_web::rp_comp_get_stored(name, "parent")
                        } else {
                            parent_val
                        };
                        let parent_str = if matches!(parent, Value::Null) { None } else { Some(parent.to_string_val()) };
                        let parent_el = get_parent_client(&parent_str);
                        let _ = parent_el.append_child(&new_el);

                        if let Some(parent) = el.parent_node() {
                            let _ = parent.remove_child(&el);
                        }
                    } else {
                        if let Some(parent) = el.parent_node() {
                            let _ = parent.replace_child(&new_el, &el);
                        } else {
                            let parent_el = document().body().unwrap();
                            let _ = parent_el.append_child(&new_el);
                        }
                    }
                }

                // Rebind event handlers to the new DOM node
                crate::object_web::rp_rebind_component_events(name);
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
            let left = style.get_property_value("left").unwrap_or_default();
            let top = style.get_property_value("top").unwrap_or_default();
            let width = style.get_property_value("width").unwrap_or_default();
            let height = style.get_property_value("height").unwrap_or_default();
            let position = style.get_property_value("position").unwrap_or_default();
            let display = style.get_property_value("display").unwrap_or_default();
            let box_sizing = style.get_property_value("box-sizing").unwrap_or_default();
            let z_index = style.get_property_value("z-index").unwrap_or_default();

            let _ = el.set_attribute("style", &s);

            if !left.is_empty() { let _ = style.set_property("left", &left); }
            if !top.is_empty() { let _ = style.set_property("top", &top); }
            if !width.is_empty() { let _ = style.set_property("width", &width); }
            if !height.is_empty() { let _ = style.set_property("height", &height); }
            if !position.is_empty() { let _ = style.set_property("position", &position); }
            if !display.is_empty() { let _ = style.set_property("display", &display); }
            if !box_sizing.is_empty() { let _ = style.set_property("box-sizing", &box_sizing); }
            if !z_index.is_empty() { let _ = style.set_property("z-index", &z_index); }
        }
        // Web-exclusive: RWebAudio/RWebVideo
        "volume" => {
            if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                audio.set_volume(val.to_f64());
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                video.set_volume(val.to_f64());
            }
        }
        "currenttime" => {
            let t = val.to_f64();
            if let Ok(audio) = el.clone().dyn_into::<web_sys::HtmlAudioElement>() {
                audio.set_current_time(t);
            } else if let Ok(video) = el.clone().dyn_into::<web_sys::HtmlVideoElement>() {
                video.set_current_time(t);
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
            let comp_type = el.get_attribute("data-rr-type").unwrap_or_default();
            if comp_type == "RDOM" {
                let _ = el.set_attribute(prop, &s);
            } else {
                // Store unrecognized props as data attributes
                let _ = el.set_attribute(&format!("data-rr-{}", prop), &s);
            }
        }
    }
}

pub fn gui_web_get_prop(name: &str, prop: &str) -> Value {
    let comp_type = crate::object_web::rp_comp_type(name);
    if comp_type == "RROUTER" {
        if prop == "route" || prop == "hash" {
            if let Some(window) = web_sys::window() {
                if let Ok(hash) = window.location().hash() {
                    let clean = if hash.starts_with('#') {
                        hash.trim_start_matches('#').to_string()
                    } else {
                        hash
                    };
                    return v_str(&clean);
                }
            }
            return v_str("");
        }
    }

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
                // What's shown has its `&` accelerator marks taken out; the
                // program reads back what it set while that's still shown.
                // A form's is its title bar's text (not its buttons' and
                // controls').
                let title = el.query_selector(":scope > .rr-form-titlebar > .rr-form-title-text").ok().flatten();
                let stored = crate::object_web::rp_comp_get_stored(name, prop).to_string_val();
                // A container's (a panel holding controls): its own, not
                // theirs too.
                if title.is_none() && el.query_selector("[data-rr-name]").ok().flatten().is_some() {
                    return v_str(&stored);
                }
                let shown = title.and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()).unwrap_or_else(|| el.clone()).inner_text();
                let squash = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
                if squash(&strip_ampersands(&stored)) == squash(&shown) { v_str(&stored) } else { v_str(&shown) }
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
            } else if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                Value::Boolean(!sel.disabled())
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                Value::Boolean(!ta.disabled())
            } else {
                Value::Boolean(true)
            }
        }
        "disabled" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                Value::Boolean(input.disabled())
            } else if let Ok(btn) = el.clone().dyn_into::<web_sys::HtmlButtonElement>() {
                Value::Boolean(btn.disabled())
            } else if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                Value::Boolean(sel.disabled())
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                Value::Boolean(ta.disabled())
            } else {
                Value::Boolean(el.has_attribute("disabled"))
            }
        }
        "readonly" | "read_only" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                Value::Boolean(input.read_only())
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                Value::Boolean(ta.read_only())
            } else {
                Value::Boolean(el.has_attribute("readonly"))
            }
        }
        "checked" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                Value::Boolean(input.checked())
            } else {
                Value::Boolean(false)
            }
        }
        "value" => {
            if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
                v_str(&input.value())
            } else if let Ok(ta) = el.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
                v_str(&ta.value())
            } else if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                v_str(&sel.value())
            } else {
                match el.get_attribute("value") {
                    Some(v) => v_str(&v),
                    None => v_str(""),
                }
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
        "cssstyle" => match el.get_attribute("style") {
            Some(v) => v_str(&v),
            None => v_str(""),
        },
        "tagname" => {
            v_str(&el.tag_name().to_lowercase())
        }
        "url" => {
            if let Ok(iframe) = el.clone().dyn_into::<web_sys::HtmlIFrameElement>() {
                v_str(&iframe.src())
            } else {
                match el.get_attribute("src") {
                    Some(v) => v_str(&v),
                    None => v_str(""),
                }
            }
        }
        "html" => {
            if let Ok(iframe) = el.clone().dyn_into::<web_sys::HtmlIFrameElement>() {
                v_str(&iframe.srcdoc())
            } else {
                match el.get_attribute("srcdoc") {
                    Some(v) => v_str(&v),
                    None => v_str(""),
                }
            }
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
            // Check live attribute first (useful for RDOM / standard attributes)
            match el.get_attribute(prop) {
                Some(v) => v_str(&v),
                None => {
                    // Check data attributes
                    match el.get_attribute(&format!("data-rr-{}", prop)) {
                        Some(v) => v_str(&v),
                        None => v_null(),
                    }
                }
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
        // A tree's GetItemAt(X, Y): the node shown there (-1: none).
        ("RTREEVIEW", "getitemat") => {
            let y = args.get(1).map_or(0, Value::to_i64) as f64;
            let Some(el) = get_el(&id) else { return v_int(-1) };
            let top = el.get_bounding_client_rect().top() + el.client_top() as f64;
            let rows = el.query_selector_all("[data-node]").ok();
            let hit = rows.and_then(|rows| {
                (0..rows.length()).filter_map(|i| rows.item(i)?.dyn_into::<web_sys::Element>().ok()).find_map(|r| {
                    let b = r.get_bounding_client_rect();
                    (b.top() - top <= y && y < b.bottom() - top).then(|| r.get_attribute("data-node")?.parse::<i64>().ok()).flatten()
                })
            });
            v_int(hit.unwrap_or(-1))
        }
        // LIST methods (RCOMBOBOX, RLISTBOX)
        (_, "additem") | (_, "additems") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                if let Ok(sel) = el.dyn_into::<web_sys::HtmlSelectElement>() {
                    for arg in args {
                        let opt = document()
                            .create_element("option")
                            .unwrap()
                            .dyn_into::<web_sys::HtmlOptionElement>()
                            .unwrap();
                        let text = arg.to_string_val();
                        opt.set_text(&text);
                        opt.set_value(&text);
                        let _ = sel.add_with_html_option_element(&opt);
                    }
                }
            }
            v_null()
        }
        // RIMAGE load methods
        ("RIMAGE", "loadfromfile" | "load") if args.len() >= 1 => {
            let path = args[0].to_string_val();
            if let Some(el) = get_el(&id) {
                if let Ok(img) = el.dyn_into::<web_sys::HtmlImageElement>() {
                    img.set_src(&path);
                }
            }
            v_null()
        }
        ("RIMAGE", "loadfromplot") if args.len() >= 1 => {
            let plot_name = args[0].to_string_val();
            crate::datascience_web::render_plot(&plot_name.to_uppercase());
            let canvas_id = format!("rr-{}-canvas", plot_name.to_lowercase());
            if let Some(canvas_el) = document().get_element_by_id(&canvas_id) {
                if let Ok(canvas) = canvas_el.dyn_into::<web_sys::HtmlCanvasElement>() {
                    if let Ok(data_url) = canvas.to_data_url() {
                        if let Some(el) = get_el(&id) {
                            if let Ok(img) = el.dyn_into::<web_sys::HtmlImageElement>() {
                                img.set_src(&data_url);
                            }
                        }
                    }
                }
            }
            v_null()
        }
        // RWEBSTORAGE clear — must come before generic (_, "clear")
        ("RWEBSTORAGE", "clear") => {
            let st = crate::object_web::rp_comp_get_stored(name, "storagetype").to_string_val();
            crate::storage_web::storage_clear(&st);
            v_null()
        }
        (_, "clear") => {
            if let Some(el) = get_el(&id) {
                if let Ok(sel) = el.clone().dyn_into::<web_sys::HtmlSelectElement>() {
                    while sel.length() > 0 {
                        sel.remove_with_index(0);
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
        ("RHEADER", "refresh" | "repaint" | "update" | "paint") => {
            refresh_header(name);
            v_null()
        }
        ("RCANVAS" | "RFORM", "refresh" | "repaint" | "update" | "paint") => {
            render_canvas(name);
            crate::object_web::rp_fire_event(name, "onpaint");
            v_null()
        }
        (_, "refresh") | (_, "repaint") | (_, "invalidate") => {
            // no-op on web — browser handles repainting
            v_null()
        }
        // RWEBNOTIFICATION show — must come before generic (_, "show")
        ("RWEBNOTIFICATION", "show") => {
            let title = crate::object_web::rp_comp_get_stored(name, "title").to_string_val();
            let body = crate::object_web::rp_comp_get_stored(name, "body").to_string_val();
            let options = web_sys::NotificationOptions::new();
            options.set_body(&body);
            let _ = web_sys::Notification::new_with_options(&title, &options);
            v_null()
        }
        (_, "show") => {
            crate::object_web::rp_comp_set(name, "visible", crate::value::v_bool(true));
            if let Some(el) = get_el(&id) {
                let _ = el.style().set_property("display", "");
                // If it's a form, bring to front
                if el.class_list().contains("rr-form") {
                    form_bring_to_front(&id);
                }
            }
            v_null()
        }
        // Pseudo-modal show on web. True blocking is impossible in single-thread
        // wasm; we instead dim the page with a backdrop overlay and bring the
        // form to the front. Backdrop is removed when the form is closed/hidden.
        (_, "showmodal") if comp_type == "RFORM" => {
            crate::object_web::rp_comp_set(name, "visible", crate::value::v_bool(true));
            if let Some(el) = get_el(&id) {
                let _ = el.style().set_property("display", "");
                show_modal_backdrop(&id);
                form_bring_to_front(&id);
                // Center on screen
                let w = el.offset_width();
                let h = el.offset_height();
                let vw = document().document_element().map(|d| d.client_width()).unwrap_or(800);
                let vh = document().document_element().map(|d| d.client_height()).unwrap_or(600);
                let _ = el.style().set_property("left", &format!("{}px", ((vw - w) / 2).max(0)));
                let _ = el.style().set_property("top", &format!("{}px", ((vh - h) / 2).max(0)));
            }
            // As on the desktop, ShowModal waits until the form closes (the
            // VM suspends the code that called it); where nothing can wait it
            // returns at once.
            crate::dialog_web::begin_modal(&id);
            v_null()
        }
        (_, "center") if comp_type == "RFORM" => {
            if let Some(el) = get_el(&id) {
                let w = el.offset_width();
                let h = el.offset_height();
                let vw = document().document_element().map(|d| d.client_width()).unwrap_or(800);
                let vh = document().document_element().map(|d| d.client_height()).unwrap_or(600);
                let _ = el.style().set_property("left", &format!("{}px", ((vw - w) / 2).max(0)));
                let _ = el.style().set_property("top", &format!("{}px", ((vh - h) / 2).max(0)));
            }
            v_null()
        }
        (_, "hide") => {
            crate::object_web::rp_comp_set(name, "visible", crate::value::v_bool(false));
            if let Some(el) = get_el(&id) {
                let _ = el.style().set_property("display", "none");
                if el.class_list().contains("rr-form") {
                    hide_modal_backdrop(&id);
                }
            }
            v_null()
        }
        (_, "close") if comp_type == "RFORM" => {
            close_form(name);
            v_null()
        }
        // Re-parent a widget (or form) into another container.
        (_, "setparent") if args.len() >= 1 => {
            let parent_name = args[0].to_string_val();
            gui_web_set_parent(name, &parent_name);
            crate::object_web::rp_comp_set(name, "parent", v_str(&parent_name));
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
        // Web-exclusive: RWebView
        ("RWEBVIEW", "sethtml") if args.len() >= 1 => {
            if let Some(el) = get_el(&id) {
                if let Ok(iframe) = el.dyn_into::<web_sys::HtmlIFrameElement>() {
                    iframe.set_srcdoc(&args[0].to_string_val());
                }
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
            // Running developer-supplied JS is this component's purpose.
            #[allow(clippy::disallowed_methods)]
            let evaluated = js_sys::eval(&args[0].to_string_val());
            match evaluated {
                Ok(result) => jsvalue_to_value(&result),
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
                js_args.push(&value_to_jsvalue(arg));
            }
            if let Some(window) = web_sys::window() {
                match js_sys::Reflect::get(&window, &JsValue::from_str(&func_name)) {
                    Ok(func) => {
                        if let Ok(func) = func.dyn_into::<js_sys::Function>() {
                            match func.apply(&JsValue::NULL, &js_args) {
                                Ok(result) => jsvalue_to_value(&result),
                                Err(e) => {
                                    web_sys::console::error_1(&e);
                                    v_null()
                                }
                            }
                        } else {
                            v_null()
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&e);
                        v_null()
                    }
                }
            } else {
                v_null()
            }
        }
        // Web-exclusive: RWebStorage
        ("RWEBSTORAGE", "set") if args.len() >= 2 => {
            let st = crate::object_web::rp_comp_get_stored(name, "storagetype").to_string_val();
            crate::storage_web::storage_set(&st, &args[0].to_string_val(), &args[1].to_string_val());
            v_null()
        }
        ("RWEBSTORAGE", "get") if args.len() >= 1 => {
            let st = crate::object_web::rp_comp_get_stored(name, "storagetype").to_string_val();
            crate::storage_web::storage_get(&st, &args[0].to_string_val())
        }
        ("RWEBSTORAGE", "remove") if args.len() >= 1 => {
            let st = crate::object_web::rp_comp_get_stored(name, "storagetype").to_string_val();
            crate::storage_web::storage_remove(&st, &args[0].to_string_val());
            v_null()
        }
        // RWEBSTORAGE clear is handled above (before generic "clear")
        ("RWEBSTORAGE", "keys") => {
            let st = crate::object_web::rp_comp_get_stored(name, "storagetype").to_string_val();
            crate::storage_web::storage_keys(&st)
        }
        ("RWEBSTORAGE", "haskey") if args.len() >= 1 => {
            let st = crate::object_web::rp_comp_get_stored(name, "storagetype").to_string_val();
            crate::storage_web::storage_has_key(&st, &args[0].to_string_val())
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
            if let Ok(promise) = web_sys::Notification::request_permission() {
                let _ = promise;
            }
            v_null()
        }
        // RWEBNOTIFICATION show is handled above (before generic "show")
        // Web-exclusive: RWebGeolocation
        ("RWEBGEOLOCATION", "getposition") => {
            let name_owned = name.to_string();
            if let Some(window) = web_sys::window() {
                if let Ok(geolocation) = window.navigator().geolocation() {
                    let success_cb = Closure::<dyn FnMut(JsValue)>::new(move |pos_val: JsValue| {
                        if let Ok(pos) = pos_val.dyn_into::<web_sys::Position>() {
                            let coords = pos.coords();
                            crate::object_web::rp_comp_set(&name_owned, "latitude", Value::Double(coords.latitude()));
                            crate::object_web::rp_comp_set(&name_owned, "longitude", Value::Double(coords.longitude()));
                            crate::object_web::rp_comp_set(&name_owned, "accuracy", Value::Double(coords.accuracy()));
                            crate::object_web::rp_fire_event(&name_owned, "onchange");
                        }
                    });
                    let error_cb = Closure::<dyn FnMut(JsValue)>::new(move |err_val: JsValue| {
                        if let Ok(err) = err_val.dyn_into::<web_sys::PositionError>() {
                            web_sys::console::error_1(&err.message().into());
                        }
                    });
                    let _ = geolocation.get_current_position_with_error_callback(
                        success_cb.as_ref().unchecked_ref(),
                        Some(error_cb.as_ref().unchecked_ref()),
                    );
                    success_cb.forget();
                    error_cb.forget();
                }
            }
            v_null()
        }
        ("RWEBGEOLOCATION", "watchposition") => {
            let name_owned = name.to_string();
            if let Some(window) = web_sys::window() {
                if let Ok(geolocation) = window.navigator().geolocation() {
                    let success_cb = Closure::<dyn FnMut(JsValue)>::new(move |pos_val: JsValue| {
                        if let Ok(pos) = pos_val.dyn_into::<web_sys::Position>() {
                            let coords = pos.coords();
                            crate::object_web::rp_comp_set(&name_owned, "latitude", Value::Double(coords.latitude()));
                            crate::object_web::rp_comp_set(&name_owned, "longitude", Value::Double(coords.longitude()));
                            crate::object_web::rp_comp_set(&name_owned, "accuracy", Value::Double(coords.accuracy()));
                            crate::object_web::rp_fire_event(&name_owned, "onchange");
                        }
                    });
                    let error_cb = Closure::<dyn FnMut(JsValue)>::new(move |err_val: JsValue| {
                        if let Ok(err) = err_val.dyn_into::<web_sys::PositionError>() {
                            web_sys::console::error_1(&err.message().into());
                        }
                    });
                    if let Ok(watch_id) = geolocation.watch_position_with_error_callback(
                        success_cb.as_ref().unchecked_ref(),
                        Some(error_cb.as_ref().unchecked_ref()),
                    ) {
                        crate::object_web::rp_comp_set(name, "watchid", Value::Integer(watch_id as i64));
                        success_cb.forget();
                        error_cb.forget();
                        return Value::Integer(watch_id as i64);
                    }
                }
            }
            v_null()
        }
        ("RWEBGEOLOCATION", "clearwatch") => {
            let watch_id = crate::object_web::rp_comp_get_stored(name, "watchid").to_i64();
            if watch_id != 0 {
                if let Some(window) = web_sys::window() {
                    if let Ok(geolocation) = window.navigator().geolocation() {
                        let _ = geolocation.clear_watch(watch_id as i32);
                        crate::object_web::rp_comp_set(name, "watchid", Value::Integer(0));
                    }
                }
            }
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
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "[RapidR][NotImplemented] {}.{}() — method not implemented on web runtime (component type: {}). This call will return Null. Native target may support it.",
                name, method, comp_type
            )));
            v_null()
        }
    }
}

// ---------------------------------------------------------------------------
// Form creation
// ---------------------------------------------------------------------------

fn create_form(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_id(id);
    el.set_class_name("rr-form");
    // Focusable by a click (not by Tab), so a form's OnKeyDown gets the
    // keys when none of its controls has the focus.
    let _ = el.set_attribute("tabindex", "-1");
    let _ = el.style().set_property("outline", "none");
    track_mouse();
    let _ = el.set_attribute("data-rr-name", name);
    let _ = el.set_attribute("data-rr-type", "RFORM");
    if let Some(p) = props.get("parent").map(|v| v.to_string_val()).filter(|s| !s.is_empty()) {
        let _ = el.set_attribute("data-rr-parent", &p);
    }

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
    // Width / Height are the whole form, frame included, as in RapidQ.
    let _ = style.set_property("box-sizing", "border-box");

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
    // Its icon (Icon / IcoHandle, else Application.Icon): `apply_form_icon`.
    let icon = create_el("img");
    icon.set_class_name("rr-form-icon");
    let _ = icon.set_attribute("style", "width:16px;height:16px;margin-right:6px;display:none;");
    let _ = icon.set_attribute("draggable", "false");
    let _ = titlebar.append_child(&icon);
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
    let _ = c_style.set_property("position", "absolute");
    let _ = c_style.set_property("left", "0");
    let _ = c_style.set_property("top", "29px");
    let _ = c_style.set_property("width", "100%");
    let _ = c_style.set_property("height", "calc(100% - 29px)");
    let _ = c_style.set_property("overflow", "auto");
    let _ = el.append_child(&client);

    // Hidden initially — shown via gui_web_finalize()
    let _ = style.set_property("display", "none");
    let border_style = props.get("borderstyle").map_or(2, |v| v.to_i64());
    place_form_chrome(&el, border_style, false);

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

    // If a parent form is specified, nest inside its client area;
    // otherwise the form is top-level under <body>.
    let parent_name = props.get("parent").map(|v| v.to_string_val()).filter(|s| !s.is_empty());
    let host = get_parent_client(&parent_name);
    let _ = host.append_child(&el);
    apply_form_icon(name);
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
    el.set_inner_text(&strip_ampersands(&caption));
    el.set_class_name("rr-widget");
    setup_widget(&el, id, name, props);
}

fn create_coolbtn(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("button");
    let caption = props.get("caption").map(|v| v.to_string_val()).unwrap_or_default();
    el.set_inner_text(&strip_ampersands(&caption));
    el.set_class_name("rr-widget rr-coolbtn");

    let flat = props.get("flat").map(|v| v.to_i64() != 0).unwrap_or(false);
    let style = el.style();
    if flat {
        let _ = style.set_property("border", "none");
        let _ = style.set_property("background", "transparent");
    } else {
        let _ = style.set_property("border", "1px solid #999");
        let _ = style.set_property("background", "#e1e1e1");
    }
    let _ = style.set_property("cursor", "pointer");

    // Load BMP if specified
    let bmp_path = props.get("bmp").map(|v| v.to_string_val()).unwrap_or_default();
    if !bmp_path.is_empty() {
        let img = document().create_element("img").unwrap();
        let _ = img.set_attribute("src", &bmp_path);
        let _ = img.set_attribute("style", "vertical-align: middle; margin-right: 4px;");
        let _ = el.prepend_with_node_1(&img);
    }

    setup_widget(&el, id, name, props);
    toggle_on_click(&el, name);
    show_down(name);
}

/// A QCOOLBTN / QOVALBTN press applies its group's rule (added before the
/// program's OnClick, which then reads the new Down).
fn toggle_on_click(el: &web_sys::HtmlElement, name: &str) {
    let owner = name.to_uppercase();
    let cb = Closure::<dyn FnMut()>::new(move || toggle_press(&owner));
    let _ = el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
    cb.forget();
}

/// The toggle buttons sharing `name`'s parent.
fn toggle_members(name: &str) -> Vec<rapidr_value::toggle_group::Member> {
    use crate::object_web::{get_children_of, rp_comp_get_stored};
    let parent = rp_comp_get_stored(name, "parent").to_string_val();
    get_children_of(&parent)
        .into_iter()
        .filter(|(_, t)| matches!(t.to_uppercase().as_str(), "RCOOLBTN" | "ROVALBTN"))
        .map(|(n, _)| rapidr_value::toggle_group::Member {
            group: rp_comp_get_stored(&n, "groupindex").to_i64(),
            down: rp_comp_get_stored(&n, "down").to_bool(),
            name: n,
        })
        .collect()
}

/// A button shows its Down (pressed in).
fn show_down(name: &str) {
    let down = crate::object_web::rp_comp_get_stored(name, "down").to_bool();
    if let Some(el) = document().get_element_by_id(&comp_id(name)) {
        let _ = el.class_list().toggle_with_force("rr-down", down);
        if let Ok(el) = el.dyn_into::<web_sys::HtmlElement>() {
            let _ = el.style().set_property("border-style", if down { "inset" } else { "" });
        }
    }
}

fn toggle_apply(changes: Vec<(String, bool)>) {
    for (n, down) in changes {
        crate::object_web::rp_comp_set_prop_only(&n, "down", v_int(if down { -1 } else { 0 }));
        show_down(&n);
    }
}

fn toggle_press(name: &str) {
    let allow_all_up = crate::object_web::rp_comp_get_stored(name, "allowallup").to_bool();
    toggle_apply(rapidr_value::toggle_group::press(name, allow_all_up, &toggle_members(name)));
}

/// The program set a button's Down: the others of its group come up, and
/// it shows the new state.
pub fn toggle_down_set(name: &str) {
    if !matches!(crate::object_web::rp_comp_type(name).to_uppercase().as_str(), "RCOOLBTN" | "ROVALBTN") {
        return;
    }
    let down = crate::object_web::rp_comp_get_stored(name, "down").to_bool();
    let mut changes = rapidr_value::toggle_group::set_down(name, down, &toggle_members(name));
    changes.push((name.to_string(), down));
    toggle_apply(changes);
}

/// A QFORMMDI child window's frame (`RMDICHILD`, placed by
/// `rapidr_value::mdi`): border, a title bar with the title and minimize /
/// maximize / close buttons; dragged by its title bar or its corner.
fn create_mdi_frame(id: &str, name: &str, props: &HashMap<String, Value>) {
    use rapidr_value::mdi::{Action, BORDER, TITLE_HEIGHT};
    let el = create_el("div");
    el.set_class_name("rr-widget rr-mdichild");
    let style = el.style();
    for (k, v) in [("background", "#d4d0c8"), ("border", "1px outset #eee"), ("box-sizing", "border-box"), ("user-select", "none")] {
        let _ = style.set_property(k, v);
    }
    let bar = create_el("div");
    bar.set_class_name("rr-mdichild-title");
    let bs = bar.style();
    let bar_h = (TITLE_HEIGHT - 2).to_string() + "px";
    for (k, v) in [("position", "absolute"), ("left", &format!("{BORDER}px")), ("right", &format!("{BORDER}px")), ("top", &format!("{BORDER}px")), ("height", bar_h.as_str()), ("display", "flex"), ("align-items", "center"), ("color", "#fff"), ("font", "bold 12px sans-serif"), ("padding-left", "4px"), ("box-sizing", "border-box"), ("cursor", "default")] {
        let _ = bs.set_property(k, v);
    }
    let title = create_el("span");
    title.set_class_name("rr-mdichild-caption");
    let _ = title.style().set_property("flex", "1");
    let _ = title.style().set_property("overflow", "hidden");
    let _ = title.style().set_property("white-space", "nowrap");
    let _ = bar.append_child(&title);
    let owner = name.to_uppercase();
    let target = |owner: &str| {
        let form = crate::object_web::rp_comp_get_stored(owner, "__form").to_string_val();
        let component = crate::object_web::rp_comp_get_stored(owner, "__component").to_string_val();
        (form, component)
    };
    for (glyph, action) in [("_", Action::Minimize), ("\u{25a1}", Action::ToggleMaximize), ("x", Action::Close)] {
        let b = create_el("button");
        b.set_inner_text(glyph);
        b.set_class_name("rr-mdichild-button");
        let _ = b.set_attribute("data-action", &format!("{action:?}").to_lowercase());
        let s = b.style();
        for (k, v) in [("width", "17px"), ("height", "15px"), ("padding", "0"), ("margin-left", "2px"), ("font", "bold 10px sans-serif"), ("line-height", "10px")] {
            let _ = s.set_property(k, v);
        }
        let o = owner.clone();
        let click = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            e.stop_propagation();
            let (form, component) = target(&o);
            crate::mdi_web::user(&form, &component, action);
        });
        let _ = b.add_event_listener_with_callback("click", click.as_ref().unchecked_ref());
        click.forget();
        // (a press on a button doesn't start a drag)
        let stop = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| e.stop_propagation());
        let _ = b.add_event_listener_with_callback("mousedown", stop.as_ref().unchecked_ref());
        stop.forget();
        let _ = bar.append_child(&b);
    }
    let _ = el.append_child(&bar);
    let grip = create_el("div");
    let gs = grip.style();
    for (k, v) in [("position", "absolute"), ("right", "0"), ("bottom", "0"), ("width", "12px"), ("height", "12px"), ("cursor", "nwse-resize")] {
        let _ = gs.set_property(k, v);
    }
    let _ = el.append_child(&grip);
    // A press anywhere activates; on the title bar it moves the window, on
    // the corner it resizes it.
    for (part, resize) in [(bar.clone(), false), (grip.clone(), true), (el.clone(), false)] {
        let o = owner.clone();
        let is_frame = part == el;
        let down = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let (form, component) = target(&o);
            crate::mdi_web::user(&form, &component, Action::Activate);
            if is_frame {
                return;
            }
            e.stop_propagation();
            e.prevent_default();
            let get = |p: &str| crate::object_web::rp_comp_get_stored(&o, p).to_i64();
            let start = if resize { (get("width"), get("height")) } else { (get("left"), get("top")) };
            let (mx, my) = (e.client_x() as i64, e.client_y() as i64);
            let (f2, c2) = (form.clone(), component.clone());
            let moving = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |m: web_sys::MouseEvent| {
                let (dx, dy) = (m.client_x() as i64 - mx, m.client_y() as i64 - my);
                let action = if resize { Action::Resize(start.0 + dx, start.1 + dy) } else { Action::Move(start.0 + dx, start.1 + dy) };
                crate::mdi_web::user(&f2, &c2, action);
            });
            let doc = document();
            let _ = doc.add_event_listener_with_callback("mousemove", moving.as_ref().unchecked_ref());
            let moving_fn: js_sys::Function = moving.into_js_value().unchecked_into();
            let mv = moving_fn.clone();
            let up = Closure::once_into_js(move || {
                let _ = document().remove_event_listener_with_callback("mousemove", &mv);
            });
            let options = web_sys::AddEventListenerOptions::new();
            options.set_once(true);
            let _ = doc.add_event_listener_with_callback_and_add_event_listener_options("mouseup", up.unchecked_ref(), &options);
        });
        let _ = part.add_event_listener_with_callback("mousedown", down.as_ref().unchecked_ref());
        down.forget();
    }
    let o = owner.clone();
    let dbl = Closure::<dyn FnMut()>::new(move || {
        let (form, component) = target(&o);
        crate::mdi_web::user(&form, &component, Action::ToggleMaximize);
    });
    let _ = bar.add_event_listener_with_callback("dblclick", dbl.as_ref().unchecked_ref());
    dbl.forget();
    setup_widget(&el, id, name, props);
    mdi_frame_update(name);
}

/// A child window's frame shows its title, whether it's the active one,
/// and whether it's maximized.
pub fn mdi_frame_update(name: &str) {
    let Some(el) = document().get_element_by_id(&comp_id(name)) else { return };
    let get = |p: &str| crate::object_web::rp_comp_get_stored(name, p);
    if let Ok(Some(t)) = el.query_selector(".rr-mdichild-caption") {
        t.set_text_content(Some(&get("caption").to_string_val()));
    }
    if let Ok(Some(bar)) = el.query_selector(".rr-mdichild-title") {
        if let Ok(bar) = bar.dyn_into::<web_sys::HtmlElement>() {
            let _ = bar.style().set_property("background", if get("active").to_bool() { "#0a246a" } else { "#808080" });
        }
    }
    if let Ok(Some(b)) = el.query_selector("[data-action=togglemaximize]") {
        b.set_text_content(Some(if get("childstate").to_i64() == 2 { "=" } else { "\u{25a1}" }));
    }
}

/// Puts these elements on top of the others in their parent, in order (the
/// last on top): a QFORMMDI's frames and components.
pub fn stack_elements(names: &[String]) {
    for (i, name) in names.iter().enumerate() {
        if let Some(el) = document().get_element_by_id(&comp_id(name)).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
            let _ = el.style().set_property("z-index", &(100 + i).to_string());
        }
    }
}

fn create_ovalbtn(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("button");
    let caption = props.get("caption").map(|v| v.to_string_val()).unwrap_or_default();
    el.set_inner_text(&strip_ampersands(&caption));
    el.set_class_name("rr-widget rr-ovalbtn");

    let color = props.get("color").map(|v| {
        let c = v.to_i64();
        let r = c & 0xFF;
        let g = (c >> 8) & 0xFF;
        let b = (c >> 16) & 0xFF;
        format!("rgb({},{},{})", r, g, b)
    }).unwrap_or_else(|| "#dcdcdc".to_string());
    let hl = props.get("colorhighlight").map(|v| {
        let c = v.to_i64();
        format!("rgb({},{},{})", c & 0xFF, (c >> 8) & 0xFF, (c >> 16) & 0xFF)
    }).unwrap_or_else(|| "#fff".to_string());

    let style = el.style();
    let _ = style.set_property("border-radius", "50%");
    let _ = style.set_property("background", &color);
    let _ = style.set_property("border", &format!("2px outset {}", hl));
    let _ = style.set_property("cursor", "pointer");

    setup_widget(&el, id, name, props);
    toggle_on_click(&el, name);
    show_down(name);
}

fn create_label(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("span");
    let caption = props.get("caption").map(|v| v.to_string_val()).unwrap_or_default();
    el.set_inner_text(&strip_ampersands(&caption));
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
    let _ = el.style().set_property("background", "#f0f0f0");
    // RapidQ's panel shows its Caption centered, under its children (which
    // setting the caption must not remove).
    let caption = create_el("span");
    caption.set_class_name("rr-panel-caption");
    let cs = caption.style();
    for (k, v) in [("position", "absolute"), ("inset", "0"), ("display", "flex"), ("align-items", "center"), ("justify-content", "center"), ("pointer-events", "none"), ("overflow", "hidden"), ("white-space", "nowrap")] {
        let _ = cs.set_property(k, v);
    }
    caption.set_text_content(Some(&strip_ampersands(&props.get("caption").map(|v| v.to_string_val()).unwrap_or_default())));
    let _ = el.append_child(&caption);
    setup_widget(&el, id, name, props);
    render_panel_bevels(name);
}

/// A QPANEL's BevelOuter / BevelInner frames (rapidr_value::objects::
/// bevel): one-pixel boxes over its edges, under its children.
pub fn render_panel_bevels(name: &str) {
    let Some(el) = get_el(&comp_id(name)) else { return };
    if let Ok(old) = el.query_selector_all(":scope > .rr-bevel") {
        for i in 0..old.length() {
            if let Some(n) = old.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                n.remove();
            }
        }
    }
    let prop = |p: &str| crate::object_web::rp_comp_get(name, p).to_i64();
    let hex = |c: u32| format!("#{c:06x}");
    let first = el.first_child();
    for f in rapidr_value::objects::bevel::frames(prop("bevelouter"), prop("bevelinner"), prop("bevelwidth"), prop("borderwidth")) {
        let b = create_el("div");
        b.set_class_name("rr-bevel");
        let (tl, br) = (hex(f.top_left), hex(f.bottom_right));
        for (k, v) in [
            ("position", "absolute".to_string()),
            ("inset", format!("{}px", f.inset)),
            ("pointer-events", "none".into()),
            ("border", "1px solid".into()),
            ("border-color", format!("{tl} {br} {br} {tl}")),
        ] {
            let _ = b.style().set_property(k, &v);
        }
        let _ = el.insert_before(&b, first.as_ref());
    }
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
    span.set_inner_text(&strip_ampersands(&caption));
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
    span.set_inner_text(&strip_ampersands(&caption));
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

/// QCOMBOBOX (a drop-down) or QLISTBOX (`list`: a list of rows), drawn
/// from the shared items (rapidr_value::objects::list) by [`render_list`].
fn create_select(id: &str, name: &str, list: bool, props: &HashMap<String, Value>) {
    let el = create_el("select");
    if list {
        let _ = el.set_attribute("size", "2");
    }
    el.set_class_name("rr-widget");
    // The user's pick goes into the items first, so the program's OnClick /
    // OnChange (bound later) reads the new ItemIndex / Text / Selected().
    let owner = name.to_uppercase();
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
        let Some(sel) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()) else { return };
        let options = sel.options();
        let picked: Vec<bool> = (0..options.length())
            .map(|i| options.item(i).and_then(|o| o.dyn_into::<web_sys::HtmlOptionElement>().ok()).is_some_and(|o| o.selected()))
            .collect();
        let index = sel.selected_index() as i64;
        rapidr_value::objects::with_list_mut(&owner, |l| {
            if l.multi_select {
                l.selected = picked;
                l.selected.resize(l.items.len(), false);
                l.item_index = index;
            } else {
                l.select(index);
            }
        });
    });
    // (OnChange is bound to `input`, which comes before `change`.)
    for dom_event in ["input", "change"] {
        let _ = el.add_event_listener_with_callback(dom_event, cb.as_ref().unchecked_ref());
    }
    cb.forget();
    setup_widget(&el, id, name, props);
    render_list_now(name);
    // (its Style or Columns was set before the element was made)
    if list && rapidr_value::objects::with_list(name, |l| l.custom_drawn()).unwrap_or(false) {
        convert_to_owner_list(name);
    }
}

/// An owner-drawn QLISTBOX (`Style = lbOwnerDrawFixed / lbOwnerDrawVariable`):
/// a scrolling list of canvases, one per item, from the shared model
/// (rapidr_value::objects::list, `render_item`): what OnDrawItem drew, or the
/// plain item. A click or the arrow keys select (the items first, then the
/// program's OnClick, bound later); a MultiSelect list toggles.
fn create_owner_list(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_class_name("rr-widget");
    attach_owner_list(&el, name);
    setup_widget(&el, id, name, props);
    render_list_now(name);
}

/// `Style = lbOwnerDrawFixed / lbOwnerDrawVariable` or `Columns` set on a
/// list box whose element is a `<select>` already (the style comes after it's made): the
/// element becomes the owner-drawn list, keeping its place and geometry.
/// (Events the program bound to the old element must come after the Style.)
pub fn convert_to_owner_list(name: &str) {
    let Some(old) = get_el(&comp_id(name)) else { return };
    if !old.tag_name().eq_ignore_ascii_case("select") {
        return;
    }
    let el = create_el("div");
    for attr in ["id", "class", "style", "data-rr-name", "data-rr-type"] {
        if let Some(v) = old.get_attribute(attr) {
            let _ = el.set_attribute(attr, &v);
        }
    }
    attach_owner_list(&el, name);
    let _ = old.replace_with_with_node_1(&el);
    render_list_now(name);
}

/// The look and the click / keyboard selection of an owner-drawn list.
fn attach_owner_list(el: &web_sys::HtmlElement, name: &str) {
    let _ = el.set_attribute("tabindex", "0");
    let st = el.style();
    let _ = st.set_property("overflow-y", "auto");
    let _ = st.set_property("overflow-x", "hidden");
    let _ = st.set_property("background", "white");
    let _ = st.set_property("border", "1px solid #999");
    let _ = st.set_property("box-sizing", "border-box");
    let owner = name.to_uppercase();
    // (MultiSelect: Shift / Ctrl held extend or toggle, as on the desktop)
    let select = move |i: i64, shift: bool, ctrl: bool| {
        rapidr_value::objects::with_list_mut(&owner, |l| l.click(i, shift, ctrl));
        render_list(&owner);
    };
    let click_select = select.clone();
    let click = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        let mut node = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        while let Some(n) = node {
            if let Some(i) = n.get_attribute("data-item").and_then(|v| v.parse::<i64>().ok()) {
                click_select(i, e.shift_key(), e.ctrl_key() || e.meta_key());
                return;
            }
            node = n.parent_element();
        }
    });
    let _ = el.add_event_listener_with_callback("click", click.as_ref().unchecked_ref());
    click.forget();
    let key_owner = name.to_uppercase();
    let key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        let Some((count, current, height, multi, per)) = rapidr_value::objects::with_list(&key_owner, |l| (l.items.len() as i64, l.item_index, l.row_height(), l.multi_column(), l.column_layout().0)) else { return };
        let page = if multi { per } else { (e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).map_or(200, |t| t.client_height() as i64) / height).max(1) };
        let next = match e.key().as_str() {
            "ArrowDown" => current + 1,
            "ArrowUp" => (current - 1).max(0),
            // In columns: the item beside.
            "ArrowRight" if multi => current + per,
            "ArrowLeft" if multi => (current - per).max(0),
            "PageDown" => current + page,
            "PageUp" => (current - page).max(0),
            "Home" => 0,
            "End" => count - 1,
            _ => return,
        };
        e.prevent_default();
        let next = next.clamp(0, (count - 1).max(0));
        if count > 0 && next != current {
            select(next, false, false);
            crate::object_web::rp_fire_event(&key_owner, "onclick");
        }
    });
    let _ = el.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref());
    key.forget();
}

/// A QCOMBOBOX with an edit box (as on the desktop): an input suggesting
/// the items (a datalist). Typing or picking sets its Text (and the
/// ItemIndex of the item it matches, else -1) before OnChange, which is
/// bound to `input`.
fn create_edit_combo(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("input");
    el.set_class_name("rr-widget");
    let list_id = format!("{id}-items");
    let _ = el.set_attribute("list", &list_id);
    let _ = el.set_attribute("autocomplete", "off");
    let owner = name.to_uppercase();
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
        let Some(input) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) else { return };
        let text = input.value();
        rapidr_value::objects::with_list_mut(&owner, |l| l.set("text", &crate::value::v_str(&text)));
    });
    let _ = el.add_event_listener_with_callback("input", cb.as_ref().unchecked_ref());
    cb.forget();
    setup_widget(&el, id, name, props);
    let datalist = create_el("datalist");
    datalist.set_id(&list_id);
    if let Some(parent) = el.parent_node() {
        let _ = parent.append_child(&datalist);
    }
    render_list_now(name);
}

/// A QDIRTREE (as on the desktop): its rows in a list; a click selects a
/// directory (OnChange), a double click opens or closes it.
fn create_dirtree(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("select");
    let _ = el.set_attribute("size", "2");
    el.set_class_name("rr-widget");
    let owner = name.to_uppercase();
    let pick = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
        let Some(sel) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()) else { return };
        let Ok(i) = usize::try_from(sel.selected_index()) else { return };
        if rapidr_value::objects::with_dirtree(&owner, |t| t.click(i)).unwrap_or(false) {
            crate::object_web::rp_fire_event(&owner, "onchange");
        }
    });
    let _ = el.add_event_listener_with_callback("change", pick.as_ref().unchecked_ref());
    pick.forget();
    let owner = name.to_uppercase();
    let open = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
        let Some(sel) = e.current_target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()) else { return };
        let Ok(i) = usize::try_from(sel.selected_index()) else { return };
        rapidr_value::objects::with_dirtree(&owner, |t| t.toggle(i));
        render_dirtree(&owner);
    });
    let _ = el.add_event_listener_with_callback("dblclick", open.as_ref().unchecked_ref());
    open.forget();
    setup_widget(&el, id, name, props);
    render_dirtree(name);
}

/// Shows a QDIRTREE's rows (indented, `+` closed / `-` open).
pub fn render_dirtree(name: &str) {
    let Some(sel) = get_el(&comp_id(name)).and_then(|e| e.dyn_into::<web_sys::HtmlSelectElement>().ok()) else { return };
    let Some((lines, selected)) = rapidr_value::objects::with_dirtree(name, |t| {
        (t.rows().iter().map(rapidr_value::objects::dirtree::DirTree::row_text).collect::<Vec<_>>(), t.selected_row())
    }) else {
        return;
    };
    sel.set_length(0);
    for line in lines {
        let Ok(opt) = document().create_element("option").map(|o| o.unchecked_into::<web_sys::HtmlOptionElement>()) else { continue };
        opt.set_text(&line.replace(' ', "\u{a0}"));
        let _ = sel.add_with_html_option_element(&opt);
    }
    sel.set_selected_index(selected.map_or(-1, |i| i as i32));
}

thread_local! {
    /// Lists to redraw once the program yields (a loop of AddItems redraws
    /// once).
    static LISTS_TO_RENDER: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Redraws a QLISTBOX's / QCOMBOBOX's options from its items, soon.
pub fn render_list(name: &str) {
    let name = name.to_uppercase();
    let first = LISTS_TO_RENDER.with(|l| {
        let mut l = l.borrow_mut();
        let first = l.is_empty();
        if !l.contains(&name) {
            l.push(name);
        }
        first
    });
    if !first {
        return;
    }
    let flush = Closure::once_into_js(move || {
        for name in LISTS_TO_RENDER.with(|l| std::mem::take(&mut *l.borrow_mut())) {
            render_list_now(&name);
        }
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(flush.unchecked_ref(), 0);
    }
}

/// The options as plain text, ItemIndex (or, with MultiSelect, every
/// selected item) selected.
fn render_list_now(name: &str) {
    let Some(el) = get_el(&comp_id(name)) else { return };
    // An edit combo: the items as suggestions, the Text in the edit box.
    if let Ok(input) = el.clone().dyn_into::<web_sys::HtmlInputElement>() {
        let Some(datalist) = get_el(&format!("{}-items", comp_id(name))) else { return };
        let _ = rapidr_value::objects::with_list(name, |l| {
            datalist.set_inner_text("");
            for item in &l.items {
                let Ok(opt) = document().create_element("option").map(|o| o.unchecked_into::<web_sys::HtmlOptionElement>()) else { continue };
                opt.set_value(item);
                let _ = datalist.append_child(&opt);
            }
            if input.value() != l.text {
                input.set_value(&l.text);
            }
        });
        return;
    }
    // An owner-drawn combo box: its box.
    if el.class_list().contains("rr-owner-combo") {
        render_owner_combo(&el, name);
        return;
    }
    // An owner-drawn or multi-column list box: a canvas per item.
    if el.class_list().contains("rr-widget") && el.tag_name().eq_ignore_ascii_case("div") {
        render_owner_list(&el, name);
        return;
    }
    let Ok(sel) = el.dyn_into::<web_sys::HtmlSelectElement>() else { return };
    let _ = rapidr_value::objects::with_list(name, |l| {
        sel.set_length(0);
        if !l.combo {
            sel.set_multiple(l.multi_select);
        }
        for (i, item) in l.items.iter().enumerate() {
            let Ok(opt) = document().create_element("option").map(|o| o.unchecked_into::<web_sys::HtmlOptionElement>()) else { continue };
            opt.set_text(item);
            opt.set_value(&i.to_string());
            opt.set_selected(l.is_selected(i));
            let _ = sel.add_with_html_option_element(&opt);
        }
        if !l.multi_select {
            sel.set_selected_index(l.item_index as i32);
        }
    });
}

/// Shows an owner-drawn list box's items (see [`create_owner_list`]) and
/// fires OnDrawItem for them if the list changed.
fn render_owner_list(el: &web_sys::Element, name: &str) {
    // What shows (as on the desktop): the control less its frame and a
    // scroll bar — beside the items, or under them in columns.
    let stored = |p: &str| crate::object_web::rp_comp_get_stored(name, p).to_i64();
    let multi = rapidr_value::objects::with_list(name, |l| l.multi_column()).unwrap_or(false);
    let (vw, vh) = if multi { (stored("width") - 4, stored("height") - 20) } else { (stored("width") - 20, stored("height") - 4) };
    rapidr_value::objects::with_list_mut(name, |l| l.set_view(vw, vh));
    if list_measure(name) {
        return;
    }
    let Some((per, cw, rh)) = rapidr_value::objects::with_list(name, |l| {
        let (per, cw) = l.column_layout();
        (per, cw, l.row_height())
    }) else {
        return;
    };
    let width = if multi { cw } else { vw.max(20) };
    if let Some(html) = el.dyn_ref::<web_sys::HtmlElement>() {
        let st = html.style();
        let layout: &[(&str, String)] = if multi {
            &[("display", "grid".into()), ("grid-auto-flow", "column".into()), ("grid-template-rows", format!("repeat({per}, {rh}px)")), ("grid-auto-columns", format!("{cw}px")), ("align-content", "start".into()), ("overflow-x", "auto".into()), ("overflow-y", "hidden".into())]
        } else {
            &[("display", "block".into()), ("overflow-x", "hidden".into()), ("overflow-y", "auto".into())]
        };
        for (k, v) in layout {
            let _ = st.set_property(k, v);
        }
    }
    let (scroll, scroll_x) = (el.scroll_top(), el.scroll_left());
    el.set_inner_html("");
    for i in 0..rapidr_value::objects::with_list(name, |l| l.items.len()).unwrap_or(0).min(rapidr_value::objects::list::MAX_OWNER_DRAWN) {
        if let Some(row) = item_canvas(name, i, width) {
            let _ = row.set_attribute("data-item", &i.to_string());
            let _ = row.style().set_property("cursor", "default");
            let _ = el.append_child(&row);
        }
    }
    el.set_scroll_top(scroll);
    el.set_scroll_left(scroll_x);
    list_owner_draw(name);
}

/// The page's scale (`devicePixelRatio`: 2 on a Retina screen, more when
/// zoomed in), for bitmaps to keep what they show at it
/// (rapidr_value::objects::bitmap); a page's `RAPIDR_SCALE` sets it (tests).
fn note_display_scale() {
    let Some(window) = web_sys::window() else { return };
    let forced = js_sys::Reflect::get(&window, &"RAPIDR_SCALE".into()).ok().and_then(|v| v.as_f64());
    rapidr_value::objects::bitmap::set_display_scale(forced.unwrap_or_else(|| window.device_pixel_ratio()));
}

/// Puts what a bitmap shows (`display_rgba`: w × h device pixels, `scale`
/// of them a pixel) on `canvas`, sized in pixels on the page.
fn put_display(canvas: &web_sys::HtmlCanvasElement, w: usize, h: usize, rgba: &[u8], scale: usize) {
    let (w32, h32) = (w as u32, h as u32);
    if canvas.width() != w32 || canvas.height() != h32 {
        canvas.set_width(w32);
        canvas.set_height(h32);
    }
    let _ = canvas.style().set_property("width", &format!("{}px", w / scale.max(1)));
    let _ = canvas.style().set_property("height", &format!("{}px", h / scale.max(1)));
    if let Some(ctx) = canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()) {
        if let Ok(data) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(rgba), w32, h32) {
            let _ = ctx.put_image_data(&data, 0.0, 0.0);
        }
    }
}

/// Item `i` of a list as its model draws it (`render_item`: what OnDrawItem
/// drew, or the plain item), `width` wide: a div holding a canvas.
fn item_canvas(name: &str, i: usize, width: i64) -> Option<web_sys::HtmlElement> {
    let font = rapidr_value::objects::font_from_props(name, &|id, p| crate::object_web::rp_comp_get_stored(id, p));
    note_display_scale();
    let (w, h, rgba, scale) = rapidr_value::objects::list_item_pixels(name, i, width, &font)?;
    let row = create_el("div");
    let _ = row.style().set_property("height", &format!("{}px", h / scale.max(1)));
    let canvas = document().create_element("canvas").ok()?.dyn_into::<web_sys::HtmlCanvasElement>().ok()?;
    let _ = canvas.style().set_property("display", "block");
    put_display(&canvas, w, h, &rgba, scale);
    let _ = row.append_child(&canvas);
    Some(row)
}

/// An owner-drawn QCOMBOBOX (`Style` csOwnerDrawFixed / csOwnerDrawVariable),
/// as on the desktop: a box showing the selected item as OnDrawItem drew it
/// and a button; a click drops down the items, each as drawn, and the pick
/// is the ItemIndex (OnChange). Up / Down pick the item before / after.
fn create_owner_combo(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_class_name("rr-widget");
    attach_owner_combo(&el, name);
    setup_widget(&el, id, name, props);
    render_list_now(name);
}

/// `Style = csOwnerDrawFixed / csOwnerDrawVariable` set on a combo box whose
/// element is made already (the style comes after it): the element becomes
/// the owner-drawn box, keeping its place and geometry.
pub fn convert_to_owner_combo(name: &str) {
    let Some(old) = get_el(&comp_id(name)) else { return };
    if old.class_list().contains("rr-owner-combo") {
        return;
    }
    let el = create_el("div");
    for attr in ["id", "class", "style", "data-rr-name", "data-rr-type"] {
        if let Some(v) = old.get_attribute(attr) {
            let _ = el.set_attribute(attr, &v);
        }
    }
    attach_owner_combo(&el, name);
    let _ = old.replace_with_with_node_1(&el);
    if let Some(items) = get_el(&format!("{}-items", comp_id(name))) {
        items.remove();
    }
    render_list_now(name);
}

/// The look, the drop-down and the keys of an owner-drawn combo box.
fn attach_owner_combo(el: &web_sys::HtmlElement, name: &str) {
    let _ = el.class_list().add_1("rr-owner-combo");
    let _ = el.set_attribute("tabindex", "0");
    let st = el.style();
    for (k, v) in [("background", "white"), ("border", "1px solid #999"), ("box-sizing", "border-box"), ("overflow", "hidden"), ("cursor", "default"), ("outline", "none")] {
        let _ = st.set_property(k, v);
    }
    let owner = name.to_uppercase();
    let click = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        e.stop_propagation();
        if let Some(box_el) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            owner_combo_drop(&owner, &box_el);
        }
    });
    let _ = el.add_event_listener_with_callback("mousedown", click.as_ref().unchecked_ref());
    click.forget();
    let key_owner = name.to_uppercase();
    let key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        let step = match e.key().as_str() {
            "ArrowDown" => 1,
            "ArrowUp" => -1,
            _ => return,
        };
        e.prevent_default();
        let Some((index, count)) = rapidr_value::objects::with_list(&key_owner, |l| (l.item_index, l.items.len() as i64)) else { return };
        let next = (index + step).clamp(0, (count - 1).max(0));
        if count > 0 && next != index {
            owner_combo_pick(&key_owner, next);
        }
    });
    let _ = el.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref());
    key.forget();
}

/// Shows an owner-drawn combo box's selected item and button.
fn render_owner_combo(el: &web_sys::Element, name: &str) {
    let stored = |p: &str| crate::object_web::rp_comp_get_stored(name, p).to_i64();
    let (w, h) = (stored("width"), stored("height"));
    rapidr_value::objects::with_list_mut(name, |l| l.set_view(w - 8, 10_000));
    if list_measure(name) {
        return;
    }
    el.set_inner_html("");
    let button = h.min(20);
    let index = rapidr_value::objects::with_list(name, |l| l.item_index).unwrap_or(-1);
    if let Some(item) = usize::try_from(index).ok().and_then(|i| item_canvas(name, i, w - button - 6)) {
        let st = item.style();
        let _ = st.set_property("position", "absolute");
        let _ = st.set_property("left", "2px");
        let _ = st.set_property("top", "50%");
        let _ = st.set_property("transform", "translateY(-50%)");
        let _ = st.set_property("overflow", "hidden");
        let _ = el.append_child(&item);
    }
    let arrow = create_el("div");
    arrow.set_text_content(Some("▾"));
    let st = arrow.style();
    for (k, v) in [("position", "absolute"), ("right", "1px"), ("top", "1px"), ("bottom", "1px"), ("background", "#e6e6e6"), ("border-left", "1px solid #999"), ("text-align", "center"), ("font-size", "12px")] {
        let _ = st.set_property(k, v);
    }
    let _ = st.set_property("width", &format!("{button}px"));
    let _ = st.set_property("line-height", &format!("{}px", h - 4));
    let _ = el.append_child(&arrow);
    list_owner_draw(name);
}

/// The combo box's drop-down: its items as drawn, under the box.
fn owner_combo_drop(name: &str, box_el: &web_sys::Element) {
    close_grid_drop_down();
    let width = crate::object_web::rp_comp_get_stored(name, "width").to_i64();
    let rect = box_el.get_bounding_client_rect();
    let list = create_el("div");
    list.set_class_name("rr-grid-dropdown");
    let st = list.style();
    for (k, v) in [("position", "fixed"), ("background", "white"), ("border", "1px solid #666"), ("z-index", "100000"), ("max-height", "300px"), ("overflow-y", "auto"), ("box-shadow", "2px 2px 4px rgba(0,0,0,.3)"), ("padding", "2px")] {
        let _ = st.set_property(k, v);
    }
    let _ = st.set_property("left", &format!("{}px", rect.left()));
    let _ = st.set_property("top", &format!("{}px", rect.bottom()));
    for i in 0..rapidr_value::objects::with_list(name, |l| l.items.len()).unwrap_or(0).min(rapidr_value::objects::list::MAX_OWNER_DRAWN) {
        let Some(row) = item_canvas(name, i, width - 8) else { continue };
        row.set_class_name("rr-grid-dropdown-item");
        let _ = row.set_attribute("data-item", &i.to_string());
        let owner = name.to_uppercase();
        let pick = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            e.stop_propagation();
            close_grid_drop_down();
            owner_combo_pick(&owner, i as i64);
        });
        let _ = row.add_event_listener_with_callback("mousedown", pick.as_ref().unchecked_ref());
        pick.forget();
        let _ = list.append_child(&row);
    }
    if let Some(body) = document().body() {
        let _ = body.append_child(&list);
    }
    // A click anywhere else closes it.
    let close = Closure::once_into_js(move || {
        let outside = Closure::<dyn FnMut()>::new(close_grid_drop_down);
        let options = web_sys::AddEventListenerOptions::new();
        options.set_once(true);
        let _ = document().add_event_listener_with_callback_and_add_event_listener_options("mousedown", outside.as_ref().unchecked_ref(), &options);
        outside.forget();
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(close.unchecked_ref(), 0);
    }
}

/// The user picked item `i` of an owner-drawn combo box: its ItemIndex,
/// then OnChange.
fn owner_combo_pick(name: &str, i: i64) {
    rapidr_value::objects::with_list_mut(name, |l| l.select(i));
    render_list_now(name);
    crate::object_web::rp_fire_event(name, "onchange");
}

/// OnMeasureItem(Index, Height) for each item of a `lbOwnerDrawVariable`
/// list whose items changed, as on the desktop (rapidr_value::objects::
/// list): each answer is the item's height, and the list is shown again
/// once the last is in. `true` while answers are still to come.
fn list_measure(name: &str) -> bool {
    if crate::object_web::rp_has_handler(name, "onmeasureitem") {
        let asks = rapidr_value::objects::with_list_mut(name, |l| l.measure_needed()).unwrap_or_default();
        for (round, i, h) in asks {
            let list = name.to_string();
            crate::object_web::rp_fire_event_then(name, "onmeasureitem", &[v_int(i as i64), v_int(h)], move |a| {
                if rapidr_value::objects::with_list_mut(&list, |l| l.measured(round, i, a[1].to_i64())).unwrap_or(false) {
                    render_list(&list);
                }
            });
        }
    }
    rapidr_value::objects::with_list(name, |l| l.measuring()).unwrap_or(false)
}

/// OnDrawItem(Index, State, Rect): fired for every item after the list
/// changed, as on the desktop (rapidr_value::objects::list). Each item's
/// Rect is a QRECT (a property bag).
fn list_owner_draw(name: &str) {
    if !crate::object_web::rp_has_handler(name, "ondrawitem") {
        return;
    }
    if !rapidr_value::objects::with_list_mut(name, |l| l.owner_drawn() && l.owner_draw_needed()).unwrap_or(false) {
        return;
    }
    let items = rapidr_value::objects::with_list(name, |l| l.owner_draw_items()).unwrap_or_default();
    for (i, state, (left, top, right, bottom)) in items {
        let rect = format!("{}.ITEMRECT({i})", name.to_uppercase());
        crate::object_web::rp_create_component(&rect, "RUDT");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            crate::object_web::rp_comp_set(&rect, prop, v_int(v));
        }
        crate::object_web::rp_fire_event_args(name, "ondrawitem", &[v_int(i as i64), v_int(state), crate::value::v_str(&rect)]);
    }
    // The items were shown before OnDrawItem ran: show what it drew (or,
    // where it drew nothing, the plain item as selected now).
    render_list(name);
}

fn create_image(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("img");
    if let Ok(img) = el.clone().dyn_into::<web_sys::HtmlImageElement>() {
        if let Some(src) = props.get("picture").or_else(|| props.get("src")) {
            let path = src.to_string_val();
            let resolved = if path.starts_with("assets/") || (!path.contains("://") && !path.starts_with("data:")) {
                crate::database_web::get_rapidr_asset(&path).unwrap_or(path)
            } else {
                path
            };
            img.set_src(&resolved);
        }
    }
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("object-fit", "contain");
    let _ = el.set_attribute("draggable", "false");
    picture_mouse(&el, name);
    setup_widget(&el, id, name, props);
    render_picture(name);
}

/// A QIMAGE's mouse events (manual), as on the desktop: OnMouseDown /
/// OnMouseUp (Button, X, Y, Shift), OnMouseMove (X, Y, Shift), OnClick,
/// OnDblClick; X and Y are in the image. (`object_web::bind_dom_event`
/// leaves images to this.)
fn picture_mouse(el: &web_sys::HtmlElement, name: &str) {
    for dom_event in ["mousedown", "mouseup", "mousemove", "click", "dblclick"] {
        let owner = name.to_uppercase();
        let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let Some(target) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
            let rect = target.get_bounding_client_rect();
            let x = v_int((e.client_x() as f64 - rect.left()) as i64);
            let y = v_int((e.client_y() as f64 - rect.top()) as i64);
            let shift = v_int(i64::from(e.shift_key()) * 256 | i64::from(e.ctrl_key()) * 16 | i64::from(e.alt_key()));
            // mbLeft = 0, mbRight = 1, mbMiddle = 2 (the DOM's middle is 1).
            let button = v_int(match e.button() {
                1 => 2,
                2 => 1,
                _ => 0,
            });
            match dom_event {
                "mousedown" | "mouseup" => {
                    let event = if dom_event == "mousedown" { "onmousedown" } else { "onmouseup" };
                    crate::object_web::rp_fire_event_args(&owner, event, &[button, x, y, shift]);
                }
                "mousemove" => crate::object_web::rp_fire_event_args(&owner, "onmousemove", &[x, y, shift]),
                "click" => crate::object_web::rp_fire_event(&owner, "onclick"),
                _ => crate::object_web::rp_fire_event(&owner, "ondblclick"),
            }
        });
        let _ = el.add_event_listener_with_callback(dom_event, cb.as_ref().unchecked_ref());
        cb.forget();
    }
}

thread_local! {
    /// QIMAGEs whose picture changed since they were last shown (shown
    /// once, soon, however many drawing calls change it).
    static PICTURES_TO_RENDER: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Shows a QIMAGE's picture (rapidr_value::objects, a Bitmap, as on the
/// desktop), soon (batched).
pub fn render_picture(name: &str) {
    let name = name.to_uppercase();
    let first = PICTURES_TO_RENDER.with(|g| {
        let mut g = g.borrow_mut();
        let first = g.is_empty();
        if !g.contains(&name) {
            g.push(name);
        }
        first
    });
    if !first {
        return;
    }
    let flush = Closure::once_into_js(move || {
        for name in PICTURES_TO_RENDER.with(|g| std::mem::take(&mut *g.borrow_mut())) {
            render_picture_now(&name);
        }
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(flush.unchecked_ref(), 0);
    }
}

/// Shows a QIMAGE's picture now, as a PNG (so Transparent keeps its alpha):
/// at the top left, centered (Center) or scaled to the control (Stretch).
/// An image without a picture keeps what it shows.
fn render_picture_now(name: &str) {
    let Some(img) = get_el(&comp_id(name)).and_then(|e| e.dyn_into::<web_sys::HtmlImageElement>().ok()) else { return };
    note_display_scale();
    let Some(Some((w, h, rgba, scale))) = rapidr_value::objects::with_picture(name, |b| (!b.img.pixels.is_empty()).then(|| b.display_rgba())) else {
        return;
    };
    let (w, h) = (w as u32, h as u32);
    let Ok(data) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&rgba), w, h) else { return };
    let Some(off) = document().create_element("canvas").ok().and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else { return };
    off.set_width(w);
    off.set_height(h);
    let Some(ctx) = off.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()) else { return };
    let _ = ctx.put_image_data(&data, 0.0, 0.0);
    if let Ok(url) = off.to_data_url() {
        // (`2x`: the picture's device pixels shown in its size)
        let _ = img.set_attribute("srcset", &format!("{url} {scale}x"));
        img.set_src(&url);
    }
    let prop = |p: &str| crate::object_web::rp_comp_get_stored(name, p).to_bool();
    let (fit, position) = match (prop("stretch"), prop("center")) {
        (true, _) => ("fill", "center"),
        (false, true) => ("none", "center"),
        _ => ("none", "left top"),
    };
    let _ = img.style().set_property("object-fit", fit);
    let _ = img.style().set_property("object-position", position);
}

thread_local! {
    static CANVASES_TO_RENDER: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Shows a QCANVAS's surface (rapidr_value::objects, a Bitmap, as on the
/// desktop), soon (batched).
pub fn render_canvas(name: &str) {
    let name = name.to_uppercase();
    let first = CANVASES_TO_RENDER.with(|g| {
        let mut g = g.borrow_mut();
        let first = g.is_empty();
        if !g.contains(&name) {
            g.push(name);
        }
        first
    });
    if !first {
        return;
    }
    let flush = Closure::once_into_js(move || {
        for name in CANVASES_TO_RENDER.with(|g| std::mem::take(&mut *g.borrow_mut())) {
            render_canvas_now(&name);
        }
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(flush.unchecked_ref(), 0);
    }
}

/// Puts a QCANVAS's surface on its HTML canvas, whose pixels are the
/// control's size; a QFORM's on a canvas under its controls (made the first
/// time), whose pixels are its client area's.
fn render_canvas_now(name: &str) {
    let form = rapidr_value::objects::is_form_surface(name);
    let canvas_id = if form { format!("{}-surface", comp_id(name)) } else { comp_id(name) };
    let mut canvas = get_el(&canvas_id).and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok());
    let stored = |p: &str| crate::object_web::rp_comp_get_stored(name, p).to_i64();
    let (cw, ch) = if form {
        (crate::object_web::rp_comp_get(name, "clientwidth").to_i64(), crate::object_web::rp_comp_get(name, "clientheight").to_i64())
    } else {
        (stored("width"), stored("height"))
    };
    if canvas.is_none() && form {
        // Under the form's controls: the first child of its client area.
        let Some(client) = get_el(&format!("{}-client", comp_id(name))) else { return };
        let Some(el) = document().create_element("canvas").ok().and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else { return };
        el.set_id(&canvas_id);
        let _ = el.set_attribute("style", "position:absolute;left:0;top:0;pointer-events:none;");
        let _ = client.insert_before(&el, client.first_child().as_ref());
        canvas = Some(el);
    }
    let Some(canvas) = canvas else { return };
    note_display_scale();
    let Some((w, h, rgba, scale)) = rapidr_value::objects::with_canvas(name, cw, ch, |b| b.display_rgba()) else {
        return;
    };
    if w == 0 || h == 0 {
        return;
    }
    put_display(&canvas, w, h, &rgba, scale);
}

thread_local! {
    static HEADERS_TO_REFRESH: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Paints a QHEADER's section faces again and fires OnDrawSection (Index,
/// Pressed, Rect) for its owner-drawn ones, as on the desktop — soon
/// (batched, and never inside the CREATE that adds its sections).
pub fn refresh_header(name: &str) {
    let name = name.to_uppercase();
    let first = HEADERS_TO_REFRESH.with(|g| {
        let mut g = g.borrow_mut();
        let first = g.is_empty();
        if !g.contains(&name) {
            g.push(name);
        }
        first
    });
    if !first {
        return;
    }
    let flush = Closure::once_into_js(move || {
        for name in HEADERS_TO_REFRESH.with(|g| std::mem::take(&mut *g.borrow_mut())) {
            refresh_header_now(&name);
        }
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(flush.unchecked_ref(), 0);
    }
}

fn refresh_header_now(name: &str) {
    let stored = |p: &str| crate::object_web::rp_comp_get_stored(name, p).to_i64();
    for (i, pressed, (left, top, right, bottom)) in rapidr_value::objects::paint_header(name, stored("width"), stored("height")) {
        let rect = format!("{name}.SECTIONRECT({i})");
        crate::object_web::rp_create_component(&rect, "RUDT");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            crate::object_web::rp_comp_set(&rect, prop, v_int(v));
        }
        crate::object_web::rp_fire_event_args(name, "ondrawsection", &[v_int(i as i64), v_int(if pressed { -1 } else { 0 }), crate::value::v_str(&rect)]);
    }
    render_canvas(name);
}

/// A QHEADER's left button, as on the desktop: sections pressed, clicked
/// and resized (OnSectionClick, OnSectionTrack, OnSectionResize); a drag
/// follows the mouse outside the header; the resize cursor on an edge.
fn header_mouse_events(id: &str, name: &str) {
    use rapidr_value::input::Mouse;
    let fire = |name: &str, kind: Mouse, x: i64| {
        use rapidr_value::objects::header::{Action, TS_END};
        let before = rapidr_value::objects::with_header(name, |h| (h.pressed, h.sections.clone()));
        let actions = rapidr_value::objects::with_header(name, |h| match kind {
            Mouse::Down => h.press(x),
            Mouse::Move => h.drag_to(x),
            Mouse::Up => h.release(x),
        })
        .unwrap_or_default();
        for action in actions {
            match action {
                Action::Click(i) => crate::object_web::rp_fire_event_args(name, "onsectionclick", &[v_int(i as i64)]),
                Action::Track(i, width, state) => {
                    crate::object_web::rp_fire_event_args(name, "onsectiontrack", &[v_int(i as i64), v_int(width), v_int(state)]);
                    if state == TS_END {
                        crate::object_web::rp_fire_event_args(name, "onsectionresize", &[v_int(i as i64)]);
                    }
                }
            }
        }
        if before != rapidr_value::objects::with_header(name, |h| (h.pressed, h.sections.clone())) {
            refresh_header(name);
        }
    };
    let Some(el) = get_el(id) else { return };
    let x_in = |el: &web_sys::HtmlElement, e: &web_sys::MouseEvent| (e.client_x() as f64 - el.get_bounding_client_rect().left()) as i64;
    let (owner, target) = (name.to_uppercase(), el.clone());
    let down = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        if e.button() == 0 {
            fire(&owner, Mouse::Down, x_in(&target, &e));
        }
    });
    let _ = el.add_event_listener_with_callback("mousedown", down.as_ref().unchecked_ref());
    down.forget();
    // (on the document: a drag goes on outside the header)
    for dom_event in ["mousemove", "mouseup"] {
        let (owner, target) = (name.to_uppercase(), el.clone());
        let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let x = x_in(&target, &e);
            // (`grip`: also while an edge is dragged)
            let Some((held, grip)) = rapidr_value::objects::with_header(&owner, |h| (h.pressed.is_some(), h.on_grip(x))) else { return };
            if dom_event == "mousemove" {
                let over = e.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok()).is_some_and(|n| target.contains(Some(&n)));
                let custom = crate::object_web::rp_comp_get_stored(&owner, "cursor").to_i64() != 0;
                if !custom {
                    let _ = target.style().set_property("cursor", if grip && over { "col-resize" } else { "" });
                }
                fire(&owner, Mouse::Move, x);
            } else if held || grip {
                fire(&owner, Mouse::Up, x);
            }
        });
        let _ = document().add_event_listener_with_callback(dom_event, cb.as_ref().unchecked_ref());
        cb.forget();
    }
}

thread_local! {
    /// The last mouse position in the page (for MOUSEX / MOUSEY).
    static MOUSE_AT: std::cell::Cell<(f64, f64)> = const { std::cell::Cell::new((0.0, 0.0)) };
    static MOUSE_TRACKED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Follows the mouse over the page (once), for MOUSEX / MOUSEY.
pub fn track_mouse() {
    if MOUSE_TRACKED.with(|t| t.replace(true)) {
        return;
    }
    let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| {
        MOUSE_AT.with(|m| m.set((e.client_x() as f64, e.client_y() as f64)));
    });
    for event in ["mousemove", "mousedown"] {
        // Captured, so it's known before any handler of the click runs.
        let _ = document().add_event_listener_with_callback_and_bool(event, cb.as_ref().unchecked_ref(), true);
    }
    cb.forget();
}

/// `MOUSEX` / `MOUSEY`: the mouse relative to the client area of the form
/// it's over (else the frontmost form), as on the desktop.
pub fn mouse_in_form() -> (i64, i64) {
    let (x, y) = MOUSE_AT.with(std::cell::Cell::get);
    let doc = document();
    let form = doc
        .element_from_point(x as f32, y as f32)
        .and_then(|e| e.closest(".rr-form").ok().flatten())
        .or_else(|| {
            let forms = doc.query_selector_all(".rr-form").ok()?;
            (0..forms.length()).rev().filter_map(|i| forms.item(i)?.dyn_into::<web_sys::Element>().ok()).find(|f| {
                f.dyn_ref::<web_sys::HtmlElement>().is_some_and(|h| h.style().get_property_value("display").ok().as_deref() != Some("none"))
            })
        });
    let client = form.and_then(|f| get_el(&format!("{}-client", f.id())).map(|c| c.get_bounding_client_rect()));
    match client {
        Some(r) => ((x - r.left()) as i64, (y - r.top()) as i64),
        None => (x as i64, y as i64),
    }
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
    // Focusable by a click, for its key events (as the desktop's).
    let _ = el.set_attribute("tabindex", "-1");
    let _ = el.style().set_property("outline", "none");
    setup_widget(&el, id, name, props);
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

/// A QTREEVIEW (as on the desktop): rows for the shared model's visible
/// nodes (rapidr_value::objects::tree) — a button to expand / collapse, the
/// node's image from its Images list, its text. What the user does asks
/// the program first — OnChanging (Index, AllowChange), OnExpanding /
/// OnCollapsing (Index, Allow…) — and changes the nodes if it may, then
/// OnChange / OnExpanded / OnCollapsed; OnClick / OnDblClick follow a
/// click (a double click also expands or collapses, as Windows does).
fn create_treeview(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_class_name("rr-widget rr-tree");
    let _ = el.set_attribute("tabindex", "0");
    for (k, v) in [("border", "1px solid #999"), ("background", "white"), ("overflow", "auto"), ("font-size", "13px"), ("box-sizing", "border-box"), ("outline", "none"), ("cursor", "default"), ("user-select", "none")] {
        let _ = el.style().set_property(k, v);
    }
    // HideSelection: the selection shows only while the tree has focus.
    for dom_event in ["focus", "blur"] {
        let owner = name.to_uppercase();
        let cb = Closure::<dyn FnMut()>::new(move || {
            if rapidr_value::objects::with_tree(&owner, |m| m.hide_selection).unwrap_or(false) {
                render_tree(&owner);
            }
        });
        let _ = el.add_event_listener_with_callback(dom_event, cb.as_ref().unchecked_ref());
        cb.forget();
    }
    for dom_event in ["click", "dblclick"] {
        let owner = name.to_uppercase();
        let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let Some(target) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
            // (clicks in the node editor are its own)
            if target.class_name().contains("rr-tree-editor") {
                return;
            }
            let Some(row) = target.closest("[data-node]").ok().flatten() else { return };
            let Some(i) = row.get_attribute("data-node").and_then(|v| v.parse::<usize>().ok()) else { return };
            let on_button = target.closest(".rr-tree-button").ok().flatten().is_some();
            let click = TREE_CLICKS.with(|c| {
                c.set(c.get() + 1);
                c.get()
            });
            if dom_event == "dblclick" {
                crate::object_web::rp_fire_event(&owner, "ondblclick");
                tree_toggle(&owner, i);
                return;
            }
            let was_selected = rapidr_value::objects::with_tree(&owner, |m| m.item_index == i as i64).unwrap_or(false);
            crate::object_web::rp_fire_event(&owner, "onclick");
            if on_button {
                tree_toggle(&owner, i);
            } else if was_selected && target.closest(".rr-tree-text").ok().flatten().is_some() {
                // A click on the selected node's text: an edit, a moment
                // later, unless a double click comes.
                let tree = owner.clone();
                let later = Closure::once_into_js(move || {
                    if TREE_CLICKS.with(|c| c.get()) == click {
                        tree_begin_edit(&tree, i);
                    }
                });
                if let Some(window) = web_sys::window() {
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), 500);
                }
            } else {
                tree_user_select(&owner, i);
            }
        });
        let _ = el.add_event_listener_with_callback(dom_event, cb.as_ref().unchecked_ref());
        cb.forget();
    }
    let key_owner = name.to_uppercase();
    let key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        // (keys in the node editor are its own)
        if e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).is_some_and(|t| t.class_name().contains("rr-tree-editor")) {
            return;
        }
        if e.key() == "F2" {
            e.prevent_default();
            if let Some(i) = rapidr_value::objects::with_tree(&key_owner, |m| usize::try_from(m.item_index).ok()).flatten() {
                tree_begin_edit(&key_owner, i);
            }
            return;
        }
        let Some((rows, current, expanded, has_children, parent)) = rapidr_value::objects::with_tree(&key_owner, |m| {
            let cur = usize::try_from(m.item_index).ok().filter(|&i| i < m.nodes.len());
            (m.visible_rows(), m.item_index, cur.is_some_and(|i| m.nodes[i].expanded), cur.is_some_and(|i| m.has_children(i)), cur.and_then(|i| m.parent(i)))
        }) else {
            return;
        };
        let at = rows.iter().position(|&r| r as i64 == current);
        let target = match (e.key().as_str(), at) {
            ("ArrowDown", Some(p)) => rows.get(p + 1).copied(),
            ("ArrowDown", None) => rows.first().copied(),
            ("ArrowUp", Some(p)) => p.checked_sub(1).and_then(|p| rows.get(p)).copied(),
            ("ArrowRight", Some(_)) if has_children && !expanded => {
                e.prevent_default();
                tree_toggle(&key_owner, current as usize);
                return;
            }
            ("ArrowLeft", Some(_)) if expanded => {
                e.prevent_default();
                tree_toggle(&key_owner, current as usize);
                return;
            }
            ("ArrowLeft", Some(_)) => parent,
            _ => return,
        };
        e.prevent_default();
        if let Some(i) = target {
            tree_user_select(&key_owner, i);
        }
    });
    let _ = el.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref());
    key.forget();
    setup_widget(&el, id, name, props);
    render_tree(name);
}

thread_local! {
    /// Bumped by every click on a tree (a double click cancels the edit
    /// a click on the selected node starts).
    static TREE_CLICKS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// The node each tree's editor edits.
    static TREE_EDITING: std::cell::RefCell<HashMap<String, usize>> = std::cell::RefCell::new(HashMap::new());
}

/// The user starts editing node `i`'s text (F2, or a click on the selected
/// node): not in a ReadOnly tree; OnEditing(Index, AllowEdit) may refuse;
/// then an input over the node's text — Enter or leaving it keeps the
/// edit, Escape drops it.
fn tree_begin_edit(name: &str, i: usize) {
    if rapidr_value::objects::with_tree(name, |m| m.read_only || i >= m.nodes.len()).unwrap_or(true) {
        return;
    }
    let tree = name.to_uppercase();
    crate::object_web::rp_fire_event_then(name, "onediting", &[v_int(i as i64), v_int(-1)], move |a| {
        if a[1].to_i64() == 0 {
            return;
        }
        let Some(el) = get_el(&comp_id(&tree)) else { return };
        let Some(label) = el.query_selector(&format!("[data-node=\"{i}\"] .rr-tree-text")).ok().flatten() else { return };
        let Some(text) = rapidr_value::objects::with_tree(&tree, |m| m.nodes.get(i).map(|n| n.text.clone())).flatten() else { return };
        let Some(input) = document().create_element("input").ok().and_then(|x| x.dyn_into::<web_sys::HtmlInputElement>().ok()) else { return };
        input.set_class_name("rr-tree-editor");
        input.set_value(&text);
        for (k, v) in [("font", "inherit"), ("padding", "0 2px"), ("border", "1px solid #333"), ("height", "16px"), ("box-sizing", "border-box"), ("min-width", "60px")] {
            let _ = input.style().set_property(k, v);
        }
        let _ = input.style().set_property("width", &format!("{}px", (label.get_bounding_client_rect().width() as i64 + 30).max(60)));
        let _ = label.set_attribute("style", &format!("{};display:none", label.get_attribute("style").unwrap_or_default()));
        if let Some(row) = label.parent_node() {
            let _ = row.insert_before(&input, label.next_sibling().as_ref());
        }
        TREE_EDITING.with(|t| t.borrow_mut().insert(tree.clone(), i));
        let key_tree = tree.clone();
        let key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| match e.key().as_str() {
            "Enter" => {
                e.prevent_default();
                tree_end_edit(&key_tree, true);
            }
            "Escape" => {
                e.prevent_default();
                tree_end_edit(&key_tree, false);
            }
            _ => {}
        });
        let _ = input.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref());
        key.forget();
        let blur_tree = tree.clone();
        let blur = Closure::<dyn FnMut()>::new(move || tree_end_edit(&blur_tree, true));
        let _ = input.add_event_listener_with_callback("blur", blur.as_ref().unchecked_ref());
        blur.forget();
        let _ = input.focus();
        input.select();
    });
}

/// Ends an edit: with `keep`, OnEdited(Index, S) — S the text, which the
/// program may change — and the node gets it.
fn tree_end_edit(name: &str, keep: bool) {
    let tree = name.to_uppercase();
    let Some(i) = TREE_EDITING.with(|t| t.borrow_mut().remove(&tree)) else { return };
    let el = get_el(&comp_id(&tree));
    let text = el
        .as_ref()
        .and_then(|el| el.query_selector(".rr-tree-editor").ok().flatten())
        .and_then(|x| x.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|x| x.value())
        .unwrap_or_default();
    // (the tree drawn again drops the editor; focus back on the tree)
    render_tree(&tree);
    if let Some(el) = el.and_then(|x| x.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = el.focus();
    }
    if !keep {
        return;
    }
    crate::object_web::rp_fire_event_then(&tree.clone(), "onedited", &[v_int(i as i64), v_str(&text)], move |a| {
        let text = a[1].to_string_val();
        rapidr_value::objects::with_tree(&tree, |m| m.set_text(i, text));
        render_tree(&tree);
    });
}

/// The user picked node `i`: OnChanging may refuse; then OnChange.
fn tree_user_select(name: &str, i: usize) {
    if rapidr_value::objects::with_tree(name, |m| m.item_index) == Some(i as i64) {
        return;
    }
    let tree = name.to_uppercase();
    crate::object_web::rp_fire_event_then(name, "onchanging", &[v_int(i as i64), v_int(-1)], move |a| {
        if a[1].to_i64() != 0 {
            rapidr_value::objects::with_tree(&tree, |m| m.select(i as i64));
            render_tree(&tree);
            crate::object_web::rp_fire_event_1(&tree, "onchange", v_int(i as i64));
        }
    });
}

/// The user expands or collapses node `i`: OnExpanding / OnCollapsing may
/// refuse; then OnExpanded / OnCollapsed.
fn tree_toggle(name: &str, i: usize) {
    let Some((open, has)) = rapidr_value::objects::with_tree(name, |m| (!m.nodes[i].expanded, m.has_children(i))) else { return };
    if !has {
        return;
    }
    let tree = name.to_uppercase();
    crate::object_web::rp_fire_event_then(name, if open { "onexpanding" } else { "oncollapsing" }, &[v_int(i as i64), v_int(-1)], move |a| {
        if a[1].to_i64() != 0 {
            rapidr_value::objects::with_tree(&tree, |m| m.set_expanded(i, open, false));
            render_tree(&tree);
            crate::object_web::rp_fire_event_1(&tree, if open { "onexpanded" } else { "oncollapsed" }, v_int(i as i64));
        }
    });
}

/// Shows a tree's visible nodes again (and fires OnDeletion for nodes the
/// program deleted).
thread_local! {
    /// Trees asking their program for icons (OnGetImageIndex): what that
    /// changes shows without asking again.
    static TREES_ASKING: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(std::collections::HashSet::new());
    /// What each tree showed when it last asked (`TreeView::view_hash`).
    static TREES_ASKED: std::cell::RefCell<HashMap<String, u64>> = std::cell::RefCell::new(HashMap::new());
}

/// OnGetImageIndex (Index) for each shown node, OnGetSelectedIndex (Index)
/// for the selected one, as on the desktop — soon (never inside the
/// program's own statement that changed the tree), then shown again.
fn tree_ask_images(name: &str) {
    let ask = |e: &str| crate::object_web::rp_has_handler(name, e);
    let key = name.to_uppercase();
    let Some(view) = rapidr_value::objects::with_tree(name, |m| m.view_hash()) else { return };
    if !(ask("ongetimageindex") || ask("ongetselectedindex")) || TREES_ASKED.with(|a| a.borrow().get(&key) == Some(&view)) {
        return;
    }
    if !TREES_ASKING.with(|a| a.borrow_mut().insert(key.clone())) {
        return;
    }
    TREES_ASKED.with(|a| a.borrow_mut().insert(key.clone(), view));
    let later = Closure::once_into_js(move || {
        let (rows, selected) = rapidr_value::objects::with_tree(&key, |m| (m.visible_rows(), m.item_index)).unwrap_or_default();
        for i in rows {
            let event = if i as i64 == selected { "ongetselectedindex" } else { "ongetimageindex" };
            crate::object_web::rp_fire_event_1(&key, event, v_int(i as i64));
        }
        render_tree(&key);
        TREES_ASKING.with(|a| a.borrow_mut().remove(&key));
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), 0);
    }
}

pub fn render_tree(name: &str) {
    note_display_scale();
    tree_ask_images(name);
    for i in rapidr_value::objects::with_tree(name, |m| m.take_deleted()).unwrap_or_default() {
        crate::object_web::rp_fire_event_1(name, "ondeletion", v_int(i as i64));
    }
    let Some(el) = get_el(&comp_id(name)) else { return };
    let images = crate::object_web::rp_comp_get_stored(name, "images").to_string_val();
    let state_images = crate::object_web::rp_comp_get_stored(name, "stateimages").to_string_val();
    // (HideSelection: none shown while the tree hasn't focus)
    let focused = document().active_element().is_some_and(|a| a == *el);
    let Some(rows) = rapidr_value::objects::with_tree(name, |m| {
        let hidden = m.hide_selection && !focused;
        m.visible_rows()
            .into_iter()
            .map(|i| {
                let n = &m.nodes[i];
                let selected = m.item_index == i as i64;
                let image = if selected { n.selected_index } else { n.image_index };
                (i, n.level, n.text.clone(), m.has_children(i), n.expanded, selected && !hidden, (image, n.state_index))
            })
            .collect::<Vec<_>>()
    }) else {
        return;
    };
    let (indent, buttons) = rapidr_value::objects::with_tree(name, |m| (m.indent, m.show_buttons)).unwrap_or((19, true));
    let scroll = el.scroll_top();
    el.set_inner_html("");
    for (i, level, text, has, expanded, selected, image) in rows {
        let row = create_el("div");
        let _ = row.set_attribute("data-node", &i.to_string());
        let st = row.style();
        for (k, v) in [("display", "flex"), ("align-items", "center"), ("height", "18px"), ("white-space", "nowrap")] {
            let _ = st.set_property(k, v);
        }
        let _ = st.set_property("padding-left", &format!("{}px", 2 + level as i64 * indent));
        let button = create_el("span");
        button.set_class_name("rr-tree-button");
        let _ = button.style().set_property("width", "14px");
        let _ = button.style().set_property("display", "inline-block");
        if has && buttons {
            button.set_text_content(Some(if expanded { "▾" } else { "▸" }));
        }
        let _ = row.append_child(&button);
        if let Some((w, h, rgba, scale)) = rapidr_value::objects::tree_icon(&images, &state_images, image.0, image.1) {
            if let Some(canvas) = document().create_element("canvas").ok().and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok()) {
                let _ = canvas.style().set_property("margin-right", "3px");
                put_display(&canvas, w, h, &rgba, scale);
                let _ = row.append_child(&canvas);
            }
        }
        let label = create_el("span");
        label.set_class_name("rr-tree-text");
        label.set_text_content(Some(&text));
        let _ = label.style().set_property("padding", "0 2px");
        if selected {
            let _ = label.style().set_property("background", "#0078d7");
            let _ = label.style().set_property("color", "white");
        }
        let _ = row.append_child(&label);
        let _ = el.append_child(&row);
    }
    el.set_scroll_top(scroll);
}

fn create_mainmenu(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("nav");
    let _ = el.style().set_property("display", "flex");
    let _ = el.style().set_property("background", "#f0f0f0");
    let _ = el.style().set_property("border-bottom", "1px solid #ccc");
    let _ = el.style().set_property("font-size", "13px");
    el.set_id(id);
    let _ = el.set_attribute("data-rr-name", name);
    let _ = el.set_attribute("data-rr-type", "RMAINMENU");
    let style = el.style();
    let _ = style.set_property("position", "absolute");
    let _ = style.set_property("top", "29px");
    let _ = style.set_property("left", "0");
    let _ = style.set_property("width", "100%");
    let _ = style.set_property("height", "28px");
    let _ = style.set_property("box-sizing", "border-box");
    let _ = style.set_property("z-index", "100");

    let parent = props.get("parent").map(|v| v.to_string_val());
    if let Some(ref p) = parent {
        let parent_id = comp_id(p);
        if let (Some(form_el), Some(client_el)) = (get_el(&parent_id), get_el(&format!("{}-client", parent_id))) {
            let _ = form_el.insert_before(&el, Some(&client_el));
            let bs = crate::object_web::rp_comp_get_stored(p, "borderstyle");
            place_form_chrome(&form_el, if matches!(bs, Value::Null) { 2 } else { bs.to_i64() }, true);
        } else {
            let _ = get_parent_client(&parent).append_child(&el);
        }
    } else {
        let _ = get_parent_client(&parent).append_child(&el);
    }
}

fn create_menuitem(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    let caption = props.get("caption").map(|v| v.to_string_val()).unwrap_or_default();
    el.set_inner_text(&strip_ampersands(&caption));
    let _ = el.set_attribute("data-rr-name", name);
    let _ = el.set_attribute("data-rr-type", "RMENUITEM");
    el.set_id(id);

    // Append to body first so it exists in DOM for get_el lookup
    let _ = document().body().unwrap().append_child(&el);

    let parent = props.get("parent").map(|v| v.to_string_val()).unwrap_or_default();
    if !parent.is_empty() {
        gui_web_set_parent(name, &parent);
    }
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
    legend.set_inner_text(&strip_ampersands(&caption));
    let _ = el.append_child(&legend);
    setup_widget(&el, id, name, props);
}

fn create_statusbar(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    let _ = el.style().set_property("background", "#e8e8e8");
    let _ = el.style().set_property("border-top", "1px solid #ccc");
    let _ = el.style().set_property("font-size", "13px");
    let _ = el.style().set_property("padding", "2px 1px");
    el.set_id(id);
    let _ = el.set_attribute("data-rr-name", name);
    let style = el.style();
    let _ = style.set_property("position", "absolute");
    // Docked by its Align (alBottom by default): layout_web.
    apply_geometry(&el, props, 0, 0, 200, 24);
    let _ = style.set_property("display", "flex");
    let _ = style.set_property("box-sizing", "border-box");
    let parent = props.get("parent").map(|v| v.to_string_val());
    let _ = get_parent_client(&parent).append_child(&el);
    render_statusbar(name);
}

/// A QSTATUSBAR's panels left to right (`Panel(i).Width` px, 100 by default;
/// the last one takes the rest), or its SimpleText when `SimplePanel` is
/// set or it has no panels — as the desktop runtime draws it. Captions are
/// plain text, never markup.
pub fn render_statusbar(name: &str) {
    use crate::object_web::rp_comp_get_stored as get;
    let Some(el) = get_el(&comp_id(name)) else { return };
    el.set_inner_text("");
    let count = get(name, "panelcount").to_i64().clamp(0, 256);
    let panel = |text: &str, width: Option<i64>| {
        let span = create_el("span");
        span.set_text_content(Some(text));
        let st = span.style();
        let _ = st.set_property("overflow", "hidden");
        let _ = st.set_property("white-space", "nowrap");
        let _ = st.set_property("padding", "0 4px");
        let _ = st.set_property("border", "1px inset #ccc");
        let _ = st.set_property("line-height", "18px");
        match width {
            Some(w) => {
                let _ = st.set_property("flex", &format!("0 0 {w}px"));
                let _ = st.set_property("box-sizing", "border-box");
            }
            None => {
                let _ = st.set_property("flex", "1 1 auto");
            }
        }
        let _ = el.append_child(&span);
    };
    if count == 0 || get(name, "simplepanel").to_bool() {
        panel(&get(name, "simpletext").to_string_val(), None);
        return;
    }
    for i in 0..count {
        let caption = get(name, &format!("panel({i}).caption")).to_string_val();
        let width = get(name, &format!("panel({i}).width")).to_i64();
        let width = if i == count - 1 { None } else if width > 0 { Some(width.min(10_000)) } else { Some(100) };
        panel(&caption, width);
    }
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

/// QSPLITTER: dragging it resizes the control next to it
/// (layout_web::splitter_begin / _move / _end, as on the desktop).
fn create_splitter(id: &str, name: &str, props: &HashMap<String, Value>) {
    let el = create_el("div");
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("background", "#ccc");
    let owner = name.to_uppercase();
    let vertical = move |n: &str| matches!(crate::object_web::rp_comp_get_stored(n, "align").to_i64(), 1 | 2);
    // The cursor follows its Align.
    {
        let owner = owner.clone();
        let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            if let Some(el) = e.current_target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) {
                let _ = el.style().set_property("cursor", if vertical(&owner) { "row-resize" } else { "col-resize" });
            }
        });
        let _ = el.add_event_listener_with_callback("mouseenter", cb.as_ref().unchecked_ref());
        cb.forget();
    }
    let down = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        if !crate::layout_web::splitter_begin(&owner) {
            return;
        }
        e.prevent_default();
        let v = vertical(&owner);
        let start = if v { e.client_y() } else { e.client_x() };
        let move_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let now = if v { e.client_y() } else { e.client_x() };
            crate::layout_web::splitter_move((now - start) as i64);
        });
        let doc = document();
        let _ = doc.add_event_listener_with_callback("mousemove", move_cb.as_ref().unchecked_ref());
        let move_ref: JsValue = move_cb.as_ref().into();
        let up_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::once(move |_e: web_sys::MouseEvent| {
            let _ = document().remove_event_listener_with_callback("mousemove", move_ref.unchecked_ref());
            drop(move_cb);
            crate::layout_web::splitter_end();
        });
        let options = web_sys::AddEventListenerOptions::new();
        options.set_once(true);
        let _ = doc.add_event_listener_with_callback_and_add_event_listener_options("mouseup", up_cb.as_ref().unchecked_ref(), &options);
        up_cb.forget();
    });
    let _ = el.add_event_listener_with_callback("mousedown", down.as_ref().unchecked_ref());
    down.forget();
    setup_widget(&el, id, name, props);
}

fn create_listview(id: &str, name: &str, props: &HashMap<String, Value>) {
    // A table drawn from the shared QLISTVIEW data (rapidr_value::objects::
    // listview), like the desktop runtime's browser.
    let wrapper = create_el("div");
    wrapper.set_class_name("rr-widget");
    let _ = wrapper.style().set_property("overflow", "auto");
    let _ = wrapper.style().set_property("border", "1px solid #aaa");
    let _ = wrapper.style().set_property("background", "white");
    let table = create_el("table");
    table.set_id(&format!("{}-table", id));
    table.set_class_name("rr-grid");
    let _ = table.style().set_property("border-collapse", "collapse");
    let _ = table.style().set_property("table-layout", "fixed");
    let _ = table.style().set_property("font-size", "13px");
    let _ = wrapper.append_child(&table);
    // Row click: ItemIndex, then OnClick / OnDblClick; header click:
    // OnColumnClick(Column%).
    for dom_event in ["click", "dblclick"] {
        let owner = name.to_uppercase();
        let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let Some(target) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
            if let Ok(Some(th)) = target.closest("th") {
                if dom_event == "click" {
                    let col = th.get_attribute("data-col").and_then(|c| c.parse::<i64>().ok()).unwrap_or(0);
                    crate::object_web::rp_fire_event_1(&owner, "oncolumnclick", v_int(col));
                }
                return;
            }
            let Ok(Some(tr)) = target.closest("tr") else { return };
            let Some(row) = tr.get_attribute("data-row").and_then(|r| r.parse::<i64>().ok()) else { return };
            rapidr_value::objects::set(&owner, "itemindex", &v_int(row));
            render_listview(&owner);
            crate::object_web::rp_fire_event(&owner, if dom_event == "click" { "onclick" } else { "ondblclick" });
        });
        let _ = wrapper.add_event_listener_with_callback(dom_event, cb.as_ref().unchecked_ref());
        cb.forget();
    }
    setup_widget(&wrapper, id, name, props);
    render_listview(name);
}

/// A QSTRINGGRID: a table drawn from the shared grid data
/// (rapidr_value::objects::grid), like the desktop runtime's. Clicking a
/// cell selects it (OnSelectCell, OnClick); double-clicking fires
/// OnDblClick and, with goEditing, edits the cell in place (every click with
/// goAlwaysShowEditor; also Enter, F2 or typing). Enter or leaving the cell
/// stores it (OnSetEditText, then OnChange); Escape drops the edit. An
/// ellipsis column (or a "..." cell) has a button: OnEllipsisClick(Col, Row)
/// and OnDblClick.
fn create_grid(id: &str, name: &str, props: &HashMap<String, Value>) {
    let wrapper = create_el("div");
    wrapper.set_class_name("rr-widget");
    let _ = wrapper.set_attribute("tabindex", "0");
    let st = wrapper.style();
    let _ = st.set_property("overflow", "auto");
    let _ = st.set_property("border", "1px solid #aaa");
    let _ = st.set_property("background", "white");
    let _ = st.set_property("outline", "none");
    let table = create_el("table");
    table.set_id(&format!("{}-table", id));
    table.set_class_name("rr-grid");
    let ts = table.style();
    let _ = ts.set_property("border-collapse", "collapse");
    let _ = ts.set_property("table-layout", "fixed");
    let _ = ts.set_property("font-size", "13px");
    // Dragging selects cells, not text.
    let _ = ts.set_property("user-select", "none");
    let _ = wrapper.append_child(&table);

    let owner = name.to_uppercase();
    let click_owner = owner.clone();
    let click = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        let Some(target) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some((c, r)) = grid_target_cell(&target) else { return };
        if target.closest("[contenteditable=true]").ok().flatten().is_some() {
            return; // clicks inside the cell being edited
        }
        // The end of a drag that selected a range, not a click on a cell.
        if GRID_DRAG.with(|d| d.borrow_mut().take()).is_some_and(|(_, _, moved)| moved) {
            return;
        }
        if e.shift_key() && grid_extend(&click_owner, c, r) {
            return;
        }
        if let Ok(Some(_)) = target.closest(".rr-grid-list") {
            if let Ok(Some(td)) = target.closest("td") {
                grid_drop_down(&click_owner, &td);
            }
            return;
        }
        grid_select(&click_owner, c, r);
        if target.closest(".rr-grid-ellipsis").ok().flatten().is_some() {
            crate::object_web::rp_fire_event_2(&click_owner, "onellipsisclick", v_int(c), v_int(r));
            crate::object_web::rp_fire_event(&click_owner, "ondblclick");
            return;
        }
        crate::object_web::rp_fire_event(&click_owner, "onclick");
        if grid_option(&click_owner, rapidr_value::objects::grid::GO_ALWAYS_SHOW_EDITOR) {
            grid_start_edit(&click_owner, None);
        }
    });
    let _ = wrapper.add_event_listener_with_callback("click", click.as_ref().unchecked_ref());
    click.forget();

    // Dragging over cells selects a range (goRangeSelect).
    let down_owner = owner.clone();
    let down = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        let Some(target) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        // On a header's border: resizing (goColSizing / goRowSizing).
        if let Some((col, index, start)) = grid_size_edge(&down_owner, &target, &e) {
            e.prevent_default();
            let size = rapidr_value::objects::with_grid(&down_owner, |g| if col { g.col_widths[index] } else { g.row_heights[index] }).unwrap_or(0);
            GRID_SIZING.with(|s| *s.borrow_mut() = Some((down_owner.clone(), col, index, start, size)));
            // (the click that ends it doesn't select a cell)
            GRID_DRAG.with(|d| *d.borrow_mut() = Some((down_owner.clone(), (-1, -1), true)));
            track_grid_sizing();
            return;
        }
        let cell = grid_target_cell(&target).filter(|_| e.button() == 0 && !e.shift_key());
        GRID_DRAG.with(|d| *d.borrow_mut() = cell.map(|c| (down_owner.clone(), c, false)));
    });
    let _ = wrapper.add_event_listener_with_callback("mousedown", down.as_ref().unchecked_ref());
    down.forget();
    let move_owner = owner.clone();
    let drag = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        if e.buttons() & 1 == 0 {
            // Over a header's border: the resize cursor.
            if let (Some(target), Some(wrapper)) = (
                e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()),
                e.current_target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()),
            ) {
                let cursor = match grid_size_edge(&move_owner, &target, &e) {
                    Some((true, _, _)) => "col-resize",
                    Some((false, _, _)) => "row-resize",
                    None => "",
                };
                let _ = wrapper.style().set_property("cursor", cursor);
            }
            return;
        }
        let Some(target) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some((c, r)) = grid_target_cell(&target) else { return };
        let Some((owner, start, moved)) = GRID_DRAG.with(|d| d.borrow().clone()) else { return };
        if owner != move_owner || (!moved && (c, r) == start) || !grid_option(&owner, rapidr_value::objects::grid::GO_RANGE_SELECT) {
            return;
        }
        e.prevent_default();
        if !moved {
            grid_select(&owner, start.0, start.1);
        }
        GRID_DRAG.with(|d| *d.borrow_mut() = Some((owner.clone(), start, true)));
        grid_extend(&owner, c, r);
    });
    let _ = wrapper.add_event_listener_with_callback("mousemove", drag.as_ref().unchecked_ref());
    drag.forget();

    let dbl_owner = owner.clone();
    // The first click redrew the table, so this acts on the cell it
    // selected rather than on the event's target.
    let dbl = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        let Some(target) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        if target.closest(".rr-grid-ellipsis").ok().flatten().is_some() || target.closest("[contenteditable=true]").ok().flatten().is_some() {
            return;
        }
        crate::object_web::rp_fire_event(&dbl_owner, "ondblclick");
        grid_start_edit(&dbl_owner, None);
    });
    let _ = wrapper.add_event_listener_with_callback("dblclick", dbl.as_ref().unchecked_ref());
    dbl.forget();

    let key_owner = owner.clone();
    let keys = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        let editing = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).is_some_and(|t| t.get_attribute("contenteditable").as_deref() == Some("true"));
        if editing {
            match e.key().as_str() {
                "Enter" => {
                    e.prevent_default();
                    grid_finish_edit(&key_owner, true);
                }
                "Escape" => {
                    e.prevent_default();
                    grid_finish_edit(&key_owner, false);
                }
                _ => {}
            }
            return;
        }
        let Some((c, r)) = rapidr_value::objects::with_grid(&key_owner, |g| (g.col, g.row)) else { return };
        let moved = match e.key().as_str() {
            "ArrowUp" => Some((c, r - 1)),
            "ArrowDown" => Some((c, r + 1)),
            "ArrowLeft" => Some((c - 1, r)),
            "ArrowRight" => Some((c + 1, r)),
            _ => None,
        };
        if let Some((nc, nr)) = moved {
            e.prevent_default();
            if !(e.shift_key() && grid_extend(&key_owner, nc, nr)) {
                grid_select(&key_owner, nc, nr);
            }
            return;
        }
        let key = e.key();
        if key == "Enter" || key == "F2" {
            e.prevent_default();
            grid_start_edit(&key_owner, None);
        } else if key.chars().count() == 1 && !e.ctrl_key() && !e.meta_key() && !e.alt_key() {
            e.prevent_default();
            grid_start_edit(&key_owner, Some(key));
        }
    });
    let _ = wrapper.add_event_listener_with_callback("keydown", keys.as_ref().unchecked_ref());
    keys.forget();

    // Leaving an edited cell stores it.
    let blur_owner = owner;
    let blur = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
        let editing = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).is_some_and(|t| t.get_attribute("contenteditable").as_deref() == Some("true"));
        if editing {
            grid_finish_edit(&blur_owner, true);
        }
    });
    let _ = wrapper.add_event_listener_with_callback("focusout", blur.as_ref().unchecked_ref());
    blur.forget();

    setup_widget(&wrapper, id, name, props);
    render_grid(name);
}

/// The grid cell (col, row) a clicked element is in.
fn grid_target_cell(target: &web_sys::Element) -> Option<(i64, i64)> {
    let td = target.closest("td").ok()??;
    let c = td.get_attribute("data-col")?.parse().ok()?;
    let r = td.get_attribute("data-row")?.parse().ok()?;
    Some((c, r))
}

fn grid_option(name: &str, option: u32) -> bool {
    rapidr_value::objects::with_grid(name, |g| g.has_option(option)).unwrap_or(false)
}

/// Selects a cell as a click or an arrow key does (fixed cells can't be
/// selected).
fn grid_select(name: &str, c: i64, r: i64) {
    grid_user_select(name, c, r, false);
}

/// A selection the user made (`StringGrid::user_select`, as on the
/// desktop): when it moves, OnSelectCell(Col, Row, CanSelect) is fired, and
/// `CanSelect = 0` puts the selection back.
fn grid_user_select(name: &str, c: i64, r: i64, extend: bool) -> bool {
    let Some(before) = rapidr_value::objects::with_grid_mut(name, |g| g.user_select(c, r, extend)).flatten() else {
        return false;
    };
    render_grid_now(name);
    let grid = name.to_string();
    crate::object_web::rp_fire_event_then(name, "onselectcell", &[v_int(c), v_int(r), v_int(-1)], move |a| {
        if a[2].to_i64() == 0 {
            rapidr_value::objects::with_grid_mut(&grid, |g| g.set_selection(before));
            render_grid_now(&grid);
        }
    });
    true
}

/// Whether the mouse is on the border of a header cell that can be dragged
/// to resize: goColSizing and the right border of a fixed row's cell (a
/// column), or goRowSizing and the bottom border of a fixed column's cell
/// (a row). Returns (column?, index, mouse position along it).
fn grid_size_edge(name: &str, target: &web_sys::Element, e: &web_sys::MouseEvent) -> Option<(bool, usize, f64)> {
    use rapidr_value::objects::grid::{GO_COL_SIZING, GO_ROW_SIZING};
    let td = target.closest("td").ok()??;
    let (c, r) = grid_target_cell(&td)?;
    let rect = td.get_bounding_client_rect();
    let (x, y) = (e.client_x() as f64, e.client_y() as f64);
    rapidr_value::objects::with_grid(name, |g| {
        if g.has_option(GO_COL_SIZING) && (r as usize) < g.fixed_rows() && x >= rect.right() - 5.0 {
            Some((true, c as usize, x))
        } else if g.has_option(GO_ROW_SIZING) && (c as usize) < g.fixed_cols() && y >= rect.bottom() - 5.0 {
            Some((false, r as usize, y))
        } else {
            None
        }
    })?
}

thread_local! {
    /// A header border being dragged: the grid, column (else row), its
    /// index, where the drag started and the size then.
    static GRID_SIZING: std::cell::RefCell<Option<(String, bool, usize, f64, i64)>> = const { std::cell::RefCell::new(None) };
    static GRID_SIZING_TRACKED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Follows a header border drag over the whole page (once): the column's
/// width / row's height follows the mouse until the button is released.
fn track_grid_sizing() {
    if GRID_SIZING_TRACKED.with(|t| t.replace(true)) {
        return;
    }
    let moved = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| {
        let Some((name, col, index, start, size)) = GRID_SIZING.with(|s| s.borrow().clone()) else { return };
        let at = if col { e.client_x() as f64 } else { e.client_y() as f64 };
        let new = (size + (at - start).round() as i64).clamp(0, 10_000);
        rapidr_value::objects::with_grid_mut(&name, |g| {
            let sizes = if col { &mut g.col_widths } else { &mut g.row_heights };
            if let Some(s) = sizes.get_mut(index) {
                *s = new;
            }
        });
        render_grid(&name);
    });
    let _ = document().add_event_listener_with_callback("mousemove", moved.as_ref().unchecked_ref());
    moved.forget();
    let up = Closure::<dyn FnMut()>::new(|| {
        GRID_SIZING.with(|s| s.borrow_mut().take());
    });
    let _ = document().add_event_listener_with_callback("mouseup", up.as_ref().unchecked_ref());
    up.forget();
}

thread_local! {
    /// A mouse button held on a grid: the grid, the cell it went down on,
    /// and whether it has dragged to another cell since.
    static GRID_DRAG: std::cell::RefCell<Option<(String, (i64, i64), bool)>> = const { std::cell::RefCell::new(None) };
}

/// The user dragged or shift-clicked to a cell: with goRangeSelect the
/// range grows to it (rapidr_value::objects::grid, as on the desktop).
/// Returns whether the selection changed.
fn grid_extend(name: &str, c: i64, r: i64) -> bool {
    grid_user_select(name, c, r, true)
}

fn grid_cell_el(name: &str, c: i64, r: i64) -> Option<web_sys::HtmlElement> {
    let table = get_el(&format!("{}-table", comp_id(name)))?;
    table.query_selector(&format!("td[data-col=\"{c}\"][data-row=\"{r}\"] .rr-grid-text")).ok()??.dyn_into::<web_sys::HtmlElement>().ok()
}

/// Makes the selected cell editable (with `initial` text, or its own).
fn grid_start_edit(name: &str, initial: Option<String>) {
    let Some((c, r, editable)) = rapidr_value::objects::with_grid(name, |g| (g.col, g.row, g.editable())) else { return };
    if !editable || c < 0 || r < 0 {
        return;
    }
    let Some(el) = grid_cell_el(name, c, r) else { return };
    if let Some(text) = initial {
        el.set_text_content(Some(&text));
    }
    let _ = el.set_attribute("contenteditable", "true");
    let _ = el.set_attribute("spellcheck", "false");
    let st = el.style();
    let _ = st.set_property("background", "white");
    let _ = st.set_property("color", "black");
    let _ = st.set_property("outline", "1px solid #0078d7");
    let _ = st.set_property("display", "block");
    let _ = el.focus();
    // Caret at the end.
    if let (Some(win), Some(doc)) = (web_sys::window(), web_sys::window().and_then(|w| w.document())) {
        if let (Ok(Some(sel)), Ok(range)) = (win.get_selection(), doc.create_range()) {
            let _ = range.select_node_contents(&el);
            range.collapse_with_to_start(false);
            let _ = sel.remove_all_ranges();
            let _ = sel.add_range(&range);
        }
    }
}

/// Ends an edit: stores the text (`keep`) and fires OnSetEditText(Col,
/// Row, Value$) and OnChange.
fn grid_finish_edit(name: &str, keep: bool) {
    let Some((c, r)) = rapidr_value::objects::with_grid(name, |g| (g.col, g.row)) else { return };
    let Some(el) = grid_cell_el(name, c, r) else { return };
    if el.get_attribute("contenteditable").as_deref() != Some("true") {
        return;
    }
    let _ = el.remove_attribute("contenteditable");
    let value = el.text_content().unwrap_or_default();
    if keep {
        grid_store(name, value);
    }
    render_grid_now(name);
    if let Some(wrapper) = get_el(&comp_id(name)).and_then(|w| w.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = wrapper.focus();
    }
}

/// The user entered `value` in the selected cell (edited it, or picked it
/// from a gcsList column's drop-down): stored, then OnSetEditText(Col, Row,
/// Value) and RapidR's OnChange, if it changed (as on the desktop).
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
        render_grid_now(name);
        crate::object_web::rp_fire_event_args(name, "onsetedittext", &[v_int(c), v_int(r), v_str(&value)]);
        crate::object_web::rp_fire_event(name, "onchange");
    }
}

/// A gcsList column's drop-down: its ColumnList under the selected cell;
/// picking an item stores it like an edit, a click elsewhere closes it.
fn grid_drop_down(name: &str, cell: &web_sys::Element) {
    close_grid_drop_down();
    let Some((c, r)) = rapidr_value::objects::with_grid(name, |g| (g.col, g.row)) else { return };
    let Some(list) = rapidr_value::objects::with_grid(name, |g| g.list_text(c as usize, r as usize)).flatten() else { return };
    // OnListDropDown(Col, Row, S) may change the items (as on the desktop).
    let (grid, cell) = (name.to_string(), cell.clone());
    crate::object_web::rp_fire_event_then(name, "onlistdropdown", &[v_int(c), v_int(r), v_str(&list)], move |a| {
        show_grid_drop_down(&grid, &cell, rapidr_value::objects::grid::list_lines(&a[2].to_string_val()));
    });
}

fn show_grid_drop_down(name: &str, cell: &web_sys::Element, items: Vec<String>) {
    if items.is_empty() {
        return;
    }
    let rect = cell.get_bounding_client_rect();
    let list = create_el("div");
    list.set_class_name("rr-grid-dropdown");
    let st = list.style();
    for (k, v) in [("position", "fixed"), ("background", "white"), ("border", "1px solid #666"), ("z-index", "100000"), ("max-height", "200px"), ("overflow-y", "auto"), ("font-size", "13px"), ("box-shadow", "2px 2px 4px rgba(0,0,0,.3)")] {
        let _ = st.set_property(k, v);
    }
    let _ = st.set_property("left", &format!("{}px", rect.left()));
    let _ = st.set_property("top", &format!("{}px", rect.bottom()));
    let _ = st.set_property("min-width", &format!("{}px", rect.width()));
    for item in items {
        let row = create_el("div");
        row.set_class_name("rr-grid-dropdown-item");
        row.set_text_content(Some(&item));
        let _ = row.style().set_property("padding", "1px 4px");
        let _ = row.style().set_property("cursor", "default");
        let owner = name.to_uppercase();
        let pick = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            e.stop_propagation();
            close_grid_drop_down();
            grid_store(&owner, item.clone());
        });
        let _ = row.add_event_listener_with_callback("mousedown", pick.as_ref().unchecked_ref());
        pick.forget();
        let _ = list.append_child(&row);
    }
    if let Some(body) = document().body() {
        let _ = body.append_child(&list);
    }
    // A click anywhere else closes it.
    let close = Closure::once_into_js(move || {
        let outside = Closure::<dyn FnMut()>::new(close_grid_drop_down);
        let options = web_sys::AddEventListenerOptions::new();
        options.set_once(true);
        let _ = document().add_event_listener_with_callback_and_add_event_listener_options("mousedown", outside.as_ref().unchecked_ref(), &options);
        outside.forget();
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(close.unchecked_ref(), 0);
    }
}

fn close_grid_drop_down() {
    if let Ok(open) = document().query_selector_all(".rr-grid-dropdown") {
        for i in 0..open.length() {
            if let Some(el) = open.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                el.remove();
            }
        }
    }
}

thread_local! {
    /// Grids to redraw once the program yields (a loop of SetCell calls
    /// redraws once).
    static GRIDS_TO_RENDER: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Redraws a QSTRINGGRID's table from its data, soon (batched).
pub fn render_grid(name: &str) {
    let name = name.to_uppercase();
    let first = GRIDS_TO_RENDER.with(|g| {
        let mut g = g.borrow_mut();
        let first = g.is_empty();
        if !g.contains(&name) {
            g.push(name);
        }
        first
    });
    if !first {
        return;
    }
    let flush = Closure::once_into_js(move || {
        for name in GRIDS_TO_RENDER.with(|g| std::mem::take(&mut *g.borrow_mut())) {
            render_grid_now(&name);
        }
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(flush.unchecked_ref(), 0);
    }
}

/// Redraws a QSTRINGGRID's table now: fixed cells shaded, the selected
/// cell highlighted, cell text as plain text (never markup).
fn render_grid_now(name: &str) {
    use rapidr_value::objects::grid::{GO_HORZ_LINE, GO_VERT_LINE};
    let Some(table) = get_el(&format!("{}-table", comp_id(name))) else { return };
    let _ = rapidr_value::objects::with_grid(name, |g| {
        table.set_inner_text("");
        let colgroup = create_el("colgroup");
        for w in &g.col_widths {
            let col = create_el("col");
            let _ = col.style().set_property("width", &format!("{}px", (*w).clamp(0, 10_000)));
            let _ = colgroup.append_child(&col);
        }
        let _ = table.append_child(&colgroup);
        let total: i64 = g.col_widths.iter().map(|w| (*w).clamp(0, 10_000)).sum();
        let _ = table.style().set_property("width", &format!("{total}px"));
        let border = match (g.has_option(GO_HORZ_LINE), g.has_option(GO_VERT_LINE)) {
            (true, true) => "1px solid #c0c0c0",
            (false, false) => "none",
            _ => "1px solid #e0e0e0",
        };
        let tbody = create_el("tbody");
        for r in 0..g.row_count() {
            let tr = create_el("tr");
            let _ = tr.style().set_property("height", &format!("{}px", g.row_heights[r].clamp(0, 10_000)));
            for c in 0..g.col_count() {
                let td = create_el("td");
                let _ = td.set_attribute("data-col", &c.to_string());
                let _ = td.set_attribute("data-row", &r.to_string());
                let fixed = r < g.fixed_rows() || c < g.fixed_cols();
                let selected = g.is_selected(c, r);
                let st = td.style();
                // Text at Left + 2, Top + 2, as Delphi's grid (and the
                // desktop runtime) draws it.
                let _ = st.set_property("padding", "2px 2px 0 2px");
                let _ = st.set_property("vertical-align", "top");
                let _ = st.set_property("overflow", "hidden");
                let _ = st.set_property("white-space", "nowrap");
                let _ = st.set_property("cursor", "default");
                if fixed {
                    let _ = st.set_property("background", "#d4d0c8");
                    let _ = st.set_property("border", "1px outset #e8e6e0");
                } else {
                    let _ = st.set_property("border", border);
                    if selected {
                        let _ = st.set_property("background", "#0078d7");
                        let _ = st.set_property("color", "white");
                        let _ = td.set_attribute("aria-selected", "true");
                    }
                }
                let text = create_el("span");
                text.set_class_name("rr-grid-text");
                let text_cell = g.cell(c, r);
                let ellipsis = !fixed && (g.column_style(c) == rapidr_value::objects::grid::GCS_ELLIPSIS || text_cell == "...");
                if !(ellipsis && text_cell == "...") {
                    text.set_text_content(Some(text_cell));
                }
                let _ = td.append_child(&text);
                // A gcsList column's selected cell: its drop-down button.
                if (g.col, g.row) == (c as i64, r as i64) && g.list_items(c, r).is_some() {
                    let _ = st.set_property("position", "relative");
                    let button = create_el("span");
                    button.set_class_name("rr-grid-list");
                    button.set_text_content(Some("\u{25BE}"));
                    let bs = button.style();
                    for (k, v) in [("position", "absolute"), ("right", "0"), ("top", "0"), ("bottom", "0"), ("padding", "0 4px"), ("background", "#e6e6e6"), ("color", "black"), ("border", "1px outset #f4f4f4"), ("cursor", "pointer")] {
                        let _ = bs.set_property(k, v);
                    }
                    let _ = td.append_child(&button);
                }
                if ellipsis {
                    let _ = st.set_property("position", "relative");
                    let button = create_el("span");
                    button.set_class_name("rr-grid-ellipsis");
                    button.set_text_content(Some("..."));
                    let bs = button.style();
                    let _ = bs.set_property("position", "absolute");
                    let _ = bs.set_property("right", "0");
                    let _ = bs.set_property("top", "0");
                    let _ = bs.set_property("bottom", "0");
                    let _ = bs.set_property("padding", "0 4px");
                    let _ = bs.set_property("background", "#e6e6e6");
                    let _ = bs.set_property("color", "black");
                    let _ = bs.set_property("border", "1px outset #f4f4f4");
                    let _ = bs.set_property("cursor", "pointer");
                    let _ = td.append_child(&button);
                }
                // What OnDrawCell drew, over the cell.
                if let Some(ops) = g.owner_drawing.get(&(c, r)) {
                    let _ = st.set_property("position", "relative");
                    grid_replay(&td, ops, g.col_widths[c].clamp(0, 10_000) as u32, g.row_heights[r].clamp(0, 10_000) as u32);
                }
                let _ = tr.append_child(&td);
            }
            let _ = tbody.append_child(&tr);
        }
        let _ = table.append_child(&tbody);
    });
    grid_owner_draw(name);
}

/// OnDrawCell(Col, Row, State, Rect): fired for every cell after the grid
/// changed, as on the desktop (rapidr_value::objects::grid). Each cell's
/// Rect is a QRECT (a property bag).
fn grid_owner_draw(name: &str) {
    if !crate::object_web::rp_has_handler(name, "ondrawcell") {
        return;
    }
    if !rapidr_value::objects::with_grid_mut(name, |g| g.owner_draw_needed()).unwrap_or(false) {
        return;
    }
    let cells = rapidr_value::objects::with_grid(name, |g| g.owner_draw_cells()).unwrap_or_default();
    for (col, row, state, (left, top, right, bottom)) in cells {
        let rect = format!("{}.CELLRECT({col},{row})", name.to_uppercase());
        // A property bag, as the desktop runtime's.
        crate::object_web::rp_create_component(&rect, "RUDT");
        for (prop, v) in [("left", left), ("top", top), ("right", right), ("bottom", bottom)] {
            crate::object_web::rp_comp_set(&rect, prop, v_int(v));
        }
        crate::object_web::rp_fire_event_args(name, "ondrawcell", &[v_int(col as i64), v_int(row as i64), v_int(state), crate::value::v_str(&rect)]);
    }
}

/// Draws what OnDrawCell drew on a cell: a canvas over the cell.
fn grid_replay(td: &web_sys::HtmlElement, ops: &[rapidr_value::objects::grid::CellDraw], w: u32, h: u32) {
    use rapidr_value::objects::grid::CellDraw;
    let Some(canvas) = document().create_element("canvas").ok().and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else { return };
    canvas.set_width(w.max(1));
    canvas.set_height(h.max(1));
    let cs = canvas.style();
    let _ = cs.set_property("position", "absolute");
    let _ = cs.set_property("left", "0");
    let _ = cs.set_property("top", "0");
    let _ = cs.set_property("pointer-events", "none");
    let Some(ctx) = canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok()) else { return };
    let css = |c: u32| format!("rgb({},{},{})", c & 0xFF, (c >> 8) & 0xFF, (c >> 16) & 0xFF);
    let f = |v: i64| v.clamp(-100_000, 100_000) as f64;
    for op in ops {
        match op {
            CellDraw::Line(x1, y1, x2, y2, c) => {
                ctx.set_stroke_style_str(&css(*c));
                ctx.begin_path();
                ctx.move_to(f(*x1) + 0.5, f(*y1) + 0.5);
                ctx.line_to(f(*x2) + 0.5, f(*y2) + 0.5);
                ctx.stroke();
            }
            CellDraw::Rect(x1, y1, x2, y2, c) => {
                ctx.set_stroke_style_str(&css(*c));
                ctx.stroke_rect(f(*x1.min(x2)) + 0.5, f(*y1.min(y2)) + 0.5, f((x2 - x1).abs() - 1), f((y2 - y1).abs() - 1));
            }
            CellDraw::Fill(x1, y1, x2, y2, c) => {
                ctx.set_fill_style_str(&css(*c));
                ctx.fill_rect(f(*x1.min(x2)), f(*y1.min(y2)), f((x2 - x1).abs()), f((y2 - y1).abs()));
            }
            CellDraw::Ellipse(x1, y1, x2, y2, c, fill) => {
                let (rx, ry) = (f((x2 - x1).abs()) / 2.0, f((y2 - y1).abs()) / 2.0);
                let (cx, cy) = (f(*x1.min(x2)) + rx, f(*y1.min(y2)) + ry);
                ctx.begin_path();
                let _ = ctx.ellipse(cx, cy, rx.max(0.0), ry.max(0.0), 0.0, 0.0, std::f64::consts::TAU);
                if let Some(fc) = fill {
                    ctx.set_fill_style_str(&css(*fc));
                    ctx.fill();
                }
                ctx.set_stroke_style_str(&css(*c));
                ctx.stroke();
            }
            CellDraw::Pixel(x, y, c) => {
                ctx.set_fill_style_str(&css(*c));
                ctx.fill_rect(f(*x), f(*y), 1.0, 1.0);
            }
            CellDraw::Text(x, y, text, c, bg) => {
                ctx.set_font("13px sans-serif");
                ctx.set_text_baseline("top");
                if let Some(bg) = bg {
                    let width = ctx.measure_text(text).map(|m| m.width()).unwrap_or(0.0);
                    ctx.set_fill_style_str(&css(*bg));
                    ctx.fill_rect(f(*x), f(*y), width, 15.0);
                }
                ctx.set_fill_style_str(&css(*c));
                let _ = ctx.fill_text(text, f(*x), f(*y));
            }
            CellDraw::Image(x, y, b) => {
                let (bw, bh) = (b.img.width as u32, b.img.height as u32);
                if bw == 0 || bh == 0 {
                    continue;
                }
                let rgba = b.to_rgba();
                let Ok(data) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&rgba), bw, bh) else { continue };
                let _ = ctx.put_image_data(&data, f(*x), f(*y));
            }
        }
    }
    let _ = td.append_child(&canvas);
}

/// Fills a QLISTVIEW's table from its data: a header row (unless
/// ShowColumnHeaders is False), then one row per item — the caption and its
/// sub-items in the columns. Cells are plain text, never markup.
pub fn render_listview(name: &str) {
    let Some(table) = get_el(&format!("{}-table", comp_id(name))) else { return };
    let show = crate::object_web::rp_comp_get_stored(name, "showcolumnheaders");
    let show_header = matches!(show, Value::Null) || show.to_bool();
    let _ = rapidr_value::objects::with_listview(name, |lv| {
        table.set_inner_text("");
        let n_cols = lv.columns.len().max(1);
        if !lv.columns.is_empty() {
            let colgroup = create_el("colgroup");
            for c in &lv.columns {
                let col = create_el("col");
                let _ = col.style().set_property("width", &format!("{}px", c.width.clamp(1, 10_000)));
                let _ = colgroup.append_child(&col);
            }
            let _ = table.append_child(&colgroup);
            let total: i64 = lv.columns.iter().map(|c| c.width.clamp(1, 10_000)).sum();
            let _ = table.style().set_property("width", &format!("{total}px"));
        }
        if show_header && !lv.columns.is_empty() {
            let thead = create_el("thead");
            let tr = create_el("tr");
            for (i, c) in lv.columns.iter().enumerate() {
                let th = create_el("th");
                th.set_text_content(Some(&c.caption));
                let _ = th.set_attribute("data-col", &i.to_string());
                let st = th.style();
                let _ = st.set_property("text-align", "left");
                let _ = st.set_property("background", "#eee");
                let _ = st.set_property("border", "1px outset #ddd");
                let _ = st.set_property("padding", "1px 4px");
                let _ = st.set_property("overflow", "hidden");
                let _ = st.set_property("white-space", "nowrap");
                let _ = st.set_property("cursor", "default");
                let _ = tr.append_child(&th);
            }
            let _ = thead.append_child(&tr);
            let _ = table.append_child(&thead);
        }
        let tbody = create_el("tbody");
        for (row, item) in lv.items.iter().enumerate() {
            let tr = create_el("tr");
            let _ = tr.set_attribute("data-row", &row.to_string());
            if row as i64 == lv.item_index || item.selected {
                let _ = tr.style().set_property("background", "#3366dd");
                let _ = tr.style().set_property("color", "white");
                let _ = tr.set_attribute("aria-selected", "true");
            }
            let empty = String::new();
            let cells = std::iter::once(&item.caption).chain(item.sub_items.iter()).chain(std::iter::repeat(&empty));
            for cell in cells.take(n_cols) {
                let td = create_el("td");
                if !cell.is_empty() {
                    td.set_text_content(Some(cell));
                }
                let st = td.style();
                let _ = st.set_property("padding", "1px 4px");
                let _ = st.set_property("overflow", "hidden");
                let _ = st.set_property("white-space", "nowrap");
                let _ = tr.append_child(&td);
            }
            let _ = tbody.append_child(&tr);
        }
        let _ = table.append_child(&tbody);
    });
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
    // Always an <iframe> so Navigate() / SetHtml() can drive it.
    let el = create_el("iframe");
    if let Ok(iframe) = el.clone().dyn_into::<web_sys::HtmlIFrameElement>() {
        if let Some(url) = props.get("url") {
            iframe.set_src(&url.to_string_val());
        } else if let Some(html) = props.get("html") {
            iframe.set_srcdoc(&html.to_string_val());
        }
        let _ = iframe.set_attribute("sandbox", "allow-scripts allow-same-origin allow-forms allow-popups allow-modals allow-downloads");
    }
    el.set_class_name("rr-widget");
    let _ = el.style().set_property("border", "1px solid #aaa");
    let _ = el.style().set_property("background", "white");
    setup_widget(&el, id, name, props);
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

    let is_head_tag = matches!(
        tag.to_uppercase().as_str(),
        "STYLE" | "SCRIPT" | "LINK" | "META" | "TITLE"
    );

    if !is_head_tag {
        let _ = el.style().set_property("position", "absolute");
        let _ = el.style().set_property("box-sizing", "border-box");
        apply_geometry(&el, props, 0, 0, 100, 25);

        // Append to parent if specified, otherwise to body
        let parent = props.get("parentid").or_else(|| props.get("parent")).map(|v| v.to_string_val());
        let parent_el = get_parent_client(&parent);
        let _ = parent_el.append_child(&el);
    } else {
        if let Ok(Some(head)) = document().query_selector("head") {
            let _ = head.append_child(&el);
        } else {
            let _ = document().body().unwrap().append_child(&el);
        }
    }
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
// StringGrid helpers
// ---------------------------------------------------------------------------


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
    let el = match get_el(&id) {
        Some(e) => e,
        None => return,
    };

    let comp_type = crate::object_web::rp_comp_type(name);
    if comp_type == "RMAINMENU" {
        let parent_id = comp_id(parent_name);
        if let (Some(form_el), Some(client_el)) = (get_el(&parent_id), get_el(&format!("{}-client", parent_id))) {
            let _ = form_el.insert_before(&el, Some(&client_el));
            let bs = crate::object_web::rp_comp_get_stored(parent_name, "borderstyle");
            place_form_chrome(&form_el, if matches!(bs, Value::Null) { 2 } else { bs.to_i64() }, true);
        }
        return;
    }

    if comp_type == "RMENUITEM" {
        let parent_type = crate::object_web::rp_comp_type(parent_name);
        if parent_type == "RMENUITEM" {
            // Submenu item
            let parent_id = comp_id(parent_name);
            if let Some(parent_el) = get_el(&parent_id) {
                let dropdown_el = if let Ok(Some(d)) = parent_el.query_selector(".rr-dropdown-menu") {
                    d.dyn_into::<web_sys::HtmlElement>().unwrap()
                } else {
                    let d = create_el("div");
                    d.set_class_name("rr-dropdown-menu");
                    let _ = parent_el.append_child(&d);
                    d
                };
                el.set_class_name("rr-menu-item-sub");
                let _ = dropdown_el.append_child(&el);
            }
        } else if parent_type == "RMAINMENU" {
            // Top-level menu item
            let parent_id = comp_id(parent_name);
            if let Some(parent_el) = get_el(&parent_id) {
                el.set_class_name("rr-menu-item-top");
                let _ = parent_el.append_child(&el);
            }
        } else {
            // Fallback (e.g. parent is a form or panel)
            el.set_class_name("");
            let parent_client_id = format!("{}-client", comp_id(parent_name));
            let parent_el = get_el(&parent_client_id).or_else(|| get_el(&comp_id(parent_name)));
            if let Some(p) = parent_el {
                let _ = p.append_child(&el);
            }
        }
        return;
    }

    let parent_client_id = format!("{}-client", comp_id(parent_name));
    // Try parent's client area first, then the parent element itself
    let parent_el = get_el(&parent_client_id)
        .or_else(|| get_el(&comp_id(parent_name)));
    if let Some(p) = parent_el {
        let _ = p.append_child(&el);
        // Mark nested forms so finalize can route visibility correctly.
        if el.class_list().contains("rr-form") {
            let _ = el.set_attribute("data-rr-parent", parent_name);
            // Nested forms are positioned relative to parent client and
            // should not be modal/full-screen.
            let _ = el.style().set_property("position", "absolute");
        }
    }
}

/// Auto-parent orphan widgets (those appended to body) to the first top-level
/// form, then show all top-level forms and fire `onload` for each form.
pub fn gui_web_finalize() {
    let doc = document();

    // Find the first TOP-LEVEL form (not nested inside another form).
    // Top-level forms are direct body children OR carry no data-rr-parent.
    let first_top_form = match doc.query_selector(".rr-form:not([data-rr-parent])") {
        Ok(Some(el)) => Some(el),
        _ => doc.query_selector(".rr-form").ok().flatten(),
    };

    // Move orphan widgets from body into the first top-level form's client area.
    // Skip elements that are themselves forms (`.rr-form`).
    if let Some(form_el) = first_top_form.as_ref() {
        let form_id = form_el.get_attribute("id").unwrap_or_default();
        let client_id = format!("{}-client", form_id);
        let client = doc.get_element_by_id(&client_id).unwrap_or_else(|| form_el.clone());

        let selector = "body > [data-rr-name], body > .rr-widget, body > .rr-plot-container";
        if let Ok(orphans) = doc.query_selector_all(selector) {
            let mut elems: Vec<web_sys::Element> = Vec::new();
            for i in 0..orphans.length() {
                if let Some(node) = orphans.item(i) {
                    if let Some(el) = node.dyn_ref::<web_sys::Element>() {
                        // Skip forms — they manage their own placement.
                        if el.class_list().contains("rr-form") {
                            continue;
                        }
                        elems.push(el.clone());
                    }
                }
            }
            for el in &elems {
                let _ = client.append_child(el);
            }
        }
    }

    // Show top-level forms whose `visible` prop is true (default), assigning a
    // stacking z-index. Forms whose visible was set false (e.g. by `.Hide()`
    // before the runtime started) stay hidden.
    if let Ok(top_forms) = doc.query_selector_all(".rr-form:not([data-rr-parent])") {
        for i in 0..top_forms.length() {
            if let Some(node) = top_forms.item(i) {
                let comp_name = node
                    .dyn_ref::<web_sys::Element>()
                    .and_then(|e| e.get_attribute("data-rr-name"))
                    .unwrap_or_default();
                let visible = crate::object_web::rp_comp_get_stored(&comp_name, "visible").to_bool();
                let id_attr = node
                    .dyn_ref::<web_sys::Element>()
                    .and_then(|e| e.get_attribute("id"))
                    .unwrap_or_default();
                let is_modal = !id_attr.is_empty()
                    && doc.get_element_by_id(&format!("{}-backdrop", id_attr)).is_some();
                if let Ok(html) = node.dyn_into::<web_sys::HtmlElement>() {
                    if visible {
                        let _ = html.style().set_property("display", "block");
                        if is_modal {
                            let _ = html.style().set_property("z-index", "9999");
                        } else {
                            let _ = html.style().set_property("z-index", &format!("{}", 10 + i));
                        }
                    } else {
                        let _ = html.style().set_property("display", "none");
                    }
                }
            }
        }
    }
    // Nested forms: honor their visible prop too.
    if let Ok(nested) = doc.query_selector_all(".rr-form[data-rr-parent]") {
        for i in 0..nested.length() {
            if let Some(node) = nested.item(i) {
                let comp_name = node
                    .dyn_ref::<web_sys::Element>()
                    .and_then(|e| e.get_attribute("data-rr-name"))
                    .unwrap_or_default();
                let visible = crate::object_web::rp_comp_get_stored(&comp_name, "visible").to_bool();
                if let Ok(html) = node.dyn_into::<web_sys::HtmlElement>() {
                    let _ = html.style().set_property(
                        "display",
                        if visible { "block" } else { "none" },
                    );
                }
            }
        }
    }

    // Inject hover styles for titlebar buttons
    inject_form_styles();

    // Fire onload for every form (top-level and nested), in DOM order.
    if let Ok(all_forms) = doc.query_selector_all(".rr-form") {
        for i in 0..all_forms.length() {
            if let Some(node) = all_forms.item(i) {
                if let Some(el) = node.dyn_ref::<web_sys::Element>() {
                    if let Some(comp_name) = el.get_attribute("data-rr-name") {
                        crate::object_web::rp_fire_event(&comp_name, "onload");
                    }
                }
            }
        }
    }
    // Then the first OnPaint of each form and canvas (RapidQ programs draw
    // there); the surfaces keep what's drawn.
    if let Ok(all) = doc.query_selector_all(".rr-form, [data-rr-type=\"RCANVAS\"]") {
        for i in 0..all.length() {
            if let Some(el) = all.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                if let Some(comp_name) = el.get_attribute("data-rr-name") {
                    crate::object_web::rp_fire_event(&comp_name, "onpaint");
                }
            }
        }
    }
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
        form_resized(form_id, l, t, w, h);
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
        let (vw, vh) = web_sys::window()
            .map(|w| (w.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0), w.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0)))
            .unwrap_or((0.0, 0.0));
        form_resized(form_id, 0, 0, vw as i32, vh as i32);
    }
}

/// Places a form's title bar, main menu and client area for its
/// BorderStyle (bsNone: no title bar or border) — the frame
/// `rapidr_value::layout::form_frame` accounts for.
fn place_form_chrome(form: &web_sys::HtmlElement, border_style: i64, has_menu: bool) {
    use rapidr_value::layout::{FORM_BORDER, FORM_CAPTION};
    let framed = border_style != 0;
    let menu = if has_menu { crate::layout_web::MENU_HEIGHT } else { 0 };
    let caption = if framed { FORM_CAPTION } else { 0 };
    let _ = form.style().set_property("border", if framed { &"1px solid #999"[..] } else { "none" });
    let _ = form.style().set_property("border-width", &format!("{}px", if framed { FORM_BORDER } else { 0 }));
    if let Ok(Some(bar)) = form.query_selector(":scope > .rr-form-titlebar") {
        if let Ok(bar) = bar.dyn_into::<web_sys::HtmlElement>() {
            let _ = bar.style().set_property("display", if framed { "flex" } else { "none" });
        }
    }
    if let Ok(Some(nav)) = form.query_selector(":scope > nav[data-rr-type=\"RMAINMENU\"]") {
        if let Ok(nav) = nav.dyn_into::<web_sys::HtmlElement>() {
            let _ = nav.style().set_property("top", &format!("{caption}px"));
        }
    }
    if let Some(client) = get_el(&format!("{}-client", form.id())) {
        let top = caption + menu;
        let _ = client.style().set_property("top", &format!("{top}px"));
        let _ = client.style().set_property("height", &format!("calc(100% - {top}px)"));
    }
}

/// A form was moved / resized by the user (maximize, restore, drag): its
/// Left / Top / Width / Height follow, its aligned children are laid out
/// again and OnResize fires (as on the desktop).
fn form_resized(form_id: &str, left: i32, top: i32, width: i32, height: i32) {
    let name = form_id.strip_prefix("rr-").unwrap_or(form_id).to_uppercase();
    crate::object_web::rp_comp_set_prop_only(&name, "left", v_int(left as i64));
    crate::object_web::rp_comp_set_prop_only(&name, "top", v_int(top as i64));
    let stored = |p: &str| crate::object_web::rp_comp_get_stored(&name, p).to_i64();
    if (stored("width"), stored("height")) == (width as i64, height as i64) {
        return;
    }
    crate::object_web::rp_comp_set_prop_only(&name, "width", v_int(width as i64));
    crate::object_web::rp_comp_set_prop_only(&name, "height", v_int(height as i64));
    crate::layout_web::realign(&name, None);
    crate::object_web::rp_fire_event(&name, "onresize");
}

/// For tests (as the desktop's `RAPIDR_TEST_RESIZE`): the user resizes
/// form `name` to Width × Height.
pub fn test_resize_form(name: &str, width: i32, height: i32) {
    let id = comp_id(name);
    let Some(el) = get_el(&id) else { return };
    let _ = el.style().set_property("width", &format!("{width}px"));
    let _ = el.style().set_property("height", &format!("{height}px"));
    form_resized(&id, el.offset_left(), el.offset_top(), width, height);
}

/// Hides a form (END: no OnClose).
pub fn hide_form(name: &str) {
    let id = comp_id(name);
    if let Some(el) = get_el(&id) {
        let _ = el.style().set_property("display", "none");
    }
    hide_modal_backdrop(&id);
}

fn form_close(form_id: &str) {
    close_form(&form_id.strip_prefix("rr-").unwrap_or(form_id).to_uppercase());
}

/// `Form.Close` and the title bar's ✕: OnClose's `Action` (it starts as
/// `caHide`) decides whether the form goes, stays or is minimized.
pub fn close_form(name: &str) {
    use rapidr_value::events::{CloseAction, CA_HIDE};
    let form = name.to_string();
    crate::object_web::rp_fire_event_then(name, "onclose", &[v_int(CA_HIDE)], move |a| match CloseAction::of(&a[0]) {
        CloseAction::Stay => {}
        CloseAction::Minimize => form_minimize(&comp_id(&form)),
        CloseAction::Close => {
            crate::object_web::rp_comp_set(&form, "visible", crate::value::v_bool(false));
            hide_form(&form);
        }
    });
}

/// Show a dimmed backdrop behind a modal form. The backdrop sits one z-index
/// below the form and is removed when the form is closed/hidden.
///
/// In a single-form, full-viewport web runtime (the common case for RapidR
/// IDE Run + bundled apps) the backdrop is just visual noise that obscures
/// the running app — the *form* is already the entire UI. We keep the
/// z-index lift so the form is on top of any other windows that might
/// appear later, but skip creating the dim overlay.
fn show_modal_backdrop(form_id: &str) {
    let doc = document();
    if let Some(el) = doc.get_element_by_id(form_id) {
        if let Some(html) = el.dyn_ref::<web_sys::HtmlElement>() {
            let _ = html.style().set_property("z-index", "9999");
        }
    }
}

fn hide_modal_backdrop(form_id: &str) {
    let backdrop_id = format!("{}-backdrop", form_id);
    if let Some(bd) = document().get_element_by_id(&backdrop_id) {
        bd.remove();
    }
    // A program waiting in this form's ShowModal continues.
    crate::dialog_web::modal_closed(form_id);
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
        let form_id_up = form_id_owned.clone();
        let up_cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::once(move |_e: web_sys::MouseEvent| {
            let doc = document();
            let _ = doc.remove_event_listener_with_callback("mousemove", move_ref.unchecked_ref());
            drop(move_cb); // prevent leak
            // Form.Left / Form.Top read where it was dragged.
            if let Some(f) = get_el(&form_id_up) {
                let name = form_id_up.strip_prefix("rr-").unwrap_or(&form_id_up).to_uppercase();
                crate::object_web::rp_comp_set_prop_only(&name, "left", v_int(f.offset_left() as i64));
                crate::object_web::rp_comp_set_prop_only(&name, "top", v_int(f.offset_top() as i64));
            }
        });
        let options = web_sys::AddEventListenerOptions::new();
        options.set_once(true);
        let _ = doc.add_event_listener_with_callback_and_add_event_listener_options(
            "mouseup",
            up_cb.as_ref().unchecked_ref(),
            &options,
        );
        up_cb.forget();
    });
    let _ = titlebar.add_event_listener_with_callback("mousedown", cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Inject CSS for base styles and form button hover effects.
fn inject_form_styles() {
    let doc = document();
    if doc.get_element_by_id("rr-form-styles").is_some() {
        return; // Already injected
    }
    let style = doc.create_element("style").unwrap();
    style.set_id("rr-form-styles");
    let full_css = format!(
        "{}\n{}",
        crate::RR_BASE_CSS,
        ".rr-form-btn-min:hover,.rr-form-btn-max:hover{background:rgba(255,255,255,0.2)!important}\
         .rr-form-btn-close:hover{background:#e74c3c!important}\
         .rr-tab-btn{padding:6px 16px;border:none;background:#f0f0f0;cursor:pointer;font-size:13px;\
         border-bottom:2px solid transparent;transition:background 0.15s}\
         .rr-tab-btn:hover{background:#e0e0e0}\
         .rr-menu-item-top{position:relative;display:inline-block;padding:6px 12px;cursor:pointer;font-weight:500}\
         .rr-menu-item-top:hover{background:#e0e0e0}\
         .rr-menu-item-top:hover > .rr-dropdown-menu{display:block!important}\
         .rr-dropdown-menu{display:none;position:absolute;top:100%;left:0;background:#fff;border:1px solid #ccc;\
         box-shadow:0 2px 8px rgba(0,0,0,0.15);min-width:150px;z-index:1000;padding:4px 0;border-radius:4px}\
         .rr-menu-item-sub{padding:6px 16px;cursor:pointer;white-space:nowrap;font-size:13px;color:#333;display:block}\
         .rr-menu-item-sub:hover{background:#007acc;color:#fff!important}"
    );
    style.set_text_content(Some(&full_css));
    if let Ok(Some(head)) = doc.query_selector("head") {
        let _ = head.append_child(&style);
    }
}

pub fn setup_data_binding(name: &str) {
    let uname = name.to_uppercase();
    let ds = crate::object_web::rp_comp_get_stored(&uname, "datasource").to_string_val();
    let df = crate::object_web::rp_comp_get_stored(&uname, "datafield").to_string_val();
    if ds.is_empty() || df.is_empty() {
        return;
    }

    thread_local! {
        static BOUND: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(std::collections::HashSet::new());
    }
    let already = BOUND.with(|b| !b.borrow_mut().insert(uname.clone()));
    if already {
        return;
    }

    let id = comp_id(name);
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };
    let el = match doc.get_element_by_id(&id) {
        Some(e) => e,
        None => return,
    };

    let ds_closure = ds.clone();
    let df_closure = df.clone();
    let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let val = if let Ok(input) = target.clone().dyn_into::<web_sys::HtmlInputElement>() {
            if input.type_() == "checkbox" || input.type_() == "radio" {
                if input.checked() {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            } else {
                input.value()
            }
        } else if let Ok(ta) = target.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
            ta.value()
        } else if let Ok(sel) = target.clone().dyn_into::<web_sys::HtmlSelectElement>() {
            sel.value()
        } else {
            String::new()
        };
        crate::database_web::update_bound_data(&ds_closure, &df_closure, &val);
    });

    let _ = el.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref());
    let _ = el.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
    closure.forget();
}
