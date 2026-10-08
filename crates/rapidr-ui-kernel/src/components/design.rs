//! RDESIGNSURFACE (I4, lane L-DVIEW): RapidR Studio's form designer — the
//! shared model (`rapidr_value::objects::design` on the designer model:
//! the designed form, the selection, the grid- and guide-snapped moves and
//! resizes, the resize preview) drawn and driven by the kernel.
//!
//! - **WYSIWYG.** The designed form is drawn as the running program draws
//!   it: by the kernel's own components ([`super::KINDS`]: a QLABEL by the
//!   label's code, a QSTRINGGRID by the grid's …), from a **design-time
//!   store** ([`DesignStore`]: the form's CREATE blocks replayed as the
//!   runtimes' `rp_comp_set` stores them, each component where the
//!   designer's layout puts it — the runtimes' own layout engine), in a
//!   [`FormUi`] of its own ([`View`]), in the theme and at the scale of the
//!   surface. Around it: the form's frame and title bar
//!   ([`crate::frame`], as the web host draws a window's), its main menu
//!   (the kernel's menu bar), a backdrop, and under it a tray strip of
//!   the non-visual components (each one's icon — `rapidr_icons`'s
//!   component icons — and name). The grid's dots go on the form's face
//!   under the components; the designer's chrome (the selection, handles,
//!   anchor pins, guides, the rubber band, the form's corner grip) over
//!   everything ([`rapidr_value::objects::design::DesignSurface::
//!   chrome_ops`]).
//! - **The mouse** (the left button) is the designer's, in the form's client
//!   coordinates: a press grabs a handle, a pin or the form's corner, or
//!   selects the component under it (OnSelect; OnDblClick for a double
//!   click's second press), or clears the selection (OnBgClick with the
//!   point); a drag moves or resizes (OnMove at each step); the release
//!   ends it. The designed components never see it.
//! - **The placing tool** (PlaceType) shows where the new component would
//!   go under the mouse; a click places it at its default size, a drag
//!   draws its rectangle. A component **dragged in** from elsewhere
//!   (DragComponent: a toolbox) shows the same over any design surface the
//!   mouse crosses, and is added where the mouse is let go ([`drop_move`],
//!   [`drop_up`], called by the kernel's input before the component under
//!   the mouse hears it).
//! - **The keyboard** (it takes the focus): arrows nudge, Shift by the grid,
//!   Ctrl / ⌘ resize; Delete; Tab / Shift+Tab through the components; Esc;
//!   Enter (the default event's handler); Ctrl / ⌘ + Z, Y, A, C, X, V, D.
//! - What the surface's model leaves to hear (OnSelect, OnSourceEdit,
//!   OnChange …) is fired after each of these; it fires no OnClick, and the
//!   program hears no OnMouseDown / Move / Up of it: runtime-core drops
//!   those.
//!
//! A screen reader sees a list box whose options are the designed
//! components ("Button1 (QBUTTON), 16, 24, 75 × 25"), the selected ones
//! selected; clicking one selects it (OnSelect); a live region (a status)
//! says what each change did.

use std::collections::HashMap;

use rapidr_value::designer::{FormDesign, Item};
use rapidr_value::objects::a11y::{node_id, part_id, AccessNode, Action, Role, PART_ITEM, PART_STATUS};
use rapidr_value::objects::design::{DesignEvent, DesignSurface, TrayItem, TRAY_ICON};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Op, Place, Rect};
use rapidr_value::objects::{with_design, with_design_mut};
use rapidr_value::{input::Button, v_int, v_str, Value};

use super::list::{act, ListAction};
use super::{ComponentKind, Cx, KeyIn, MouseIn, MouseKind, MouseOut};
use crate::input::Clipboard;
use crate::a11y::AccessValue;
use crate::paint::{color_of, Painter};
use crate::store::{MemStore, Store};
use crate::tree::FormUi;

pub struct Design;

/// What the program hears, queued (after the pump, as its handlers run).
fn heard(cx: &mut Cx, e: DesignEvent) {
    act(cx, ListAction::Fire(e.event().to_string(), e.args()));
}

/// Everything the surface's model left to hear, queued (and the change
/// hook told).
fn drain(cx: &mut Cx) {
    for e in rapidr_value::objects::take_design_events(cx.id) {
        heard(cx, e);
    }
}

// ------------------------------------------------- the design-time store --

/// The designed form's components as the kernel reads them: a [`MemStore`]
/// filled as the runtimes fill their registries — each component made with
/// its type's creation defaults (`component_defaults::creation`, the
/// runtimes' `RpComponent::new`), its CREATE block's assignments and
/// statements replayed as `rp_comp_set` / `rp_comp_method` store them
/// (fonts kept under both spellings, `Kind`'s caption, a QBEVEL's bevels,
/// the shared models' properties and methods: a list's AddItems, a grid's
/// Cell …), and read back as `RtStore` / `WebStore` read them (a Font.Color
/// the component didn't set is its parent's, RapidQ's ParentFont; a
/// panel's bevels and the Anchors / Constraints default as RapidQ's).
///
/// Ids are the surface's id, `:` and the component's name, lowercase: they
/// never meet a program's components (`:` is in no BASIC name).
pub struct DesignStore {
    mem: MemStore,
    /// What each component starts with (the runtimes' creation defaults).
    defaults: HashMap<String, HashMap<String, Value>>,
}

impl DesignStore {
    fn new() -> Self {
        DesignStore { mem: MemStore::new(), defaults: HashMap::new() }
    }

    /// A component made, as the runtimes make one.
    fn create(&mut self, id: &str, type_name: &str, parent: Option<&str>) {
        self.mem.add(id, type_name, parent);
        let mut d: HashMap<String, Value> = rapidr_value::component_defaults::creation(type_name).into_iter().collect();
        if let Some((w, h)) = rapidr_value::layout::default_size(type_name) {
            d.insert("width".into(), v_int(w));
            d.insert("height".into(), v_int(h));
        }
        self.defaults.insert(id.to_lowercase(), d);
        if let Some(p) = parent {
            // (a nested CREATE sets its Parent: a menu item hangs under its menu)
            self.mem.set(id, "parent", v_str(p));
        }
    }

    /// The value stored, without the runtimes' read rules.
    fn raw(&self, id: &str, prop: &str) -> Value {
        match self.mem.get(id, prop) {
            Value::Null => self.defaults.get(&id.to_lowercase()).and_then(|d| d.get(prop)).cloned().unwrap_or(Value::Null),
            v => v,
        }
    }

    /// An assignment of a CREATE block, as `rp_comp_set` stores it.
    fn set(&mut self, id: &str, type_name: &str, prop: &str, val: Value) {
        let prop = prop.to_ascii_lowercase();
        let val = rapidr_value::layout::property_value(&prop, val);
        // (QBUTTON Kind: its caption and ModalResult)
        if prop == "kind" {
            if let Some((caption, mr)) = rapidr_value::events::button_kind(val.to_i64()) {
                let now = self.raw(id, "caption").to_string_val();
                if now.is_empty() || (1..=10).any(|k| rapidr_value::events::button_kind(k).map(|(c, _)| c) == Some(now.as_str())) {
                    self.set(id, type_name, "caption", v_str(caption));
                }
                self.set(id, type_name, "modalresult", v_int(mr));
            }
        }
        // (a QGLASSFRAME's Transparency …, as RC.EXE stores them)
        let val = if type_name == "RGLASSFRAME" { rapidr_value::objects::glass::stored(&prop, &val).unwrap_or(val) } else { val };
        // (a QBEVEL's Shape / Style set its bevels)
        if type_name == "RBEVEL" {
            let other = self.raw(id, if prop == "shape" { "style" } else { "shape" }).to_i64();
            if let Some(bevels) = rapidr_value::objects::bevel::qbevel_set(&prop, val.to_i64(), other) {
                for (p, v) in bevels {
                    self.mem.set(id, p, v_int(v));
                }
            }
        }
        // (the program chose a Font.Color: ParentFont no longer applies)
        if matches!(prop.as_str(), "font.color" | "fontcolor") {
            self.mem.set(id, "__fontcolorset", Value::Boolean(true));
        }
        if rapidr_value::objects::exists(id) && rapidr_value::objects::set(id, &prop, &val).is_some_and(|r| r.is_ok()) {
            return;
        }
        // (a font property under both spellings, as the runtimes keep them)
        for (dotted, flat) in [("font.name", "fontname"), ("font.size", "fontsize"), ("font.bold", "fontbold"), ("font.italic", "fontitalic"), ("font.color", "fontcolor")] {
            if prop == dotted || prop == flat {
                self.mem.set(id, dotted, val.clone());
                self.mem.set(id, flat, val.clone());
            }
        }
        if prop.starts_with("font") && prop != "font" {
            self.mem.set(id, "__fontset", Value::Boolean(true));
        }
        if prop == "color" {
            self.mem.set(id, "__colorset", Value::Boolean(true));
        }
        self.mem.set(id, &prop, val);
    }

    /// A statement of a CREATE block (`AddItems "a"`, `Cell(1, 1) = "x"`,
    /// `Panel(0).Width = 80`), as `rp_comp_method` runs it: the shared
    /// models' methods, a status bar's AddPanels, an indexed sub-object's
    /// member kept as `panel(0).width`, a form's Add / DelBorderIcons.
    fn call(&mut self, id: &str, type_name: &str, method: &str, args: &[Value]) {
        let method = method.to_ascii_lowercase();
        let reader = |_: &str, _: &str| Value::Null;
        if rapidr_value::objects::call(id, &method, args, &reader).is_some() {
            return;
        }
        if let Some((sub, member)) = method.split_once('.') {
            if let Some(member) = member.strip_suffix('=').filter(|m| !m.contains('.') && !sub.is_empty()) {
                let Some((value, index)) = args.split_last() else { return };
                let index: Vec<String> = index.iter().map(Value::to_string_val).collect();
                self.set(id, type_name, &format!("{sub}({}).{member}", index.join(",")), value.clone());
            }
            return;
        }
        match (type_name, method.as_str()) {
            ("RSTATUSBAR", "addpanels") => {
                let mut n = self.raw(id, "panelcount").to_i64().max(0);
                for a in args {
                    self.set(id, type_name, &format!("panel({n}).caption"), v_str(&a.to_string_val()));
                    n += 1;
                }
                self.set(id, type_name, "panelcount", v_int(n));
            }
            ("RFORM", "addbordericons" | "delbordericons") => {
                let bits = rapidr_value::builtins::border_icons(&self.raw(id, "bordericons"), args, method == "addbordericons");
                self.set(id, type_name, "bordericons", bits);
            }
            _ => {}
        }
    }

    /// Forgets the components and their models.
    fn clear(&mut self) {
        for id in self.mem.ids().to_vec() {
            rapidr_value::scrollbars::remove(&id);
        }
        self.mem.clear();
        self.defaults.clear();
    }
}

impl Store for DesignStore {
    fn get(&self, id: &str, prop: &str) -> Value {
        let prop = prop.to_ascii_lowercase();
        // (a Font.Color the component didn't set: its parent's, RapidQ's
        // ParentFont — RtStore's drawn_font_color)
        if matches!(prop.as_str(), "fontcolor" | "font.color") {
            let mut at = id.to_string();
            for _ in 0..32 {
                if self.raw(&at, "__fontcolorset").to_bool() {
                    return self.raw(&at, &prop);
                }
                let parent = self.raw(&at, "parent").to_string_val();
                if parent.is_empty() || parent.eq_ignore_ascii_case(&at) || self.mem.type_of(&parent).is_empty() {
                    break;
                }
                at = parent;
            }
        }
        match self.raw(id, &prop) {
            Value::Null => {
                let ty = self.mem.type_of(id);
                if matches!(ty.as_str(), "RPANEL" | "RBEVEL") {
                    if let Some(v) = rapidr_value::objects::bevel::default(&prop) {
                        return v_int(v);
                    }
                }
                if let Some(v) = rapidr_value::layout::default_property(&ty, &prop) {
                    return v_int(v);
                }
                if matches!(prop.as_str(), "left" | "top" | "right" | "bottom") {
                    v_int(0)
                } else {
                    Value::Null
                }
            }
            v => v,
        }
    }

    fn type_of(&self, id: &str) -> String {
        self.mem.type_of(id)
    }

    fn children(&self, id: &str) -> Vec<(String, String)> {
        self.mem.children(id)
    }

    fn font(&self, id: &str) -> Font {
        rapidr_value::objects::font_from_props(id, &|i, p| self.raw(i, p))
    }
}

/// A component's id in the design-time store.
fn key(surface: &str, name: &str) -> String {
    format!("{}:{}", surface.to_lowercase(), name.to_lowercase())
}

/// A value as the designer reads it, as a program would have it.
fn value_of(design: &FormDesign, text: &str) -> Option<Value> {
    use rapidr_value::designer::value::PropValue;
    match design.read_value(text) {
        PropValue::Number(f) if f.fract() == 0.0 && f.abs() < 9.0e15 => Some(Value::Integer(f as i64)),
        PropValue::Number(f) => Some(Value::Double(f)),
        PropValue::Str(s) => Some(Value::String(s)),
        _ => None,
    }
}

/// Splits `text` at its first `sep` outside quotes and parentheses.
fn split_top(text: &str, sep: char) -> Option<(&str, &str)> {
    let (mut depth, mut quoted) = (0i32, false);
    for (i, c) in text.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            c if c == sep && !quoted && depth == 0 => return Some((&text[..i], &text[i + c.len_utf8()..])),
            _ => {}
        }
    }
    None
}

/// Comma-separated arguments, each a value the designer reads.
fn args_of(design: &FormDesign, text: &str) -> Option<Vec<Value>> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    if rest.is_empty() {
        return Some(out);
    }
    loop {
        let (arg, tail) = split_top(rest, ',').unwrap_or((rest, ""));
        out.push(value_of(design, arg)?);
        if tail.is_empty() && split_top(rest, ',').is_none() {
            return Some(out);
        }
        rest = tail.trim();
    }
}

/// A statement of a CREATE block the designer replays, as the runtimes
/// call it: `AddItems "a", "b"` → (`additems`, ["a", "b"]); `Cell(1, 2) =
/// "x"` → (`cell`, [1, 2, "x"]); `Panel(0).Width = 80` → (`panel.width=`,
/// [0, 80]). `None`: not one it can (an expression it can't read, IF …).
pub fn statement(design: &FormDesign, text: &str) -> Option<(String, Vec<Value>)> {
    let t = text.trim();
    let ident = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if let Some((lhs, rhs)) = split_top(t, '=') {
        // (an indexed property: Name(args)[.Member] = value)
        let lhs = lhs.trim();
        let open = lhs.find('(')?;
        let close = lhs.rfind(')')?;
        let name = lhs[..open].trim();
        let member = lhs[close + 1..].trim();
        if !ident(name) || name.contains('.') || !(member.is_empty() || ident(member.trim_start_matches('.')) && member.starts_with('.')) {
            return None;
        }
        let mut args = args_of(design, &lhs[open + 1..close])?;
        args.push(value_of(design, rhs)?);
        let method = if member.is_empty() { name.to_ascii_lowercase() } else { format!("{}{}=", name.to_ascii_lowercase(), member.to_ascii_lowercase()) };
        return Some((method, args));
    }
    let end = t.find(|c: char| c == ' ' || c == '(').unwrap_or(t.len());
    let name = &t[..end];
    if !ident(name) {
        return None;
    }
    let rest = t[end..].trim();
    let inner = match rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        Some(inner) if split_top(rest, ')').is_some_and(|(_, after)| after.is_empty()) => inner,
        _ => rest,
    };
    Some((name.to_ascii_lowercase(), args_of(design, inner)?))
}

/// Whether the kernel draws components of this type on a form (else the
/// tray strip shows it, or it's a menu the menu bar shows).
fn drawn(type_name: &str) -> bool {
    super::kind_of(type_name).is_some() || rapidr_value::layout::default_size(type_name).is_some()
}

// ----------------------------------------------------------- the view --

/// An RDESIGNSURFACE's designed form as the kernel draws it: the
/// design-time store of its components and the form's own tree (kept in
/// the surface's [`crate::tree::NodeUi`], rebuilt when the design changes;
/// a drag or the resize preview only moves components in the store).
pub struct View {
    /// The design the store was filled from.
    built: Option<FormDesign>,
    store: DesignStore,
    /// The form's id in the store.
    form: String,
    ui: Option<FormUi>,
}

impl View {
    fn new() -> View {
        View { built: None, store: DesignStore::new(), form: String::new(), ui: None }
    }

    /// The design-time store (what the designed form is drawn from).
    pub fn store(&self) -> &DesignStore {
        &self.store
    }

    /// The designed form's tree.
    pub fn form(&self) -> Option<&FormUi> {
        self.ui.as_ref()
    }

    /// An editor's layout of a designed component (the renderer finds a
    /// display list's editor items through the window's form).
    pub fn editor_layout_at(&self, id: &str, para: usize) -> Option<&parley::Layout<crate::text::Ink>> {
        self.ui.as_ref()?.editor_layout_at(id, para)
    }

    /// The store filled again from `design`: every component as its
    /// CREATE block makes it.
    fn rebuild(&mut self, surface: &str, design: &FormDesign) {
        self.store.clear();
        let root = design.root();
        for id in design.ids() {
            let Some(n) = design.node(id) else { continue };
            let ty = n.canonical.clone();
            let is_root = id == root;
            let menu = matches!(ty.as_str(), "RMAINMENU" | "RMENUITEM");
            if !is_root && !menu && !(n.is_visual() && drawn(&ty)) {
                continue;
            }
            // (a QFORM inside the form is a child window: drawn as a form is not)
            let ty = if is_root { "RFORM".to_string() } else { ty };
            let k = key(surface, &n.name);
            let parent = n.parent.and_then(|p| design.node(p)).map(|p| key(surface, &p.name));
            self.store.create(&k, &ty, parent.as_deref());
            for item in &n.body {
                match item {
                    Item::Prop(p) => {
                        let prop = rapidr_value::designer::model::prop_key(&p.name);
                        // (event handlers — bare names — are bound, not stored:
                        // `value_of` reads none; what's hidden shows at design
                        // time, as in Delphi's designer)
                        if prop == "visible" || prop == "parent" {
                            continue;
                        }
                        if let Some(v) = value_of(design, &p.value) {
                            self.store.set(&k, &ty, &prop, v);
                        }
                    }
                    Item::Code(text) => {
                        if let Some((method, args)) = statement(design, text) {
                            self.store.call(&k, &ty, &method, &args);
                        }
                    }
                    Item::Child(_) => {}
                }
            }
        }
        self.form = key(surface, &design.node(root).map(|n| n.name.clone()).unwrap_or_default());
        self.built = Some(design.clone());
        self.ui = None;
    }

    /// Each component's place (and the form's size) as the designer shows
    /// them now ([`placed`]), its scroll bars as laid out.
    fn place(&mut self, surface: &str, placed: Vec<Placed>) {
        for (name, r, scroller) in placed {
            let k = key(surface, &name);
            if !self.store.mem.ids().contains(&k) {
                continue;
            }
            for (p, v) in [("left", r.left), ("top", r.top), ("width", r.width), ("height", r.height)] {
                if self.store.raw(&k, p).to_i64() != v || matches!(self.store.mem.get(&k, p), Value::Null) {
                    self.store.mem.set(&k, p, v_int(v));
                }
            }
            match scroller {
                Some(s) => rapidr_value::scrollbars::with_mut(&k, |b| *b = s),
                None => rapidr_value::scrollbars::remove(&k),
            }
        }
    }
}

/// A component's name, its place in its parent (Left, Top, Width, Height)
/// and its scroll bars (a form's, a scroll box's).
type Placed = (String, rapidr_value::layout::Rect, Option<rapidr_value::scrollbars::Scroller>);

/// Every component's place as the designer shows it now (the layout's, a
/// drag's, the preview's).
fn placed(d: &DesignSurface) -> Vec<Placed> {
    let design = &d.designer.design;
    let layout = d.layout();
    d.placed().into_iter().filter_map(|(id, r)| Some((design.node(id)?.name.clone(), r, layout.scroller(id)))).collect()
}

/// What the surface's paint needs of its model (read without holding it:
/// filling the store makes the shared models).
struct Shown {
    changed: Option<FormDesign>,
    title: String,
    border: bool,
    form: Rect,
    client: (i64, i64),
    tray: Vec<TrayItem>,
    tray_rect: Option<Rect>,
    chrome: Vec<Op>,
    placed: Vec<Placed>,
}

impl ComponentKind for Design {
    fn name(&self) -> &'static str {
        "RDESIGNSURFACE"
    }

    fn focusable(&self, _store: &dyn Store, _id: &str) -> bool {
        true
    }

    fn paint(&self, cx: &mut Cx, p: &mut Painter) {
        let (w, h) = (cx.width(), cx.height());
        let mut view = cx.ui.design.take().unwrap_or_else(|| Box::new(View::new()));
        // (the surface sizes a form made through the API)
        with_design_mut(cx.id, |d| d.set_size(w, h));
        let Some(mut shown) = with_design(cx.id, |d| Shown {
            changed: (view.built.as_ref() != Some(&d.designer.design)).then(|| d.designer.design.clone()),
            title: d.title(),
            border: d.border(),
            form: d.form_rect(),
            client: d.client_origin(),
            tray: d.tray(),
            tray_rect: d.tray_rect(),
            chrome: d.chrome_ops(),
            placed: placed(d),
        }) else {
            cx.ui.design = Some(view);
            return;
        };
        if let Some(design) = &shown.changed {
            view.rebuild(cx.id, design);
        }
        view.place(cx.id, std::mem::take(&mut shown.placed));
        let t = p.theme();
        // the backdrop
        p.fill((0, 0, w, h), t.shadow);
        // (no form in the source: the surface says so)
        if let Some(text) = with_design(cx.id, |d| d.empty_text()).flatten() {
            let font = rapidr_value::objects::design::tray_font();
            let lines: Vec<&str> = text.lines().collect();
            let lh = 20;
            let top = (h - lh * lines.len() as i64) / 2;
            for (k, line) in lines.iter().enumerate() {
                p.text((16, top + k as i64 * lh, (w - 32).max(0), lh), line, &font, t.text, Place::Center);
            }
            cx.ui.design = Some(view);
            return;
        }
        // the form's window: its frame, then its inside (menu bar, client)
        let (fx, fy, fw, fh) = shown.form;
        let form_id = view.form.clone();
        let look = crate::frame::Look {
            title: shown.title.clone(),
            active: true,
            border: shown.border,
            frame: crate::frame::frame_of(view.store.get(&form_id, "borderstyle").to_i64().max(if shown.border { 1 } else { 0 }), match view.store.get(&form_id, "bordericons") {
                Value::Null => crate::frame::BI_DEFAULT,
                v => v.to_i64(),
            }),
            maximized: false,
            icon: crate::frame::icon_of(&view.store, &form_id),
        };
        p.at((fx, fy), |p| crate::frame::paint_into(p, &look, (fw, fh)));
        let (ix, iy) = crate::frame::inset(shown.border);
        let (ox, oy) = (fx + ix, fy + iy);
        let face = color_of(&view.store, &form_id).unwrap_or(t.face);
        {
            let View { ui, store, form, .. } = &mut *view;
            let f = ui.get_or_insert_with(|| {
                let mut f = FormUi::build_unfocused(store, form, true);
                f.blinks = false;
                f
            });
            f.sync(store);
            let (iw, ih) = (f.client.0, f.client.1 + f.menu_offset);
            let (gw, gh) = f.client;
            // (the grid's dots on the form's face, under its components)
            let dots = with_design(cx.id, |d| d.grid_ops(gw, gh, face)).unwrap_or_default();
            p.at((ox, oy), |p| {
                p.clipped((0, 0, iw, ih), |p| {
                    let mut under = |p: &mut Painter| p.ops(dots.iter().cloned());
                    f.paint_into(store, cx.text, p, &mut under);
                });
            });
        }
        // the tray strip of non-visual components
        if let Some((tx, ty, tw, th)) = shown.tray_rect {
            let (sx, sy) = shown.client;
            p.at((sx, sy), |p| {
                p.fill((tx, ty, tw, th), t.face);
                p.frame((tx, ty, tw, th), if t.fluent() { t.border } else { t.shadow });
                let font = rapidr_value::objects::design::tray_font();
                for item in &shown.tray {
                    let (x, y, iw, ih) = item.rect;
                    p.icon(&item.type_name, (x + 4, y + (ih - TRAY_ICON) / 2, TRAY_ICON, TRAY_ICON), None, false);
                    p.text((x + 4 + TRAY_ICON + 4, y, iw - TRAY_ICON - 8, ih), &item.name, &font, t.text, Place::Left);
                }
            });
        }
        // the designer's chrome over everything
        p.at(shown.client, |p| p.ops(shown.chrome));
        // (the code has errors: a banner says why nothing can change)
        if let Some(text) = with_design(cx.id, |d| d.banner()).flatten() {
            let bg = if t.dark { 0x43_35_19 } else { 0xFF_F4_CE };
            p.fill((0, 0, w, 28), bg);
            p.fill((0, 27, w, 1), t.shadow);
            let font = rapidr_value::objects::design::tray_font();
            p.text((10, 0, (w - 20).max(0), 28), &text, &font, t.text, Place::Left);
        }
        cx.ui.design = Some(view);
    }

    fn mouse(&self, cx: &mut Cx, m: &MouseIn) -> MouseOut {
        let out = MouseOut { press: false, focus: Some(true) };
        let (w, h) = (cx.width(), cx.height());
        with_design_mut(cx.id, |d| d.set_size(w, h));
        let (ox, oy) = with_design(cx.id, |d| d.client_origin()).unwrap_or((0, 0));
        let (x, y) = (m.x.floor() as i64 - ox, m.y.floor() as i64 - oy);
        if m.kind == MouseKind::Leave {
            with_design_mut(cx.id, |d| d.mouse_leave());
            return out;
        }
        if m.kind == MouseKind::Move && !m.captured {
            with_design_mut(cx.id, |d| d.mouse_hover(x, y, m.mods.alt));
            return out;
        }
        if m.button != Button::Left {
            return out;
        }
        // (Shift / Ctrl / Cmd+click: in or out of the selection; Alt /
        // Option: no snapping)
        let add = m.mods.shift || m.mods.ctrl || m.mods.command;
        let e = match m.kind {
            MouseKind::Down => with_design_mut(cx.id, |d| d.mouse_down_keys(x, y, m.clicks >= 2, add, m.mods.shift)),
            MouseKind::Move if m.captured => with_design_mut(cx.id, |d| d.mouse_drag_with(x, y, m.mods.alt)),
            MouseKind::Up => with_design_mut(cx.id, |d| {
                d.mouse_up();
                None
            }),
            _ => None,
        };
        if let Some(e) = e.flatten() {
            let notify = [e.clone()];
            rapidr_value::objects::design::notify(cx.id, &notify);
            heard(cx, e);
        }
        drain(cx);
        out
    }

    fn key(&self, cx: &mut Cx, k: &KeyIn, clip: &mut dyn Clipboard) -> bool {
        let ctrl = k.mods.ctrl || k.mods.command;
        let handled = match (ctrl, k.vk) {
            // Copy / Cut: the CREATE blocks' text on the clipboard
            (true, 67) | (true, 88) => {
                let text = with_design_mut(cx.id, |d| if k.vk == 67 { d.copy() } else { d.cut() }).flatten();
                if let Some(t) = &text {
                    clip.set_text(t);
                }
                text.is_some()
            }
            _ if k.mods.alt => false,
            _ => with_design_mut(cx.id, |d| d.key(k.vk, k.text, k.mods.shift, ctrl)).unwrap_or(false),
        };
        drain(cx);
        handled
    }

    fn wheel(&self, cx: &mut Cx, dx: f64, dy: f64, mods: crate::input::Mods) -> bool {
        // (Shift turns the wheel sideways, as Windows' scroll views do)
        let (dx, dy) = if mods.shift && dx == 0.0 { (dy, 0.0) } else { (dx, dy) };
        let step = 48.0;
        with_design_mut(cx.id, |d| d.scroll_by((dx * step).round() as i64, (dy * step).round() as i64)).unwrap_or(false)
    }

    fn describe(&self, cx: &mut Cx) -> AccessNode {
        let mut n = AccessNode::new(node_id(cx.id), Role::ListBox);
        n.name = with_design(cx.id, |d| d.title()).unwrap_or_default();
        n.bounds = cx.rect;
        let (x0, y0) = (cx.rect.0, cx.rect.1);
        let comps = with_design(cx.id, |d| {
            let sel = d.selected();
            let (ox, oy) = d.client_origin();
            let tray = d.tray();
            d.components()
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let (x, y, w, h) = tray.iter().find(|t| t.index == i).map_or(c.bounds(), |t| t.rect);
                    let place = if c.visual { format!(", {}, {}, {} × {}", c.x, c.y, c.w, c.h) } else { String::new() };
                    (format!("{} ({}){place}", c.name, c.type_name), (x + ox, y + oy, w, h), sel.contains(&i))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
        for (i, (name, (x, y, w, h), selected)) in comps.into_iter().enumerate() {
            let mut o = AccessNode::new(part_id(cx.id, PART_ITEM, i), Role::ListBoxOption);
            o.name = name;
            o.states.selected = Some(selected);
            o.actions = vec![Action::Click];
            o.bounds = (x0 + x, y0 + y, w, h);
            n.children.push(o);
        }
        // (what the last change did, said politely)
        let said = with_design(cx.id, |d| d.announcement.clone()).unwrap_or_default();
        if !said.is_empty() {
            let mut live = AccessNode::new(part_id(cx.id, PART_STATUS, 0), Role::Status);
            live.name = said;
            live.bounds = (x0, y0 + cx.rect.3 - 1, 1, 1);
            n.children.push(live);
        }
        n
    }

    fn access(&self, cx: &mut Cx, action: Action, part: Option<usize>, _value: Option<&AccessValue>) -> bool {
        let (Action::Click, Some(i)) = (action, part) else { return false };
        let picked = with_design_mut(cx.id, |d| d.select(i));
        if picked == Some(true) {
            heard(cx, DesignEvent::Select(i as i64));
        }
        true
    }
}

// ------------------------------------- a component dragged in (a toolbox) --

/// The mouse moved to (x, y) of its form while a component is being
/// dragged in (`design::begin_drop`): the design surface under it (`hit`)
/// shows where it would go; the others show nothing.
pub fn drop_move(ui: &mut FormUi, store: &dyn Store, hit: Option<usize>, x: f64, y: f64, free: bool) {
    let Some(ty) = rapidr_value::objects::design::drop_pending() else { return };
    let over = hit.filter(|&i| store.type_of(&ui.nodes[i].id) == "RDESIGNSURFACE");
    for i in 0..ui.nodes.len() {
        let id = ui.nodes[i].id.clone();
        if store.type_of(&id) != "RDESIGNSURFACE" {
            continue;
        }
        let (rx, ry, _, _) = ui.nodes[i].abs;
        let (ox, oy) = with_design(&id, |d| d.client_origin()).unwrap_or((0, 0));
        let (cx, cy) = (x.floor() as i64 - rx - ox, y.floor() as i64 - ry - oy);
        with_design_mut(&id, |d| {
            if over == Some(i) && !d.no_form() {
                let (cw, ch) = d.client_size();
                d.ghost = (cx >= 0 && cy >= 0 && cx < cw && cy < ch).then(|| (d.new_rect(&ty, cx, cy, free), ty.clone()));
            } else if d.place_type.is_empty() {
                d.ghost = None;
            }
        });
    }
    ui.dirty = true;
}

/// The mouse let go at (x, y) while a component was being dragged in: added
/// where the design surface under it showed it (its events queued); the
/// drag ends wherever it was let go.
pub fn drop_up(ui: &mut FormUi, store: &dyn Store, hit: Option<usize>, x: f64, y: f64, free: bool) {
    let Some(ty) = rapidr_value::objects::design::drop_pending() else { return };
    rapidr_value::objects::design::end_drop();
    for i in 0..ui.nodes.len() {
        let id = ui.nodes[i].id.clone();
        if store.type_of(&id) != "RDESIGNSURFACE" {
            continue;
        }
        if hit == Some(i) {
            let (rx, ry, _, _) = ui.nodes[i].abs;
            let (ox, oy) = with_design(&id, |d| d.client_origin()).unwrap_or((0, 0));
            let (cx, cy) = (x.floor() as i64 - rx - ox, y.floor() as i64 - ry - oy);
            with_design_mut(&id, |d| {
                let r = d.new_rect(&ty, cx, cy, free);
                d.ghost = None;
                d.add_at(&ty, (r.left, r.top), Some(r))
            });
            for e in rapidr_value::objects::take_design_events(&id) {
                ui.events.push(crate::input::KernelEvent::List(id.clone(), ListAction::Fire(e.event().to_string(), e.args())));
            }
            ui.focus = Some(i);
        } else {
            with_design_mut(&id, |d| d.ghost = None);
        }
    }
    ui.dirty = true;
}

/// Escape while a component is being dragged in: the drag ends, nothing
/// added; whether there was one.
pub fn drop_cancel(ui: &mut FormUi, store: &dyn Store) -> bool {
    if rapidr_value::objects::design::drop_pending().is_none() {
        return false;
    }
    rapidr_value::objects::design::end_drop();
    for n in &ui.nodes {
        if store.type_of(&n.id) == "RDESIGNSURFACE" {
            with_design_mut(&n.id, |d| {
                if d.place_type.is_empty() {
                    d.ghost = None;
                }
            });
        }
    }
    ui.dirty = true;
    true
}

/// Whether node `i` is a design surface whose placing tool shows under the
/// mouse (it's drawn again as the mouse moves).
pub fn follows_mouse(ui: &FormUi, store: &dyn Store, i: usize) -> bool {
    let id = &ui.nodes[i].id;
    store.type_of(id) == "RDESIGNSURFACE" && with_design(id, |d| !d.place_type.is_empty() || d.ghost.is_some()).unwrap_or(false)
}

/// Whether the focused component is a design surface with a form (it takes
/// Tab: the next component, not the next control).
pub fn takes_tab(ui: &FormUi, store: &dyn Store) -> bool {
    ui.focus.is_some_and(|f| store.type_of(&ui.nodes[f].id) == "RDESIGNSURFACE" && with_design(&ui.nodes[f].id, |d| !d.no_form() && d.get("compcount").is_some_and(|c| c.to_i64() > 0)).unwrap_or(false))
}

#[cfg(test)]
mod tests;
