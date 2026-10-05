//! The web-only components on the UI kernel's page (docs/web-host-plan.md
//! §3.6, Stage W6): RWEBVIEW (an `<iframe>`), RDOM (the program's own
//! element), RWEBAUDIO / RWEBVIDEO (`<audio>` / `<video>`), and the data
//! science lane's RPLOT (its chart's HTML canvas) need the DOM by their
//! nature. Each is a real element with the id `rr-<name>` (the one
//! scripts and RDOM's AppendTo / QuerySelector use), and the kernel places
//! it: the component is a node of its form like any other (Left, Top,
//! Width, Height, Visible, Align, its parents'), drawn by nothing; the host
//! puts the element over the window's canvas at the node's place, clipped
//! to its parents (`rapidr_ui_host_web::host::set_overlay_types`). The
//! element takes its own pointer events and keyboard (an iframe's page, a
//! video's controls).
//!
//! An RDOM whose Parent isn't a kernel component (none, or another RDOM by
//! ParentId / AppendTo) stays where the program put it in the page.

use rapidr_value::{v_null, v_str, Value};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlMediaElement};

use crate::object_web::{rp_comp_get_stored, rp_comp_type};

/// The components the host places as DOM elements over its canvases.
pub const TYPES: &[&str] = &["RWEBVIEW", "RDOM", "RWEBAUDIO", "RWEBVIDEO", "RPLOT"];

pub fn is_overlay(type_name: &str) -> bool {
    TYPES.iter().any(|t| t.eq_ignore_ascii_case(type_name))
}

fn document() -> web_sys::Document {
    web_sys::window().expect("a page").document().expect("a document")
}

fn id_of(name: &str) -> String {
    format!("rr-{}", name.to_lowercase())
}

/// Component `name`'s element, if it has one.
pub fn element(name: &str) -> Option<HtmlElement> {
    document().get_element_by_id(&id_of(name))?.dyn_into().ok()
}

fn create_el(tag: &str) -> Option<HtmlElement> {
    document().create_element(tag).ok()?.dyn_into().ok()
}

fn stored(name: &str, prop: &str) -> Option<Value> {
    let v = rp_comp_get_stored(name, prop);
    (!matches!(v, Value::Null)).then_some(v)
}

/// Where an asset name or a URL points (a project's file: its data URL).
fn resolve(s: &str) -> String {
    if s.starts_with("assets/") || (!s.contains("://") && !s.starts_with("data:") && !s.is_empty()) {
        crate::database_web::get_rapidr_asset(s).unwrap_or_else(|| s.to_string())
    } else {
        s.to_string()
    }
}

/// Whether `parent` is a component the kernel places (a form or a control
/// of one), so its child's element goes over the window's canvas.
fn kernel_placed(parent: &str) -> bool {
    !parent.trim().is_empty() && !rp_comp_type(parent.trim()).is_empty() && rp_comp_type(parent.trim()) != "RDOM"
}

/// Component `name` (of a web-only type) made: its element, with what the
/// program already set.
pub fn create(name: &str, type_name: &str) {
    if element(name).is_some() {
        return;
    }
    let t = type_name.to_uppercase();
    let tag = match t.as_str() {
        "RWEBVIEW" => "iframe".to_string(),
        "RWEBAUDIO" => "audio".to_string(),
        "RWEBVIDEO" => "video".to_string(),
        "RDOM" => stored(name, "tagname").map(|v| v.to_string_val().to_lowercase()).filter(|s| !s.is_empty()).unwrap_or_else(|| "div".into()),
        _ => "div".to_string(),
    };
    let Some(el) = create_el(&tag) else { return };
    el.set_id(&id_of(name));
    let _ = el.set_attribute("data-rr-name", name);
    let _ = el.set_attribute("data-rr-type", &t);
    match t.as_str() {
        "RWEBVIEW" => {
            // (sandboxed as the DOM runtime's: the page it shows runs its
            // scripts, never the program's origin's privileges)
            let _ = el.set_attribute("sandbox", "allow-scripts allow-same-origin allow-forms allow-popups allow-modals allow-downloads");
            let _ = el.style().set_property("border", "1px solid #aaa");
            let _ = el.style().set_property("background", "white");
        }
        "RWEBVIDEO" => {
            if let Some(v) = el.dyn_ref::<HtmlMediaElement>() {
                v.set_controls(stored(name, "controls").is_none_or(|c| c.to_bool()));
            }
        }
        "RPLOT" => {
            el.set_class_name("rr-plot-container");
            let _ = el.style().set_property("background", "white");
            let _ = el.style().set_property("border", "1px solid #ccc");
        }
        _ => {}
    }
    let _ = el.style().set_property("position", "absolute");
    let _ = el.style().set_property("box-sizing", "border-box");
    // (what the program set before the element was made)
    let props = ["url", "html", "src", "loop", "autoplay", "volume", "poster", "cssclass", "cssstyle", "innerhtml", "innertext"];
    for p in props {
        if let Some(v) = stored(name, p) {
            set_prop(name, p, &v);
        }
    }
    attach(name, &t, &tag, &el);
    crate::object_web::rp_rebind_component_events(name);
}

/// Where a new element goes: a kernel-placed one into the page, hidden
/// until the host places it over its window; an RDOM by its ParentId /
/// Parent in the page, or the page's body; a STYLE / SCRIPT … into the head.
fn attach(name: &str, t: &str, tag: &str, el: &HtmlElement) {
    let doc = document();
    if t == "RDOM" && matches!(tag, "style" | "script" | "link" | "meta" | "title") {
        let _ = el.style().remove_property("position");
        if let Some(head) = doc.head() {
            let _ = head.append_child(el);
        }
        return;
    }
    let parent = stored(name, "parent").map(|v| v.to_string_val()).unwrap_or_default();
    let parent_id = stored(name, "parentid").map(|v| v.to_string_val()).unwrap_or_default();
    if t != "RDOM" || (parent_id.is_empty() && kernel_placed(&parent)) {
        let _ = el.set_attribute("data-rr-overlay", "kernel");
        let _ = el.style().set_property("display", "none");
        if let Some(body) = doc.body() {
            let _ = body.append_child(el);
        }
        return;
    }
    // (an RDOM of the page: placed by its own Left / Top / … in its parent
    // element, as the DOM runtime placed it)
    for (p, d) in [("left", 0), ("top", 0), ("width", 100), ("height", 25)] {
        let v = stored(name, p).map_or(d, |v| v.to_i64());
        let _ = el.style().set_property(p, &format!("{v}px"));
    }
    let target = [parent_id, parent].into_iter().find(|p| !p.trim().is_empty()).and_then(|p| element(p.trim()));
    if let Some(p) = target.or_else(|| doc.body()) {
        let _ = p.append_child(el);
    }
}

/// A property of a web-only component set (stored already): `true` when
/// it's the element's own (anything placing it is the kernel's).
pub fn set_prop(name: &str, prop: &str, val: &Value) -> bool {
    let Some(el) = element(name) else { return false };
    let s = val.to_string_val();
    let media = el.dyn_ref::<HtmlMediaElement>();
    match prop {
        "html" => match el.dyn_ref::<web_sys::HtmlIFrameElement>() {
            Some(f) => f.set_srcdoc(&s),
            None => el.set_inner_html(&s),
        },
        "url" => {
            if let Some(f) = el.dyn_ref::<web_sys::HtmlIFrameElement>() {
                f.set_src(&s);
            }
        }
        "sandbox" => {
            let _ = el.set_attribute("sandbox", &s);
        }
        "src" | "picture" if media.is_some() => media.expect("media").set_src(&resolve(&s)),
        "volume" if media.is_some() => media.expect("media").set_volume(val.to_f64()),
        "currenttime" if media.is_some() => media.expect("media").set_current_time(val.to_f64()),
        "loop" if media.is_some() => media.expect("media").set_loop(val.to_bool()),
        "autoplay" if media.is_some() => media.expect("media").set_autoplay(val.to_bool()),
        "controls" if media.is_some() => media.expect("media").set_controls(val.to_bool()),
        "poster" => {
            if let Some(v) = el.dyn_ref::<web_sys::HtmlVideoElement>() {
                v.set_poster(&resolve(&s));
            }
        }
        "innerhtml" => el.set_inner_html(&s),
        "innertext" => el.set_inner_text(&s),
        "cssclass" => el.set_class_name(&s),
        "cssstyle" => {
            // (its own style, keeping where it's placed)
            let keep: Vec<(String, String)> = ["left", "top", "width", "height", "position", "display", "box-sizing", "z-index", "clip-path"]
                .iter()
                .map(|p| (p.to_string(), el.style().get_property_value(p).unwrap_or_default()))
                .collect();
            let _ = el.set_attribute("style", &s);
            for (p, v) in keep {
                if !v.is_empty() {
                    let _ = el.style().set_property(&p, &v);
                }
            }
        }
        "tooltip" | "hint" => el.set_title(&s),
        "tagname" => retag(name, &el, &s),
        // (an RDOM of the page places itself)
        "left" | "top" | "width" | "height" if el.get_attribute("data-rr-overlay").is_none() => {
            let _ = el.style().set_property(prop, &format!("{}px", val.to_i64()));
        }
        "visible" if el.get_attribute("data-rr-overlay").is_none() => {
            let _ = el.style().set_property("display", if val.to_bool() { "" } else { "none" });
        }
        "parent" | "parentid" => {
            let t = rp_comp_type(name);
            let tag = el.tag_name().to_lowercase();
            el.remove();
            let _ = el.remove_attribute("data-rr-overlay");
            attach(name, &t, &tag, &el);
            // (the kernel's tree follows too)
            return prop == "parentid";
        }
        "left" | "top" | "width" | "height" | "visible" | "align" | "anchors" => return false,
        _ if rp_comp_type(name) == "RDOM" => {
            let _ = el.set_attribute(prop, &s);
        }
        _ => return false,
    }
    true
}

/// An RDOM's TagName changed: a new element of that tag, with the old
/// one's attributes and children, in its place.
fn retag(name: &str, el: &HtmlElement, tag: &str) {
    let tag = tag.to_lowercase();
    if tag.is_empty() || el.tag_name().eq_ignore_ascii_case(&tag) {
        return;
    }
    let Some(new_el) = create_el(&tag) else { return };
    let attrs = el.attributes();
    for i in 0..attrs.length() {
        if let Some(a) = attrs.item(i) {
            let _ = new_el.set_attribute(&a.name(), &a.value());
        }
    }
    while let Some(child) = el.first_child() {
        let _ = new_el.append_child(&child);
    }
    if let Some(parent) = el.parent_node() {
        let _ = parent.replace_child(&new_el, el);
    }
    crate::object_web::rp_rebind_component_events(name);
}

/// A web-only component's property as its element has it now (`None`: the
/// stored one answers).
pub fn get_prop(name: &str, prop: &str) -> Option<Value> {
    let el = element(name)?;
    let media = el.dyn_ref::<HtmlMediaElement>();
    Some(match prop {
        "innerhtml" => v_str(&el.inner_html()),
        "innertext" => v_str(&el.inner_text()),
        "cssclass" => v_str(&el.class_name()),
        "cssstyle" => v_str(&el.get_attribute("style").unwrap_or_default()),
        "tagname" => v_str(&el.tag_name().to_lowercase()),
        "url" => match el.dyn_ref::<web_sys::HtmlIFrameElement>() {
            Some(f) => v_str(&f.src()),
            None => v_str(&el.get_attribute("src").unwrap_or_default()),
        },
        "html" => match el.dyn_ref::<web_sys::HtmlIFrameElement>() {
            Some(f) => v_str(&f.srcdoc()),
            None => v_str(&el.get_attribute("srcdoc").unwrap_or_default()),
        },
        "volume" => Value::Double(media?.volume()),
        "currenttime" => Value::Double(media?.current_time()),
        "duration" => Value::Double(media?.duration()),
        "playing" => Value::Boolean(!media?.paused()),
        "paused" => Value::Boolean(media?.paused()),
        _ if rp_comp_type(name) == "RDOM" => v_str(&el.get_attribute(prop)?),
        _ => return None,
    })
}

/// A web-only component's method (`None`: not one of these).
pub fn method(name: &str, comp_type: &str, method: &str, args: &[Value]) -> Option<Value> {
    let arg = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    let el = element(name);
    match (comp_type, method) {
        ("RWEBVIEW", "sethtml") => {
            if let Some(f) = el.as_ref().and_then(|e| e.dyn_ref::<web_sys::HtmlIFrameElement>()) {
                f.set_srcdoc(&arg(0));
            }
        }
        ("RWEBVIEW", "navigate") => {
            if let Some(f) = el.as_ref().and_then(|e| e.dyn_ref::<web_sys::HtmlIFrameElement>()) {
                f.set_src(&arg(0));
            }
        }
        ("RDOM", "create") => {}
        ("RDOM", "appendto") => {
            if let (Some(el), Some(parent)) = (el, element(&arg(0))) {
                let _ = el.remove_attribute("data-rr-overlay");
                let _ = el.style().remove_property("clip-path");
                for (p, d) in [("left", 0), ("top", 0), ("width", 100), ("height", 25)] {
                    let v = stored(name, p).map_or(d, |v| v.to_i64());
                    let _ = el.style().set_property(p, &format!("{v}px"));
                }
                let _ = el.style().set_property("display", "");
                let _ = parent.append_child(&el);
            }
        }
        ("RDOM", "setattribute") => {
            if let Some(el) = el {
                let _ = el.set_attribute(&arg(0), &arg(1));
            }
        }
        ("RDOM", "getattribute") => return Some(el.and_then(|e| e.get_attribute(&arg(0))).map_or_else(v_null, |v| v_str(&v))),
        ("RDOM", "addclass") => {
            if let Some(el) = el {
                let _ = el.class_list().add_1(&arg(0));
            }
        }
        ("RDOM", "removeclass") => {
            if let Some(el) = el {
                let _ = el.class_list().remove_1(&arg(0));
            }
        }
        ("RDOM", "toggleclass") => {
            if let Some(el) = el {
                let _ = el.class_list().toggle(&arg(0));
            }
        }
        ("RDOM", "remove") => {
            if let Some(el) = el {
                el.remove();
            }
        }
        ("RDOM", "queryselector") => {
            return Some(match document().query_selector(&arg(0)) {
                Ok(Some(found)) => v_str(&found.id()),
                _ => v_null(),
            })
        }
        ("RWEBAUDIO" | "RWEBVIDEO", "play" | "pause" | "stop" | "seek") => {
            if let Some(m) = el.as_ref().and_then(|e| e.dyn_ref::<HtmlMediaElement>()) {
                match method {
                    "play" => {
                        let _ = m.play();
                    }
                    "pause" => {
                        let _ = m.pause();
                    }
                    "stop" => {
                        let _ = m.pause();
                        m.set_current_time(0.0);
                    }
                    _ => m.set_current_time(args.first().map_or(0.0, Value::to_f64)),
                }
            }
        }
        ("RWEBVIDEO", "fullscreen") => {
            if let Some(el) = el {
                let _ = el.request_fullscreen();
            }
        }
        _ => return None,
    }
    Some(v_null())
}
