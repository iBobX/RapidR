//! QFORMMDI (RapidQ manual, QFormMDI in RAPIDQ2.INC): a form whose client
//! area holds child windows, each showing one component of the program
//! (`AddChild(Edit(i).Handle, "Title", i, …)`). This is the model — the
//! children, which is active, their places and states, and what every
//! method does — shared by the desktop and web runtimes, which draw each
//! child's frame (a title bar with its buttons) and place its component
//! where [`frames`] says, and report the user's clicks and drags back
//! ([`user`]).
//!
//! Children are kept in z-order (the last is on top). The active child is
//! the one on top that isn't minimized. Events the program gets:
//! `OnChildActive`, `OnChildClose` (setting `ChildResult` to False keeps the
//! child open), `OnChildResize` — each with (handle, index, title).

use std::cell::RefCell;
use std::collections::HashMap;

use crate::Value;

/// The height of a child's title bar, and its border.
pub const TITLE_HEIGHT: i64 = 22;
pub const BORDER: i64 = 3;
/// A minimized child: a title bar this wide.
pub const ICON_WIDTH: i64 = 160;
/// Cascaded / default children step by this much.
const CASCADE_STEP: i64 = 24;

/// A child window's state (RapidQ's ChildState: wsNormal, wsMinimized,
/// wsMaximized).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Normal = 0,
    Minimized = 1,
    Maximized = 2,
}

impl State {
    fn from(n: i64) -> State {
        match n {
            1 => State::Minimized,
            2 => State::Maximized,
            _ => State::Normal,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Child {
    /// The component it shows (lowercase name).
    pub component: String,
    pub handle: i64,
    pub title: String,
    /// The program's index for it (`AddChild`'s third argument).
    pub index: i64,
    pub rect: Rect,
    pub state: State,
    /// Where it goes back to from minimized / maximized.
    normal: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub left: i64,
    pub top: i64,
    pub width: i64,
    pub height: i64,
}

impl Rect {
    pub fn new(left: i64, top: i64, width: i64, height: i64) -> Rect {
        Rect { left, top, width: width.max(ICON_WIDTH / 2), height: height.max(TITLE_HEIGHT + 2 * BORDER) }
    }
}

#[derive(Clone, Debug)]
struct Mdi {
    children: Vec<Child>,
    child_max: i64,
    result: bool,
    menu: i64,
    /// Children added so far (where the next default-placed one goes).
    added: i64,
}

impl Default for Mdi {
    fn default() -> Self {
        // (RC.EXE: ChildResult reads 0 until a child closes — close_next sets it)
        Mdi { children: Vec::new(), child_max: 1024, result: false, menu: 0, added: 0 }
    }
}

thread_local! {
    static FORMS: RefCell<HashMap<String, Mdi>> = RefCell::new(HashMap::new());
}

fn with<R>(form: &str, f: impl FnOnce(&mut Mdi) -> R) -> R {
    FORMS.with(|m| f(m.borrow_mut().entry(form.to_ascii_lowercase()).or_default()))
}

/// Makes `form` an MDI form (a QFORMMDI).
pub fn register(form: &str) {
    with(form, |_| {});
}

pub fn is_mdi(form: &str) -> bool {
    FORMS.with(|m| m.borrow().contains_key(&form.to_ascii_lowercase()))
}

/// An event for the program: `OnChildActive` / `OnChildResize` (the
/// runtime fires it with the handle, index and title).
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub name: &'static str,
    pub handle: i64,
    pub index: i64,
    pub title: String,
}

impl Event {
    fn of(name: &'static str, c: &Child) -> Event {
        Event { name, handle: c.handle, index: c.index, title: c.title.clone() }
    }

    /// The arguments the handler gets: (handleChild&, index&, titleChild$).
    pub fn args(&self) -> Vec<Value> {
        vec![Value::Integer(self.handle), Value::Integer(self.index), Value::String(self.title.clone())]
    }
}

/// What a method, property or user action did.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    /// The method's result (a FUNCTION's: GetChild, ChildExist, FreeChild).
    pub value: Value,
    /// Events to fire, in order.
    pub events: Vec<Event>,
    /// Children to close: the runtime fires OnChildClose for each (with
    /// `ChildResult` True) and closes it with [`close`] unless the handler
    /// set `ChildResult` to False.
    pub closing: Vec<String>,
}

impl Default for Outcome {
    fn default() -> Self {
        Outcome { value: Value::Null, events: Vec::new(), closing: Vec::new() }
    }
}

/// Where a child's component goes, and its frame (what the runtimes draw).
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub component: String,
    pub title: String,
    pub rect: Rect,
    pub state: State,
    pub active: bool,
    /// The component's place (the frame's inside), or None when minimized.
    pub body: Option<Rect>,
}

/// The children of `form` in z-order (the last on top).
pub fn frames(form: &str) -> Vec<Frame> {
    with(form, |m| {
        let active = active_pos(m);
        m.children
            .iter()
            .enumerate()
            .map(|(i, c)| Frame {
                component: c.component.clone(),
                title: c.title.clone(),
                rect: c.rect,
                state: c.state,
                active: Some(i) == active,
                body: (c.state != State::Minimized).then(|| body_of(c.rect)),
            })
            .collect()
    })
}

/// A frame's inside: below its title bar, within its border.
pub fn body_of(r: Rect) -> Rect {
    Rect {
        left: r.left + BORDER,
        top: r.top + TITLE_HEIGHT + BORDER,
        width: (r.width - 2 * BORDER).max(0),
        height: (r.height - TITLE_HEIGHT - 2 * BORDER).max(0),
    }
}

/// The active child: the top one that isn't minimized (else the top one).
fn active_pos(m: &Mdi) -> Option<usize> {
    m.children.iter().rposition(|c| c.state != State::Minimized).or(m.children.len().checked_sub(1))
}

fn active(m: &Mdi) -> Option<&Child> {
    active_pos(m).map(|i| &m.children[i])
}

/// Brings child `i` to the top; the OnChildActive it causes, if it wasn't
/// the active one.
fn raise(m: &mut Mdi, i: usize) -> Vec<Event> {
    let before = active(m).map(|c| c.component.clone());
    let c = m.children.remove(i);
    m.children.push(c);
    let now = active(m);
    match now {
        Some(c) if before.as_deref() != Some(c.component.as_str()) => vec![Event::of("onchildactive", c)],
        _ => Vec::new(),
    }
}

fn find(m: &Mdi, component: &str) -> Option<usize> {
    m.children.iter().position(|c| c.component.eq_ignore_ascii_case(component))
}

/// The window state of the child showing `component` (`None`: no child shows it).
pub fn child_state(form: &str, component: &str) -> Option<State> {
    with(form, |m| find(m, component).map(|i| m.children[i].state))
}

/// Sets the title of the child showing `component` (active or not); false
/// when no child shows it.
pub fn set_child_title(form: &str, component: &str, title: &str) -> bool {
    if !is_mdi(form) {
        return false;
    }
    with(form, |m| match find(m, component) {
        Some(i) => {
            m.children[i].title = title.to_string();
            true
        }
        None => false,
    })
}

/// The program's index of the child showing `component`.
pub fn child_index(form: &str, component: &str) -> Option<i64> {
    with(form, |m| find(m, component).map(|i| m.children[i].index))
}

/// Takes a child away (after OnChildClose let it go): the child now on top
/// becomes active.
pub fn close(form: &str, component: &str) -> Outcome {
    with(form, |m| {
        let Some(i) = find(m, component) else { return Outcome::default() };
        let was_active = active_pos(m) == Some(i);
        m.children.remove(i);
        let events = match active(m) {
            Some(c) if was_active => vec![Event::of("onchildactive", c)],
            _ => Vec::new(),
        };
        Outcome { events, ..Default::default() }
    })
}

/// The component hosted by the child the program's `index` names.
fn by_index(m: &Mdi, index: i64) -> Option<usize> {
    m.children.iter().position(|c| c.index == index)
}

fn by_title(m: &Mdi, title: &str) -> Option<usize> {
    m.children.iter().position(|c| c.title.eq_ignore_ascii_case(title))
}

/// A QFORMMDI method. `client` is the form's client size; `name_of` finds a
/// component from its Handle. None if `method` isn't an MDI method.
pub fn call(form: &str, method: &str, args: &[Value], client: (i64, i64), name_of: &dyn Fn(i64) -> Option<String>) -> Option<Outcome> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    let (cw, ch) = client;
    let method = method.to_ascii_lowercase();
    with(form, |m| -> Option<Outcome> {
        let mut out = Outcome::default();
        match method.as_str() {
            // AddChild(handle&, title$, index&, left%, top%, width%, height%, DefaultSize&)
            "addchild" => {
                let handle = arg(0).to_i64();
                let Some(component) = name_of(handle) else { return Some(out) };
                if m.children.len() as i64 >= m.child_max || find(m, &component).is_some() {
                    return Some(out);
                }
                let rect = if arg(7).to_bool() || args.len() < 7 {
                    // Windows' default place: cascading, three quarters of the client area.
                    let step = (m.added % 8) * CASCADE_STEP;
                    Rect::new(step, step, cw * 3 / 4, ch * 3 / 4)
                } else {
                    Rect::new(arg(3).to_i64(), arg(4).to_i64(), arg(5).to_i64(), arg(6).to_i64())
                };
                m.added += 1;
                m.children.push(Child { component, handle, title: arg(1).to_string_val(), index: arg(2).to_i64(), rect, state: State::Normal, normal: rect });
                out.events.push(Event::of("onchildactive", m.children.last().unwrap()));
            }
            "closechild" => out.closing = active(m).map(|c| c.component.clone()).into_iter().collect(),
            "closeallchild" => out.closing = m.children.iter().rev().map(|c| c.component.clone()).collect(),
            "cascadechild" => {
                for (n, c) in m.children.iter_mut().enumerate() {
                    let step = (n as i64 % 8) * CASCADE_STEP;
                    c.state = State::Normal;
                    c.rect = Rect::new(step, step, cw * 3 / 4, ch * 3 / 4);
                    c.normal = c.rect;
                }
            }
            // Tiled one above another (SetHorzChild) or side by side (SetVertChild).
            "sethorzchild" | "setvertchild" => {
                let open: Vec<usize> = (0..m.children.len()).filter(|&i| m.children[i].state != State::Minimized).collect();
                let n = open.len().max(1) as i64;
                for (k, &i) in open.iter().enumerate() {
                    let k = k as i64;
                    let c = &mut m.children[i];
                    c.state = State::Normal;
                    c.rect = if method == "sethorzchild" {
                        Rect::new(0, k * ch / n, cw, ch / n)
                    } else {
                        Rect::new(k * cw / n, 0, cw / n, ch)
                    };
                    c.normal = c.rect;
                }
            }
            "iconarrangechild" => arrange_icons(m, cw, ch),
            "minimizeallchild" => {
                for c in m.children.iter_mut() {
                    minimize(c);
                }
                arrange_icons(m, cw, ch);
            }
            "maximizeallchild" => {
                for c in m.children.iter_mut() {
                    maximize(c, cw, ch);
                }
            }
            "restorechild" => {
                for c in m.children.iter_mut() {
                    restore(c);
                }
            }
            "activenextchild" | "activepreviouschild" if m.children.len() > 1 => {
                // Next: the bottom one comes up; previous: the top one goes down.
                if method == "activenextchild" {
                    out.events = raise(m, 0);
                } else {
                    let top = m.children.pop().unwrap();
                    m.children.insert(0, top);
                    out.events = active(m).map(|c| Event::of("onchildactive", c)).into_iter().collect();
                }
            }
            "activechild" => {
                if let Some(i) = by_index(m, arg(0).to_i64()) {
                    restore_if_minimized(&mut m.children[i]);
                    out.events = raise(m, i);
                }
            }
            "getchild" => out.value = Value::Integer(by_title(m, &arg(0).to_string_val()).map_or(-1, |i| m.children[i].index)),
            "childexist" => out.value = Value::Integer(if by_title(m, &arg(0).to_string_val()).is_some() { -1 } else { 0 }),
            // True when the component isn't in a child window (free to use).
            "freechild" => {
                let used = name_of(arg(0).to_i64()).is_some_and(|n| find(m, &n).is_some());
                out.value = Value::Integer(if used { 0 } else { -1 });
            }
            "setdeskbar" => {}
            _ => return None,
        }
        Some(out)
    })
}

fn minimize(c: &mut Child) {
    if c.state == State::Normal {
        c.normal = c.rect;
    }
    c.state = State::Minimized;
}

fn maximize(c: &mut Child, cw: i64, ch: i64) {
    if c.state == State::Normal {
        c.normal = c.rect;
    }
    c.state = State::Maximized;
    c.rect = Rect { left: 0, top: 0, width: cw, height: ch };
}

fn restore(c: &mut Child) {
    c.state = State::Normal;
    c.rect = c.normal;
}

fn restore_if_minimized(c: &mut Child) {
    if c.state == State::Minimized {
        restore(c);
    }
}

/// Minimized children in a row along the bottom of the client area.
fn arrange_icons(m: &mut Mdi, cw: i64, ch: i64) {
    let per_row = (cw / ICON_WIDTH).max(1);
    let mut n = 0;
    for c in m.children.iter_mut().filter(|c| c.state == State::Minimized) {
        let (col, row) = (n % per_row, n / per_row);
        c.rect = Rect { left: col * ICON_WIDTH, top: ch - (row + 1) * (TITLE_HEIGHT + 2 * BORDER), width: ICON_WIDTH, height: TITLE_HEIGHT + 2 * BORDER };
        n += 1;
    }
}

/// A QFORMMDI property (the active child's), if it's one.
pub fn get(form: &str, prop: &str) -> Option<Value> {
    if !is_mdi(form) {
        return None;
    }
    with(form, |m| {
        let a = active(m);
        let s = |f: &dyn Fn(&Child) -> Value| a.map_or(Value::Null, f);
        Some(match prop.to_ascii_lowercase().as_str() {
            "childcount" => Value::Integer(m.children.len() as i64),
            "childmax" => Value::Integer(m.child_max),
            "childresult" => Value::Integer(m.result as i64),
            "mdimenu" => Value::Integer(m.menu),
            "childcaption" => s(&|c| Value::String(c.title.clone())),
            "childhandle" => s(&|c| Value::Integer(c.handle)),
            "componentindex" => s(&|c| Value::Integer(c.index)),
            "childleft" => s(&|c| Value::Integer(c.rect.left)),
            "childtop" => s(&|c| Value::Integer(c.rect.top)),
            "childwidth" => s(&|c| Value::Integer(c.rect.width)),
            "childheight" => s(&|c| Value::Integer(c.rect.height)),
            "childstate" => s(&|c| Value::Integer(c.state as i64)),
            _ => return None,
        })
    })
}

/// Sets a QFORMMDI property; Some if it was one (with the events it caused).
pub fn set(form: &str, prop: &str, v: &Value, client: (i64, i64)) -> Option<Outcome> {
    if !is_mdi(form) {
        return None;
    }
    let (cw, ch) = client;
    with(form, |m| {
        let prop = prop.to_ascii_lowercase();
        match prop.as_str() {
            "childmax" => m.child_max = v.to_i64().max(0),
            "childresult" => m.result = v.to_bool(),
            "mdimenu" => m.menu = v.to_i64(),
            "childcaption" | "childleft" | "childtop" | "childwidth" | "childheight" | "childstate" => {
                let Some(i) = active_pos(m) else { return Some(Outcome::default()) };
                let c = &mut m.children[i];
                let before = c.rect;
                match prop.as_str() {
                    "childcaption" => c.title = v.to_string_val(),
                    "childleft" => c.rect.left = v.to_i64(),
                    "childtop" => c.rect.top = v.to_i64(),
                    "childwidth" => c.rect.width = v.to_i64().max(ICON_WIDTH / 2),
                    "childheight" => c.rect.height = v.to_i64().max(TITLE_HEIGHT + 2 * BORDER),
                    _ => match State::from(v.to_i64()) {
                        State::Minimized => minimize(c),
                        State::Maximized => maximize(c, cw, ch),
                        State::Normal => restore(c),
                    },
                }
                if c.state == State::Normal {
                    c.normal = c.rect;
                }
                let mut out = Outcome::default();
                if (c.rect.width, c.rect.height) != (before.width, before.height) {
                    out.events.push(Event::of("onchildresize", c));
                }
                if c.state == State::Minimized {
                    arrange_icons(m, cw, ch);
                }
                return Some(out);
            }
            _ => return None,
        }
        Some(Outcome::default())
    })
}

/// What the user did to a child's frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// Clicked it (or its component): it becomes active.
    Activate,
    Close,
    Minimize,
    /// The maximize button or a double click on the title: maximized, or
    /// back to normal.
    ToggleMaximize,
    /// Dragged by its title bar to here.
    Move(i64, i64),
    /// Dragged by an edge or a corner: its new Left, Top, Width, Height
    /// ([`resized`] keeps it at least Windows' least size).
    Resize(i64, i64, i64, i64),
}

pub fn user(form: &str, component: &str, action: Action, client: (i64, i64)) -> Outcome {
    let (cw, ch) = client;
    with(form, |m| {
        let Some(i) = find(m, component) else { return Outcome::default() };
        let mut out = Outcome::default();
        match action {
            Action::Activate => {
                restore_if_minimized(&mut m.children[i]);
                out.events = raise(m, i);
            }
            Action::Close => out.closing.push(m.children[i].component.clone()),
            Action::Minimize => {
                minimize(&mut m.children[i]);
                arrange_icons(m, cw, ch);
                // The minimized one goes to the bottom; the next becomes active.
                let c = m.children.remove(i);
                m.children.insert(0, c);
                out.events = active(m).filter(|c| c.state != State::Minimized).map(|c| Event::of("onchildactive", c)).into_iter().collect();
            }
            Action::ToggleMaximize => {
                let c = &mut m.children[i];
                if c.state == State::Maximized { restore(c) } else { maximize(c, cw, ch) }
                out.events.push(Event::of("onchildresize", &m.children[i]));
                out.events.extend(raise(m, i));
            }
            Action::Move(l, t) => {
                let c = &mut m.children[i];
                if c.state == State::Normal {
                    c.rect.left = l;
                    c.rect.top = t;
                    c.normal = c.rect;
                }
            }
            Action::Resize(l, t, w, h) => {
                let c = &mut m.children[i];
                if c.state == State::Normal && (c.rect.left, c.rect.top, c.rect.width, c.rect.height) != (l, t, w, h) {
                    c.rect = Rect::new(l, t, w, h);
                    c.normal = c.rect;
                    out.events.push(Event::of("onchildresize", c));
                }
            }
        }
        out
    })
}

/// The form's client area changed size: maximized children follow.
pub fn client_resized(form: &str, client: (i64, i64)) {
    with(form, |m| {
        for c in m.children.iter_mut().filter(|c| c.state == State::Maximized) {
            c.rect = Rect { left: 0, top: 0, width: client.0, height: client.1 };
        }
        arrange_icons(m, client.0, client.1);
    });
}

// ---------------------------------------------------------------------------
// Applying the model: the same for the desktop and the web runtime
// ---------------------------------------------------------------------------

/// What a runtime provides to show QFORMMDI children.
pub trait Runtime: Copy + 'static {
    fn get(self, name: &str, prop: &str) -> Value;
    fn set(self, name: &str, prop: &str, v: Value);
    /// Makes a component (a child's frame, type `RMDICHILD`).
    fn create(self, name: &str, type_name: &str);
    fn exists(self, name: &str) -> bool;
    fn fire(self, name: &str, event: &str, args: &[Value]);
    /// Fires, then runs `then` once the handler has run.
    fn fire_then(self, name: &str, event: &str, args: &[Value], then: Box<dyn FnOnce()>);
    /// The components parented to `name`: (name, type).
    fn children(self, name: &str) -> Vec<(String, String)>;
    /// Puts these widgets on top, in order (the last on top).
    fn stack(self, names: &[String]);
    /// The form's client size.
    fn client(self, form: &str) -> (i64, i64);
}

/// The frame component of `component`'s child window in `form`.
pub fn frame_name(form: &str, component: &str) -> String {
    format!("{}__mdi__{}", form.to_ascii_lowercase(), component.to_ascii_lowercase())
}

/// A QFORMMDI method; None if `method` isn't one of MDI's.
pub fn rt_method<R: Runtime>(rt: R, form: &str, method: &str, args: &[Value]) -> Option<Value> {
    let out = call(form, method, args, rt.client(form), &crate::handles::name_of)?;
    let value = out.value.clone();
    apply(rt, form, out);
    Some(value)
}

/// Sets a QFORMMDI property; false if `prop` isn't one of MDI's.
pub fn rt_set<R: Runtime>(rt: R, form: &str, prop: &str, v: &Value) -> bool {
    match set(form, prop, v, rt.client(form)) {
        Some(out) => {
            apply(rt, form, out);
            true
        }
        None => false,
    }
}

/// What the user did to a child's frame.
pub fn rt_user<R: Runtime>(rt: R, form: &str, component: &str, action: Action) {
    let out = user(form, component, action, rt.client(form));
    apply(rt, form, out);
}

/// The form's client area changed size.
pub fn rt_resized<R: Runtime>(rt: R, form: &str) {
    client_resized(form, rt.client(form));
    render(rt, form);
}

/// Shows the outcome, fires its events, and closes the children it closes
/// (asking the program first: OnChildClose, ChildResult).
pub fn apply<R: Runtime>(rt: R, form: &str, out: Outcome) {
    render(rt, form);
    for e in &out.events {
        rt.fire(form, e.name, &e.args());
    }
    close_next(rt, form.to_string(), out.closing);
}

/// Closes the first of `rest` unless its OnChildClose handler set
/// `ChildResult` to False, then the others (each after its handler ran).
fn close_next<R: Runtime>(rt: R, form: String, mut rest: Vec<String>) {
    if rest.is_empty() {
        return;
    }
    let component = rest.remove(0);
    let Some(frame) = frames(&form).into_iter().find(|f| f.component == component) else { return close_next(rt, form, rest) };
    set(&form, "childresult", &Value::Integer(-1), rt.client(&form));
    let args = [Value::Integer(crate::handles::handle_of(&component)), Value::Integer(child_index(&form, &component).unwrap_or(0)), Value::String(frame.title)];
    let f = form.clone();
    rt.fire_then(
        &form,
        "onchildclose",
        &args,
        Box::new(move || {
            if get(&f, "childresult").is_some_and(|r| r.to_bool()) {
                let closed = close(&f, &component);
                rt.set(&component, "visible", Value::Integer(0));
                render(rt, &f);
                for e in &closed.events {
                    rt.fire(&f, e.name, &e.args());
                }
            }
            close_next(rt, f, rest);
        }),
    );
}

/// Places every child's frame (a `RMDICHILD` component: `caption`,
/// `active`, `childstate` for drawing) and component; hides the frames of
/// closed children; stacks them.
pub fn render<R: Runtime>(rt: R, form: &str) {
    let all = frames(form);
    let form = form.to_ascii_lowercase();
    let int = Value::Integer;
    for f in &all {
        let name = frame_name(&form, &f.component);
        let new = !rt.exists(&name);
        if new {
            rt.create(&name, "RMDICHILD");
        }
        rt.set(&name, "caption", Value::String(f.title.clone()));
        rt.set(&name, "active", int(if f.active { -1 } else { 0 }));
        rt.set(&name, "childstate", int(f.state as i64));
        rt.set(&name, "__form", Value::String(form.clone()));
        rt.set(&name, "__component", Value::String(f.component.clone()));
        rt.set(&name, "left", int(f.rect.left));
        rt.set(&name, "top", int(f.rect.top));
        rt.set(&name, "width", int(f.rect.width));
        rt.set(&name, "height", int(f.rect.height));
        if new {
            rt.set(&name, "parent", Value::String(form.clone()));
        }
        rt.set(&name, "visible", int(-1));
        match f.body {
            Some(b) => {
                rt.set(&f.component, "left", int(b.left));
                rt.set(&f.component, "top", int(b.top));
                rt.set(&f.component, "width", int(b.width));
                rt.set(&f.component, "height", int(b.height));
                rt.set(&f.component, "visible", int(-1));
            }
            None => rt.set(&f.component, "visible", int(0)),
        }
    }
    let open: Vec<String> = all.iter().map(|f| frame_name(&form, &f.component)).collect();
    for (child, t) in rt.children(&form) {
        if t.eq_ignore_ascii_case("RMDICHILD") && !open.iter().any(|o| o.eq_ignore_ascii_case(&child)) && rt.get(&child, "visible").to_bool() {
            rt.set(&child, "visible", int(0));
        }
    }
    let order: Vec<String> = all.iter().flat_map(|f| [frame_name(&form, &f.component), f.component.clone()]).collect();
    rt.stack(&order);
}

/// The edges a press on a child's frame takes to resize it (a corner:
/// two of them).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges {
    pub left: bool,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
}

impl Edges {
    /// The pointer's direction over them: "we" ↔, "ns" ↕, "nwse" ⤡,
    /// "nesw" ⤢.
    pub fn pointer(self) -> &'static str {
        match (self.left || self.right, self.top || self.bottom) {
            (true, true) if (self.left && self.top) || (self.right && self.bottom) => "nwse",
            (true, true) => "nesw",
            (true, false) => "we",
            _ => "ns",
        }
    }
}

/// How far in from a child's outside its sizing border reaches (its frame
/// and a pixel more, as Windows' WS_THICKFRAME).
pub const SIZE_BORDER: i64 = BORDER + 1;
/// Along an edge, this near a corner sizes both ways (Windows' corner
/// zone, the caption button's width).
pub const SIZE_CORNER: i64 = 16;
/// The least a child is dragged to: Windows' SM_CXMINTRACK × SM_CYMINTRACK
/// at 96 dpi (136 × 39: read by an RC.EXE-built program on Windows 11).
pub const MIN_TRACK: (i64, i64) = (136, 39);

/// The edges at (x, y) of a `w` × `h` child frame in the normal state
/// (`None`: inside it — the title bar, the client); the title bar's
/// buttons win over the top edge.
pub fn edges_at(w: i64, h: i64, x: i64, y: i64) -> Option<Edges> {
    if x < 0 || y < 0 || x >= w || y >= h {
        return None;
    }
    if button_at(w, x, y).is_some() {
        return None;
    }
    let (near_l, near_r, near_t, near_b) = (x < SIZE_BORDER, x >= w - SIZE_BORDER, y < SIZE_BORDER, y >= h - SIZE_BORDER);
    if !(near_l || near_r || near_t || near_b) {
        return None;
    }
    // (on an edge, near a corner: both ways)
    let (cl, cr, ct, cb) = (x < SIZE_CORNER, x >= w - SIZE_CORNER, y < SIZE_CORNER, y >= h - SIZE_CORNER);
    Some(Edges {
        left: near_l || ((near_t || near_b) && cl),
        right: near_r || ((near_t || near_b) && cr),
        top: near_t || ((near_l || near_r) && ct),
        bottom: near_b || ((near_l || near_r) && cb),
    })
}

/// A frame at `start` (Left, Top, Width, Height) with `edges` dragged by
/// (dx, dy): its new place, never under [`MIN_TRACK`] (a left or top edge
/// stops where the size would go under it, the opposite edge staying).
pub fn resized(start: (i64, i64, i64, i64), edges: Edges, dx: i64, dy: i64) -> (i64, i64, i64, i64) {
    let (mut l, mut t, mut w, mut h) = start;
    if edges.right {
        w = (start.2 + dx).max(MIN_TRACK.0);
    }
    if edges.bottom {
        h = (start.3 + dy).max(MIN_TRACK.1);
    }
    if edges.left {
        w = (start.2 - dx).max(MIN_TRACK.0);
        l = start.0 + start.2 - w;
    }
    if edges.top {
        h = (start.3 - dy).max(MIN_TRACK.1);
        t = start.1 + start.3 - h;
    }
    (l, t, w, h)
}

/// Where a click at (x, y) in a child's frame of width `w` lands: a title
/// bar button, or None.
pub fn button_at(w: i64, x: i64, y: i64) -> Option<Action> {
    if y < BORDER || y >= BORDER + TITLE_HEIGHT - 2 || w - BORDER - x < 0 {
        return None;
    }
    match (w - BORDER - x) / (TITLE_HEIGHT - 2) {
        0 => Some(Action::Close),
        1 => Some(Action::ToggleMaximize),
        2 => Some(Action::Minimize),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_edge_and_corner_sizes_a_child() {
        let (w, h) = (300, 200);
        let e = |x, y| edges_at(w, h, x, y);
        let edges = |l, t, r, b| Some(Edges { left: l, top: t, right: r, bottom: b });
        // (the edges, along their middles)
        assert_eq!(e(0, 100), edges(true, false, false, false));
        assert_eq!(e(SIZE_BORDER - 1, 100), edges(true, false, false, false));
        assert_eq!(e(w - 1, 100), edges(false, false, true, false));
        assert_eq!(e(150, 0), edges(false, true, false, false));
        assert_eq!(e(150, h - 1), edges(false, false, false, true));
        // (inside the border: not an edge — the title bar, the client)
        assert_eq!(e(SIZE_BORDER, 100), None);
        assert_eq!(e(150, BORDER + 5), None);
        assert_eq!(e(150, 100), None);
        // (the corners, from either edge within their zone)
        assert_eq!(e(0, 0), edges(true, true, false, false));
        assert_eq!(e(SIZE_CORNER - 1, 0), edges(true, true, false, false));
        assert_eq!(e(0, SIZE_CORNER - 1), edges(true, true, false, false));
        assert_eq!(e(w - 1, h - 1), edges(false, false, true, true));
        assert_eq!(e(w - 8, 3), None, "the close button wins over the top edge");
        assert_eq!(e(w - 1, 2), edges(false, true, true, false));
        assert_eq!(e(w - 2, h - SIZE_CORNER), edges(false, false, true, true));
        assert_eq!(e(2, h - 2), edges(true, false, false, true));
        assert_eq!(e(SIZE_CORNER, h - 1), edges(false, false, false, true));
        // (outside: nothing)
        assert_eq!(e(-1, 100), None);
        assert_eq!(e(w, 100), None);
        // (their pointers)
        assert_eq!(e(0, 0).unwrap().pointer(), "nwse");
        assert_eq!(e(w - 1, h - 1).unwrap().pointer(), "nwse");
        assert_eq!(e(2, h - 2).unwrap().pointer(), "nesw");
        assert_eq!(e(0, 100).unwrap().pointer(), "we");
        assert_eq!(e(150, h - 1).unwrap().pointer(), "ns");
    }

    #[test]
    fn resizing_keeps_the_opposite_edge_and_the_least_size() {
        let start = (50, 40, 300, 200);
        let all = |l, t, r, b| Edges { left: l, top: t, right: r, bottom: b };
        assert_eq!(resized(start, all(false, false, true, false), 30, 99), (50, 40, 330, 200));
        assert_eq!(resized(start, all(true, false, false, false), 30, 0), (80, 40, 270, 200));
        assert_eq!(resized(start, all(false, true, false, false), 0, -20), (50, 20, 300, 220));
        assert_eq!(resized(start, all(true, true, false, false), -10, -10), (40, 30, 310, 210));
        // (never under Windows' least: the left edge stops, the right stays)
        assert_eq!(resized(start, all(true, false, false, false), 1000, 0), (50 + 300 - MIN_TRACK.0, 40, MIN_TRACK.0, 200));
        assert_eq!(resized(start, all(false, false, true, true), -1000, -1000), (50, 40, MIN_TRACK.0, MIN_TRACK.1));
        // (the model: the place, OnChildResize; a maximized child stays)
        register("edges");
        add("edges", 1, "One");
        let out = user("edges", "edit(1)", Action::Resize(10, 20, 200, 150), (800, 600));
        assert_eq!(out.events.iter().map(|e| e.name).collect::<Vec<_>>(), ["onchildresize"]);
        let r = frames("edges")[0].rect;
        assert_eq!((r.left, r.top, r.width, r.height), (10, 20, 200, 150));
        user("edges", "edit(1)", Action::ToggleMaximize, (800, 600));
        let out = user("edges", "edit(1)", Action::Resize(0, 0, 100, 100), (800, 600));
        assert!(out.events.is_empty());
    }

    fn names(h: i64) -> Option<String> {
        Some(format!("edit({h})"))
    }

    fn add(form: &str, h: i64, title: &str) -> Outcome {
        call(form, "AddChild", &[Value::Integer(h), Value::String(title.into()), Value::Integer(h), Value::Integer(0), Value::Integer(0), Value::Integer(0), Value::Integer(0), Value::Integer(-1)], (800, 600), &names).unwrap()
    }

    #[test]
    fn a_childs_title_set_by_its_component_leaves_the_active_one() {
        register("t");
        add("t", 1, "One");
        add("t", 2, "Two");
        assert!(set_child_title("t", "edit(1)", "One *"));
        let titles: Vec<String> = frames("t").iter().map(|f| f.title.clone()).collect();
        assert_eq!(titles, vec!["One *".to_string(), "Two".to_string()]);
        assert!(!set_child_title("t", "edit(9)", "x"));
        assert!(!set_child_title("not-mdi", "edit(1)", "x"));
        assert!(!is_mdi("not-mdi"));
    }

    #[test]
    fn children_activate_close_and_arrange() {
        register("f");
        assert_eq!(add("f", 1, "One").events[0].title, "One");
        add("f", 2, "Two");
        add("f", 3, "Three");
        assert_eq!(get("f", "ChildCount"), Some(Value::Integer(3)));
        assert_eq!(get("f", "ChildCaption"), Some(Value::String("Three".into())));
        // Default places cascade.
        let f = frames("f");
        assert_eq!((f[1].rect.left, f[1].rect.width), (24, 600));
        assert_eq!(f[2].body, Some(body_of(f[2].rect)));
        // Next: the bottom one comes up.
        let o = call("f", "ActiveNextChild", &[], (800, 600), &names).unwrap();
        assert_eq!(o.events[0].title, "One");
        // By index / title.
        assert_eq!(call("f", "GetChild", &[Value::String("two".into())], (800, 600), &names).unwrap().value, Value::Integer(2));
        call("f", "ActiveChild", &[Value::Integer(2)], (800, 600), &names).unwrap();
        assert_eq!(get("f", "ComponentIndex"), Some(Value::Integer(2)));
        // Tiling side by side.
        call("f", "SetVertChild", &[], (900, 600), &names).unwrap();
        let widths: Vec<i64> = frames("f").iter().map(|f| f.rect.width).collect();
        assert_eq!(widths, vec![300, 300, 300]);
        // Close: the runtime asks the program, then closes; the next is active.
        let o = call("f", "CloseChild", &[], (800, 600), &names).unwrap();
        assert_eq!(o.closing, vec!["edit(2)".to_string()]);
        let o = close("f", "edit(2)");
        assert_eq!(o.events.len(), 1);
        assert_eq!(get("f", "ChildCount"), Some(Value::Integer(2)));
        assert_eq!(call("f", "FreeChild", &[Value::Integer(2)], (800, 600), &names).unwrap().value, Value::Integer(-1));
        assert_eq!(call("f", "FreeChild", &[Value::Integer(1)], (800, 600), &names).unwrap().value, Value::Integer(0));
        // Minimize and maximize.
        let o = user("f", "edit(1)", Action::Minimize, (800, 600));
        assert!(o.events.iter().all(|e| e.title != "One"));
        let one = frames("f").into_iter().find(|f| f.component == "edit(1)").unwrap();
        assert_eq!((one.state, one.body, one.rect.top), (State::Minimized, None, 600 - TITLE_HEIGHT - 2 * BORDER));
        user("f", "edit(3)", Action::ToggleMaximize, (800, 600));
        assert_eq!(get("f", "ChildState"), Some(Value::Integer(2)));
        assert_eq!(get("f", "ChildWidth"), Some(Value::Integer(800)));
        call("f", "RestoreChild", &[], (800, 600), &names).unwrap();
        assert_eq!(get("f", "ChildState"), Some(Value::Integer(0)));
    }
}
