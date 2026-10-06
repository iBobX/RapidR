//! Where a designed form's components are: the CREATE blocks run as the
//! program runs them, through the runtimes' own layout
//! ([`crate::layout::engine`] — Align, Anchors, Constraints, a form's
//! frame and scroll bars), so the designer shows each component where the
//! running program puts it, and the resize preview ([`Layout::resize`])
//! moves anchored and aligned components exactly as resizing the running
//! form does (`RAPIDR_TEST_RESIZE`, a user dragging its corner).
//!
//! The replay ([`Layout::of`]): each component is created with its type's
//! defaults (`layout::default_size`, `default_align`), given its parent,
//! then its CREATE block's assignments run in order — nested CREATEs where
//! they are in the block. Values the designer can't read (`Width =
//! Screen.Width / 2`) are skipped: such a component is where its readable
//! properties put it.

use std::collections::{HashMap, HashSet};

use super::model::{prop_key, FormDesign, Item, NodeId};
use crate::layout::engine::{self, LayoutStore};
use crate::layout::{self as lay, Align, AnchorRules, Constraints, Rect, DEFAULT_ANCHORS};
use crate::scrollbars::{self, Scroller};
use crate::Value;

/// What the layout keeps of a component.
#[derive(Clone, Debug)]
struct Comp {
    id: NodeId,
    ty: String,
    parent: String,
    rect: Rect,
    align: Align,
    visible: bool,
    anchors: Option<i64>,
    constraints: Constraints,
    border_style: Option<i64>,
    /// Its other properties as the runtimes store them (Caption, AutoSize,
    /// the font …: what a QLABEL's AutoSize reads).
    props: HashMap<String, Value>,
}

/// The designed form laid out: every component's rectangle (its Left / Top
/// / Width / Height, in its parent's client coordinates).
#[derive(Clone, Debug)]
pub struct Layout {
    comps: HashMap<String, Comp>,
    /// Keys in creation order.
    order: Vec<String>,
    root: String,
    rules: HashMap<String, AnchorRules>,
    aligned: HashSet<String>,
    anchored: HashSet<String>,
    scrollers: HashMap<String, Scroller>,
    /// A form's main menu is inside its window (Windows, Linux, the web);
    /// false for macOS's system menu bar.
    menu_in_window: bool,
    updating: bool,
}

/// The properties layout reads (the rest don't move anything).
fn geometry(key: &str) -> bool {
    matches!(key, "left" | "top" | "width" | "height" | "align" | "anchors" | "visible" | "clientwidth" | "clientheight" | "borderstyle") || lay::CONSTRAINT_PROPERTIES.contains(&key)
}

impl Layout {
    /// `design` laid out as the program creates it.
    pub fn of(design: &FormDesign) -> Layout {
        Layout::with_menu(design, true)
    }

    /// The same, saying where a form's main menu is.
    pub fn with_menu(design: &FormDesign, menu_in_window: bool) -> Layout {
        let mut l = Layout {
            comps: HashMap::new(),
            order: Vec::new(),
            root: key_of(design, design.root()),
            rules: HashMap::new(),
            aligned: HashSet::new(),
            anchored: HashSet::new(),
            scrollers: HashMap::new(),
            menu_in_window,
            updating: false,
        };
        l.create(design, design.root());
        l
    }

    fn create(&mut self, design: &FormDesign, id: NodeId) {
        let Some(node) = design.node(id) else { return };
        let key = key_of(design, id);
        let ty = node.canonical.clone();
        // (as the runtimes' registries give a new component)
        let (w, h) = lay::default_size(&ty).unwrap_or((0, 0));
        let comp = Comp {
            id,
            ty: ty.clone(),
            parent: String::new(),
            rect: Rect::new(0, 0, w, h),
            align: lay::default_align(&ty),
            visible: true,
            anchors: None,
            constraints: Constraints::default(),
            border_style: None,
            props: crate::component_defaults::shared(&ty).into_iter().collect(),
        };
        self.comps.insert(key.clone(), comp);
        self.order.push(key.clone());
        if let Some(p) = node.parent {
            let parent = key_of(design, p);
            let before = self.labels_before(&key, "parent");
            if let Some(c) = self.comps.get_mut(&key) {
                c.parent = parent;
            }
            engine::after_set(self, &key, "parent");
            self.labels_after(&key, before);
        }
        for item in &node.body {
            match item {
                Item::Prop(p) => self.assign(&key, &prop_key(&p.name), &p.value),
                Item::Child(c) => self.create(design, *c),
                Item::Code(_) => {}
            }
        }
    }

    /// One assignment of a CREATE block, as `rp_comp_set` runs it: stored
    /// (geometry through [`Layout::set`]), then the AutoSize labels it
    /// changed take their text's size (`crate::autosize`, as the runtimes).
    /// A value the designer can't read is skipped.
    fn assign(&mut self, key: &str, prop: &str, text: &str) {
        let value = match super::value::read(text) {
            super::value::PropValue::Number(f) if f.fract() == 0.0 => Value::Integer(f as i64),
            super::value::PropValue::Number(f) => Value::Double(f),
            super::value::PropValue::Str(s) => Value::String(s),
            _ => return,
        };
        let flat = crate::autosize::font_property(prop).map_or_else(|| prop.to_string(), str::to_string);
        let before = self.labels_before(key, &flat);
        if geometry(&flat) {
            self.set(key, &flat, crate::layout::property_value(&flat, value).to_i64());
        } else if let Some(c) = self.comps.get_mut(key) {
            c.props.insert(flat, value);
        }
        self.labels_after(key, before);
    }

    /// A stored property, as the runtimes read one (Null when unset).
    fn stored(&self, key: &str, prop: &str) -> Value {
        let Some(c) = self.comps.get(&key.to_lowercase()) else { return Value::Null };
        match prop {
            "left" => Value::Integer(c.rect.left),
            "top" => Value::Integer(c.rect.top),
            "width" => Value::Integer(c.rect.width),
            "height" => Value::Integer(c.rect.height),
            "parent" => Value::String(c.parent.clone()),
            "align" => Value::Integer(c.align.value()),
            _ => c.props.get(prop).cloned().unwrap_or(Value::Null),
        }
    }

    fn font_key(&self, key: &str) -> crate::autosize::FontKey {
        (crate::objects::font_from_props(key, &|i, p| self.stored(i, p)), self.stored(key, "fontcolor").to_i64())
    }

    fn labels_before(&self, key: &str, prop: &str) -> Option<crate::autosize::Before> {
        let ty = self.comps.get(key)?.ty.clone();
        let children = |p: &str| -> Vec<(String, String)> { self.children_of(&p.to_lowercase()).into_iter().map(|k| (k.clone(), self.comps.get(&k).map(|c| c.ty.clone()).unwrap_or_default())).collect() };
        crate::autosize::before_set(key, &ty, prop, &|i, p| self.stored(i, p), &children, &|l| self.font_key(l))
    }

    fn labels_after(&mut self, key: &str, before: Option<crate::autosize::Before>) {
        let Some(before) = before else { return };
        for label in crate::autosize::changed_labels(key, before, &|i, p| self.stored(i, p), &|l| self.font_key(l)) {
            let Some(r) = crate::autosize::label_bounds(&label, &|i, p| self.stored(i, p)) else { continue };
            let now = self.stored(&label, "left").to_i64();
            let cur = self.comps.get(&label).map(|c| c.rect).unwrap_or_default();
            for (p, v, was) in [("left", r.left, now), ("width", r.width, cur.width), ("height", r.height, cur.height)] {
                if v != was {
                    self.set(&label, p, v);
                }
            }
        }
    }

    /// Sets a geometry property as the runtimes' `rp_comp_set` does, then
    /// lays out as they do ([`engine::after_set`]).
    fn set(&mut self, key: &str, prop: &str, v: i64) {
        let Some(c) = self.comps.get(key) else { return };
        let is_form = c.ty == "RFORM";
        match prop {
            "clientwidth" | "clientheight" => {
                if is_form {
                    let (cw, ch) = self.form_client(key);
                    let (cw, ch) = if prop == "clientwidth" { (v, ch) } else { (cw, v) };
                    let (w, h) = lay::form_outer_size(cw, ch, self.border_style(key), self.menu(key));
                    self.set(key, if prop == "clientwidth" { "width" } else { "height" }, if prop == "clientwidth" { w } else { h });
                } else {
                    self.set(key, if prop == "clientwidth" { "width" } else { "height" }, v);
                }
            }
            p if lay::CONSTRAINT_PROPERTIES.contains(&p) => {
                let Some(k) = c.constraints.with(p, v) else { return };
                let rect = c.rect;
                if let Some(c) = self.comps.get_mut(key) {
                    c.constraints = k;
                }
                for (p, size) in [("width", rect.width), ("height", rect.height)] {
                    let bounded = if p == "width" { k.width(size) } else { k.height(size) };
                    if bounded != size {
                        self.set(key, p, bounded);
                    }
                }
            }
            _ => {
                let k = c.constraints;
                if let Some(c) = self.comps.get_mut(key) {
                    match prop {
                        "left" => c.rect.left = v,
                        "top" => c.rect.top = v,
                        "width" => c.rect.width = k.width(v),
                        "height" => c.rect.height = k.height(v),
                        "align" => c.align = Align::from_value(v),
                        "anchors" => c.anchors = Some(v),
                        "visible" => c.visible = v != 0,
                        "borderstyle" => c.border_style = Some(v),
                        _ => {}
                    }
                }
                if prop == "borderstyle" {
                    if is_form {
                        engine::client_changed(self, key);
                    } else {
                        self.scroll_update(key);
                        engine::reanchor(self, key);
                    }
                } else {
                    engine::after_set(self, key, prop);
                }
            }
        }
    }

    /// The form resized to `width` × `height` (its Width / Height, frame
    /// included), as a user dragging its corner does (bounded by its
    /// Constraints): its aligned and anchored components follow.
    pub fn resize(&mut self, width: i64, height: i64) {
        let root = self.root.clone();
        let Some(c) = self.comps.get(&root) else { return };
        let (w, h) = c.constraints.size(width, height);
        if (c.rect.width, c.rect.height) == (w, h) {
            return;
        }
        if let Some(c) = self.comps.get_mut(&root) {
            c.rect.width = w;
            c.rect.height = h;
        }
        engine::client_changed(self, &root);
        self.scroll_update(&root);
    }

    /// A component's rectangle (Left / Top / Width / Height).
    pub fn rect(&self, id: NodeId) -> Option<Rect> {
        self.comps.values().find(|c| c.id == id).map(|c| c.rect)
    }

    /// Every component's rectangle, in creation order.
    pub fn rects(&self) -> Vec<(NodeId, Rect)> {
        self.order.iter().filter_map(|k| self.comps.get(k)).map(|c| (c.id, c.rect)).collect()
    }

    /// The form's Width × Height.
    pub fn form_size(&self) -> (i64, i64) {
        self.comps.get(&self.root).map_or((0, 0), |c| (c.rect.width, c.rect.height))
    }

    /// The form's inside (ClientWidth × ClientHeight: frame, menu and the
    /// scroll bars shown excluded).
    pub fn form_client_size(&self) -> (i64, i64) {
        self.form_client(&self.root.clone())
    }

    /// A container's client rectangle in its children's coordinates.
    pub fn client_of(&self, id: NodeId) -> Option<Rect> {
        let key = self.comps.iter().find(|(_, c)| c.id == id).map(|(k, _)| k.clone())?;
        Some(LayoutStore::client_rect(self, &key))
    }

    /// The scroll bars a form or scroll box shows (horizontal, vertical).
    pub fn scroll_bars(&self, id: NodeId) -> (bool, bool) {
        self.comps.iter().find(|(_, c)| c.id == id).and_then(|(k, _)| self.scrollers.get(k)).map_or((false, false), |s| (s.horz.shown, s.vert.shown))
    }

    /// Where a component's client origin is in the form's client area
    /// (its parents' Left / Top added up).
    pub fn origin(&self, design: &FormDesign, id: NodeId) -> (i64, i64) {
        let mut at = (0, 0);
        let mut p = design.parent(id);
        while let Some(pid) = p {
            if pid == design.root() {
                break;
            }
            if let Some(r) = self.rect(pid) {
                at.0 += r.left;
                at.1 += r.top;
            }
            p = design.parent(pid);
        }
        at
    }

    fn border_style(&self, key: &str) -> i64 {
        self.comps.get(key).and_then(|c| c.border_style).unwrap_or(2)
    }

    fn menu(&self, key: &str) -> i64 {
        let has = self.order.iter().filter_map(|k| self.comps.get(k)).any(|c| c.parent == key && c.ty == "RMAINMENU");
        if has && self.menu_in_window { lay::MAIN_MENU_HEIGHT } else { 0 }
    }

    /// A form's inside (frame and menu excluded, its scroll bars not).
    fn form_area(&self, key: &str) -> (i64, i64) {
        let r = self.comps.get(key).map(|c| c.rect).unwrap_or_default();
        lay::form_client_size(r.width, r.height, self.border_style(key), self.menu(key))
    }

    /// A scrolling container's area (as runtime-core's `scroll::area`).
    fn area(&self, key: &str) -> (i64, i64) {
        let Some(c) = self.comps.get(key) else { return (0, 0) };
        if c.ty == "RFORM" {
            return self.form_area(key);
        }
        let b = if c.ty == "RSCROLLBOX" && c.border_style.unwrap_or(2) != 0 { 2 } else { 0 };
        ((c.rect.width - 2 * b).max(0), (c.rect.height - 2 * b).max(0))
    }

    /// The area less the bars shown.
    fn form_client(&self, key: &str) -> (i64, i64) {
        let (w, h) = self.area(key);
        self.scrollers.get(key).map_or((w, h), |s| s.client(w, h))
    }

    fn scrolls(&self, key: &str) -> bool {
        self.comps.get(key).is_some_and(|c| scrollbars::scrolls(&c.ty))
    }
}

fn key_of(design: &FormDesign, id: NodeId) -> String {
    design.node(id).map_or_else(String::new, |n| n.name.to_lowercase())
}

impl LayoutStore for Layout {
    fn rect(&self, name: &str) -> Rect {
        self.comps.get(name).map(|c| c.rect).unwrap_or_default()
    }
    fn align(&self, name: &str) -> Align {
        self.comps.get(name).map_or(Align::None, |c| c.align)
    }
    fn visible(&self, name: &str) -> bool {
        self.comps.get(name).is_none_or(|c| c.visible)
    }
    fn anchors(&self, name: &str) -> i64 {
        self.comps.get(name).and_then(|c| c.anchors).unwrap_or(DEFAULT_ANCHORS)
    }
    fn constraints(&self, name: &str) -> Constraints {
        self.comps.get(name).map(|c| c.constraints).unwrap_or_default()
    }
    fn type_of(&self, name: &str) -> String {
        self.comps.get(name).map(|c| c.ty.clone()).unwrap_or_default()
    }
    fn parent_of(&self, name: &str) -> String {
        self.comps.get(name).map(|c| c.parent.clone()).unwrap_or_default()
    }
    fn key(&self, name: &str) -> String {
        name.to_lowercase()
    }
    fn children_of(&self, parent: &str) -> Vec<String> {
        self.order.iter().filter(|k| self.comps.get(*k).is_some_and(|c| c.parent == parent)).cloned().collect()
    }
    fn client_rect(&self, parent: &str) -> Rect {
        // (as the runtimes: a form's / scroll box's is the scrolled area,
        // as large as the ranges)
        if self.scrolls(parent) {
            let (cw, ch) = self.form_client(parent);
            let (hp, vp, hr, vr) = self.scrollers.get(parent).map_or((0, 0, 0, 0), |s| (s.horz.position, s.vert.position, s.horz.range, s.vert.range));
            return Rect::new(-hp, -vp, cw.max(hr), ch.max(vr));
        }
        let r = LayoutStore::rect(self, parent);
        Rect::new(0, 0, r.width, r.height)
    }
    fn anchor_area(&self, parent: &str) -> (i64, i64) {
        if self.scrolls(parent) {
            return self.area(parent);
        }
        let r = self.client_rect(parent);
        (r.width, r.height)
    }
    fn store_rect(&mut self, name: &str, r: Rect) {
        if let Some(c) = self.comps.get_mut(name) {
            c.rect = r;
        }
    }
    fn rules(&self, name: &str) -> Option<AnchorRules> {
        self.rules.get(name).copied()
    }
    fn set_rules(&mut self, name: &str, rules: Option<AnchorRules>) {
        match rules {
            Some(r) => {
                self.rules.insert(name.to_string(), r);
            }
            None => {
                self.rules.remove(name);
            }
        }
    }
    fn has_aligned(&self, parent: &str) -> bool {
        self.aligned.contains(parent)
    }
    fn mark_aligned(&mut self, parent: &str) {
        self.aligned.insert(parent.to_string());
    }
    fn has_anchored(&self, parent: &str) -> bool {
        self.anchored.contains(parent)
    }
    fn mark_anchored(&mut self, parent: &str) {
        self.anchored.insert(parent.to_string());
    }
    /// The bars worked out again, as the runtimes' `scroll::update`: the
    /// components moved when a position must change, laid out again when a
    /// bar came or went.
    fn scroll_update(&mut self, name: &str) {
        if !self.scrolls(name) || self.updating {
            return;
        }
        self.updating = true;
        for _ in 0..2 {
            let (w, h) = self.area(name);
            let kids: Vec<(String, scrollbars::Child)> = self
                .children_of(name)
                .into_iter()
                .filter_map(|k| {
                    let c = self.comps.get(&k)?;
                    (!matches!(c.ty.as_str(), "RMAINMENU" | "RPOPUPMENU" | "RMENUITEM") && lay::default_size(&c.ty).is_some()).then(|| {
                        let child = scrollbars::Child { left: c.rect.left, top: c.rect.top, width: c.rect.width, height: c.rect.height, align: c.align, visible: c.visible };
                        (k.clone(), child)
                    })
                })
                .collect();
            let s = self.scrollers.entry(name.to_string()).or_default();
            let before = (s.horz.shown, s.vert.shown);
            let children: Vec<scrollbars::Child> = kids.iter().map(|(_, c)| *c).collect();
            let (dx, dy) = s.update(w, h, &children);
            let after = (s.horz.shown, s.vert.shown);
            if (dx, dy) != (0, 0) {
                for (k, c) in &kids {
                    if let Some(comp) = self.comps.get_mut(k) {
                        comp.rect.left = c.left - dx;
                        comp.rect.top = c.top - dy;
                    }
                }
            }
            if before == after {
                break;
            }
            engine::realign(self, name, None);
        }
        self.updating = false;
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{Prop, SubItem, Subtree};
    use super::*;

    fn p(n: &str, v: &str) -> SubItem {
        SubItem::Prop(Prop { name: n.into(), value: v.into() })
    }

    fn comp(name: &str, ty: &str, props: &[(&str, &str)]) -> Subtree {
        Subtree { id: 0, name: name.into(), type_written: ty.into(), body: props.iter().map(|(n, v)| p(n, v)).collect() }
    }

    /// tests/fixtures/anchors.bas's form (as far as its CREATE goes).
    fn anchors_form() -> FormDesign {
        let mut inner = comp("Inner", "QBUTTON", &[("Caption", "\"in\""), ("Left", "200"), ("Top", "10"), ("Width", "60"), ("Height", "25"), ("Anchors", "akTop + akRight")]);
        inner.id = 0;
        let mut pn = comp("Pn", "QPANEL", &[("Left", "10"), ("Top", "50"), ("Width", "300"), ("Height", "100"), ("Anchors", "akLeft OR akTop OR akRight"), ("Constraints.MinWidth", "250")]);
        pn.body.push(SubItem::Child(inner));
        let mut form = comp("Form", "QFORM", &[("Caption", "\"Anchors\""), ("Width", "400"), ("Height", "300"), ("Constraints.MinWidth", "300"), ("MinHeight", "200")]);
        form.body.push(SubItem::Child(comp("Ok", "QBUTTON", &[("Left", "300"), ("Top", "230"), ("Width", "75"), ("Height", "25"), ("Anchors", "akRight + akBottom")])));
        form.body.push(SubItem::Child(comp("Ed", "QEDIT", &[("Left", "10"), ("Top", "10"), ("Width", "200"), ("Anchors", "akLeft + akTop + akRight")])));
        form.body.push(SubItem::Child(pn));
        form.body.push(SubItem::Child(comp("Mid", "QLABEL", &[("Left", "150"), ("Top", "180"), ("Width", "100"), ("Height", "20"), ("Anchors", "akTop")])));
        FormDesign::from_subtree(form)
    }

    #[test]
    fn the_resize_preview_follows_anchors_as_the_runtime() {
        let d = anchors_form();
        let mut l = Layout::of(&d);
        let r = |l: &Layout, n: &str| l.rect(d.find(n).unwrap()).unwrap();
        assert_eq!(r(&l, "Ok"), Rect::new(300, 230, 75, 25));
        assert_eq!(l.form_client_size(), (398, 269));
        // tests/gui_parity_cases.mjs "anchors": the user makes it 250x180
        // (bounded to 300x200): B's line "200,130|100|250|150|100|300x200".
        l.resize(250, 180);
        assert_eq!(l.form_size(), (300, 200));
        assert_eq!((r(&l, "Ok").left, r(&l, "Ok").top), (200, 130));
        assert_eq!(r(&l, "Ed").width, 100);
        assert_eq!(r(&l, "Pn").width, 250);
        assert_eq!(r(&l, "Inner").left, 150);
        assert_eq!(r(&l, "Mid").left, 100);
        // then 500x400: "400,330|300|400|300|200|500x400"
        l.resize(500, 400);
        assert_eq!((r(&l, "Ok").left, r(&l, "Ok").top, r(&l, "Ed").width, r(&l, "Pn").width, r(&l, "Inner").left, r(&l, "Mid").left), (400, 330, 300, 400, 300, 200));
    }

    #[test]
    fn aligned_children_and_scroll_bars_as_the_runtime() {
        // scratch probe align1.bas, as RapidR lays it out (changed first)
        let mut form = comp("Form", "QFORM", &[("Width", "400"), ("Height", "300")]);
        for (n, t, props) in [
            ("Sp", "QSPLITTER", vec![("Align", "alLeft")]),
            ("Tree", "QPANEL", vec![("Width", "200"), ("Align", "alLeft")]),
            ("Bar", "QPANEL", vec![("Height", "30"), ("Align", "alTop")]),
            ("Foot", "QPANEL", vec![("Height", "20"), ("Align", "alBottom")]),
            ("Ed", "QPANEL", vec![("Align", "alClient")]),
            ("Ok", "QBUTTON", vec![("Left", "300"), ("Top", "230")]),
        ] {
            form.body.push(SubItem::Child(comp(n, t, &props)));
        }
        let d = FormDesign::from_subtree(form);
        let mut l = Layout::of(&d);
        let r = |l: &Layout, n: &str| l.rect(d.find(n).unwrap()).unwrap();
        assert_eq!(r(&l, "Sp"), Rect::new(200, 30, 3, 225));
        assert_eq!(r(&l, "Tree"), Rect::new(0, 30, 200, 225));
        assert_eq!(r(&l, "Bar"), Rect::new(0, 0, 381, 30));
        assert_eq!(r(&l, "Ed"), Rect::new(203, 30, 178, 225));
        assert_eq!(l.form_client_size(), (381, 269));
        assert_eq!(l.scroll_bars(d.root()), (false, true), "the button reaches 255 + 20 below…");
        l.resize(500, 400);
        assert_eq!(r(&l, "Ed"), Rect::new(203, 30, 295, 319));
        assert_eq!(r(&l, "Foot"), Rect::new(0, 349, 498, 20));
    }
}
