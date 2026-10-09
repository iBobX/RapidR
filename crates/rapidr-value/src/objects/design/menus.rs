//! The menu editor (docs/studio-wow.md DES-13): the designed form's main
//! menu edited where it shows — on its own menu bar, as VB6's and Delphi's
//! menu designers and Visual Studio's "Type Here" do it, but in the form's
//! real look.
//!
//! - A click on a bar item selects it (the inspector shows its Caption,
//!   ShortCut, Checked, Enabled, OnClick …) and drops its menu down; a
//!   click on an item of an open menu selects it and opens its submenu.
//! - The last slot of the bar and of every open menu is **Type Here**: a
//!   click (or typing on it) edits a new item's caption in place; Enter adds
//!   it — a `CREATE … AS QMENUITEM` block in the right place, named as
//!   Delphi names them (`&Open…` → `Open1`, a separator `N1`) — and goes on
//!   to the next slot. A selected item without items shows a Type Here to
//!   its right: its submenu.
//! - `-` as a caption is a separator; `&` marks the mnemonic; Tab in the
//!   editor goes to the ShortCut field, which takes the keys pressed
//!   (Ctrl+O). F2, a slow click or typing on a selected item edits it; a
//!   click in a selected item's check-mark gutter toggles Checked; a
//!   double click makes or finds its OnClick handler; Delete removes it.
//! - Dragging an item moves it (a line shows where it goes; over a bar
//!   item, its menu opens to drop into): the CREATE block moves.
//! - The arrows walk the menus: Up / Down in a menu, Left / Right along the
//!   bar (Right opens a submenu, Left closes it).
//!
//! Every change is one command of the designer — the smallest text edit,
//! one undo step — like the rest of the designer's.

use super::inline::Target;
use super::{Command, DesignEvent, DesignSurface, Drag, Grip, NodeId};
use crate::designer::model::Item;
use crate::layout::MAIN_MENU_HEIGHT;
use crate::objects::font::Font;
use crate::objects::menu::{bar_item_width, panel_size, BORDER, GUTTER, ITEM_H, SEP_H};
use crate::objects::ops::{Op, Place, Rect};

/// What the placeholders say.
pub const TYPE_HERE: &str = "Type Here";

/// A place in the menu editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// A menu item.
    Item(NodeId),
    /// "Type Here": a new item under `parent`, before its child `before`
    /// (`None`: last).
    New { parent: NodeId, before: Option<NodeId> },
}

/// A slot and its rectangle (client coordinates: the bar is above 0).
#[derive(Clone, Debug, PartialEq)]
pub struct SlotRect {
    pub slot: Slot,
    pub rect: Rect,
    pub separator: bool,
}

/// An open menu: the items under `parent` (and its Type Here).
#[derive(Clone, Debug, PartialEq)]
pub struct MenuPanel {
    pub parent: NodeId,
    pub rect: Rect,
    pub rows: Vec<SlotRect>,
    /// Only a Type Here (a selected item's submenu to be).
    pub placeholder: bool,
}

/// The designed form's menu as the editor shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuView {
    pub main: NodeId,
    pub bar: Vec<SlotRect>,
    pub panels: Vec<MenuPanel>,
}

/// Where a press on the menus landed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hit {
    Slot(Slot, bool),
    /// An open menu's frame, not a row.
    Panel,
    /// The bar where there is no item.
    Bar,
}

fn inside((x, y, w, h): Rect, px: i64, py: i64) -> bool {
    px >= x && py >= y && px < x + w && py < y + h
}

impl DesignSurface {
    /// The designed form's main menu (its first QMAINMENU).
    pub fn main_menu(&self) -> Option<NodeId> {
        let d = &self.designer.design;
        d.children(d.root()).into_iter().find(|&c| d.node(c).is_some_and(|n| n.canonical == "RMAINMENU"))
    }

    fn is_item(&self, id: NodeId) -> bool {
        self.designer.design.node(id).is_some_and(|n| n.canonical == "RMENUITEM")
    }

    fn items_of(&self, id: NodeId) -> Vec<NodeId> {
        self.designer.design.children(id).into_iter().filter(|&c| self.is_item(c)).collect()
    }

    /// An item's caption as written (its text).
    pub fn menu_caption(&self, id: NodeId) -> String {
        self.designer.design.node(id).and_then(|n| n.prop("Caption")).map(super::plain_value).unwrap_or_default()
    }

    /// An item's ShortCut as a menu shows it.
    fn menu_keys(&self, id: NodeId) -> String {
        let n = self.designer.design.node(id);
        let sc = n.and_then(|n| n.prop("ShortCut")).map(super::plain_value).unwrap_or_default();
        match crate::objects::menu::parse_shortcut(&sc) {
            Some(s) => s.text(),
            None => crate::objects::menu::split_caption(&self.menu_caption(id)).1.to_string(),
        }
    }

    /// Whether the menu editor shows its Type Here slots: the main menu or
    /// an item selected, a menu open, an item being typed, or a bar with
    /// nothing on it yet.
    pub fn menu_active(&self) -> bool {
        let Some(main) = self.main_menu() else { return false };
        let sel = self.designer.selection.primary();
        sel == Some(main) || sel.is_some_and(|s| self.is_item(s)) || !self.menu_open.is_empty() || self.editing.as_ref().is_some_and(|e| e.menu) || self.items_of(main).is_empty()
    }

    /// The menu bar and the open menus, where the editor shows them.
    pub fn menu_view(&self) -> Option<MenuView> {
        let main = self.main_menu()?;
        if self.menu_height() == 0 {
            return None;
        }
        let active = self.menu_active();
        let mh = MAIN_MENU_HEIGHT;
        let mut x = 0;
        let mut bar = Vec::new();
        for id in self.items_of(main) {
            let caption = self.menu_caption(id);
            if caption == "-" {
                continue;
            }
            let w = bar_item_width(&caption);
            bar.push(SlotRect { slot: Slot::Item(id), rect: (x, -mh, w, mh), separator: false });
            x += w;
        }
        if active {
            bar.push(SlotRect { slot: Slot::New { parent: main, before: None }, rect: (x, -mh, bar_item_width(TYPE_HERE), mh), separator: false });
        }
        let mut panels: Vec<MenuPanel> = Vec::new();
        for &open in &self.menu_open {
            // (where it opens: under its bar item, or right of its row)
            let at = match panels.last() {
                None => bar.iter().find(|s| s.slot == Slot::Item(open)).map(|s| (s.rect.0, 0)),
                Some(p) => p.rows.iter().find(|r| r.slot == Slot::Item(open)).map(|r| (p.rect.0 + p.rect.2 - BORDER, r.rect.1 - BORDER)),
            };
            let Some((px, py)) = at else { break };
            panels.push(self.panel(open, px, py, active, false));
        }
        // (a selected item without items: its submenu's Type Here)
        if active {
            if let (Some(sel), Some(last)) = (self.designer.selection.primary(), panels.last()) {
                let row = last.rows.iter().find(|r| r.slot == Slot::Item(sel) && !r.separator).map(|r| r.rect);
                if let Some((_, ry, _, _)) = row.filter(|_| self.items_of(sel).is_empty() && !self.menu_open.contains(&sel)) {
                    panels.push(self.panel(sel, last.rect.0 + last.rect.2 - BORDER, ry - BORDER, true, true));
                }
            }
        }
        Some(MenuView { main, bar, panels })
    }

    /// An open menu's panel at (x, y): its items' rows and its Type Here.
    fn panel(&self, parent: NodeId, x: i64, y: i64, active: bool, placeholder: bool) -> MenuPanel {
        let items = if placeholder { Vec::new() } else { self.items_of(parent) };
        let mut rows: Vec<(String, String, bool, Slot)> = items.iter().map(|&id| {
            let c = self.menu_caption(id);
            let sep = c == "-";
            (c, self.menu_keys(id), sep, Slot::Item(id))
        }).collect();
        if active {
            rows.push((TYPE_HERE.to_string(), String::new(), false, Slot::New { parent, before: None }));
        }
        let sizes: Vec<(&str, &str, bool)> = rows.iter().map(|r| (r.0.as_str(), r.1.as_str(), r.2)).collect();
        let (w, h) = panel_size(&sizes);
        let mut ry = y + BORDER;
        let rows = rows
            .into_iter()
            .map(|(_, _, sep, slot)| {
                let rh = if sep { SEP_H } else { ITEM_H };
                let r = SlotRect { slot, rect: (x + BORDER, ry, w - 2 * BORDER, rh), separator: sep };
                ry += rh;
                r
            })
            .collect();
        MenuPanel { parent, rect: (x, y, w, h), rows, placeholder }
    }

    /// Where a slot is (client coordinates).
    pub fn menu_slot_rect(&self, slot: Slot) -> Option<Rect> {
        let v = self.menu_view()?;
        v.bar.iter().chain(v.panels.iter().flat_map(|p| p.rows.iter())).find(|s| s.slot == slot).map(|s| s.rect)
    }

    fn menu_hit(&self, x: i64, y: i64) -> Option<Hit> {
        let v = self.menu_view()?;
        for p in v.panels.iter().rev() {
            if inside(p.rect, x, y) {
                return Some(p.rows.iter().find(|r| inside(r.rect, x, y)).map_or(Hit::Panel, |r| Hit::Slot(r.slot, x < r.rect.0 + GUTTER)));
            }
        }
        if let Some(s) = v.bar.iter().find(|s| inside(s.rect, x, y)) {
            return Some(Hit::Slot(s.slot, false));
        }
        let (cw, _) = self.client_size();
        (y < 0 && y >= -MAIN_MENU_HEIGHT && x >= 0 && x < cw).then_some(Hit::Bar)
    }

    /// The menus open down to item `id` (its own too when it has items or
    /// is a bar item).
    pub(super) fn open_path_to(&mut self, id: NodeId) {
        let d = &self.designer.design;
        let main = self.main_menu();
        let mut path = Vec::new();
        let mut at = Some(id);
        while let Some(n) = at.filter(|&n| Some(n) != main && self.is_item(n)) {
            path.push(n);
            at = d.parent(n);
        }
        path.reverse();
        // (an item without items opens nothing, but a bar item does: its Type Here)
        if let Some(&last) = path.last() {
            if path.len() > 1 && self.items_of(last).is_empty() {
                path.pop();
            }
        }
        self.menu_open = path;
    }

    /// A press on the menus (client coordinates): whether it was theirs
    /// (and what the program hears).
    pub(super) fn menu_press(&mut self, x: i64, y: i64, double: bool, add: bool) -> Option<Option<DesignEvent>> {
        let Some(hit) = self.menu_hit(x, y) else {
            if !self.menu_open.is_empty() {
                self.menu_open.clear();
            }
            return None;
        };
        self.outside_sel = None;
        match hit {
            Hit::Panel => Some(None),
            Hit::Bar => {
                let main = self.main_menu()?;
                self.menu_open.clear();
                self.designer.selection.set(main);
                self.selection_changed();
                Some(self.take_select_event())
            }
            Hit::Slot(Slot::New { parent, before }, _) => {
                self.begin_new_item(parent, before, "");
                Some(None)
            }
            Hit::Slot(Slot::Item(id), gutter) => {
                let i = self.index_of(id)?;
                let was_alone = self.designer.selection.ids() == [id];
                if double {
                    self.slow = None;
                    return Some(Some(DesignEvent::DblClick(i)));
                }
                if gutter && was_alone && self.menu_caption(id) != "-" && self.designer.design.parent(id) != self.main_menu() {
                    self.toggle_checked(id);
                    return Some(None);
                }
                if add {
                    self.designer.selection.toggle(id);
                } else {
                    self.designer.selection.set(id);
                }
                self.open_path_to(id);
                self.slow = was_alone.then_some(id);
                self.drag = Some(Drag { grip: Grip::MenuItem, from: (x, y), start: Vec::new(), now: Vec::new(), offset: (0, 0), moved: false, keep: false, last: (x, y) });
                self.menu_drop = None;
                self.selection_changed();
                Some(self.take_select_event())
            }
        }
    }

    /// The OnSelect `selection_changed` queued, given back to be returned
    /// as the press's event (the kernel fires it).
    fn take_select_event(&mut self) -> Option<DesignEvent> {
        match self.outbox.last() {
            Some(DesignEvent::Select(_)) => self.outbox.pop(),
            _ => None,
        }
    }

    /// Checked on or off (one undo step; off removes the line).
    fn toggle_checked(&mut self, id: NodeId) {
        let on = self.designer.design.node(id).and_then(|n| n.int("Checked")).unwrap_or(0) != 0;
        let value = (!on).then(|| "1".to_string());
        let _ = self.designer.execute(Command::SetProp { node: id, name: "Checked".into(), value });
        self.commit();
        let caption = crate::objects::a11y::mnemonic(&self.menu_caption(id)).0;
        self.say(format!("{caption}: {}", if on { "not checked" } else { "checked" }));
    }

    /// The mouse dragged while a menu item is held: where it would go.
    pub(super) fn menu_drag(&mut self, drag: &mut Drag, x: i64, y: i64) {
        drag.last = (x, y);
        if !drag.moved && (x - drag.from.0).abs() + (y - drag.from.1).abs() < 4 {
            return;
        }
        drag.moved = true;
        self.slow = None;
        let Some(moving) = self.designer.selection.primary().filter(|&m| self.is_item(m)) else { return };
        // (over a bar item: its menu drops down to drop into)
        if let Some(Hit::Slot(Slot::Item(t), _)) = self.menu_hit(x, y) {
            if self.designer.design.parent(t) == self.main_menu() && self.menu_open.first() != Some(&t) && t != moving {
                self.menu_open = vec![t];
            }
        }
        self.menu_drop = self.drop_place(moving, x, y);
    }

    /// Where an item dropped at (x, y) goes: under which menu, before which
    /// item, and the insertion line (client coordinates).
    fn drop_place(&self, moving: NodeId, x: i64, y: i64) -> Option<(NodeId, Option<NodeId>, Rect)> {
        let v = self.menu_view()?;
        let d = &self.designer.design;
        let ok = |parent: NodeId| !d.is_within(parent, moving);
        for p in v.panels.iter().rev().filter(|p| !p.placeholder) {
            if !inside(p.rect, x, y) {
                continue;
            }
            for (k, r) in p.rows.iter().enumerate() {
                if !inside((p.rect.0, r.rect.1, p.rect.2, r.rect.3), x, y) {
                    continue;
                }
                let lower = y >= r.rect.1 + r.rect.3 / 2;
                let before = match (r.slot, lower) {
                    (Slot::Item(t), false) => Some(t),
                    (Slot::Item(_), true) => p.rows.get(k + 1).and_then(|n| if let Slot::Item(t) = n.slot { Some(t) } else { None }),
                    (Slot::New { .. }, _) => None,
                };
                let ly = if lower && matches!(r.slot, Slot::Item(_)) { r.rect.1 + r.rect.3 } else { r.rect.1 };
                return ok(p.parent).then_some((p.parent, before, (r.rect.0, ly - 1, r.rect.2, 2)));
            }
        }
        for (k, s) in v.bar.iter().enumerate() {
            if !inside(s.rect, x, y) {
                continue;
            }
            let right = x >= s.rect.0 + s.rect.2 / 2;
            let before = match (s.slot, right) {
                (Slot::Item(t), false) => Some(t),
                (Slot::Item(_), true) => v.bar.get(k + 1).and_then(|n| if let Slot::Item(t) = n.slot { Some(t) } else { None }),
                (Slot::New { .. }, _) => None,
            };
            let lx = if right && matches!(s.slot, Slot::Item(_)) { s.rect.0 + s.rect.2 } else { s.rect.0 };
            return ok(v.main).then_some((v.main, before, (lx - 1, s.rect.1 + 2, 2, s.rect.3 - 4)));
        }
        None
    }

    /// The mouse let go after a menu item was held: moved where it was
    /// dropped (one undo step), or — a slow click — its caption edited.
    pub(super) fn menu_release(&mut self, drag: Drag) {
        let target = self.menu_drop.take();
        let Some(moving) = self.designer.selection.primary() else { return };
        if !drag.moved {
            if self.slow.take() == Some(moving) {
                self.begin_edit();
            }
            return;
        }
        let Some((parent, before, _)) = target else { return };
        if before == Some(moving) {
            return;
        }
        let Some(p) = self.designer.design.node(parent) else { return };
        let body: Vec<&Item> = p.body.iter().filter(|i| **i != Item::Child(moving)).collect();
        let index = match before {
            Some(b) => body.iter().position(|i| **i == Item::Child(b)).unwrap_or(body.len()),
            None => body.len(),
        };
        // (dropped where it is: nothing to do)
        if self.designer.design.parent(moving) == Some(parent) && p.body.iter().position(|i| *i == Item::Child(moving)) == Some(index) {
            return;
        }
        if self.designer.execute(Command::Move { node: moving, parent, index }).is_ok() {
            self.commit();
            self.open_path_to(moving);
            let caption = crate::objects::a11y::mnemonic(&self.menu_caption(moving)).0;
            self.say(format!("Moved {caption}"));
        }
    }

    /// A key on a selected menu item (no editor): the arrows walk the
    /// menus, F2 or a letter edits it. Whether it was the menus'.
    pub(super) fn menu_key(&mut self, vk: i64, text: &str, ctrl: bool) -> bool {
        let Some(sel) = self.designer.selection.primary() else { return false };
        let main = self.main_menu();
        let is_main = Some(sel) == main;
        if !self.is_item(sel) && !is_main {
            return false;
        }
        let d = &self.designer.design;
        let parent = d.parent(sel);
        let on_bar = parent == main && !is_main;
        let siblings = |s: &Self, of: Option<NodeId>| of.map(|p| s.items_of(p)).unwrap_or_default();
        let pick = |s: &mut Self, id: NodeId| {
            s.designer.selection.set(id);
            s.open_path_to(id);
            s.selection_changed();
        };
        let step = |list: &[NodeId], by: i64| -> Option<NodeId> {
            let k = list.iter().position(|&c| c == sel)? as i64;
            let n = list.len() as i64;
            list.get(((k + by).rem_euclid(n)) as usize).copied()
        };
        match vk {
            113 => return self.begin_edit(),
            37 | 39 if on_bar => {
                if let Some(t) = step(&siblings(self, parent), if vk == 37 { -1 } else { 1 }) {
                    pick(self, t);
                }
            }
            40 if on_bar => {
                if let Some(&f) = self.items_of(sel).first() {
                    pick(self, f);
                }
            }
            38 | 40 if !is_main => {
                if let Some(t) = step(&siblings(self, parent), if vk == 38 { -1 } else { 1 }) {
                    pick(self, t);
                }
            }
            39 if !is_main => {
                if let Some(&f) = self.items_of(sel).first() {
                    pick(self, f);
                }
            }
            37 if !is_main => {
                if let Some(p) = parent.filter(|&p| self.is_item(p)) {
                    pick(self, p);
                }
            }
            _ if !ctrl && !is_main && !text.is_empty() && text.chars().all(|c| !c.is_control()) && vk != 13 && vk != 9 => {
                // (typing on an item edits its caption, the letters typed first)
                if self.begin_edit() {
                    if let Some(e) = &mut self.editing {
                        e.text.clear();
                        e.caret = 0;
                        e.anchor = 0;
                    }
                    return self.edit_key(vk, text, false, false, false);
                }
                return false;
            }
            _ => return false,
        }
        true
    }

    /// The menu editor's marks (surface pixels from the client area's
    /// origin): Type Here slots, the selected items framed, where a dragged
    /// item would go.
    pub(super) fn menu_ops(&self, out: &mut Vec<Op>) {
        let Some(v) = self.menu_view() else { return };
        let t = crate::theme::current();
        let font = Font::default();
        let vr = |r: Rect| self.view_rect(r);
        for s in v.bar.iter().chain(v.panels.iter().flat_map(|p| p.rows.iter())) {
            let r = vr(s.rect);
            match s.slot {
                Slot::New { .. } if !self.editing_slot(s.slot) => {
                    let (x, y, w, h) = r;
                    dashed(out, (x + 2, y + 2, w - 4, h - 4), t.accent);
                    out.push(Op::Text { rect: (x, y, w, h), text: TYPE_HERE.into(), font: font.clone(), color: t.gray_text, angle: 0, place: Place::Center });
                }
                Slot::Item(id) if self.designer.selection.contains(id) && !self.editing_slot(s.slot) => {
                    let (x, y, w, h) = r;
                    out.push(Op::Edge { rect: (x, y, w, h), light: vec![t.accent, t.accent], dark: vec![t.accent, t.accent] });
                }
                _ => {}
            }
        }
        if let (Some((_, _, line)), Some(g)) = (self.menu_drop, self.drag.as_ref().filter(|g| g.grip == Grip::MenuItem && g.moved)) {
            let _ = g;
            out.push(Op::Fill { rect: vr(line), color: t.accent });
        }
    }

    /// The menu editor on: the form's main menu selected (one added first
    /// when it has none), its bar's Type Here showing. Whether it could.
    pub fn edit_menu(&mut self) -> bool {
        if self.no_form() || self.read_only() {
            return false;
        }
        if self.main_menu().is_none() && self.add_at("QMAINMENU", (0, 0), None).is_none() {
            return false;
        }
        let Some(main) = self.main_menu() else { return false };
        self.typing = None;
        self.outside_sel = None;
        self.designer.selection.set(main);
        self.menu_open.clear();
        self.selection_changed();
        // (the bar's Type Here ready: the caption typed at once)
        self.begin_new_item(main, None, "");
        true
    }

    /// The editing target's menu, if the editor is on one.
    pub fn editing_menu(&self) -> Option<NodeId> {
        match &self.editing.as_ref()?.target {
            Target::NewItem { parent, .. } => Some(*parent),
            Target::Prop { node, .. } => self.designer.design.parent(*node),
        }
    }
}

/// A dashed one-pixel outline inside `rect`.
fn dashed(out: &mut Vec<Op>, (x, y, w, h): Rect, color: u32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let (r, b) = (x + w - 1, y + h - 1);
    let mut k = x;
    while k <= r {
        let e = (k + 2).min(r);
        out.push(Op::Fill { rect: (k, y, e - k + 1, 1), color });
        out.push(Op::Fill { rect: (k, b, e - k + 1, 1), color });
        k += 5;
    }
    let mut k = y;
    while k <= b {
        let e = (k + 2).min(b);
        out.push(Op::Fill { rect: (x, k, 1, e - k + 1), color });
        out.push(Op::Fill { rect: (r, k, 1, e - k + 1), color });
        k += 5;
    }
}
