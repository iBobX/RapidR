//! QMAINMENU as the kernel draws it — the in-window menu bar (everywhere
//! but macOS' system menu bar, and there with `RAPIDR_MENU=window`):
//! [`layout::MAIN_MENU_HEIGHT`] at the top of the window, its items'
//! captions (`&` underlined), Windows-classic: raised under the mouse,
//! sunken while open. A click (or Alt + an item's `&` letter) drops its
//! menu down; menus are panels drawn over the form ([`MenuUi::panels`]) —
//! check marks, radio dots, ShortCuts on the right, separators, submenus
//! opening to the right, disabled items greyed. The mouse, the arrow keys,
//! Enter, Escape and the items' `&` letters work them; a picked item is
//! [`KernelEvent::MenuPick`] (its OnClick, after the pump). A main menu's
//! ShortCut picks its item before the key reaches anything (Windows'
//! accelerators). QPOPUPMENU's panels are the same (`popupmenu.rs`).
//!
//! A fluent theme draws the panels rounded on the theme's menu colour,
//! the item under the mouse a soft rounded highlight, vector check marks
//! and chevrons; the bar's open and hot items the same highlight.
//!
//! Everything is read from the shared model (`rapidr_value::objects::menu`)
//! when drawn or touched: the kernel keeps only which menus are open.
//!
//! [`layout::MAIN_MENU_HEIGHT`]: rapidr_value::layout::MAIN_MENU_HEIGHT

use rapidr_value::objects::a11y::{mnemonic, node_id, part_id, AccessNode, Action, Orientation, Role, PART_MENU, PART_POPUP};
use rapidr_value::objects::font::Font;
use rapidr_value::objects::menu::{self, Kind};
use rapidr_value::objects::ops::{Op, Place, Rect};
use rapidr_value::objects::text::text_size;

use super::check::check_mark;
use super::radio::disc;
use crate::input::{KernelEvent, Mods};
use crate::paint::{caption, Painter};
use crate::store::Store;
use crate::tree::FormUi;

/// A menu's metrics: the shared model's (the form designer's menu editor
/// measures menus with the same numbers).
pub use rapidr_value::objects::menu::{BORDER, GUTTER, ITEM_H, SEP_H};
use rapidr_value::objects::menu::RIGHT;

/// An open menu: the items under `parent` (a menu or an item) in a panel.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    /// The menu or item whose items it shows.
    pub parent: String,
    /// Where it is in the window's inside (logical; the bar's included).
    pub rect: Rect,
    /// The item under the mouse or chosen with the keys (an index into
    /// [`items`]).
    pub hot: Option<usize>,
}

/// Which of a form's menus are open, and the bar's state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuUi {
    /// The main menu is the system's (macOS' menu bar, built by the host):
    /// the kernel neither draws it nor takes its ShortCuts.
    pub system_bar: bool,
    /// The bar's item under the mouse (no menu open).
    pub hot_top: Option<usize>,
    /// The bar's item whose menu is down.
    pub open_top: Option<usize>,
    /// A bar item without items, pressed (picked when let go on it).
    pub pressed_top: Option<usize>,
    /// The open panels, outermost first.
    pub panels: Vec<Panel>,
    /// The QPOPUPMENU the panels are (`None`: the bar's).
    pub popup: Option<String>,
    /// The press that opened or closed a menu: its release is the menu's
    /// too.
    pub swallow_up: bool,
    /// (the input lane's) The bar selected from the keyboard — F10, or Alt
    /// pressed and released alone (Windows' menu mode): `hot_top` is its
    /// chosen item; Left / Right move it, Down / Up / Enter open it, a
    /// letter picks by mnemonic, Escape, Alt or F10 leave.
    pub keyboard: bool,
    /// (the input lane's) Alt went down and nothing else since: its release
    /// selects the bar.
    pub alt_alone: bool,
}

/// One item as a panel shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemView {
    pub id: String,
    pub caption: String,
    pub separator: bool,
    pub checked: bool,
    pub radio: bool,
    pub enabled: bool,
    pub submenu: bool,
    /// Its ShortCut as Windows shows it ("Ctrl+N").
    pub keys: String,
    /// Its Hint (a screen reader reads it as the item's description).
    pub hint: String,
}

/// The visible items under `parent`, as drawn.
pub fn items(parent: &str) -> Vec<ItemView> {
    menu::children(parent)
        .into_iter()
        .filter_map(|id| {
            let submenu = !menu::children(&id).is_empty();
            menu::with(&id, |n| {
                // (a tab in the caption: the key text after it, as Windows')
                let (text, shown) = menu::split_caption(&n.caption);
                ItemView {
                    id: id.clone(),
                    caption: text.to_string(),
                    separator: n.caption == "-",
                    checked: n.checked,
                    radio: n.radio,
                    enabled: n.enabled,
                    submenu,
                    keys: if submenu {
                        String::new()
                    } else {
                        menu::parse_shortcut(&n.shortcut).map(|s| s.text()).unwrap_or_else(|| shown.to_string())
                    },
                    hint: n.hint.clone(),
                }
            })
        })
        .collect()
}

/// Menus' font (the system's menu font: RapidQ's menus have no Font).
pub fn menu_font() -> Font {
    menu::menu_font()
}

/// A panel's size for its items.
pub fn panel_size(items: &[ItemView]) -> (i64, i64) {
    let rows: Vec<(&str, &str, bool)> = items.iter().map(|i| (i.caption.as_str(), i.keys.as_str(), i.separator)).collect();
    menu::panel_size(&rows)
}

/// Each item's row in a panel at (0, 0): its top and height.
pub fn rows(items: &[ItemView]) -> Vec<(i64, i64)> {
    let mut y = BORDER;
    items
        .iter()
        .map(|i| {
            let h = if i.separator { SEP_H } else { ITEM_H };
            let r = (y, h);
            y += h;
            r
        })
        .collect()
}

fn inside((x, y, w, h): Rect, px: f64, py: f64) -> bool {
    px >= x as f64 && py >= y as f64 && px < (x + w) as f64 && py < (y + h) as f64
}

/// Draws a panel of `items` at `rect` (`hot`: the highlighted one).
pub fn paint_panel(p: &mut Painter, rect: Rect, items: &[ItemView], hot: Option<usize>) {
    let (x0, y0, w, h) = rect;
    let font = menu_font();
    let t = p.theme();
    p.at((x0, y0), |p| {
        if t.fluent() {
            p.elevate((0, 0, w, h), t.panel_radius(), 10.0);
            p.round((0, 0, w, h), t.panel_radius(), Some(t.menu), Some(t.border), 1.0);
        } else {
            p.fill((0, 0, w, h), t.menu);
            // (EDGE_RAISED: COLOR_3DLIGHT and white, dark grey and grey)
            p.raised_edge((0, 0, w, h));
        }
        let keys_right = w - BORDER - RIGHT;
        for (k, ((y, rh), item)) in rows(items).into_iter().zip(items).enumerate() {
            if item.separator {
                let ly = y + rh / 2 - 1;
                if t.fluent() {
                    p.fill((BORDER + 1, ly + 1, w - 2 * BORDER - 2, 1), t.border);
                } else {
                    p.fill((BORDER + 1, ly, w - 2 * BORDER - 2, 1), t.shadow);
                    p.fill((BORDER + 1, ly + 1, w - 2 * BORDER - 2, 1), t.light);
                }
                continue;
            }
            let lit = hot == Some(k);
            if lit {
                if t.fluent() {
                    p.round((BORDER + 1, y + 1, w - 2 * BORDER - 2, rh - 2), (t.radius - 1.0).max(2.0), Some(t.menu_highlight), None, 1.0);
                } else {
                    p.fill((BORDER, y, w - 2 * BORDER, rh), t.menu_highlight);
                }
            }
            let color = match (item.enabled, lit) {
                (true, true) => t.menu_highlight_text,
                (true, false) => t.menu_text,
                (false, _) => t.gray_text,
            };
            // (a disabled item not lit is embossed: white under the grey —
            // classic)
            let emboss = [(1, t.light), (0, t.gray_text)];
            let draws: &[(i64, u32)] = if !item.enabled && !lit && !t.fluent() { &emboss } else { &[(0, color)] };
            for &(d, c) in draws {
                let text_x = BORDER + GUTTER + d;
                if item.checked {
                    let (gx, gy) = (BORDER + d, y + d);
                    if t.fluent() {
                        let (mx, my) = ((gx + GUTTER / 2) as f64, (gy + rh / 2) as f64);
                        if item.radio {
                            p.round(((mx - 3.0) as i64, (my - 3.0) as i64, 6, 6), 3.0, Some(c), None, 1.0);
                        } else {
                            p.check_glyph(mx - 6.5, my - 6.5, 13.0, c);
                        }
                    } else if item.radio {
                        p.shape(disc((gx + GUTTER / 2) as f64 - 0.5, (gy + rh / 2) as f64 - 0.5, 2.5, 0.0, 360.0, c));
                    } else {
                        check_mark(p, gx + (GUTTER - 7) / 2, gy + (rh - 7) / 2, c);
                    }
                }
                caption(p, (text_x, y + d, keys_right - text_x, rh), &item.caption, &font, c, Place::Left);
                if !item.keys.is_empty() {
                    let kw = text_size(&item.keys, &font).0;
                    p.text((keys_right - kw + d, y + d, kw, rh), &item.keys, &font, c, Place::Left);
                }
                if item.submenu {
                    let (ax, ay) = ((w - BORDER - 10 + d) as f64, (y + d + rh / 2) as f64);
                    if t.fluent() {
                        p.chevron(ax + 2.0, ay, 8.0, false, c);
                    } else {
                        p.op(Op::Arrow { points: [(ax, ay - 4.0), (ax + 4.0, ay), (ax, ay + 4.0)], color: c });
                    }
                }
            }
        }
    });
}

impl FormUi {
    /// The form's QMAINMENU (its first), lowercase.
    pub fn main_menu(&self, store: &dyn Store) -> Option<String> {
        store.children(&self.form).into_iter().find(|(_, t)| t.eq_ignore_ascii_case("RMAINMENU")).map(|(id, _)| id.to_lowercase())
    }

    /// The bar's items (separators left out) and their rectangles.
    pub fn bar_items(&self, store: &dyn Store) -> Vec<(ItemView, Rect)> {
        if self.menu_offset <= 0 {
            return Vec::new();
        }
        let Some(main) = self.main_menu(store) else { return Vec::new() };
        let mut x = 0;
        items(&main)
            .into_iter()
            .filter(|i| !i.separator)
            .map(|i| {
                let w = menu::bar_item_width(&i.caption);
                let r = (x, 0, w, self.menu_offset);
                x += w;
                (i, r)
            })
            .collect()
    }

    fn bar_item_at(&self, store: &dyn Store, x: f64, y: f64) -> Option<usize> {
        if self.menus.popup.is_some() || y >= self.menu_offset as f64 {
            return None;
        }
        self.bar_items(store).iter().position(|(_, r)| inside(*r, x, y))
    }

    /// A menu is open (the bar's or a pop-up).
    pub fn menu_open(&self) -> bool {
        !self.menus.panels.is_empty()
    }

    /// Closes every open menu.
    pub fn close_menus(&mut self) {
        let m = &mut self.menus;
        if !m.panels.is_empty() || m.open_top.is_some() || m.pressed_top.is_some() {
            self.dirty = true;
        }
        m.panels.clear();
        m.open_top = None;
        m.pressed_top = None;
        m.popup = None;
        // (the input lane's: and the bar's keyboard selection)
        if std::mem::take(&mut m.keyboard) {
            m.hot_top = None;
            self.dirty = true;
        }
    }

    /// (the input lane's) F10 or a lone Alt: the in-window bar's first item
    /// selected from the keyboard (not with the system's menu bar, nor
    /// while a pop-up menu is open). Whether it was.
    pub(crate) fn select_bar(&mut self, store: &dyn Store) -> bool {
        if self.menus.system_bar || self.menu_open() || self.bar_items(store).is_empty() {
            return false;
        }
        self.menus.keyboard = true;
        self.menus.hot_top = Some(0);
        self.dirty = true;
        true
    }

    /// (the input lane's) A key while the bar is selected from the keyboard
    /// (every key is the menus' then, as in Windows' menu mode).
    fn bar_key(&mut self, store: &dyn Store, vk: i64) {
        let bar = self.bar_items(store);
        let at = self.menus.hot_top.unwrap_or(0).min(bar.len().saturating_sub(1));
        let n = bar.len();
        if n == 0 {
            self.close_menus();
            return;
        }
        // (an item chosen: its menu drops with its first item lit, or one
        // without items is picked)
        let choose = |f: &mut FormUi, i: usize, by_enter: bool| {
            f.menus.keyboard = false;
            if bar[i].0.submenu {
                f.open_top(store, i, true);
            } else if by_enter && bar[i].0.enabled {
                let id = bar[i].0.id.clone();
                f.pick(&id);
            } else {
                f.menus.keyboard = true;
            }
        };
        match vk {
            37 => self.menus.hot_top = Some((at + n - 1) % n),
            39 => self.menus.hot_top = Some((at + 1) % n),
            38 | 40 => choose(self, at, false),
            13 => choose(self, at, true),
            27 | 18 | 121 => self.close_menus(),
            65..=90 | 48..=57 => {
                let letter = (vk as u8 as char).to_ascii_lowercase();
                if let Some(i) = bar.iter().position(|(item, _)| mnemonic(&item.caption).1.map(|m| m.1) == Some(letter)) {
                    choose(self, i, true);
                }
            }
            _ => {}
        }
    }

    /// The window's inside, the bar included (where panels must fit).
    fn menu_area(&self) -> (i64, i64) {
        (self.client.0, self.client.1 + self.menu_offset)
    }

    /// A panel for `parent`'s items with its top left at (x, y), kept in
    /// the window (`flip_x`: where it goes when it doesn't fit to the
    /// right, a submenu's other side).
    pub(crate) fn place_panel(&self, parent: &str, x: i64, y: i64, flip_x: Option<i64>) -> Option<Panel> {
        let list = items(parent);
        if list.is_empty() {
            return None;
        }
        let (w, h) = panel_size(&list);
        let (aw, ah) = self.menu_area();
        let mut px = x;
        if px + w > aw {
            px = flip_x.map_or(aw - w, |f| f - w);
        }
        let py = if y + h > ah { (ah - h).max(0) } else { y };
        Some(Panel { parent: parent.to_string(), rect: (px.max(0), py.max(0), w, h), hot: None })
    }

    /// Bar item `i`'s menu down (`hot_first`: its first item chosen, as
    /// with the keys).
    fn open_top(&mut self, store: &dyn Store, i: usize, hot_first: bool) {
        let bar = self.bar_items(store);
        let Some((item, r)) = bar.get(i).cloned() else { return };
        self.menus.panels.clear();
        self.menus.popup = None;
        self.menus.open_top = Some(i);
        self.menus.hot_top = None;
        self.dirty = true;
        if !item.enabled {
            return;
        }
        if let Some(mut panel) = self.place_panel(&item.id, r.0, r.1 + r.3, None) {
            if hot_first {
                panel.hot = first_item(&items(&item.id));
            }
            self.menus.panels.push(panel);
        }
    }

    /// Panel `pi`'s item `k`'s submenu open to its right.
    fn open_sub(&mut self, pi: usize, k: usize, hot_first: bool) {
        let Some(panel) = self.menus.panels.get(pi).cloned() else { return };
        let list = items(&panel.parent);
        let Some(item) = list.get(k) else { return };
        self.menus.panels.truncate(pi + 1);
        self.menus.panels[pi].hot = Some(k);
        self.dirty = true;
        if !item.submenu || !item.enabled {
            return;
        }
        let (x, y, w, _) = panel.rect;
        let top = y + rows(&list)[k].0 - BORDER;
        if let Some(mut sub) = self.place_panel(&item.id, x + w - BORDER, top, Some(x + BORDER)) {
            if hot_first {
                sub.hot = first_item(&items(&item.id));
            }
            self.menus.panels.push(sub);
        }
    }

    /// The panel (and its item) at a point, innermost first.
    fn panel_at(&self, x: f64, y: f64) -> Option<(usize, Option<usize>)> {
        let pi = (0..self.menus.panels.len()).rev().find(|&pi| inside(self.menus.panels[pi].rect, x, y))?;
        let panel = &self.menus.panels[pi];
        let list = items(&panel.parent);
        let ry = y - panel.rect.1 as f64;
        let k = rows(&list).iter().position(|(top, h)| ry >= *top as f64 && ry < (top + h) as f64);
        Some((pi, k))
    }

    /// An item picked: the menus close, its OnClick after the pump.
    fn pick(&mut self, id: &str) {
        self.close_menus();
        self.menus.hot_top = None;
        // (the kernel's own edit menu: the text lane's, done on the edit)
        if super::edit::is_menu_item(id) {
            super::edit::queue_pick(id);
            return;
        }
        self.events.push(KernelEvent::MenuPick(id.to_string()));
    }

    /// A press while a menu is open, or on the bar: whether it was the
    /// menus'.
    pub(crate) fn menu_mouse_down(&mut self, store: &dyn Store, x: f64, y: f64) -> bool {
        // (the input lane's: a press leaves the bar's keyboard selection)
        if self.menus.keyboard && !self.menu_open() {
            self.close_menus();
        }
        if self.menu_open() {
            self.menus.swallow_up = true;
            self.dirty = true;
            if let Some((pi, k)) = self.panel_at(x, y) {
                if let Some(k) = k {
                    let list = items(&self.menus.panels[pi].parent);
                    if !list[k].separator {
                        self.open_sub(pi, k, false);
                    }
                }
                return true;
            }
            match self.bar_item_at(store, x, y) {
                Some(i) if self.menus.open_top == Some(i) => self.close_menus(),
                Some(i) => self.open_top(store, i, false),
                // (a press elsewhere closes the menus, and does nothing else)
                None => self.close_menus(),
            }
            return true;
        }
        if y < self.menu_offset as f64 {
            self.menus.swallow_up = true;
            if let Some(i) = self.bar_item_at(store, x, y) {
                let bar = self.bar_items(store);
                if bar[i].0.submenu {
                    self.open_top(store, i, false);
                } else if bar[i].0.enabled {
                    self.menus.pressed_top = Some(i);
                    self.dirty = true;
                }
            }
            return true;
        }
        self.menus.swallow_up = false;
        false
    }

    /// The mouse moved: whether the menus took it (one is open).
    pub(crate) fn menu_mouse_move(&mut self, store: &dyn Store, x: f64, y: f64) -> bool {
        if !self.menu_open() {
            let hot = if y < self.menu_offset as f64 { self.bar_item_at(store, x, y) } else { None };
            // (the bar selected from the keyboard keeps its item off the bar)
            if self.menus.keyboard && hot.is_none() {
                return self.menus.pressed_top.is_some();
            }
            if hot != self.menus.hot_top {
                self.menus.hot_top = hot;
                self.dirty = true;
            }
            return self.menus.pressed_top.is_some();
        }
        if let Some((pi, k)) = self.panel_at(x, y) {
            let list = items(&self.menus.panels[pi].parent);
            if let Some(k) = k.filter(|&k| !list[k].separator) {
                if self.menus.panels.get(pi + 1).is_some_and(|s| s.parent == list[k].id) {
                    // (its submenu is open already: deeper ones close)
                    self.menus.panels.truncate(pi + 2);
                    self.menus.panels[pi + 1].hot = None;
                    self.menus.panels[pi].hot = Some(k);
                } else {
                    self.open_sub(pi, k, false);
                }
            }
            self.dirty = true;
        } else if let Some(i) = self.bar_item_at(store, x, y) {
            if self.menus.open_top != Some(i) {
                self.open_top(store, i, false);
            }
        } else if let Some(last) = self.menus.panels.last_mut() {
            // (off the menus: the innermost panel's item isn't lit)
            if last.hot.is_some() {
                last.hot = None;
                self.dirty = true;
            }
        }
        true
    }

    /// A release: whether it was the menus' (an item let go on is picked).
    pub(crate) fn menu_mouse_up(&mut self, store: &dyn Store, x: f64, y: f64) -> bool {
        if self.menu_open() {
            self.menus.swallow_up = false;
            if let Some((pi, Some(k))) = self.panel_at(x, y) {
                let item = items(&self.menus.panels[pi].parent).swap_remove(k);
                if !item.separator && !item.submenu && item.enabled {
                    self.pick(&item.id);
                }
            }
            return true;
        }
        if let Some(i) = self.menus.pressed_top.take() {
            self.menus.swallow_up = false;
            self.dirty = true;
            if self.bar_item_at(store, x, y) == Some(i) {
                let id = self.bar_items(store)[i].0.id.clone();
                self.pick(&id);
            }
            return true;
        }
        std::mem::take(&mut self.menus.swallow_up)
    }

    /// A key: whether the menus took it — every key while one is open; a
    /// main menu's ShortCut (its item picked, as Windows' accelerators:
    /// nothing else hears the key).
    pub(crate) fn menu_key(&mut self, store: &dyn Store, vk: i64, mods: Mods) -> bool {
        if self.menu_open() {
            self.dirty = true;
            self.menu_open_key(store, vk);
            return true;
        }
        // (the input lane's: the bar selected from the keyboard)
        if self.menus.keyboard {
            self.dirty = true;
            self.bar_key(store, vk);
            return true;
        }
        if self.menus.system_bar || (16..=18).contains(&vk) {
            return false;
        }
        let Some(main) = self.main_menu(store) else { return false };
        // ("Ctrl+" is the platform's command key too: Cmd on a Mac)
        let item = menu::item_for_shortcut(&main, vk, mods.ctrl, mods.shift, mods.alt)
            .or_else(|| (mods.command && !mods.ctrl).then(|| menu::item_for_shortcut(&main, vk, true, mods.shift, mods.alt)).flatten());
        match item {
            Some(item) => {
                self.events.push(KernelEvent::MenuPick(item));
                true
            }
            None => false,
        }
    }

    fn menu_open_key(&mut self, store: &dyn Store, vk: i64) {
        let last = self.menus.panels.len() - 1;
        let list = items(&self.menus.panels[last].parent);
        let hot = self.menus.panels[last].hot;
        let bar = self.menus.popup.is_none() && self.menus.open_top.is_some();
        let tops = self.bar_items(store).len();
        let step = |from: Option<usize>, back: bool| -> Option<usize> {
            let n = list.len();
            let start = from.unwrap_or(if back { 0 } else { n - 1 });
            (1..=n).map(|d| if back { (start + n * 2 - d) % n } else { (start + d) % n }).find(|&k| !list[k].separator)
        };
        match vk {
            27 => {
                if last > 0 {
                    self.menus.panels.pop();
                } else {
                    // (the input lane's: the bar's menu closes, its item stays
                    // selected — a second Escape leaves, as Windows')
                    let top = self.menus.open_top.filter(|_| bar);
                    self.close_menus();
                    if top.is_some() && !self.menus.system_bar {
                        self.menus.keyboard = true;
                        self.menus.hot_top = top;
                    }
                }
            }
            38 | 40 => self.menus.panels[last].hot = step(hot, vk == 38),
            39 => match hot {
                Some(k) if list[k].submenu && list[k].enabled => self.open_sub(last, k, true),
                _ if bar && tops > 0 => self.open_top(store, (self.menus.open_top.unwrap_or(0) + 1) % tops, true),
                _ => {}
            },
            37 => {
                if last > 0 {
                    self.menus.panels.pop();
                } else if bar && tops > 0 {
                    self.open_top(store, (self.menus.open_top.unwrap_or(0) + tops - 1) % tops, true);
                }
            }
            13 | 32 => {
                if let Some(k) = hot {
                    self.choose(last, k, &list);
                }
            }
            18 => self.close_menus(),
            65..=90 | 48..=57 => {
                let letter = (vk as u8 as char).to_ascii_lowercase();
                if let Some(k) = list.iter().position(|i| !i.separator && mnemonic(&i.caption).1.map(|m| m.1) == Some(letter)) {
                    self.choose(last, k, &list);
                }
            }
            _ => {}
        }
    }

    /// Item `k` of panel `pi` chosen with the keys: its submenu opens, or
    /// it's picked.
    fn choose(&mut self, pi: usize, k: usize, list: &[ItemView]) {
        let item = &list[k];
        if item.submenu {
            self.open_sub(pi, k, true);
        } else if item.enabled {
            let id = item.id.clone();
            self.pick(&id);
        } else {
            self.menus.panels[pi].hot = Some(k);
        }
    }

    /// Alt + `letter`: the bar's item it marks drops its menu down.
    pub(crate) fn menu_mnemonic(&mut self, store: &dyn Store, letter: char) -> bool {
        let bar = self.bar_items(store);
        let Some(i) = bar.iter().position(|(item, _)| mnemonic(&item.caption).1.map(|m| m.1) == Some(letter)) else { return false };
        if bar[i].0.submenu {
            self.open_top(store, i, true);
        } else if bar[i].0.enabled {
            let id = bar[i].0.id.clone();
            self.pick(&id);
        }
        true
    }

    /// The in-window bar (its strip of the client area's top).
    pub(crate) fn paint_menu_bar(&mut self, store: &dyn Store, p: &mut Painter) {
        let (w, h) = (self.client.0, self.menu_offset);
        let t = p.theme();
        p.fill((0, 0, w, h), t.face);
        let font = menu_font();
        let m = &self.menus;
        for (i, (item, r)) in self.bar_items(store).into_iter().enumerate() {
            let open = m.open_top == Some(i) && m.popup.is_none() || m.pressed_top == Some(i);
            let hot = m.hot_top == Some(i) && m.panels.is_empty();
            let frame = (r.0, r.1 + 2, r.2, r.3 - 4);
            if t.fluent() {
                if open || hot {
                    p.round(frame, (t.radius - 1.0).max(2.0), Some(if open { t.menu_highlight } else { t.control_hot }), None, 1.0);
                }
            } else if open {
                p.thin_sunken(frame);
            } else if hot {
                p.thin_raised(frame);
            }
            let d = i64::from(open && !t.fluent());
            // (RapidR's look underlines the bar's access keys only while the
            // keyboard is in the menus, as Windows 11 does)
            let shown_caption = if t.fluent() && !self.menus.keyboard && !self.menu_open() { mnemonic(&item.caption).0.replace('&', "&&") } else { item.caption.clone() };
            let item_caption = &shown_caption;
            if item.enabled {
                let ink = if t.fluent() && open { t.menu_highlight_text } else { t.menu_text };
                caption(p, (r.0 + d, r.1 + d, r.2, r.3), item_caption, &font, ink, Place::Center);
            } else {
                if !t.fluent() {
                    caption(p, (r.0 + 1, r.1 + 1, r.2, r.3), item_caption, &font, t.light, Place::Center);
                }
                caption(p, r, item_caption, &font, t.gray_text, Place::Center);
            }
        }
    }

    /// The open menus, over the form.
    pub(crate) fn paint_menus(&mut self, _store: &dyn Store, p: &mut Painter) {
        for panel in &self.menus.panels {
            let list = items(&panel.parent);
            paint_panel(p, panel.rect, &list, panel.hot);
        }
    }

    /// The bar for screen readers (a MenuBar of MenuItems), when drawn: the
    /// item whose menu is down has that menu inside it.
    pub fn describe_menu_bar(&self, store: &dyn Store) -> Option<AccessNode> {
        let main = self.main_menu(store)?;
        let bar = self.bar_items(store);
        if bar.is_empty() {
            return None;
        }
        let mut n = AccessNode::new(node_id(&main), Role::MenuBar);
        n.bounds = (0, 0, self.client.0, self.menu_offset);
        n.orientation = Some(Orientation::Horizontal);
        // (the shared items, each where it's drawn, a submenu open or not)
        n.children = rapidr_value::objects::a11y::menu_bar(&main);
        for (k, (c, (item, r))) in n.children.iter_mut().zip(bar).enumerate() {
            c.bounds = r;
            c.accelerator = Some(item.keys.clone()).filter(|k| !k.is_empty());
            c.description = item.hint.clone();
            // (the keyboard on the bar: this item is where a screen reader's
            // cursor is)
            c.states.focused = self.menus.keyboard && !self.menu_open() && self.menus.hot_top == Some(k);
            if c.states.expanded.is_some() {
                let open = self.menus.open_top == Some(k) && self.menu_open() && self.menus.popup.is_none();
                c.states.expanded = Some(open);
                c.actions.push(if open { Action::Collapse } else { Action::Expand });
                if open && self.menus.panels.first().is_some_and(|p| p.parent == item.id) {
                    c.children.push(self.describe_panel(0));
                }
            }
        }
        Some(n)
    }

    /// A pop-up menu (QPOPUPMENU, an edit's context menu) open now, for
    /// screen readers: its panel, items and open submenus inside.
    pub fn describe_popup_menu(&self) -> Option<AccessNode> {
        self.menus.popup.as_ref().filter(|_| self.menu_open())?;
        Some(self.describe_panel(0))
    }

    /// Whether the menus hold the keyboard: a menu is open, or the bar is
    /// selected from the keyboard (F10, Alt). A screen reader's focus is on
    /// one of their nodes then, and on nothing else.
    pub fn menus_have_focus(&self) -> bool {
        self.menu_open() || self.menus.keyboard
    }

    /// Open panel `pi` as a Menu node (named by the item it drops from),
    /// its items inside — a check mark, a radio dot, a disabled item as
    /// states, the ShortCut as the accelerator — and the panel's open
    /// submenu inside the item it belongs to. The item the keys or the mouse
    /// chose has the focus (the innermost panel's; the panel itself while no
    /// item is chosen).
    fn describe_panel(&self, pi: usize) -> AccessNode {
        let panel = &self.menus.panels[pi];
        let last = pi + 1 == self.menus.panels.len();
        let list = items(&panel.parent);
        let mut menu_node = AccessNode::new(part_id(&panel.parent, PART_POPUP, 0), Role::Menu);
        menu_node.name = menu::with(&panel.parent, |n| mnemonic(menu::split_caption(&n.caption).0).0).filter(|c| !c.is_empty()).unwrap_or_else(|| rapidr_value::i18n::tr_ui("Menu"));
        menu_node.bounds = panel.rect;
        menu_node.orientation = Some(Orientation::Vertical);
        menu_node.states.focused = last && panel.hot.is_none();
        for (k, ((top, h), item)) in rows(&list).into_iter().zip(&list).enumerate() {
            // (a separator is a line, not something to read: left out, as the bar's)
            if item.separator {
                continue;
            }
            let role = if item.radio {
                Role::MenuItemRadio
            } else if item.checked {
                Role::MenuItemCheckBox
            } else {
                Role::MenuItem
            };
            let mut n = AccessNode::new(node_id(&item.id), role);
            n.name = mnemonic(&item.caption).0;
            n.description = item.hint.clone();
            n.accelerator = Some(item.keys.clone()).filter(|k| !k.is_empty());
            n.states.disabled = !item.enabled;
            if item.radio || item.checked {
                n.states.checked = Some(item.checked);
            }
            n.states.focused = last && panel.hot == Some(k);
            n.bounds = (panel.rect.0 + BORDER, panel.rect.1 + top, panel.rect.2 - 2 * BORDER, h);
            n.actions = vec![Action::Click, Action::Focus];
            if item.submenu {
                let open = self.menus.panels.get(pi + 1).is_some_and(|s| s.parent == item.id);
                n.states.expanded = Some(open);
                n.actions.push(if open { Action::Collapse } else { Action::Expand });
                if open {
                    n.children.push(self.describe_panel(pi + 1));
                }
            }
            menu_node.children.push(n);
        }
        menu_node
    }

    /// A screen reader's request on a menu node (`target` its id): a bar
    /// item or an item of an open panel — Click picks it (a submenu's item
    /// opens or closes its menu), Expand and Collapse open and close a
    /// submenu, Focus chooses it as the arrow keys would. `None` when the
    /// target isn't a menu's.
    pub(crate) fn menu_access(&mut self, store: &dyn Store, target: u64, action: Action) -> Option<bool> {
        for pi in 0..self.menus.panels.len() {
            let list = items(&self.menus.panels[pi].parent);
            if let Some(k) = list.iter().position(|i| !i.separator && node_id(&i.id) == target) {
                return Some(self.panel_item_access(pi, k, &list[k], action));
            }
        }
        let main = self.main_menu(store)?;
        let bar = self.bar_items(store);
        let k = (0..bar.len()).find(|&k| part_id(&main, PART_MENU, k) == target)?;
        let open = self.menus.open_top == Some(k) && self.menu_open() && self.menus.popup.is_none();
        let item = &bar[k].0;
        match action {
            Action::Click if item.enabled => {
                if item.submenu {
                    if open {
                        self.close_menus();
                    } else {
                        self.open_top(store, k, true);
                    }
                } else {
                    let id = item.id.clone();
                    self.pick(&id);
                }
                Some(true)
            }
            Action::Expand if item.enabled && item.submenu => {
                if !open {
                    self.open_top(store, k, true);
                }
                Some(true)
            }
            Action::Collapse if item.submenu => {
                if open {
                    self.close_menus();
                }
                Some(true)
            }
            Action::Focus if !self.menu_open() => {
                self.menus.keyboard = true;
                self.menus.hot_top = Some(k);
                self.dirty = true;
                Some(true)
            }
            _ => Some(false),
        }
    }

    /// `action` on panel `pi`'s item `k` (see [`FormUi::menu_access`]).
    fn panel_item_access(&mut self, pi: usize, k: usize, item: &ItemView, action: Action) -> bool {
        let open = self.menus.panels.get(pi + 1).is_some_and(|s| s.parent == item.id);
        match action {
            Action::Click if item.enabled => {
                if item.submenu {
                    if open {
                        self.menus.panels.truncate(pi + 1);
                        self.menus.panels[pi].hot = Some(k);
                        self.dirty = true;
                    } else {
                        self.open_sub(pi, k, true);
                    }
                } else {
                    self.pick(&item.id);
                }
                true
            }
            Action::Expand if item.enabled && item.submenu => {
                if !open {
                    self.open_sub(pi, k, true);
                }
                true
            }
            Action::Collapse if item.submenu => {
                if open {
                    self.menus.panels.truncate(pi + 1);
                    self.menus.panels[pi].hot = Some(k);
                    self.dirty = true;
                }
                true
            }
            Action::Focus => {
                // (as the mouse's move onto it: deeper menus close)
                self.menus.panels.truncate(pi + 1);
                self.menus.panels[pi].hot = Some(k);
                self.dirty = true;
                true
            }
            _ => false,
        }
    }

    /// Whether `id` is a menu of this form's (its main menu, or a pop-up
    /// menu on it).
    pub fn has_menu(&self, store: &dyn Store, id: &str) -> bool {
        menu::root_of(id).is_some_and(|root| {
            store.children(&self.form).iter().any(|(c, _)| c.eq_ignore_ascii_case(&root)) && menu::kind(&root) != Some(Kind::Item)
        })
    }
}

/// The first item that isn't a separator.
fn first_item(list: &[ItemView]) -> Option<usize> {
    list.iter().position(|i| !i.separator)
}

#[cfg(test)]
mod tests {
    use rapidr_value::input::Button;
    use rapidr_value::objects::a11y::{node_id, AccessNode, Action, Orientation, Role};
    use rapidr_value::{v_int, v_str};

    use crate::{FormUi, KernelEvent, MemClipboard, MemStore, Mods, TextSystem};

    const CTRL: Mods = Mods { shift: false, ctrl: true, alt: false, command: true, word: true };
    const ALT: Mods = Mods { shift: false, ctrl: false, alt: true, command: false, word: false };

    /// menus.bas's form: File (New Ctrl+N, -, Beginner (radio, checked),
    /// Expert (radio), Exit (disabled)) and Edit (Sub (Deep)).
    fn store(tag: &str) -> MemStore {
        let id = |s: &str| format!("{tag}{s}");
        let mut s = MemStore::new();
        s.add(&id("form"), "RFORM", None).set(&id("form"), "clientwidth", v_int(300)).set(&id("form"), "clientheight", v_int(200));
        s.add(&id("main"), "RMAINMENU", Some(&id("form")));
        let item = |s: &mut MemStore, name: &str, parent: &str, caption: &str| {
            s.add(&id(name), "RMENUITEM", Some(&id(parent)));
            s.set(&id(name), "caption", v_str(caption)).set(&id(name), "parent", v_str(&id(parent)));
        };
        item(&mut s, "file", "main", "&File");
        item(&mut s, "new", "file", "&New");
        s.set(&id("new"), "shortcut", v_str("Ctrl+N"));
        item(&mut s, "sep", "file", "-");
        item(&mut s, "beg", "file", "&Beginner");
        s.set(&id("beg"), "radioitem", v_int(1)).set(&id("beg"), "checked", v_int(1));
        item(&mut s, "exp", "file", "&Expert");
        s.set(&id("exp"), "radioitem", v_int(1));
        item(&mut s, "quit", "file", "E&xit");
        s.set(&id("quit"), "enabled", v_int(0));
        item(&mut s, "edit", "main", "&Edit");
        item(&mut s, "sub", "edit", "&Sub");
        item(&mut s, "deep", "sub", "&Deep");
        s.add(&id("lbl"), "RLABEL", Some(&id("form")));
        s
    }

    fn setup(tag: &str) -> (MemStore, FormUi, TextSystem) {
        let s = store(tag);
        let mut ts = TextSystem::new();
        let mut f = FormUi::build(&s, &format!("{tag}form"), true);
        drop(f.paint(&s, &mut ts, 1.0));
        (s, f, ts)
    }

    fn click(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, x: f64, y: f64) {
        f.mouse_down(s, ts, x, y, Button::Left, Mods::NONE);
        f.mouse_up(s, ts, x, y, Button::Left, Mods::NONE);
    }

    fn picks(f: &mut FormUi) -> Vec<String> {
        f.take_events().into_iter().filter_map(|e| if let KernelEvent::MenuPick(id) = e { Some(id) } else { None }).collect()
    }

    #[test]
    fn the_bar_drops_menus_down_and_picks_items() {
        let (s, mut f, mut ts) = setup("mb1");
        assert_eq!(f.menu_offset, rapidr_value::layout::MAIN_MENU_HEIGHT);
        let bar = f.bar_items(&s);
        assert_eq!(bar.len(), 2);
        // A click on File drops its menu under it; the components don't hear it.
        let (fx, fy) = ((bar[0].1 .0 + 5) as f64, 10.0);
        click(&mut f, &s, &mut ts, fx, fy);
        assert!(f.menu_open());
        assert!(f.take_events().is_empty());
        let panel = f.menus.panels[0].clone();
        assert_eq!(panel.rect.1, f.menu_offset);
        // The first row is New: let go on it, it's picked and the menu closes.
        let (px, py) = ((panel.rect.0 + 30) as f64, (panel.rect.1 + super::BORDER + 5) as f64);
        f.mouse_move(&s, &mut ts, px, py, Mods::NONE);
        assert_eq!(f.menus.panels[0].hot, Some(0));
        click(&mut f, &s, &mut ts, px, py);
        assert_eq!(picks(&mut f), vec!["mb1new".to_string()]);
        assert!(!f.menu_open());
        // A disabled item isn't picked; a press outside closes the menu.
        click(&mut f, &s, &mut ts, fx, fy);
        let list = super::items("mb1file");
        let quit = list.iter().position(|i| i.id == "mb1quit").unwrap();
        let (top, _) = super::rows(&list)[quit];
        let (px, py) = ((panel.rect.0 + 30) as f64, (panel.rect.1 + top + 5) as f64);
        click(&mut f, &s, &mut ts, px, py);
        assert!(picks(&mut f).is_empty());
        assert!(f.menu_open());
        click(&mut f, &s, &mut ts, 250.0, 150.0);
        assert!(!f.menu_open());
        assert!(f.take_events().is_empty(), "the closing click reaches nothing");
    }

    #[test]
    fn keys_work_the_menus() {
        let (s, mut f, mut ts) = setup("mb2");
        let mut clip = MemClipboard::default();
        // Alt+E opens Edit with its first item chosen; Right opens Sub; Enter picks Deep.
        f.key_down(&s, &mut ts, 69, "", ALT, &mut clip);
        assert_eq!(f.menus.open_top, Some(1));
        assert_eq!(f.menus.panels[0].hot, Some(0));
        f.key_down(&s, &mut ts, 39, "", Mods::NONE, &mut clip);
        assert_eq!(f.menus.panels.len(), 2);
        f.key_down(&s, &mut ts, 13, "", Mods::NONE, &mut clip);
        assert_eq!(picks(&mut f), vec!["mb2deep".to_string()]);
        // Alt+F, Down skips the separator, then an item's letter picks it.
        f.key_down(&s, &mut ts, 70, "", ALT, &mut clip);
        f.key_down(&s, &mut ts, 40, "", Mods::NONE, &mut clip);
        assert_eq!(f.menus.panels[0].hot, Some(2));
        f.key_down(&s, &mut ts, 69, "e", Mods::NONE, &mut clip);
        assert_eq!(picks(&mut f), vec!["mb2exp".to_string()]);
        // Escape closes the menu, File stays selected; a second Escape
        // leaves (Windows'); no key event reached the form meanwhile.
        f.key_down(&s, &mut ts, 70, "", ALT, &mut clip);
        f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
        assert!(!f.menu_open());
        assert!(f.menus.keyboard && f.menus.hot_top == Some(0));
        f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
        assert!(!f.menus.keyboard);
        assert!(f.take_events().iter().all(|e| !matches!(e, KernelEvent::KeyPress { .. })));
        // Ctrl+N is New's ShortCut: picked, and nothing else hears the key.
        f.key_down(&s, &mut ts, 78, "", CTRL, &mut clip);
        let events = f.take_events();
        assert_eq!(events, vec![KernelEvent::MenuPick("mb2new".into())]);
    }

    // (the input lane's)
    #[test]
    fn f10_and_a_lone_alt_select_the_bar() {
        // (RapidQ's look, checked op for op: the classic theme, named)
        rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
        let (s, mut f, mut ts) = setup("mb4");
        let mut clip = MemClipboard::default();
        // F10: its OnKeyDown, then File selected (no menu yet)
        f.key_down(&s, &mut ts, 121, "", Mods::NONE, &mut clip);
        assert!(matches!(f.take_events()[..], [KernelEvent::KeyDown { vk: 121, .. }]));
        assert!(f.menus.keyboard && f.menus.hot_top == Some(0) && !f.menu_open());
        // Right selects Edit; keys reach nothing else meanwhile
        f.key_down(&s, &mut ts, 39, "", Mods::NONE, &mut clip);
        assert_eq!(f.menus.hot_top, Some(1));
        assert!(f.take_events().is_empty());
        // Down drops Edit's menu, its first item lit; Enter on Sub opens it
        f.key_down(&s, &mut ts, 40, "", Mods::NONE, &mut clip);
        assert_eq!((f.menus.open_top, f.menus.panels[0].hot, f.menus.keyboard), (Some(1), Some(0), false));
        // Escape: back to the selected bar; Left; a letter opens by mnemonic
        f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
        f.key_down(&s, &mut ts, 37, "", Mods::NONE, &mut clip);
        assert_eq!(f.menus.hot_top, Some(0));
        f.key_down(&s, &mut ts, 69, "e", Mods::NONE, &mut clip);
        assert_eq!(f.menus.open_top, Some(1));
        f.key_down(&s, &mut ts, 18, "", ALT, &mut clip);
        assert!(!f.menu_open() && !f.menus.keyboard, "Alt closes the menus");
        f.key_up(18, Mods::NONE);
        assert!(!f.menus.keyboard, "that Alt wasn't a lone one");
        f.take_events();
        // a lone Alt, pressed and let go: File selected; Alt again leaves
        f.key_down(&s, &mut ts, 18, "", ALT, &mut clip);
        f.key_up(18, Mods::NONE);
        assert!(f.menus.keyboard && f.menus.hot_top == Some(0));
        let list = f.paint(&s, &mut ts, 1.0).dump();
        assert!(list.contains("edge"), "the selected item is drawn raised: {list}");
        f.key_down(&s, &mut ts, 18, "", ALT, &mut clip);
        f.key_up(18, Mods::NONE);
        assert!(!f.menus.keyboard);
        // Alt + a letter isn't a lone Alt: its release selects nothing
        f.key_down(&s, &mut ts, 18, "", ALT, &mut clip);
        f.key_down(&s, &mut ts, 70, "", ALT, &mut clip);
        f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
        f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
        f.key_up(70, ALT);
        f.key_up(18, Mods::NONE);
        assert!(!f.menus.keyboard && !f.menu_open());
        // a press leaves the selection
        f.key_down(&s, &mut ts, 121, "", Mods::NONE, &mut clip);
        click(&mut f, &s, &mut ts, 250.0, 150.0);
        assert!(!f.menus.keyboard);
        // with the system's menu bar (macOS), F10 is an ordinary key
        f.menus.system_bar = true;
        f.key_down(&s, &mut ts, 121, "", Mods::NONE, &mut clip);
        assert!(!f.menus.keyboard);
    }

    /// The nodes of the tree that have the focus, as "role name".
    fn focused(tree: &AccessNode) -> Vec<String> {
        let mut out = Vec::new();
        tree.walk(&mut |n| {
            if n.states.focused {
                out.push(format!("{} {}", n.role.name(), n.name));
            }
        });
        out
    }

    // (the menus in the accessibility tree: the AccessKit tree's source)
    #[test]
    fn an_open_menu_is_in_the_accessibility_tree_and_the_keys_walk_it() {
        let (s, mut f, mut ts) = setup("mb5");
        let mut clip = MemClipboard::default();
        // Closed: the bar's items, File collapsed, the form focused.
        let tree = f.access_tree(&s, &mut ts);
        let file = &tree.children[0].children[0];
        assert_eq!((file.role, file.name.as_str(), file.states.expanded), (Role::MenuItem, "File", Some(false)));
        assert!(file.children.is_empty() && file.actions.contains(&Action::Expand));
        assert_eq!(focused(&tree), vec!["window ".to_string()]);
        // Alt+F: File's menu drops down inside File, its first item chosen.
        f.key_down(&s, &mut ts, 70, "", ALT, &mut clip);
        let tree = f.access_tree(&s, &mut ts);
        let file = &tree.children[0].children[0];
        assert_eq!(file.states.expanded, Some(true));
        assert!(file.actions.contains(&Action::Collapse));
        let menu = &file.children[0];
        assert_eq!((menu.role, menu.name.as_str(), menu.orientation), (Role::Menu, "File", Some(Orientation::Vertical)));
        // (the separator isn't a node; each item has its role, name and states)
        let names: Vec<(Role, &str)> = menu.children.iter().map(|c| (c.role, c.name.as_str())).collect();
        assert_eq!(names, vec![(Role::MenuItem, "New"), (Role::MenuItemRadio, "Beginner"), (Role::MenuItemRadio, "Expert"), (Role::MenuItem, "Exit")]);
        let [new, beginner, expert, exit] = &menu.children[..] else { panic!("four items") };
        assert_eq!(new.accelerator.as_deref(), Some("Ctrl+N"));
        assert_eq!((beginner.states.checked, expert.states.checked), (Some(true), Some(false)));
        assert!(exit.states.disabled && !new.states.disabled);
        assert_eq!(new.shortcut, None);
        let (_, _, w, h) = new.bounds;
        assert!(w > 20 && h == super::ITEM_H, "{:?}", new.bounds);
        // The focus is on New alone: not the form, not File.
        assert_eq!(focused(&tree), vec!["menuitem New".to_string()]);
        // The keys walk it: Down is Beginner, Down Expert, Down Exit (disabled, still chosen), Down wraps to New.
        for want in ["menuitemradio Beginner", "menuitemradio Expert", "menuitem Exit", "menuitem New"] {
            f.key_down(&s, &mut ts, 40, "", Mods::NONE, &mut clip);
            assert_eq!(focused(&f.access_tree(&s, &mut ts)), vec![want.to_string()]);
        }
        f.key_down(&s, &mut ts, 38, "", Mods::NONE, &mut clip);
        assert_eq!(focused(&f.access_tree(&s, &mut ts)), vec!["menuitem Exit".to_string()]);
        // Right moves to Edit: its menu replaces File's; Sub has a submenu, collapsed, then Right opens it.
        f.key_down(&s, &mut ts, 39, "", Mods::NONE, &mut clip);
        let tree = f.access_tree(&s, &mut ts);
        let (file, edit) = (&tree.children[0].children[0], &tree.children[0].children[1]);
        assert!(file.children.is_empty() && file.states.expanded == Some(false));
        let sub = &edit.children[0].children[0];
        assert_eq!((sub.name.as_str(), sub.states.expanded), ("Sub", Some(false)));
        assert_eq!(focused(&tree), vec!["menuitem Sub".to_string()]);
        f.key_down(&s, &mut ts, 39, "", Mods::NONE, &mut clip);
        let tree = f.access_tree(&s, &mut ts);
        let sub = &tree.children[0].children[1].children[0].children[0];
        assert_eq!(sub.states.expanded, Some(true));
        let deep = &sub.children[0];
        assert_eq!((deep.role, deep.name.as_str()), (Role::Menu, "Sub"));
        assert_eq!(deep.children[0].name, "Deep");
        assert_eq!(focused(&tree), vec!["menuitem Deep".to_string()]);
        // Escape closes the submenu, then the menu: the bar's item is chosen (F10's state), then the form has the focus again.
        f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
        f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
        assert_eq!(focused(&f.access_tree(&s, &mut ts)), vec!["menuitem Edit".to_string()]);
        f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
        assert_eq!(focused(&f.access_tree(&s, &mut ts)), vec!["window ".to_string()]);
        // It all reads as JSON (RAPIDR_TEST_A11Y's, the AX probe's).
        f.key_down(&s, &mut ts, 70, "", ALT, &mut clip);
        let json = f.access_tree(&s, &mut ts).to_json();
        for want in ["\"role\":\"menu\"", "\"role\":\"menuitemradio\"", "\"name\":\"Beginner\"", "\"accelerator\":\"Ctrl+N\"", "\"checked\"", "\"not-checked\""] {
            assert!(json.contains(want), "{want} in {json}");
        }
    }

    #[test]
    fn a_screen_reader_drives_the_menus() {
        let (s, mut f, mut ts) = setup("mb6");
        let id = |name: &str| node_id(&format!("mb6{name}"));
        let bar = |f: &mut FormUi, ts: &mut TextSystem| f.access_tree(&s, ts).children[0].children.iter().map(|c| c.id).collect::<Vec<_>>();
        let (file, edit) = (bar(&mut f, &mut ts)[0], bar(&mut f, &mut ts)[1]);
        // Expand File: its menu is open, the first item chosen. Collapse closes it.
        assert!(f.access_action(&s, &mut ts, file, Action::Expand, None));
        assert!(f.menu_open());
        assert_eq!(focused(&f.access_tree(&s, &mut ts)), vec!["menuitem New".to_string()]);
        assert!(f.access_action(&s, &mut ts, file, Action::Collapse, None));
        assert!(!f.menu_open());
        // Click Edit drops it; Expand on Sub (an item of the open panel) opens its submenu.
        assert!(f.access_action(&s, &mut ts, edit, Action::Click, None));
        assert!(f.access_action(&s, &mut ts, id("sub"), Action::Expand, None));
        assert_eq!(f.menus.panels.len(), 2);
        assert!(f.access_action(&s, &mut ts, id("sub"), Action::Collapse, None));
        assert_eq!(f.menus.panels.len(), 1);
        // Click File while Edit's is down: File's drops instead. A disabled item's Click is refused.
        assert!(f.access_action(&s, &mut ts, file, Action::Click, None));
        assert!(!f.access_action(&s, &mut ts, id("quit"), Action::Click, None));
        assert!(picks(&mut f).is_empty() && f.menu_open());
        // Focus chooses an item as the arrow keys would; Click picks it.
        assert!(f.access_action(&s, &mut ts, id("exp"), Action::Focus, None));
        assert_eq!(focused(&f.access_tree(&s, &mut ts)), vec!["menuitemradio Expert".to_string()]);
        assert!(f.access_action(&s, &mut ts, id("exp"), Action::Click, None));
        assert_eq!(picks(&mut f), vec!["mb6exp".to_string()]);
        assert!(!f.menu_open());
        // Closed menus have no item nodes: a stale id does nothing.
        assert!(!f.access_action(&s, &mut ts, id("exp"), Action::Click, None));
    }

    #[test]
    fn the_bar_selected_with_f10_has_the_focus_and_a_popup_menu_is_in_the_tree() {
        let (s, mut f, mut ts) = setup("mb7");
        let mut clip = MemClipboard::default();
        f.key_down(&s, &mut ts, 121, "", Mods::NONE, &mut clip);
        assert_eq!(focused(&f.access_tree(&s, &mut ts)), vec!["menuitem File".to_string()]);
        f.key_down(&s, &mut ts, 39, "", Mods::NONE, &mut clip);
        assert_eq!(focused(&f.access_tree(&s, &mut ts)), vec!["menuitem Edit".to_string()]);
        f.key_down(&s, &mut ts, 27, "", Mods::NONE, &mut clip);
        assert_eq!(focused(&f.access_tree(&s, &mut ts)), vec!["window ".to_string()]);
        // A pop-up menu (the context menu of an edit, QPOPUPMENU): last in the form, its items inside.
        let mut s = s;
        s.add("mb7pop", "RPOPUPMENU", Some("mb7form"));
        for (id, caption) in [("mb7p1", "Cut"), ("mb7p2", "&Paste")] {
            s.add(id, "RMENUITEM", Some("mb7pop")).set(id, "caption", v_str(caption)).set(id, "parent", v_str("mb7pop"));
        }
        assert!(f.open_popup("mb7pop", 40, 60));
        let tree = f.access_tree(&s, &mut ts);
        let menu = tree.children.last().unwrap();
        assert_eq!((menu.role, menu.name.as_str()), (Role::Menu, "Menu"));
        assert_eq!(menu.children.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Cut", "Paste"]);
        assert_eq!((menu.bounds.0, menu.bounds.1), (40, 60));
        // (the menu itself has the focus until an item is chosen)
        assert_eq!(focused(&tree), vec!["menu Menu".to_string()]);
        f.key_down(&s, &mut ts, 40, "", Mods::NONE, &mut clip);
        assert_eq!(focused(&f.access_tree(&s, &mut ts)), vec!["menuitem Cut".to_string()]);
    }

    #[test]
    fn the_bar_is_drawn_with_its_menu() {
        // (RapidQ's look, checked op for op: the classic theme, named)
        rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
        let (s, mut f, mut ts) = setup("mb3");
        let list = f.paint(&s, &mut ts, 1.0).dump();
        assert!(list.contains("\"File\""), "{list}");
        f.key_down(&s, &mut ts, 70, "", ALT, &mut MemClipboard::default());
        let list = f.paint(&s, &mut ts, 1.0).dump();
        assert!(list.contains("\"Ctrl+N\""), "{list}");
        assert!(list.contains("\"Beginner\""), "{list}");
        // (the radio item's dot, the separator's two lines)
        assert!(list.contains("shape"), "{list}");
        // Screen readers see the bar.
        let tree = f.access_tree(&s, &mut ts);
        assert_eq!(tree.children[0].role, rapidr_value::objects::a11y::Role::MenuBar);
        assert_eq!(tree.children[0].children[0].name, "File");
    }
}
