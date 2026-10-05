//! Accessibility in the page (docs/desktop-host-plan.md §6, Stage 12): what
//! a screen reader gets from a program's forms is what the desktop's UI
//! kernel gives AccessKit — the same nodes, from the same rules
//! (`rapidr_value::objects::a11y`: `describe`, `apply_name_rule`,
//! `label_for`, `tab_order`, `next_stop_after`) — mapped onto the elements
//! gui_web.rs makes:
//!
//! - native elements keep their own semantics (a `<button>` needs no
//!   role, an `<input type=checkbox>` no aria-checked, a `<select>` lists
//!   its options); the others get a role and the ARIA states the node has
//!   (aria-selected, aria-expanded, aria-level, aria-pressed for a toggle
//!   button, aria-valuenow / min / max, aria-disabled …). What the node
//!   says the element does too: a disabled panel's controls are disabled,
//!   a ReadOnly edit (its text model's) is read-only;
//! - the name: a caption the element shows names it; AccessibleName,
//!   a Hint or a caption it doesn't show become aria-label (a Hint through
//!   its title), the QLABEL naming a control its aria-labelledby;
//!   AccessibleDescription is aria-description, an unused Hint the title's
//!   description; a caption's `&` letter is aria-keyshortcuts;
//! - parts drawn on an SVG or a canvas get elements of their own: a tab
//!   control's tabs over its tabs, a header's sections and a list view's
//!   items as the canvas's fallback content; a tree's rows, a grid's cells
//!   and an owner-drawn list's rows are already elements;
//! - a status bar is role=status, a polite live region: what it says is
//!   announced when it changes. Nothing else is live: a label a timer
//!   updates every second would talk over everything else.
//!
//! The keyboard is the desktop's too ([`listen`]): Tab and Shift+Tab follow
//! TabOrder / TabStop within the form; Alt + a caption's `&` letter clicks
//! the button (check box …) or focuses what follows the label, else opens
//! the menu bar's item; Enter clicks the form's Default button, Escape its
//! Cancel one (which closes a modal form with its ModalResult); a memo with
//! WantTabs types a Tab; a focused component shows a dotted focus ring and
//! the Default button its dark frame.
//!
//! What changes a form (a property set, a part drawn again) marks it
//! ([`changed`]); the marked forms are brought up to date once the
//! program's code returns.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use rapidr_value::objects::a11y::{self, AccessNode, NameFrom, Orientation, Role};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::Rect;

use crate::gui_web::{comp_id, create_el, document, get_el};
use crate::object_web::{form_of, get_children_of, rp_comp_get, rp_comp_get_stored, rp_comp_type};
use crate::value::Value;

thread_local! {
    /// The forms to bring up to date.
    static DIRTY: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
    static QUEUED: Cell<bool> = const { Cell::new(false) };
    static LISTENING: Cell<bool> = const { Cell::new(false) };
}

/// `name` changed (a property, a part drawn again): its form's ARIA follows
/// once the program's code returns.
pub fn changed(name: &str) {
    // (Stage W3: with the kernel hosting, the mirror is the host's)
    #[cfg(feature = "kernel")]
    if crate::kernel_web::on() {
        return;
    }
    listen();
    let Some(form) = form_of(name) else { return };
    DIRTY.with(|d| d.borrow_mut().insert(form));
    if QUEUED.with(|q| q.replace(true)) {
        return;
    }
    let cb = Closure::once_into_js(|| {
        QUEUED.with(|q| q.set(false));
        for form in DIRTY.with(|d| std::mem::take(&mut *d.borrow_mut())) {
            sync_form(&form);
        }
    });
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 0);
    }
}

/// A caption shown as Windows shows it: `&&` an ampersand, the letter
/// after a lone `&` underlined (Alt + it activates the component) — text
/// nodes and a `<u>`, never markup from the caption.
pub fn caption_into(el: &web_sys::Element, caption: &str) {
    let (shown, mark) = a11y::mnemonic(caption);
    let chars: Vec<char> = shown.chars().collect();
    let part = |r: std::ops::Range<usize>| chars[r].iter().collect::<String>();
    // (the same caption again — a timer setting it — changes nothing)
    let underlined = el.query_selector(":scope > u").ok().flatten().and_then(|u| u.text_content());
    let before = el.first_child().filter(|c| c.node_type() == web_sys::Node::TEXT_NODE).and_then(|c| c.text_content()).unwrap_or_default();
    let same = el.text_content().as_deref() == Some(shown.as_str())
        && match mark {
            None => el.child_element_count() == 0,
            Some((at, _)) => el.child_element_count() == 1 && underlined.as_deref() == Some(part(at..at + 1).as_str()) && before == part(0..at),
        };
    if same {
        return;
    }
    el.set_text_content(None);
    let doc = document();
    let Some((at, _)) = mark else {
        el.set_text_content(Some(&shown));
        return;
    };
    let _ = el.append_child(&doc.create_text_node(&part(0..at)));
    if let Ok(u) = doc.create_element("u") {
        u.set_text_content(Some(&part(at..at + 1)));
        let _ = el.append_child(&u);
    }
    let _ = el.append_child(&doc.create_text_node(&part(at + 1..chars.len())));
}

// ------------------------------------------------------------- reading --

/// A property as the rules read it: what the program set, else what the
/// runtime answers (a model's, the element's).
fn prop(name: &str, p: &str) -> Value {
    match rp_comp_get_stored(name, p) {
        Value::Null => rp_comp_get(name, p),
        v => v,
    }
}

fn flag(name: &str, p: &str, default: bool) -> bool {
    a11y::flag(&|q: &str| rp_comp_get_stored(name, q), p, default)
}

/// Its Left, Top, Width, Height (RapidQ's default size when it has none),
/// as the kernel places it.
fn geometry(name: &str, t: &str) -> Rect {
    let (dw, dh) = rapidr_value::layout::default_size(t).unwrap_or((75, 25));
    let n = |p: &str, d: i64| match prop(name, p) {
        Value::Null => d,
        v => v.to_i64(),
    };
    (n("left", 0), n("top", 0), n("width", dw), n("height", dh))
}

/// The components on `parent` that have an element of their own (menus and
/// non-visual ones left out), in creation order.
fn placed_children(parent: &str) -> Vec<(String, String)> {
    get_children_of(parent)
        .into_iter()
        .map(|(n, t)| (n, t.to_ascii_uppercase()))
        .filter(|(n, t)| !matches!(t.as_str(), "RFORM" | "RMAINMENU" | "RPOPUPMENU" | "RMENUITEM") && get_el(&comp_id(n)).is_some())
        .collect()
}

/// The element ARIA goes on: a check box's / radio button's input (its
/// label wraps it), else the component's own.
fn semantic(name: &str, t: &str) -> Option<web_sys::HtmlElement> {
    match t {
        "RCHECKBOX" => get_el(&format!("{}-cb", comp_id(name))),
        "RRADIOBUTTON" => get_el(&format!("{}-rb", comp_id(name))),
        _ => get_el(&comp_id(name)),
    }
}

// ----------------------------------------------------------- the forms --

fn set(el: &web_sys::Element, attr: &str, value: Option<&str>) {
    match value {
        Some(v) if el.get_attribute(attr).as_deref() != Some(v) => {
            let _ = el.set_attribute(attr, v);
        }
        None if el.has_attribute(attr) => {
            let _ = el.remove_attribute(attr);
        }
        _ => {}
    }
}

fn yes_no(on: bool) -> &'static str {
    if on {
        "true"
    } else {
        "false"
    }
}

/// Form `form` brought up to date: the window, then its components.
fn sync_form(form: &str) {
    let Some(el) = get_el(&comp_id(form)) else { return };
    // (a form is a window: a dialog in ARIA's words, modal while ShowModal waits)
    set(&el, "role", Some("dialog"));
    set(&el, "aria-modal", crate::dialog_web::is_modal(&comp_id(form)).then_some("true"));
    let caption = prop(form, "caption").to_string_val();
    set(&el, "aria-label", Some(&caption).filter(|c| !c.is_empty()).map(String::as_str));
    let description = prop(form, "accessibledescription").to_string_val();
    set(&el, "aria-description", Some(&description).filter(|d| !d.is_empty()).map(String::as_str));
    title_buttons(&el);
    let enabled = flag(form, "enabled", true);
    walk(form, enabled);
    if let Some((main, _)) = get_children_of(form).into_iter().find(|(_, t)| t.eq_ignore_ascii_case("RMAINMENU")) {
        menu_bar(&main);
    }
}

/// The title bar's buttons named for what they do (they show symbols).
fn title_buttons(form: &web_sys::Element) {
    if let Ok(buttons) = form.query_selector_all(":scope > .rr-form-titlebar > button[title]") {
        for i in 0..buttons.length() {
            if let Some(b) = buttons.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                let title = b.get_attribute("title").unwrap_or_default();
                set(&b, "aria-label", Some(&title));
            }
        }
    }
}

/// The components on `parent` (shown ones), each described and its ARIA
/// set, then theirs.
fn walk(parent: &str, enabled: bool) {
    let shown: Vec<(String, String)> = placed_children(parent).into_iter().filter(|(n, _)| flag(n, "visible", true)).collect();
    let rects: Vec<(Rect, bool)> = shown.iter().map(|(n, t)| (geometry(n, t), t == "RLABEL")).collect();
    for (k, (name, t)) in shown.iter().enumerate() {
        let on = enabled && flag(name, "enabled", true);
        if described(t) {
            apply(name, t, on, || a11y::label_for(k, &rects).map(|l| shown[l].0.clone()));
        }
        walk(name, on);
    }
}

/// RapidQ's components, which the shared rules describe (the web's own —
/// an RDOM's HTML, a web view — say what their program makes them say).
fn described(t: &str) -> bool {
    matches!(
        t,
        "RLABEL" | "RBUTTON" | "RCOOLBTN" | "ROVALBTN" | "REDIT" | "RMEMO" | "RRICHEDIT" | "RCHECKBOX" | "RRADIOBUTTON" | "RPANEL" | "RGROUPBOX" | "RSCROLLBOX" | "RSPLITTER" | "RSTATUSBAR" | "RMDICHILD" | "RTRACKBAR" | "RTABCONTROL" | "RPROGRESS" | "RPROGRESSBAR" | "RUPDOWN" | "RLISTBOX" | "RFILELISTBOX" | "RCOMBOBOX" | "RLISTVIEW" | "RSTRINGGRID" | "RHEADER" | "RTREEVIEW" | "RDIRTREE" | "RCANVAS" | "RIMAGE" | "RBEVEL" | "RDIGDISPLAY" | "RGLASSFRAME"
    )
}

/// What a component's element can't say itself: its node's role, name,
/// states, value and parts.
fn apply(name: &str, t: &str, enabled: bool, label: impl FnOnce() -> Option<String>) {
    let Some(el) = semantic(name, t) else { return };
    let (_, _, w, h) = geometry(name, t);
    let font = if t == "RTABCONTROL" { rapidr_value::objects::font_from_props(name, &|id, p| rp_comp_get(id, p)) } else { Font::default() };
    let get = |p: &str| prop(name, p);
    let mut n = a11y::describe(name, t, &get, (w, h), &font);
    let mut label_name = None;
    let from = a11y::apply_name_rule(&mut n, &get, || {
        let l = label()?;
        let caption = prop(&l, "caption").to_string_val();
        label_name = Some(l.clone());
        Some((a11y::node_id(&l), caption))
    });
    // The role, where the element's own isn't it.
    let role = match t {
        "RPANEL" | "RBEVEL" | "RGLASSFRAME" | "RSCROLLBOX" => Some("group"),
        "RDIGDISPLAY" => Some("img"),
        "RMDICHILD" => Some("dialog"),
        "RTABCONTROL" => Some("tablist"),
        "RTRACKBAR" => Some("slider"),
        "RTREEVIEW" => Some("tree"),
        "RSTRINGGRID" => Some("grid"),
        "RHEADER" => Some("group"),
        "RSTATUSBAR" => Some("status"),
        "RSPLITTER" => Some("separator"),
        "RIMAGE" => Some("img"),
        "RLISTVIEW" => Some("listbox"),
        "RLISTBOX" | "RFILELISTBOX" if el.tag_name().eq_ignore_ascii_case("div") => Some("listbox"),
        "RCOMBOBOX" if el.tag_name().eq_ignore_ascii_case("div") => Some("combobox"),
        _ => None,
    };
    if role.is_some() || !matches!(el.get_attribute("role").as_deref(), None | Some("")) {
        set(&el, "role", role);
    }
    // The name: shown by the element itself (a button's caption, a check
    // box's label), else said. (A Hint is the title gui_web gives the
    // component's element, which names it — or describes it, when it has a
    // name — unless the ARIA goes on an element inside it: a check box's.)
    let shows_caption = matches!(t, "RBUTTON" | "RCOOLBTN" | "ROVALBTN" | "RCHECKBOX" | "RRADIOBUTTON" | "RLABEL" | "RGROUPBOX");
    let titled = get_el(&comp_id(name)).is_some_and(|c| c.is_same_node(Some(&el)));
    let said = match from {
        NameFrom::Given => Some(n.name.as_str()),
        NameFrom::Own if !shows_caption && !n.name.is_empty() => Some(n.name.as_str()),
        NameFrom::Hint if !titled => Some(n.name.as_str()),
        _ => None,
    };
    set(&el, "aria-label", said);
    set(&el, "aria-labelledby", label_name.as_deref().filter(|_| from == NameFrom::Label).map(comp_id).as_deref());
    let hint = prop(name, "hint").to_string_val();
    set(&el, "aria-description", Some(n.description.as_str()).filter(|d| !d.is_empty() && !(titled && *d == hint)));
    set(&el, "aria-keyshortcuts", n.shortcut.map(|c| format!("Alt+{}", c.to_ascii_uppercase())).as_deref());
    // States.
    let native = el.dyn_ref::<web_sys::HtmlInputElement>().is_some() || el.dyn_ref::<web_sys::HtmlButtonElement>().is_some() || el.dyn_ref::<web_sys::HtmlSelectElement>().is_some() || el.dyn_ref::<web_sys::HtmlTextAreaElement>().is_some();
    if native {
        // (a disabled container's controls take no input, as on the desktop)
        set(&el, "disabled", (!enabled).then_some(""));
    }
    set(&el, "aria-disabled", (!enabled && !native).then_some("true"));
    if matches!(t, "RCOOLBTN" | "ROVALBTN") {
        set(&el, "aria-pressed", n.states.checked.map(yes_no));
    }
    // (ReadOnly is the text model's: the element takes it from there)
    if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>().filter(|_| t == "REDIT") {
        input.set_read_only(n.states.read_only);
    } else if let Some(ta) = el.dyn_ref::<web_sys::HtmlTextAreaElement>().filter(|_| matches!(t, "RMEMO" | "RRICHEDIT")) {
        ta.set_read_only(n.states.read_only);
    }
    if let Some(num) = n.numeric.filter(|_| matches!(t, "RTRACKBAR" | "RPROGRESS" | "RPROGRESSBAR")) {
        set(&el, "aria-valuenow", Some(&num.value.to_string()));
        set(&el, "aria-valuemin", Some(&num.min.to_string()));
        set(&el, "aria-valuemax", Some(&num.max.to_string()));
    }
    set(&el, "aria-valuetext", n.value.as_deref().filter(|_| matches!(t, "RPROGRESS" | "RPROGRESSBAR")));
    set(&el, "aria-orientation", n.orientation.filter(|_| matches!(t, "RTRACKBAR" | "RSPLITTER")).map(|o| if o == Orientation::Vertical { "vertical" } else { "horizontal" }));
    // TabStop = False: not a Tab stop, a click still focuses it.
    if a11y::takes_focus(t) {
        if !flag(name, "tabstop", true) {
            set(&el, "tabindex", Some("-1"));
            set(&el, "data-rr-tabstop", Some("0"));
        } else if el.has_attribute("data-rr-tabstop") {
            set(&el, "tabindex", if native { None } else { Some("0") });
            set(&el, "data-rr-tabstop", None);
        }
    }
    // The form's Default button, framed.
    set(&el, "data-rr-default", (t == "RBUTTON" && flag(name, "default", false)).then_some(""));
    parts(name, t, &el, &n);
}

/// The parts that are elements: rows, items, cells, tabs, sections.
fn parts(name: &str, t: &str, el: &web_sys::HtmlElement, n: &AccessNode) {
    match t {
        "RLISTBOX" | "RFILELISTBOX" if el.tag_name().eq_ignore_ascii_case("div") => {
            each(el, ":scope > [data-item]", |row| {
                let i = row.get_attribute("data-item").and_then(|v| v.parse::<usize>().ok());
                let o = i.and_then(|i| n.children.get(i));
                set(row, "role", Some("option"));
                set(row, "aria-selected", o.and_then(|o| o.states.selected).map(yes_no));
                set(row, "aria-label", o.map(|o| o.name.as_str()));
            });
        }
        "RCOMBOBOX" if el.tag_name().eq_ignore_ascii_case("div") => {
            // (its list while it's dropped: in the page's body)
            let list = document().query_selector(&format!(".rr-grid-dropdown[data-for=\"{}\"]", name.to_uppercase())).ok().flatten();
            let list_id = format!("{}-list", comp_id(name));
            if let Some(list) = &list {
                set(list, "id", Some(&list_id));
                set(list, "role", Some("listbox"));
                each(list, "[data-item]", |row| {
                    let o = row.get_attribute("data-item").and_then(|v| v.parse::<usize>().ok()).and_then(|i| n.children.get(i));
                    set(row, "role", Some("option"));
                    set(row, "aria-selected", o.and_then(|o| o.states.selected).map(yes_no));
                    set(row, "aria-label", o.map(|o| o.name.as_str()));
                });
            }
            set(el, "aria-controls", list.as_ref().map(|_| list_id.as_str()));
            set(el, "aria-expanded", Some(yes_no(list.is_some())));
            // (what it shows is drawn: its text said, unseen)
            each(el, ":scope > canvas, :scope > div", |c| set(c, "aria-hidden", Some("true")));
            let said = match el.query_selector(":scope > .rr-a11y-text").ok().flatten() {
                Some(s) => s,
                None => {
                    let s = create_el("span");
                    s.set_class_name("rr-a11y-text");
                    let _ = s.set_attribute("style", "position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap;");
                    let _ = el.append_child(&s);
                    s.into()
                }
            };
            if said.text_content().as_deref() != n.value.as_deref() {
                said.set_text_content(n.value.as_deref());
            }
        }
        "RTREEVIEW" => {
            each(el, ":scope > [data-node]", |row| {
                let i = row.get_attribute("data-node").and_then(|v| v.parse::<usize>().ok());
                let item = i.and_then(|i| n.children.iter().find(|c| c.id == a11y::part_id(name, a11y::PART_ITEM, i)));
                set(row, "role", Some("treeitem"));
                set(row, "aria-level", item.and_then(|c| c.level).map(|l| l.to_string()).as_deref());
                set(row, "aria-expanded", item.and_then(|c| c.states.expanded).map(yes_no));
                set(row, "aria-selected", item.and_then(|c| c.states.selected).map(yes_no));
                set(row, "aria-label", item.map(|c| c.name.as_str()));
            });
            each(el, ".rr-tree-button, [data-node] canvas", |b| set(b, "aria-hidden", Some("true")));
        }
        "RSTRINGGRID" => {
            if let Some(table) = get_el(&format!("{}-table", comp_id(name))) {
                set(&table, "role", Some("presentation"));
            }
            each(el, "tr", |tr| set(tr, "role", Some("row")));
            each(el, "td[data-col][data-row]", |td| {
                let at = |a: &str| td.get_attribute(a).and_then(|v| v.parse::<usize>().ok());
                let cell = at("data-col").zip(at("data-row")).and_then(|(c, r)| n.children.get(r)?.children.get(c));
                set(td, "role", Some("gridcell"));
                set(td, "aria-selected", cell.and_then(|c| c.states.selected).map(yes_no));
            });
            // (a cell's buttons and what OnDrawCell drew over it: not its text)
            each(el, ".rr-grid-list, .rr-grid-ellipsis, td canvas", |b| set(b, "aria-hidden", Some("true")));
        }
        "RTABCONTROL" => overlay(el, "tab", &n.children.iter().filter(|c| c.role == Role::Tab).collect::<Vec<_>>()),
        "RHEADER" => fallback(el, "button", &n.children.iter().collect::<Vec<_>>()),
        "RLISTVIEW" => fallback(el, "option", &n.children.iter().collect::<Vec<_>>()),
        _ => {}
    }
}

/// Every element under `el` matching `selector`.
fn each(el: &web_sys::Element, selector: &str, mut f: impl FnMut(&web_sys::Element)) {
    if let Ok(list) = el.query_selector_all(selector) {
        for i in 0..list.length() {
            if let Some(e) = list.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                f(&e);
            }
        }
    }
}

/// Parts drawn on an SVG: an element per part over it, where it's drawn
/// (pointer events go through).
fn overlay(el: &web_sys::HtmlElement, role: &str, nodes: &[&AccessNode]) {
    let layer = match el.query_selector(":scope > .rr-a11y-parts").ok().flatten() {
        Some(l) => l,
        None => {
            let l = create_el("div");
            l.set_class_name("rr-a11y-parts");
            let _ = l.set_attribute("style", "position:absolute;left:0;top:0;width:100%;height:100%;pointer-events:none;");
            let _ = el.append_child(&l);
            l.into()
        }
    };
    part_elements(&layer, "div", role, nodes, true);
}

/// Parts drawn on a canvas: its fallback content (the elements in it,
/// which screen readers read and the page doesn't draw).
fn fallback(el: &web_sys::HtmlElement, role: &str, nodes: &[&AccessNode]) {
    part_elements(el, "div", role, nodes, false);
}

/// `container`'s children made the parts: one element per node, named,
/// selected or not (`placed`: at its bounds).
fn part_elements(container: &web_sys::Element, tag: &str, role: &str, nodes: &[&AccessNode], placed: bool) {
    let kids = container.children();
    while kids.length() > nodes.len() as u32 {
        if let Some(last) = kids.item(kids.length() - 1) {
            last.remove();
        }
    }
    while kids.length() < nodes.len() as u32 {
        let _ = container.append_child(&create_el(tag));
    }
    for (k, node) in nodes.iter().enumerate() {
        let Some(p) = kids.item(k as u32) else { continue };
        set(&p, "role", Some(role));
        set(&p, "aria-label", Some(&node.name));
        set(&p, "aria-selected", node.states.selected.map(yes_no));
        if placed {
            let (x, y, w, h) = node.bounds;
            let _ = p.set_attribute("style", &format!("position:absolute;left:{x}px;top:{y}px;width:{w}px;height:{h}px;"));
        }
    }
}

/// A QMAINMENU's bar: a menu bar of menu items, a submenu a menu.
fn menu_bar(main: &str) {
    let Some(nav) = get_el(&comp_id(main)) else { return };
    set(&nav, "role", Some("menubar"));
    let items = a11y::menu_bar(main);
    let tops: Vec<web_sys::Element> = {
        let mut v = Vec::new();
        each(&nav, ":scope > .rr-menu-item-top", |e| v.push(e.clone()));
        v
    };
    for (top, item) in tops.iter().zip(&items) {
        set(top, "role", Some("menuitem"));
        set(top, "aria-disabled", item.states.disabled.then_some("true"));
        set(top, "aria-haspopup", item.states.expanded.map(|_| "menu"));
        let open = top.matches(":hover").unwrap_or(false) || top.class_list().contains("rr-menu-open");
        set(top, "aria-expanded", item.states.expanded.map(|_| yes_no(open)));
        set(top, "aria-keyshortcuts", item.shortcut.map(|c| format!("Alt+{}", c.to_ascii_uppercase())).as_deref());
    }
    each(&nav, ".rr-dropdown-menu", |m| set(m, "role", Some("menu")));
    each(&nav, ".rr-menu-sep", |s| set(s, "role", Some("separator")));
    each(&nav, ".rr-menu-item-sub", |e| {
        let Some(id) = e.get_attribute("data-rr-name") else { return };
        let Some((checked, radio, enabled)) = rapidr_value::objects::menu::with(&id, |m| (m.checked, m.radio, m.enabled)) else { return };
        let checkable = checked || radio;
        set(e, "role", Some(if radio { "menuitemradio" } else if checkable { "menuitemcheckbox" } else { "menuitem" }));
        set(e, "aria-checked", checkable.then_some(yes_no(checked)));
        set(e, "aria-disabled", (!enabled).then_some("true"));
    });
    each(&nav, ".rr-menu-mark, .rr-menu-keys", |m| set(m, "aria-hidden", Some("true")));
}

// ----------------------------------------------------------- the keys --

/// The page's keys for the forms and its focus ring, once.
fn listen() {
    if LISTENING.with(|l| l.replace(true)) {
        return;
    }
    let doc = document();
    let style = create_el("style");
    // (the focus ring where the component draws none of its own; the
    // Default button's dark frame)
    style.set_text_content(Some(
        ".rr-form :is(button, select, [tabindex]:not(.rr-form):not(.rr-tabcontrol), input[type=checkbox], input[type=radio]):focus-visible { outline: 1px dotted #000 !important; outline-offset: -4px !important; }\n\
         .rr-form button[data-rr-default] { box-shadow: 0 0 0 1px #000; }\n\
         .rr-form .rr-a11y-parts > * { pointer-events: none; }",
    ));
    if let Some(head) = doc.query_selector("head").ok().flatten() {
        let _ = head.append_child(&style);
    }
    let keys = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(|e: web_sys::KeyboardEvent| {
        if !e.default_prevented() && key(&e) {
            e.prevent_default();
        }
    });
    let _ = doc.add_event_listener_with_callback("keydown", keys.as_ref().unchecked_ref());
    keys.forget();
}

/// The form a key goes to: the one holding the focus, else (nothing has
/// it) the frontmost shown one; none for the page's other elements.
fn form_for(target: Option<&web_sys::Element>) -> Option<(String, web_sys::Element)> {
    if let Some(f) = target.and_then(|t| t.closest(".rr-form[data-rr-name]").ok().flatten()) {
        return Some((f.get_attribute("data-rr-name")?, f));
    }
    let doc = document();
    let unfocused = target.is_none_or(|t| doc.body().is_some_and(|b| b.is_same_node(Some(t))) || doc.document_element().is_some_and(|d| d.is_same_node(Some(t))));
    if !unfocused {
        return None;
    }
    let forms = document().query_selector_all(".rr-form[data-rr-name]").ok()?;
    (0..forms.length())
        .filter_map(|i| forms.item(i)?.dyn_into::<web_sys::HtmlElement>().ok())
        .filter(|f| f.offset_width() > 0 && f.parent_element().is_some_and(|p| p.closest(".rr-form").ok().flatten().is_none()))
        .max_by_key(|f| f.style().get_property_value("z-index").ok().and_then(|z| z.parse::<i64>().ok()).unwrap_or(0))
        .and_then(|f| Some((f.get_attribute("data-rr-name")?, f.into())))
}

/// The form's components, depth first in creation order, with whether
/// each is shown and enabled (with its parents).
fn components(form: &str) -> Vec<(String, String, bool)> {
    fn walk(parent: &str, on: bool, out: &mut Vec<(String, String, bool)>) {
        for (n, t) in placed_children(parent) {
            let here = on && flag(&n, "visible", true) && flag(&n, "enabled", true);
            out.push((n.clone(), t, here));
            walk(&n, here, out);
        }
    }
    let mut out = Vec::new();
    walk(form, flag(form, "enabled", true), &mut out);
    out
}

/// The form as Tab walks it.
struct Tabs(String);

impl a11y::TabTree for Tabs {
    type Id = String;

    fn children(&self, parent: Option<&String>) -> Vec<String> {
        placed_children(parent.unwrap_or(&self.0)).into_iter().map(|(n, _)| n).collect()
    }

    fn tab_order_of(&self, c: &String) -> Option<i64> {
        match rp_comp_get_stored(c, "taborder") {
            Value::Null => None,
            v => Some(v.to_i64()),
        }
    }

    fn active(&self, c: &String) -> bool {
        flag(c, "visible", true) && flag(c, "enabled", true)
    }

    fn stops(&self, c: &String) -> bool {
        a11y::takes_focus(&rp_comp_type(c)) && flag(c, "tabstop", true)
    }
}

/// The component an element is (or is in), on `form`.
fn component_of(target: &web_sys::Element) -> Option<String> {
    let el = target.closest("[data-rr-name]").ok().flatten()?;
    el.get_attribute("data-rr-name").map(|n| n.to_uppercase())
}

fn focus(name: &str) {
    if let Some(el) = semantic(name, &rp_comp_type(name)) {
        let _ = el.focus();
    }
}

/// A key the forms take (Tab, Alt + a letter, Enter, Escape): whether it
/// was theirs.
fn key(e: &web_sys::KeyboardEvent) -> bool {
    let target = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
    let Some((form, form_el)) = form_for(target.as_ref()) else { return false };
    if e.meta_key() {
        return false;
    }
    let me = target.as_ref().and_then(component_of);
    match e.key().as_str() {
        "Tab" if !e.ctrl_key() && !e.alt_key() => {
            // A memo with WantTabs types it.
            if let (Some(ta), Some(name)) = (target.as_ref().and_then(|t| t.dyn_ref::<web_sys::HtmlTextAreaElement>()), me.as_ref()) {
                if !e.shift_key() && flag(name, "wanttabs", false) {
                    let start = ta.selection_start().ok().flatten().unwrap_or(0);
                    let end = ta.selection_end().ok().flatten().unwrap_or(start);
                    let _ = ta.set_range_text_with_start_and_end("\t", start, end);
                    let _ = ta.set_selection_range(start + 1, start + 1);
                    if let Ok(ev) = web_sys::Event::new("input") {
                        let _ = ta.dispatch_event(&ev);
                    }
                    return true;
                }
            }
            let order = a11y::tab_order(&Tabs(form.to_uppercase()));
            if order.is_empty() {
                return true;
            }
            let n = order.len();
            let at = me.as_ref().and_then(|m| order.iter().position(|o| o == m));
            let next = match at {
                Some(k) if e.shift_key() => (k + n - 1) % n,
                Some(k) => (k + 1) % n,
                None if e.shift_key() => n - 1,
                None => 0,
            };
            focus(&order[next]);
            true
        }
        _ if e.alt_key() && !e.ctrl_key() => {
            let code = e.code();
            let Some(letter) = code.strip_prefix("Key").filter(|l| l.len() == 1).and_then(|l| l.chars().next()).map(|c| c.to_ascii_lowercase()) else { return false };
            mnemonic(&form, &form_el, letter)
        }
        "Enter" | "Escape" if !e.ctrl_key() && !e.alt_key() && !e.shift_key() => {
            // (only when the key is on a component itself — an in-place
            // editor, a button, a memo answer it themselves)
            let Some(t) = target.as_ref() else { return false };
            let own = me.as_ref().and_then(|m| semantic(m, &rp_comp_type(m))).is_some_and(|s| s.is_same_node(Some(t))) || t.is_same_node(Some(&form_el));
            if !own || matches!(t.tag_name().as_str(), "BUTTON" | "TEXTAREA") {
                return false;
            }
            let prop = if e.key() == "Enter" { "default" } else { "cancel" };
            let button = components(&form).into_iter().find(|(n, t, on)| *on && t == "RBUTTON" && flag(n, prop, false));
            match button.and_then(|(n, _, _)| get_el(&comp_id(&n))) {
                Some(b) => {
                    b.click();
                    true
                }
                None => false,
            }
        }
        _ => false,
    }
}

/// Alt + `letter` on `form`: the component whose caption marks it clicked
/// (focused first), or the one after the label marking it focused, else
/// the menu bar's item opened (or picked).
fn mnemonic(form: &str, form_el: &web_sys::Element, letter: char) -> bool {
    let all = components(form);
    let found = all.iter().position(|(n, t, on)| *on && a11y::mnemonic_of(t, &|p| prop(n, p)) == Some(letter));
    if let Some(k) = found {
        let (name, t, _) = &all[k];
        if a11y::mnemonic_clicks(t) {
            if a11y::takes_focus(t) {
                focus(name);
            }
            if let Some(el) = semantic(name, t) {
                el.click();
            }
        } else if let Some(next) = a11y::next_stop_after(&Tabs(form.to_uppercase()), name) {
            // (a label: the next Tab stop after it)
            focus(&next);
        }
        return true;
    }
    // The menu bar's item.
    let Ok(tops) = form_el.query_selector_all(":scope nav[data-rr-type=\"RMAINMENU\"] > .rr-menu-item-top") else { return false };
    for i in 0..tops.length() {
        let Some(top) = tops.item(i).and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok()) else { continue };
        let Some(id) = top.get_attribute("data-rr-name") else { continue };
        let caption = rapidr_value::objects::menu::with(&id, |m| m.caption.clone()).unwrap_or_default();
        if a11y::mnemonic(&caption).1.map(|m| m.1) != Some(letter) {
            continue;
        }
        if rapidr_value::objects::menu::children(&id).is_empty() {
            top.click();
        } else {
            open_menu(&top);
        }
        return true;
    }
    false
}

/// A menu bar's item opened from the keyboard: its list shown, its first
/// item focused; the arrows move, Enter picks, Escape (or the focus
/// leaving) closes it.
fn open_menu(top: &web_sys::HtmlElement) {
    let Some(list) = top.query_selector(":scope > .rr-dropdown-menu").ok().flatten().and_then(|l| l.dyn_into::<web_sys::HtmlElement>().ok()) else { return };
    let _ = top.class_list().add_1("rr-menu-open");
    let _ = list.style().set_property("display", "block");
    set(top, "aria-expanded", Some("true"));
    if let Some(first) = menu_items(&list).first() {
        let _ = first.focus();
    }
    // (the list's keys, once per list: the bar draws new ones when it changes)
    if list.has_attribute("data-rr-keys") {
        return;
    }
    let _ = list.set_attribute("data-rr-keys", "");
    let keys = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        let Some(list) = e.current_target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) else { return };
        let all = menu_items(&list);
        let at = document().active_element().and_then(|a| all.iter().position(|i| i.is_same_node(Some(&a))));
        let n = all.len();
        match e.key().as_str() {
            "ArrowDown" | "ArrowUp" if n > 0 => {
                let k = match (at, e.key() == "ArrowDown") {
                    (Some(k), true) => (k + 1) % n,
                    (Some(k), false) => (k + n - 1) % n,
                    (None, _) => 0,
                };
                let _ = all[k].focus();
            }
            "Enter" | " " => {
                close_menu(&list);
                if let Some(k) = at {
                    all[k].click();
                }
            }
            "Escape" => close_menu(&list),
            _ => return,
        }
        e.prevent_default();
        e.stop_propagation();
    });
    let _ = list.add_event_listener_with_callback("keydown", keys.as_ref().unchecked_ref());
    keys.forget();
    let out = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
        let Some(list) = e.current_target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) else { return };
        let into = js_sys::Reflect::get(&e, &"relatedTarget".into()).ok().and_then(|t| t.dyn_into::<web_sys::Node>().ok());
        if !into.is_some_and(|n| list.contains(Some(&n))) {
            close_menu(&list);
        }
    });
    let _ = list.add_event_listener_with_callback("focusout", out.as_ref().unchecked_ref());
    out.forget();
}

/// An open menu's items the keys go through (enabled ones).
fn menu_items(list: &web_sys::HtmlElement) -> Vec<web_sys::HtmlElement> {
    let mut v = Vec::new();
    each(list, ":scope > .rr-menu-item-sub:not(.rr-menu-disabled)", |e| {
        if let Ok(h) = e.clone().dyn_into::<web_sys::HtmlElement>() {
            set(&h, "tabindex", Some("-1"));
            v.push(h);
        }
    });
    v
}

/// A menu opened from the keyboard closed again.
fn close_menu(list: &web_sys::HtmlElement) {
    let _ = list.style().remove_property("display");
    if let Some(top) = list.parent_element() {
        let _ = top.class_list().remove_1("rr-menu-open");
        set(&top, "aria-expanded", Some("false"));
    }
}
