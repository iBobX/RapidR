//! A window's accessibility mirror on the page (docs/web-host-plan.md
//! §3.5), in Rust (the spike's page did it in JavaScript): the kernel's
//! `AccessNode` tree — the one AccessKit gets on the desktop — as invisible
//! elements over the client canvas ([`crate::aria`] says which element and
//! which ARIA each node gets), patched after each frame (ids are the nodes'
//! stable ones, so a frame changes few elements). The DOM focus is kept on
//! the kernel's focused node in the active window, so a screen reader reads
//! what the kernel focused, and the keyboard, input methods and a phone's
//! keyboard type into the edit's real `<input>` / `<textarea>`, whose text
//! and selection are the kernel's.

use std::collections::HashMap;

use rapidr_value::objects::a11y::AccessNode;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement};

use crate::aria::{self, Spec};

/// A component's node: its name, and what a text field tells autofill and
/// password managers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hint {
    /// The `autocomplete` token ("" none: `off`).
    pub autocomplete: String,
    /// The component's name: a field's `name`; every component's element
    /// gets `data-rr-name` and the id `rr-<name>` (lowercase), the ids the
    /// DOM runtime gave its elements — scripts (RJAVASCRIPT) and tests find
    /// a component's element by them.
    pub name: String,
}

pub struct Mirror {
    root: HtmlElement,
    doc: Document,
    form: String,
    nodes: HashMap<u64, HtmlElement>,
    /// Each node's place in the window (for an input method's window).
    abs: HashMap<u64, (i64, i64)>,
    /// What the mirror last put in each text field (autofill is told apart
    /// from it).
    values: HashMap<u64, String>,
    focused: Option<u64>,
    focusing: bool,
    composing: bool,
}

fn px(v: i64) -> String {
    format!("{v}px")
}

impl Mirror {
    pub fn new(doc: &Document, form: &str) -> Option<Mirror> {
        let root: HtmlElement = doc.create_element("div").ok()?.dyn_into().ok()?;
        root.set_class_name("rr-a11y");
        Some(Mirror { root, doc: doc.clone(), form: form.to_string(), nodes: HashMap::new(), abs: HashMap::new(), values: HashMap::new(), focused: None, focusing: false, composing: false })
    }

    pub fn root(&self) -> &HtmlElement {
        &self.root
    }

    pub fn focusing(&self) -> bool {
        self.focusing
    }

    pub fn composing(&self) -> bool {
        self.composing
    }

    pub fn focused_node(&self) -> Option<u64> {
        self.focused
    }

    /// What the mirror last put in text field `field`.
    pub fn last_value(&self, field: &web_sys::EventTarget) -> Option<String> {
        let el = field.dyn_ref::<HtmlElement>()?;
        let id = el.dataset().get("node")?.parse::<u64>().ok()?;
        self.values.get(&id).cloned()
    }

    fn element(&mut self, s: &Spec) -> Option<HtmlElement> {
        if let Some(el) = self.nodes.get(&s.id) {
            if el.tag_name().eq_ignore_ascii_case(s.tag) {
                return Some(el.clone());
            }
            el.remove();
            self.nodes.remove(&s.id);
        }
        let el: HtmlElement = self.doc.create_element(s.tag).ok()?.dyn_into().ok()?;
        el.set_id(&format!("rrn-{}-{}", self.form, s.id));
        let _ = el.set_attribute("data-node", &s.id.to_string());
        if s.tag == "input" {
            let _ = el.set_attribute("type", "text");
        }
        if s.tag == "input" || s.tag == "textarea" {
            el.set_spellcheck(false);
            let _ = el.set_attribute("autocomplete", "off");
            let _ = el.set_attribute("autocapitalize", "off");
        }
        self.nodes.insert(s.id, el.clone());
        Some(el)
    }

    /// The mirror made what `tree` says; the DOM focus on `focused` (the
    /// focused component, if any) when the window is `active`.
    pub fn sync(&mut self, tree: &AccessNode, focused: Option<&str>, active: bool, hints: &HashMap<u64, Hint>) {
        let specs = aria::specs(tree);
        let mut seen = std::collections::HashSet::new();
        let mut focus_el: Option<(u64, HtmlElement)> = None;
        let mut first: Option<HtmlElement> = None;
        self.abs.clear();
        // (each element's place in its parent element)
        let mut rel: HashMap<u64, (i64, i64)> = HashMap::new();
        for s in &specs {
            seen.insert(s.id);
            let Some(el) = self.element(s) else { continue };
            let parent = s.parent.and_then(|p| self.nodes.get(&p).cloned()).unwrap_or_else(|| self.root.clone());
            // (a text field holds no elements: a combo box's list goes beside it)
            let field = matches!(parent.tag_name().as_str(), "INPUT" | "TEXTAREA");
            let (parent, shift) = if field {
                // (placed as the field's parent places it: the field's own place added)
                let at = s.parent.and_then(|p| rel.get(&p).copied()).unwrap_or((0, 0));
                (parent.parent_element().and_then(|e| e.dyn_into::<HtmlElement>().ok()).unwrap_or_else(|| self.root.clone()), at)
            } else {
                (parent, (0, 0))
            };
            if el.parent_node().as_ref() != Some(parent.as_ref()) {
                let _ = parent.append_child(&el);
            }
            // (the role's, the name's, the states' attributes)
            let attrs = el.attributes();
            let mut stale = Vec::new();
            for i in 0..attrs.length() {
                if let Some(a) = attrs.item(i) {
                    let n = a.name();
                    if (n == "role" || n.starts_with("aria-")) && !s.attrs.iter().any(|(k, _)| *k == n) {
                        stale.push(n);
                    }
                }
            }
            for n in stale {
                let _ = el.remove_attribute(&n);
            }
            for (k, v) in &s.attrs {
                if el.get_attribute(k).as_deref() != Some(v.as_str()) {
                    let _ = el.set_attribute(k, v);
                }
            }
            if s.focusable || s.parent.is_none() {
                el.set_tab_index(-1);
            } else {
                let _ = el.remove_attribute("tabindex");
            }
            if let Some(h) = hints.get(&s.id) {
                let id = format!("rr-{}", h.name.to_lowercase());
                if el.id() != id {
                    el.set_id(&id);
                    let _ = el.set_attribute("data-rr-name", &h.name);
                }
            }
            if s.tag == "div" && el.child_element_count() == 0 && el.text_content().unwrap_or_default() != s.text {
                el.set_text_content(Some(&s.text));
            }
            // (autofill and password managers: a text field's hints, from
            // its component's AutoComplete — "username", "email",
            // "current-password" …; without one the browser offers nothing)
            if s.tag == "input" || s.tag == "textarea" {
                let (auto, name) = hints.get(&s.id).map_or(("off", ""), |h| (if h.autocomplete.is_empty() { "off" } else { h.autocomplete.as_str() }, h.name.as_str()));
                if el.get_attribute("autocomplete").as_deref() != Some(auto) {
                    let _ = el.set_attribute("autocomplete", auto);
                }
                if !name.is_empty() && el.get_attribute("name").as_deref() != Some(name) {
                    let _ = el.set_attribute("name", name);
                }
            }
            if !self.composing {
                let field_value = |el: &HtmlElement| el.dyn_ref::<web_sys::HtmlInputElement>().map(|i| i.value()).or_else(|| el.dyn_ref::<web_sys::HtmlTextAreaElement>().map(|t| t.value()));
                if let Some(v) = field_value(&el) {
                    if v != s.value {
                        set_value(&el, &s.value);
                    }
                    self.values.insert(s.id, s.value.clone());
                }
            }
            let (x, y, w, h) = (s.bounds.0 + shift.0, s.bounds.1 + shift.1, s.bounds.2, s.bounds.3);
            rel.insert(s.id, (x, y));
            let pabs = s.parent.and_then(|p| self.abs.get(&p).copied()).unwrap_or((0, 0));
            self.abs.insert(s.id, (pabs.0 + s.bounds.0, pabs.1 + s.bounds.1));
            if !(self.composing && Some(s.id) == self.focused) {
                let st = el.style();
                for (k, v) in [("left", px(x)), ("top", px(y)), ("width", px(w)), ("height", px(h))] {
                    if st.get_property_value(k).ok().as_deref() != Some(v.as_str()) {
                        let _ = st.set_property(k, &v);
                    }
                }
            }
            if s.focused {
                focus_el = Some((s.id, el.clone()));
            }
            if first.is_none() {
                first = Some(el.clone());
            }
        }
        let stale: Vec<u64> = self.nodes.keys().filter(|k| !seen.contains(k)).copied().collect();
        for k in stale {
            if let Some(el) = self.nodes.remove(&k) {
                el.remove();
            }
            self.values.remove(&k);
        }
        self.focused = focus_el.as_ref().map(|(id, _)| *id);
        if !active {
            return;
        }
        // (the DOM focus where the kernel's is; the window's own node when
        // nothing in it has the focus, so its keys reach it)
        let Some(target) = focus_el.map(|(_, el)| el).or(first) else { return };
        let has = self.doc.active_element().is_some_and(|a| a == *target.as_ref());
        if !has {
            self.focusing = true;
            let opts = web_sys::FocusOptions::new();
            opts.set_prevent_scroll(true);
            let _ = target.focus_with_options(&opts);
            self.focusing = false;
        }
        // (a text field's selection as the kernel's, so an input method
        // starts where the kernel's caret is)
        if self.composing {
            return;
        }
        let Some(comp) = focused else { return };
        let Some((raw, s, l)) = rapidr_value::objects::with_textedit(comp, |t| (t.raw(), t.sel_start, t.sel_len)) else { return };
        let utf16 = |chars: usize| raw.chars().take(chars).map(char::len_utf16).sum::<usize>() as u32;
        let (start, end) = (utf16(s), utf16(s + l));
        if let Some(i) = target.dyn_ref::<web_sys::HtmlInputElement>() {
            if i.value() != raw {
                i.set_value(&raw);
            }
            if i.selection_start().ok().flatten() != Some(start) || i.selection_end().ok().flatten() != Some(end) {
                let _ = i.set_selection_range(start, end);
            }
        } else if let Some(t) = target.dyn_ref::<web_sys::HtmlTextAreaElement>() {
            if t.value() != raw {
                t.set_value(&raw);
            }
            if t.selection_start().ok().flatten() != Some(start) || t.selection_end().ok().flatten() != Some(end) {
                let _ = t.set_selection_range(start, end);
            }
        }
    }

    /// An input method's composition starts: the focused field moves to the
    /// kernel's caret (`area`, in the window's inside), emptied, so the
    /// input method's window opens there — whatever the field scrolled.
    /// The kernel keeps the text; the field gets it back at the end.
    pub fn compose_start(&mut self, area: Option<(i64, i64, i64, i64)>) {
        self.composing = true;
        let (Some(id), Some((x, y, _w, h))) = (self.focused, area) else { return };
        let Some(el) = self.nodes.get(&id) else { return };
        let parent = self.abs.get(&id).zip(el.parent_element()).map(|(abs, _)| *abs);
        let Some((ax, ay)) = parent else { return };
        let (bx, by) = self.bounds_origin(id, (ax, ay));
        let st = el.style();
        let _ = st.set_property("left", &px(x - bx));
        let _ = st.set_property("top", &px(y - by));
        let _ = st.set_property("height", &px(h.max(1) + 2));
        let _ = st.set_property("width", &px(200));
        set_value(el, "");
    }

    /// Where node `id`'s parent element's (0, 0) is in the window.
    fn bounds_origin(&self, id: u64, abs: (i64, i64)) -> (i64, i64) {
        let Some(el) = self.nodes.get(&id) else { return (0, 0) };
        let left = el.style().get_property_value("left").ok().and_then(|v| v.trim_end_matches("px").parse::<i64>().ok()).unwrap_or(0);
        let top = el.style().get_property_value("top").ok().and_then(|v| v.trim_end_matches("px").parse::<i64>().ok()).unwrap_or(0);
        (abs.0 - left, abs.1 - top)
    }

    /// The composition ended: the field goes back (the next sync).
    pub fn compose_end(&mut self) {
        self.composing = false;
    }
}

fn set_value(el: &HtmlElement, v: &str) {
    if let Some(i) = el.dyn_ref::<web_sys::HtmlInputElement>() {
        i.set_value(v);
    } else if let Some(t) = el.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        t.set_value(v);
    }
}
