//! A form as the kernel holds it: a retained tree of its visual
//! components, built from `Store::children`, with each one's absolute
//! rectangle in the window's client area (Left / Top are relative to the
//! parent; a form's children sit below an in-window main menu), and the
//! UI-only state the store doesn't keep: focus, hover, the pressed button,
//! the mouse capture, the caret's blink, parley editors.
//!
//! The tree is rebuilt when the form's structure changes ([`FormUi::rebuild`]
//! keeps each surviving component's UI state); geometry, visibility and
//! enabled-ness are re-read from the store before every paint
//! ([`FormUi::sync`]), so the store stays the only source of truth.

use rapidr_value::objects::ops::Rect;

use crate::components::{self, edit::EditUi, ComponentKind};
use crate::input::KernelEvent;
use crate::store::{self, Store};

/// UI-only state of one component.
#[derive(Default)]
pub struct NodeUi {
    /// A QEDIT's parley editor.
    pub edit: Option<Box<EditUi>>,
    /// An RCODEEDITOR's view (components/codeeditor).
    pub code: Option<Box<crate::components::codeeditor::CodeUi>>,
    /// A track bar's thumb being dragged.
    pub dragging: bool,
    /// The part pressed (an up-down's arrow: 0 up, 1 down).
    pub part: Option<usize>,
    /// (the surfaces lane's) What its bitmap shows, as last converted (a
    /// QCANVAS's surface, a QIMAGE's picture, a speed button's glyph).
    pub surface: Option<crate::components::canvas::Shown>,
    /// When its kind's `tick` runs next (tick.rs).
    pub wake: Option<crate::tick::Instant>,
    /// (I4 L-DVIEW) An RDESIGNSURFACE's designed form: its design-time
    /// store and the form's own tree (components/design.rs).
    pub design: Option<Box<crate::components::design::View>>,
}

pub struct Node {
    /// Its id, lowercase.
    pub id: String,
    /// Its RapidR type name (`RBUTTON` …).
    pub type_name: String,
    /// Its parent node (`None`: a child of the form).
    pub parent: Option<usize>,
    /// Left, Top, Width, Height (relative to the parent).
    pub rect: Rect,
    /// Where it is in the window's client area.
    pub abs: Rect,
    /// Visible, and every parent too.
    pub shown: bool,
    /// Enabled, and every parent too.
    pub enabled: bool,
    pub kind: Option<&'static dyn ComponentKind>,
    pub ui: NodeUi,
}

/// A form's kernel side.
pub struct FormUi {
    /// The form's id, lowercase.
    pub form: String,
    /// Its components, parents before children, siblings in creation
    /// order (the last created drawn on top).
    pub nodes: Vec<Node>,
    pub focus: Option<usize>,
    /// Under the mouse.
    pub hover: Option<usize>,
    /// A button held down (drawn pushed while the mouse is on it).
    pub pressed: Option<usize>,
    /// The node the mouse is captured by (pressed on, until released);
    /// `Some(None)`: the form itself.
    pub capture: Option<Option<usize>>,
    /// The caret shows (it blinks).
    pub caret_on: bool,
    /// Goes up at every rebuild.
    pub structure_rev: u64,
    /// The in-window main menu's height (0 without one, or with the
    /// system's menu bar).
    pub menu_offset: i64,
    /// The client area (ClientWidth × ClientHeight), under the menu.
    pub client: (i64, i64),
    /// The scale it was last painted at.
    pub scale: f64,
    /// Shown modally (its accessibility node is a dialog).
    pub modal: bool,
    /// Needs painting again.
    pub dirty: bool,
    /// Its main menu and pop-up menus as the kernel draws them (the bar,
    /// open menus): components/menubar.rs.
    pub menus: crate::components::menubar::MenuUi,
    pub(crate) menu_in_window: bool,
    pub(crate) events: Vec<KernelEvent>,
    /// (the surfaces lane's) What the form's own drawing surface shows, as
    /// last converted.
    pub(crate) surface: Option<crate::components::canvas::Shown>,
    /// Its deadlines (the caret's blink, held scroll bars): tick.rs.
    pub wakes: crate::tick::Wakes,
    /// Its tooltip (a Hint shown): tooltip.rs.
    pub tip: crate::tooltip::TipUi,
    /// The caret blinks (the host turns it off for captures: a headless
    /// host draws it steadily on).
    pub blinks: bool,
    /// Its window's bottom-right corner is the system's (the host says):
    /// rounded off and resizing the window itself — macOS' titled windows.
    /// A status bar's size grip isn't drawn there (its square keeps its
    /// place, cursor and drag).
    pub system_corner: bool,
    /// The last press: when, on what, where, and how many clicks it made.
    pub(crate) last_click: Option<(crate::tick::Instant, Option<usize>, f64, f64, u8)>,
    /// The wheel's turn not yet a whole notch (form scroll bars, lists).
    pub(crate) wheel_rest: (f64, f64),
    /// The open drop-down list and menus are drawn apart, by
    /// [`FormUi::paint_popups`], not by [`FormUi::paint`]: a host with its
    /// own elements over the window's drawing (the web's RWEBVIEW …) puts
    /// them on a layer above those.
    pub popups_apart: bool,
}

/// Visible / Enabled as the runtimes keep them (-1, True, "0" …).
fn on(store: &dyn Store, id: &str, prop: &str) -> bool {
    store::flag(store, id, prop, true)
}

/// A component's Left, Top, Width, Height (RapidQ's default size when it
/// has none).
pub fn geometry(store: &dyn Store, id: &str, type_name: &str) -> Rect {
    let (dw, dh) = rapidr_value::layout::default_size(type_name).unwrap_or((75, 25));
    (store::int(store, id, "left", 0), store::int(store, id, "top", 0), store::int(store, id, "width", dw), store::int(store, id, "height", dh))
}

/// Components the kernel doesn't place (non-visual, or windows of their
/// own).
fn placed(type_name: &str) -> bool {
    type_name != "RFORM" && type_name != "RFORMMDI" && (components::kind_of(type_name).is_some() || rapidr_value::layout::default_size(type_name).is_some())
}

impl FormUi {
    /// Form `form`'s tree. `menu_in_window`: a QMAINMENU is a bar inside
    /// the window (everywhere but macOS' system menu bar), so the form's
    /// components sit below it.
    pub fn build(store: &dyn Store, form: &str, menu_in_window: bool) -> FormUi {
        let mut f = FormUi::build_unfocused(store, form, menu_in_window);
        f.focus = f.tab_order(store).first().copied();
        // (a QEDIT focused as the form shows selects its text: AutoSelect)
        if let Some(i) = f.focus {
            crate::focus::select_on_entry(&f.nodes[i].id, &f.nodes[i].type_name);
        }
        f
    }

    /// The same with nothing focused (a form shown in the designer).
    pub fn build_unfocused(store: &dyn Store, form: &str, menu_in_window: bool) -> FormUi {
        let mut f = FormUi {
            form: form.to_lowercase(),
            nodes: Vec::new(),
            focus: None,
            hover: None,
            pressed: None,
            capture: None,
            caret_on: true,
            structure_rev: 0,
            menu_offset: 0,
            client: (0, 0),
            scale: 1.0,
            modal: false,
            dirty: true,
            menus: Default::default(),
            menu_in_window,
            events: Vec::new(),
            surface: None,
            wakes: Default::default(),
            tip: Default::default(),
            blinks: true,
            system_corner: false,
            last_click: None,
            wheel_rest: (0.0, 0.0),
            popups_apart: false,
        };
        f.rebuild(store);
        f
    }

    /// The structure changed (components added, removed, reparented): the
    /// tree again, each surviving component keeping its UI state (an
    /// editor's scroll, the focus, …).
    pub fn rebuild(&mut self, store: &dyn Store) {
        let focus = self.focus.map(|i| self.nodes[i].id.clone());
        let hover = self.hover.map(|i| self.nodes[i].id.clone());
        let pressed = self.pressed.map(|i| self.nodes[i].id.clone());
        let capture = self.capture.map(|c| c.map(|i| self.nodes[i].id.clone()));
        let mut old: Vec<(String, NodeUi)> = self.nodes.drain(..).map(|n| (n.id, n.ui)).collect();
        let form = self.form.clone();
        self.add_children(store, &form, None, &mut old);
        let find = |id: &Option<String>| id.as_ref().and_then(|id| self.index_of(id));
        let (f, h, p) = (find(&focus), find(&hover), find(&pressed));
        let c = capture.and_then(|c| match c {
            None => Some(None),
            Some(id) => self.index_of(&id).map(Some),
        });
        (self.focus, self.hover, self.pressed, self.capture) = (f, h, p, c);
        self.structure_rev += 1;
        self.dirty = true;
        self.sync(store);
    }

    fn add_children(&mut self, store: &dyn Store, parent_id: &str, parent: Option<usize>, old: &mut Vec<(String, NodeUi)>) {
        // (a QFORMMDI's child frames in their stacking order)
        // (and a dock manager's slid-out pane over its groups)
        // (and a shown command palette, a toolbox dragging, over their neighbours)
        let children = components::dock::stacked(parent_id, components::mdi::stacked(parent_id, store.children(parent_id)));
        let children = components::panels::palette::stacked(parent_id, components::panels::toolbox::stacked(parent_id, children));
        for (id, type_name) in children {
            let type_name = type_name.to_ascii_uppercase();
            if !placed(&type_name) {
                continue;
            }
            let id = id.to_lowercase();
            let ui = old.iter().position(|(o, _)| *o == id).map(|k| old.swap_remove(k).1).unwrap_or_default();
            self.nodes.push(Node { id: id.clone(), type_name: type_name.clone(), parent, rect: (0, 0, 0, 0), abs: (0, 0, 0, 0), shown: true, enabled: true, kind: components::kind_of(&type_name), ui });
            let me = self.nodes.len() - 1;
            self.add_children(store, &id, Some(me), old);
        }
    }

    /// Geometry, Visible and Enabled again from the store (before painting;
    /// the host calls it on a geometry change). Also the client area and
    /// the menu's height.
    pub fn sync(&mut self, store: &dyn Store) {
        let has_menu = store.children(&self.form).iter().any(|(_, t)| t.eq_ignore_ascii_case("RMAINMENU"));
        self.menu_offset = if has_menu && self.menu_in_window { rapidr_value::layout::MAIN_MENU_HEIGHT } else { 0 };
        self.client = client_size(store, &self.form, self.menu_offset);
        for i in 0..self.nodes.len() {
            let (id, type_name) = (self.nodes[i].id.clone(), self.nodes[i].type_name.clone());
            let rect = geometry(store, &id, &type_name);
            let (origin, shown, enabled) = match self.nodes[i].parent {
                Some(p) => {
                    let pn = &self.nodes[p];
                    // (its client area: a scroll box's inside its edge)
                    let (cx, cy, _, _) = pn.kind.map_or((0, 0, 0, 0), |k| k.client_area(store, &pn.id, pn.abs.2, pn.abs.3));
                    ((pn.abs.0 + cx, pn.abs.1 + cy), pn.shown, pn.enabled)
                }
                None => ((0, self.menu_offset), true, on(store, &self.form, "enabled")),
            };
            let n = &mut self.nodes[i];
            n.rect = rect;
            n.abs = (origin.0 + rect.0, origin.1 + rect.1, rect.2.max(0), rect.3.max(0));
            n.shown = shown && on(store, &id, "visible");
            n.enabled = enabled && on(store, &id, "enabled");
        }
        // (the focus leaves what can't have it any more)
        if let Some(f) = self.focus {
            if !self.can_focus(store, f) {
                self.focus = None;
            }
        }
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id.eq_ignore_ascii_case(id))
    }

    pub fn node(&self, id: &str) -> Option<&Node> {
        self.index_of(id).map(|i| &self.nodes[i])
    }

    /// The form's own children, in creation order.
    pub fn roots(&self) -> Vec<usize> {
        (0..self.nodes.len()).filter(|&i| self.nodes[i].parent.is_none()).collect()
    }

    pub fn children(&self, i: usize) -> Vec<usize> {
        (0..self.nodes.len()).filter(|&c| self.nodes[c].parent == Some(i)).collect()
    }

    /// Node `i` and its parents up to the form's child, innermost first.
    pub fn ancestry(&self, i: usize) -> Vec<usize> {
        let mut out = vec![i];
        let mut at = self.nodes[i].parent;
        while let Some(p) = at {
            out.push(p);
            at = self.nodes[p].parent;
        }
        out
    }

    /// The topmost shown component at a point of the client area (logical
    /// pixels, the menu bar's included): the innermost, the last created
    /// on top, only where its parents show it.
    pub fn hit(&self, x: f64, y: f64) -> Option<usize> {
        let inside = |r: Rect| x >= r.0 as f64 && y >= r.1 as f64 && x < (r.0 + r.2) as f64 && y < (r.1 + r.3) as f64;
        (0..self.nodes.len()).rev().find(|&i| self.nodes[i].shown && self.ancestry(i).iter().all(|&a| inside(self.nodes[a].abs)))
    }

    /// The events queued since the last call (for runtime-core to
    /// dispatch after the pump).
    pub fn take_events(&mut self) -> Vec<KernelEvent> {
        std::mem::take(&mut self.events)
    }

    /// The QBUTTON with Default = True that Enter clicks (enabled, shown).
    pub fn default_button(&self, store: &dyn Store) -> Option<usize> {
        self.button_with(store, "default")
    }

    /// The QBUTTON with Cancel = True that Escape clicks.
    pub fn cancel_button(&self, store: &dyn Store) -> Option<usize> {
        self.button_with(store, "cancel")
    }

    fn button_with(&self, store: &dyn Store, prop: &str) -> Option<usize> {
        (0..self.nodes.len()).find(|&i| {
            let n = &self.nodes[i];
            n.type_name == "RBUTTON" && n.shown && n.enabled && store::flag(store, &n.id, prop, false)
        })
    }

    /// The parley layout of node `id`'s editor (what a display list's
    /// [`crate::TextItem`] draws).
    pub fn editor_layout(&self, id: &str) -> Option<&parley::Layout<crate::text::Ink>> {
        self.editor_layout_at(id, 0)
    }

    /// Paragraph `para`'s layout of node `id`'s editor (a memo has one per
    /// paragraph).
    pub fn editor_layout_at(&self, id: &str, para: usize) -> Option<&parley::Layout<crate::text::Ink>> {
        match self.node(id) {
            Some(n) => {
                if let Some(code) = &n.ui.code {
                    // (a code editor's rows: `para` is the layout's cache slot)
                    return code.cache.layout(para);
                }
                n.ui.edit.as_ref()?.para_layout(para)
            }
            // (a component of a form shown in a designer on this one)
            None => self.nodes.iter().filter_map(|n| n.ui.design.as_ref()).find_map(|v| v.editor_layout_at(id, para)),
        }
    }
}

/// A form's client area: ClientWidth × ClientHeight (and its scroll bars)
/// when the store knows them, else its Width × Height less the frame (BorderStyle, default
/// bsSizeable) and the menu.
pub fn client_size(store: &dyn Store, form: &str, menu: i64) -> (i64, i64) {
    let cw = store.get(form, "clientwidth");
    let ch = store.get(form, "clientheight");
    if !matches!(cw, rapidr_value::Value::Null) && !matches!(ch, rapidr_value::Value::Null) {
        // (ClientWidth / ClientHeight leave out the scroll bars shown; the
        // window's inside has them)
        let (bw, bh) = components::scrollbox::bars_taken(form);
        return (cw.to_i64().max(0) + bw, ch.to_i64().max(0) + bh);
    }
    let (dw, dh) = rapidr_value::layout::default_size("RFORM").unwrap_or((320, 240));
    let (w, h) = (store::int(store, form, "width", dw), store::int(store, form, "height", dh));
    rapidr_value::layout::form_client_size(w, h, store::int(store, form, "borderstyle", 2), menu)
}
