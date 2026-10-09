//! Editing a caption in place: F2 on the selected component (or a slow
//! click — a second click on the component already selected, as Explorer
//! renames), the menu editor's items and its "Type Here" slots. Delphi's
//! convention holds: a double click makes the event handler, so it isn't
//! the way in (docs/studio-wow.md DES-17).
//!
//! The editor sits over the component (or the menu row) with the caption
//! selected: typing replaces it, the arrows, Home, End, Backspace and
//! Delete edit it, Enter writes it (one undo step — `Caption = "…"`, the
//! smallest edit), Escape leaves it as it was; a press elsewhere writes it.
//! A menu item's editor has a second field, its ShortCut (Tab goes there):
//! the key pressed there with Ctrl, Alt or Shift, or a function key, is
//! the shortcut ("Ctrl+O"); Backspace clears it.

use super::menus::Slot;
use super::{value, Command, DesignSurface, NodeId, Subtree};
use crate::designer::model::Item;
use crate::objects::font::Font;
use crate::objects::ops::{Op, Place, Rect};
use crate::objects::text::text_size;

/// What the editor changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A component's Caption (or Text).
    Prop { node: NodeId, prop: &'static str },
    /// A new menu item typed on a "Type Here" slot: under `parent`, before
    /// its child `before` (`None`: last).
    NewItem { parent: NodeId, before: Option<NodeId> },
}

/// Which field has the keys (a menu item's editor has two).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Caption,
    ShortCut,
}

/// The in-place editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineEdit {
    pub target: Target,
    /// The caption as typed so far.
    pub text: String,
    /// The caret and the selection's other end (characters).
    pub caret: usize,
    pub anchor: usize,
    pub field: Field,
    /// A menu item's ShortCut ("Ctrl+O"; "" none).
    pub shortcut: String,
    /// It edits a menu item (it has the ShortCut field).
    pub menu: bool,
}

impl InlineEdit {
    fn new(target: Target, text: String, shortcut: String, menu: bool) -> InlineEdit {
        let n = text.chars().count();
        InlineEdit { target, text, caret: n, anchor: 0, field: Field::Caption, shortcut, menu }
    }

    fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    fn byte(&self, ch: usize) -> usize {
        self.text.char_indices().nth(ch).map_or(self.text.len(), |(i, _)| i)
    }

    /// The selection replaced by `s`.
    fn insert(&mut self, s: &str) {
        let (a, b) = self.selection();
        let (ba, bb) = (self.byte(a), self.byte(b));
        self.text.replace_range(ba..bb, s);
        self.caret = a + s.chars().count();
        self.anchor = self.caret;
    }

    /// A key on the caption field: whether it was the editor's.
    fn caption_key(&mut self, vk: i64, text: &str, shift: bool, ctrl: bool) -> bool {
        let n = self.text.chars().count();
        let (a, b) = self.selection();
        let mv = |e: &mut InlineEdit, to: usize| {
            e.caret = to;
            if !shift {
                e.anchor = to;
            }
        };
        match vk {
            8 => {
                if a == b && a > 0 {
                    self.anchor = a - 1;
                }
                self.insert("");
            }
            46 => {
                if a == b && b < n {
                    self.anchor = b + 1;
                }
                self.insert("");
            }
            37 => mv(self, if !shift && a != b { a } else { self.caret.saturating_sub(1) }),
            39 => mv(self, if !shift && a != b { b } else { (self.caret + 1).min(n) }),
            36 => mv(self, 0),
            35 => mv(self, n),
            65 if ctrl => {
                self.anchor = 0;
                self.caret = n;
            }
            _ if !ctrl && !text.is_empty() && text.chars().all(|c| !c.is_control()) => self.insert(text),
            _ => return false,
        }
        true
    }

    /// A key on the ShortCut field: a chord is the shortcut.
    fn shortcut_key(&mut self, vk: i64, shift: bool, ctrl: bool, alt: bool) -> bool {
        match vk {
            8 | 46 if !ctrl && !alt && !shift => self.shortcut.clear(),
            // (the modifier keys alone wait for the key)
            16 | 17 | 18 | 91 | 92 | 93 => {}
            _ => {
                let fkey = (112..=123).contains(&vk);
                let named = matches!(vk, 8 | 32 | 33..=40 | 45 | 46) || fkey;
                let printable = (48..=57).contains(&vk) || (65..=90).contains(&vk);
                if !(fkey || ((ctrl || alt) && (printable || named)) || (shift && named)) {
                    return false;
                }
                let sc = crate::objects::menu::Shortcut { vk, ctrl, shift, alt };
                self.shortcut = sc.text();
            }
        }
        true
    }
}

/// The name a new menu item gets: its caption's letters and digits (`&Open…`
/// → `Open1`), `N1` for a separator, as Delphi's menu designer names them.
pub fn item_name(design: &crate::designer::FormDesign, taken: &dyn Fn(&str) -> bool, caption: &str) -> String {
    let mut base: String = caption.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if base.is_empty() || caption.trim() == "-" {
        base = "N".into();
    }
    if base.starts_with(|c: char| c.is_ascii_digit()) {
        base.insert(0, 'N');
    }
    (1..).map(|k| format!("{base}{k}")).find(|n| design.find(n).is_none() && !taken(n)).unwrap_or(base)
}

impl DesignSurface {
    /// F2: the primary selection's caption edited in place (a menu item's
    /// with its ShortCut). Whether it can be.
    pub fn begin_edit(&mut self) -> bool {
        if self.read_only() || self.no_form() {
            return false;
        }
        let Some(id) = self.designer.selection.primary() else { return false };
        let Some(n) = self.designer.design.node(id) else { return false };
        let menu = n.canonical == "RMENUITEM";
        let prop = if menu {
            "Caption"
        } else {
            if !self.on_form(id) {
                return false;
            }
            match rapidr_lang::component(&n.canonical).and_then(|c| ["Caption", "Text"].into_iter().find(|p| c.property(p).is_some())) {
                Some(p) => p,
                None => return false,
            }
        };
        let text = n.prop(prop).map(super::plain_value).unwrap_or_default();
        let shortcut = if menu { n.prop("ShortCut").map(super::plain_value).unwrap_or_default() } else { String::new() };
        self.typing = None;
        self.editing = Some(InlineEdit::new(Target::Prop { node: id, prop }, text, shortcut, menu));
        self.say(format!("Editing {}'s {prop}: Enter writes it, Escape leaves it", n.name));
        true
    }

    /// A "Type Here" slot of the menu editor: a new item's caption typed.
    pub(super) fn begin_new_item(&mut self, parent: NodeId, before: Option<NodeId>, first: &str) {
        self.typing = None;
        let mut e = InlineEdit::new(Target::NewItem { parent, before }, String::new(), String::new(), true);
        e.insert(first);
        self.editing = Some(e);
        self.say("New menu item: type its caption (- for a separator), Tab for its ShortCut, Enter to add it");
    }

    /// Whether the editor shows over this menu slot.
    pub fn editing_slot(&self, slot: Slot) -> bool {
        match (&self.editing, slot) {
            (Some(InlineEdit { target: Target::Prop { node, .. }, menu: true, .. }), Slot::Item(id)) => *node == id,
            (Some(InlineEdit { target: Target::NewItem { parent, before }, .. }), Slot::New { parent: p, before: b }) => *parent == p && *before == b,
            _ => false,
        }
    }

    /// A key while editing: whether the editor took it (every key does but
    /// the ones it leaves: Ctrl+Z …).
    pub(super) fn edit_key(&mut self, vk: i64, text: &str, shift: bool, ctrl: bool, alt: bool) -> bool {
        let Some(mut e) = self.editing.take() else { return false };
        match vk {
            27 => {
                self.say("Edit cancelled");
                return true;
            }
            13 => {
                self.end_edit_with(e, true);
                return true;
            }
            9 if e.menu => {
                e.field = if e.field == Field::Caption { Field::ShortCut } else { Field::Caption };
                self.say(if e.field == Field::ShortCut { "ShortCut: press the keys (Backspace clears it)" } else { "Caption" });
                self.editing = Some(e);
                return true;
            }
            _ => {}
        }
        let took = match e.field {
            Field::Caption => e.caption_key(vk, text, shift, ctrl),
            Field::ShortCut => e.shortcut_key(vk, shift, ctrl, alt),
        };
        if took && e.field == Field::ShortCut {
            let said = if e.shortcut.is_empty() { "No ShortCut".to_string() } else { format!("ShortCut {}", e.shortcut) };
            self.say(said);
        }
        self.editing = Some(e);
        took || !ctrl
    }

    /// The editor closed: what it holds written (`write`), else dropped.
    pub fn end_edit(&mut self, write: bool) {
        if let Some(e) = self.editing.take() {
            self.end_edit_with(e, write);
        }
    }

    fn end_edit_with(&mut self, e: InlineEdit, write: bool) {
        if !write || self.read_only() {
            return;
        }
        match e.target {
            Target::Prop { node, prop } => {
                let Some(n) = self.designer.design.node(node) else { return };
                let mut cmds = Vec::new();
                let old = n.prop(prop).map(super::plain_value).unwrap_or_default();
                if old != e.text || n.prop(prop).is_none() {
                    cmds.push(Command::SetProp { node, name: prop.to_string(), value: Some(value::write_str(&e.text)) });
                }
                if e.menu {
                    let old = n.prop("ShortCut").map(super::plain_value).unwrap_or_default();
                    if old != e.shortcut {
                        let v = (!e.shortcut.is_empty()).then(|| value::write_str(&e.shortcut));
                        cmds.push(Command::SetProp { node, name: "ShortCut".into(), value: v });
                    }
                }
                let name = n.name.clone();
                if cmds.is_empty() {
                    return;
                }
                let _ = self.designer.execute(Command::Batch(cmds));
                self.commit();
                self.say(format!("{name}'s {prop}: {}", e.text));
            }
            Target::NewItem { parent, before } => {
                if e.text.trim().is_empty() {
                    return;
                }
                let taken = |n: &str| self.source.as_ref().is_some_and(|a| a.doc.borrow().name_taken(n));
                let name = item_name(&self.designer.design, &taken, &e.text);
                let mut props = vec![("Caption", value::write_str(&e.text))];
                if !e.shortcut.is_empty() {
                    props.push(("ShortCut", value::write_str(&e.shortcut)));
                }
                // (in the file's own names: RMenuItem, or QMENUITEM in a
                // RapidQ-style file — R-NAMES)
                let item = rapidr_lang::component("RMENUITEM").map_or_else(|| "RMenuItem".to_string(), |c| c.name_in(self.designer.design.names()));
                let tree = Subtree::new(&name, &item, &props);
                let Some(p) = self.designer.design.node(parent) else { return };
                let index = match before {
                    Some(b) => p.body.iter().position(|i| *i == Item::Child(b)).unwrap_or(p.body.len()),
                    None => p.body.len(),
                };
                if self.designer.execute(Command::Insert { parent, index, tree }).is_err() {
                    return;
                }
                self.commit();
                let Some(id) = self.designer.design.find(&name) else { return };
                self.designer.selection.set(id);
                self.outside_sel = None;
                self.selection_changed();
                self.say(format!("Added menu item {name}: {}", e.text));
                // (and on to the next: a bar item's menu opens on its first
                // slot; in a menu, the slot after it)
                let main = self.main_menu();
                if Some(parent) == main {
                    self.menu_open = vec![id];
                    self.begin_new_item(id, None, "");
                } else if before.is_none() {
                    self.open_path_to(parent);
                    self.begin_new_item(parent, None, "");
                }
            }
        }
    }

    /// Where the editor is: its rectangle on the surface (from the client
    /// area's origin, zoomed places), the menu's rows included.
    pub fn edit_rect(&self) -> Option<Rect> {
        let e = self.editing.as_ref()?;
        let r = match &e.target {
            Target::Prop { node, .. } if !e.menu => {
                let r = self.rect_of(*node)?;
                (r.left, r.top, r.width, r.height)
            }
            Target::Prop { node, .. } => self.menu_slot_rect(Slot::Item(*node))?,
            Target::NewItem { parent, before } => self.menu_slot_rect(Slot::New { parent: *parent, before: *before })?,
        };
        Some(self.view_rect(r))
    }

    /// The editor's fields on the surface (from the client area's origin):
    /// the caption's, over the component and wide enough for its text, and
    /// a menu item's ShortCut's beside it.
    pub fn edit_fields(&self) -> Option<(Rect, Option<Rect>)> {
        let (e, (x, y, w, h)) = (self.editing.as_ref()?, self.edit_rect()?);
        let font = Font::default();
        let (tw, th) = text_size(if e.text.is_empty() { " " } else { &e.text }, &font);
        let field_h = th + 6;
        let keys_w = 76;
        let fw = (tw + 10).max(if e.menu { w - keys_w - 10 } else { w }).max(40);
        let fy = y + (h - field_h) / 2;
        let keys = e.menu.then_some((x + fw + 6, fy, keys_w, field_h));
        Some(((x, fy, fw, field_h), keys))
    }

    /// The editor drawn (surface pixels from the client area's origin).
    pub(super) fn edit_ops(&self, out: &mut Vec<Op>) {
        let (Some(e), Some((field, keys))) = (self.editing.as_ref(), self.edit_fields()) else { return };
        let t = crate::theme::current();
        let font = Font::default();
        let pad = 4;
        let (x, fy, fw, field_h) = field;
        let frame = |out: &mut Vec<Op>, r: Rect, on: bool| {
            out.push(Op::Fill { rect: (r.0 - 1, r.1 - 1, r.2 + 2, r.3 + 2), color: if on { t.accent } else { t.border_strong } });
            out.push(Op::Fill { rect: r, color: t.window });
        };
        let caption_on = e.field == Field::Caption;
        frame(out, field, caption_on);
        let (a, b) = e.selection();
        let prefix = |k: usize| text_size(&e.text.chars().take(k).collect::<String>(), &font).0;
        if a != b && caption_on {
            let (sx, ex) = (prefix(a), prefix(b));
            out.push(Op::Fill { rect: (x + pad + sx, fy + 3, ex - sx, field_h - 6), color: t.highlight });
        }
        let ink = if a != b && caption_on { t.highlight_text } else { t.text };
        out.push(Op::Text { rect: (x + pad, fy, fw - pad, field_h), text: e.text.clone(), font: font.clone(), color: ink, angle: 0, place: Place::Left });
        if caption_on {
            let cx = x + pad + prefix(e.caret);
            out.push(Op::Fill { rect: (cx, fy + 3, 1, field_h - 6), color: t.text });
        }
        if let Some(k) = keys {
            frame(out, k, !caption_on);
            let (label, color) = if e.shortcut.is_empty() { ("ShortCut".to_string(), t.gray_text) } else { (e.shortcut.clone(), t.text) };
            out.push(Op::Text { rect: (k.0 + pad, k.1, k.2 - pad, k.3), text: label, font: font.clone(), color, angle: 0, place: Place::Left });
        }
    }

    /// A press while editing (client coordinates): inside the editor it
    /// stays (a press on the other field moves there); elsewhere the
    /// caption is written and the press goes on. Whether it was the
    /// editor's.
    pub(super) fn edit_press(&mut self, x: i64, y: i64) -> bool {
        let Some((field, keys)) = self.edit_fields() else { return false };
        let (vx, vy) = (self.zv(x), self.zv(y));
        let inside = |r: Rect| vx >= r.0 && vx < r.0 + r.2 && vy >= r.1 && vy < r.1 + r.3;
        if inside(field) || keys.is_some_and(inside) {
            let to = if keys.is_some_and(inside) { Field::ShortCut } else { Field::Caption };
            if let Some(e) = &mut self.editing {
                e.field = to;
            }
            return true;
        }
        self.end_edit(true);
        false
    }
}
