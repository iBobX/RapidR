//! RDOCKMANAGER through a runtime — the same for the desktop and the web
//! (as `crate::mdi`'s `Runtime`): the methods, properties and the user's
//! actions carried out on the model ([`super::manager`]), then the result
//! shown by placing components:
//!
//! - each docked group is an `RDOCKGROUP` component (`<dock>__g<i>`) on
//!   the dock manager, which the kernel draws (header, tabs, buttons) and
//!   whose shown pane's component sits inside it;
//! - the document area is an `RDOCKDOCS` (`<dock>__docs`): an MDI client
//!   (`crate::mdi`, its child frames `RMDICHILD`s) or the tabbed
//!   documents' page;
//! - an auto-hidden pane slid out is an `RDOCKGROUP` over the others
//!   (`<dock>__fly`);
//! - a floating group is a window of its own (an `RFORM`, `<dock>__f<i>`:
//!   a top-level window on the desktop, a kernel window on the web) with
//!   an `RDOCKGROUP` filling it (`<dock>__fg<i>`).
//!
//! Then the program's events: OnPaneChange (Name), OnDocumentActivate
//! (Name), OnDocumentClose (Name, Cancel) — Cancel set keeps the document
//! — and OnLayoutChange.

use std::cell::Cell;

use super::geometry::{self, Slot};
use super::manager::{self, docs_name, parse_where, Event, Outcome, User};
use super::{DocumentMode, Rect, Side, Target};
use crate::mdi;
use crate::objects::font::Font;
use crate::Value;

/// What runs once a handler has run, with its arguments as it left them.
pub type Then = Box<dyn FnOnce(&[Value])>;

/// What a runtime provides to a dock manager.
pub trait Runtime: Copy + 'static {
    fn get(self, name: &str, prop: &str) -> Value;
    fn set(self, name: &str, prop: &str, v: Value);
    /// Makes a component of `type_name` (a group, a floating window).
    fn create(self, name: &str, type_name: &str);
    fn exists(self, name: &str) -> bool;
    fn type_of(self, name: &str) -> String;
    fn fire(self, name: &str, event: &str, args: &[Value]);
    /// Fires, then runs `then` with the arguments as the handler left them.
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Then);
    /// The components parented to `name`: (name, type), in creation order.
    fn children(self, name: &str) -> Vec<(String, String)>;
    /// Components were made or moved to other parents: the windows' trees
    /// are built again.
    fn restructure(self);
    /// `name.SetFocus`.
    fn focus(self, name: &str);
    /// `form.Show` (a floating window's first showing: built, shown).
    fn show(self, form: &str);
}

thread_local! {
    /// Placing components sets their properties: those sets mustn't place
    /// them again.
    static APPLYING: Cell<bool> = const { Cell::new(false) };
}

/// The component of a group slot.
pub fn group_name(dock: &str, slot: Slot) -> String {
    format!("{}__{}", dock.to_ascii_lowercase(), slot.suffix())
}

/// A floating window's form.
pub fn float_form(dock: &str, i: usize) -> String {
    format!("{}__f{i}", dock.to_ascii_lowercase())
}

/// The dock manager and slot of a group component (`<dock>__g0` …).
pub fn group_of(name: &str) -> Option<(String, Slot)> {
    let lower = name.to_ascii_lowercase();
    let (dock, slot) = lower.rsplit_once("__")?;
    let slot = Slot::parse(slot)?;
    manager::exists(dock).then(|| (dock.to_string(), slot))
}

/// The dock manager and index of a floating window's form.
fn float_of(name: &str) -> Option<(String, usize)> {
    let lower = name.to_ascii_lowercase();
    let (dock, f) = lower.rsplit_once("__f")?;
    let i = f.parse().ok()?;
    manager::exists(dock).then(|| (dock.to_string(), i))
}

fn int(v: i64) -> Value {
    Value::Integer(v)
}

fn font_of<R: Runtime>(rt: R, dock: &str) -> Font {
    crate::objects::font_from_props(dock, &|i, p| rt.get(i, p))
}

/// The pane's component as the program named it.
fn given(dock: &str, pane: &str) -> String {
    manager::with(dock, |m| m.pane(pane).map(|p| p.given.clone())).flatten().unwrap_or_else(|| pane.to_string())
}

/// A component argument: its name, or its Handle's component.
fn component_arg(v: &Value) -> String {
    match v {
        Value::Integer(h) => crate::handles::name_of(*h).unwrap_or_default(),
        Value::Double(h) => crate::handles::name_of(*h as i64).unwrap_or_default(),
        v => v.to_string_val(),
    }
}

/// Where the dock manager's corner is on the screen (its window's corner,
/// a frame's width and a title bar's height in, its parents' places).
fn screen_origin<R: Runtime>(rt: R, dock: &str) -> (i64, i64) {
    let (mut x, mut y) = (0, 0);
    let mut at = dock.to_string();
    for _ in 0..64 {
        x += rt.get(&at, "left").to_i64();
        y += rt.get(&at, "top").to_i64();
        if rt.type_of(&at).eq_ignore_ascii_case("RFORM") {
            return (x + 8, y + 31);
        }
        let parent = rt.get(&at, "parent").to_string_val();
        if parent.is_empty() {
            break;
        }
        at = parent;
    }
    (x, y)
}

// ------------------------------------------------------- the methods --

/// An RDOCKMANAGER method; `None` if `method` isn't one of its own.
pub fn rt_method<R: Runtime>(rt: R, dock: &str, method: &str, args: &[Value]) -> Option<Value> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    let s = |i: usize| arg(i).to_string_val();
    let name = |i: usize| component_arg(&arg(i)).to_ascii_lowercase();
    let method = method.to_ascii_lowercase();
    sync(rt, dock);
    let out = match method.as_str() {
        "addpane" => {
            let comp = component_arg(&arg(0));
            manager::with_mut(dock, |m| m.add_pane(&comp, &s(1), &s(2), &s(3)))
        }
        "showpane" => manager::with_mut(dock, |m| m.show_pane(&name(0))),
        "hidepane" => manager::with_mut(dock, |m| m.hide_pane(&name(0))),
        "floatpane" => manager::with_mut(dock, |m| m.float_pane(&name(0), None)),
        "autohide" => {
            let on = args.len() < 2 || arg(1).to_bool();
            manager::with_mut(dock, |m| m.auto_hide(&name(0), on))
        }
        "focuspane" => manager::with_mut(dock, |m| m.focus_pane(&name(0))),
        "closepane" => manager::with_mut(dock, |m| m.close_pane(&name(0))),
        "dockpane" => match parse_where(&s(1)) {
            Some(Target::Float(_)) => manager::with_mut(dock, |m| m.float_pane(&name(0), None)),
            Some(t) => manager::with_mut(dock, |m| m.dock_pane(&name(0), t)),
            None => Outcome::default(),
        },
        "movepane" => {
            let ok = manager::with_mut(dock, |m| m.begin_move(&name(0)));
            rt.restructure();
            return Some(int(if ok { -1 } else { 0 }));
        }
        "savelayout" => return Some(Value::String(manager::with_mut(dock, |m| m.save()))),
        "loadlayout" => manager::with_mut(dock, |m| m.load(&s(0))),
        "resetlayout" => manager::with_mut(dock, |m| m.reset()),
        "pane" => {
            let i = arg(0).to_i64();
            return Some(Value::String(manager::with(dock, |m| (i >= 0).then(|| m.panes.get(i as usize).map(|p| p.given.clone())).flatten()).flatten().unwrap_or_default()));
        }
        "document" => {
            let i = arg(0).to_i64();
            let d = manager::with(dock, |m| (i >= 0).then(|| m.layout.documents.get(i as usize).cloned()).flatten()).flatten();
            return Some(Value::String(d.map(|d| given(dock, &d)).unwrap_or_default()));
        }
        "panetitle" => {
            if args.len() >= 2 {
                let (p, t) = (name(0), s(1));
                manager::with_mut(dock, |m| {
                    if let Some(info) = m.panes.iter_mut().find(|i| i.name == p) {
                        info.title = t.clone();
                    }
                    m.touch();
                });
                // (an MDI window's title follows: that pane's window, not
                // the active one)
                mdi::set_child_title(&docs_name(dock), &p, &t);
                Outcome::default()
            } else {
                return Some(Value::String(manager::with(dock, |m| m.pane(&name(0)).map(|p| p.title.clone())).flatten().unwrap_or_default()));
            }
        }
        "panestate" => return Some(Value::String(manager::with(dock, |m| m.state_of(&name(0)).to_string()).unwrap_or_default())),
        "panevisible" => {
            let p = name(0);
            let shown = manager::with(dock, |m| m.layout.find(&p).is_some() && m.layout.is_active(&p)).unwrap_or(false);
            return Some(int(if shown { -1 } else { 0 }));
        }
        // (RapidR Studio's) An MDI document's window: 0 normal, 1 minimized,
        // 2 maximized; with State, it becomes that, as its title bar's
        // buttons would make it
        "documentstate" => {
            let p = name(0);
            let docs = docs_name(dock);
            let now = mdi::child_state(&docs, &p);
            if args.len() < 2 {
                return Some(int(now.map_or(-1, |s| s as i64)));
            }
            let want = arg(1).to_i64();
            let action = match (now, want) {
                (None, _) => None,
                (Some(mdi::State::Maximized), 2) | (Some(mdi::State::Minimized), 1) | (Some(mdi::State::Normal), 0) => None,
                (Some(mdi::State::Maximized), 0) => Some(mdi::Action::ToggleMaximize),
                (Some(mdi::State::Minimized), 0) => Some(mdi::Action::Activate),
                (Some(_), 1) => Some(mdi::Action::Minimize),
                (Some(_), _) => Some(mdi::Action::ToggleMaximize),
            };
            if let Some(a) = action {
                docs_user(rt, &docs, &p, a);
            }
            return Some(Value::Null);
        }
        // (tabbed documents) AddView(Document, Component, Caption): one of
        // the document's views, on its tab strip's switch
        "addview" => {
            let (doc, comp) = (name(0), component_arg(&arg(1)).to_ascii_lowercase());
            let out = manager::with_mut(dock, |m| m.add_view(&doc, &comp, &s(2)));
            // (a view's component lives in the document area)
            let docs = docs_name(dock);
            if rt.exists(&docs) && !rt.get(&comp, "parent").to_string_val().eq_ignore_ascii_case(&docs) {
                rt.set(&comp, "parent", Value::String(docs));
            }
            Outcome { changed: false, ..out }
        }
        // DocumentView(Name[, View]): the view shown, by caption ("Split":
        // the first two side by side)
        "documentview" => {
            let p = name(0);
            if args.len() < 2 {
                return Some(Value::String(manager::with(dock, |m| m.pane(&p).map(|i| i.view_caption())).flatten().unwrap_or_default()));
            }
            let v = manager::with(dock, |m| m.view_named(&p, &s(1))).flatten();
            match v {
                Some(v) => manager::with_mut(dock, |m| m.set_view(&p, v)),
                None => Outcome::default(),
            }
        }
        // DocumentModified(Name[, Modified]): a dot on its tab
        "documentmodified" => {
            let p = name(0);
            if args.len() < 2 {
                return Some(int(if manager::with(dock, |m| m.pane(&p).is_some_and(|i| i.modified)).unwrap_or(false) { -1 } else { 0 }));
            }
            let on = arg(1).to_bool();
            manager::with_mut(dock, |m| {
                if let Some(i) = m.panes.iter_mut().find(|i| i.name == p) {
                    i.modified = on;
                }
                m.touch();
            });
            Outcome::default()
        }
        // SplitDocument(Name, Side): a new group of documents on that side
        "splitdocument" => match Side::parse(&s(1)) {
            Some(side) => manager::with_mut(dock, |m| m.split_document(&name(0), side)),
            None => Outcome::default(),
        },
        "nextdocument" => manager::with_mut(dock, |m| m.next_document(false)),
        "previousdocument" => manager::with_mut(dock, |m| m.next_document(true)),
        // (the documents' MDI client: QFORMMDI's arrangements)
        "cascade" | "tilehorizontal" | "tilevertical" | "arrangeicons" => {
            let docs = docs_name(dock);
            let mdi_method = match method.as_str() {
                "cascade" => "cascadechild",
                "tilehorizontal" => "sethorzchild",
                "tilevertical" => "setvertchild",
                _ => "iconarrangechild",
            };
            if mdi::is_mdi(&docs) {
                let client = (rt.get(&docs, "width").to_i64(), rt.get(&docs, "height").to_i64());
                let _ = mdi::call(&docs, mdi_method, &[], client, &crate::handles::name_of);
                mdi::render(M(rt), &docs);
            }
            Outcome::default()
        }
        _ => return None,
    };
    let value = out.value.clone().unwrap_or(Value::Null);
    finish(rt, dock, out);
    Some(value)
}

/// An RDOCKMANAGER property; `None` if `prop` isn't one of its own.
pub fn rt_get(dock: &str, prop: &str) -> Option<Value> {
    let p = prop.to_ascii_lowercase();
    // (reading never makes the model: the first method or property set
    // does, on every runtime alike — what it places is then made in the
    // same order)
    let empty = manager::Manager::default();
    let read = |m: &manager::Manager| -> Option<Value> {
        Some(match p.as_str() {
            "documentmode" => Value::String(m.layout.mode.name().into()),
            "activedocument" => Value::String(m.active_document().map(|d| m.pane(&d).map_or(d.clone(), |i| i.given.clone())).unwrap_or_default()),
            "activepane" => Value::String(m.active_pane.as_ref().map(|a| m.pane(a).map_or(a.clone(), |i| i.given.clone())).unwrap_or_default()),
            "panecount" => Value::Integer(m.panes.len() as i64),
            "documentcount" => Value::Integer(m.layout.documents.len() as i64),
            "documentgroupcount" => Value::Integer(if m.layout.documents.is_empty() { 0 } else { m.layout.group_count() as i64 }),
            "layout" => Value::String(m.save()),
            _ => return None,
        })
    };
    manager::with(dock, read).unwrap_or_else(|| read(&empty))
}

/// Sets an RDOCKMANAGER property; whether it was one of its own.
pub fn rt_set<R: Runtime>(rt: R, dock: &str, prop: &str, v: &Value) -> bool {
    let p = prop.to_ascii_lowercase();
    let out = match p.as_str() {
        "documentmode" => {
            let Some(mode) = DocumentMode::parse(&v.to_string_val()) else { return true };
            manager::with_mut(dock, |m| {
                let changed = m.layout.mode != mode;
                m.layout.mode = mode;
                m.touch();
                Outcome { changed, ..Default::default() }
            })
        }
        "activedocument" => {
            let name = component_arg(v).to_ascii_lowercase();
            manager::with_mut(dock, |m| m.activate_document(&name))
        }
        "activepane" => {
            let name = component_arg(v).to_ascii_lowercase();
            manager::with_mut(dock, |m| m.focus_pane(&name))
        }
        "layout" => manager::with_mut(dock, |m| m.load(&v.to_string_val())),
        _ => return false,
    };
    if APPLYING.with(Cell::get) {
        return true;
    }
    finish(rt, dock, out);
    true
}

/// A user's action from the kernel (`Container::Dock`).
pub fn rt_user<R: Runtime>(rt: R, dock: &str, action: User) {
    sync(rt, dock);
    let out = manager::with_mut(dock, |m| m.user(action));
    finish(rt, dock, out);
}

/// After a property of `name` was stored: a dock manager (or a floating
/// window, its group) resized or moved, or a floating window closed.
pub fn rt_after_set<R: Runtime>(rt: R, name: &str, prop: &str) {
    if APPLYING.with(Cell::get) {
        return;
    }
    let p = prop.to_ascii_lowercase();
    if !matches!(p.as_str(), "width" | "height" | "left" | "top" | "visible" | "fontname" | "fontsize" | "fontbold") {
        return;
    }
    let lower = name.to_ascii_lowercase();
    if manager::exists(&lower) && rt.type_of(&lower).eq_ignore_ascii_case("RDOCKMANAGER") {
        if matches!(p.as_str(), "width" | "height" | "fontname" | "fontsize" | "fontbold") {
            apply(rt, &lower);
        }
        return;
    }
    if let Some((dock, Slot::Float(_))) = group_of(&lower) {
        if matches!(p.as_str(), "width" | "height") {
            apply(rt, &dock);
        }
        return;
    }
    if let Some((dock, i)) = float_of(&lower) {
        // (moved or resized by the user: remembered; closed: its panes hide)
        let shown = rt.get(&lower, "__dockshown").to_bool();
        if p == "visible" && shown && !rt.get(&lower, "visible").to_bool() {
            rt.set(&lower, "__dockshown", int(0));
            let out = manager::with_mut(&dock, |m| m.float_closed(i));
            finish(rt, &dock, out);
            return;
        }
        let r = (rt.get(&lower, "left").to_i64(), rt.get(&lower, "top").to_i64(), rt.get(&lower, "width").to_i64(), rt.get(&lower, "height").to_i64());
        manager::with_mut(&dock, |m| m.float_moved(i, r));
    }
}

/// The size and font the dock manager has now, into its model.
fn sync<R: Runtime>(rt: R, dock: &str) {
    let size = (rt.get(dock, "width").to_i64(), rt.get(dock, "height").to_i64());
    let font = font_of(rt, dock);
    manager::with_mut(dock, |m| {
        m.resize(size, &font);
    });
}

/// Carries out an outcome: floating windows made, the components placed,
/// the events fired, documents closed (each after OnDocumentClose).
pub fn finish<R: Runtime>(rt: R, dock: &str, out: Outcome) {
    let dock = dock.to_ascii_lowercase();
    if !out.floating.is_empty() {
        let origin = screen_origin(rt, &dock);
        manager::with_mut(&dock, |m| {
            for (p, at) in &out.floating {
                let (fw, fh) = m.pane(p).map_or(super::DEFAULT_FLOAT, |i| i.float_size);
                let (x, y) = at.unwrap_or(((m.size.0 - fw) / 2, (m.size.1 - fh) / 2));
                m.floated(p, (origin.0 + x, origin.1 + y, fw, fh));
            }
        });
    }
    apply(rt, &dock);
    // (a window just floated comes to the front, over the one it left)
    for (p, _) in &out.floating {
        let at = manager::with(&dock, |m| m.layout.floating.iter().position(|f| f.panes.contains(p))).flatten();
        if let Some(i) = at {
            rt.show(&float_form(&dock, i));
        }
    }
    for e in &out.events {
        fire(rt, &dock, e);
    }
    if out.changed {
        rt.fire(&dock, "onlayoutchange", &[]);
    }
    if let Some(p) = &out.focus {
        focus_pane(rt, &dock, p);
    }
    close_next(rt, dock, out.closing);
}

fn fire<R: Runtime>(rt: R, dock: &str, e: &Event) {
    // (the pane named as the program named it)
    let args: Vec<Value> = e.args.iter().map(|a| Value::String(given(dock, &a.to_string_val()))).collect();
    rt.fire(dock, e.name, &args);
}

/// Asks the program about the first document (OnDocumentClose (Name,
/// Cancel)), closes it unless Cancel was set, then the others.
fn close_next<R: Runtime>(rt: R, dock: String, mut rest: Vec<String>) {
    if rest.is_empty() {
        return;
    }
    let doc = rest.remove(0);
    if !manager::with(&dock, |m| m.layout.documents.contains(&doc)).unwrap_or(false) {
        return close_next(rt, dock, rest);
    }
    let name = given(&dock, &doc);
    let d = dock.clone();
    let views: Vec<String> = manager::with(&dock, |m| m.pane(&doc).map(|i| i.views.iter().map(|v| v.0.clone()).collect())).flatten().unwrap_or_default();
    rt.fire_then(
        &dock,
        "ondocumentclose",
        &[Value::String(name), int(0)],
        Box::new(move |args: &[Value]| {
            let cancel = args.get(1).is_some_and(Value::to_bool);
            if !cancel {
                let out = manager::with_mut(&d, |m| m.close_document(&doc));
                let docs = docs_name(&d);
                if mdi::child_index(&docs, &doc).is_some() {
                    mdi::close(&docs, &doc);
                }
                rt.set(&doc, "visible", int(0));
                for v in views {
                    hide(rt, &v);
                }
                finish(rt, &d, out);
            }
            close_next(rt, d, rest);
        }),
    );
}

/// Focuses the first component in pane `pane`'s component that takes the
/// focus (itself first).
fn focus_pane<R: Runtime>(rt: R, dock: &str, pane: &str) {
    // (a document with views: the shown view's component)
    let shown = manager::with(dock, |m| m.pane(pane).filter(|i| !i.views.is_empty()).and_then(|i| i.shown_components().last().cloned())).flatten();
    let pane = shown.as_deref().unwrap_or(pane);
    fn first<R: Runtime>(rt: R, name: &str, t: &str, depth: usize) -> Option<String> {
        // (Visible and Enabled never set: true, as the kernel reads them)
        let on = |prop: &str| match rt.get(name, prop) {
            crate::Value::Null => true,
            v => v.to_bool(),
        };
        if crate::objects::a11y::takes_focus(t) && on("visible") && on("enabled") {
            return Some(name.to_string());
        }
        if depth > 16 {
            return None;
        }
        rt.children(name).into_iter().find_map(|(c, ct)| first(rt, &c, &ct, depth + 1))
    }
    let t = rt.type_of(pane);
    if let Some(target) = first(rt, pane, &t, 0) {
        rt.focus(&target);
    }
}

// ------------------------------------------------------------ placing --

fn place<R: Runtime>(rt: R, name: &str, parent: &str, r: Rect) {
    if !rt.get(name, "parent").to_string_val().eq_ignore_ascii_case(parent) {
        rt.set(name, "parent", Value::String(parent.to_string()));
    }
    for (p, v) in [("left", r.0), ("top", r.1), ("width", r.2), ("height", r.3)] {
        if rt.get(name, p).to_i64() != v {
            rt.set(name, p, int(v));
        }
    }
    if !rt.get(name, "visible").to_bool() {
        rt.set(name, "visible", int(-1));
    }
}

fn hide<R: Runtime>(rt: R, name: &str) {
    // (Visible unset reads as nothing: shown)
    let v = rt.get(name, "visible");
    if rt.exists(name) && (matches!(v, Value::Null) || v.to_bool()) {
        rt.set(name, "visible", int(0));
    }
}

/// A group component (made the first time).
fn ensure_group<R: Runtime>(rt: R, dock: &str, slot: Slot, parent: &str) -> String {
    let name = group_name(dock, slot);
    if !rt.exists(&name) {
        rt.create(&name, "RDOCKGROUP");
        rt.set(&name, "__dock", Value::String(dock.to_string()));
        rt.set(&name, "__slot", Value::String(slot.suffix()));
        rt.set(&name, "parent", Value::String(parent.to_string()));
    }
    name
}

/// Places everything where the model says.
pub fn apply<R: Runtime>(rt: R, dock: &str) {
    if APPLYING.with(|a| a.replace(true)) {
        return;
    }
    apply_now(rt, dock);
    APPLYING.with(|a| a.set(false));
    rt.restructure();
}

fn apply_now<R: Runtime>(rt: R, dock: &str) {
    sync(rt, dock);
    let (g, layout) = manager::with_mut(dock, |m| (m.geometry().clone(), m.layout.clone()));
    let mut placed: Vec<String> = Vec::new();
    // ---- docked groups
    for gr in &g.groups {
        let name = ensure_group(rt, dock, gr.slot, dock);
        place(rt, &name, dock, gr.rect);
        for (k, p) in gr.panes.iter().enumerate() {
            if k == gr.active {
                place(rt, p, &name, gr.content);
            } else {
                hide(rt, p);
            }
            placed.push(p.clone());
        }
    }
    for i in g.groups.len()..g.groups.len() + 64 {
        let name = group_name(dock, Slot::Docked(i));
        if !rt.exists(&name) {
            break;
        }
        hide(rt, &name);
    }
    // ---- the documents
    if let Some(d) = &g.documents {
        let docs = docs_name(dock);
        if !rt.exists(&docs) {
            rt.create(&docs, "RDOCKDOCS");
            rt.set(&docs, "__dock", Value::String(dock.to_string()));
            rt.set(&docs, "parent", Value::String(dock.to_string()));
        }
        let size_before = (rt.get(&docs, "width").to_i64(), rt.get(&docs, "height").to_i64());
        place(rt, &docs, dock, d.rect);
        let client = (d.rect.2, d.rect.3);
        let mdi_on = layout.mode == DocumentMode::Mdi;
        if mdi_on {
            mdi::register(&docs);
        }
        // (children no longer documents, or in tabbed mode: out of the MDI client)
        if mdi::is_mdi(&docs) {
            for f in mdi::frames(&docs) {
                if !mdi_on || !layout.documents.contains(&f.component) {
                    mdi::close(&docs, &f.component);
                }
            }
        }
        // (each document's components: its own, its views')
        let comps = |p: &str| -> (Vec<String>, Vec<String>) {
            manager::with(dock, |m| {
                m.pane(p).map(|i| {
                    let mut all = vec![i.name.clone()];
                    all.extend(i.views.iter().map(|v| v.0.clone()).filter(|v| *v != i.name));
                    (all, i.shown_components())
                })
            })
            .flatten()
            .unwrap_or_else(|| (vec![p.to_string()], vec![p.to_string()]))
        };
        for (k, p) in layout.documents.iter().enumerate() {
            placed.push(p.clone());
            let (all, shown) = comps(p);
            for c in &all {
                if !rt.get(c, "parent").to_string_val().eq_ignore_ascii_case(&docs) {
                    rt.set(c, "parent", Value::String(docs.clone()));
                }
            }
            if mdi_on {
                // (an MDI window holds the document's own component)
                for c in all.iter().filter(|c| *c != p) {
                    hide(rt, c);
                }
                if mdi::child_index(&docs, p).is_none() {
                    let title = manager::with(dock, |m| m.pane(p).map(|i| i.title.clone())).flatten().unwrap_or_default();
                    let h = crate::handles::handle_of(p);
                    let _ = mdi::call(&docs, "addchild", &[int(h), Value::String(title), int(k as i64), int(0), int(0), int(0), int(0), int(-1)], client, &crate::handles::name_of);
                }
                continue;
            }
            // (tabbed: the shown document of each group in its group's places)
            let gr = d.groups.iter().find(|g| g.shown().map(String::as_str) == Some(p.as_str()));
            for c in &all {
                match (gr, shown.iter().position(|x| x == c)) {
                    (Some(g), Some(i)) if i < g.places.len() => place(rt, c, &docs, g.places[i]),
                    _ => hide(rt, c),
                }
            }
        }
        if mdi_on {
            if size_before != client {
                mdi::client_resized(&docs, client);
            }
            // (the active document's window on top)
            if let Some(a) = layout.active_document.and_then(|i| layout.documents.get(i)) {
                let top = mdi::frames(&docs).into_iter().rev().find(|f| f.state != mdi::State::Minimized).map(|f| f.component);
                if top.as_deref() != Some(a.as_str()) {
                    let _ = mdi::user(&docs, a, mdi::Action::Activate, client);
                }
            }
        }
        if mdi::is_mdi(&docs) {
            mdi::render(M(rt), &docs);
        }
    }
    // ---- the flyout
    let fly = group_name(dock, Slot::Flyout);
    match &g.flyout {
        Some(f) => {
            let name = ensure_group(rt, dock, Slot::Flyout, dock);
            place(rt, &name, dock, f.rect);
            place(rt, &f.panes[0], &name, f.content);
            placed.push(f.panes[0].clone());
        }
        None => hide(rt, &fly),
    }
    // ---- floating windows
    let font = font_of(rt, dock);
    for (i, fl) in layout.floating.iter().enumerate() {
        let form = float_form(dock, i);
        if !rt.exists(&form) {
            rt.create(&form, "RFORM");
            rt.set(&form, "borderstyle", int(2));
        }
        let title = manager::with(dock, |m| m.pane(&fl.panes[fl.active]).map(|p| p.title.clone())).flatten().unwrap_or_default();
        rt.set(&form, "caption", Value::String(title));
        for (p, v) in [("left", fl.rect.0), ("top", fl.rect.1), ("width", fl.rect.2), ("height", fl.rect.3)] {
            if rt.get(&form, p).to_i64() != v {
                rt.set(&form, p, int(v));
            }
        }
        let gname = ensure_group(rt, dock, Slot::Float(i), &form);
        if rt.get(&gname, "align").to_i64() != 5 {
            rt.set(&gname, "align", int(5));
        }
        let (cw, ch) = (rt.get(&gname, "width").to_i64(), rt.get(&gname, "height").to_i64());
        let gr = manager::with(dock, |m| geometry::group(Slot::Float(i), &fl.panes, fl.active, (0, 0, cw, ch), Vec::new(), &m.titles(), &font)).unwrap();
        for (k, p) in fl.panes.iter().enumerate() {
            placed.push(p.clone());
            if k == fl.active {
                place(rt, p, &gname, gr.content);
            } else {
                hide(rt, p);
            }
        }
        if !rt.get(&gname, "visible").to_bool() {
            rt.set(&gname, "visible", int(-1));
        }
        if !rt.get(&form, "__dockshown").to_bool() || !rt.get(&form, "visible").to_bool() {
            rt.show(&form);
        }
        rt.set(&form, "__dockshown", int(-1));
    }
    for i in layout.floating.len()..layout.floating.len() + 64 {
        let form = float_form(dock, i);
        if !rt.exists(&form) {
            break;
        }
        if rt.get(&form, "visible").to_bool() {
            rt.set(&form, "__dockshown", int(0));
            rt.set(&form, "visible", int(0));
        }
    }
    // ---- the rest: hidden (auto-hidden ones not slid out, hidden ones)
    let panes = manager::with(dock, |m| m.panes.iter().map(|p| p.name.clone()).collect::<Vec<_>>()).unwrap_or_default();
    for p in panes {
        if !placed.contains(&p) {
            hide(rt, &p);
        }
    }
}

// ------------------------------------------------- the MDI documents --

/// `crate::mdi::Runtime` over a dock manager's runtime (the document
/// area's MDI client).
#[derive(Clone, Copy)]
struct M<R>(R);

impl<R: Runtime> mdi::Runtime for M<R> {
    fn get(self, name: &str, prop: &str) -> Value {
        self.0.get(name, prop)
    }
    fn set(self, name: &str, prop: &str, v: Value) {
        self.0.set(name, prop, v)
    }
    fn create(self, name: &str, type_name: &str) {
        self.0.create(name, type_name)
    }
    fn exists(self, name: &str) -> bool {
        self.0.exists(name)
    }
    fn fire(self, name: &str, event: &str, args: &[Value]) {
        self.0.fire(name, event, args)
    }
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Box<dyn FnOnce()>) {
        self.0.fire_then(name, event, args, Box::new(move |_| then()))
    }
    fn children(self, name: &str) -> Vec<(String, String)> {
        self.0.children(name)
    }
    fn stack(self, _names: &[String]) {
        self.0.restructure()
    }
    fn client(self, form: &str) -> (i64, i64) {
        (self.0.get(form, "width").to_i64(), self.0.get(form, "height").to_i64())
    }
}

/// What the user did to a document's MDI window (the runtime's MDI glue
/// sends a dock manager's document area here): its close box asks the
/// program (OnDocumentClose), activating it is OnDocumentActivate.
pub fn docs_user<R: Runtime>(rt: R, docs: &str, component: &str, action: mdi::Action) {
    let Some(dock) = manager::is_docs_area(docs) else { return };
    let component = component.to_ascii_lowercase();
    if action == mdi::Action::Close {
        return finish(rt, &dock, Outcome { closing: vec![component], ..Default::default() });
    }
    let client = (rt.get(docs, "width").to_i64(), rt.get(docs, "height").to_i64());
    let _ = mdi::user(docs, &component, action, client);
    mdi::render(M(rt), docs);
    // (the window on top is the active document)
    let top = mdi::frames(docs).into_iter().rev().find(|f| f.state != mdi::State::Minimized).map(|f| f.component);
    if let Some(t) = top {
        let out = manager::with_mut(&dock, |m| m.activate_document(&t));
        if !out.events.is_empty() {
            for e in &out.events {
                fire(rt, &dock, e);
            }
        }
    }
    rt.restructure();
}
